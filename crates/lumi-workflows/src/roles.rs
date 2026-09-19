//! Role Pack resolution and preparation.
//!
//! This module only composes existing [`lumi_packs::WorkflowPack`] values.
//! It does not execute actions or authorize them.  Preparation applies the
//! role ceiling, returns the role-intersected budget, and leaves the final
//! decision to the local policy/orchestrator gate.

use lumi_audit::VerificationEnvironment;
use lumi_orchestrator::{Orchestrator, StepOutcome};
use lumi_packs::{
    prepare_run, PackRunError, PreparedPackRun, RolePack, RoleWorkItem, WorkflowPack,
};
use lumi_protocol::{
    canonical::sha256_canonical, ActionProposal, ApprovalId, Budget, ConsumedBudget,
    ExecutionResult, Principal, RunId, TaskId,
};
use lumi_state::StateStore;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// A role manifest plus the exact workflow-pack versions it resolved.
#[derive(Debug, Clone)]
pub struct ResolvedRolePack {
    role: RolePack,
    workflows: BTreeMap<String, WorkflowPack>,
    role_content_digest: String,
    workflow_content_digests: BTreeMap<String, String>,
}

/// A role run prepared for the existing workflow runner.
#[derive(Debug, Clone)]
pub struct PreparedRoleRun {
    work_item_id: String,
    role_id: String,
    role_version: String,
    workflow_id: String,
    pack_version: String,
    role_content_digest: String,
    workflow_content_digest: String,
    effective_budget: Budget,
    /// These steps must remain approval-gated by the caller.  The local
    /// policy engine remains the final authority and may demand more.
    approval_required_steps: BTreeSet<String>,
    prepared: PreparedPackRun,
}

impl PreparedRoleRun {
    #[must_use]
    pub fn work_item_id(&self) -> &str {
        &self.work_item_id
    }

    #[must_use]
    pub fn role_id(&self) -> &str {
        &self.role_id
    }

    #[must_use]
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }

    #[must_use]
    pub fn effective_budget(&self) -> &Budget {
        &self.effective_budget
    }

    #[must_use]
    pub fn requires_approval(&self, step_id: &str) -> bool {
        self.approval_required_steps.contains(step_id)
    }

    /// Stable step identities available to a supervisor without exposing an
    /// unguarded action list.
    pub fn step_ids(&self) -> impl Iterator<Item = &str> {
        self.prepared
            .actions
            .iter()
            .map(|(step_id, _)| step_id.as_str())
    }
}

/// Errors raised before any action is executed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleRunError {
    UnknownWorkflow(String),
    Pack(PackRunError),
    InvalidWorkItem(Vec<String>),
    Authority(Vec<String>),
}

impl std::fmt::Display for RoleRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownWorkflow(id) => write!(f, "workflow {id:?} is not composed by this role"),
            Self::Pack(error) => write!(f, "workflow pack preparation failed: {error}"),
            Self::InvalidWorkItem(violations) => {
                write!(f, "role work item rejected: {}", violations.join("; "))
            }
            Self::Authority(violations) => {
                write!(
                    f,
                    "role authority rejected action: {}",
                    violations.join("; ")
                )
            }
        }
    }
}

impl std::error::Error for RoleRunError {}

impl From<PackRunError> for RoleRunError {
    fn from(error: PackRunError) -> Self {
        Self::Pack(error)
    }
}

impl ResolvedRolePack {
    /// Returns the immutable resolved role manifest.
    #[must_use]
    pub fn role(&self) -> &RolePack {
        &self.role
    }

    /// Returns the exact composed workflow by id.
    #[must_use]
    fn workflow(&self, workflow_id: &str) -> Option<&WorkflowPack> {
        self.workflows.get(workflow_id)
    }

