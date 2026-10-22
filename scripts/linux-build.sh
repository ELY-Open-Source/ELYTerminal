#!/bin/sh
# Build once on Ubuntu 22.04; all Linux formats use this binary.
set -eu
cd "$(dirname "$0")/.."
. scripts/linux-common.sh
case $(uname -m) in
    x86_64|aarch64|arm64) ;;
    *) echo "linux-build: only native x86_64 and ARM64 builds are supported" >&2; exit 1 ;;
esac
docker build --network "${LINUX_DOCKER_NETWORK:-default}" \
    --build-arg http_proxy --build-arg https_proxy --build-arg no_proxy \
    --build-arg HTTP_PROXY --build-arg HTTPS_PROXY --build-arg NO_PROXY \
    -t elyterminal-release packaging/linux
mkdir -p target/release target/linux-cargo target/linux-build
linux_run \
    -e CARGO_HOME=/src/target/linux-cargo -e CARGO_TARGET_DIR=/src/target/linux-build \
    -e CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" \
    elyterminal-release sh -eu -c '
        cargo build --release --locked
        install -m755 target/linux-build/release/elyterminal target/release/elyterminal
    '
