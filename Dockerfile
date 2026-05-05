FROM lukemathwalker/cargo-chef:latest-rust-1-alpine AS chef
WORKDIR /app
RUN apk add --no-cache musl-dev pkgconfig openssl-dev g++ make zlib-dev

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN apk add --no-cache musl-dev pkgconfig openssl-dev g++ make zlib-dev
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo run --release -- download-model
RUN cargo build --release --bin vectoriser

FROM alpine:latest AS runtime
WORKDIR /app

RUN apk add --no-cache \
    libgcc \
    libstdc++ \
    ca-certificates \
    openssl \
    zlib

COPY --from=builder /app/target/release/vectoriser /usr/local/bin/
COPY --from=builder /root/.cache/fastembed /root/.cache/fastembed
COPY .env.example .env

ENTRYPOINT ["vectoriser", "serve"]
