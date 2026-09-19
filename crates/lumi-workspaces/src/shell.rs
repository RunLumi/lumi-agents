//! Sandboxed shell execution (spec 08 §8.6–8.9).
//!
//! Every execution specifies its workspace, program, arguments,
//! environment ALLOWLIST (the host environment is never inherited — no
//! secrets leak into child processes by default), network policy,
//! timeout, output limits, and the isolation class it can honestly
//! provide. v1 provides HOST_BOUNDED execution; a workflow policy that
//! demands SANDBOXED/CONTAINERIZED/REMOTE_SANDBOX is refused up front
//! rather than silently downgraded (§8.7, fail closed).

use crate::workspace::Workspace;
use lumi_protocol::FailureCategory;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Isolation classes (spec 08 §8.7), ordered weakest to strongest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IsolationClass {
    /// Workspace-confined execution on the host: cleared environment,
    /// cwd bound to the workspace, deadline, output caps. Network
    /// restrictions are policy-recorded but not kernel-enforced in v1.
    HostBounded,
    /// OS-level sandbox (future).
    Sandboxed,
    /// Container isolation (future).
    Containerized,
    /// Remote ephemeral sandbox (future).
    RemoteSandbox,
}

/// Declared network egress policy for the execution (§8.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPolicy {
    /// No network egress authorized. Recorded on the result; with
    /// HOST_BOUNDED isolation this is a policy statement the result
    /// attests to, not a kernel guarantee.
    Denied,
    /// Network egress authorized for this execution.
    Allowed,
}

/// A fully-specified shell execution (§8.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellSpec {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// The ONLY environment variables the child receives. Never inherit
    /// the host environment (§8.6: no inherited secrets by default).
    #[serde(default)]
    pub env_allowlist: Vec<(String, String)>,
    pub network: NetworkPolicy,
    pub timeout_ms: u64,
    /// Maximum captured stdout/stderr bytes each (bounded capture).
    pub max_output_bytes: u64,
    /// Isolation this execution REQUIRES. Executions are refused when the
    /// sandbox cannot provide it (§8.7).
    pub min_isolation: IsolationClass,
}

impl Default for ShellSpec {
    fn default() -> Self {
        Self {
            program: String::new(),
            args: Vec::new(),
            env_allowlist: Vec::new(),
            network: NetworkPolicy::Denied,
            timeout_ms: 30_000,
            max_output_bytes: 1_000_000,
            min_isolation: IsolationClass::HostBounded,
        }
    }
}

/// Terminal status of a shell execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellStatus {
    /// Exited with code 0.
    Completed,
    /// Exited with a non-zero code.
    Failed(i32),
    /// Killed after the deadline (§8.14).
    TimedOut,
    /// Killed by cancellation before completion (§8.14).
    Cancelled,
    /// Could not spawn.
    SpawnError,
}

/// The captured result (§8.6 output capture).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellOutcome {
    pub status: ShellStatus,
    pub stdout: String,
    pub stderr: String,
    /// Output was truncated at `max_output_bytes`.
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub duration_ms: u64,
    pub isolation: IsolationClass,
    pub network: NetworkPolicy,
    /// Canonical failure category for ErrorEnvelope mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_category: Option<FailureCategory>,
}

impl ShellOutcome {
    /// True only for `Completed` (shell exit codes are not success).
    #[must_use]
    pub const fn succeeded(&self) -> bool {
        matches!(self.status, ShellStatus::Completed)
    }
}

/// Executes shell commands bounded to one workspace.
pub struct ShellSandbox<'a> {
    pub workspace: &'a Workspace,
    /// Isolation this sandbox can honestly provide.
    pub isolation: IsolationClass,
    /// Cancellation check polled while the child runs (§8.14 cancel).
    pub cancelled: Option<Box<dyn Fn() -> bool + 'a>>,
}

impl<'a> ShellSandbox<'a> {
    #[must_use]
    pub const fn new(workspace: &'a Workspace) -> Self {
        Self {
            workspace,
            isolation: IsolationClass::HostBounded,
            cancelled: None,
        }
    }

    #[must_use]
    pub fn with_isolation(mut self, isolation: IsolationClass) -> Self {
        self.isolation = isolation;
        self
    }

    #[must_use]
    pub fn with_cancellation(mut self, cancelled: Box<dyn Fn() -> bool + 'a>) -> Self {
        self.cancelled = Some(cancelled);
        self
    }

    /// Runs `spec` inside the workspace.
    ///
    /// # Errors
    /// Nothing is returned as `Err`: every failure (refused isolation,
    /// spawn error, timeout) is a normalized [`ShellOutcome`] so callers
    /// always get auditable evidence.
    pub fn run(&self, spec: &ShellSpec) -> ShellOutcome {
        run_spec(
            self.workspace.root(),
            spec,
            self.isolation,
            self.cancelled.as_deref(),
        )
    }
}

