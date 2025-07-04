FROM rust:1.86 AS base

RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake clang libssl-dev llvm libudev-dev protobuf-compiler pkg-config \
    && apt-get autoremove -y

WORKDIR /app
# Build application
COPY . .

ENV RUST_BACKTRACE=1 \
    SQLX_OFFLINE=1

RUN --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --locked --bin=argon-node --bin=argon-notary --bin=argon-oracle --features=simulated-prices \
    # copy artefacts *out of* the ephemeral cache
    && install -Dm755 target/debug/argon-node   /out/argon-node \
    && install -Dm755 target/debug/argon-notary /out/argon-notary \
    && install -Dm755 target/debug/argon-oracle /out/argon-oracle \
    && ls /out

FROM ubuntu:22.04 AS base_ubuntu

# Update and install dependencies
RUN apt-get update \
    && apt-get upgrade -y \
    && apt-get install -y --no-install-recommends curl jq  libssl-dev libudev-dev pkg-config ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# We do not need the Rust toolchain to run the binary!
# Runtime image for argon-node
FROM base_ubuntu AS argon-node
WORKDIR /app
COPY --from=base /out/argon-node /usr/local/bin/argon-node
ENTRYPOINT ["/usr/local/bin/argon-node"]

# Runtime image for argon-notary
FROM base_ubuntu AS argon-notary
WORKDIR /app
COPY --from=base /out/argon-notary /usr/local/bin/argon-notary
ENTRYPOINT ["/usr/local/bin/argon-notary"]

# Runtime image for oracle
FROM base_ubuntu AS oracle
WORKDIR /app
COPY --from=base /out/argon-oracle /usr/local/bin/argon-oracle
ENTRYPOINT ["/usr/local/bin/argon-oracle"]
