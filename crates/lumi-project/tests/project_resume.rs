//! Durability and resume contracts for Folder-as-Project (spec 26
//! §26.34 items 2, 13, 22): the project-bound task survives a runtime
//! restart with its binding intact, resume re-observes the world
//! instead of replaying mutations, and an external edit made during a
//! pause is detected, surfaced, and then cleanly rebased on re-observe.

use lumi_project::{
    ChangeEntry, ChangeKind, ChangeSetStore, ChangeSource, FileOpsError, OpenFolderRequest,
    ProjectError, ProjectFiles, ProjectStore, SearchMode,
};
use lumi_protocol::ids::{EnvironmentId, PrincipalId, TaskId, TenantId};
use lumi_protocol::principal::{AuthenticationStrength, Principal, PrincipalKind};
use lumi_protocol::{ProjectId, ProjectTaskBinding, Task, TaskStatus, Timestamp, WorkspaceKind};
use lumi_state::{JsonStateStore, StateStore};
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
        "lumi-project-resume-{tag}-{}-{}",
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
        execution_environment_id: EnvironmentId::parse("env-device-1").unwrap(),
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
fn project_bound_task_survives_restart_with_binding_intact() {
    let state_dir = unique_dir("dur-state");
    let repo = unique_dir("dur-repo");
    std::fs::write(repo.join("README.md"), b"# demo\n").unwrap();

    let mut projects = ProjectStore::new(state_dir.join("projects.json"));
    let project_id = open(&mut projects, &repo, 10);
    let binding = ProjectTaskBinding {
        project_id: ProjectId::parse(&project_id).unwrap(),
        execution_environment_id: EnvironmentId::parse("env-device-1").unwrap(),
        workspace_root: repo.display().to_string(),
        workspace_kind: WorkspaceKind::ProjectRoot,
    };
    let task = Task {
        task_id: TaskId::generate(),
        tenant_id: TenantId::parse("t-1").unwrap(),
        principal: owner(),
        mode: lumi_protocol::TaskMode::Work,
        goal: "Refactor the sync module and make tests pass".to_owned(),
        created_at: Timestamp::now(),
        deadline: None,
        budget: lumi_protocol::Budget::default(),
        privacy_constraints: lumi_protocol::PrivacyConstraint::default(),
        status: TaskStatus::Paused,
        requested_outputs: vec![],
        project_binding: Some(binding),
    };

    // Session 1: persist the task (and a change set) durably.
    {
        let mut store = JsonStateStore::new(state_dir.join("runtime-state.json"));
        store.save_task(&task).unwrap();
    }
    let changes = ChangeSetStore::new(state_dir.join("changes"));
    let mut set = changes.load(&project_id, &task.task_id).unwrap();
    set.push(
        ChangeEntry {
            kind: ChangeKind::Modified,
            source: ChangeSource::Agent,
            path: "README.md".to_owned(),
            from_path: None,
            sha256_before: None,
            sha256_after: None,
            patch: None,
            task_id: task.task_id.clone(),
            recorded_at: Timestamp::now(),
        },
        Timestamp::now(),
    );
    changes.save(&set).unwrap();

    // Session 2 (restart): everything is restored from durable state and
    // re-observed against the live filesystem.
    let restarted_projects = ProjectStore::new(state_dir.join("projects.json"));
    let health = restarted_projects
        .root_health(&ProjectId::parse(&project_id).unwrap())
        .unwrap();
    assert_eq!(health, lumi_project::RootHealth::Available);

    let store = JsonStateStore::new(state_dir.join("runtime-state.json"));
    let restored = store.load_task(&task.task_id).unwrap();
    let restored_binding = restored.project_binding.as_ref().unwrap();
    assert_eq!(restored_binding.project_id.to_string(), project_id);
    assert_eq!(restored_binding.workspace_kind, WorkspaceKind::ProjectRoot);
    assert_eq!(restored_binding.workspace_root, repo.display().to_string());
    assert_eq!(
        restored.status,
        TaskStatus::Paused,
        "no blind replay: paused stays paused"
    );

    // The project identity still resolves to the SAME folder, so resume
    // binds to the correct project/workspace (item 22).
    let record = restarted_projects
        .get(&ProjectId::parse(&project_id).unwrap())
        .unwrap();
    let resolved = lumi_project::resolve_in_project(&record, Path::new("README.md")).unwrap();
    assert!(resolved.resolved.starts_with(&repo));

    let reloaded_set = ChangeSetStore::new(state_dir.join("changes"))
        .load(&project_id, &task.task_id)
        .unwrap();
    assert_eq!(reloaded_set.entries.len(), 1);
    std::fs::remove_dir_all(&state_dir).ok();
    std::fs::remove_dir_all(&repo).ok();
}