/// Executes one [`ShellSpec`] with its cwd bound to an explicit root —
/// the shared engine behind [`ShellSandbox`] and project-root execution
/// (spec 26 §26.14: project shell uses the same bounds as task shells).
pub fn run_spec(
    cwd: &Path,
    spec: &ShellSpec,
    isolation: IsolationClass,
    cancelled: Option<&(dyn Fn() -> bool + '_)>,
) -> ShellOutcome {
    let started = Instant::now();
    // Fail closed on isolation demands we cannot meet (§8.7).
    if spec.min_isolation > isolation {
        return ShellOutcome {
            status: ShellStatus::SpawnError,
            stdout: String::new(),
            stderr: format!(
                "execution requires {:?} isolation but sandbox provides {:?}",
                spec.min_isolation, isolation
            ),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms: 0,
            isolation,
            network: spec.network,
            failure_category: Some(FailureCategory::SecurityViolation),
        };
    }

    // Resolve the program against the PARENT's PATH before clearing
    // the child environment (a cleared env would otherwise break
    // PATH-based lookup on macOS posix_spawnp).
    let program = resolve_program(&spec.program);
    let program = match program {
        Some(path) => path,
        None => {
            return ShellOutcome {
                status: ShellStatus::SpawnError,
                stdout: String::new(),
                stderr: format!("program not found on PATH: {}", spec.program),
                stdout_truncated: false,
                stderr_truncated: false,
                duration_ms: started.elapsed().as_millis() as u64,
                isolation,
                network: spec.network,
                failure_category: Some(FailureCategory::ShellExecution),
            };
        }
    };

    let mut command = Command::new(program);
    command
        .args(&spec.args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Environment: cleared, then minimal non-secret defaults (PATH so
    // the child can spawn subprocesses; SystemRoot on Windows), then
    // the allowlist. Host secrets are never inherited (§8.6).
    command.env_clear();
    if let Ok(path) = std::env::var("PATH") {
        command.env("PATH", path);
    }
    #[cfg(windows)]
    {
        for key in ["SystemRoot", "TEMP", "TMP"] {
            if let Ok(value) = std::env::var(key) {
                command.env(key, value);
            }
        }
    }
    for (key, value) in &spec.env_allowlist {
        command.env(key, value);
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            return ShellOutcome {
                status: ShellStatus::SpawnError,
                stdout: String::new(),
                stderr: format!("spawn {}: {e}", spec.program),
                stdout_truncated: false,
                stderr_truncated: false,
                duration_ms: started.elapsed().as_millis() as u64,
                isolation,
                network: spec.network,
                failure_category: Some(FailureCategory::ShellExecution),
            };
        }
    };

    // Bounded capture: read pipes on threads, capped at the limit.
    let deadline = started + Duration::from_millis(spec.timeout_ms);
    let stdout_cap = spec.max_output_bytes as usize;
    let stderr_cap = spec.max_output_bytes as usize;
    let stdout_handle = child
        .stdout
        .take()
        .map(|pipe| std::thread::spawn(move || read_capped(pipe, stdout_cap)));
    let stderr_handle = child
        .stderr
        .take()
        .map(|pipe| std::thread::spawn(move || read_capped(pipe, stderr_cap)));

    let status = loop {
        if let Some(is_cancelled) = cancelled {
            if is_cancelled() {
                let _ = child.kill();
                let _ = child.wait();
                break ShellStatus::Cancelled;
            }
        }
        match child.try_wait() {
            Ok(Some(code)) => {
                break match code.code() {
                    Some(0) => ShellStatus::Completed,
                    Some(nonzero) => ShellStatus::Failed(nonzero),
                    None => ShellStatus::Failed(-1),
                };
            }
            Ok(None) => {}
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break ShellStatus::SpawnError;
            }
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            break ShellStatus::TimedOut;
        }
        std::thread::sleep(Duration::from_millis(10));
    };

    let (stdout, stdout_truncated) = stdout_handle
        .and_then(|h| h.join().ok())
        .unwrap_or((String::new(), false));
    let (stderr, stderr_truncated) = stderr_handle
        .and_then(|h| h.join().ok())
        .unwrap_or((String::new(), false));

    let failure_category = match status {
        ShellStatus::Completed => None,
        ShellStatus::Failed(_) => Some(FailureCategory::ShellExecution),
        ShellStatus::TimedOut => Some(FailureCategory::ShellExecution),
        ShellStatus::Cancelled => Some(FailureCategory::UserCancel),
        ShellStatus::SpawnError => Some(FailureCategory::ShellExecution),
    };

    ShellOutcome {
        status,
        stdout,
        stderr,
        stdout_truncated,
        stderr_truncated,
        duration_ms: started.elapsed().as_millis() as u64,
        isolation,
        network: spec.network,
        failure_category,
    }
}

/// Reads a pipe to EOF or the byte cap; lossy-UTF8 into a string.
fn read_capped(pipe: impl std::io::Read, cap: usize) -> (String, bool) {
    let mut handle = pipe;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut truncated = false;
    loop {
        match handle.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                let remaining = cap.saturating_sub(buf.len());
                if remaining == 0 {
                    truncated = true;
                    // Keep draining (cheaply) so the child does not block
                    // on a full pipe, but stop retaining.
                    continue;
                }
                let take = n.min(remaining);
                buf.extend_from_slice(&chunk[..take]);
                if take < n {
                    truncated = true;
                }
            }
            Err(_) => break,
        }
    }
    (String::from_utf8_lossy(&buf).into_owned(), truncated)
}

/// Resolves a program name to an absolute path using the parent process
/// PATH. Programs containing a separator are used as-is (relative to the
/// workspace cwd).
fn resolve_program(program: &str) -> Option<PathBuf> {
    if program.contains('/') || program.contains('\\') {
        return Some(PathBuf::from(program));
    }
    let path_var = std::env::var("PATH").unwrap_or_default();
    for entry in std::env::split_paths(&path_var) {
        #[cfg(windows)]
        let candidate = entry.join(format!("{program}.exe"));
        #[cfg(not(windows))]
        let candidate = entry.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let bare = entry.join(program);
            if bare.is_file() {
                return Some(bare);
            }
        }
    }
    None
}
