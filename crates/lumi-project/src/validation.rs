//! Project validation execution (spec 26 §26.14, §26.20).
//!
//! Validations (tests, lint, build, format) run through the same
//! bounded shell engine as everything else — cwd bound to the task
//! workspace/project root, environment allowlist, deadline, capped
//! output, declared network policy, cancellation. Outcomes are honest
//! five-state results; "files written" is never "task completed"
//! (§26.20). Discovered commands are executed only when a caller
//! explicitly submits them — discovery proposals carry no authority.

use crate::error::ProjectError;
use crate::record::ProjectRecord;
use lumi_protocol::Timestamp;
use lumi_workspaces::{run_spec, IsolationClass, NetworkPolicy, ShellSpec, ShellStatus};
use std::path::Path;

/// Honest validation result vocabulary (spec 08 §8.13, spec 26 §26.20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    /// The validation command completed successfully.
    Passed,
    /// The validation command ran and reported failure.
    Failed,
    /// The caller chose not to run this validation.
    Skipped,
    /// The validation could not run (tool missing, spawn failure).
    Unavailable,
    /// The result could not be determined (timeout, cancellation).
    Ambiguous,
}

/// One executed validation with its evidence tail.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ValidationRecord {
    /// What role this validation played ("test", "lint", ...).
    pub role: String,
    /// The exact command that ran (from the project root).
    pub command: String,
    pub status: ValidationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    /// Bounded output tail for evidence (§26.31 minimization).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tail: Option<String>,
    pub recorded_at: Timestamp,
}

/// Executes `program` with `args` at the project root under spec 08
/// bounds and records the honest outcome.
///
/// # Errors
/// [`ProjectError::RootMissing`] when the root is gone; the command
/// result itself is a value, not an error.
pub fn run_validation(
    project: &ProjectRecord,
    role: &str,
    command: &str,
    timeout_ms: u64,
) -> Result<ValidationRecord, ProjectError> {
    let (program, args) = split_command(command)
        .ok_or_else(|| ProjectError::Io(format!("unbalanced quoting in command: {command}")))?;
    run_validation_args(
        project,
        role,
        &program,
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
        timeout_ms,
    )
}

/// Args-based validation: `program` and `args` are passed to the child
/// verbatim (no shell, no quoting ambiguity).
///
/// # Errors
/// [`ProjectError::RootMissing`] when the root is gone.
pub fn run_validation_args(
    project: &ProjectRecord,
    role: &str,
    program: &str,
    args: &[&str],
    timeout_ms: u64,
) -> Result<ValidationRecord, ProjectError> {
    let root = &project.primary_root;
    if !root.exists() {
        return Err(ProjectError::RootMissing {
            path: root.display().to_string(),
        });
    }
    let command = [program]
        .into_iter()
        .chain(args.iter().copied())
        .collect::<Vec<_>>()
        .join(" ");
    let outcome = run_spec(
        Path::new(root),
        &ShellSpec {
            program: program.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            // Validation commands are Lumi-submitted: the environment is
            // minimal, never inherited (no host secrets reach builds).
            env_allowlist: vec![],
            network: NetworkPolicy::Denied,
            timeout_ms,
            max_output_bytes: 1_000_000,
            min_isolation: IsolationClass::HostBounded,
        },
        IsolationClass::HostBounded,
        None,
    );
    let status = match outcome.status {
        ShellStatus::Completed => ValidationStatus::Passed,
        ShellStatus::Failed(-1) => ValidationStatus::Unavailable,
        ShellStatus::Failed(_) => ValidationStatus::Failed,
        ShellStatus::TimedOut | ShellStatus::Cancelled => ValidationStatus::Ambiguous,
        ShellStatus::SpawnError => ValidationStatus::Unavailable,
    };
    let exit_code = match outcome.status {
        ShellStatus::Completed => Some(0),
        ShellStatus::Failed(code) => Some(code),
        _ => None,
    };
    let tail = output_tail(&outcome.stdout, &outcome.stderr);
    Ok(ValidationRecord {
        role: role.to_owned(),
        command: command.to_owned(),
        status,
        exit_code,
        duration_ms: outcome.duration_ms,
        output_tail: tail,
        recorded_at: Timestamp::now(),
    })
}

fn output_tail(stdout: &str, stderr: &str) -> Option<String> {
    let mut combined = String::new();
    if !stdout.trim().is_empty() {
        combined.push_str(stdout.trim_end());
    }
    if !stderr.trim().is_empty() {
        if !combined.is_empty() {
            combined.push_str("\n--- stderr ---\n");
        }
        combined.push_str(stderr.trim_end());
    }
    if combined.is_empty() {
        return None;
    }
    // Keep the LAST 4_000 chars, char-safe.
    const TAIL_CHARS: usize = 4_000;
    if combined.chars().count() > TAIL_CHARS {
        let tail: String = combined
            .chars()
            .rev()
            .take(TAIL_CHARS)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return Some(tail);
    }
    Some(combined)
}