#[test]
fn external_edit_during_pause_is_detected_then_rebased_on_reobserve() {
    let state_dir = unique_dir("pause-state");
    let repo = unique_dir("pause-repo");
    let mut projects = ProjectStore::new(state_dir.join("projects.json"));
    let project_id = open(&mut projects, &repo, 10);
    let record = projects
        .get(&ProjectId::parse(&project_id).unwrap())
        .unwrap();
    let files = ProjectFiles::new(&record);
    files
        .create_file(Path::new("plan.txt"), b"prepared against v1")
        .unwrap();
    let prepared_against = files.checksum(Path::new("plan.txt")).unwrap();

    // --- The world moves while the task is paused: an external editor
    // rewrites the file on disk.
    std::fs::write(repo.join("plan.txt"), b"external rewrite during pause").unwrap();

    // Resume step 1: re-observe BEFORE mutating. The prepared change no
    // longer matches; applying it must refuse (never clobber).
    let conflict = files.edit_file(Path::new("plan.txt"), &prepared_against, b"stale apply");
    assert!(
        matches!(conflict, Err(FileOpsError::StaleWrite { .. })),
        "{conflict:?}"
    );
    assert_eq!(
        std::fs::read(repo.join("plan.txt")).unwrap(),
        b"external rewrite during pause",
        "external work preserved"
    );

    // Resume step 2: re-read, re-prepare against the CURRENT state, and
    // apply. The conflict resolves without losing either side's intent.
    let current = files.checksum(Path::new("plan.txt")).unwrap();
    let rebased = files
        .edit_file(
            Path::new("plan.txt"),
            &current,
            b"rebased on external rewrite",
        )
        .unwrap();
    assert!(rebased.patch.is_some());
    assert_eq!(
        files.read_text(Path::new("plan.txt"), 4096).unwrap(),
        "rebased on external rewrite"
    );

    // The conflict AND the recovery are both visible in the change set:
    // the user can audit exactly what happened during the pause.
    let task = TaskId::parse("task-pause-1").unwrap();
    let changes = ChangeSetStore::new(state_dir.join("changes"));
    let mut set = changes.load(&project_id, &task).unwrap();
    set.push(
        ChangeEntry {
            kind: ChangeKind::Modified,
            source: ChangeSource::ExternalConflict,
            path: "plan.txt".to_owned(),
            from_path: None,
            sha256_before: Some(prepared_against),
            sha256_after: None,
            patch: None,
            task_id: task.clone(),
            recorded_at: Timestamp::now(),
        },
        Timestamp::now(),
    );
    set.push(
        ChangeEntry {
            kind: ChangeKind::Modified,
            source: ChangeSource::Agent,
            path: "plan.txt".to_owned(),
            from_path: None,
            sha256_before: Some(current),
            sha256_after: Some("new".to_owned()),
            patch: None,
            task_id: task.clone(),
            recorded_at: Timestamp::now(),
        },
        Timestamp::now(),
    );
    changes.save(&set).unwrap();
    let audited = changes.load(&project_id, &task).unwrap();
    assert_eq!(audited.entries.len(), 2);
    assert_eq!(
        audited.entries[0].source,
        ChangeSource::ExternalConflict,
        "the conflict itself is part of the record"
    );
    std::fs::remove_dir_all(&state_dir).ok();
    std::fs::remove_dir_all(&repo).ok();
}

#[test]
fn resume_refuses_when_root_moved_during_downtime() {
    let state_dir = unique_dir("moved-state");
    let repo = unique_dir("moved-repo");
    let mut projects = ProjectStore::new(state_dir.join("projects.json"));
    let project_id = open(&mut projects, &repo, 10);

    // Folder replaced while Lumi was down.
    std::fs::remove_dir_all(&repo).unwrap();
    std::fs::create_dir_all(&repo).unwrap();

    // Resume must fail closed on the identity check, never silently
    // continue against a folder it cannot prove is the same resource.
    let restarted = ProjectStore::new(state_dir.join("projects.json"));
    let health = restarted
        .root_health(&ProjectId::parse(&project_id).unwrap())
        .unwrap();
    assert_ne!(health, lumi_project::RootHealth::Available);
    // The replaced folder is empty: reads fail honestly instead of
    // fabricating state, and file-level mutation remains blocked until
    // the user relinks (health gates every resume flow upstream).
    let record = restarted
        .get(&ProjectId::parse(&project_id).unwrap())
        .unwrap();
    let files = ProjectFiles::new(&record);
    let read = files.read(Path::new("README.md"), 4096);
    assert!(
        matches!(read, Err(FileOpsError::NotFound { .. })),
        "nothing may be fabricated after an identity loss: {read:?}"
    );
    std::fs::remove_dir_all(&state_dir).ok();
    std::fs::remove_dir_all(&repo).ok();
}

