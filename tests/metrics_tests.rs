use dark_bot::metrics::Metrics;

#[test]
fn metrics_render_contains_dark_bot_metrics() {
    let metrics = Metrics::new().expect("metrics registry should initialize");
    metrics.record_worker_event("started");
    metrics.record_job_completed();
    metrics.set_queue_depths(1, 2, 3);
    metrics.set_health(true, true);

    let rendered = metrics.render().expect("metrics should render");
    assert!(rendered.contains("dark_bot_jobs_total"));
    assert!(rendered.contains("dark_bot_queue_depth"));
    assert!(rendered.contains("dark_bot_database_up"));
    assert!(rendered.contains("dark_bot_redis_up"));
}
