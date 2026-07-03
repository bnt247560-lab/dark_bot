use prometheus::{Encoder, GaugeVec, IntCounterVec, IntGauge, Registry, TextEncoder};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct Metrics {
    registry: Registry,
    started_at: Instant,
    jobs_total: IntCounterVec,
    job_failures_total: IntCounterVec,
    worker_events_total: IntCounterVec,
    queue_depth: GaugeVec,
    db_status: IntGauge,
    redis_status: IntGauge,
    uptime_seconds: IntGauge,
}

pub type SharedMetrics = Arc<Metrics>;

impl Metrics {
    pub fn new() -> anyhow::Result<Self> {
        let registry = Registry::new();

        let jobs_total = IntCounterVec::new(
            prometheus::Opts::new("dark_bot_jobs_total", "Total jobs processed by status"),
            &["status"],
        )?;
        let job_failures_total = IntCounterVec::new(
            prometheus::Opts::new("dark_bot_job_failures_total", "Total job failures by stage"),
            &["stage"],
        )?;
        let worker_events_total = IntCounterVec::new(
            prometheus::Opts::new("dark_bot_worker_events_total", "Worker lifecycle and processing events"),
            &["event"],
        )?;
        let queue_depth = GaugeVec::new(
            prometheus::Opts::new("dark_bot_queue_depth", "Redis queue depth by queue name"),
            &["queue"],
        )?;
        let db_status = IntGauge::new("dark_bot_database_up", "Database health status: 1 up, 0 down")?;
        let redis_status = IntGauge::new("dark_bot_redis_up", "Redis health status: 1 up, 0 down")?;
        let uptime_seconds = IntGauge::new("dark_bot_uptime_seconds", "Bot process uptime in seconds")?;

        registry.register(Box::new(jobs_total.clone()))?;
        registry.register(Box::new(job_failures_total.clone()))?;
        registry.register(Box::new(worker_events_total.clone()))?;
        registry.register(Box::new(queue_depth.clone()))?;
        registry.register(Box::new(db_status.clone()))?;
        registry.register(Box::new(redis_status.clone()))?;
        registry.register(Box::new(uptime_seconds.clone()))?;

        Ok(Self {
            registry,
            started_at: Instant::now(),
            jobs_total,
            job_failures_total,
            worker_events_total,
            queue_depth,
            db_status,
            redis_status,
            uptime_seconds,
        })
    }

    pub fn record_job_completed(&self) {
        self.jobs_total.with_label_values(&["completed"]).inc();
    }

    pub fn record_job_failed(&self, stage: &'static str) {
        self.jobs_total.with_label_values(&["failed"]).inc();
        self.job_failures_total.with_label_values(&[stage]).inc();
    }

    pub fn record_job_retried(&self) {
        self.jobs_total.with_label_values(&["retried"]).inc();
    }

    pub fn record_worker_event(&self, event: &'static str) {
        self.worker_events_total.with_label_values(&[event]).inc();
    }

    pub fn set_queue_depths(&self, pending: i64, processing: i64, dead: i64) {
        self.queue_depth.with_label_values(&["pending"]).set(pending as f64);
        self.queue_depth.with_label_values(&["processing"]).set(processing as f64);
        self.queue_depth.with_label_values(&["dead"]).set(dead as f64);
    }

    pub fn set_health(&self, database_up: bool, redis_up: bool) {
        self.db_status.set(if database_up { 1 } else { 0 });
        self.redis_status.set(if redis_up { 1 } else { 0 });
        self.uptime_seconds.set(self.started_at.elapsed().as_secs() as i64);
    }

    pub fn render(&self) -> anyhow::Result<String> {
        self.uptime_seconds.set(self.started_at.elapsed().as_secs() as i64);
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}
