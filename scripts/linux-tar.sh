#!/bin/sh
# A flat tarball: binary, entry, icon, man page, licenses.
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
cp "$work/stage/usr/share/man/man1/elyterminal.1.gz" "$work/$name/"
cp -R "$work/stage/usr/share/doc/elyterminal" "$work/$name/licenses"
tar -C "$work" -czf "$out/$name.tar.gz" "$name"
echo "linux-tar: $out/$name.tar.gz"
