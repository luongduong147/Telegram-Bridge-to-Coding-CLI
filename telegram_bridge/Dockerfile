FROM rust:1.95-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    mkdir src/handler && touch src/handler/mod.rs
RUN cargo build --release 2>/dev/null || true
COPY src/ src/
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y tmux ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/telegram_bridge /usr/local/bin/telegram_bridge
WORKDIR /workspace
ENTRYPOINT ["telegram_bridge"]
