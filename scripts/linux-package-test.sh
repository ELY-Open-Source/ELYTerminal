#!/bin/sh
# linux-package-test.sh PACKAGE: validates packages in a disposable container.
set -eu
fail() { echo "linux-package-test: $*" >&2; exit 1; }
[ "$#" -eq 1 ] || fail "expected one absolute package path"
package=$1
case $package in /*) ;; *) fail "package path must be absolute" ;; esac
[ -f "$package" ] || fail "no package $package"
[ -e /.dockerenv ] || [ -e /run/.containerenv ] || fail "use a disposable container"
[ "$(id -u)" -eq 0 ] || fail "container package tests need root"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
app_id=com.zacharyzhang.elyterminal

seed_config() {
    config=/home/ci/.config/ELYTerminal/settings.json
    [ ! -e "$config" ] || fail "refusing to replace an existing settings file"
    mkdir -p "$(dirname "$config")"
    printf '%s\n' '{"check_updates":false}' > "$config"
    sha256sum "$config" > "$work/settings.sha256"
}

gui_test() {
    [ "${LINUX_GUI_TEST:-0}" = 1 ] || return 0
    echo "linux-package-test: GUI uses Mesa software OpenGL under Xvfb"
    runuser -u ci -- env LINUX_TEST_OUTPUT="${LINUX_TEST_OUTPUT:-/tmp}" \
        LIBGL_ALWAYS_SOFTWARE=1 VK_ICD_FILENAMES=/dev/null \
        xvfb-run -a -s '-screen 0 1280x900x24' \
        python3 "$(dirname "$0")/linux-gui-test.py" "$@"
}

payload() {
    root=$1
    [ -x "$root/usr/bin/elyterminal" ] || fail "missing executable"
    desktop="$root/usr/share/applications/$app_id.desktop"
    [ -f "$desktop" ] || fail "missing desktop entry"
    desktop-file-validate "$desktop"
    [ -s "$root/usr/share/icons/hicolor/scalable/apps/$app_id.svg" ] || fail "missing icon"
    [ -s "$root/usr/share/doc/elyterminal/copyright" ] || fail "missing license"
    [ -s "$root/usr/share/man/man1/elyterminal.1.gz" ] || fail "missing manual page"
    grep -q 'MIT License' "$root/usr/share/doc/elyterminal/copyright" || fail "missing MIT license text"
    grep -q 'SunSoft' "$root/usr/share/doc/elyterminal/libX11-lcUniConv" || fail "missing libX11's notice for the decoder's tables"
    freetype_license="$root/usr/share/doc/elyterminal/FreeType-LICENSE.txt"
    [ -s "$freetype_license" ] || fail "missing FreeType license"
    grep -q 'The FreeType Project LICENSE' "$freetype_license" || fail "missing FreeType license text"
    ldd "$root/usr/bin/elyterminal" > "$work/ldd"
    cat "$work/ldd"
    if grep -q 'not found' "$work/ldd"; then fail "unresolved ELF dependency"; fi
}

case $package in
    *.AppImage)
        [ -x "$package" ] || fail "AppImage is not executable"
        magic=$(od -An -tx1 -j8 -N3 "$package" | tr -d ' \n')
        [ "$magic" = 414902 ] || fail "not a type 2 AppImage"
        file "$package"
        cd "$work"
        "$package" --appimage-extract > extract.log
        [ -x squashfs-root/AppRun ] || fail "missing AppRun"
        [ -f "squashfs-root/$app_id.desktop" ] || fail "missing root desktop entry"
        [ -s "squashfs-root/$app_id.svg" ] || fail "missing root icon"
        payload "$work/squashfs-root"
        seed_config
        chmod 755 "$work"
        install -m755 "$package" "$work/portable.AppImage"
        gui_test "$work/portable.AppImage" --appimage-extract-and-run
        sha256sum -c "$work/settings.sha256"
        rm "$work/portable.AppImage"
        [ ! -e "$work/portable.AppImage" ] || fail "portable copy remains after removal"
        sha256sum -c "$work/settings.sha256"
        echo "PASSED: AppImage type, permissions, extraction, payload, license and ELF dependencies"
        echo "PASSED: AppImage portable copy, removal and settings preservation"
        [ "${LINUX_GUI_TEST:-0}" = 1 ] || echo "NOT TESTED: AppImage GUI launch"
        echo "NOT TESTED: Wayland in this container"
        exit 0
        ;;
    *.deb)
        export DEBIAN_FRONTEND=noninteractive
        [ "$(dpkg-deb -f "$package" Package)" = elyterminal ] || fail "wrong DEB package name"
        [ "$(dpkg-deb -f "$package" Architecture)" = amd64 ] || fail "wrong DEB architecture"
        dpkg-deb --info "$package"
        dpkg-deb --contents "$package"
        if dpkg-query -W -f='${db:Status-Status}' elyterminal 2>/dev/null | grep -qx installed; then
            fail "use a clean container without ELYTerminal installed"
        fi
        printf '%s\n' 'path-include=/usr/share/doc/elyterminal/*' \
            'path-include=/usr/share/man/man1/elyterminal.1.gz' \
            > /etc/dpkg/dpkg.cfg.d/zz-elyterminal-package-test
        apt-get update -o APT::Update::Error-Mode=any -o Acquire::Retries=3
        apt-get install -y --no-install-recommends ca-certificates desktop-file-utils
        ;;
    *.rpm)
        [ "$(rpm -qp --qf '%{NAME}' "$package")" = elyterminal ] || fail "wrong RPM package name"
        [ "$(rpm -qp --qf '%{ARCH}' "$package")" = x86_64 ] || fail "wrong RPM architecture"
        rpm -qpi "$package"
        rpm -qpl "$package"
        rpm -qpR "$package"
        if rpm -q elyterminal >/dev/null 2>&1; then fail "use a clean container"; fi
        dnf install -y --setopt=install_weak_deps=False desktop-file-utils
        ;;
    *) fail "unsupported package format: $package" ;;
esac

seed_config

case $package in
    *.deb)
        version=$(dpkg-deb -f "$package" Version)
        case $version in ''|*[!0-9.]*) fail "unsupported DEB fixture version $version" ;; esac
        older_version=$version~packaging-test
        dpkg --compare-versions "$older_version" lt "$version" || fail "fixture must sort before target"
        dpkg-deb -R "$package" "$work/older"
        sed "s/^Version: .*/Version: $older_version/" "$work/older/DEBIAN/control" > "$work/control"
        mv "$work/control" "$work/older/DEBIAN/control"
        dpkg-deb --root-owner-group --build "$work/older" "$work/older.deb"
        apt-get install -y --no-install-recommends "$work/older.deb"
        [ "$(dpkg-query -W -f='${Version}' elyterminal)" = "$older_version" ] || fail "fixture was not installed"
        sha256sum -c "$work/settings.sha256"
        apt-get install -y --no-install-recommends "$package"
        [ "$(dpkg-query -W -f='${Version}' elyterminal)" = "$version" ] || fail "target version was not installed"
        echo "PASSED: DEB metadata-only upgrade fixture, $older_version to $version"
        ;;
    *.rpm) dnf install -y --setopt=install_weak_deps=False --setopt=tsflags='' "$package" ;;
