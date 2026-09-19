//! Side-effect journal: the crash-recovery truth source (spec 02 §2.6–2.7,
//! spec 18 §18.4, §18.11).
//!
//! Every consequential action gets a journal record with lifecycle
//! `Proposed -> (Succeeded | Failed | Ambiguous)`. A record stuck in
//! `Proposed` after a crash means "we do not know whether this took
//! effect" — recovery MUST resolve it through external postconditions
//! before any retry is considered.

use lumi_protocol::{ActionId, ErrorEnvelope, Timestamp};
use serde::{Deserialize, Serialize};

/// Outcome of resolving an ambiguous side effect by checking external
/// state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbiguousResolution {
    /// External postconditions hold: the effect happened. Mark succeeded;
    /// never re-execute.
    ConfirmedApplied,
    /// External postconditions do not hold: the effect did not happen.
    /// Mark failed; a retry decision may now be made by normal rules.
    ConfirmedNotApplied,
    /// External state still cannot be determined. The run stays blocked
    /// (AMBIGUOUS); automatic retry remains forbidden.
    StillUnknown,
}

/// One consequential action's durable record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SideEffectRecord {
    pub action_id: ActionId,
    pub run_id: lumi_protocol::RunId,
    /// Client idempotency key, when the action declared one.
    pub idempotency_key: Option<String>,
    /// The action's material digest at proposal time.
    pub action_digest: String,
    pub status: SideEffectStatus,
    pub proposed_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<ErrorEnvelope>,
}

/// Journal lifecycle of a side effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SideEffectStatus {
    /// Persisted before execution; outcome unknown yet.
    Proposed,
    /// Postconditions verified; the effect happened exactly once.
    Succeeded,
    /// The effect definitely did not happen (or was compensated).
    Failed,
    /// Outcome unknown; blocked pending external verification.
    Ambiguous,
}

impl SideEffectStatus {
    /// True when retrying the action would risk duplication.
    #[must_use]
    pub const fn retry_may_duplicate(self) -> bool {
        matches!(self, Self::Proposed | Self::Succeeded | Self::Ambiguous)
    }
}

/// Append/update journal for side effects of one tenant/run set.
#[derive(Debug, Default)]
pub struct SideEffectJournal {
    records: std::collections::BTreeMap<ActionId, SideEffectRecord>,
}

impl SideEffectJournal {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the intent to execute a consequential action. MUST be
    /// persisted before the executor is invoked.
    pub fn propose(
        &mut self,
        action: &lumi_protocol::ActionProposal,
        now: Timestamp,
    ) -> SideEffectRecord {
        let record = SideEffectRecord {
            action_id: action.action_id.clone(),
            run_id: action.run_id.clone(),
            idempotency_key: action.idempotency.key.clone(),
            action_digest: action.material_digest(),
            status: SideEffectStatus::Proposed,
            proposed_at: now,
            resolved_at: None,
            result_digest: None,
            failure: None,
        };
        self.records
            .insert(record.action_id.clone(), record.clone());
        record
    }

    /// Marks the effect as verified-applied (postconditions passed).
    pub fn mark_succeeded(
        &mut self,
        action_id: &ActionId,
        result_digest: String,
        now: Timestamp,
    ) -> Result<(), String> {
        let record = self
            .records
            .get_mut(action_id)
            .ok_or_else(|| "unknown action id".to_owned())?;
        record.status = SideEffectStatus::Succeeded;
        record.result_digest = Some(result_digest);
        record.resolved_at = Some(now);
        Ok(())
    }

    /// Marks the effect as definitely not applied.
    pub fn mark_failed(
        &mut self,
        action_id: &ActionId,
        failure: ErrorEnvelope,
        now: Timestamp,
    ) -> Result<(), String> {
        let record = self
            .records
            .get_mut(action_id)
            .ok_or_else(|| "unknown action id".to_owned())?;
        record.status = SideEffectStatus::Failed;
        record.failure = Some(failure);
        record.resolved_at = Some(now);
        Ok(())
    }

    /// Marks the outcome unknown (e.g. timeout after submit).
    pub fn mark_ambiguous(&mut self, action_id: &ActionId) -> Result<(), String> {
        let record = self
            .records
            .get_mut(action_id)
            .ok_or_else(|| "unknown action id".to_owned())?;
        record.status = SideEffectStatus::Ambiguous;
        Ok(())
    }

    /// Applies an external-state resolution to an ambiguous record.
    pub fn resolve_ambiguous(
        &mut self,
        action_id: &ActionId,
        resolution: AmbiguousResolution,
        result_digest: Option<String>,
        failure: Option<ErrorEnvelope>,
        now: Timestamp,
    ) -> Result<SideEffectStatus, String> {
        let record = self
            .records
            .get_mut(action_id)
            .ok_or_else(|| "unknown action id".to_owned())?;
        match resolution {
            AmbiguousResolution::ConfirmedApplied => {
                record.status = SideEffectStatus::Succeeded;
                record.result_digest = result_digest;
            }
            AmbiguousResolution::ConfirmedNotApplied => {
                record.status = SideEffectStatus::Failed;
                record.failure = failure;
            }
            AmbiguousResolution::StillUnknown => {
                // stays Ambiguous / or Proposed -> Ambiguous
                record.status = SideEffectStatus::Ambiguous;
            }
        }
        if resolution != AmbiguousResolution::StillUnknown {
            record.resolved_at = Some(now);
        }
        Ok(record.status)
    }

