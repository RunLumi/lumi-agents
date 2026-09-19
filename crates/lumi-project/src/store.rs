//! Durable project registry (spec 26 §26.3–26.4, §26.25).
//!
//! The [`ProjectStore`] persists project records as one JSON document
//! with atomic replacement (temp file + rename) and compare-before-save
//! generation transactions, following the same conventions as the task
//! state store. Project identity survives application restart; a
//! missing or moved root is never silently recreated or re-pointed
//! (§26.4): reopening a broken project fails closed until the user
//! deliberately relinks it.
//!
//! Identity proof: opening a folder writes a `.lumi/project.json`
//! marker carrying the project id. A root whose marker is missing or
//! names a different project is reported as moved/replaced and MUST be
//! relinked explicitly before use.

use crate::error::ProjectError;
use crate::record::{
    capability_snapshot, detect_git, detect_source, scan_instructions, AuthorizedRoot,
    OpenFolderRequest, OpenOutcome, ProjectRecord, RootAccess,
};
use lumi_protocol::{ProjectId, Timestamp};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Current durable schema version (spec 20: unknown versions fail closed).
pub const PROJECTS_SCHEMA_VERSION: u32 = 1;

/// On-disk identity marker written under `<root>/.lumi/`.
pub const PROJECT_MARKER_FILE: &str = "project.json";

const MARKER_DIR: &str = ".lumi";

/// Full durable document (serialized by [`ProjectStore`]).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistedProjects {
    pub schema_version: u32,
    /// Monotonic generation for compare-before-save transactions.
    #[serde(default)]
    pub generation: u64,
    #[serde(default)]
    pub projects: Vec<ProjectRecord>,
}

/// Where a project's primary root currently stands (§26.30 root
/// missing/moved failures are explicit health, not silent repair).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootHealth {
    /// Root exists, canonicalizes to the recorded path, and carries this
    /// project's identity marker.
    Available,
    /// The recorded path no longer exists.
    Missing,
    /// The path exists but is not provably the same resource (marker
    /// missing, named for another project, or canonicalized elsewhere).
    Moved { observed: String },
}

/// Transaction lock guard: removes the lock file on drop. A lock file
/// left behind by a crash is NOT reclaimed automatically — an operator
/// must inspect and remove it (same convention as the task state store).
struct ProjectLock {
    path: PathBuf,
}

impl Drop for ProjectLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// File-backed project registry with atomic replacement and
/// compare-before-save transactions. A lock file that survives a crash
/// is deliberately not reclaimed automatically; an operator must remove
/// it after inspection.
pub struct ProjectStore {
    path: PathBuf,
    observed_generation: Cell<Option<u64>>,
}

