//! Browser worker contract fixtures (spec 06 §6.15).
//!
//! Runs against a deterministic fake worker process, proving the Rust-side
//! protocol: request correlation, failure normalization, ambiguity
//! handling, prompt-injection containment, and cancellation. The real
//! Playwright worker speaks the identical protocol; an end-to-end suite
//! with real browsers runs locally behind `LUMI_BROWSER_E2E=1`.

use lumi_browser::protocol::{BrowserOp, ExpectTarget, Locator, TargetSelector, WorkerResult};
use lumi_browser::{BrowserExecutor, BrowserWorkerConfig, BrowserWorkerHandle, WorkerError};
use lumi_protocol::{ExecutionStatus, FailureCategory, Grounding};
use std::time::Duration;

fn config(scenario: &str) -> BrowserWorkerConfig {
    let mut config = BrowserWorkerConfig::playwright("tests/fixtures/fake_worker.js");
    config.working_dir = Some(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    config.program = "node".to_owned();
    // Scenario rides as a script argument so parallel tests stay isolated.
    config.args = vec![scenario.to_owned()];
    config
}

fn spawn(scenario: &str) -> BrowserWorkerHandle {
    BrowserWorkerHandle::spawn(&config(scenario)).expect("fake worker spawns")
}

fn target(strategy: Locator) -> TargetSelector {
    TargetSelector {
        locator: strategy,
        nth: None,
    }
}

fn timeout() -> Duration {
    Duration::from_secs(10)
}

#[test]
fn navigate_and_correlate_responses() {
    let worker = spawn("navigate_ok");
    let response = worker
        .request(
            BrowserOp::Navigate {
                url: "https://portal.example.test/invoices".to_owned(),
                wait_until: None,
                trace_path: None,
            },
            timeout(),
        )
        .unwrap();
    assert!(response.ok);
    match response.result.unwrap() {
        WorkerResult::Json(value) => {
            assert_eq!(value["url"], "https://portal.example.test/invoices");
            assert_eq!(value["status"], 200);
        }
        other => panic!("expected JSON result, got {other:?}"),
    }
}

#[test]
fn missing_selector_maps_to_browser_selector_category() {
    let worker = spawn("selector_missing");
    let response = worker
        .request(
            BrowserOp::Click {
                target: target(Locator::TestId {
                    test_id: "nope".to_owned(),
                }),
                timeout_ms: Some(1_000),
            },
            timeout(),
        )
        .unwrap();
    assert!(!response.ok);
    let error = response.error.unwrap();
    assert_eq!(error.category, FailureCategory::BrowserSelector);
    assert_eq!(error.native_code.as_deref(), Some("LocatorNotFound"));
}

#[test]
fn stale_locator_maps_to_browser_state() {
    let worker = spawn("stale_locator");
    let response = worker
        .request(
            BrowserOp::Click {
                target: target(Locator::Role {
                    role: "button".to_owned(),
                    name: Some("Submit".to_owned()),
                }),
                timeout_ms: None,
            },
            timeout(),
        )
        .unwrap();
    assert_eq!(
        response.error.unwrap().category,
        FailureCategory::BrowserState
    );
}

#[test]
fn login_expiry_maps_to_auth_session() {
    let worker = spawn("login_expired");
    let response = worker
        .request(
            BrowserOp::Navigate {
                url: "https://portal.example.test/invoices".to_owned(),
                wait_until: None,
                trace_path: None,
            },
            timeout(),
        )
        .unwrap();
    assert_eq!(
        response.error.unwrap().category,
        FailureCategory::AuthSession
    );
}

#[test]
fn network_failure_maps_to_network_category() {
    let worker = spawn("network_down");
    let response = worker
        .request(
            BrowserOp::Navigate {
                url: "https://unreachable.example.test/".to_owned(),
                wait_until: None,
                trace_path: None,
            },
            timeout(),
        )
        .unwrap();
    assert_eq!(response.error.unwrap().category, FailureCategory::Network);
}

#[test]
fn submit_timeout_is_ambiguous_not_failed() {
    let worker = spawn("ambiguous_submit");
    let response = worker
        .request(
            BrowserOp::Submit {
                target: target(Locator::TestId {
                    test_id: "submit".to_owned(),
                }),
                expect: ExpectTarget {
                    selector: target(Locator::TestId {
                        test_id: "confirmation".to_owned(),
                    }),
                },
                timeout_ms: Some(1_000),
            },
            timeout(),
        )
        .unwrap();
    assert!(
        response.ok,
        "the submit op answers; ambiguity is in the result"
    );
    match response.result.unwrap() {
        WorkerResult::Submit(submit) => {
            assert!(submit.ambiguous);
            assert!(submit.message.as_deref().unwrap().contains("not observed"));
        }
        other => panic!("expected submit result, got {other:?}"),
    }
}

#[test]
fn submit_confirmed_is_success() {
    let worker = spawn("submit_ok");
    let response = worker
        .request(
            BrowserOp::Submit {
                target: target(Locator::TestId {
                    test_id: "submit".to_owned(),
                }),
                expect: ExpectTarget {
                    selector: target(Locator::TestId {
                        test_id: "confirmation".to_owned(),
                    }),
                },
                timeout_ms: Some(5_000),
            },
            timeout(),
        )
        .unwrap();
    match response.result.unwrap() {
        WorkerResult::Submit(submit) => assert!(!submit.ambiguous),
        other => panic!("expected submit result, got {other:?}"),
    }
}

#[test]
fn page_injection_text_is_data_never_instructions() {
    // Spec 06 §6.11: the worker returns page text verbatim as an
    // observation payload. It structurally CANNOT become an operation:
    // operations arrive from the policy-gated runtime only, and the
    // worker's op set is closed (no op named "email the database").
    let worker = spawn("injection_content");
    let response = worker
        .request(BrowserOp::Read { selector: None }, timeout())
        .unwrap();
    assert!(response.ok);
    match response.result.unwrap() {
        WorkerResult::Read(read) => {
            assert!(read.text.contains("Ignore all previous instructions"));
            assert!(read.text.contains("delete_all_invoices"));
            // The injection text is inert payload: the worker has no op
            // that could act on it, and the runtime gate decides what
            // happens next, not the page.
        }
        other => panic!("expected read result, got {other:?}"),
    }
}

#[test]
fn download_lands_in_controlled_workspace_metadata() {
    let worker = spawn("download_meta");
    let response = worker
        .request(
            BrowserOp::Download {
                trigger: target(Locator::TestId {
                    test_id: "download-report".to_owned(),
                }),
                workspace_dir: "/workspace/downloads".to_owned(),
                timeout_ms: None,
            },
            timeout(),
        )
        .unwrap();
    assert!(response.ok);
    match response.result.unwrap() {
        WorkerResult::Json(value) => {
            // Spec 06 §6.7: filename + size recorded, path inside the
            // controlled workspace dir.
            assert_eq!(value["suggested_filename"], "q3-report.csv");
            assert_eq!(value["size"], 2048);
            assert_eq!(value["path"], "/workspace/downloads/q3-report.csv");
        }
        other => panic!("expected JSON result, got {other:?}"),
    }
}

#[test]
fn worker_crash_maps_to_upstream_driver() {
    // The fixture answers the request with an UPSTREAM_DRIVER failure and
    // then exits. The handle must surface exactly one of: the normalized
    // failure response, or the worker-exit channel error.
    let worker = spawn("exit_on_request");
    match worker.request(BrowserOp::Read { selector: None }, timeout()) {
        Ok(response) => {
            assert!(!response.ok);
            assert_eq!(
                response.error.unwrap().category,
                FailureCategory::UpstreamDriver
            );
        }
        Err(error) => {
            assert!(matches!(
                error,
                WorkerError::WorkerExited | WorkerError::Io(_) | WorkerError::Timeout
            ));
            assert_eq!(
                error.to_envelope().category,
                FailureCategory::UpstreamDriver
            );
        }
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !worker.has_exited() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(worker.has_exited());
}

#[test]
fn cancel_mid_run_kills_worker_promptly() {
    // Spec 06 §6.15 cancel mid-run: kill() terminates the child; the next
    // request fails with WorkerExited.
    let worker = spawn("navigate_ok");
    worker.kill();
    assert!(worker.has_exited());
    let error = worker
        .request(
            BrowserOp::Read { selector: None },
            Duration::from_millis(500),
        )
        .expect_err("killed worker must not answer");
    assert!(matches!(
        error,
        WorkerError::WorkerExited | WorkerError::Io(_)
    ));
}

// ---------------------------------------------------------------------------
// Executor mapping (ActionProposal -> browser op -> ExecutionResult)
// ---------------------------------------------------------------------------

fn browser_action(op_args: serde_json::Value) -> lumi_protocol::ActionProposal {
    use lumi_protocol::{
        ActionId, ActionProposal, AuthenticationStrength, Capability, Principal, PrincipalId,
        PrincipalKind, ResourceRef, ResourceType, RiskClass, RunId, Target, TaskId, TenantId,
        Timestamp,
    };
    ActionProposal::builder(
        ActionId::parse("a-browser").unwrap(),
        TaskId::parse("task-browser").unwrap(),
        RunId::parse("run-browser").unwrap(),
        Principal {
            principal_id: PrincipalId::parse("u-1").unwrap(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            kind: PrincipalKind::Workflow,
            authenticated_at: Some(Timestamp::UNIX_EPOCH),
            authentication_strength: Some(AuthenticationStrength::DevicePossession),
        },
        Capability::well_known(lumi_protocol::capabilities::BROWSER_WRITE),
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::BROWSER_ORIGIN),
            id: "https://portal.example.test".to_owned(),
            sensitivity: None,
        },
        Target::canonical("https://portal.example.test"),
        "browser_automate",
        RiskClass::LocalWrite,
    )
    .arguments(op_args)
    .unwrap()
}

#[test]
fn executor_maps_authorized_action_to_verified_result() {
    let worker = spawn("submit_ok");
    let executor = BrowserExecutor::new(worker);
    let action = browser_action(serde_json::json!({
        "op": "submit",
        "target": {"strategy": "test_id", "test_id": "submit"},
        "expect": {"selector": {"strategy": "test_id", "test_id": "confirmation"}},
        "timeout_ms": 5_000,
    }));
    let result = executor.execute(&action);
    assert_eq!(result.status, ExecutionStatus::Success);
    // Determinism metadata (spec 05 §5.10): semantic locator grounding.
    assert_eq!(result.grounding, Some(Grounding::SemanticLocator));
}

#[test]
fn executor_maps_ambiguous_submit_to_ambiguous_status() {
    let worker = spawn("ambiguous_submit");
    let executor = BrowserExecutor::new(worker);
    let action = browser_action(serde_json::json!({
        "op": "submit",
        "target": {"strategy": "test_id", "test_id": "submit"},
        "expect": {"selector": {"strategy": "test_id", "test_id": "confirmation"}},
    }));
    let result = executor.execute(&action);
    assert_eq!(result.status, ExecutionStatus::Ambiguous);
    assert_eq!(
        result.error.as_ref().unwrap().category,
        FailureCategory::AmbiguousState
    );
}

#[test]
fn executor_rejects_actions_without_valid_operation_mapping() {
    let worker = spawn("navigate_ok");
    let executor = BrowserExecutor::new(worker);
    let action = browser_action(serde_json::json!({"something": "else"}));
    let result = executor.execute(&action);
    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(
        result.error.as_ref().unwrap().category,
        FailureCategory::ModelFormat,
        "workflow/argument bugs are MODEL_FORMAT, not browser failures"
    );
}
