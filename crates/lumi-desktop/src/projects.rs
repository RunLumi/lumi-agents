//! Project service for the desktop Work mode (spec 26 §26.29).
//!
//! Composes the durable project registry, project file operations,
//! bounded search, the Git capability, validation execution, and the
//! runtime's project-bound task store behind one service the Tauri
//! shell can call. Every method returns serializable, honest state:
//! missing roots are health, empty task lists are empty, and refused
//! operations are errors — never fabricated data.

use crate::runtime::DesktopRuntime;
use lumi_project::{
    clone_repository, discover, list_artifacts, run_validation, ArtifactEntry, ChangeSet,
    ChangeSetStore, GitError, GitRepo, MemoryKind, MemoryProvenance, MemoryRecord,
    OpenFolderRequest, ProjectDiscovery, ProjectFiles, ProjectMemoryStore, ProjectStore,
    RootHealth, SearchHit, SearchMode, ValidationRecord,
};
use lumi_protocol::ids::{EnvironmentId, PrincipalId, ProjectId, TaskId, TenantId};
use lumi_protocol::principal::{AuthenticationStrength, Principal, PrincipalKind};
use lumi_protocol::{Timestamp, WorkspaceKind};
use std::path::{Path, PathBuf};

/// Reserved change-set attribution for edits performed directly by the
/// local user through the desktop UI (never agent work).
pub const MANUAL_TASK_ID: &str = "task-manual-local";

/// The desktop project service. Cloneable handles share one durable
/// state directory.
pub struct ProjectService {
    store: ProjectStore,
    changes: ChangeSetStore,
    memory: ProjectMemoryStore,
    state_dir: PathBuf,
    environment_id: EnvironmentId,
    tenant_id: TenantId,
    principal: Principal,
}

impl ProjectService {
    /// Opens (or initializes) the service over one durable state
    /// directory: `projects.json`, `environment.json`, `changes/`.
    ///
    /// # Errors
    /// I/O or serialization failures surface as strings (the shell
    /// boundary format).
    pub fn open(state_dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(state_dir).map_err(|e| e.to_string())?;
        let environment_id = load_or_create_environment(state_dir)?;
        Ok(Self {
            store: ProjectStore::new(state_dir.join("projects.json")),
            changes: ChangeSetStore::new(state_dir.join("changes")),
            memory: ProjectMemoryStore::new(state_dir.join("memory")),
            state_dir: state_dir.to_path_buf(),
            environment_id,
            tenant_id: TenantId::parse(DesktopRuntime::LOCAL_TENANT)
                .map_err(|e| format!("local tenant id: {e}"))?,
            principal: Principal {
                principal_id: PrincipalId::parse("principal-local")
                    .map_err(|e| format!("local principal id: {e}"))?,
                tenant_id: TenantId::parse(DesktopRuntime::LOCAL_TENANT)
                    .map_err(|e| format!("local tenant id: {e}"))?,
                kind: PrincipalKind::User,
                authenticated_at: Some(Timestamp::now()),
                authentication_strength: Some(AuthenticationStrength::DevicePossession),
            },
        })
    }

    #[must_use]
    pub const fn state_dir(&self) -> &PathBuf {
        &self.state_dir
    }

    /// Open Folder (§26.4): canonicalizes, creates or reopens the
    /// durable project, writes the identity marker.
    ///
    /// # Errors
    /// Project registry errors (missing root, duplicate claims, ...).
    pub fn open_folder(
        &mut self,
        path: &str,
        display_name: Option<String>,
    ) -> Result<OpenedProject, String> {
        let request = OpenFolderRequest {
            tenant_id: self.tenant_id.clone(),
            owner: self.principal.clone(),
            execution_environment_id: self.environment_id.clone(),
            path: PathBuf::from(path),
            display_name,
            policy_reference: None,
        };
        let outcome = self
            .store
            .open_folder(&request, Timestamp::now())
            .map_err(|e| e.to_string())?;
        Ok(OpenedProject {
            created: matches!(outcome, lumi_project::OpenOutcome::Created(_)),
            project: ProjectSummary::of(outcome.record(), &self.store)?,
        })
    }

    /// Recent projects, most recently opened first, each with current
    /// root health (§26.25).
    ///
    /// # Errors
    /// Registry read failure.
    pub fn list_recent(&self) -> Result<Vec<ProjectSummary>, String> {
        self.store
            .list_recent()
            .map_err(|e| e.to_string())?
            .iter()
            .map(|record| ProjectSummary::of(record, &self.store))
            .collect()
    }

