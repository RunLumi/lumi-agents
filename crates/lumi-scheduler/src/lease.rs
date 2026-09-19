//! Unattended execution leases (spec 13 §13.5).
//!
//! Managed background work requires a revocable lease binding device,
//! workflow, capability scope, expiry, and budget. Leases are checked at
//! admission AND polled during execution: revocation halts the run at
//! its next gate check — the same discipline as user cancellation.

use lumi_protocol::{Capability, RunId, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The service name leases live under (for keychain/registry parity with
/// the secrets broker).
pub const DEVICE_LEASE_SERVICE: &str = "lumi-agents/device-lease";

/// A revocable authorization for unattended work (§13.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnattendedLease {
    pub lease_id: String,
    /// Device the lease is bound to (§13.5 device).
    pub device_id: String,
    /// Workflow (or pack id) the lease authorizes.
    pub workflow_id: String,
    /// Capability scope — execution outside these capabilities is
    /// refused even if policy would allow them for interactive runs.
    pub capability_scope: Vec<Capability>,
    pub issued_at: Timestamp,
    pub expires_at: Timestamp,
    /// Maximum actions across all runs under this lease.
    pub max_actions_total: u32,
    /// Actions already consumed under this lease.
    pub actions_consumed: u32,
    pub revoked: bool,
}

impl UnattendedLease {
    /// Admits a run at `now` when not revoked, unexpired, and the
    /// capability is in scope.
    #[must_use]
    pub fn admits(&self, now: Timestamp, workflow_id: &str, capability: &Capability) -> bool {
        !self.revoked
            && now <= self.expires_at
            && self.workflow_id == workflow_id
            && self.capability_scope.contains(capability)
            && self.actions_consumed < self.max_actions_total
    }

    /// Remaining action headroom.
    #[must_use]
    pub fn remaining_actions(&self) -> u32 {
        self.max_actions_total.saturating_sub(self.actions_consumed)
    }
}

/// Why a lease was refused/revoked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseRevocation {
    Revoked,
    Expired,
    ActionBudgetExhausted,
    WorkflowMismatch,
    CapabilityOutOfScope,
}

impl LeaseRevocation {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Revoked => "lease revoked",
            Self::Expired => "lease expired",
            Self::ActionBudgetExhausted => "lease action budget exhausted",
            Self::WorkflowMismatch => "workflow not covered by lease",
            Self::CapabilityOutOfScope => "capability outside lease scope",
        }
    }
}

/// Registry of unattended leases. Admission + mid-run re-checks both go
/// through `check_run` so revocation halts runs at the next gate (§13.5).
#[derive(Debug, Default)]
pub struct LeaseRegistry {
    leases: BTreeMap<String, UnattendedLease>,
    /// Per-run consumption: run_id -> lease_id.
    run_leases: BTreeMap<RunId, String>,
}

