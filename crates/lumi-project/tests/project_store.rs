//! Durable project registry acceptance tests (spec 26 §26.34 items 1–5:
//! open-folder creation, durable reopen, missing/moved-root handling,
//! single-root isolation, multi-root isolation).

use lumi_project::{
    OpenFolderRequest, OpenOutcome, ProjectError, ProjectStore, RootAccess, RootHealth,
};
use lumi_protocol::ids::{EnvironmentId, PrincipalId, TenantId};
use lumi_protocol::principal::{AuthenticationStrength, Principal, PrincipalKind};
use lumi_protocol::{ProjectId, Timestamp};
use std::path::{Path, PathBuf};

fn owner() -> Principal {
    Principal {
        principal_id: PrincipalId::parse("u-owner").unwrap(),
        tenant_id: TenantId::parse("t-1").unwrap(),
        kind: PrincipalKind::User,
        authenticated_at: Some(Timestamp::from_epoch(0, 0).unwrap()),
        authentication_strength: Some(AuthenticationStrength::DevicePossession),
    }
}

fn open_request(path: &Path) -> OpenFolderRequest {
    OpenFolderRequest {
        tenant_id: TenantId::parse("t-1").unwrap(),
        owner: owner(),
        execution_environment_id: EnvironmentId::parse("env-device-1").unwrap(),
        path: path.to_path_buf(),
        display_name: None,
        policy_reference: None,
    }
}

fn unique_dir(tag: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "lumi-project-store-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::canonicalize(&dir).unwrap()
}

fn store_in(dir: &Path) -> ProjectStore {
    ProjectStore::new(dir.join("projects.json"))
}

/// A realistic minimal repository: manifests, instructions, git.
fn sample_repo(tag: &str) -> PathBuf {
    let dir = unique_dir(tag);
    std::fs::write(dir.join("Cargo.toml"), b"[package]\nname = \"demo\"\n").unwrap();
    std::fs::write(dir.join("AGENTS.md"), b"# Working rules\nRun cargo test.\n").unwrap();
    std::fs::write(dir.join("README.md"), b"# demo\n").unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    dir
}

#[test]
fn open_folder_creates_durable_project() {
    let home = unique_dir("home");
    let repo = sample_repo("create");
    let mut store = store_in(&home);

    let outcome = store
        .open_folder(&open_request(&repo), Timestamp::from_epoch(10, 0).unwrap())
        .unwrap();
    let record = outcome.record();
    assert!(matches!(outcome, OpenOutcome::Created(_)));
    assert_eq!(
        record.display_name,
        repo.file_name().unwrap().to_string_lossy()
    );
    assert_eq!(record.detected_source, lumi_project::DetectedSource::Rust);
    assert_eq!(record.primary_root, repo);
    assert_eq!(record.authorized_roots.len(), 1);
    assert_eq!(record.authorized_roots[0].access, RootAccess::ReadWrite);
    assert!(record.capability_snapshot.contains(&"git_read".to_owned()));
    assert_eq!(record.instruction_sources.len(), 2, "AGENTS.md + README.md");
    assert!(record.git_metadata.as_ref().unwrap().is_repository);
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&repo).ok();
}

#[test]
fn project_identity_survives_restart() {
    let home = unique_dir("restart-home");
    let repo = sample_repo("restart");
    let path = home.join("projects.json");

    let mut first = ProjectStore::new(path.clone());
    let created = first
        .open_folder(&open_request(&repo), Timestamp::from_epoch(10, 0).unwrap())
        .unwrap();
    let id = created.record().project_id.clone();

    // Simulated restart: an entirely new store instance over the same
    // durable path, plus a fresh Open Folder click on the same folder.
    let mut restarted = ProjectStore::new(path);
    let outcome = restarted
        .open_folder(&open_request(&repo), Timestamp::from_epoch(20, 0).unwrap())
        .unwrap();
    let reopened = outcome.record();
    assert!(matches!(outcome, OpenOutcome::Reopened(_)));
    assert_eq!(reopened.project_id, id);
    assert!(
        reopened.last_opened_at.epoch_seconds() >= created.record().last_opened_at.epoch_seconds()
    );

    // And the recent list still contains exactly one project.
    let recents = restarted.list_recent().unwrap();
    assert_eq!(recents.len(), 1);
    assert_eq!(recents[0].project_id, id);
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&repo).ok();
}

