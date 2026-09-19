//! Bounded retries with classified recovery (spec 02 §2.11–2.12,
//! spec 18 §18.3–18.8).
//!
//! Unlimited retry loops are forbidden. Only retryable classes auto-retry;
//! rate limits and network errors back off exponentially with jitter;
//! ambiguous consequential side effects never retry automatically.

use lumi_protocol::{ErrorEnvelope, FailureCategory, RecoveryAction, RetryClass, Timestamp};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Backoff schedule: bounded exponential with full jitter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Backoff {
    pub initial_ms: u64,
    pub max_ms: u64,
    /// Multiplier per attempt in tenths (e.g. 20 = 2.0x) to keep integer
    /// math exact.
    pub multiplier_tenths: u32,
}

impl Backoff {
    pub const DEFAULT: Self = Self {
        initial_ms: 500,
        max_ms: 30_000,
        multiplier_tenths: 20,
    };

    /// Nominal (pre-jitter) delay for an attempt number (0-based).
    #[must_use]
    pub fn nominal_delay(&self, attempt: u32) -> Duration {
        let mut delay_ms = self.initial_ms;
        for _ in 0..attempt.min(32) {
            delay_ms = delay_ms.saturating_mul(u64::from(self.multiplier_tenths)) / 10;
            if delay_ms >= self.max_ms {
                return Duration::from_millis(self.max_ms);
            }
        }
        Duration::from_millis(delay_ms.min(self.max_ms))
    }

    /// Applies full jitter: uniform in `[0, nominal)`. `random01` is in
    /// `[0,1)` so tests can be deterministic.
    #[must_use]
    pub fn jittered(&self, attempt: u32, random01: f64) -> Duration {
        let nominal = self.nominal_delay(attempt);
        let jittered_ms = (nominal.as_millis() as f64 * random01.clamp(0.0, 0.999_999)) as u64;
        Duration::from_millis(jittered_ms.min(self.max_ms))
    }
}

/// Bounded retry budget (spec 02 §2.12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub backoff: Backoff,
    /// Maximum attempts for one step/action (1 = no retries).
    pub max_attempts: u32,
    /// Overall deadline; retries past it escalate to the human queue.
    pub deadline: Option<Timestamp>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            backoff: Backoff::DEFAULT,
            max_attempts: 3,
            deadline: None,
        }
    }
}

/// Mutable retry accounting for one step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RetryState {
    pub attempts: u32,
    pub next_allowed_at: Option<Timestamp>,
}

/// What the runtime should do after a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryDecision {
    /// Sleep the given duration, then retry the same action.
    RetryAfter(Duration),
    /// Re-observe external state before retrying (stale-state class).
    ReobserveThenRetry(Duration),
    /// Refresh authentication, then retry once.
    RefreshAuthThenRetry,
    /// Verify the external postcondition first; automatic retry is
    /// forbidden until the journal resolves the ambiguity.
    VerifyBeforeRetry,
    /// Ask a human for approval (approval-timeout paths).
    RequireApproval,
    /// Stop retrying; route to the human exception queue with context.
    EscalateToHuman,
    /// Terminal failure (e.g. policy bug, user cancel, budget exhausted).
    Terminal,
}

/// Drives classified retry decisions within a bounded budget.
#[derive(Debug, Clone)]
pub struct RetryController {
    pub policy: RetryPolicy,
    pub state: RetryState,
}

impl RetryController {
    #[must_use]
    pub fn new(policy: RetryPolicy) -> Self {
        Self {
            policy,
            state: RetryState::default(),
        }
    }

    /// Decides recovery for one failure envelope, enforcing the retry
    /// budget (spec 18 §18.3–18.4).
    pub fn on_failure(&mut self, error: &ErrorEnvelope, now: Timestamp) -> RetryDecision {
        self.state.attempts = self.state.attempts.saturating_add(1);
        if self.state.attempts > self.policy.max_attempts {
            return RetryDecision::EscalateToHuman;
        }
        if let Some(deadline) = self.policy.deadline {
            if now > deadline {
                return RetryDecision::EscalateToHuman;
            }
        }
        let attempt = self.state.attempts.saturating_sub(1);
        let backoff = self.policy.backoff.jittered(attempt, 0.5);
        self.state.next_allowed_at = now.checked_add(backoff);
        match error.recovery {
            RecoveryAction::RetryImmediate => RetryDecision::RetryAfter(Duration::ZERO),
            RecoveryAction::RetryBackoff => RetryDecision::RetryAfter(backoff),
            RecoveryAction::RefreshAuth => RetryDecision::RefreshAuthThenRetry,
            RecoveryAction::Reobserve => RetryDecision::ReobserveThenRetry(backoff),
            RecoveryAction::RequireApproval => RetryDecision::RequireApproval,
            RecoveryAction::RequireUser => RetryDecision::EscalateToHuman,
            RecoveryAction::Terminal => RetryDecision::Terminal,
            RecoveryAction::AmbiguousNoRetry => RetryDecision::VerifyBeforeRetry,
        }
    }

