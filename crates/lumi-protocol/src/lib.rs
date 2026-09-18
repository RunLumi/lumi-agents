//! Stable vocabulary shared by the Lumi planner, policy engine, and executors.

/// Ordered from the most deterministic integration surface to the least.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionTier {
    ConnectorApi,
    BrowserSemantic,
    NativeSemantic,
    AppAdapter,
    Vision,
}

/// Risk is about the real-world effect, not how technically difficult an action is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    ReadOnly,
    Reversible,
    ExternalSideEffect,
    Destructive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRequest {
    pub action_id: String,
    pub workflow_id: String,
    pub tier: ExecutionTier,
    pub risk: RiskLevel,
    pub target: String,
    pub operation: String,
}

impl ActionRequest {
    #[must_use]
    pub fn new(
        action_id: impl Into<String>,
        workflow_id: impl Into<String>,
        tier: ExecutionTier,
        risk: RiskLevel,
        target: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            workflow_id: workflow_id.into(),
            tier,
            risk,
            target: target.into(),
            operation: operation.into(),
        }
    }
}
