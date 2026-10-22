#!/bin/sh
# Keep the original tarball's flat binary, desktop entry and icon layout.
set -eu
cd "$(dirname "$0")/.."
. scripts/linux-common.sh
linux_init ''
name=ELYTerminal-$version-linux-${LINUX_HOST_ARCH:-$arch}
linux_stage "$work/stage"
mkdir "$work/$name"
cp "$work/stage/usr/bin/elyterminal" "$work/$name/"
cp "$work/stage/usr/share/applications/$id.desktop" "$work/$name/"
cp "$work/stage/usr/share/icons/hicolor/scalable/apps/$id.svg" "$work/$name/"
cp -R "$work/stage/usr/share/doc/elyterminal" "$work/$name/licenses"
tar -C "$work" -czf "$out/$name.tar.gz" "$name"
echo "linux-tar: $out/$name.tar.gz"
