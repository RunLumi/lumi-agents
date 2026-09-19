//! Browser executor: maps authorized actions to browser operations
//! (spec 06 §6.9, spec 05 §5.7–5.10).
//!
//! The executor receives already-authorized actions only. Clicking a UI
//! element that sends/publishes/submits corresponds to a normalized action
//! that policy has already authorized (spec 06 §6.9) — this executor adds
//! no authority. Failures map onto the canonical taxonomy (spec 06 §6.14)
//! and grounding is always `SemanticLocator` (spec 05 §5.10).

use crate::protocol::{
    BrowserOp, ExpectTarget, TargetSelector, WorkerErrorBody, WorkerResponse, WorkerResult,
};
use crate::worker::{BrowserWorkerHandle, WorkerError};
use lumi_protocol::{
    ActionProposal, ErrorEnvelope, ExecutionResult, ExecutionStatus, FailureCategory, Grounding,
};

/// The browser executor bound to a live worker handle.
#[derive(Debug, Clone)]
pub struct BrowserExecutor {
    pub handle: BrowserWorkerHandle,
    request_timeout_ms: u64,
}

impl BrowserExecutor {
    #[must_use]
    pub fn new(handle: BrowserWorkerHandle) -> Self {
        Self {
            handle,
            request_timeout_ms: 60_000,
        }
    }

