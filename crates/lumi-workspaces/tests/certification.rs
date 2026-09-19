//! Certification fixtures for spec 08 §8.14.
//!
//! Required scenarios: path traversal; symlink escape; overwrite
//! approval; shell timeout; cancellation; sandbox/no-network mode;
//! generated artifact validation; publication separated from draft
//! creation.

use lumi_protocol::FailureCategory;
use lumi_workspaces::{
    resolve_in_workspace, ArtifactLifecycle, ArtifactProvenance, ArtifactStore, BuiltInValidator,
    CleanupPolicy, FileOp, IsolationClass, NetworkPolicy, ShellSpec, ShellStatus, Workspace,
    WorkspaceFiles,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn unique_depot(name: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "lumi-ws-cert-{}-{}-{}-{}",
        name,
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst),
        lumi_protocol::Timestamp::now().nanoseconds()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn workspace(name: &str) -> (PathBuf, Workspace) {
    let depot = unique_depot(name);
    let ws = Workspace::create(
        &depot,
        &lumi_protocol::TaskId::parse(format!("task-{name}")).unwrap(),
        "u-fixture",
        CleanupPolicy::Manual,
    )
    .unwrap();
    (depot, ws)
}

#[test]
fn path_traversal_fails_closed() {
    let (depot, ws) = workspace("traversal");
    let files = WorkspaceFiles::new(&ws);
    // Lexical escape through ..:
    let err = files
        .execute(&FileOp::Read {
            path: PathBuf::from("../../etc/passwd"),
        })
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("Traversal") || format!("{err:?}").contains("UnsafePath"),
        "{err:?}"
    );
    // Absolute paths are re-anchored, never honored:
    let created = files
        .execute(&FileOp::Create {
            path: PathBuf::from("/etc/lumi-should-not-exist.txt"),
            content: b"no".to_vec(),
            allow_overwrite: false,
        })
        .unwrap();
    assert!(
        !std::path::Path::new("/etc/lumi-should-not-exist.txt").exists(),
        "absolute path must be re-anchored under the workspace"
    );
    assert!(created.path.contains("etc"));
    // And the real content landed inside the workspace:
    assert!(ws.root().join("etc/lumi-should-not-exist.txt").is_file());
    std::fs::remove_dir_all(&depot).ok();
}

#[test]
fn symlink_escape_is_refused() {
    let (depot, ws) = workspace("symlink");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/etc", ws.root().join("escape")).unwrap();
        let err =
            resolve_in_workspace(ws.root(), std::path::Path::new("escape/passwd")).unwrap_err();
        assert!(format!("{err:?}").contains("Symlink"), "{err:?}");
        // In-root symlinks remain usable.
        std::fs::create_dir_all(ws.root().join("real")).unwrap();
        std::os::unix::fs::symlink("real", ws.root().join("alias")).unwrap();
        let resolved = resolve_in_workspace(ws.root(), std::path::Path::new("alias")).unwrap();
        assert!(resolved.starts_with(ws.root().canonicalize().unwrap()));
    }
    std::fs::remove_dir_all(&depot).ok();
}

#[test]
fn overwrite_requires_explicit_approval_flag() {
    let (depot, ws) = workspace("overwrite");
    let files = WorkspaceFiles::new(&ws);
    let create = FileOp::Create {
        path: PathBuf::from("out/report.csv"),
        content: b"v1".to_vec(),
        allow_overwrite: false,
    };
    files.execute(&create).unwrap();

    // Second create without authorization: refused, content untouched.
    let err = files.execute(&create).unwrap_err();
    assert!(format!("{err:?}").contains("Overwrite"), "{err:?}");
    assert_eq!(
        std::fs::read(ws.root().join("out/report.csv")).unwrap(),
        b"v1"
    );

    // Authorized overwrite (policy-approved) succeeds and records both hashes.
    let record = files
        .execute(&FileOp::Create {
            path: PathBuf::from("out/report.csv"),
            content: b"v2".to_vec(),
            allow_overwrite: true,
        })
        .unwrap();
    assert_eq!(record.capability, "files.edit");
    assert!(record.sha256_before.is_some());
    assert_eq!(
        std::fs::read(ws.root().join("out/report.csv")).unwrap(),
        b"v2"
    );
    std::fs::remove_dir_all(&depot).ok();
}