    /// True when the budget is exhausted.
    #[must_use]
    pub const fn exhausted(&self) -> bool {
        self.state.attempts >= self.policy.max_attempts
    }
}

/// Convenience: the retry class implied by a failure category.
#[must_use]
pub const fn class_for(category: FailureCategory) -> RetryClass {
    RetryClass::from_failure(category)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(category: FailureCategory) -> ErrorEnvelope {
        ErrorEnvelope::new(category, "fixture failure")
    }

    fn ts(secs: i64) -> Timestamp {
        Timestamp::from_epoch(secs, 0).unwrap()
    }

    #[test]
    fn rate_limit_backs_off_exponentially_within_budget() {
        let mut controller = RetryController::new(RetryPolicy::default());
        let d1 = match controller.on_failure(&envelope(FailureCategory::ProviderRateLimit), ts(0)) {
            RetryDecision::RetryAfter(d) => d,
            other => panic!("expected RetryAfter, got {other:?}"),
        };
        let d2 = match controller.on_failure(&envelope(FailureCategory::ProviderRateLimit), ts(1)) {
            RetryDecision::RetryAfter(d) => d,
            other => panic!("expected RetryAfter, got {other:?}"),
        };
        assert!(d1 < d2, "backoff should grow: {d1:?} then {d2:?}");
        assert!(d2 <= Duration::from_millis(30_000));
    }

    #[test]
    fn budget_exhaustion_escalates_instead_of_looping() {
        let mut controller = RetryController::new(RetryPolicy {
            max_attempts: 2,
            ..RetryPolicy::default()
        });
        let _ = controller.on_failure(&envelope(FailureCategory::Network), ts(0));
        let _ = controller.on_failure(&envelope(FailureCategory::Network), ts(1));
        assert!(controller.exhausted());
        assert_eq!(
            controller.on_failure(&envelope(FailureCategory::Network), ts(2)),
            RetryDecision::EscalateToHuman
        );
    }

    #[test]
    fn ambiguous_side_effect_never_auto_retries() {
        let mut controller = RetryController::new(RetryPolicy::default());
        let decision = controller.on_failure(&envelope(FailureCategory::AmbiguousState), ts(0));
        assert_eq!(decision, RetryDecision::VerifyBeforeRetry);
        let decision = controller.on_failure(&envelope(FailureCategory::Postcondition), ts(1));
        assert_eq!(decision, RetryDecision::VerifyBeforeRetry);
    }

    #[test]
    fn stale_state_reobserves() {
        let mut controller = RetryController::new(RetryPolicy::default());
        let decision = controller.on_failure(&envelope(FailureCategory::BrowserSelector), ts(0));
        assert!(matches!(decision, RetryDecision::ReobserveThenRetry(_)));
    }

    #[test]
    fn auth_failures_request_refresh() {
        let mut controller = RetryController::new(RetryPolicy::default());
        assert_eq!(
            controller.on_failure(&envelope(FailureCategory::AuthSession), ts(0)),
            RetryDecision::RefreshAuthThenRetry
        );
    }

    #[test]
    fn terminal_categories_terminate() {
        for category in [
            FailureCategory::PolicyBug,
            FailureCategory::UserCancel,
            FailureCategory::BudgetExceeded,
            FailureCategory::SecurityViolation,
        ] {
            let mut controller = RetryController::new(RetryPolicy::default());
            assert_eq!(
                controller.on_failure(&envelope(category), ts(0)),
                RetryDecision::Terminal,
                "{category:?}"
            );
        }
    }

    #[test]
    fn deadline_stops_retries() {
        let mut controller = RetryController::new(RetryPolicy {
            deadline: Some(ts(10)),
            ..RetryPolicy::default()
        });
        assert_eq!(
            controller.on_failure(&envelope(FailureCategory::Network), ts(11)),
            RetryDecision::EscalateToHuman
        );
    }

    #[test]
    fn backoff_nominal_and_jitter_bounds() {
        let b = Backoff::DEFAULT;
        assert_eq!(b.nominal_delay(0), Duration::from_millis(500));
        assert_eq!(b.nominal_delay(1), Duration::from_millis(1000));
        assert_eq!(b.nominal_delay(2), Duration::from_millis(2000));
        // Jitter stays within [0, nominal].
        let j = b.jittered(2, 0.999);
        assert!(j <= b.nominal_delay(2));
        assert_eq!(b.jittered(2, 0.0), Duration::ZERO);
    }
}