    #[must_use]
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.request_timeout_ms = timeout_ms;
        self
    }

    /// Translates an authorized action into a browser operation.
    ///
    /// Argument schema (workflow-defined, policy-authorized):
    /// - `op`: which browser operation (`navigate`|`click`|`fill`|...);
    /// - `target`: `{strategy, ...}` locator per spec 06 §6.5;
    /// - op-specific fields (`url`, `value`, `file_path`, `expect`...).
    ///
    /// # Errors
    /// [`BrowserExecutorError`] when the action does not carry a valid
    /// operation mapping — a workflow bug, not a browser failure.
    pub fn operation_for(action: &ActionProposal) -> Result<BrowserOp, BrowserExecutorError> {
        let get = |key: &str| action.arguments.get(key).cloned();
        let op = get("op")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| {
                BrowserExecutorError("arguments.op (browser operation) is required".to_owned())
            })?;
        let target = |key: &str| -> Result<TargetSelector, BrowserExecutorError> {
            let value = get(key)
                .or_else(|| get("target"))
                .ok_or_else(|| BrowserExecutorError(format!("arguments.{key} is required")))?;
            serde_json::from_value(value)
                .map_err(|e| BrowserExecutorError(format!("bad target selector: {e}")))
        };
        let timeout_ms = get("timeout_ms").and_then(|v| v.as_u64());

        match op.as_str() {
            "navigate" => {
                let url = get("url")
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .ok_or_else(|| BrowserExecutorError("arguments.url is required".to_owned()))?;
                Ok(BrowserOp::Navigate {
                    url,
                    wait_until: None,
                    trace_path: None,
                })
            }
            "read" | "read_page" => Ok(BrowserOp::Read {
                selector: target("target").ok(),
            }),
            "click" => Ok(BrowserOp::Click {
                target: target("target")?,
                timeout_ms,
            }),
            "fill" => {
                let value = get("value")
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .ok_or_else(|| {
                        BrowserExecutorError("arguments.value is required".to_owned())
                    })?;
                Ok(BrowserOp::Fill {
                    target: target("target")?,
                    value,
                    timeout_ms,
                })
            }
            "select" => {
                let value = get("value")
                    .and_then(|v| {
                        v.as_array().map(|items| {
                            items
                                .iter()
                                .filter_map(|i| i.as_str().map(str::to_owned))
                                .collect::<Vec<_>>()
                        })
                    })
                    .ok_or_else(|| {
                        BrowserExecutorError("arguments.value must be an array".to_owned())
                    })?;
                Ok(BrowserOp::Select {
                    target: target("target")?,
                    value,
                    timeout_ms,
                })
            }
            "check" => Ok(BrowserOp::Check {
                target: target("target")?,
                uncheck: get("uncheck").and_then(|v| v.as_bool()).unwrap_or(false),
                timeout_ms,
            }),
            "upload" => {
                let file_path = get("file_path")
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .ok_or_else(|| {
                        BrowserExecutorError("arguments.file_path is required".to_owned())
                    })?;
                Ok(BrowserOp::Upload {
                    target: target("target")?,
                    file_path,
                    timeout_ms,
                })
            }
            "download" => {
                let workspace_dir = get("workspace_dir")
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .ok_or_else(|| {
                        BrowserExecutorError("arguments.workspace_dir is required".to_owned())
                    })?;
                Ok(BrowserOp::Download {
                    trigger: target("target")?,
                    workspace_dir,
                    timeout_ms,
                })
            }
            "wait" => Ok(BrowserOp::WaitFor {
                selector: target("target").ok(),
                state: get("state").and_then(|v| v.as_str().map(str::to_owned)),
                timeout_ms,
            }),
            "screenshot" => Ok(BrowserOp::Screenshot {
                target: target("target").ok(),
                full_page: get("full_page").and_then(|v| v.as_bool()).unwrap_or(false),
                timeout_ms,
            }),
            "submit" => {
                let expect = get("expect").ok_or_else(|| {
                    BrowserExecutorError("arguments.expect is required for submit".to_owned())
                })?;
                let expect: ExpectTarget = serde_json::from_value(expect)
                    .map_err(|e| BrowserExecutorError(format!("bad expect selector: {e}")))?;
                Ok(BrowserOp::Submit {
                    target: target("target")?,
                    expect,
                    timeout_ms,
                })
            }
            other => Err(BrowserExecutorError(format!(
                "unsupported browser operation {other:?}"
            ))),
        }
    }

    /// Executes one authorized action against the worker.
    pub fn execute(&self, action: &ActionProposal) -> ExecutionResult {
        match self.execute_inner(action) {
            Ok(result) => result,
            Err(error) => ExecutionResult {
                action_id: action.action_id.clone(),
                task_id: action.task_id.clone(),
                run_id: action.run_id.clone(),
                status: ExecutionStatus::Failed,
                started_at: lumi_protocol::Timestamp::now(),
                ended_at: lumi_protocol::Timestamp::now(),
                grounding: Some(Grounding::SemanticLocator),
                observation_ids: vec![],
                error: Some(error),
            },
        }
    }

    #[allow(clippy::result_large_err)] // internal helper; boxed at the boundary
    fn execute_inner(&self, action: &ActionProposal) -> Result<ExecutionResult, ErrorEnvelope> {
        let op = Self::operation_for(action)
            .map_err(|e| ErrorEnvelope::new(FailureCategory::ModelFormat, e.0))?;
        let response = self
            .handle
            .request(
                op,
                std::time::Duration::from_millis(self.request_timeout_ms),
            )
            .map_err(|e| e.to_envelope())?;
        Ok(self.response_to_result(action, response))
    }

    fn response_to_result(
        &self,
        action: &ActionProposal,
        response: WorkerResponse,
    ) -> ExecutionResult {
        let base = |status: ExecutionStatus, error: Option<ErrorEnvelope>| ExecutionResult {
            action_id: action.action_id.clone(),
            task_id: action.task_id.clone(),
            run_id: action.run_id.clone(),
            status,
            started_at: lumi_protocol::Timestamp::now(),
            ended_at: lumi_protocol::Timestamp::now(),
            grounding: Some(Grounding::SemanticLocator),
            observation_ids: vec![],
            error,
        };
        if response.ok {
            // An explicit ambiguous submit result maps to AMBIGUOUS.
            if let Some(WorkerResult::Submit(submit)) = &response.result {
                if submit.ambiguous {
                    return base(
                        ExecutionStatus::Ambiguous,
                        Some(ErrorEnvelope::new(
                            FailureCategory::AmbiguousState,
                            submit.message.clone().unwrap_or_else(|| {
                                "submit outcome not observed within timeout".to_owned()
                            }),
                        )),
                    );
                }
            }
            base(ExecutionStatus::Success, None)
        } else {
            let envelope = response.error.unwrap_or_else(|| WorkerErrorBody {
                category: FailureCategory::BrowserState,
                message: "worker reported no error detail".to_owned(),
                native_code: None,
            });
            base(
                ExecutionStatus::Failed,
                Some(
                    ErrorEnvelope::new(envelope.category, envelope.message).with_detail(
                        serde_json::json!({
                            "native_code": envelope.native_code,
                        }),
                    ),
                ),
            )
        }
    }
}

/// Workflow/argument mapping errors (not browser failures).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserExecutorError(pub String);

/// Public helper: worker channel errors → canonical execution failure.
#[must_use]
pub fn browser_executor_error(error: &WorkerError) -> ErrorEnvelope {
    error.to_envelope()
}
