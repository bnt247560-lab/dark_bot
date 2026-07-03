use crate::app::AppState;
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct DashboardAuthQuery {
    token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DashboardSummary {
    pub status: &'static str,
    pub database: &'static str,
    pub redis: &'static str,
    pub workers: usize,
    pub users_total: i64,
    pub jobs_total: i64,
    pub queue: DashboardQueue,
    pub job_status_counts: Vec<DashboardStatusCount>,
    pub recent_failed_jobs: Vec<DashboardJob>,
}

#[derive(Debug, Serialize)]
pub struct DashboardQueue {
    pub pending: i64,
    pub processing: i64,
    pub dead: i64,
}

#[derive(Debug, Serialize)]
pub struct DashboardStatusCount {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct DashboardJob {
    pub id: Uuid,
    pub user_id: i64,
    pub status: String,
    pub progress: i32,
    pub updated_at: DateTime<Utc>,
    pub error_message: Option<String>,
}

fn authorized(state: &AppState, headers: &HeaderMap, query: &DashboardAuthQuery) -> bool {
    if !state.config.dashboard_enabled {
        return false;
    }

    let expected = state.config.dashboard_token.trim();
    if expected.is_empty() {
        return false;
    }

    let header_token = headers
        .get("x-dashboard-token")
        .and_then(|value| value.to_str().ok());

    let bearer_token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    header_token == Some(expected)
        || bearer_token == Some(expected)
        || query.token.as_deref() == Some(expected)
}

async fn summary(State(state): State<Arc<AppState>>, headers: HeaderMap, Query(query): Query<DashboardAuthQuery>) -> impl IntoResponse {
    if !authorized(&state, &headers, &query) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    match build_summary(&state).await {
        Ok(data) => Json(data).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("dashboard summary failed: {err}")).into_response(),
    }
}

async fn page(State(state): State<Arc<AppState>>, headers: HeaderMap, Query(query): Query<DashboardAuthQuery>) -> impl IntoResponse {
    if !authorized(&state, &headers, &query) {
        return (
            StatusCode::UNAUTHORIZED,
            Html("<h1>401 Unauthorized</h1><p>Pass <code>?token=...</code>, <code>Authorization: Bearer ...</code>, or <code>x-dashboard-token</code>.</p>".to_string()),
        )
            .into_response();
    }

    match build_summary(&state).await {
        Ok(data) => Html(render_dashboard(&data)).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("dashboard failed: {err}")).into_response(),
    }
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/dashboard", get(page))
        .route("/api/dashboard", get(summary))
}

async fn build_summary(state: &AppState) -> anyhow::Result<DashboardSummary> {
    let db_ok = state.db.health_check().await.is_ok();
    let queue_stats = state.queue.stats().await?;
    let (users_total, jobs_total) = state.db.get_stats().await.unwrap_or((0, 0));
    let job_status_counts = state
        .db
        .get_job_status_counts()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(status, count)| DashboardStatusCount { status, count })
        .collect();
    let recent_failed_jobs = state
        .db
        .get_recent_failed_jobs(10)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|job| DashboardJob {
            id: job.id,
            user_id: job.user_id,
            status: format!("{:?}", job.status),
            progress: job.progress,
            updated_at: job.updated_at,
            error_message: job.error_message,
        })
        .collect();

    Ok(DashboardSummary {
        status: if db_ok { "ok" } else { "degraded" },
        database: if db_ok { "ok" } else { "error" },
        redis: "ok",
        workers: state.config.worker_count,
        users_total,
        jobs_total,
        queue: DashboardQueue {
            pending: queue_stats.pending,
            processing: queue_stats.processing,
            dead: queue_stats.dead,
        },
        job_status_counts,
        recent_failed_jobs,
    })
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('`', "&#96;")
}