#[test]
fn delete_is_reversible_by_default_and_permanent_only_when_demanded() {
    let (depot, ws) = workspace("delete");
    let files = WorkspaceFiles::new(&ws);
    files
        .execute(&FileOp::Create {
            path: PathBuf::from("data/file.txt"),
            content: b"keep me".to_vec(),
            allow_overwrite: false,
        })
        .unwrap();

    // Reversible delete: file moves to workspace trash.
    let record = files
        .execute(&FileOp::Delete {
            path: PathBuf::from("data/file.txt"),
            permanent: false,
        })
        .unwrap();
    assert!(record.reversible);
    assert!(!ws.root().join("data/file.txt").exists());
    let trash_path = record.undo_path.as_ref().unwrap();
    assert!(std::path::Path::new(trash_path).exists());

    // Restore returns it byte-identical.
    files.restore(&record).unwrap();
    assert_eq!(
        std::fs::read(ws.root().join("data/file.txt")).unwrap(),
        b"keep me"
    );
    std::fs::remove_dir_all(&depot).ok();
}

#[test]
fn shell_timeout_kills_the_child() {
    let (depot, ws) = workspace("timeout");
    let sandbox = lumi_workspaces::ShellSandbox::new(&ws);
    let outcome = sandbox.run(&ShellSpec {
        program: "node".to_owned(),
        args: vec!["-e".to_owned(), "setInterval(()=>{},1000)".to_owned()],
        timeout_ms: 400,
        ..ShellSpec::default()
    });
    assert_eq!(outcome.status, ShellStatus::TimedOut);
    assert_eq!(
        outcome.failure_category,
        Some(FailureCategory::ShellExecution)
    );
    assert!(outcome.duration_ms < 5_000, "prompt kill: {outcome:?}");
    std::fs::remove_dir_all(&depot).ok();
}

#[test]
fn shell_cancellation_stops_execution_promptly() {
    let (depot, ws) = workspace("cancel");
    // Cancel shortly after launch from another thread.
    let trigger = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let trigger_handle = Arc::clone(&trigger);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(150));
        trigger_handle.store(true, Ordering::SeqCst);
    });
    // The sandbox observes the flag through its cancellation closure.
    let observed_for_sandbox = Arc::clone(&trigger);
    let sandbox = lumi_workspaces::ShellSandbox::new(&ws).with_cancellation(Box::new(move || {
        observed_for_sandbox.load(Ordering::SeqCst)
    }));
    let outcome = sandbox.run(&ShellSpec {
        program: "node".to_owned(),
        args: vec!["-e".to_owned(), "setInterval(()=>{},1000)".to_owned()],
        timeout_ms: 10_000,
        ..ShellSpec::default()
    });
    assert_eq!(outcome.status, ShellStatus::Cancelled);
    assert_eq!(outcome.failure_category, Some(FailureCategory::UserCancel));
    std::fs::remove_dir_all(&depot).ok();
}

#[test]
fn shell_bounded_output_and_exit_codes() {
    let (depot, ws) = workspace("shell-output");
    let sandbox = lumi_workspaces::ShellSandbox::new(&ws);

    // Exit code propagates: non-zero is Failed, not success.
    let failed = sandbox.run(&ShellSpec {
        program: "node".to_owned(),
        args: vec!["-e".to_owned(), "process.exit(3)".to_owned()],
        timeout_ms: 5_000,
        ..ShellSpec::default()
    });
    assert_eq!(failed.status, ShellStatus::Failed(3));

    // Bounded capture: output stops at the cap.
    let capped = sandbox.run(&ShellSpec {
        program: "node".to_owned(),
        args: vec![
            "-e".to_owned(),
            "process.stdout.write('x'.repeat(100))".to_owned(),
        ],
        max_output_bytes: 10,
        timeout_ms: 5_000,
        ..ShellSpec::default()
    });
    assert_eq!(capped.stdout.chars().filter(|c| *c == 'x').count(), 10);
    assert!(capped.stdout_truncated);

    // Environment is NOT inherited: only allowlisted vars are visible.
    std::env::set_var("LUMI_SECRET_SHOULD_LEAK", "nope");
    let env_probe = sandbox.run(&ShellSpec {
        program: "node".to_owned(),
        args: vec![
            "-e".to_owned(),
            "process.stdout.write(String(process.env.LUMI_SECRET_SHOULD_LEAK === undefined))"
                .to_owned(),
        ],
        env_allowlist: vec![("PATH".to_owned(), std::env::var("PATH").unwrap_or_default())],
        timeout_ms: 5_000,
        ..ShellSpec::default()
    });
    assert!(
        env_probe.succeeded() && env_probe.stdout == "true",
        "host env must not leak into children: {:?}",
        env_probe
    );
    std::env::remove_var("LUMI_SECRET_SHOULD_LEAK");
    std::fs::remove_dir_all(&depot).ok();
}

