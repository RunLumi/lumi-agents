//! Local execution boundary.
//!
//! Executor adapters live behind this crate so upstream automation engines can be
//! replaced without changing Lumi's workflow or policy contracts.

use lumi_policy::{Decision, DefaultPolicy};
use lumi_protocol::ActionRequest;

pub trait Executor {
    type Error;

    fn execute(&mut self, request: &ActionRequest) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchOutcome {
    Executed,
    ApprovalRequired { reason: &'static str },
    Denied { reason: &'static str },
}

pub fn dispatch<E: Executor>(
    policy: &DefaultPolicy,
    executor: &mut E,
    request: &ActionRequest,
) -> Result<DispatchOutcome, E::Error> {
    match policy.evaluate(request) {
        Decision::Allow => {
            executor.execute(request)?;
            Ok(DispatchOutcome::Executed)
        }
        Decision::RequireApproval { reason } => Ok(DispatchOutcome::ApprovalRequired { reason }),
        Decision::Deny { reason } => Ok(DispatchOutcome::Denied { reason }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumi_protocol::{ExecutionTier, RiskLevel};

    #[derive(Default)]
    struct FixtureExecutor {
        calls: usize,
    }

    impl Executor for FixtureExecutor {
        type Error = ();

        fn execute(&mut self, _request: &ActionRequest) -> Result<(), Self::Error> {
            self.calls += 1;
            Ok(())
        }
    }

    #[test]
    fn policy_blocks_execution_until_approval() {
        let mut executor = FixtureExecutor::default();
        let request = ActionRequest::new(
            "a-1",
            "wf-1",
            ExecutionTier::BrowserSemantic,
            RiskLevel::ExternalSideEffect,
            "fixture",
            "submit",
        );

        let outcome = dispatch(&DefaultPolicy::new(), &mut executor, &request).unwrap();

        assert!(matches!(outcome, DispatchOutcome::ApprovalRequired { .. }));
        assert_eq!(executor.calls, 0);
    }
}