    /// The record for an action, if journaled.
    #[must_use]
    pub fn get(&self, action_id: &ActionId) -> Option<&SideEffectRecord> {
        self.records.get(action_id)
    }

    /// All unresolved records (Proposed or Ambiguous) for a run, in
    /// proposal order. Recovery iterates these first.
    #[must_use]
    pub fn unresolved_for_run(&self, run_id: &lumi_protocol::RunId) -> Vec<&SideEffectRecord> {
        let mut records: Vec<&SideEffectRecord> = self
            .records
            .values()
            .filter(|r| {
                &r.run_id == run_id
                    && matches!(
                        r.status,
                        SideEffectStatus::Proposed | SideEffectStatus::Ambiguous
                    )
            })
            .collect();
        records.sort_by_key(|r| r.proposed_at);
        records
    }

    /// All records (for persistence serialization).
    #[must_use]
    pub fn all(&self) -> Vec<&SideEffectRecord> {
        self.records.values().collect()
    }

    /// Restores journal contents (e.g. after loading from disk).
    pub fn restore(&mut self, records: Vec<SideEffectRecord>) {
        for record in records {
            self.records.insert(record.action_id.clone(), record);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumi_protocol::{
        AuthenticationStrength, Capability, FailureCategory, Principal, PrincipalId, PrincipalKind,
        ResourceRef, ResourceType, RiskClass, RunId, Target, TenantId,
    };

    fn ts(secs: i64) -> Timestamp {
        Timestamp::from_epoch(secs, 0).unwrap()
    }

    fn action(id: &str, key: Option<&str>) -> lumi_protocol::ActionProposal {
        let mut builder = lumi_protocol::ActionProposal::builder(
            ActionId::parse(id).unwrap(),
            lumi_protocol::TaskId::parse("task-1").unwrap(),
            RunId::parse("run-1").unwrap(),
            Principal {
                principal_id: PrincipalId::parse("u-1").unwrap(),
                tenant_id: TenantId::parse("t-1").unwrap(),
                kind: PrincipalKind::User,
                authenticated_at: Some(Timestamp::UNIX_EPOCH),
                authentication_strength: Some(AuthenticationStrength::Mfa),
            },
            Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND),
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                id: "outbound/x".to_owned(),
                sensitivity: None,
            },
            Target::canonical("mailto:c@example.test"),
            "send_customer_email",
            RiskClass::Communication,
        )
        .arguments(serde_json::json!({"subject": "hi"}));
        if let Some(key) = key {
            builder = builder.idempotency(lumi_protocol::Idempotency {
                key: Some(key.to_owned()),
                semantics: lumi_protocol::IdempotencySemantics::ClientKey,
            });
        }
        builder.unwrap()
    }

    #[test]
    fn proposed_record_blocks_blind_retry() {
        let mut journal = SideEffectJournal::new();
        let a = action("a-1", Some("key-1"));
        journal.propose(&a, ts(10));
        let record = journal.get(&a.action_id).unwrap();
        assert!(record.status.retry_may_duplicate());
        assert_eq!(record.idempotency_key.as_deref(), Some("key-1"));
        assert_eq!(
            journal
                .unresolved_for_run(&RunId::parse("run-1").unwrap())
                .len(),
            1
        );
    }

    #[test]
    fn ambiguous_resolved_by_external_state() {
        let mut journal = SideEffectJournal::new();
        let a = action("a-1", None);
        journal.propose(&a, ts(10));
        journal.mark_ambiguous(&a.action_id).unwrap();

        // Recovery checks external postconditions: the record exists.
        let status = journal
            .resolve_ambiguous(
                &a.action_id,
                AmbiguousResolution::ConfirmedApplied,
                Some("result-digest".to_owned()),
                None,
                ts(20),
            )
            .unwrap();
        assert_eq!(status, SideEffectStatus::Succeeded);
        let record = journal.get(&a.action_id).unwrap();
        // Succeeded: re-executing WOULD duplicate, which the flag encodes.
        assert!(record.status.retry_may_duplicate());
        assert_eq!(record.status, SideEffectStatus::Succeeded);
        assert_eq!(
            journal
                .unresolved_for_run(&RunId::parse("run-1").unwrap())
                .len(),
            0
        );

        // And the not-applied resolution routes back to normal retry rules.
        let b = action("a-2", None);
        journal.propose(&b, ts(30));
        journal.mark_ambiguous(&b.action_id).unwrap();
        let status = journal
            .resolve_ambiguous(
                &b.action_id,
                AmbiguousResolution::ConfirmedNotApplied,
                None,
                Some(ErrorEnvelope::new(FailureCategory::Network, "never landed")),
                ts(40),
            )
            .unwrap();
        assert_eq!(status, SideEffectStatus::Failed);
        assert_eq!(
            journal.get(&b.action_id).unwrap().failure,
            Some(ErrorEnvelope::new(FailureCategory::Network, "never landed"))
        );
    }

    #[test]
    fn journal_serializes_for_persistence() {
        let mut journal = SideEffectJournal::new();
        journal.propose(&action("a-1", Some("k")), ts(1));
        let json = serde_json::to_string(&journal.all()).unwrap();
        let restored: Vec<SideEffectRecord> = serde_json::from_str(&json).unwrap();
        let mut journal2 = SideEffectJournal::new();
        journal2.restore(restored);
        assert!(journal2.get(&ActionId::parse("a-1").unwrap()).is_some());
    }
}
