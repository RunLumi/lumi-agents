//! Couple the Rust producer to the fixtures consumed by the real JS handler.

use lumi_browser::protocol::{SessionParams, WorkerRequest};

#[test]
fn real_worker_fixtures_round_trip_through_current_rust_wire_types() {
    let fixtures =
        include_str!("../../../workers/playwright/tests/fixtures/rust-worker-requests.jsonl");
    for line in fixtures.lines().filter(|line| !line.trim().is_empty()) {
        let expected: serde_json::Value = serde_json::from_str(line).unwrap();
        let request: WorkerRequest = serde_json::from_value(expected.clone()).unwrap();
        assert_eq!(serde_json::to_value(request).unwrap(), expected);
    }
}

#[test]
fn managed_browser_defaults_to_headless_without_profile_or_capture() {
    let session = SessionParams::default();
    assert!(session.headless);
    assert!(session.profile.is_none());
    assert!(session.trace.is_none());
    let decoded: SessionParams = serde_json::from_str("{}").unwrap();
    assert_eq!(decoded, session);
}
