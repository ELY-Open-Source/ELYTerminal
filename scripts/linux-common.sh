#!/bin/sh
# Shared metadata and payload for Linux packages.
set -eu
export LC_ALL=C
umask 022

linux_run() {
    docker run --rm --user "$(id -u):$(id -g)" \
        --network "${LINUX_DOCKER_NETWORK:-bridge}" -v "$PWD:/src" -w /src \
        -e http_proxy -e https_proxy -e no_proxy -e HTTP_PROXY -e HTTPS_PROXY -e NO_PROXY "$@"
}

linux_require() {
    for tool do
        command -v "$tool" >/dev/null 2>&1 || {
            echo "linux-packaging: missing $tool; see packaging/linux/README.md" >&2
            exit 1
        }
    done
}

linux_version() {
    version=$(sed -n '/^\[package\]/,/^\[/s/^version = "\([^"]*\)"$/\1/p' Cargo.toml)
    case $version in
        ''|*[!0-9.]*) echo "linux-packaging: unsupported Cargo version $version" >&2; exit 1 ;;
    esac
}

linux_init() {
    if [ -z "${LINUX_BINARY:-}" ]; then
        if [ -n "${1:-}" ] && [ "$(uname -m)" != "$1" ]; then
            echo "linux-packaging: this format supports $1 only; use linux-tar.sh for ARM64" >&2
            exit 1
        fi
        scripts/linux-build.sh
        linux_run \
            -e LINUX_BINARY=/src/target/release/elyterminal \
            -e LINUX_HOST_ARCH="$(uname -m)" \
            elyterminal-release "scripts/$(basename "$0")"
        exit
    fi
    linux_require readelf objdump
    linux_version
    out=${LINUX_OUT_DIR:-"$PWD/target/release"}
    mkdir -p "$out"
    out=$(cd "$out" && pwd)
    binary=${LINUX_BINARY:-"$PWD/target/release/elyterminal"}
    [ -x "$binary" ] || { echo "linux-packaging: no executable at $binary" >&2; exit 1; }
    binary=$(cd "$(dirname "$binary")" && pwd)/$(basename "$binary")
    machine=$(readelf -h "$binary" | sed -n 's/.*Machine: *//p')
    case $machine in
        *X86-64*) arch=x86_64 ;;
        AArch64) arch=aarch64 ;;
        *) echo "linux-packaging: unsupported ELF machine $machine" >&2; exit 1 ;;
    esac
    if [ -n "${1:-}" ] && [ "$arch" != "$1" ]; then
        echo "linux-packaging: this format requires $1, found $arch" >&2
        exit 1
    fi
    glibc=$(objdump -T "$binary" | sed -n 's/.*GLIBC_\([0-9][0-9.]*\).*/\1/p' | sort -Vu | tail -n 1)
    [ -n "$glibc" ] || { echo "linux-packaging: ELF has no glibc requirements" >&2; exit 1; }
    newest=$(printf '%s\n' 2.35 "$glibc" | sort -V | tail -n 1)
    [ "$newest" = 2.35 ] || {
        echo "linux-packaging: binary requires glibc $glibc, above the 2.35 baseline" >&2; exit 1
    }
    id=com.zacharyzhang.elyterminal
    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT
    trap 'exit 1' HUP INT TERM
    echo "linux-packaging: $version $arch, glibc $glibc or later"
}

linux_stage() {
    dest=$1
    install -Dm755 "$binary" "$dest/usr/bin/elyterminal"
    install -Dm644 "packaging/linux/$id.desktop" "$dest/usr/share/applications/$id.desktop"
    install -Dm644 assets/icons/elyterminal.svg "$dest/usr/share/icons/hicolor/scalable/apps/$id.svg"
    docs=$dest/usr/share/doc/elyterminal
    install -Dm644 LICENSE "$docs/copyright"
    install -m644 LICENSE-GHOSTTY "$docs/"
    install -m644 packaging/linux/FreeType-LICENSE.txt "$docs/"
    install -m644 vendor/xim-ctext/LICENSE "$docs/xim-ctext-LICENSE"
    install -m644 assets/fonts/*LICENSE.txt "$docs/"
    install -m644 assets/themes/LICENSE "$docs/themes-LICENSE"
    install -d "$dest/usr/share/man/man1"
    gzip -9 -n -c packaging/linux/elyterminal.1 > "$dest/usr/share/man/man1/elyterminal.1.gz"
}
