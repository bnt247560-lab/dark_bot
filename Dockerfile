FROM rust:1-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
COPY migrations ./migrations
COPY src ./src

RUN cargo build --release


FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg \
    python3 \
    python3-pip \
    ca-certificates \
    curl \
    unzip \
    && curl -L -o /tmp/deno.zip https://github.com/denoland/deno/releases/download/v2.4.5/deno-x86_64-unknown-linux-gnu.zip \
    && unzip /tmp/deno.zip -d /usr/local/bin \
    && chmod +x /usr/local/bin/deno \
    && rm /tmp/deno.zip \
    && pip3 install --break-system-packages --no-cache-dir -U "yt-dlp[default]" \
    && useradd -m -u 10001 appuser \
    && mkdir -p /app/storage/downloads /app/storage/processed /app/storage/temp /app/logs \
    && chown -R appuser:appuser /app \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/dark_bot /usr/local/bin/dark_bot

USER appuser

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --retries=3 CMD curl -fsS http://127.0.0.1:8080/health || exit 1

CMD ["dark_bot"]
