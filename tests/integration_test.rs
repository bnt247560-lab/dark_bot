use dark_bot::models::job::JobStatus;

#[test]
fn crate_exports_core_modules_for_integration_tests() {
    assert_eq!(JobStatus::Pending, JobStatus::Pending);
}