    /// Bounded discovery + Git status for the project home (§26.7,
    /// §26.16).
    ///
    /// # Errors
    /// Unknown project or failing reads.
    pub fn overview(&self, project_id: &str) -> Result<ProjectOverview, String> {
        let id = ProjectId::parse(project_id).map_err(|e| e.to_string())?;
        let record = self.store.get(&id).map_err(|e| e.to_string())?;
        let health = self.store.root_health(&id).map_err(|e| e.to_string())?;
        let discovery = if health == RootHealth::Available {
            discover(&record).unwrap_or_default()
        } else {
            ProjectDiscovery::default()
        };
        let git = git_status_if_available(&record)
            .transpose()
            .map_err(|e| e.to_string())?;
        Ok(ProjectOverview {
            project_id: project_id.to_owned(),
            display_name: record.display_name.clone(),
            primary_root: record.primary_root.display().to_string(),
            environment_id: record.execution_environment_id.to_string(),
            detected_source: source_label(record.detected_source),
            capabilities: record.capability_snapshot.clone(),
            instructions: record
                .instruction_sources
                .iter()
                .map(|i| format!("{} ({:?})", i.path, i.kind))
                .collect(),
            health: health_label(&health),
            git,
            discovery,
        })
    }

    /// Deliberate relink to a new root (§26.4).
    ///
    /// # Errors
    /// Registry errors (relink refused while healthy, etc.).
    pub fn relink(&mut self, project_id: &str, path: &str) -> Result<ProjectSummary, String> {
        let id = ProjectId::parse(project_id).map_err(|e| e.to_string())?;
        let updated = self
            .store
            .relink(&id, Path::new(path), Timestamp::now())
            .map_err(|e| e.to_string())?;
        ProjectSummary::of(&updated, &self.store)
    }

    /// Removes the project from the registry; the folder is untouched.
    ///
    /// # Errors
    /// Unknown project or registry failure.
    pub fn remove(&mut self, project_id: &str) -> Result<(), String> {
        let id = ProjectId::parse(project_id).map_err(|e| e.to_string())?;
        self.store.remove(&id).map_err(|e| e.to_string())
    }

    /// Clones a repository into a NEW directory under `destination_parent`
    /// and opens the result as a durable project (spec 26 §26.27). The
    /// clone never executes repository code; hooks and scripts stay
    /// policy-gated. The destination must not already exist.
    ///
    /// # Errors
    /// Existing destination, clone failure, or registry failure.
    pub fn clone_repository(
        &mut self,
        source: &str,
        destination_parent: &str,
        display_name: Option<String>,
    ) -> Result<OpenedProject, String> {
        let parent = Path::new(destination_parent);
        if !parent.is_dir() {
            return Err(format!(
                "destination parent is not a directory: {destination_parent}"
            ));
        }
        let repo = clone_repository(source, parent).map_err(|e| e.to_string())?;
        self.open_folder(&repo.root().display().to_string(), display_name)
    }

    /// Records durable project memory with mandatory provenance
    /// (§26.22): guesses and unattributed claims are refused.
    pub fn memory_remember(
        &self,
        project_id: &str,
        submission: &MemorySubmission,
    ) -> Result<(), String> {
        let kind = match submission.kind.as_str() {
            "validated_command" => MemoryKind::ValidatedCommand,
            "convention" => MemoryKind::Convention,
            "environment_requirement" => MemoryKind::EnvironmentRequirement,
            "recovery_procedure" => MemoryKind::RecoveryProcedure,
            other => return Err(format!("unknown memory kind: {other}")),
        };
        let record = self.record(project_id)?;
        let head = git_repo_if_available(&record)
            .map(|repo| repo.head_hash())
            .transpose()
            .map_err(|e| e.to_string())?
            .flatten();
        let memory = MemoryRecord::new(
            submission.memory_id.clone(),
            kind,
            &submission.content,
            MemoryProvenance {
                task_id: TaskId::parse(&submission.task_id).map_err(|e| e.to_string())?,
                command: submission.command.clone(),
                git_head: head,
                evidence: submission.evidence.clone(),
            },
            None,
            Timestamp::now(),
        )
        .map_err(|e| e.to_string())?;
        self.memory
            .remember(project_id, memory)
            .map_err(|e| e.to_string())
    }