impl LeaseRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, lease: UnattendedLease) {
        self.leases.insert(lease.lease_id.clone(), lease);
    }

    /// Binds a run to a lease and records one action consumption.
    ///
    /// Returns the refusal reason when the lease does not admit the run
    /// (§13.5: revocable at admission).
    pub fn check_run(
        &mut self,
        run_id: &RunId,
        lease_id: &str,
        workflow_id: &str,
        capability: &Capability,
        now: Timestamp,
    ) -> Result<(), LeaseRevocation> {
        let Some(lease) = self.leases.get_mut(lease_id) else {
            return Err(LeaseRevocation::Revoked);
        };
        if lease.revoked {
            return Err(LeaseRevocation::Revoked);
        }
        if now > lease.expires_at {
            return Err(LeaseRevocation::Expired);
        }
        if lease.workflow_id != workflow_id {
            return Err(LeaseRevocation::WorkflowMismatch);
        }
        if !lease.capability_scope.contains(capability) {
            return Err(LeaseRevocation::CapabilityOutOfScope);
        }
        if lease.actions_consumed >= lease.max_actions_total {
            return Err(LeaseRevocation::ActionBudgetExhausted);
        }
        lease.actions_consumed += 1;
        self.run_leases.insert(run_id.clone(), lease_id.to_owned());
        Ok(())
    }

    /// Mid-run re-check (polled at each step gate): revocation/expiry
    /// halts the run without consuming further budget.
    pub fn recheck(&self, run_id: &RunId, now: Timestamp) -> Result<(), LeaseRevocation> {
        let Some(lease_id) = self.run_leases.get(run_id) else {
            return Err(LeaseRevocation::Revoked);
        };
        let Some(lease) = self.leases.get(lease_id) else {
            return Err(LeaseRevocation::Revoked);
        };
        if lease.revoked {
            return Err(LeaseRevocation::Revoked);
        }
        if now > lease.expires_at {
            return Err(LeaseRevocation::Expired);
        }
        Ok(())
    }

    /// Revokes a lease by id. Runs under it halt at their next re-check.
    pub fn revoke(&mut self, lease_id: &str) -> bool {
        if let Some(lease) = self.leases.get_mut(lease_id) {
            lease.revoked = true;
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn get(&self, lease_id: &str) -> Option<&UnattendedLease> {
        self.leases.get(lease_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lease(expires_at: Timestamp) -> UnattendedLease {
        UnattendedLease {
            lease_id: "lease-1".to_owned(),
            device_id: "dev-1".to_owned(),
            workflow_id: "invoice-reconciliation".to_owned(),
            capability_scope: vec![Capability::well_known("api.read")],
            issued_at: Timestamp::from_epoch(0, 0).unwrap(),
            expires_at,
            max_actions_total: 3,
            actions_consumed: 0,
            revoked: false,
        }
    }

    fn ts(secs: i64) -> Timestamp {
        Timestamp::from_epoch(secs, 0).unwrap()
    }

    #[test]
    fn admission_requires_scope_and_validity() {
        let mut registry = LeaseRegistry::new();
        registry.register(lease(ts(10_000)));
        let run = RunId::parse("run-1").unwrap();

        // In scope: admitted.
        assert!(registry
            .check_run(
                &run,
                "lease-1",
                "invoice-reconciliation",
                &Capability::well_known("api.read"),
                ts(100),
            )
            .is_ok());

        // Out-of-scope capability: refused.
        let other_run = RunId::parse("run-2").unwrap();
        assert_eq!(
            registry.check_run(
                &other_run,
                "lease-1",
                "invoice-reconciliation",
                &Capability::well_known("email.send"),
                ts(100),
            ),
            Err(LeaseRevocation::CapabilityOutOfScope)
        );
    }

    #[test]
    fn revoked_lease_halts_mid_run() {
        let mut registry = LeaseRegistry::new();
        registry.register(lease(ts(10_000)));
        let run = RunId::parse("run-1").unwrap();
        registry
            .check_run(
                &run,
                "lease-1",
                "invoice-reconciliation",
                &Capability::well_known("api.read"),
                ts(1),
            )
            .unwrap();

        registry.revoke("lease-1");
        // Mid-run re-check fails: the run halts at the next gate.
        assert_eq!(registry.recheck(&run, ts(2)), Err(LeaseRevocation::Revoked));
        // New admission also refused.
        let run2 = RunId::parse("run-2").unwrap();
        assert_eq!(
            registry.check_run(
                &run2,
                "lease-1",
                "invoice-reconciliation",
                &Capability::well_known("api.read"),
                ts(2)
            ),
            Err(LeaseRevocation::Revoked)
        );
    }

    #[test]
    fn expiry_and_action_budget_are_enforced() {
        let mut registry = LeaseRegistry::new();
        registry.register(lease(ts(100)));
        let now = ts(50);

        for i in 0..3 {
            let run = RunId::parse(format!("run-{i}")).unwrap();
            assert!(registry
                .check_run(
                    &run,
                    "lease-1",
                    "invoice-reconciliation",
                    &Capability::well_known("api.read"),
                    now
                )
                .is_ok());
        }
        // Action budget exhausted on the 4th run.
        let run4 = RunId::parse("run-4").unwrap();
        assert_eq!(
            registry.check_run(
                &run4,
                "lease-1",
                "invoice-reconciliation",
                &Capability::well_known("api.read"),
                now
            ),
            Err(LeaseRevocation::ActionBudgetExhausted)
        );

        // Expiry: fresh lease past expires_at.
        let mut registry2 = LeaseRegistry::new();
        registry2.register(lease(ts(100)));
        let run5 = RunId::parse("run-5").unwrap();
        assert_eq!(
            registry2.check_run(
                &run5,
                "lease-1",
                "invoice-reconciliation",
                &Capability::well_known("api.read"),
                ts(101)
            ),
            Err(LeaseRevocation::Expired)
        );
    }
}
