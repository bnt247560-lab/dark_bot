use crate::app::AppState;
use axum::{extract::State, http::header, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub database: &'static str,
    pub redis: &'static str,
    pub queue: QueueHealth,
}

#[derive(Debug, Serialize)]
pub struct QueueHealth {
    pub pending: i64,
    pub processing: i64,
    pub dead: i64,
}

async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db_ok = state.db.health_check().await.is_ok();
    let queue_stats = state.queue.stats().await;

    match queue_stats {
        Ok(stats) => {
            state.metrics.set_health(db_ok, true);
            state.metrics.set_queue_depths(stats.pending, stats.processing, stats.dead);
            let status = if db_ok { "ok" } else { "degraded" };
            Json(HealthResponse {
                status,
                database: if db_ok { "ok" } else { "error" },
                redis: "ok",
                queue: QueueHealth {
                    pending: stats.pending,
                    processing: stats.processing,
                    dead: stats.dead,
                },
            })
        }
        Err(_) => {
            state.metrics.set_health(db_ok, false);
            Json(HealthResponse {
                status: "degraded",
                database: if db_ok { "ok" } else { "error" },
                redis: "error",
                queue: QueueHealth { pending: -1, processing: -1, dead: -1 },
            })
        }
    }
}

async fn metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db_ok = state.db.health_check().await.is_ok();
    match state.queue.stats().await {
        Ok(stats) => {
            state.metrics.set_health(db_ok, true);
            state.metrics.set_queue_depths(stats.pending, stats.processing, stats.dead);
        }
        Err(_) => {
            state.metrics.set_health(db_ok, false);
        }
    }

    match state.metrics.render() {
        Ok(body) => ([(header::CONTENT_TYPE, "text/plain; version=0.0.4")], body).into_response(),
        Err(err) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to render metrics: {err}"),
        )
            .into_response(),
    }
}

pub async fn serve(state: Arc<AppState>, bind: String) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(health))
        .route("/metrics", get(metrics))
        .merge(crate::dashboard::routes())
        .with_state(state);

    let addr: SocketAddr = bind.parse()?;
    tracing::info!(%addr, "Health and metrics server started");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
