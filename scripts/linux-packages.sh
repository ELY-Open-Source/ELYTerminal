#!/bin/sh
# One build, four packages; packaging tools stay in the build container.
set -eu
cd "$(dirname "$0")/.."
. scripts/linux-common.sh
case $(uname -m) in
    x86_64) ;;
    *) echo "linux-packages: DEB, RPM and AppImage require x86_64; use linux-tar.sh for ARM64" >&2; exit 1 ;;
esac
scripts/linux-build.sh
linux_run \
    -e LINUX_BINARY=/src/target/release/elyterminal elyterminal-release sh -eu -c '
        scripts/linux-tar.sh
        scripts/linux-deb.sh
        scripts/linux-appimage.sh
        scripts/linux-rpm.sh
    '
