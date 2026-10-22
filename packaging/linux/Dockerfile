FROM rust:1.95.0-bookworm AS rust
FROM ubuntu:22.04

COPY --from=rust /usr/local/cargo /usr/local/cargo
COPY --from=rust /usr/local/rustup /usr/local/rustup
COPY --from=rust /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
ENV PATH=/usr/local/cargo/bin:$PATH RUSTUP_HOME=/usr/local/rustup
RUN sed -i 's|http://|https://|g' /etc/apt/sources.list \
 && apt-get update -o APT::Update::Error-Mode=any -o Acquire::Retries=3 \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends -o Acquire::Retries=3 \
    build-essential ca-certificates curl pkg-config libwayland-dev libxkbcommon-dev \
    libxkbcommon-x11-dev libxcb1-dev libx11-xcb-dev libfontconfig-dev libfreetype-dev \
    dpkg-dev rpm patchelf file xz-utils desktop-file-utils shellcheck lintian xvfb \
 && rm -rf /var/lib/apt/lists/* \
 && rustup component add clippy rustfmt
WORKDIR /src
