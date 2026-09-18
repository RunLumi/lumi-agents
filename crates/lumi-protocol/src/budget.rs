//! Budgets: bounded execution resources (spec 01 §1.13).

use crate::timestamp::Timestamp;
use serde::{Deserialize, Serialize};

/// Cost accumulator. Canonical unit is micro-USD (10⁻⁶ dollars) so all
/// accounting is integer math; model/token prices are converted at the
/// provider adapter boundary.
pub type MicroUsd = u64;

/// Bounds on one task/run. Budget exhaustion MUST produce a controlled
/// stop/exception (spec 01 §1.13), never a runaway loop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Budget {
    /// Wall-clock deadline; `None` means no time bound (still bounded by
    /// other dimensions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<Timestamp>,
    /// Maximum model spend in micro-USD.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_model_cost_micro_usd: Option<MicroUsd>,
    /// Maximum number of actions executed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_actions: Option<u32>,
    /// Maximum retry attempts per step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    /// Maximum vision/coordinate actions. Vision is a fallback; this cap
    /// keeps accidental vision-heavy loops visible and bounded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_vision_actions: Option<u32>,
    /// Maximum external side-effecting writes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_external_writes: Option<u32>,
}

/// What has been consumed against a [`Budget`]. Durable: persisted in
/// checkpoints so restarts cannot reset accounting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConsumedBudget {
    pub actions: u32,
    pub retries: u32,
    pub vision_actions: u32,
    pub external_writes: u32,
    pub model_cost_micro_usd: MicroUsd,
}

impl ConsumedBudget {
    /// Records one more action, optionally vision and/or external-write.
    pub fn record_action(&mut self, vision: bool, external_write: bool) {
        self.actions = self.actions.saturating_add(1);
        if vision {
            self.vision_actions = self.vision_actions.saturating_add(1);
        }
        if external_write {
            self.external_writes = self.external_writes.saturating_add(1);
        }
    }

    /// Records a retry.
    pub fn record_retry(&mut self) {
        self.retries = self.retries.saturating_add(1);
    }

    /// Records model spend.
    pub fn record_model_cost(&mut self, micro_usd: MicroUsd) {
        self.model_cost_micro_usd = self.model_cost_micro_usd.saturating_add(micro_usd);
    }
}

/// Which budget dimension was exhausted, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExhaustedDimension {
    Deadline,
    ModelCost,
    Actions,
    Retries,
    VisionActions,
    ExternalWrites,
}

impl Budget {
    /// Evaluates all dimensions. Returns the first exhausted dimension.
    #[must_use]
    pub fn check(&self, consumed: &ConsumedBudget, now: Timestamp) -> Option<ExhaustedDimension> {
        if let Some(deadline) = self.deadline {
            if now > deadline {
                return Some(ExhaustedDimension::Deadline);
            }
        }
        if let Some(max) = self.max_model_cost_micro_usd {
            if consumed.model_cost_micro_usd >= max {
                return Some(ExhaustedDimension::ModelCost);
            }
        }
        if let Some(max) = self.max_actions {
            if consumed.actions >= max {
                return Some(ExhaustedDimension::Actions);
            }
        }
        if let Some(max) = self.max_retries {
            if consumed.retries >= max {
                return Some(ExhaustedDimension::Retries);
            }
        }
        if let Some(max) = self.max_vision_actions {
            if consumed.vision_actions >= max {
                return Some(ExhaustedDimension::VisionActions);
            }
        }
        if let Some(max) = self.max_external_writes {
            if consumed.external_writes >= max {
                return Some(ExhaustedDimension::ExternalWrites);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> Budget {
        Budget {
            deadline: None,
            max_model_cost_micro_usd: Some(1_000),
            max_actions: Some(3),
            max_retries: Some(2),
            max_vision_actions: Some(1),
            max_external_writes: Some(2),
        }
    }

    #[test]
    fn exhaustion_is_detected_per_dimension() {
        let b = budget();
        let mut used = ConsumedBudget::default();
        assert_eq!(b.check(&used, Timestamp::UNIX_EPOCH), None);

        used.record_action(false, true);
        used.record_action(false, true);
        assert_eq!(
            b.check(&used, Timestamp::UNIX_EPOCH),
            Some(ExhaustedDimension::ExternalWrites)
        );
    }

    #[test]
    fn vision_cap_trips_before_action_cap() {
        let b = budget();
        let mut used = ConsumedBudget::default();
        used.record_action(true, false);
        assert_eq!(
            b.check(&used, Timestamp::UNIX_EPOCH),
            Some(ExhaustedDimension::VisionActions)
        );
    }

    #[test]
    fn deadline_is_enforced() {
        let b = Budget {
            deadline: Some(Timestamp::from_epoch(100, 0).unwrap()),
            ..Budget::default()
        };
        assert_eq!(
            b.check(
                &ConsumedBudget::default(),
                Timestamp::from_epoch(101, 0).unwrap()
            ),
            Some(ExhaustedDimension::Deadline)
        );
        assert_eq!(
            b.check(
                &ConsumedBudget::default(),
                Timestamp::from_epoch(100, 0).unwrap()
            ),
            None
        );
    }

    #[test]
    fn cost_accumulates_saturating() {
        let mut used = ConsumedBudget::default();
        used.record_model_cost(600);
        used.record_model_cost(600);
        assert_eq!(used.model_cost_micro_usd, 1_200);
        assert_eq!(
            budget().check(&used, Timestamp::UNIX_EPOCH),
            Some(ExhaustedDimension::ModelCost)
        );
    }
}