#[test]
fn search_stays_current_across_a_restart_boundary() {
    let state_dir = unique_dir("search-state");
    let repo = unique_dir("search-repo");
    let mut projects = ProjectStore::new(state_dir.join("projects.json"));
    let project_id = open(&mut projects, &repo, 10);

    // Session 1 writes a file containing a marker.
    std::fs::write(repo.join("notes.md"), b"the launch codeword is zanzibar\n").unwrap();

    // Session 2 re-opens the project and searches: the filesystem is the
    // source of truth, so the new file is found without any reindex.
    let restarted = ProjectStore::new(state_dir.join("projects.json"));
    let record = restarted
        .get(&ProjectId::parse(&project_id).unwrap())
        .unwrap();
    let hits = lumi_project::search(&record, "zanzibar", SearchMode::Text, 50).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path.replace('\\', "/"), "notes.md");
    std::fs::remove_dir_all(&state_dir).ok();
    std::fs::remove_dir_all(&repo).ok();
}

#[test]
fn resume_against_the_wrong_project_or_environment_is_refused() {
    // spec 26 26.34 "resume against wrong Project": the durable binding
    // is the source of truth. A resume request naming another project,
    // or another environment, must fail closed instead of redirecting
    // the task's authority.
    let home = unique_dir("wrong-resume-state");
    let repo_a = unique_dir("wrong-repo-a");
    let repo_b = unique_dir("wrong-repo-b");
    std::fs::write(repo_a.join("a.txt"), b"a").unwrap();
    std::fs::write(repo_b.join("b.txt"), b"b").unwrap();

    let mut projects = ProjectStore::new(home.join("projects.json"));
    let id_a = open(&mut projects, &repo_a, 10);
    let id_b = open(&mut projects, &repo_b, 11);

    let env_a = lumi_protocol::EnvironmentId::parse("env-device-1").unwrap();
    let binding = lumi_protocol::ProjectTaskBinding {
        project_id: ProjectId::parse(&id_a).unwrap(),
        execution_environment_id: env_a.clone(),
        workspace_root: repo_a.display().to_string(),
        workspace_kind: lumi_protocol::WorkspaceKind::ProjectRoot,
    };

    // Wrong project: refused even though project B exists and is healthy.
    let err =
        lumi_project::assert_resume_target(&binding, &ProjectId::parse(&id_b).unwrap(), &env_a)
            .unwrap_err();
    assert!(matches!(err, ProjectError::InvalidRecord(_)), "{err}");

    // Wrong environment: refused (the task may not hop devices).
    let err = lumi_project::assert_resume_target(
        &binding,
        &ProjectId::parse(&id_a).unwrap(),
        &lumi_protocol::EnvironmentId::parse("env-other").unwrap(),
    )
    .unwrap_err();
    assert!(matches!(err, ProjectError::InvalidRecord(_)), "{err}");

    // The correct project + environment resumes.
    lumi_project::assert_resume_target(&binding, &ProjectId::parse(&id_a).unwrap(), &env_a)
        .unwrap();
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&repo_a).ok();
    std::fs::remove_dir_all(&repo_b).ok();
}

#[cfg(windows)]
#[test]
fn junction_escape_is_refused_like_symlinks() {
    // Windows junctions are reparse points; canonicalize resolves them,
    // so the same fail-closed rule must hold (spec 26 26.34).
    let home = unique_dir("junc-state");
    let repo = unique_dir("junc-repo");
    let outside = unique_dir("junc-outside");
    let mut projects = ProjectStore::new(home.join("projects.json"));
    let project_id = open(&mut projects, &repo, 10);
    let record = projects
        .get(&ProjectId::parse(&project_id).unwrap())
        .unwrap();

    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(repo.join("junction"))
        .arg(&outside)
        .status()
        .expect("mklink available");
    assert!(status.success(), "junction creation failed");

    let err = lumi_project::resolve_in_project(&record, Path::new("junction/secret.txt"));
    assert!(
        matches!(err, Err(ProjectError::OutsideProjectRoots { .. })),
        "{err:?}"
    );
    let files = ProjectFiles::new(&record);
    assert!(files
        .create_file(Path::new("junction/weaponized.txt"), b"no")
        .is_err());
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&repo).ok();
    std::fs::remove_dir_all(&outside).ok();
}
