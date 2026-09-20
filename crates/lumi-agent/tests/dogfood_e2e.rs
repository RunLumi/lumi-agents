//! Dogfood harness end-to-end certification: a real git repository is
//! seeded, the harness clones it, runs the fixture-driven delegation
//! loop through the orchestrator gate, and the independent checks must
//! all pass — and must FAIL when the fixture's claims don't hold.

use lumi_agent::dogfood::{run_dogfood, DogfoodConfig};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn depot(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "lumi-dogfood-e2e-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct DepotGuard(PathBuf);

impl Drop for DepotGuard {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .status()
        .expect("git available");
    assert!(status.success(), "git {args:?} failed");
}

fn seed_repo(source: &Path) {
    std::fs::create_dir_all(source).unwrap();
    std::fs::write(
        source.join("README.md"),
        "# Dogfood Target\n\nA real repository for the harness to act on.\n",
    )
    .unwrap();
    git(source, &["init", "-q"]);
    git(
        source,
        &[
            "-c",
            "user.email=dogfood@example.test",
            "-c",
            "user.name=Dogfood",
            "add",
            "-A",
        ],
    );
    git(
        source,
        &[
            "-c",
            "user.email=dogfood@example.test",
            "-c",
            "user.name=Dogfood",
            "commit",
            "-q",
            "-m",
            "seed",
        ],
    );
}

const SCENARIO: &str = r##"{
  "scenario": "e2e-note",
  "description": "Harness e2e scenario.",
  "goal": "Read README.md, write repo/notes/e2e.md.",
  "turns": [
    { "type": "tool_calls", "calls": [
      { "name": "read_file", "arguments": { "path": "repo/README.md" } }
    ]},
    { "type": "tool_calls", "calls": [
      { "name": "write_file", "arguments": {
          "path": "repo/notes/e2e.md",
          "content": "# E2E\n\nProduced through the gated loop.\n" } }
    ]},
    { "type": "tool_calls", "calls": [
      { "name": "run_shell", "arguments": { "command": "git", "args": ["-C", "repo", "status", "--porcelain"] } }
    ]},
    { "type": "answer", "content": "Done." }
  ],
  "expected_files": ["repo/notes/e2e.md"]
}"##;

#[test]
fn dogfood_pass_verifies_end_to_end_on_a_real_repo() {
    let guard = DepotGuard(depot("ok"));
    let source = guard.0.join("source");
    seed_repo(&source);
    let fixture_path = guard.0.join("scenario.json");
    std::fs::write(&fixture_path, SCENARIO).unwrap();

    let report = run_dogfood(&DogfoodConfig {
        source_repo: source.clone(),
        fixture_path: fixture_path.clone(),
        work_dir: guard.0.join("work"),
        provider: None,
    })
    .unwrap();

    assert!(
        report.verified,
        "report: {}",
        serde_json::to_string_pretty(&report).unwrap()
    );
    let check_names: Vec<&str> = report.checks.iter().map(|c| c.check.as_str()).collect();
    assert!(check_names.contains(&"loop_completed"));
    assert!(check_names.contains(&"expected_file"));
    assert!(check_names.contains(&"git_working_tree_dirty"));
    assert!(check_names.contains(&"durable_task_completed"));
    assert!(check_names.contains(&"audit_chain_intact"));
    assert_eq!(report.audit_chain, "intact");
    assert_eq!(report.policy_denials, 0);
}

#[test]
fn dogfood_pass_fails_when_the_claimed_work_is_absent() {
    let guard = DepotGuard(depot("claim"));
    let source = guard.0.join("source");
    seed_repo(&source);

    // The scenario CLAIMS a second file it never produces.
    let lying_scenario = SCENARIO.replace(
        r#""expected_files": ["repo/notes/e2e.md"]"#,
        r#""expected_files": ["repo/notes/e2e.md", "repo/notes/never-written.md"]"#,
    );
    let fixture_path = guard.0.join("scenario.json");
    std::fs::write(&fixture_path, lying_scenario).unwrap();

    let report = run_dogfood(&DogfoodConfig {
        source_repo: source.clone(),
        fixture_path,
        work_dir: guard.0.join("work"),
        provider: None,
    })
    .unwrap();

    assert!(!report.verified, "unmet claims must fail the evaluation");
    let missing = report
        .checks
        .iter()
        .find(|c| c.check == "expected_file" && !c.passed)
        .expect("missing expected_file check");
    assert!(missing.detail.contains("never-written.md"));
}
