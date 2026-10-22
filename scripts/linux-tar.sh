#!/bin/sh
# linux-tar.sh: the release build on Debian 12's glibc 2.36, in a tarball with its desktop entry and icon.
set -eu
cd "$(dirname "$0")/.."
docker build -q -t elyterminal-release - >/dev/null <<'EOF'
FROM rust:1.95.0-bookworm
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config libwayland-dev libxkbcommon-dev \
    libxkbcommon-x11-dev libxcb1-dev libx11-xcb-dev libfontconfig-dev libfreetype-dev \
 && rm -rf /var/lib/apt/lists/* \
 && rustup component add clippy rustfmt
EOF
run() { docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/src" -w /src elyterminal-release "$@"; }
run cargo build --release --locked
version=$(run cargo pkgid)
version=${version##*[#@]}
case $version in [0-9]*) ;; *) echo "linux-tar: cargo pkgid names no version" >&2; exit 1 ;; esac
out=target/release
id=com.zacharyzhang.elyterminal
name=ELYTerminal-$version-linux-$(uname -m)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/$name"
cp "$out/elyterminal" "$work/$name/"
cp assets/icons/elyterminal.svg "$work/$name/$id.svg"
mkdir "$work/$name/licenses"
cp vendor/xim-ctext/LICENSE "$work/$name/licenses/xim-ctext-LICENSE"
cat > "$work/$name/$id.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=ELYTerminal
Exec=elyterminal
Icon=$id
Terminal=false
Categories=System;TerminalEmulator;
StartupWMClass=$id
EOF
tar -C "$work" -czf "$out/$name.tar.gz" "$name"
glibc=$(run sh -c "objdump -T $out/elyterminal | grep -o 'GLIBC_[0-9.]*' | sort -u -V | tail -n 1")
echo "linux-tar: $out/$name.tar.gz, glibc ${glibc#GLIBC_} or later"