impl ProjectStore {
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self {
            path,
            observed_generation: Cell::new(None),
        }
    }

    fn lock_path(&self) -> PathBuf {
        PathBuf::from(format!("{}.lock", self.path.display()))
    }

    fn acquire_lock(&self) -> Result<ProjectLock, ProjectError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ProjectError::Io(e.to_string()))?;
        }
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.lock_path())
        {
            Ok(mut lock) => {
                lock.write_all(b"lumi-project transaction lock\n")
                    .map_err(|e| ProjectError::Io(e.to_string()))?;
                Ok(ProjectLock {
                    path: self.lock_path(),
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Err(
                ProjectError::TransactionBusy(self.lock_path().display().to_string()),
            ),
            Err(error) => Err(ProjectError::Io(error.to_string())),
        }
    }

    /// Reads the durable document; a missing file is a fresh registry.
    ///
    /// # Errors
    /// [`ProjectError::UnsupportedSchemaVersion`] for newer documents,
    /// [`ProjectError::Serde`] for corrupt ones.
    pub fn read(&self) -> Result<PersistedProjects, ProjectError> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(PersistedProjects {
                    schema_version: PROJECTS_SCHEMA_VERSION,
                    generation: 0,
                    projects: vec![],
                });
            }
            Err(e) => return Err(ProjectError::Io(e.to_string())),
        };
        let state: PersistedProjects = serde_json::from_str(&text)
            .map_err(|e| ProjectError::Serde(format!("corrupt project registry: {e}")))?;
        if state.schema_version != PROJECTS_SCHEMA_VERSION {
            return Err(ProjectError::UnsupportedSchemaVersion {
                found: state.schema_version,
                supported: PROJECTS_SCHEMA_VERSION,
            });
        }
        Ok(state)
    }

    fn observe_generation(&self, generation: u64) {
        self.observed_generation.set(Some(generation));
    }

    fn transact(
        &self,
        mutate: impl FnOnce(&mut PersistedProjects) -> Result<(), ProjectError>,
    ) -> Result<(), ProjectError> {
        let expected = self.observed_generation.get();
        self.transact_from(expected, mutate)
    }

    fn transact_from(
        &self,
        expected: Option<u64>,
        mutate: impl FnOnce(&mut PersistedProjects) -> Result<(), ProjectError>,
    ) -> Result<(), ProjectError> {
        let _lock = self.acquire_lock()?;
        let mut state = self.read()?;
        let actual = state.generation;
        if let Some(expected) = expected {
            if expected != actual {
                return Err(ProjectError::StaleWriter { expected, actual });
            }
        }
        let next_generation = actual
            .checked_add(1)
            .ok_or(ProjectError::GenerationExhausted)?;
        mutate(&mut state)?;
        state.generation = next_generation;
        state.schema_version = PROJECTS_SCHEMA_VERSION;
        self.write_atomic(&state)?;
        self.observed_generation.set(Some(next_generation));
        Ok(())
    }

    /// Commits a deliberate record update (multi-root changes, display
    /// rename, policy reference) conditional on `expected_generation`
    /// — the generation the caller observed via [`ProjectStore::read`].
    ///
    /// # Errors
    /// [`ProjectError::StaleWriter`] when the registry changed since the
    /// caller's observation; [`ProjectError::NotFound`] when the record
    /// is gone; validation failures for structurally invalid records.
    pub fn update_record(
        &mut self,
        record: ProjectRecord,
        expected_generation: u64,
    ) -> Result<ProjectRecord, ProjectError> {
        record.validate()?;
        let project_id = record.project_id.clone();
        self.transact_from(Some(expected_generation), |state| {
            if !state.projects.iter().any(|p| p.project_id == project_id) {
                return Err(ProjectError::NotFound {
                    project_id: project_id.to_string(),
                });
            }
            Self::retain_project(state, record.clone())
        })?;
        Ok(record)
    }

    fn write_atomic(&self, state: &PersistedProjects) -> Result<(), ProjectError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ProjectError::Io(e.to_string()))?;
        }
        let text =
            serde_json::to_string_pretty(state).map_err(|e| ProjectError::Serde(e.to_string()))?;
        let tmp = self.path.with_extension("json.tmp");
        {
            let mut file =
                std::fs::File::create(&tmp).map_err(|e| ProjectError::Io(e.to_string()))?;
            file.write_all(text.as_bytes())
                .map_err(|e| ProjectError::Io(e.to_string()))?;
            file.sync_all()
                .map_err(|e| ProjectError::Io(e.to_string()))?;
        }
        std::fs::rename(&tmp, &self.path).map_err(|e| ProjectError::Io(e.to_string()))
    }

    fn retain_project(
        state: &mut PersistedProjects,
        record: ProjectRecord,
    ) -> Result<(), ProjectError> {
        record.validate()?;
        state.projects.retain(|p| p.project_id != record.project_id);
        state.projects.push(record);
        Ok(())
    }

    /// Canonicalizes the user-selected folder or fails closed.
    fn canonicalize_selected(path: &Path) -> Result<PathBuf, ProjectError> {
        if !path.exists() {
            return Err(ProjectError::RootMissing {
                path: path.display().to_string(),
            });
        }
        if !path.is_dir() {
            return Err(ProjectError::RootNotADirectory {
                path: path.display().to_string(),
            });
        }
        std::fs::canonicalize(path).map_err(|e| ProjectError::Io(e.to_string()))
    }

    /// Open Folder: creates a durable project for the selected folder or
    /// reopens the existing project registered for it (§26.4).
    ///
    /// The folder is canonicalized, proven by writing the identity
    /// marker, and recorded with bounded discovery metadata. An existing
    /// registration for the same canonical root is reopened under its
    /// stable identity — never duplicated.
    ///
    /// # Errors
    /// [`ProjectError`] for missing/invalid roots, marker or registry
    /// write failures, and stale-writer conflicts.
    pub fn open_folder(
        &mut self,
        request: &OpenFolderRequest,
        now: Timestamp,
    ) -> Result<OpenOutcome, ProjectError> {
        let root = Self::canonicalize_selected(&request.path)?;
        let marker = read_marker(&root)?;
        if let Some(existing_id) = marker {
            // The folder already carries a Lumi identity: reopen that
            // project when it is in this registry (deliberate; a foreign
            // marker is reported, never overwritten).
            let state = self.read()?;
            if let Some(record) = state
                .projects
                .iter()
                .find(|p| p.project_id.as_str() == existing_id)
            {
                let mut record = record.clone();
                record.last_opened_at = now;
                self.transact(|state| Self::retain_project(state, record.clone()))?;
                return Ok(OpenOutcome::Reopened(record));
            }
            return Err(ProjectError::IdentityUnproven {
                path: root.display().to_string(),
            });
        }

        let project_id = ProjectId::generate();
        let record = Self::build_record(request, &root, project_id, now)?;
        // Claim check before any side effect: a registration whose
        // identity is unproven at this path must be resolved explicitly
        // (relink/remove), never silently replaced.
        let state = self.read()?;
        self.observe_generation(state.generation);
        if let Some(other) = state.projects.iter().find(|p| Self::claims_root(p, &root)) {
            return Err(ProjectError::PathClaimedByUnprovenProject {
                project_id: other.project_id.to_string(),
                path: root.display().to_string(),
            });
        }
        write_marker(&root, &record.project_id)?;
        if let Err(e) = self.transact(|state| Self::retain_project(state, record.clone())) {
            // Roll the marker back so the folder carries no dangling
            // identity for a project that was never registered.
            let _ = std::fs::remove_file(root.join(MARKER_DIR).join(PROJECT_MARKER_FILE));
            return Err(e);
        }
        Ok(OpenOutcome::Created(record))
    }

    fn claims_root(project: &ProjectRecord, canonical: &Path) -> bool {
        project.authorized_roots.iter().any(|r| r.path == canonical)
    }

    fn build_record(
        request: &OpenFolderRequest,
        root: &Path,
        project_id: ProjectId,
        now: Timestamp,
    ) -> Result<ProjectRecord, ProjectError> {
        let is_repository = detect_git(root, now).is_some();
        let display_name = request.display_name.clone().unwrap_or_else(|| {
            root.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "project".to_owned())
        });
        let record = ProjectRecord {
            project_id,
            tenant_id: request.tenant_id.clone(),
            owner: request.owner.clone(),
            execution_environment_id: request.execution_environment_id.clone(),
            display_name,
            primary_root: root.to_path_buf(),
            authorized_roots: vec![AuthorizedRoot {
                root_id: "primary".to_owned(),
                path: root.to_path_buf(),
                access: RootAccess::ReadWrite,
            }],
            created_at: now,
            last_opened_at: now,
            detected_source: detect_source(root),
            capability_snapshot: capability_snapshot(is_repository),
            policy_reference: request.policy_reference.clone(),
            instruction_sources: scan_instructions(root, now),
            indexing_state: crate::record::IndexingState::NotIndexed,
            git_metadata: detect_git(root, now),
        };
        record.validate()?;
        Ok(record)
    }

    /// Loads one project record.
    ///
    /// # Errors
    /// [`ProjectError::NotFound`] when absent.
    pub fn get(&self, project_id: &ProjectId) -> Result<ProjectRecord, ProjectError> {
        let state = self.read()?;
        self.observe_generation(state.generation);
        state
            .projects
            .iter()
            .find(|p| p.project_id == *project_id)
            .cloned()
            .ok_or_else(|| ProjectError::NotFound {
                project_id: project_id.to_string(),
            })
    }

    /// Projects sorted by most-recently-opened first.
    ///
    /// # Errors
    /// [`ProjectError`] from the underlying read.
    pub fn list_recent(&self) -> Result<Vec<ProjectRecord>, ProjectError> {
        let mut state = self.read()?;
        self.observe_generation(state.generation);
        state.projects.sort_by(|a, b| {
            let a_key = (
                a.last_opened_at.epoch_seconds(),
                a.last_opened_at.nanoseconds(),
            );
            let b_key = (
                b.last_opened_at.epoch_seconds(),
                b.last_opened_at.nanoseconds(),
            );
            b_key.cmp(&a_key)
        });
        Ok(state.projects)
    }

    /// Re-evaluates the health of a project's primary root without
    /// mutating anything (§26.4: no silent recreation).
    ///
    /// # Errors
    /// [`ProjectError::NotFound`] when the project is unknown.
    pub fn root_health(&self, project_id: &ProjectId) -> Result<RootHealth, ProjectError> {
        let record = self.get(project_id)?;
        Ok(Self::assess_health(&record))
    }

    fn assess_health(record: &ProjectRecord) -> RootHealth {
        let path = &record.primary_root;
        if !path.exists() {
            return RootHealth::Missing;
        }
        let canonical = match std::fs::canonicalize(path) {
            Ok(c) => c,
            Err(e) => {
                return RootHealth::Moved {
                    observed: e.to_string(),
                };
            }
        };
        if canonical != record.primary_root {
            return RootHealth::Moved {
                observed: canonical.display().to_string(),
            };
        }
        match read_marker(path) {
            Ok(Some(id)) if id == record.project_id.as_str() => RootHealth::Available,
            Ok(_) => RootHealth::Moved {
                observed: "identity marker missing or foreign".to_owned(),
            },
            Err(e) => RootHealth::Moved {
                observed: e.to_string(),
            },
        }
    }

    /// Deliberately re-points a project to a new root after the old one
    /// disappeared or was replaced (§26.4 relink flow).
    ///
    /// Refuses when the current root is still healthy
    /// ([`ProjectError::RelinkUnnecessary`]) and when the new root is
    /// already claimed by any project.
    ///
    /// # Errors
    /// [`ProjectError`] as above plus registry/transaction failures.
    pub fn relink(
        &mut self,
        project_id: &ProjectId,
        new_root: &Path,
        now: Timestamp,
    ) -> Result<ProjectRecord, ProjectError> {
        let canonical = Self::canonicalize_selected(new_root)?;
        let state = self.read()?;
        self.observe_generation(state.generation);
        let record = state
            .projects
            .iter()
            .find(|p| p.project_id == *project_id)
            .cloned()
            .ok_or_else(|| ProjectError::NotFound {
                project_id: project_id.to_string(),
            })?;
        if Self::assess_health(&record) == RootHealth::Available {
            return Err(ProjectError::RelinkUnnecessary {
                path: record.primary_root.display().to_string(),
            });
        }
        if let Some(other) = state
            .projects
            .iter()
            .find(|p| Self::claims_root(p, &canonical))
        {
            if other.project_id != *project_id {
                return Err(ProjectError::RootAlreadyRegistered {
                    project_id: other.project_id.to_string(),
                    path: canonical.display().to_string(),
                });
            }
        }
        // Never clobber another project's identity marker at the target.
        match read_marker(&canonical)? {
            Some(holder) if holder != project_id.as_str() => {
                return Err(ProjectError::RootAlreadyRegistered {
                    project_id: holder,
                    path: canonical.display().to_string(),
                });
            }
            _ => {}
        }
        let mut updated = record.clone();
        write_marker(&canonical, &updated.project_id)?;
        updated.primary_root = canonical.clone();
        for root in &mut updated.authorized_roots {
            if root.path == record.primary_root {
                root.path = canonical.clone();
            }
        }
        updated.last_opened_at = now;
        if let Err(e) = self.transact(|state| Self::retain_project(state, updated.clone())) {
            let _ = std::fs::remove_file(canonical.join(MARKER_DIR).join(PROJECT_MARKER_FILE));
            return Err(e);
        }
        Ok(updated)
    }

    /// Removes a project from the registry. The folder on disk is never
    /// touched; the marker becomes stale and a fresh open creates a new
    /// identity.
    ///
    /// # Errors
    /// [`ProjectError::NotFound`] when absent.
    pub fn remove(&mut self, project_id: &ProjectId) -> Result<(), ProjectError> {
        let state = self.read()?;
        self.observe_generation(state.generation);
        if !state.projects.iter().any(|p| p.project_id == *project_id) {
            return Err(ProjectError::NotFound {
                project_id: project_id.to_string(),
            });
        }
        self.transact(|state| {
            state.projects.retain(|p| p.project_id != *project_id);
            Ok(())
        })
    }
}

/// Reads `.lumi/project.json` and returns the recorded project id.
fn read_marker(root: &Path) -> Result<Option<String>, ProjectError> {
    let path = root.join(MARKER_DIR).join(PROJECT_MARKER_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ProjectError::Io(e.to_string())),
    };
    #[derive(Deserialize)]
    struct Marker {
        project_id: String,
    }
    let marker: Marker = serde_json::from_str(&text)
        .map_err(|e| ProjectError::Serde(format!("corrupt project marker: {e}")))?;
    Ok(Some(marker.project_id))
}

fn write_marker(root: &Path, project_id: &ProjectId) -> Result<(), ProjectError> {
    let dir = root.join(MARKER_DIR);
    std::fs::create_dir_all(dir)
        .map_err(|e| ProjectError::Io(format!("creating marker dir: {e}")))?;
    let text = serde_json::to_string_pretty(&serde_json::json!({
        "project_id": project_id.as_str(),
    }))
    .map_err(|e| ProjectError::Serde(e.to_string()))?;
    std::fs::write(root.join(MARKER_DIR).join(PROJECT_MARKER_FILE), text)
        .map_err(|e| ProjectError::Io(e.to_string()))
}