    fn current_role_content_digest(&self) -> Result<String, String> {
        let packs = self
            .role
            .workflow_packs
            .iter()
            .map(|reference| {
                self.workflows
                    .get(&reference.pack_id)
                    .cloned()
                    .ok_or_else(|| format!("missing workflow {}", reference.pack_id))
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.role.content_digest(&packs)
    }

    /// Prepares one queue work item and applies the role ceiling to every
    /// materialized action. No executor, connector, or side effect runs.
    ///
    /// # Errors
    /// Returns a work-item, unknown-workflow, pack-preparation, or
    /// role-authority error.
    pub fn prepare_work_item(
        &self,
        item: &RoleWorkItem,
        task_id: &TaskId,
        run_id: &RunId,
        principal: &Principal,
    ) -> Result<PreparedRoleRun, RoleRunError> {
        self.role
            .validate_work_item(item)
            .map_err(RoleRunError::InvalidWorkItem)?;
        self.prepare_workflow(
            &item.work_item_id,
            &item.workflow_id,
            &item.inputs,
            task_id,
            run_id,
            principal,
        )
    }

    fn prepare_workflow(
        &self,
        work_item_id: &str,
        workflow_id: &str,
        inputs: &serde_json::Value,
        task_id: &TaskId,
        run_id: &RunId,
        principal: &Principal,
    ) -> Result<PreparedRoleRun, RoleRunError> {
        let pack = self
            .workflow(workflow_id)
            .ok_or_else(|| RoleRunError::UnknownWorkflow(workflow_id.to_owned()))?;
        let prepared = prepare_run(pack, inputs, task_id, run_id, principal)?;
        let mut violations = Vec::new();
        for (step_id, action) in &prepared.actions {
            if let Err(error) = self.role.check_action_ceiling(action) {
                violations.push(format!("{step_id}: {error}"));
            }
        }
        if prepared.actions.len() as u32 > self.role.authority.max_actions {
            violations.push(format!(
                "prepared action count {} exceeds role max_actions {}",
                prepared.actions.len(),
                self.role.authority.max_actions
            ));
        }
        if !violations.is_empty() {
            return Err(RoleRunError::Authority(violations));
        }
        let approval_required_steps = prepared
            .actions
            .iter()
            .filter(|(_, action)| self.role.requires_approval(action.risk_class))
            .map(|(step_id, _)| step_id.clone())
            .collect();
        Ok(PreparedRoleRun {
            work_item_id: work_item_id.to_owned(),
            role_id: self.role.role_id.clone(),
            role_version: self.role.version.clone(),
            workflow_id: workflow_id.to_owned(),
            pack_version: pack.manifest.version.clone(),
            role_content_digest: self.role_content_digest.clone(),
            workflow_content_digest: self
                .workflow_content_digests
                .get(workflow_id)
                .cloned()
                .ok_or_else(|| {
                    RoleRunError::Authority(vec![format!(
                        "missing content digest for workflow {workflow_id:?}"
                    )])
                })?,
            effective_budget: self.role.effective_budget(pack),
            prepared,
            approval_required_steps,
        })
    }

    /// Executes one already-prepared role step through the existing
    /// orchestrator gate.  A role-required approval is checked before the
    /// orchestrator or executor can run, even when tenant policy has a broad
    /// pre-authorization.  The orchestrator remains the final policy,
    /// journal, verification, and audit authority.
    // Keep the policy owner, observation environment and executor explicit at
    // this boundary instead of combining them into a new authority container.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_step<S, E>(
        &self,
        prepared: &PreparedRoleRun,
        step_id: &str,
        approval_id: Option<&ApprovalId>,
        consumed: &mut ConsumedBudget,
        orchestrator: &mut Orchestrator<S>,
        env: &dyn VerificationEnvironment,
        executor: &mut E,
    ) -> StepOutcome
    where
        S: StateStore,
        E: FnMut(&ActionProposal) -> ExecutionResult,
    {
        if prepared.role_id != self.role.role_id
            || prepared.role_version != self.role.version
            || prepared.role_content_digest != self.role_content_digest
            || self.current_role_content_digest().ok().as_deref()
                != Some(self.role_content_digest.as_str())
        {
            return StepOutcome::Stopped {
                reason: "prepared role identity/version/content no longer matches resolved role"
                    .to_owned(),
            };
        }
        let Some(pack) = self.workflow(&prepared.workflow_id) else {
            return StepOutcome::Stopped {
                reason: format!(
                    "prepared workflow {:?} is not part of the resolved role",
                    prepared.workflow_id
                ),
            };
        };
        let current_workflow_content_digest = pack_content_digest(pack);
        if pack.manifest.version != prepared.pack_version
            || prepared.workflow_content_digest != current_workflow_content_digest
            || self.role.effective_budget(pack) != prepared.effective_budget
        {
            return StepOutcome::Stopped {
                reason: "prepared workflow version/content or effective budget changed".to_owned(),
            };
        }
        let Some((_, action)) = prepared
            .prepared
            .actions
            .iter()
            .find(|(candidate, _)| candidate == step_id)
        else {
            return StepOutcome::Stopped {
                reason: format!("step {step_id:?} is not part of the prepared role run"),
            };
        };
        if let Err(error) = self.role.check_action_ceiling(action) {
            return StepOutcome::Stopped {
                reason: format!("prepared action escaped role ceiling: {error}"),
            };
        }
        if prepared.requires_approval(step_id) {
            orchestrator.execute_step_requiring_approval(
                action,
                approval_id,
                &prepared.effective_budget,
                consumed,
                env,
                executor,
            )
        } else {
            orchestrator.execute_step(
                action,
                approval_id,
                &prepared.effective_budget,
                consumed,
                env,
                executor,
            )
        }
    }
}

/// Loads a role manifest and all exact workflow-pack references it names.
///
/// # Errors
/// Returns filesystem, JSON, missing-reference, version, or authority
/// validation errors.  No fixture execution occurs.
pub fn load_role_pack(role_path: &Path, packs_dir: &Path) -> Result<ResolvedRolePack, String> {
    let text = std::fs::read_to_string(role_path)
        .map_err(|error| format!("reading {}: {error}", role_path.display()))?;
    let role: RolePack = serde_json::from_str(&text)
        .map_err(|error| format!("parsing {}: {error}", role_path.display()))?;
    let mut workflows = BTreeMap::new();
    let mut loaded = Vec::new();
    for reference in &role.workflow_packs {
        let path = packs_dir.join(&reference.pack_id).join("pack.json");
        let pack = super::load_pack(&path)?;
        validate_runtime_compatibility(&pack.manifest.runtime_version_range)?;
        loaded.push(pack.clone());
        workflows.insert(reference.pack_id.clone(), pack);
    }
    role.validate(&loaded)
        .map_err(|violations| format!("role invalid: {}", violations.join("; ")))?;
    let role_content_digest = role
        .content_digest(&loaded)
        .map_err(|error| format!("role content digest: {error}"))?;
    let workflow_content_digests = workflows
        .iter()
        .map(|(id, pack)| (id.clone(), pack_content_digest(pack)))
        .collect();
    Ok(ResolvedRolePack {
        role,
        workflows,
        role_content_digest,
        workflow_content_digests,
    })
}

/// Repository-relative role directory for tests and local tooling.
#[must_use]
pub fn repo_roles_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../roles")
}

