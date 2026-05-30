FROM lukemathwalker/cargo-chef:latest-rust-slim-trixie AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# Install before any COPY so this layer is never invalidated by source changes
RUN apt-get update && apt-get install -y pkg-config libssl-dev cmake mold g++ \
    && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=cargo-registry \
    --mount=type=cache,target=/app/target,id=vectoriser-target \
    RUSTFLAGS="-C link-arg=-fuse-ld=mold" \
    cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=cargo-registry \
    --mount=type=cache,target=/app/target,id=vectoriser-target \
    RUSTFLAGS="-C link-arg=-fuse-ld=mold" \
    cargo build --release --bin vectoriser \
    && cp /app/target/release/vectoriser /app/vectoriser-bin

FROM debian:trixie-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/vectoriser-bin /usr/local/bin/vectoriser
COPY .env.example .env

ENTRYPOINT ["vectoriser"]
CMD ["serve"]