#[test]
fn missing_root_is_surfaced_and_never_recreated() {
    let home = unique_dir("missing-home");
    let repo = unique_dir("missing-repo");
    let mut store = store_in(&home);
    let created = store
        .open_folder(&open_request(&repo), Timestamp::from_epoch(10, 0).unwrap())
        .unwrap();
    let id = created.record().project_id.clone();

    std::fs::remove_dir_all(&repo).unwrap();
    assert_eq!(store.root_health(&id).unwrap(), RootHealth::Missing);

    // Opening a NEW folder creates a NEW identity; the old record is
    // never silently re-pointed (§26.4).
    let other = unique_dir("missing-other");
    let outcome = store
        .open_folder(&open_request(&other), Timestamp::from_epoch(20, 0).unwrap())
        .unwrap();
    assert_ne!(outcome.record().project_id, id);

    let old = store.get(&id).unwrap();
    assert_eq!(
        old.primary_root,
        std::fs::canonicalize(&repo).unwrap_or(repo.clone())
    );

    // Relink is deliberate and works after the root is gone.
    let replacement = unique_dir("missing-replacement");
    std::fs::write(replacement.join("README.md"), b"moved here").unwrap();
    let relinked = store
        .relink(&id, &replacement, Timestamp::from_epoch(30, 0).unwrap())
        .unwrap();
    assert_eq!(relinked.primary_root, replacement);
    assert_eq!(store.root_health(&id).unwrap(), RootHealth::Available);
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn relink_is_refused_while_root_is_healthy() {
    let home = unique_dir("relink-home");
    let repo = sample_repo("relink");
    let mut store = store_in(&home);
    let created = store
        .open_folder(&open_request(&repo), Timestamp::from_epoch(10, 0).unwrap())
        .unwrap();
    let id = created.record().project_id.clone();
    let other = unique_dir("relink-other");
    let err = store
        .relink(&id, &other, Timestamp::from_epoch(20, 0).unwrap())
        .unwrap_err();
    assert!(
        matches!(err, ProjectError::RelinkUnnecessary { .. }),
        "{err}"
    );
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&repo).ok();
    std::fs::remove_dir_all(&other).ok();
}

#[test]
fn replaced_folder_requires_deliberate_relink() {
    let home = unique_dir("replaced-home");
    let repo = unique_dir("replaced-repo");
    let mut store = store_in(&home);
    let created = store
        .open_folder(&open_request(&repo), Timestamp::from_epoch(10, 0).unwrap())
        .unwrap();
    let id = created.record().project_id.clone();

    // Same path, different content: the marker is gone, so identity
    // cannot be proven. Health must NOT be Available.
    std::fs::remove_dir_all(&repo).unwrap();
    std::fs::create_dir_all(&repo).unwrap();
    match store.root_health(&id).unwrap() {
        RootHealth::Moved { .. } => {}
        other => panic!("expected Moved, got {other:?}"),
    }

    // Reopening the replaced folder must NOT resurrect the old project
    // or steal its identity: the path is claimed by a registration whose
    // identity is unproven, and the desktop must offer relink/remove.
    let err = store
        .open_folder(&open_request(&repo), Timestamp::from_epoch(20, 0).unwrap())
        .unwrap_err();
    assert!(
        matches!(
            err,
            ProjectError::PathClaimedByUnprovenProject { ref project_id, .. } if *project_id == id.as_str()
        ),
        "{err}"
    );

    // Explicit recovery (1): remove the dead registration, then the
    // folder opens as a brand-new project.
    store.remove(&id).unwrap();
    let outcome = store
        .open_folder(&open_request(&repo), Timestamp::from_epoch(25, 0).unwrap())
        .unwrap();
    assert!(matches!(outcome, OpenOutcome::Created(_)));
    assert_ne!(outcome.record().project_id, id);
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&repo).ok();
}

#[test]
fn single_root_isolation_refuses_sibling_scope() {
    let home = unique_dir("iso-home");
    let repo = sample_repo("iso");
    let mut store = store_in(&home);
    let created = store
        .open_folder(&open_request(&repo), Timestamp::from_epoch(10, 0).unwrap())
        .unwrap();
    let record = store.get(&created.record().project_id).unwrap();

    // Files inside the root resolve.
    let inside = lumi_project::resolve_in_project(&record, Path::new("Cargo.toml")).unwrap();
    assert!(inside.resolved.starts_with(&repo));

    // Sibling directory contents do not exist under the project scope:
    // an absolute sibling request is re-anchored INSIDE the root (never
    // honored as-is), and `..` traversal above the root is refused.
    let sibling = unique_dir("iso-sibling");
    std::fs::write(sibling.join("secret.txt"), b"no").unwrap();
    let re_anchored =
        lumi_project::resolve_in_project(&record, &sibling.join("secret.txt")).unwrap();
    assert!(re_anchored.resolved.starts_with(&repo));
    let traversal = lumi_project::resolve_in_project(&record, Path::new("../secret.txt"));
    assert!(matches!(
        traversal,
        Err(ProjectError::OutsideProjectRoots { .. })
    ));
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&repo).ok();
    std::fs::remove_dir_all(&sibling).ok();
}

