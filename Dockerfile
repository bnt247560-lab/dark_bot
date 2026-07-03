FROM rust:1.82-slim-bookworm AS builder

WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml ./
COPY migrations ./migrations
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg python3 python3-pip ca-certificates curl \
    && pip3 install --break-system-packages --no-cache-dir yt-dlp \
    && useradd -m -u 10001 appuser \
    && mkdir -p /app/storage/downloads /app/storage/processed /app/storage/temp /app/logs \
    && chown -R appuser:appuser /app \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/dark_bot /usr/local/bin/dark_bot
USER appuser

EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --retries=3 CMD curl -fsS http://127.0.0.1:8080/health || exit 1

CMD ["dark_bot"]
