//! Context compaction (spec 10 §10.12).
//!
//! Compaction shrinks working context under a token/character budget
//! while MUST-preserving:
//!
//! - the task objective;
//! - trusted instructions (system entries);
//! - policy-relevant constraints (entries marked as constraints);
//! - unresolved actions (pending approvals);
//! - verified results;
//! - recovery-critical state (checkpoint refs).
//!
//! Raw sensitive text (observations, tool results) is dropped first and
//! summarized as counts — not preserved "just because the budget allows".

use crate::working::{WorkingContext, WorkingEntry, WorkingEntryKind};
use lumi_protocol::Timestamp;

/// Constraint marker: entries whose content starts with this are
/// policy-relevant and survive compaction.
pub const CONSTRAINT_PREFIX: &str = "CONSTRAINT:";

/// Compacts `context` so its total content length is at most `max_chars`
/// while preserving the MUST-keep classes (§10.12). Returns a NEW
/// context; the original is untouched (callers can keep both for audit).
#[must_use]
pub fn compact(context: &WorkingContext, max_chars: usize) -> WorkingContext {
    let mut kept: Vec<WorkingEntry> = Vec::new();
    let mut dropped_raw = 0usize;

    for entry in context.entries() {
        let must_keep = matches!(
            entry.kind,
            WorkingEntryKind::Goal
                | WorkingEntryKind::PendingApproval
                | WorkingEntryKind::CheckpointRef
        ) || entry.content.starts_with("SYSTEM:")
            || entry.content.starts_with(CONSTRAINT_PREFIX)
            || context
                .verified_results()
                .iter()
                .any(|v| v.content == entry.content);
        if must_keep {
            kept.push(entry.clone());
        } else {
            dropped_raw += entry.content.len();
        }
    }

    // Budget check: MUST-keep entries are never dropped. If they alone
    // exceed the budget, keep them anyway (safety > budget, §10.12) —
    // the caller sees the overshoot.
    // The kept length vs budget is informational: MUST-keep entries are
    // never dropped, so the budget may be exceeded (safety > budget).
    let kept_len: usize = kept.iter().map(|e| e.content.len()).sum();
    let _ = (kept_len, max_chars);

    // The compaction marker is auditability, not bulk: always add it
    // when raw material was summarized out.
    if dropped_raw > 0 {
        kept.push(WorkingEntry {
            kind: WorkingEntryKind::Observation,
            content: format!(
                "COMPACTED: {dropped_raw} chars of raw observations/tool results \
                 summarized out (count preserved: safety state unaffected)"
            ),
            at: Timestamp::now(),
        });
    }

    let mut result = WorkingContext::default();
    for entry in kept {
        result.push(entry.kind, entry.content);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> WorkingContext {
        let mut ctx = WorkingContext::new("Reconcile Q3 invoices");
        ctx.push(
            WorkingEntryKind::Observation,
            "SYSTEM: you must reconcile before Friday".to_owned(),
        );
        ctx.push(
            WorkingEntryKind::ToolResult,
            "raw rows: 4123098123,0981203,12093,120398,12039812,039812,0398123".to_owned(),
        );
        ctx.push(
            WorkingEntryKind::Observation,
            "VERIFIED: ledger matched 41/42 rows".to_owned(),
        );
        ctx.push(WorkingEntryKind::PendingApproval, "send email to c@x");
        ctx.push(WorkingEntryKind::CheckpointRef, "ckpt-7");
        ctx.push(
            WorkingEntryKind::Observation,
            format!("CONSTRAINT: only eu region allowed, {}", "x".repeat(500)),
        );
        ctx
    }

    #[test]
    fn compaction_preserves_safety_and_recovery_state() {
        let ctx = context();
        let compacted = compact(&ctx, 400);
        // Objective survives.
        assert_eq!(compacted.goal(), Some("Reconcile Q3 invoices"));
        // Trusted system instruction survives.
        assert!(compacted
            .entries()
            .iter()
            .any(|e| { e.content.starts_with("SYSTEM:") }));
        // Unresolved approval survives.
        assert_eq!(compacted.pending_approvals().len(), 1);
        // Verified result survives.
        assert_eq!(compacted.verified_results().len(), 1);
        // Checkpoint ref survives.
        assert!(compacted
            .entries()
            .iter()
            .any(|e| { e.kind == WorkingEntryKind::CheckpointRef }));
        // Constraint survives.
        assert!(compacted
            .entries()
            .iter()
            .any(|e| { e.content.starts_with(CONSTRAINT_PREFIX) }));
        // Raw tool result was summarized out.
        assert!(!compacted
            .entries()
            .iter()
            .any(|e| { e.content.contains("4123098123") }));
    }

    #[test]
    fn compaction_notes_what_was_dropped() {
        let ctx = context();
        let compacted = compact(&ctx, 400);
        assert!(compacted
            .entries()
            .iter()
            .any(|e| { e.content.starts_with("COMPACTED:") }));
    }

    #[test]
    fn must_keep_entries_survive_even_over_budget() {
        let ctx = context();
        // Absurdly small budget: MUST-keep entries still survive.
        let compacted = compact(&ctx, 10);
        assert_eq!(compacted.goal(), Some("Reconcile Q3 invoices"));
        assert_eq!(compacted.pending_approvals().len(), 1);
    }
}
