# Dark Bot 🚀

Professional Telegram video processing bot written in Rust.

## Features

- Accepts video links from TikTok, Instagram, YouTube, and other supported sites through `yt-dlp`.
- Accepts direct video uploads from Telegram.
- Stores every request as a PostgreSQL job.
- Processes jobs through Redis-backed workers.
- Uses FFmpeg for metadata removal and future video transformations.
- Supports `/status`, `/cancel`, `/history`, and `/stats`.
- Includes Docker and docker-compose for local/production deployment.

## Setup

1. Copy `.env.example` to `.env`.
2. Fill in `TELOXIDE_TOKEN`, `DATABASE_URL`, and `REDIS_URL`.
3. Start the stack:

```bash
docker-compose up --build
```

## Environment variables

```env
TELOXIDE_TOKEN=your_bot_token_here
DATABASE_URL=postgres://dark_bot:dark_bot@localhost:5432/dark_bot
REDIS_URL=redis://localhost:6379
LOG_LEVEL=info
STORAGE_PATH=./storage
FFMPEG_PATH=ffmpeg
FFPROBE_PATH=ffprobe
YT_DLP_PATH=yt-dlp
WORKER_COUNT=2
MAX_JOB_RETRIES=3
MAX_TELEGRAM_FILE_MB=512
```

## Worker pipeline

### URL jobs

1. User sends a supported URL.
2. Bot creates a PostgreSQL job with `source_url`.
3. Job id is pushed to Redis queue `dark_bot:jobs:pending`.
4. Worker downloads the video using `yt-dlp`.
5. Worker processes the file using FFmpeg.
6. Worker sends the result back to Telegram.
7. Job folder is cleaned after success.

### Telegram upload jobs

1. User uploads a video directly to Telegram.
2. Bot validates the configured file-size limit.
3. Bot creates a PostgreSQL job.
4. Bot downloads the Telegram file into `storage/temp/<job_id>/telegram_upload.<ext>`.
5. Bot updates `file_path` for the job.
6. Job id is pushed to Redis queue.
7. Worker skips `yt-dlp` and processes the already-downloaded file.
8. Worker sends the processed file back to the same user.
9. Job folder is cleaned after success.

## Job states

- `pending`
- `downloading`
- `processing`
- `uploading`
- `completed`
- `failed`
- `cancelled`

## Commands

- `/start` — registers/updates the Telegram user.
- `/help` — shows help.
- `/status` — shows the latest active job.
- `/cancel` — cancels active jobs for the current user.
- `/history` — shows recent jobs.
- `/stats` — shows basic platform stats.

## Notes

- Docker now uses Rust `1.82` because the Teloxide dependency chain requires a modern Rust compiler.
- Direct video-upload ingestion is now supported and uses the same production worker pipeline as URL jobs.

## Step 4 additions — real progress

This version replaces fixed progress numbers with live progress updates:

- `yt-dlp` is launched with `--newline` and parsed for download percentage.
- FFmpeg is launched with `-progress pipe:1 -nostats` and parsed through `out_time_ms`.
- Progress is scaled by pipeline stage:
  - Download: `5% → 40%`
  - Processing: `45% → 82%`
  - Uploading: `85%`
  - Completed: `100%`
- Progress is saved to PostgreSQL through `update_job_progress`.
- The bot edits one Telegram progress message instead of sending spam messages.
- `/status` reads the saved progress from the database.

Important note: progress accuracy depends on the source. Some websites or short FFmpeg copy operations may jump quickly from low percentage to complete, which is normal.

## Step 5 — Production hardening

This version adds operational hardening on top of the real-progress worker release:

- HTTP health server on `HEALTH_BIND` with `/health` and `/ready`.
- Docker `HEALTHCHECK` against the bot process, not only Postgres/Redis.
- Queue metrics in health response: pending, processing, dead.
- Startup recovery for jobs left in Redis `processing` after a crash.
- Startup recovery for database jobs stuck in downloading/processing/uploading.
- Graceful shutdown token for workers.
- Safer URL intake: maximum URL length and basic localhost/private self-target blocking.
- Exposes port `8080` in Docker Compose for health monitoring.

