//! End-to-end project file operation flows (spec 26 §26.34 items 10–14:
//! read/search/create/edit/move/delete, reversible deletion, stale-write
//! conflict, external concurrent edit, large/binary bounds) recorded
//! through a durable task change set.

use lumi_project::{
    ChangeEntry, ChangeKind, ChangeSetStore, ChangeSource, OpenFolderRequest, ProjectError,
    ProjectFiles, ProjectStore, SearchMode,
};
use lumi_protocol::ids::{EnvironmentId, PrincipalId, TaskId, TenantId};
use lumi_protocol::principal::{AuthenticationStrength, Principal, PrincipalKind};
use lumi_protocol::Timestamp;
use std::path::{Path, PathBuf};

fn owner() -> Principal {
    Principal {
        principal_id: PrincipalId::parse("u-1").unwrap(),
        tenant_id: TenantId::parse("t-1").unwrap(),
        kind: PrincipalKind::User,
        authenticated_at: Some(Timestamp::from_epoch(0, 0).unwrap()),
        authentication_strength: Some(AuthenticationStrength::DevicePossession),
    }
}

fn unique_dir(tag: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "lumi-project-flow-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::canonicalize(&dir).unwrap()
}

fn open(store: &mut ProjectStore, path: &Path, now: i64) -> String {
    let request = OpenFolderRequest {
        tenant_id: TenantId::parse("t-1").unwrap(),
        owner: owner(),
        execution_environment_id: EnvironmentId::parse("env-1").unwrap(),
        path: path.to_path_buf(),
        display_name: None,
        policy_reference: None,
    };
    store
        .open_folder(&request, Timestamp::from_epoch(now, 0).unwrap())
        .unwrap()
        .record()
        .project_id
        .to_string()
}

#[test]
fn task_change_set_survives_restart_and_separates_sources() {
    let home = unique_dir("flow-home");
    let repo = unique_dir("flow-repo");
    std::fs::write(repo.join("README.md"), b"# demo\n").unwrap();
    let mut store = ProjectStore::new(home.join("projects.json"));
    let project_id = open(&mut store, &repo, 10);

    let record = store
        .get(&lumi_protocol::ProjectId::parse(&project_id).unwrap())
        .unwrap();
    let task_id = TaskId::parse("task-flow-1").unwrap();
    let changes_dir = home.join("changes");
    let change_store = ChangeSetStore::new(changes_dir.clone());
    let mut change_set = change_store.load(&project_id, &task_id).unwrap();
    assert!(change_set.is_empty());

    let now = Timestamp::from_epoch(20, 0).unwrap();

    // --- Agent work: create, edit, move, reversible delete.
    let files = ProjectFiles::new(&record);
    let created = files
        .create_file(Path::new("src/lib.rs"), b"fn main() {}\n")
        .unwrap();
    change_set.push(
        ChangeEntry {
            kind: ChangeKind::Created,
            source: ChangeSource::Agent,
            path: created.path.clone(),
            from_path: created.from_path.clone(),
            sha256_before: created.sha256_before.clone(),
            sha256_after: created.sha256_after.clone(),
            patch: created.patch.clone(),
            task_id: task_id.clone(),
            recorded_at: now,
        },
        now,
    );

    let sha = files.checksum(Path::new("src/lib.rs")).unwrap();
    let edited = files
        .edit_file(Path::new("src/lib.rs"), &sha, b"fn main() -> i32 { 42 }\n")
        .unwrap();
    assert!(edited
        .patch
        .as_deref()
        .is_some_and(|p| p.contains("+fn main() -> i32")));
    change_set.push(
        ChangeEntry {
            kind: ChangeKind::Modified,
            source: ChangeSource::Agent,
            path: edited.path.clone(),
            from_path: None,
            sha256_before: edited.sha256_before.clone(),
            sha256_after: edited.sha256_after.clone(),
            patch: edited.patch.clone(),
            task_id: task_id.clone(),
            recorded_at: now,
        },
        now,
    );

    let sha = files.checksum(Path::new("src/lib.rs")).unwrap();
    files
        .move_path(
            Path::new("src/lib.rs"),
            Path::new("src/main.rs"),
            Some(&sha),
        )
        .unwrap();

    files.create_file(Path::new("tmp.txt"), b"temp").unwrap();
    let sha = files.checksum(Path::new("tmp.txt")).unwrap();
    let deleted = files
        .delete_file(Path::new("tmp.txt"), Some(&sha), false)
        .unwrap();
    assert!(deleted.undo_path.is_some(), "default delete is reversible");
    files.restore(&deleted).unwrap();
    change_set.push(
        ChangeEntry {
            kind: ChangeKind::Restored,
            source: ChangeSource::Agent,
            path: deleted.path.clone(),
            from_path: None,
            sha256_before: deleted.sha256_before.clone(),
            sha256_after: None,
            patch: None,
            task_id: task_id.clone(),
            recorded_at: now,
        },
        now,
    );
    change_store.save(&change_set).unwrap();

    // --- External concurrent edit is DETECTED, refused, and recorded as
    // an external conflict — Lumi's version never clobbers it.
    let external = files.checksum(Path::new("src/main.rs")).unwrap();
    std::fs::write(repo.join("src/main.rs"), b"// external editor was here\n").unwrap();
    let conflict = files.edit_file(Path::new("src/main.rs"), &external, b"// lumi rewrite");
    assert!(matches!(
        conflict,
        Err(lumi_project::FileOpsError::StaleWrite { .. })
    ));
    let mut reloaded_set = change_store.load(&project_id, &task_id).unwrap();
    reloaded_set.push(
        ChangeEntry {
            kind: ChangeKind::Modified,
            source: ChangeSource::ExternalConflict,
            path: "src/main.rs".to_owned(),
            from_path: None,
            sha256_before: Some(external),
            sha256_after: None,
            patch: None,
            task_id: task_id.clone(),
            recorded_at: Timestamp::from_epoch(30, 0).unwrap(),
        },
        Timestamp::from_epoch(30, 0).unwrap(),
    );
    change_store.save(&reloaded_set).unwrap();

    // --- "Restart": fresh stores over the same durable paths.
    let restarted = ProjectStore::new(home.join("projects.json"));
    let still_open = restarted
        .root_health(&lumi_protocol::ProjectId::parse(&project_id).unwrap())
        .unwrap();
    assert_eq!(still_open, lumi_project::RootHealth::Available);
    let changes_dir = home.join("changes");
    let final_set = ChangeSetStore::new(changes_dir)
        .load(&project_id, &task_id)
        .unwrap();
    assert_eq!(final_set.entries.len(), 4);
    assert_eq!(final_set.agent_entries().count(), 3);
    assert_eq!(
        final_set
            .entries
            .iter()
            .filter(|e| e.source == ChangeSource::ExternalConflict)
            .count(),
        1
    );

    // --- Search sees the post-task tree, not Lumi's stale view.
    let hits = lumi_project::search(&record, "external editor", SearchMode::Text, 50).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path.replace('\\', "/"), "src/main.rs");
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&repo).ok();
}

