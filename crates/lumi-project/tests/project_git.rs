//! Git capability acceptance tests (spec 26 §26.34 items 17–21):
//! status/diff on a dirty tree, preservation of pre-existing user
//! changes, local branch/commit, worktrees, and the policy posture that
//! external/destructive git operations are refused without authority.

use lumi_project::{GitRepo, ProjectFiles, RootAccess};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs a raw git command in a repo (test-side setup only).
fn git(dir: &Path, args: &[&str]) {
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env(
            "GIT_AUTHOR_NAME",
            std::env::var("GIT_AUTHOR_NAME").unwrap_or_else(|_| "Test".to_owned()),
        )
        .env(
            "GIT_AUTHOR_EMAIL",
            std::env::var("GIT_AUTHOR_EMAIL").unwrap_or_else(|_| "test@example.com".to_owned()),
        )
        .env(
            "GIT_COMMITTER_NAME",
            std::env::var("GIT_COMMITTER_NAME").unwrap_or_else(|_| "Test".to_owned()),
        )
        .env(
            "GIT_COMMITTER_EMAIL",
            std::env::var("GIT_COMMITTER_EMAIL").unwrap_or_else(|_| "test@example.com".to_owned()),
        );
    let out = command.output().expect("git binary available");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn unique_dir(tag: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "lumi-project-git-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::canonicalize(&dir).unwrap()
}

/// A realistic dirty repository: a committed base, a pre-existing user
/// modification, a staged user file, and an untracked user file.
fn dirty_repo(tag: &str) -> PathBuf {
    let root = unique_dir(tag);
    git(&root, &["init", "--initial-branch=main"]);
    // Repository-local identity: the Lumi git wrapper deliberately does
    // not inherit host env (beyond PATH/HOME), so repo config is the
    // identity source - exactly the real-project configuration.
    git(&root, &["config", "user.name", "Lumi Test"]);
    git(&root, &["config", "user.email", "lumi-test@example.com"]);
    std::fs::write(root.join("README.md"), b"# demo\n").unwrap();
    std::fs::write(root.join("src.txt"), b"committed content\n").unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/app.rs"), b"fn main() {}\n").unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "initial"]);
    // Pre-existing user modification (unstaged).
    std::fs::write(root.join("README.md"), b"# demo\n\nuser note\n").unwrap();
    // Pre-existing user staging.
    std::fs::write(root.join("user_staged.txt"), b"staged by user\n").unwrap();
    git(&root, &["add", "user_staged.txt"]);
    // Pre-existing untracked file.
    std::fs::write(root.join("user_notes.txt"), b"untracked\n").unwrap();
    root
}

#[test]
fn status_reports_dirty_tree_across_all_buckets() {
    let root = dirty_repo("status");
    let repo = GitRepo::discover(&root).expect("repo discovered");

    let status = repo.status().unwrap();
    assert_eq!(status.branch.as_deref(), Some("main"));
    assert!(status.head.is_some());
    assert!(!status.is_clean(), "fixture is dirty by construction");
    assert!(
        status
            .staged
            .iter()
            .any(|f| f.path == "user_staged.txt" && f.index_state == Some('A')),
        "user's staged file appears: {:?}",
        status.staged
    );
    assert!(
        status
            .unstaged
            .iter()
            .any(|f| f.path == "README.md" && f.worktree_state == Some('M')),
        "user's modification appears: {:?}",
        status.unstaged
    );
    assert!(status.untracked.contains(&"user_notes.txt".to_owned()));
    assert_eq!(status.changed_count(), 3);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn diff_stat_covers_dirty_tracked_files() {
    let root = dirty_repo("diff");
    let repo = GitRepo::discover(&root).unwrap();
    let stat = repo.diff_stat().unwrap();
    assert!(stat.contains("README.md"), "{stat}");
    assert!(
        stat.contains("user_staged.txt"),
        "staged files diff vs HEAD: {stat}"
    );
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn lumi_edits_preserve_all_pre_existing_user_changes() {
    let root = dirty_repo("preserve");
    let repo = GitRepo::discover(&root).unwrap();
    let before = repo.status().unwrap();

    // Lumi (via project file ops) touches ONLY its own new file, then
    // stages and commits exactly that file.
    let record = lumi_project::ProjectRecord {
        project_id: lumi_protocol::ProjectId::parse("p-git").unwrap(),
        tenant_id: lumi_protocol::TenantId::parse("t-1").unwrap(),
        owner: test_principal(),
        execution_environment_id: lumi_protocol::EnvironmentId::parse("env-1").unwrap(),
        display_name: "git-demo".to_owned(),
        primary_root: root.clone(),
        authorized_roots: vec![lumi_project::AuthorizedRoot {
            root_id: "primary".to_owned(),
            path: root.clone(),
            access: RootAccess::ReadWrite,
        }],
        created_at: lumi_protocol::Timestamp::from_epoch(0, 0).unwrap(),
        last_opened_at: lumi_protocol::Timestamp::from_epoch(0, 0).unwrap(),
        detected_source: lumi_project::DetectedSource::Generic,
        capability_snapshot: vec![],
        policy_reference: None,
        instruction_sources: vec![],
        indexing_state: lumi_project::IndexingState::NotIndexed,
        git_metadata: None,
    };
    let files = ProjectFiles::new(&record);
    files
        .create_file(Path::new("lumi_change.txt"), b"lumi work\n")
        .unwrap();
    repo.stage(&[Path::new("lumi_change.txt")]).unwrap();
    // Path-scoped commit: Lumi's commit can never sweep in the user's
    // separately-staged work (§26.19).
    let commit = repo
        .commit("lumi: add lumi_change.txt", &[Path::new("lumi_change.txt")])
        .unwrap();
    assert!(!commit.is_empty());

    let after = repo.status().unwrap();

    // Pre-existing user work survives, still attributed to the user:
    // same staged file, same unstaged modification, same untracked file.
    assert!(
        after
            .staged
            .iter()
            .any(|f| f.path == "user_staged.txt" && f.index_state == Some('A')),
        "user's staged file untouched: {:?}",
        after.staged
    );
    assert!(
        after.unstaged.iter().any(|f| f.path == "README.md"),
        "user's modification untouched: {:?}",
        after.unstaged
    );
    assert!(after.untracked.contains(&"user_notes.txt".to_owned()));

    // The commit contains ONLY Lumi's file.
    let show = Command::new("git")
        .args(["show", "--name-only", "--pretty=format:", "HEAD"])
        .current_dir(&root)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .output()
        .unwrap();
    let files_in_commit = String::from_utf8_lossy(&show.stdout).to_string();
    let files_in_commit: Vec<&str> = files_in_commit.lines().collect();
    assert_eq!(files_in_commit, vec!["lumi_change.txt"]);

    // Baseline delta: everything still dirty after the commit was ALREADY
    // dirty before it (the three-way distinction substrate).
    assert_eq!(before.changed_count(), after.changed_count());
    std::fs::remove_dir_all(&root).ok();
}

fn test_principal() -> lumi_protocol::Principal {
    lumi_protocol::Principal {
        principal_id: lumi_protocol::PrincipalId::parse("u-1").unwrap(),
        tenant_id: lumi_protocol::TenantId::parse("t-1").unwrap(),
        kind: lumi_protocol::PrincipalKind::User,
        authenticated_at: Some(lumi_protocol::Timestamp::from_epoch(0, 0).unwrap()),
        authentication_strength: Some(lumi_protocol::AuthenticationStrength::DevicePossession),
    }
}

#[test]
fn local_branch_commit_and_log_round_trip() {
    let root = dirty_repo("branch");
    let repo = GitRepo::discover(&root).unwrap();

    repo.create_branch("feature/lumi-demo").unwrap();
    repo.switch("feature/lumi-demo").unwrap();
    assert_eq!(
        repo.status().unwrap().branch.as_deref(),
        Some("feature/lumi-demo")
    );
    // Dirty user state travels with the worktree, never blocks (§26.19).
    assert!(!repo.status().unwrap().is_clean());

    std::fs::write(root.join("feature.txt"), b"feature work\n").unwrap();
    repo.stage(&[Path::new("feature.txt")]).unwrap();
    repo.commit("feature: initial", &[Path::new("feature.txt")])
        .unwrap();

    let log = repo.log(5).unwrap();
    assert_eq!(log[0].subject, "feature: initial");
    assert!(log.len() >= 2, "history preserved: {log:?}");
    let branches = repo.branches().unwrap();
    assert!(branches.contains(&"main".to_owned()));
    assert!(branches.contains(&"feature/lumi-demo".to_owned()));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn worktree_isolation_creates_linked_checkout_outside_the_repo() {
    let root = dirty_repo("worktree");
    let repo = GitRepo::discover(&root).unwrap();
    let wt_parent = unique_dir("wt-parent");
    let wt_path = wt_parent.join("linked");

    // Isolated worktrees live OUTSIDE the project root (a Lumi depot);
    // creating one is a local write on the repo.
    repo.create_worktree(&wt_path, "feature/isolated").unwrap();

    let trees = repo.worktrees().unwrap();
    assert_eq!(trees.len(), 2, "main + linked");
    assert!(
        trees
            .iter()
            .any(|t| t.branch.as_deref() == Some("feature/isolated")),
        "{trees:?}"
    );

    // The linked checkout is its own repository root with its own
    // branch, and shares history.
    let linked = GitRepo::discover(&wt_path).unwrap();
    let expected_root = lumi_project::normalize_path(std::fs::canonicalize(&wt_path).unwrap());
    assert_eq!(linked.root(), &expected_root);
    assert_eq!(
        linked.status().unwrap().branch.as_deref(),
        Some("feature/isolated")
    );
    assert!(linked.log(1).unwrap().len() == 1);

    // Staging through the MAIN repo must only touch main-repo paths:
    // the linked worktree's files are out of scope there.
    std::fs::write(wt_path.join("isolated.txt"), b"isolated\n").unwrap();
    let err = repo.stage(&[&wt_path.join("isolated.txt")]).unwrap_err();
    assert!(
        matches!(err, lumi_project::GitError::OutsideRepo { .. }),
        "{err:?}"
    );

    std::fs::remove_dir_all(&wt_parent).ok();
    git(&root, &["worktree", "prune"]);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn stage_refuses_paths_outside_the_repo() {
    let root = dirty_repo("stage-scope");
    let repo = GitRepo::discover(&root).unwrap();
    let outside = unique_dir("stage-outside");
    std::fs::write(outside.join("secret.txt"), b"no").unwrap();

    let err = repo.stage(&[&outside.join("secret.txt")]).unwrap_err();
    assert!(
        matches!(err, lumi_project::GitError::OutsideRepo { .. }),
        "{err:?}"
    );

    // Absolute in-repo paths are fine.
    std::fs::write(root.join("in_repo.txt"), b"yes").unwrap();
    repo.stage(&[&root.join("in_repo.txt")]).unwrap();
    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&outside).ok();
}

#[test]
fn discover_requires_a_repository() {
    let plain = unique_dir("no-repo");
    assert!(GitRepo::discover(&plain).is_none());
    std::fs::remove_dir_all(&plain).ok();
}

#[test]
fn project_open_records_git_metadata() {
    // The store-level integration: opening a git repo advertises
    // git_read capability and records repository metadata.
    let root = dirty_repo("open-meta");
    let home = unique_dir("open-meta-home");
    let mut store = lumi_project::ProjectStore::new(home.join("projects.json"));
    let request = lumi_project::OpenFolderRequest {
        tenant_id: lumi_protocol::TenantId::parse("t-1").unwrap(),
        owner: test_principal(),
        execution_environment_id: lumi_protocol::EnvironmentId::parse("env-1").unwrap(),
        path: root.clone(),
        display_name: None,
        policy_reference: None,
    };
    let outcome = store
        .open_folder(
            &request,
            lumi_protocol::Timestamp::from_epoch(0, 0).unwrap(),
        )
        .unwrap();
    let record = outcome.record();
    assert!(record.capability_snapshot.contains(&"git_read".to_owned()));
    assert!(record
        .git_metadata
        .as_ref()
        .is_some_and(|g| g.is_repository));
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn git_status_on_unborn_branch_is_explicit_not_a_crash() {
    // Freshly initialized repo with no commits: the unborn branch must
    // be handled without error (§26.30: unexpected git state is
    // explicit, never a crash or silent reinterpretation).
    let root = unique_dir("unborn");
    git(&root, &["init", "--initial-branch=main"]);
    let repo = GitRepo::discover(&root).unwrap();
    let status = repo.status().unwrap();
    assert_eq!(status.branch, Some("main".to_owned()));
    assert_eq!(status.head, None, "no commits yet");
    assert!(status.is_clean());
    assert!(repo.head_hash().unwrap().is_none());
    std::fs::remove_dir_all(&root).ok();
}