    /// Project memory with trustworthiness evaluated against the
    /// project's current Git HEAD (when it has one).
    pub fn memory_list(&self, project_id: &str) -> Result<Vec<(MemoryRecord, bool)>, String> {
        let record = self.record(project_id)?;
        let head = git_repo_if_available(&record)
            .map(|repo| repo.head_hash())
            .transpose()
            .map_err(|e| e.to_string())?
            .flatten();
        let records = self.memory.load(project_id).map_err(|e| e.to_string())?;
        Ok(records
            .into_iter()
            .map(|r| {
                let trusted = r.is_trustworthy(head.as_deref());
                (r, trusted)
            })
            .collect())
    }

    /// Invalidates one memory record with a reason (audit-retained).
    pub fn memory_invalidate(
        &self,
        project_id: &str,
        memory_id: &str,
        reason: &str,
    ) -> Result<(), String> {
        self.memory
            .invalidate(project_id, memory_id, reason, Timestamp::now())
            .map_err(|e| e.to_string())
    }

    fn record(&self, project_id: &str) -> Result<lumi_project::ProjectRecord, String> {
        let id = ProjectId::parse(project_id).map_err(|e| e.to_string())?;
        self.store.get(&id).map_err(|e| e.to_string())
    }

    /// Lists a directory of the project (Files surface).
    pub fn file_list(
        &self,
        project_id: &str,
        path: &str,
    ) -> Result<Vec<lumi_project::ListedEntry>, String> {
        let record = self.record(project_id)?;
        let files = ProjectFiles::new(&record);
        lumi_project::list_dir(files.project(), Path::new(path), 500).map_err(|e| e.to_string())
    }

    /// Reads a bounded UTF-8 file with its checksum (Files viewer).
    pub fn file_read(&self, project_id: &str, path: &str) -> Result<FileContent, String> {
        let record = self.record(project_id)?;
        let files = ProjectFiles::new(&record);
        let content = files
            .read_text(Path::new(path), 1_048_576)
            .map_err(|e| e.to_string())?;
        let sha256 = files.checksum(Path::new(path)).map_err(|e| e.to_string())?;
        Ok(FileContent {
            path: path.to_owned(),
            content,
            sha256,
        })
    }

    /// Creates a file, attributing the change to the manual task set.
    pub fn file_create(&self, project_id: &str, path: &str, content: &str) -> Result<(), String> {
        self.mutate(project_id, MANUAL_TASK_ID, |files| {
            files
                .create_file(Path::new(path), content.as_bytes())
                .map(|o| vec![o])
        })
    }

    /// Checksum-guarded edit (§26.24): refuses when the file changed
    /// since the UI last read it.
    pub fn file_edit(
        &self,
        project_id: &str,
        path: &str,
        expected_sha256: &str,
        content: &str,
    ) -> Result<(), String> {
        self.mutate(project_id, MANUAL_TASK_ID, |files| {
            files
                .edit_file(Path::new(path), expected_sha256, content.as_bytes())
                .map(|o| vec![o])
        })
    }

    /// Reversible delete with checksum guard.
    pub fn file_delete(
        &self,
        project_id: &str,
        path: &str,
        expected_sha256: Option<&str>,
    ) -> Result<(), String> {
        self.mutate(project_id, MANUAL_TASK_ID, |files| {
            files
                .delete_file(Path::new(path), expected_sha256, false)
                .map(|o| vec![o])
        })
    }

    /// Bounded filename/text search (§26.11).
    pub fn file_search(
        &self,
        project_id: &str,
        query: &str,
        mode: &str,
    ) -> Result<Vec<SearchHit>, String> {
        let record = self.record(project_id)?;
        let resolved_mode = match mode {
            "text" => SearchMode::Text,
            _ => SearchMode::FileName,
        };
        lumi_project::search(&record, query, resolved_mode, 200).map_err(|e| e.to_string())
    }

    fn mutate(
        &self,
        project_id: &str,
        task_id: &str,
        op: impl FnOnce(
            &ProjectFiles<'_>,
        )
            -> Result<Vec<lumi_project::MutationOutcome>, lumi_project::FileOpsError>,
    ) -> Result<(), String> {
        let record = self.record(project_id)?;
        let files = ProjectFiles::new(&record);
        let outcomes = op(&files).map_err(|e| e.to_string())?;
        let task = TaskId::parse(task_id).map_err(|e| e.to_string())?;
        let mut set = self
            .changes
            .load(project_id, &task)
            .map_err(|e| e.to_string())?;
        let now = Timestamp::now();
        for outcome in outcomes {
            set.push(
                lumi_project::ChangeEntry {
                    kind: kind_of(&outcome),
                    source: lumi_project::ChangeSource::Agent,
                    path: outcome.path.clone(),
                    from_path: outcome.from_path.clone(),
                    sha256_before: outcome.sha256_before.clone(),
                    sha256_after: outcome.sha256_after.clone(),
                    patch: outcome.patch.clone(),
                    task_id: task.clone(),
                    recorded_at: now,
                },
                now,
            );
        }
        self.changes.save(&set).map_err(|e| e.to_string())
    }