/// Quote-aware command splitter: whitespace splits outside quotes;
/// single and double quotes group; no shell interpretation otherwise
/// (the child never runs through a shell anyway — this only decides the
/// argv boundary).
fn split_command(command: &str) -> Option<(String, Vec<String>)> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;
    for c in command.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => current.push(c),
            None if c == '\'' || c == '"' => {
                quote = Some(c);
                started = true;
            }
            None if c.is_whitespace() => {
                if started {
                    parts.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            None => {
                current.push(c);
                started = true;
            }
        }
    }
    if quote.is_some() {
        return None; // unbalanced quotes
    }
    if started {
        parts.push(current);
    }
    let program = parts.first()?.clone();
    Some((program, parts.into_iter().skip(1).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_command_respects_quotes() {
        let (program, args) = split_command("sh -c 'test -z \"$LUMI_TEST_SECRET\"'").unwrap();
        assert_eq!(program, "sh");
        assert_eq!(args, vec!["-c", "test -z \"$LUMI_TEST_SECRET\""]);
        assert!(split_command("unterminated 'quote").is_none());
    }

    #[test]
    fn output_tail_bounds_output() {
        let big = "x".repeat(10_000);
        let tail = output_tail(&big, "").unwrap();
        assert!(tail.chars().count() <= 4_000);
        assert!(output_tail("", "").is_none());
    }
}

#[cfg(test)]
mod shell_tests {
    use super::*;
    use crate::record::{AuthorizedRoot, DetectedSource, IndexingState, RootAccess};
    use lumi_protocol::ids::{EnvironmentId, PrincipalId, ProjectId};
    use lumi_protocol::principal::{AuthenticationStrength, Principal, PrincipalKind};
    use lumi_protocol::TenantId;
    use std::path::PathBuf;

    fn unique_dir(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "lumi-project-val-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::canonicalize(&dir).unwrap()
    }

    fn project_at(root: PathBuf) -> ProjectRecord {
        let now = Timestamp::from_epoch(0, 0).unwrap();
        let record = ProjectRecord {
            project_id: ProjectId::parse("p-val").unwrap(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            owner: Principal {
                principal_id: PrincipalId::parse("u-1").unwrap(),
                tenant_id: TenantId::parse("t-1").unwrap(),
                kind: PrincipalKind::User,
                authenticated_at: Some(now),
                authentication_strength: Some(AuthenticationStrength::DevicePossession),
            },
            execution_environment_id: EnvironmentId::parse("env-1").unwrap(),
            display_name: "val".to_owned(),
            primary_root: root.clone(),
            authorized_roots: vec![AuthorizedRoot {
                root_id: "primary".to_owned(),
                path: root,
                access: RootAccess::ReadWrite,
            }],
            created_at: now,
            last_opened_at: now,
            detected_source: DetectedSource::Generic,
            capability_snapshot: vec![],
            policy_reference: None,
            instruction_sources: vec![],
            indexing_state: IndexingState::NotIndexed,
            git_metadata: None,
        };
        record.validate().unwrap();
        record
    }

    #[cfg(unix)]
    #[test]
    fn passing_and_failing_commands_report_honestly() {
        let root = unique_dir("honest");
        let project = project_at(root.clone());

        let pass = run_validation(&project, "test", "true", 10_000).unwrap();
        assert_eq!(pass.status, ValidationStatus::Passed);
        assert_eq!(pass.exit_code, Some(0));

        std::fs::write(root.join("failing_test.sh"), b"#!/bin/sh\nexit 3\n").unwrap();
        let fail = run_validation(&project, "test", "sh failing_test.sh", 10_000).unwrap();
        assert_eq!(fail.status, ValidationStatus::Failed);
        assert_eq!(fail.exit_code, Some(3));
        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn cwd_is_bound_to_project_root() {
        let root = unique_dir("cwd");
        std::fs::write(root.join("marker.txt"), b"here").unwrap();
        let project = project_at(root.clone());
        let record = run_validation(&project, "check", "ls marker.txt", 10_000).unwrap();
        assert_eq!(
            record.status,
            ValidationStatus::Passed,
            "{:?}",
            record.output_tail
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn host_secrets_are_not_inherited_by_validation_commands() {
        let root = unique_dir("secrets");
        let project = project_at(root.clone());
        // SAFETY: test-only env var for this process; value is fake.
        std::env::set_var("LUMI_TEST_SECRET", "super-secret");
        let record = run_validation(
            &project,
            "check",
            "sh -c 'test -z \"$LUMI_TEST_SECRET\"'",
            10_000,
        )
        .unwrap();
        assert_eq!(
            record.status,
            ValidationStatus::Passed,
            "secret must not leak: {:?}",
            record.output_tail
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn missing_tool_is_unavailable_not_failed() {
        let root = unique_dir("missing-tool");
        let project = project_at(root.clone());
        let record = run_validation(
            &project,
            "test",
            "lumi-definitely-not-a-tool-9x7 --version",
            10_000,
        )
        .unwrap();
        assert_eq!(record.status, ValidationStatus::Unavailable);
        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn timeout_is_ambiguous() {
        let root = unique_dir("timeout");
        let project = project_at(root.clone());
        let record = run_validation(&project, "build", "sleep 5", 300).unwrap();
        assert_eq!(record.status, ValidationStatus::Ambiguous);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn validation_on_missing_root_fails_closed() {
        let root = unique_dir("gone");
        let project = project_at(root.clone());
        std::fs::remove_dir_all(&root).unwrap();
        let err = run_validation(&project, "test", "true", 1_000).unwrap_err();
        assert!(matches!(err, ProjectError::RootMissing { .. }));
    }
}
