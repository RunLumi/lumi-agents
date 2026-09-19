//! Working context: short-lived task material (spec 10 §10.3).
//!
//! The goal, plan, observations, tool results, and pending approvals of
//! ONE task. Default retention ends with the task: dropping the context
//! discards the material unless the caller explicitly promoted something
//! into durable memory or evidence.

use lumi_protocol::Timestamp;
use serde::{Deserialize, Serialize};

/// One entry in the working context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkingEntry {
    pub kind: WorkingEntryKind,
    pub content: String,
    pub at: Timestamp,
}

/// What kind of material this is (§10.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkingEntryKind {
    Goal,
    Plan,
    Observation,
    ToolResult,
    PendingApproval,
    CheckpointRef,
}

/// Short-lived task context. NOT durable memory: contents end with the
/// task unless explicitly promoted by a caller through the memory store.
#[derive(Debug, Clone, Default)]
pub struct WorkingContext {
    entries: Vec<WorkingEntry>,
}

impl WorkingContext {
    #[must_use]
    pub fn new(goal: &str) -> Self {
        Self {
            entries: vec![WorkingEntry {
                kind: WorkingEntryKind::Goal,
                content: goal.to_owned(),
                at: Timestamp::now(),
            }],
        }
    }

    pub fn push(&mut self, kind: WorkingEntryKind, content: impl Into<String>) {
        self.entries.push(WorkingEntry {
            kind,
            content: content.into(),
            at: Timestamp::now(),
        });
    }

    /// Entries visible to the model/planner, in insertion order. Used by
    /// compaction (§10.12) — the full raw set is not automatically
    /// preserved.
    #[must_use]
    pub fn entries(&self) -> &[WorkingEntry] {
        &self.entries
    }

    /// The task objective — MUST survive compaction (§10.12).
    #[must_use]
    pub fn goal(&self) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.kind == WorkingEntryKind::Goal)
            .map(|e| e.content.as_str())
    }

    /// Currently pending approvals — MUST survive compaction (§10.12:
    /// unresolved actions).
    #[must_use]
    pub fn pending_approvals(&self) -> Vec<&WorkingEntry> {
        self.entries
            .iter()
            .filter(|e| e.kind == WorkingEntryKind::PendingApproval)
            .collect()
    }

    /// Verified results — MUST survive compaction (§10.12).
    #[must_use]
    pub fn verified_results(&self) -> Vec<&WorkingEntry> {
        self.entries
            .iter()
            .filter(|e| {
                e.kind == WorkingEntryKind::Observation && e.content.starts_with("VERIFIED:")
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_survives_and_is_first() {
        let mut ctx = WorkingContext::new("Reconcile Q3 invoices");
        ctx.push(WorkingEntryKind::Observation, "read 42 rows");
        assert_eq!(ctx.goal(), Some("Reconcile Q3 invoices"));
    }

    #[test]
    fn pending_approvals_are_queryable() {
        let mut ctx = WorkingContext::new("g");
        ctx.push(WorkingEntryKind::PendingApproval, "send email to c@x");
        assert_eq!(ctx.pending_approvals().len(), 1);
    }
}