    /// Git status for the project (§26.16); `None` when the project has
    /// no repository.
    pub fn git_status(&self, project_id: &str) -> Result<Option<lumi_project::GitStatus>, String> {
        let id = ProjectId::parse(project_id).map_err(|e| e.to_string())?;
        let record = self.store.get(&id).map_err(|e| e.to_string())?;
        let status = git_repo_if_available(&record)
            .map(|repo| repo.status())
            .transpose()
            .map_err(|e| e.to_string())?;
        Ok(status)
    }

    /// Recent commits.
    pub fn git_log(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<lumi_project::CommitInfo>, String> {
        let id = ProjectId::parse(project_id).map_err(|e| e.to_string())?;
        let record = self.store.get(&id).map_err(|e| e.to_string())?;
        match git_repo_if_available(&record) {
            Some(repo) => Ok(repo.log(limit).map_err(|e| e.to_string())?),
            None => Ok(vec![]),
        }
    }

    /// Local branch names.
    pub fn git_branches(&self, project_id: &str) -> Result<Vec<String>, String> {
        let id = ProjectId::parse(project_id).map_err(|e| e.to_string())?;
        let record = self.store.get(&id).map_err(|e| e.to_string())?;
        match git_repo_if_available(&record) {
            Some(repo) => Ok(repo.branches().map_err(|e| e.to_string())?),
            None => Ok(vec![]),
        }
    }

    /// Creates a branch at HEAD (local write; §26.17).
    pub fn git_create_branch(&self, project_id: &str, name: &str) -> Result<(), String> {
        let id = ProjectId::parse(project_id).map_err(|e| e.to_string())?;
        let record = self.store.get(&id).map_err(|e| e.to_string())?;
        match git_repo_if_available(&record) {
            Some(repo) => repo.create_branch(name).map_err(|e| e.to_string()),
            None => Err("project has no git repository".to_owned()),
        }
    }

    /// Switches branches (local write; §26.17).
    pub fn git_switch(&self, project_id: &str, name: &str) -> Result<(), String> {
        let id = ProjectId::parse(project_id).map_err(|e| e.to_string())?;
        let record = self.store.get(&id).map_err(|e| e.to_string())?;
        match git_repo_if_available(&record) {
            Some(repo) => repo.switch(name).map_err(|e| e.to_string()),
            None => Err("project has no git repository".to_owned()),
        }
    }

    /// Stages paths and commits them path-scoped (local write; §26.17).
    /// Pre-existing user-staged work elsewhere in the index stays put.
    pub fn git_commit(
        &self,
        project_id: &str,
        message: &str,
        paths: &[String],
    ) -> Result<String, String> {
        let id = ProjectId::parse(project_id).map_err(|e| e.to_string())?;
        let record = self.store.get(&id).map_err(|e| e.to_string())?;
        let repo = git_repo_if_available(&record)
            .ok_or_else(|| "project has no git repository".to_owned())?;
        let path_refs: Vec<&Path> = paths.iter().map(Path::new).collect();
        repo.stage(&path_refs).map_err(|e| e.to_string())?;
        repo.commit(message, &path_refs).map_err(|e| e.to_string())
    }

    /// Runs one validation command at the project root and records the
    /// honest outcome on the task's change set (§26.20).
    pub fn validation_run(
        &self,
        project_id: &str,
        task_id: &str,
        command: &str,
    ) -> Result<ValidationRecord, String> {
        let id = ProjectId::parse(project_id).map_err(|e| e.to_string())?;
        let record = self.store.get(&id).map_err(|e| e.to_string())?;
        let validation =
            run_validation(&record, "validation", command, 120_000).map_err(|e| e.to_string())?;
        let task = TaskId::parse(task_id).map_err(|e| e.to_string())?;
        let mut set = self
            .changes
            .load(project_id, &task)
            .map_err(|e| e.to_string())?;
        set.push_validation(validation.clone());
        self.changes.save(&set).map_err(|e| e.to_string())?;
        Ok(validation)
    }

    /// Real generated outputs under `<root>/.lumi/artifacts` (§26.29):
    /// empty when the project has none — never sample rows.
    pub fn artifacts_list(&self, project_id: &str) -> Result<Vec<ArtifactEntry>, String> {
        let record = self.record(project_id)?;
        list_artifacts(&record.primary_root, 200).map_err(|e| e.to_string())
    }

    /// The durable change set of one task (§26.18).
    pub fn change_set(&self, project_id: &str, task_id: &str) -> Result<ChangeSet, String> {
        let task = TaskId::parse(task_id).map_err(|e| e.to_string())?;
        self.changes
            .load(project_id, &task)
            .map_err(|e| e.to_string())
    }

    /// All task change sets of one project, newest first.
    pub fn change_sets(&self, project_id: &str) -> Result<Vec<ChangeSet>, String> {
        self.changes
            .list_for_project(project_id)
            .map_err(|e| e.to_string())
    }

    /// Creates a durable project-bound task in the runtime store.
    pub fn task_create(
        &self,
        runtime: &mut DesktopRuntime,
        project_id: &str,
        goal: &str,
    ) -> Result<lumi_protocol::Task, String> {
        let id = ProjectId::parse(project_id).map_err(|e| e.to_string())?;
        let record = self.store.get(&id).map_err(|e| e.to_string())?;
        runtime.create_project_task(&crate::runtime::ProjectTaskSpec {
            tenant_id: self.tenant_id.clone(),
            principal: self.principal.clone(),
            goal: goal.to_owned(),
            project_id: id,
            environment_id: self.environment_id.clone(),
            workspace_root: record.primary_root.display().to_string(),
            workspace_kind: WorkspaceKind::ProjectRoot,
        })
    }

    /// Project-bound tasks, newest first.
    pub fn task_list(
        &self,
        runtime: &mut DesktopRuntime,
        project_id: &str,
    ) -> Result<Vec<lumi_protocol::Task>, String> {
        let id = ProjectId::parse(project_id).map_err(|e| e.to_string())?;
        runtime.project_tasks(&id)
    }
}

/// Convenience alias for read models.
type SourceLabel = &'static str;

/// Serializable project summary for the Recent Projects grid and the
/// open outcome.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectSummary {
    pub project_id: String,
    pub display_name: String,
    pub primary_root: String,
    pub environment_id: String,
    pub detected_source: SourceLabel,
    pub capabilities: Vec<String>,
    pub is_repository: bool,
    pub health: &'static str,
    pub last_opened_at: String,
}