### Health endpoint

```bash
curl http://localhost:8080/health
```

Example response:

```json
{
  "status": "ok",
  "database": "ok",
  "redis": "ok",
  "queue": {
    "pending": 0,
    "processing": 0,
    "dead": 0
  }
}
```

### New environment variables

```env
HEALTH_BIND=0.0.0.0:8080
MAX_URL_LENGTH=2048
```

### Run

```bash
cp .env.example .env
# edit TELOXIDE_TOKEN, DATABASE_URL, REDIS_URL
docker compose up --build
```

## Step 6 — Telegram admin commands

This version adds protected Telegram admin commands. Configure allowed Telegram user ids through:

```env
ADMIN_USER_IDS=123456789,987654321
```

Admin commands:

- `/admin` — overall dashboard with users, jobs, queue and status counts.
- `/queue` — Redis queue metrics: pending, processing, dead.
- `/failed` — latest failed jobs from PostgreSQL and dead queue samples.
- `/retryfailed` — moves up to 20 jobs from the dead queue back to pending.

Security notes:

- Admin access is checked against the Telegram sender id, not the chat id.
- If `ADMIN_USER_IDS` is empty, all admin commands are denied.
- `/retryfailed` resets failed job records to `pending` before requeueing them.

---

## Step 7: Test Suite Baseline

This version adds the first real automated test layer so regressions are caught before deployment.

### Added

- `src/lib.rs` so the project can be tested as a library crate and not only as a binary.
- Unit/integration test coverage for:
  - Startup configuration validation.
  - PostgreSQL and Redis URL validation.
  - Admin ID parsing and authorization helper logic.
  - Storage directory creation.
  - Queue payload serialization/deserialization.
  - Retry attempt increment logic.
  - Job status JSON serialization.
- Public pure helpers for safer testing:
  - `Settings::validate()`
  - `Settings::ensure_storage_dirs()`
  - `QueueJob::new()`
  - `QueueJob::next_attempt()`
  - `QueueJob::to_json()`
  - `QueueJob::from_json()`

### Run tests locally

```bash
cargo test
```

### Run compile check locally

```bash
cargo check
```

### Important note

The current execution environment used to prepare this package does not include Rust/Cargo, so the tests were added and reviewed statically. Run `cargo test` locally or inside the Docker build environment before deploying.


---

## Step 8: S3 / Cloudflare R2 Object Storage

This version adds optional S3-compatible result storage without breaking local mode.

### What changed

- Added `ObjectStorage` service using the official AWS S3 SDK.
- Supports AWS S3, Cloudflare R2, MinIO, and most S3-compatible providers.
- Local Telegram delivery still works exactly as before.
- When object storage is enabled, each processed result is uploaded to:

```text
processed/<job_id>/<file_name>
```

- The bot sends the processed Telegram file and also sends a cloud URL message.
- Startup validation rejects incomplete storage configuration before the bot starts.
- Tests cover disabled storage mode and invalid storage configuration.

### Environment variables

```env
OBJECT_STORAGE_ENABLED=false
OBJECT_STORAGE_ENDPOINT=
OBJECT_STORAGE_REGION=auto
OBJECT_STORAGE_BUCKET=
OBJECT_STORAGE_ACCESS_KEY_ID=
OBJECT_STORAGE_SECRET_ACCESS_KEY=
OBJECT_STORAGE_PUBLIC_BASE_URL=
```

### Cloudflare R2 example

```env
OBJECT_STORAGE_ENABLED=true
OBJECT_STORAGE_ENDPOINT=https://<account-id>.r2.cloudflarestorage.com
OBJECT_STORAGE_REGION=auto
OBJECT_STORAGE_BUCKET=dark-bot-results
OBJECT_STORAGE_ACCESS_KEY_ID=your_r2_access_key
OBJECT_STORAGE_SECRET_ACCESS_KEY=your_r2_secret_key
OBJECT_STORAGE_PUBLIC_BASE_URL=https://cdn.example.com
```

