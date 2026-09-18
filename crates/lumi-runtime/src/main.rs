use lumi_policy::DefaultPolicy;
use lumi_protocol::{ActionRequest, ExecutionTier, RiskLevel};
use lumi_runtime::{dispatch, Executor};

struct DryRunExecutor;

impl Executor for DryRunExecutor {
    type Error = std::convert::Infallible;

    fn execute(&mut self, request: &ActionRequest) -> Result<(), Self::Error> {
        println!(
            "dry-run execute: workflow={} tier={:?} target={} operation={}",
            request.workflow_id, request.tier, request.target, request.operation
        );
        Ok(())
    }
}

fn main() {
    let request = ActionRequest::new(
        "bootstrap",
        "bootstrap",
        ExecutionTier::NativeSemantic,
        RiskLevel::ReadOnly,
        "local-fixture",
        "inspect",
    );

    let outcome = dispatch(&DefaultPolicy::new(), &mut DryRunExecutor, &request)
        .expect("dry-run executor is infallible");

    println!("policy outcome: {outcome:?}");
}