fn pack_content_digest(pack: &WorkflowPack) -> String {
    let value = serde_json::to_value(pack).expect("workflow pack is JSON-serializable");
    sha256_canonical(&value)
}

/// Role admission supports the repository's current `>=major.minor[.patch]`
/// pack range form, plus an optional `<major[.minor[.patch]]` upper bound.
/// Any other expression is refused instead of guessed. The runtime version is
/// the actual compiled package version, not a caller-provided value.
fn validate_runtime_compatibility(range: &str) -> Result<(), String> {
    let current = parse_version(env!("CARGO_PKG_VERSION"))?;
    let parts: Vec<&str> = range.split(',').map(str::trim).collect();
    if parts.is_empty() || parts.len() > 2 {
        return Err(format!("unsupported runtime version range {range:?}"));
    }
    let lower = parts[0]
        .strip_prefix(">=")
        .ok_or_else(|| format!("unsupported runtime version range {range:?}"))?;
    if current < parse_partial_version(lower)? {
        return Err(format!(
            "runtime {} is below supported range {range:?}",
            env!("CARGO_PKG_VERSION")
        ));
    }
    if let Some(upper) = parts.get(1) {
        let upper = upper
            .strip_prefix('<')
            .ok_or_else(|| format!("unsupported runtime version range {range:?}"))?;
        if current >= parse_partial_version(upper)? {
            return Err(format!(
                "runtime {} is above supported range {range:?}",
                env!("CARGO_PKG_VERSION")
            ));
        }
    }
    Ok(())
}

fn parse_version(value: &str) -> Result<(u32, u32, u32), String> {
    let parts: Vec<&str> = value.trim().split('.').collect();
    if parts.len() != 3 {
        return Err(format!(
            "runtime version {value:?} is not major.minor.patch"
        ));
    }
    Ok((
        parts[0]
            .parse()
            .map_err(|_| format!("invalid runtime version {value:?}"))?,
        parts[1]
            .parse()
            .map_err(|_| format!("invalid runtime version {value:?}"))?,
        parts[2]
            .parse()
            .map_err(|_| format!("invalid runtime version {value:?}"))?,
    ))
}