impl ProjectSummary {
    fn of(record: &lumi_project::ProjectRecord, store: &ProjectStore) -> Result<Self, String> {
        let id = record.project_id.clone();
        let health = store.root_health(&id).map_err(|e| e.to_string())?;
        Ok(Self {
            project_id: id.to_string(),
            display_name: record.display_name.clone(),
            primary_root: record.primary_root.display().to_string(),
            environment_id: record.execution_environment_id.to_string(),
            detected_source: source_label(record.detected_source),
            capabilities: record.capability_snapshot.clone(),
            is_repository: record
                .git_metadata
                .as_ref()
                .is_some_and(|g| g.is_repository),
            health: health_label(&health),
            last_opened_at: record.last_opened_at.to_rfc3339(),
        })
    }
}

/// Open Folder outcome: created vs reopened under stable identity.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OpenedProject {
    pub created: bool,
    pub project: ProjectSummary,
}

/// Project home model (§26.29 overview).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectOverview {
    pub project_id: String,
    pub display_name: String,
    pub primary_root: String,
    pub environment_id: String,
    pub detected_source: SourceLabel,
    pub capabilities: Vec<String>,
    pub instructions: Vec<String>,
    pub health: &'static str,
    pub git: Option<lumi_project::GitStatus>,
    pub discovery: ProjectDiscovery,
}

/// A read file with the checksum edits must be prepared against.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub sha256: String,
}

/// One memory submission from the UI (kind is the wire vocabulary of
/// [`MemoryKind`]).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MemorySubmission {
    pub memory_id: String,
    pub kind: String,
    pub content: String,
    pub task_id: String,
    pub command: Option<String>,
    pub evidence: Option<String>,
}