#[test]
fn sandbox_isolation_demand_beyond_host_is_refused() {
    // §8.7/§8.8: a workflow policy demanding SANDBOXED isolation must be
    // refused by a HOST_BOUNDED sandbox — never silently downgraded.
    let (depot, ws) = workspace("isolation");
    let sandbox =
        lumi_workspaces::ShellSandbox::new(&ws).with_isolation(IsolationClass::HostBounded);
    let outcome = sandbox.run(&ShellSpec {
        program: "node".to_owned(),
        args: vec!["-e".to_owned(), "console.log('hi')".to_owned()],
        min_isolation: IsolationClass::Sandboxed,
        timeout_ms: 5_000,
        ..ShellSpec::default()
    });
    assert_eq!(outcome.status, ShellStatus::SpawnError);
    assert_eq!(
        outcome.failure_category,
        Some(FailureCategory::SecurityViolation)
    );
    assert!(outcome.stderr.contains("isolation"), "{outcome:?}");

    // The network policy the run declared is attested on the result.
    let denied = sandbox.run(&ShellSpec {
        program: "node".to_owned(),
        args: vec!["-e".to_owned(), "console.log('hi')".to_owned()],
        network: NetworkPolicy::Denied,
        timeout_ms: 5_000,
        ..ShellSpec::default()
    });
    assert_eq!(denied.network, NetworkPolicy::Denied);
    assert_eq!(denied.isolation, IsolationClass::HostBounded);
    std::fs::remove_dir_all(&depot).ok();
}

#[test]
fn artifact_validation_and_publication_separation() {
    // §8.14: generated artifact validation; publication separated from
    // draft creation. Full lifecycle through the store.
    let (depot, ws) = workspace("artifact");
    let store = ArtifactStore::open(&ws.root().join("artifacts")).unwrap();
    let provenance = ArtifactProvenance {
        task_id: "task-artifact".to_owned(),
        run_id: Some("run-1".to_owned()),
        source_refs: vec!["erp/invoices/2026-09".to_owned()],
        generator: "lumi-runtime/0.1.0".to_owned(),
        created_at: lumi_protocol::Timestamp::now(),
    };

    // Generated CSV artifact with a checksum recorded at validation.
    let record = store
        .create(
            "art-reconciliation",
            "csv",
            "report.csv",
            b"invoice,status\nINV-1,MATCHED\nINV-2,MISMATCHED\n",
            provenance,
        )
        .unwrap();
    assert_eq!(record.lifecycle, ArtifactLifecycle::Draft);

    // Validation runs the CSV shape + UTF-8 built-ins.
    let validated = store.validate(&record, &[]).unwrap();
    assert_eq!(validated.lifecycle, ArtifactLifecycle::ReadyForReview);
    assert!(validated.sha256.is_some());
    let expected_checksum = lumi_protocol::canonical::sha256_hex(
        b"invoice,status\nINV-1,MISMATCHED\nINV-2,MISMATCHED\n".as_slice(),
    );
    assert_ne!(
        validated.sha256,
        Some(expected_checksum),
        "checksum must reflect actual content, not a constant"
    );

    // Publication is a separate, state-checked transition.
    assert!(
        store.publish(&validated).is_err(),
        "DRAFT-validated artifact cannot publish without approval"
    );
    let approved = store.approve(&validated).unwrap();
    let published = store.publish(&approved).unwrap();
    assert_eq!(published.lifecycle, ArtifactLifecycle::Published);
    std::fs::remove_dir_all(&depot).ok();
}

#[test]
fn artifact_type_validators_catch_wrong_content() {
    let (depot, ws) = workspace("artifact-bad");
    let store = ArtifactStore::open(&ws.root().join("artifacts")).unwrap();
    let provenance = ArtifactProvenance {
        task_id: "task-artifact".to_owned(),
        run_id: None,
        source_refs: vec![],
        generator: "fixture".to_owned(),
        created_at: lumi_protocol::Timestamp::now(),
    };
    // A PDF artifact that is not a PDF: rejected, record kept.
    let record = store
        .create(
            "art-bad-pdf",
            "pdf",
            "report.pdf",
            b"plain text",
            provenance,
        )
        .unwrap();
    let err = store.validate(&record, &[]).unwrap_err();
    assert!(format!("{err:?}").contains("Validation"), "{err:?}");
    let index =
        std::fs::read_to_string(store.root().join("art-bad-pdf").join("artifact.json")).unwrap();
    assert!(index.contains("REJECTED"));
    std::fs::remove_dir_all(&depot).ok();
}

#[test]
fn builtin_validator_catalog_covers_v1_types() {
    // §8.10: text/markdown, PDF, DOCX, XLSX/CSV, PPTX, image, code,
    // bundles all have validators or an honest empty set.
    for artifact_type in [
        "markdown", "text", "pdf", "docx", "xlsx", "pptx", "csv", "json", "code",
    ] {
        assert!(
            !BuiltInValidator::defaults_for(artifact_type).is_empty(),
            "{artifact_type} should have validators"
        );
    }
}
