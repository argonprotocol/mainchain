FROM rust:1.77 AS base

ARG SCCACHE_VERSION=0.7.7
ENV RUSTC_WRAPPER=/usr/local/bin/sccache SCCACHE_DIR=/home/root/.cache/sccache SCCACHE_CACHE_SIZE="10G" SQLX_OFFLINE=true

RUN apt-get update \
    && apt-get install -y --no-install-recommends clang libssl-dev llvm libudev-dev protobuf-compiler pkg-config \
    && apt-get autoremove -y


RUN ARCH=$(uname -m) && \
    SCCACHE_BINARY="sccache-v${SCCACHE_VERSION}-$(echo $ARCH | sed 's/x86_64/x86_64-unknown-linux-musl/;s/aarch64/aarch64-unknown-linux-musl/')" && \
    curl -L https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/${SCCACHE_BINARY}.tar.gz | tar xz -C /tmp && \
    mv /tmp/${SCCACHE_BINARY}/sccache /usr/local/bin/sccache && \
    chmod +x /usr/local/bin/sccache

RUN curl -L "https://github.com/rui314/mold/releases/download/v2.30.0/mold-2.30.0-$(uname -m)-linux.tar.gz" -o mold.tar.gz \
    && tar -C /usr/local --strip-components=1 --no-overwrite-dir -xzf mold.tar.gz && rm mold.tar.gz \
    && ln -sf /usr/local/bin/mold /usr/bin/ld \
    && ldconfig

WORKDIR /app
# Build application
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,mode=0777,target=/home/root/.cache/sccache \
    sccache --start-server && cargo build --locked --bin=ulx-node --bin=ulx-notary --features=fast-runtime
RUN  ls /app/target/debug && sccache --show-stats

FROM ubuntu:22.04 AS base_ubuntu

# Update and install dependencies
RUN apt-get update \
    && apt-get upgrade -y \
    && apt-get install -y --no-install-recommends \
       libssl-dev \
       libudev-dev \
       pkg-config \
       python3.10 \
       python3.10-dev \
    && apt-get autoremove -y \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# We do not need the Rust toolchain to run the binary!
# Runtime image for ulx-node
FROM base_ubuntu AS ulx-node
WORKDIR /app
COPY --from=base /app/target/debug/ulx-node /usr/local/bin/ulx-node
ENTRYPOINT ["/usr/local/bin/ulx-node"]

# Runtime image for ulx-notary
FROM base_ubuntu AS ulx-notary
WORKDIR /app
COPY --from=base /app/target/debug/ulx-notary /usr/local/bin/ulx-notary
ENTRYPOINT ["/usr/local/bin/ulx-notary"]