const fn kind_of(outcome: &lumi_project::MutationOutcome) -> lumi_project::ChangeKind {
    use lumi_project::ChangeKind;
    if outcome.from_path.is_some() {
        ChangeKind::Moved
    } else if outcome.sha256_after.is_none() {
        ChangeKind::Deleted
    } else if outcome.sha256_before.is_none() {
        ChangeKind::Created
    } else {
        ChangeKind::Modified
    }
}

fn health_label(health: &RootHealth) -> &'static str {
    match health {
        RootHealth::Available => "available",
        RootHealth::Missing => "missing",
        RootHealth::Moved { .. } => "moved",
    }
}

const fn source_label(source: lumi_project::DetectedSource) -> SourceLabel {
    use lumi_project::DetectedSource;
    match source {
        DetectedSource::Rust => "Rust",
        DetectedSource::Node => "Node",
        DetectedSource::Python => "Python",
        DetectedSource::Go => "Go",
        DetectedSource::Generic => "Generic",
    }
}

fn git_repo_if_available(record: &lumi_project::ProjectRecord) -> Option<GitRepo> {
    if record.git_metadata.as_ref()?.is_repository {
        GitRepo::discover(&record.primary_root)
    } else {
        None
    }
}

fn git_status_if_available(
    record: &lumi_project::ProjectRecord,
) -> Option<Result<lumi_project::GitStatus, GitError>> {
    git_repo_if_available(record).map(|repo| repo.status())
}