### AWS S3 example

```env
OBJECT_STORAGE_ENABLED=true
OBJECT_STORAGE_ENDPOINT=
OBJECT_STORAGE_REGION=eu-central-1
OBJECT_STORAGE_BUCKET=dark-bot-results
OBJECT_STORAGE_ACCESS_KEY_ID=your_aws_access_key
OBJECT_STORAGE_SECRET_ACCESS_KEY=your_aws_secret_key
OBJECT_STORAGE_PUBLIC_BASE_URL=https://dark-bot-results.s3.eu-central-1.amazonaws.com
```

### Security note

Do not commit real object-storage credentials. Keep them only in production environment variables or your deployment provider secrets.

---

## Step 9: Observability — Prometheus and Grafana

This version adds production observability without changing the bot workflow.

### Added

- `/metrics` endpoint in Prometheus text format.
- Prometheus counters/gauges for:
  - completed jobs
  - failed jobs
  - retried jobs
  - worker events
  - pending/processing/dead queue depth
  - database health
  - Redis health
  - process uptime
- Optional Prometheus service through Docker Compose profile `monitoring`.
- Optional Grafana service with provisioned Prometheus datasource.
- Starter Grafana dashboard at `monitoring/grafana/dashboards/dark-bot.json`.

### Endpoints

```bash
curl http://localhost:8080/health
curl http://localhost:8080/ready
curl http://localhost:8080/metrics
```

### Run the bot only

```bash
docker compose up --build
```

### Run with monitoring

```bash
docker compose --profile monitoring up --build
```

Then open:

- Prometheus: `http://localhost:9090`
- Grafana: `http://localhost:3000`

Default Grafana credentials in local compose:

```text
admin / admin
```

Change these credentials before any public deployment.

### Important metrics

```text
dark_bot_queue_depth{queue="pending"}
dark_bot_queue_depth{queue="processing"}
dark_bot_queue_depth{queue="dead"}
dark_bot_jobs_total{status="completed"}
dark_bot_jobs_total{status="failed"}
dark_bot_jobs_total{status="retried"}
dark_bot_database_up
dark_bot_redis_up
dark_bot_uptime_seconds
```

### Production note

Keep `/metrics` private behind an internal network, reverse proxy allowlist, VPN, or cloud firewall. Do not expose Prometheus/Grafana publicly without authentication and TLS.

---

## Step 10 — Web Admin Dashboard

This version adds an optional lightweight web dashboard on the same HTTP server used for health and metrics.

### New routes

- `GET /dashboard` — human-readable admin dashboard.
- `GET /api/dashboard` — JSON dashboard summary for tools or custom UI.
- `GET /metrics` — Prometheus metrics from the previous step.
- `GET /health` / `GET /ready` — health endpoints.

### Security

The dashboard is disabled by default to avoid accidentally exposing operational data.

Enable it only after setting a long random token:

```env
DASHBOARD_ENABLED=true
DASHBOARD_TOKEN=replace_with_a_long_random_secret_at_least_24_chars
```

You can authenticate in one of three ways:

```bash
curl -H "x-dashboard-token: $DASHBOARD_TOKEN" http://localhost:8080/api/dashboard
curl -H "Authorization: Bearer $DASHBOARD_TOKEN" http://localhost:8080/api/dashboard
open "http://localhost:8080/dashboard?token=$DASHBOARD_TOKEN"
```

### What the dashboard shows

- System, database, and Redis status.
- Worker count.
- Total users.
- Total jobs.
- Pending, processing, and dead queue depth.
- Job counts by status.
- Last 10 failed jobs and error messages.

### Production note

Do not expose `HEALTH_BIND` publicly without a reverse proxy, firewall, VPN, or private network. If you expose `/dashboard`, always use HTTPS and a strong `DASHBOARD_TOKEN`.