esac
payload ""
gui_test /usr/bin/elyterminal
case $package in
    *.deb)
        [ -s /var/lib/dpkg/info/elyterminal.md5sums ] || fail "missing DEB file checksums"
        dpkg --verify elyterminal > "$work/verify"
        [ ! -s "$work/verify" ] || fail "DEB file verification failed"
        ;;
    *.rpm) rpm -V elyterminal ;;
esac
for program in git curl xdg-open; do command -v "$program" >/dev/null || fail "missing $program"; done
sha256sum -c "$work/settings.sha256"
case $package in
    *.deb) apt-get install -y --no-install-recommends --reinstall "$package" ;;
    *.rpm) dnf reinstall -y --setopt=install_weak_deps=False --setopt=tsflags='' "$package" ;;
esac
payload ""
sha256sum -c "$work/settings.sha256"
case $package in
    *.deb) apt-get remove -y elyterminal ;;
    *.rpm) dnf remove -y elyterminal ;;
esac
[ ! -e /usr/bin/elyterminal ] || fail "executable remains after removal"
[ ! -e "/usr/share/applications/$app_id.desktop" ] || fail "desktop entry remains after removal"
[ ! -e "/usr/share/icons/hicolor/scalable/apps/$app_id.svg" ] || fail "icon remains after removal"
sha256sum -c "$work/settings.sha256"
echo "PASSED: metadata, payload, dependencies, install, same-version reinstall, removal and settings preservation"
case $package in
    *.deb) echo "NOT TESTED: upgrade from a previous application release and Wayland" ;;
    *.rpm) echo "NOT TESTED: cross-version upgrade and Wayland in this container" ;;
esac
[ "${LINUX_GUI_TEST:-0}" = 1 ] || echo "NOT TESTED: GUI launch"
