//! Native executor: maps authorized actions to driver operations with
//! outcome classification, generation checks, and the no-silent
//! global-input rule (spec 07 §7.14–7.21).
//!
//! DELIVERED is never inferred from an OS call succeeding: the caller
//! supplies an **effect oracle** that must observe the intended
//! executor-level state change (§7.15). Oracle cannot confirm ⇒
//! NO_EFFECT; interrupted with unknown state ⇒ AMBIGUOUS.

use crate::driver::{DesktopDriver, EffectOracle, NativeOperation, NativeOutcome, RefusalReason};
use crate::session::{permissions_allow, SessionHandle};
use crate::target::{DeliveryMode, SemanticTarget};
use lumi_protocol::{
    ActionProposal, ErrorEnvelope, ExecutionResult, ExecutionStatus, FailureCategory, Grounding,
};

/// The native executor bound to a driver + session handle.
pub struct NativeExecutor<'a> {
    pub driver: &'a dyn DesktopDriver,
    pub handle: SessionHandle,
}

impl<'a> NativeExecutor<'a> {
    #[must_use]
    pub const fn new(driver: &'a dyn DesktopDriver, handle: SessionHandle) -> Self {
        Self { driver, handle }
    }

    /// Translates an authorized action into a native operation.
    ///
    /// Argument schema: `target` = semantic target; `delivery` =
    /// `semantic_background` (default) | `semantic_foreground` |
    /// `global_input` (requires explicit request + fresh policy
    /// evaluation, §7.20); `op` = focus_window | read_value | click |
    /// set_value | press_action; op fields (`value`, `action`).
    ///
    /// # Errors
    /// Workflow/argument bugs (MODEL_FORMAT at the result boundary).
    pub fn operation_for(action: &ActionProposal) -> Result<NativeOperation, String> {
        let get = |key: &str| action.arguments.get(key).cloned();
        let target_value =
            get("target").ok_or_else(|| "arguments.target is required".to_owned())?;
        let target: SemanticTarget = serde_json::from_value(target_value)
            .map_err(|e| format!("bad semantic target: {e}"))?;
        let delivery = match get("delivery") {
            Some(v) => serde_json::from_value::<DeliveryMode>(v)
                .map_err(|e| format!("bad delivery mode: {e}"))?,
            None => DeliveryMode::default(),
        };

        let op_name = get("op")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| "arguments.op is required".to_owned())?;
        match op_name.as_str() {
            "focus_window" => Ok(NativeOperation::FocusWindow { target }),
            "read_value" => Ok(NativeOperation::ReadValue { target }),
            "click" => Ok(NativeOperation::Click {
                target,
                mode: delivery,
            }),
            "set_value" => {
                let value = get("value")
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .ok_or_else(|| "arguments.value is required".to_owned())?;
                Ok(NativeOperation::SetValue { target, value })
            }
            "press_action" => {
                let press = get("action")
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .ok_or_else(|| "arguments.action is required".to_owned())?;
                Ok(NativeOperation::PressAction {
                    target,
                    action: press,
                })
            }
            other => Err(format!("unsupported native operation {other:?}")),
        }
    }

    /// Executes one authorized action with the caller's effect oracle.
    ///
    /// `authorized_delivery` is the delivery mode POLICY has authorized
    /// for this action (fresh evaluation, §7.20). An action requesting
    /// global input beyond that authorization is a security violation,
    /// not a fallback opportunity.
    pub fn execute(
        &self,
        action: &ActionProposal,
        authorized_delivery: DeliveryMode,
        oracle: EffectOracle<'_>,
    ) -> ExecutionResult {
        let base = |status: ExecutionStatus,
                    error: Option<ErrorEnvelope>,
                    outcome: Option<NativeOutcome>| ExecutionResult {
            action_id: action.action_id.clone(),
            task_id: action.task_id.clone(),
            run_id: action.run_id.clone(),
            status,
            started_at: lumi_protocol::Timestamp::now(),
            ended_at: lumi_protocol::Timestamp::now(),
            grounding: Some(Grounding::SemanticTarget),
            observation_ids: outcome
                .filter(|o| *o == NativeOutcome::Delivered)
                .map(|_| "native-effect-oracle".to_owned())
                .into_iter()
                .collect(),
            error,
        };

        // 1. Session state gate fails explicitly (§7.7).
        let state = match self.driver.session_state(&self.handle) {
            Ok(state) => state,
            Err(e) => {
                return base(
                    ExecutionStatus::Failed,
                    Some(ErrorEnvelope::new(e.category, e.message)),
                    None,
                )
            }
        };
        if !state.supports_automation() {
            return base(
                ExecutionStatus::Failed,
                Some(ErrorEnvelope::new(
                    FailureCategory::NativeSession,
                    format!("desktop session state {state:?} does not support automation"),
                )),
                None,
            );
        }

        // 2. Permission gate fails explicitly; never self-grants (§7.6).
        let permissions = match self.driver.permissions(&self.handle) {
            Ok(p) => p,
            Err(e) => {
                return base(
                    ExecutionStatus::Failed,
                    Some(ErrorEnvelope::new(e.category, e.message)),
                    None,
                )
            }
        };
        if !permissions_allow(&permissions) {
            return base(
                ExecutionStatus::Failed,
                Some(ErrorEnvelope::new(
                    FailureCategory::OsPermission,
                    "required OS permissions not granted; onboarding required",
                )),
                None,
            );
        }

        // 3. Map the action. Mapping failures are workflow bugs.
        let operation = match Self::operation_for(action) {
            Ok(operation) => operation,
            Err(detail) => {
                return base(
                    ExecutionStatus::Failed,
                    Some(ErrorEnvelope::new(FailureCategory::ModelFormat, detail)),
                    None,
                )
            }
        };

        // 4. No silent global-input fallback (§7.20): global input must be
        // BOTH explicitly requested by the action AND authorized by
        // policy. Anything else is a security violation.
        let requested_global = match &operation {
            NativeOperation::Click { mode, .. } => mode.is_global_input(),
            _ => false,
        };
        if requested_global && !authorized_delivery.is_global_input() {
            return base(
                ExecutionStatus::Failed,
                Some(ErrorEnvelope::new(
                    FailureCategory::SecurityViolation,
                    "global-input delivery not authorized by policy",
                )),
                None,
            );
        }

        // 5. Execute with the effect oracle deciding DELIVERED vs
        // NO_EFFECT vs AMBIGUOUS (§7.15).
        match self.driver.execute(&self.handle, operation, oracle) {
            Ok(result) => {
                let status = match result.outcome {
                    NativeOutcome::Delivered => ExecutionStatus::Success,
                    NativeOutcome::Refused => ExecutionStatus::Failed,
                    NativeOutcome::NoEffect => ExecutionStatus::Failed,
                    NativeOutcome::Ambiguous => ExecutionStatus::Ambiguous,
                    NativeOutcome::Error => ExecutionStatus::Failed,
                    NativeOutcome::Cancelled => ExecutionStatus::Cancelled,
                };
                let error = match (result.outcome, result.refusal, result.failure) {
                    (NativeOutcome::Refused, Some(reason), _) => Some(ErrorEnvelope::new(
                        FailureCategory::PolicyDenyExpected,
                        format!("native route refused: {reason:?}"),
                    )),
                    (NativeOutcome::NoEffect, _, _) => Some(ErrorEnvelope::new(
                        FailureCategory::Postcondition,
                        result
                            .oracle_observation
                            .unwrap_or_else(|| "effect oracle observed no change".to_owned()),
                    )),
                    (NativeOutcome::Error, _, Some(category)) => {
                        Some(ErrorEnvelope::new(category, "native operation failed"))
                    }
                    (NativeOutcome::Ambiguous, _, _) => Some(ErrorEnvelope::new(
                        FailureCategory::AmbiguousState,
                        "native effect state unknown after interruption",
                    )),
                    _ => None,
                };
                base(status, error, Some(result.outcome))
            }
            Err(e) => base(
                ExecutionStatus::Failed,
                Some(ErrorEnvelope::new(e.category, e.message)),
                None,
            ),
        }
    }
}

/// Re-exported refusal code: REFUSED must be returned BEFORE any mutation
/// (§7.15) — documented constant for driver implementors.
pub const REFUSED_PRECEDES_MUTATION: RefusalReason = RefusalReason::UnsupportedRoute;