#[test]
fn writes_stay_inside_project_roots_and_fail_closed() {
    let home = unique_dir("escape-home");
    let repo = unique_dir("escape-repo");
    let mut store = ProjectStore::new(home.join("projects.json"));
    let project_id = open(&mut store, &repo, 10);
    let record = store
        .get(&lumi_protocol::ProjectId::parse(&project_id).unwrap())
        .unwrap();
    let files = ProjectFiles::new(&record);

    // Traversal above the root never reaches the filesystem.
    let err = files
        .create_file(Path::new("../../outside.txt"), b"no")
        .unwrap_err();
    assert!(
        matches!(
            err,
            lumi_project::FileOpsError::UnsafePath(ProjectError::OutsideProjectRoots { .. })
        ),
        "{err:?}"
    );

    // An in-project symlink pointing outside is refused on write.
    #[cfg(unix)]
    {
        let outside = unique_dir("escape-outside");
        std::os::unix::fs::symlink(&outside, repo.join("escape")).unwrap();
        let err = files
            .create_file(Path::new("escape/weaponized.txt"), b"no")
            .unwrap_err();
        let lumi_project::FileOpsError::UnsafePath(ProjectError::OutsideProjectRoots { requested }) =
            &err
        else {
            panic!("expected OutsideProjectRoots, got {err:?}")
        };
        assert!(
            requested.contains("symlink"),
            "refusal must name the cause: {requested}"
        );
        std::fs::remove_dir_all(&outside).ok();
    }
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&repo).ok();
}

#[test]
fn validation_outcomes_are_recorded_durably_in_the_change_set() {
    let home = unique_dir("val-home");
    let repo = unique_dir("val-repo");
    std::fs::write(repo.join("README.md"), b"# demo\n").unwrap();
    let mut store = ProjectStore::new(home.join("projects.json"));
    let project_id = open(&mut store, &repo, 10);
    let record = store
        .get(&lumi_protocol::ProjectId::parse(&project_id).unwrap())
        .unwrap();

    // Discovery on a manifest-less project proposes nothing (never
    // invents commands), and validation on missing tools is honest.
    let discovery = lumi_project::discover(&record).unwrap();
    assert!(discovery.proposed_commands.is_empty());

    let task_id = TaskId::parse("task-val-1").unwrap();
    let changes_dir = home.join("changes");
    let change_store = ChangeSetStore::new(changes_dir);
    let mut change_set = change_store.load(&project_id, &task_id).unwrap();

    // A passing validation is recorded as PASSED with evidence.
    let passed = lumi_project::run_validation(&record, "test", "true", 5_000).unwrap();
    assert_eq!(passed.status, lumi_project::ValidationStatus::Passed);
    change_set.push_validation(passed);

    // A failing validation is recorded as FAILED - never success.
    std::fs::write(repo.join("check.sh"), b"#!/bin/sh\nexit 2\n").unwrap();
    let failed = lumi_project::run_validation(&record, "lint", "sh check.sh", 5_000).unwrap();
    assert_eq!(failed.status, lumi_project::ValidationStatus::Failed);
    assert_eq!(failed.exit_code, Some(2));
    change_set.push_validation(failed);

    change_store.save(&change_set).unwrap();

    // Restart: fresh stores see the same honest history.
    let reloaded = ChangeSetStore::new(home.join("changes"))
        .load(&project_id, &task_id)
        .unwrap();
    assert_eq!(reloaded.validations.len(), 2);
    assert_eq!(reloaded.all_validations_passed(), Some(false));
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&repo).ok();
}