fn parse_partial_version(value: &str) -> Result<(u32, u32, u32), String> {
    let parts: Vec<&str> = value.trim().split('.').collect();
    if !(1..=3).contains(&parts.len()) {
        return Err(format!("unsupported version bound {value:?}"));
    }
    let mut numbers = [0u32; 3];
    for (index, part) in parts.iter().enumerate() {
        numbers[index] = part
            .parse()
            .map_err(|_| format!("unsupported version bound {value:?}"))?;
    }
    Ok((numbers[0], numbers[1], numbers[2]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumi_protocol::{
        AuthenticationStrength, Capability, PrincipalId, PrincipalKind, ResourceType, TenantId,
        Timestamp,
    };

    fn principal() -> Principal {
        Principal {
            principal_id: PrincipalId::parse("u-role-test").unwrap(),
            tenant_id: TenantId::parse("t-role-test").unwrap(),
            kind: PrincipalKind::Workflow,
            authenticated_at: Some(Timestamp::UNIX_EPOCH),
            authentication_strength: Some(AuthenticationStrength::DevicePossession),
        }
    }

    #[test]
    fn finance_role_resolves_and_prepares_without_execution() {
        let resolved = load_role_pack(
            &repo_roles_dir().join("finance-operations-associate/role.json"),
            &repo_packs_dir(),
        )
        .unwrap();
        let prepared = resolved
            .prepare_work_item(
                &RoleWorkItem {
                    work_item_id: "expense-ER-77".to_owned(),
                    workflow_id: "expense-report-audit".to_owned(),
                    inputs: serde_json::json!({
                        "work_item_id": "expense-ER-77",
                        "source_system": "expenses",
                        "report_id": "ER-77",
                        "policy_version": "2026.1"
                    }),
                },
                &TaskId::parse("task-role-test").unwrap(),
                &RunId::parse("run-role-test").unwrap(),
                &principal(),
            )
            .unwrap();
        assert_eq!(prepared.role_id(), "finance-operations-associate");
        assert_eq!(prepared.step_ids().count(), 3);
        assert_eq!(prepared.effective_budget().max_actions, Some(6));
        assert!(prepared.requires_approval("record-audit-verdict"));
    }

    #[test]
    fn external_write_is_marked_for_approval_before_policy() {
        let resolved = load_role_pack(
            &repo_roles_dir().join("finance-operations-associate/role.json"),
            &repo_packs_dir(),
        )
        .unwrap();
        let prepared = resolved
            .prepare_work_item(
                &RoleWorkItem {
                    work_item_id: "expense-ER-77".to_owned(),
                    workflow_id: "expense-report-audit".to_owned(),
                    inputs: serde_json::json!({
                        "work_item_id": "expense-ER-77",
                        "source_system": "expenses",
                        "report_id": "ER-77",
                        "policy_version": "2026.1"
                    }),
                },
                &TaskId::parse("task-role-test").unwrap(),
                &RunId::parse("run-role-test-2").unwrap(),
                &principal(),
            )
            .unwrap();
        assert!(prepared.requires_approval("record-audit-verdict"));
        assert_eq!(prepared.effective_budget().max_external_writes, Some(1));
    }

    #[test]
    fn same_version_pack_content_mutation_stops_prepared_run() {
        let mut resolved = load_role_pack(
            &repo_roles_dir().join("finance-operations-associate/role.json"),
            &repo_packs_dir(),
        )
        .unwrap();
        let prepared = resolved
            .prepare_work_item(
                &RoleWorkItem {
                    work_item_id: "expense-ER-77".to_owned(),
                    workflow_id: "expense-report-audit".to_owned(),
                    inputs: serde_json::json!({
                        "work_item_id": "expense-ER-77",
                        "source_system": "expenses",
                        "report_id": "ER-77",
                        "policy_version": "2026.1"
                    }),
                },
                &TaskId::parse("task-role-digest").unwrap(),
                &RunId::parse("run-role-digest").unwrap(),
                &principal(),
            )
            .unwrap();
        resolved
            .workflows
            .get_mut("expense-report-audit")
            .unwrap()
            .steps[0]
            .target_template = "expenses://changed.example/reports".to_owned();
        let config = lumi_orchestrator::OrchestratorConfig {
            registry: lumi_policy::CapabilityRegistry::default(),
            device_state: lumi_policy::DeviceExecutionState::Trusted,
            pre_authorizations: vec![],
            retry: lumi_state::RetryPolicy::default(),
            policy_version: "1.0.0".to_owned(),
            executors: vec![],
            tier_policy: lumi_orchestrator::TierPolicy::default(),
        };
        let mut orchestrator = Orchestrator::new(config, lumi_state::InMemoryStateStore::new());
        let outcome = resolved.execute_step(
            &prepared,
            "fetch-report",
            None,
            &mut ConsumedBudget::default(),
            &mut orchestrator,
            &lumi_audit::FixtureEnvironment::new(),
            &mut |_action| unreachable!("content drift must stop before executor"),
        );
        assert!(matches!(outcome, StepOutcome::Stopped { .. }));
    }

    #[test]
    fn unsupported_or_incompatible_runtime_ranges_fail_role_admission() {
        assert!(validate_runtime_compatibility(">=0.1").is_ok());
        assert!(validate_runtime_compatibility(">=99.0").is_err());
        assert!(validate_runtime_compatibility("^0.1").is_err());
    }

    #[test]
    fn role_required_approval_blocks_executor() {
        let resolved = load_role_pack(
            &repo_roles_dir().join("finance-operations-associate/role.json"),
            &repo_packs_dir(),
        )
        .unwrap();
        let prepared = resolved
            .prepare_work_item(
                &RoleWorkItem {
                    work_item_id: "expense-ER-77".to_owned(),
                    workflow_id: "expense-report-audit".to_owned(),
                    inputs: serde_json::json!({
                        "work_item_id": "expense-ER-77",
                        "source_system": "expenses",
                        "report_id": "ER-77",
                        "policy_version": "2026.1"
                    }),
                },
                &TaskId::parse("task-role-test").unwrap(),
                &RunId::parse("run-role-test-3").unwrap(),
                &principal(),
            )
            .unwrap();
        let config = lumi_orchestrator::OrchestratorConfig {
            registry: lumi_policy::CapabilityRegistry::default().grant(
                lumi_policy::CapabilityGrant {
                    grant_id: "g-api-write".to_owned(),
                    tenant_id: TenantId::parse("t-role-test").unwrap(),
                    principal_id: None,
                    capability: Capability::well_known("api.write"),
                    resource_scope: lumi_policy::ResourceScope::all_of([ResourceType::well_known(
                        ResourceType::DATABASE_RECORD,
                    )]),
                    target_prefixes: BTreeSet::new(),
                    expires_at: None,
                    source: lumi_policy::GrantSource::OrganizationPolicy,
                },
            ),
            device_state: lumi_policy::DeviceExecutionState::Trusted,
            pre_authorizations: vec![],
            retry: lumi_state::RetryPolicy::default(),
            policy_version: "1.0.0".to_owned(),
            executors: vec![lumi_orchestrator::ExecutorDescriptor::new(
                lumi_protocol::ExecutionTier::ConnectorApi,
                "role-test-connector",
                "1.0.0",
                [Capability::well_known("api.write")],
            )],
            tier_policy: lumi_orchestrator::TierPolicy::default(),
        };
        let mut orchestrator = Orchestrator::new(config, lumi_state::InMemoryStateStore::new());
        let mut consumed = ConsumedBudget::default();
        let called = std::cell::Cell::new(false);
        let mut executor = |_action: &ActionProposal| {
            called.set(true);
            unreachable!("role approval gate must run before executor")
        };
        // Supply the read-only before observation required by the pack's
        // DIFF contract so this scenario reaches the approval decision.
        // Missing-before refusal is independently covered by the core suite.
        let env = lumi_audit::FixtureEnvironment::new().with_record(
            lumi_protocol::ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::DATABASE_RECORD),
                id: "expenses/audits/ER-77".to_owned(),
                sensitivity: None,
            },
            serde_json::json!({"report_id":"ER-77"}),
        );
        let outcome = resolved.execute_step(
            &prepared,
            "record-audit-verdict",
            None,
            &mut consumed,
            &mut orchestrator,
            &env,
            &mut executor,
        );
        assert!(
            matches!(outcome, StepOutcome::ApprovalNeeded { .. }),
            "first role approval outcome: {outcome:?}"
        );
        assert!(!called.get());

        let fake_approval = lumi_protocol::ApprovalId::parse("ap-fake").unwrap();
        let outcome = resolved.execute_step(
            &prepared,
            "record-audit-verdict",
            Some(&fake_approval),
            &mut consumed,
            &mut orchestrator,
            &env,
            &mut executor,
        );
        assert!(matches!(outcome, StepOutcome::ApprovalNeeded { .. }));
        assert!(!called.get());
    }

    fn repo_packs_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs")
    }
}