#[test]
fn multi_root_is_explicit_and_isolated() {
    let home = unique_dir("multi-home");
    let repo = sample_repo("multi");
    let mut store = store_in(&home);
    let created = store
        .open_folder(&open_request(&repo), Timestamp::from_epoch(10, 0).unwrap())
        .unwrap();
    let id = created.record().project_id.clone();

    // A second authorized root must be an explicit registration on the
    // record: the registry never infers siblings. The desktop's
    // multi-root editor does this as a conditional update (§26.26).
    let docs = unique_dir("multi-docs");
    std::fs::write(docs.join("spec.md"), b"spec").unwrap();
    let observed = store.read().unwrap().generation;
    let mut record = store.get(&id).unwrap();
    record.authorized_roots.push(lumi_project::AuthorizedRoot {
        root_id: "docs".to_owned(),
        path: docs.clone(),
        access: RootAccess::ReadOnly,
    });
    record = store.update_record(record, observed).unwrap();

    // Both roots resolve; each EXISTING file lands in its real
    // containing root.
    let in_main = lumi_project::resolve_in_project(&record, Path::new("Cargo.toml")).unwrap();
    assert_eq!(in_main.root.root_id, "primary");
    let in_docs = lumi_project::resolve_in_project(&record, Path::new("spec.md")).unwrap();
    assert_eq!(in_docs.root.root_id, "docs");
    assert_eq!(in_docs.root.access, RootAccess::ReadOnly);

    // Absolute paths are never honored as-is: a stranger-directory path
    // re-anchors inside project scope (here: the docs root is the
    // containing candidate only by containment, but the stranger file
    // does not exist anywhere, so it falls back to the primary root as
    // a create target — inside project scope either way).
    let stranger = unique_dir("multi-stranger");
    std::fs::write(stranger.join("other.txt"), b"no").unwrap();
    let re_anchored =
        lumi_project::resolve_in_project(&record, &stranger.join("other.txt")).unwrap();
    assert!(re_anchored.resolved.starts_with(&repo));
    assert_ne!(re_anchored.resolved, stranger.join("other.txt"));
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&repo).ok();
    std::fs::remove_dir_all(&docs).ok();
    std::fs::remove_dir_all(&stranger).ok();
}

#[test]
fn stale_writer_is_refused_across_store_instances() {
    let home = unique_dir("stale-home");
    let repo_a = sample_repo("stale-a");
    let path = home.join("projects.json");

    let mut first = ProjectStore::new(path.clone());
    first
        .open_folder(
            &open_request(&repo_a),
            Timestamp::from_epoch(10, 0).unwrap(),
        )
        .unwrap();

    // Both instances observe the same generation.
    let mut second = ProjectStore::new(path.clone());
    let mut record = second
        .get(&first.list_recent().unwrap()[0].project_id)
        .unwrap();
    let observed = second.read().unwrap().generation;

    // Second instance commits a deliberate update first.
    record.display_name = "renamed by second".to_owned();
    second.update_record(record, observed).unwrap();

    // The first instance's decision was based on the same generation:
    // committing it now must fail as a stale writer.
    let mut competing = first.list_recent().unwrap().remove(0);
    competing.display_name = "renamed by first".to_owned();
    let err = first.update_record(competing, observed).unwrap_err();
    assert!(
        matches!(
            err,
            ProjectError::StaleWriter {
                expected: 1,
                actual: 2
            }
        ),
        "{err}"
    );
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&repo_a).ok();
}

#[test]
fn unsupported_schema_version_fails_closed() {
    let home = unique_dir("schema-home");
    let path = home.join("projects.json");
    std::fs::write(
        &path,
        r#"{"schema_version": 99, "generation": 4, "projects": []}"#,
    )
    .unwrap();
    let store = ProjectStore::new(path);
    let err = store.list_recent().unwrap_err();
    assert!(
        matches!(
            err,
            ProjectError::UnsupportedSchemaVersion { found: 99, .. }
        ),
        "{err}"
    );
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn remove_drops_registration_but_keeps_folder() {
    let home = unique_dir("remove-home");
    let repo = sample_repo("remove");
    let mut store = store_in(&home);
    let created = store
        .open_folder(&open_request(&repo), Timestamp::from_epoch(10, 0).unwrap())
        .unwrap();
    let id = created.record().project_id.clone();
    store.remove(&id).unwrap();
    assert!(matches!(store.get(&id), Err(ProjectError::NotFound { .. })));
    assert!(repo.join("Cargo.toml").is_file(), "folder untouched");
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&repo).ok();
}

#[test]
fn root_already_registered_reports_existing_project() {
    let home = unique_dir("dup-home");
    let repo = sample_repo("dup");
    let mut store = store_in(&home);
    store
        .open_folder(&open_request(&repo), Timestamp::from_epoch(10, 0).unwrap())
        .unwrap();
    // Remove the marker to force the unproven-claim check (a healthy
    // reopen is covered by the restart test above): without proof of
    // identity the path cannot be claimed by a second project either.
    std::fs::remove_dir_all(repo.join(".lumi")).unwrap();
    let err = store
        .open_folder(&open_request(&repo), Timestamp::from_epoch(20, 0).unwrap())
        .unwrap_err();
    assert!(
        matches!(err, ProjectError::PathClaimedByUnprovenProject { .. }),
        "{err}"
    );
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&repo).ok();
}

#[test]
fn project_ids_are_stable_and_registry_keys() {
    // ProjectId is the durable key, independent of path and name.
    let id = ProjectId::parse("proj-demo").unwrap();
    assert_eq!(id.as_str(), "proj-demo");
    assert_ne!(ProjectId::generate(), ProjectId::generate());
}