fn render_dashboard(data: &DashboardSummary) -> String {
    let status_rows = data.job_status_counts.iter().map(|row| {
        format!("<tr><td>{}</td><td>{}</td></tr>", escape_html(&row.status), row.count)
    }).collect::<Vec<_>>().join("\n");

    let failed_rows = if data.recent_failed_jobs.is_empty() {
        "<tr><td colspan=\"5\">No recent failed jobs.</td></tr>".to_string()
    } else {
        data.recent_failed_jobs.iter().map(|job| {
            format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}%</td><td>{}</td></tr>",
                job.id,
                job.user_id,
                escape_html(&job.status),
                job.progress,
                escape_html(job.error_message.as_deref().unwrap_or("")),
            )
        }).collect::<Vec<_>>().join("\n")
    };

    format!(r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<meta http-equiv="refresh" content="20" />
<title>Dark Bot Dashboard</title>
<style>
:root {{ color-scheme: dark; font-family: Inter, system-ui, Arial, sans-serif; }}
body {{ margin:0; background:#0b1020; color:#e5e7eb; }}
main {{ max-width:1100px; margin:0 auto; padding:28px; }}
h1 {{ margin:0 0 6px; font-size:30px; }}
.sub {{ color:#9ca3af; margin-bottom:24px; }}
.grid {{ display:grid; grid-template-columns:repeat(auto-fit, minmax(190px, 1fr)); gap:14px; margin-bottom:22px; }}
.card {{ background:#111827; border:1px solid #1f2937; border-radius:16px; padding:18px; box-shadow:0 10px 30px rgba(0,0,0,.25); }}
.label {{ color:#9ca3af; font-size:13px; }}
.value {{ font-size:28px; font-weight:750; margin-top:8px; }}
.ok {{ color:#34d399; }} .bad {{ color:#f87171; }}
table {{ width:100%; border-collapse:collapse; background:#111827; border-radius:16px; overflow:hidden; margin-bottom:22px; }}
th,td {{ padding:12px 14px; border-bottom:1px solid #1f2937; text-align:left; vertical-align:top; }}
th {{ background:#182136; color:#cbd5e1; }}
code {{ color:#93c5fd; word-break:break-all; }}
a {{ color:#93c5fd; }}
</style>
</head>
<body>
<main>
<h1>Dark Bot Dashboard</h1>
<div class="sub">Auto-refreshes every 20 seconds. Keep <code>DASHBOARD_TOKEN</code> private.</div>
<section class="grid">
<div class="card"><div class="label">System</div><div class="value {status_class}">{status}</div></div>
<div class="card"><div class="label">Database</div><div class="value {db_class}">{database}</div></div>
<div class="card"><div class="label">Redis</div><div class="value {redis_class}">{redis}</div></div>
<div class="card"><div class="label">Workers</div><div class="value">{workers}</div></div>
<div class="card"><div class="label">Users</div><div class="value">{users_total}</div></div>
<div class="card"><div class="label">Jobs</div><div class="value">{jobs_total}</div></div>
<div class="card"><div class="label">Queue Pending</div><div class="value">{pending}</div></div>
<div class="card"><div class="label">Queue Processing</div><div class="value">{processing}</div></div>
<div class="card"><div class="label">Dead Queue</div><div class="value">{dead}</div></div>
</section>
<h2>Job status counts</h2>
<table><thead><tr><th>Status</th><th>Count</th></tr></thead><tbody>{status_rows}</tbody></table>
<h2>Recent failed jobs</h2>
<table><thead><tr><th>Job</th><th>User</th><th>Status</th><th>Progress</th><th>Error</th></tr></thead><tbody>{failed_rows}</tbody></table>
<p class="sub">Raw JSON: <a href="/api/dashboard">/api/dashboard</a> · Metrics: <a href="/metrics">/metrics</a> · Health: <a href="/health">/health</a></p>
</main>
</body>
</html>"#,
        status_class = if data.status == "ok" { "ok" } else { "bad" },
        db_class = if data.database == "ok" { "ok" } else { "bad" },
        redis_class = if data.redis == "ok" { "ok" } else { "bad" },
        status = data.status,
        database = data.database,
        redis = data.redis,
        workers = data.workers,
        users_total = data.users_total,
        jobs_total = data.jobs_total,
        pending = data.queue.pending,
        processing = data.queue.processing,
        dead = data.queue.dead,
        status_rows = status_rows,
        failed_rows = failed_rows,
    )
}
