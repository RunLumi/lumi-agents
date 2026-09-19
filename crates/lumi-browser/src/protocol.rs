//! Browser operation protocol (spec 06 §6.4–6.8).
//!
//! The operation set is CLOSED: the worker cannot be asked to evaluate
//! page-supplied instructions, only these structured operations.

use lumi_protocol::FailureCategory;
use serde::{Deserialize, Serialize};

/// Locator strategy in spec 06 §6.5 preference order. Externally-tagged
/// serde keeps exactly one strategy per target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "strategy")]
pub enum Locator {
    /// First-party stable test/data id.
    TestId { test_id: String },
    /// Accessibility role + optional accessible name.
    Role { role: String, name: Option<String> },
    /// Form label text.
    Label { label: String, exact: bool },
    /// Visible text with stable context.
    Text { text: String, exact: bool },
    /// CSS/XPath only when semantic strategies are unavailable (§6.5.4).
    Css { css: String },
}

impl Locator {
    /// Numeric preference rank for telemetry (lower = more semantic).
    #[must_use]
    pub const fn preference_rank(&self) -> u8 {
        match self {
            Self::TestId { .. } => 0,
            Self::Role { .. } => 1,
            Self::Label { .. } => 2,
            Self::Text { .. } => 3,
            Self::Css { .. } => 4,
        }
    }
}

/// A target element for interactions. The locator is flattened onto the
/// wire: `{"strategy": "test_id", "test_id": "submit"}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetSelector {
    #[serde(flatten)]
    pub locator: Locator,
    /// nth match when a locator is ambiguous; defaults to first. Ambiguous
    /// locators SHOULD be avoided; the worker fails with
    /// BROWSER_SELECTOR on strict-mode violations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nth: Option<u32>,
}

/// Browser profile/auth mode (spec 06 §6.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum ProfileMode {
    /// Clean, isolated managed context (default; no stored state).
    Isolated,
    /// Attach to an existing user profile. Requires the policy core's
    /// explicit approval; the worker rejects it otherwise.
    Existing {
        /// Proof the policy core authorized this attachment.
        policy_approved: bool,
    },
}

/// Trace retention policy (spec 06 §6.12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TracePolicy {
    /// No trace (production default).
    Off,
    /// Retain Playwright trace at the given path (staging/test only).
    OnDemand,
}

/// Closed browser operation set (spec 06 §6.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "op")]
pub enum BrowserOp {
    Navigate {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wait_until: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        trace_path: Option<String>,
    },
    /// Read page/DOM text for observation and verification (§6.13).
    Read {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<TargetSelector>,
    },
    Click {
        target: TargetSelector,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    Fill {
        target: TargetSelector,
        value: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    Select {
        target: TargetSelector,
        /// Option values/labels to select.
        value: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    Check {
        target: TargetSelector,
        #[serde(default)]
        uncheck: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    /// Upload requires the exact workspace file path (§6.8).
    Upload {
        target: TargetSelector,
        file_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    /// Download into the controlled workspace dir (§6.7).
    Download {
        trigger: TargetSelector,
        workspace_dir: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    WaitFor {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<TargetSelector>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    /// Selective screenshot (element) or viewport/full-page on explicit
    /// request only (§6.12, spec 11 §11.5).
    Screenshot {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<TargetSelector>,
        #[serde(default)]
        full_page: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    /// Click a submit control and wait for an expected post-state within
    /// the timeout. Unobserved outcome => AMBIGUOUS_STATE (§6.14,
    /// spec 02 §2.7) — never silent success or failure.
    Submit {
        target: TargetSelector,
        expect: ExpectTarget,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    StopTrace,
    Close,
}

/// The expected post-state of a submit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpectTarget {
    pub selector: TargetSelector,
}

/// Result payload of a submit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubmitResult {
    pub ambiguous: bool,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Result payload of a read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadResult {
    pub text: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Worker request envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerRequest {
    pub id: String,
    #[serde(flatten)]
    pub op: BrowserOp,
    /// Per-session params (headless, profile, trace) ride the FIRST
    /// request's `session` block implicitly via params; kept explicit for
    /// fixture determinism.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionParams>,
}

/// Session-level configuration supplied once at worker start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionParams {
    #[serde(default = "default_true")]
    pub headless: bool,
    pub profile: Option<ProfileMode>,
    pub trace: Option<TracePolicy>,
}

impl Default for SessionParams {
    fn default() -> Self {
        Self {
            headless: true,
            profile: None,
            trace: None,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Worker result payload per op (loose JSON for observation fields).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorkerResult {
    Submit(SubmitResult),
    Read(ReadResult),
    /// Generic JSON object result (navigate/download/screenshot/…).
    Json(serde_json::Value),
    Empty {},
}

/// Worker response envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerResponse {
    pub id: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<WorkerResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<WorkerErrorBody>,
}

/// Normalized worker error body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerErrorBody {
    pub category: FailureCategory,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locator_preference_order_matches_spec() {
        let ordered = [
            Locator::TestId {
                test_id: "submit".to_owned(),
            },
            Locator::Role {
                role: "button".to_owned(),
                name: Some("Save draft".to_owned()),
            },
            Locator::Label {
                label: "Email".to_owned(),
                exact: false,
            },
            Locator::Text {
                text: "Continue".to_owned(),
                exact: true,
            },
            Locator::Css {
                css: "#fallback".to_owned(),
            },
        ];
        for pair in ordered.windows(2) {
            assert!(pair[0].preference_rank() < pair[1].preference_rank());
        }
    }

    #[test]
    fn op_serialization_uses_tagged_ops() {
        let op = BrowserOp::Click {
            target: TargetSelector {
                locator: Locator::TestId {
                    test_id: "submit".to_owned(),
                },
                nth: None,
            },
            timeout_ms: Some(5_000),
        };
        let json = serde_json::to_value(&WorkerRequest {
            id: "r1".to_owned(),
            op,
            session: None,
        })
        .unwrap();
        assert_eq!(json["op"], "click");
        assert_eq!(json["target"]["strategy"], "test_id");
    }

    #[test]
    fn unapproved_profile_attachment_is_representable_and_rejected_upstream() {
        let mode = ProfileMode::Existing {
            policy_approved: false,
        };
        let json = serde_json::to_string(&mode).unwrap();
        assert!(json.contains("\"policy_approved\":false"));
    }
}
