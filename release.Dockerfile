FROM rust:1.86 AS base

RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake clang libssl-dev llvm libudev-dev protobuf-compiler pkg-config \
    && apt-get autoremove -y

WORKDIR /app
# Build application
COPY . .

ENV RUST_BACKTRACE=1 \
    SQLX_OFFLINE=1 \
    CARGO_TERM_PROGRESS_WHEN=always \
    CARGO_TERM_PROGRESS_WIDTH=500

RUN --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --locked --bin=argon-node --release \
    && install -Dm755 target/release/argon-node   /out/argon-node \
    && ls /out

FROM ubuntu:22.04 AS base_ubuntu

# Update and install dependencies
RUN apt-get update \
    && apt-get upgrade -y \
    && apt-get install -y --no-install-recommends curl jq  libssl-dev libudev-dev pkg-config ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=base /out/argon-node /usr/local/bin/argon-node
ENTRYPOINT ["/usr/local/bin/argon-node"]