fn load_or_create_environment(state_dir: &Path) -> Result<EnvironmentId, String> {
    let path = state_dir.join("environment.json");
    if let Ok(text) = std::fs::read_to_string(&path) {
        #[derive(serde::Deserialize)]
        struct Marker {
            environment_id: String,
        }
        let marker: Marker =
            serde_json::from_str(&text).map_err(|e| format!("corrupt environment marker: {e}"))?;
        return EnvironmentId::parse(marker.environment_id).map_err(|e| e.to_string());
    }
    let id = EnvironmentId::generate();
    let text = serde_json::json!({ "environment_id": id.as_str() }).to_string();
    std::fs::write(&path, text).map_err(|e| format!("persist environment marker: {e}"))?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "lumi-desktop-projects-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::canonicalize(&dir).unwrap()
    }

    fn fixture_repo(tag: &str) -> PathBuf {
        let root = temp_dir(tag);
        std::fs::write(root.join("README.md"), b"# demo\n").unwrap();
        std::fs::write(root.join("src.txt"), b"base\n").unwrap();
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "."],
            vec![
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "--no-verify",
                "--quiet",
                "-m",
                "initial",
            ],
        ] {
            let out = std::process::Command::new("git")
                .args(&args)
                .current_dir(&root)
                .output()
                .expect("git binary available");
            assert!(out.status.success(), "git {args:?} failed");
        }
        root
    }

    #[test]
    fn open_folder_creates_then_reopens_identity() {
        let state = temp_dir("state");
        let repo = fixture_repo("repo");
        let mut service = ProjectService::open(&state).unwrap();

        let opened = service
            .open_folder(repo.to_str().unwrap(), Some("Demo".to_owned()))
            .unwrap();
        assert!(opened.created);
        assert_eq!(opened.project.display_name, "Demo");
        assert_eq!(opened.project.health, "available");
        assert!(opened.project.is_repository);
        assert!(opened.project.capabilities.contains(&"git_read".to_owned()));

        let mut reopened_service = ProjectService::open(&state).unwrap();
        let reopened = reopened_service
            .open_folder(repo.to_str().unwrap(), None)
            .unwrap();
        assert!(!reopened.created, "same folder reopens the same identity");
        assert_eq!(reopened.project.project_id, opened.project.project_id);

        let recents = reopened_service.list_recent().unwrap();
        assert_eq!(recents.len(), 1);
        std::fs::remove_dir_all(&state).ok();
        std::fs::remove_dir_all(&repo).ok();
    }

    #[test]
    fn file_flow_records_manual_change_set_and_refuses_stale_writes() {
        let state = temp_dir("state-file");
        let repo = fixture_repo("repo-file");
        let mut service = ProjectService::open(&state).unwrap();
        let project = service
            .open_folder(repo.to_str().unwrap(), None)
            .unwrap()
            .project
            .project_id;

        service
            .file_create(&project, "docs/note.txt", "v1\n")
            .unwrap();
        let read = service.file_read(&project, "docs/note.txt").unwrap();
        assert_eq!(read.content, "v1\n");
        assert_eq!(
            read.sha256,
            service.file_read(&project, "docs/note.txt").unwrap().sha256
        );

        // A matching checksum edit succeeds; a stale one is refused.
        service
            .file_edit(&project, "docs/note.txt", &read.sha256, "v2\n")
            .unwrap();
        let err = service
            .file_edit(&project, "docs/note.txt", &read.sha256, "v3\n")
            .unwrap_err();
        assert!(err.contains("stale write"), "{err}");

        let set = service.change_set(&project, MANUAL_TASK_ID).unwrap();
        assert_eq!(set.entries.len(), 2, "create + successful edit recorded");
        std::fs::remove_dir_all(&state).ok();
        std::fs::remove_dir_all(&repo).ok();
    }

    #[test]
    fn overview_and_git_flow_survive_restart() {
        let state = temp_dir("state-git");
        let repo = fixture_repo("repo-git");
        {
            let mut service = ProjectService::open(&state).unwrap();
            let project = service
                .open_folder(repo.to_str().unwrap(), None)
                .unwrap()
                .project
                .project_id
                .clone();
            let overview = service.overview(&project).unwrap();
            assert_eq!(overview.health, "available");
            assert!(
                overview.discovery.manifests.is_empty(),
                "no manifests in fixture"
            );
            assert!(!overview.instructions.is_empty(), "README.md provenance");
            assert!(service.git_status(&project).unwrap().is_some());
            assert!(
                service
                    .git_branches(&project)
                    .unwrap()
                    .contains(&"main".to_owned())
                    || service
                        .git_branches(&project)
                        .unwrap()
                        .contains(&"master".to_owned())
            );
        }
        // Restart: new service over the same durable directory.
        let service = ProjectService::open(&state).unwrap();
        let recents = service.list_recent().unwrap();
        assert_eq!(recents.len(), 1);
        assert_eq!(recents[0].health, "available");
        std::fs::remove_dir_all(&state).ok();
        std::fs::remove_dir_all(&repo).ok();
    }

    #[test]
    fn task_create_persists_project_binding_and_validation_records() {
        let state = temp_dir("state-task");
        let repo = fixture_repo("repo-task");
        let runtime_state = state.join("runtime-state.json");
        let mut service = ProjectService::open(&state).unwrap();
        let project = service
            .open_folder(repo.to_str().unwrap(), None)
            .unwrap()
            .project
            .project_id
            .clone();

        let mut runtime = DesktopRuntime::new_for_tenant(
            runtime_state.clone(),
            TenantId::parse(DesktopRuntime::LOCAL_TENANT).unwrap(),
        )
        .unwrap();
        let task = service
            .task_create(&mut runtime, &project, "Fix the thing")
            .unwrap();
        let binding = task.project_binding.as_ref().unwrap();
        assert_eq!(binding.project_id.to_string(), project);
        assert_eq!(binding.workspace_kind, WorkspaceKind::ProjectRoot);

        let tasks = service.task_list(&mut runtime, &project).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].goal, "Fix the thing");

        // Honest validation recorded durably against the task.
        let record = service
            .validation_run(&project, task.task_id.as_str(), "sh -c 'exit 0'")
            .unwrap();
        assert_eq!(record.status, lumi_project::ValidationStatus::Passed);
        let set = service.change_set(&project, task.task_id.as_str()).unwrap();
        assert_eq!(set.all_validations_passed(), Some(true));
        std::fs::remove_dir_all(&state).ok();
        std::fs::remove_dir_all(&repo).ok();
    }

    #[test]
    fn clone_local_repository_opens_durable_project() {
        let state = temp_dir("clone-state");
        let source_repo = temp_dir("clone-source");
        std::fs::write(source_repo.join("seed.txt"), b"seed\n").unwrap();
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "."],
            vec![
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "--no-verify",
                "--quiet",
                "-m",
                "seed",
            ],
        ] {
            std::process::Command::new("git")
                .args(&args)
                .current_dir(&source_repo)
                .output()
                .expect("git available");
        }

        let mut service = ProjectService::open(&state).unwrap();
        let parent = temp_dir("clone-parent");
        let opened = service
            .clone_repository(
                &source_repo.display().to_string(),
                &parent.display().to_string(),
                Some("Cloned Demo".to_owned()),
            )
            .unwrap();
        assert!(opened.created);
        assert_eq!(opened.project.display_name, "Cloned Demo");
        assert_eq!(opened.project.health, "available");
        assert!(opened.project.is_repository);

        // The cloned folder is the project root; the marker proves it.
        // The service derives the folder name from the source's last
        // segment — the same rule the test applies here.
        let cloned_dir = parent.join(source_repo.file_name().unwrap());
        assert!(cloned_dir.join("seed.txt").is_file());
        assert!(cloned_dir.join(".lumi").is_dir());

        // Cloning onto an existing directory refuses.
        let err = service
            .clone_repository(
                &source_repo.display().to_string(),
                &parent.display().to_string(),
                None,
            )
            .unwrap_err();
        assert!(err.contains("already exists"), "{err}");
        std::fs::remove_dir_all(&state).ok();
        std::fs::remove_dir_all(&parent).ok();
        std::fs::remove_dir_all(&source_repo).ok();
    }

    #[test]
    fn project_memory_requires_provenance_and_tracks_head() {
        let state = temp_dir("mem-state");
        let repo = temp_dir("mem-repo");
        std::fs::write(repo.join("README.md"), b"# demo\n").unwrap();
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "."],
            vec![
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "--no-verify",
                "--quiet",
                "-m",
                "initial",
            ],
        ] {
            std::process::Command::new("git")
                .args(&args)
                .current_dir(&repo)
                .output()
                .expect("git available");
        }
        let mut service = ProjectService::open(&state).unwrap();
        let project = service
            .open_folder(repo.to_str().unwrap(), None)
            .unwrap()
            .project
            .project_id
            .clone();

        // A validated-command memory without the command provenance is a
        // guess and is refused.
        let err = service
            .memory_remember(
                &project,
                &MemorySubmission {
                    memory_id: "mem-1".to_owned(),
                    kind: "validated_command".to_owned(),
                    content: "npm test validates everything".to_owned(),
                    task_id: "task-manual-local".to_owned(),
                    command: None,
                    evidence: None,
                },
            )
            .unwrap_err();
        assert!(
            err.contains("provenance") || err.contains("command"),
            "{err}"
        );

        // With provenance it is remembered and trustworthy at the
        // current HEAD.
        service
            .memory_remember(
                &project,
                &MemorySubmission {
                    memory_id: "mem-1".to_owned(),
                    kind: "validated_command".to_owned(),
                    content: "npm test validates the sync module".to_owned(),
                    task_id: "task-manual-local".to_owned(),
                    command: Some("npm test".to_owned()),
                    evidence: Some("validation:passed".to_owned()),
                },
            )
            .unwrap();
        let listed = service.memory_list(&project).unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed[0].1, "fresh record at current head is trustworthy");
        assert!(
            listed[0].0.provenance.git_head.is_some(),
            "provenance captured the head"
        );

        // Invalidation is deliberate, reasoned, and audit-retained.
        service
            .memory_invalidate(&project, "mem-1", "command removed from CI")
            .unwrap();
        let listed = service.memory_list(&project).unwrap();
        assert_eq!(listed.len(), 1, "invalidation keeps the record");
        assert!(!listed[0].1);
        std::fs::remove_dir_all(&state).ok();
        std::fs::remove_dir_all(&repo).ok();
    }

    #[test]
    fn artifacts_list_returns_real_files_only() {
        let state = temp_dir("art-state");
        let repo = temp_dir("art-repo");
        let artifact_dir = repo.join(".lumi/artifacts/report-1");
        std::fs::create_dir_all(&artifact_dir).unwrap();
        std::fs::write(
            artifact_dir.join("artifact.json"),
            r#"{"artifact_id":"report-1","artifact_type":"test_report","path":"results.json","lifecycle":"READY"}"#,
        )
        .unwrap();
        std::fs::write(artifact_dir.join("results.json"), b"{}").unwrap();

        let mut service = ProjectService::open(&state).unwrap();
        let project = service
            .open_folder(repo.to_str().unwrap(), None)
            .unwrap()
            .project
            .project_id
            .clone();
        let listed = service.artifacts_list(&project).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "results.json");
        assert_eq!(listed[0].artifact_type.as_deref(), Some("test_report"));

        // A project without artifacts reports empty — never sample rows.
        let empty_repo = temp_dir("art-empty");
        let empty_project = service
            .open_folder(empty_repo.to_str().unwrap(), None)
            .unwrap()
            .project
            .project_id
            .clone();
        assert!(service.artifacts_list(&empty_project).unwrap().is_empty());
        std::fs::remove_dir_all(&state).ok();
        std::fs::remove_dir_all(&repo).ok();
        std::fs::remove_dir_all(&empty_repo).ok();
    }
}
