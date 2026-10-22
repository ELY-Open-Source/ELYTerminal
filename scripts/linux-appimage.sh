#!/bin/sh
# linux-appimage.sh: package the shared release binary without changing shell children.
set -eu
cd "$(dirname "$0")/.."
. scripts/linux-common.sh
linux_init x86_64
linux_require curl sha256sum patchelf ldd
export LC_ALL=C

appdir=$work/AppDir
linux_stage "$appdir"
install -m755 packaging/linux/AppRun "$appdir/AppRun"
ln -s "usr/share/applications/$id.desktop" "$appdir/$id.desktop"
ln -s "usr/share/icons/hicolor/scalable/apps/$id.svg" "$appdir/$id.svg"
ln -s "$id.svg" "$appdir/.DirIcon"
mkdir -p "$appdir/usr/lib" "$appdir/usr/share/doc/elyterminal/bundled-libraries"

if ! ldd "$binary" > "$work/ldd" 2>&1 || grep -q 'not found' "$work/ldd"; then
    cat "$work/ldd" >&2
    echo 'linux-appimage: resolve the release binary libraries before packaging' >&2
    exit 1
fi

# libxcb and its dependencies stay together on the host.
for entry in \
    libxkbcommon.so.0:libxkbcommon0 \
    libxkbcommon-x11.so.0:libxkbcommon-x11-0 \
    libxcb-xkb.so.1:libxcb-xkb1
do
    soname=${entry%%:*}
    package=${entry#*:}
    library=$(awk -v name="$soname" '$1 == name && $2 == "=>" {print $3}' "$work/ldd")
    if [ -z "$library" ]; then
        echo "linux-appimage: the release binary does not resolve $soname" >&2
        exit 1
    fi
    copyright=/usr/share/doc/$package/copyright
    if [ ! -f "$copyright" ]; then
        echo "linux-appimage: missing $copyright; package in the Ubuntu build environment" >&2
        exit 1
    fi
    readelf --version-info "$library" > "$work/versions"
    if ! awk '
        /Name: GLIBC_/ {
            for (i = 1; i <= NF; i++) if ($i ~ /^GLIBC_/) {
                version = $i
                sub(/^GLIBC_/, "", version)
                split(version, parts, ".")
                if (version !~ /^[0-9.]+$/ || parts[1] > 2 ||
                    (parts[1] == 2 && parts[2] > 35)) exit 1
            }
        }' "$work/versions"; then
        echo "linux-appimage: $library requires glibc newer than 2.35; use the Ubuntu builder" >&2
        exit 1
    fi
    install -m644 "$library" "$appdir/usr/lib/$soname"
    install -m644 "$copyright" "$appdir/usr/share/doc/elyterminal/bundled-libraries/$package.copyright"
    patchelf --set-rpath "\$ORIGIN" "$appdir/usr/lib/$soname"
done

# Refuse an unreviewed dependency if a bundled library changes its ABI closure.
for library in "$appdir"/usr/lib/*.so.*; do
    readelf -d "$library" > "$work/dynamic"
    sed -n 's/.*(NEEDED).*\[\(.*\)\].*/\1/p' "$work/dynamic" > "$work/needed"
    while IFS= read -r needed; do
        case $needed in
            libc.so.6|libdl.so.2|libm.so.6|libpthread.so.0|librt.so.1|libxcb.so.1) ;;
            *)
                if [ ! -f "$appdir/usr/lib/$needed" ]; then
                    echo "linux-appimage: $library needs $needed; review the bundled library list" >&2
                    exit 1
                fi ;;
        esac
    done < "$work/needed"
done
patchelf --set-rpath "\$ORIGIN/../lib" "$appdir/usr/bin/elyterminal"

cache=$out/../linux-tools
mkdir -p "$cache"
cache=$(cd "$cache" && pwd)
download() {
    tool_name=$1
    tool_url=$2
    tool_sha=$3
    if [ ! -f "$cache/$tool_name" ]; then
        echo "linux-appimage: downloading $tool_name"
        curl --fail --location --retry 3 --proto '=https' --tlsv1.2 \
            "$tool_url" --output "$work/$tool_name"
        printf '%s  %s\n' "$tool_sha" "$work/$tool_name" | sha256sum --check --status || {
            echo "linux-appimage: checksum failed for downloaded $tool_name" >&2
            exit 1
        }
        install -m755 "$work/$tool_name" "$cache/$tool_name"
    fi
    printf '%s  %s\n' "$tool_sha" "$cache/$tool_name" | sha256sum --check --status || {
        echo "linux-appimage: checksum failed for cached $tool_name; remove it and retry" >&2
        exit 1
    }
}
download appimagetool-1.9.1-x86_64.AppImage \
    https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage \
    ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
download runtime-20251108-x86_64 \
    https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64 \
    2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d

artifact=$out/ELYTerminal-$version-linux-$arch.AppImage
ARCH=$arch "$cache/appimagetool-1.9.1-x86_64.AppImage" --appimage-extract-and-run \
    --runtime-file "$cache/runtime-20251108-x86_64" "$appdir" "$artifact"
chmod 755 "$artifact"
echo "linux-appimage: $artifact"
