//! Browser worker process handle: spawn, request/response correlation,
//! timeouts, cancellation (spec 06 §6.3, §6.15).
//!
//! The worker is an unprivileged child process speaking line-delimited
//! JSON over stdio. The handle owns the child; dropping or killing it
//! cancels in-flight browser work (spec 02 §2.9).

use crate::protocol::{SessionParams, WorkerRequest, WorkerResponse};
use lumi_protocol::{ErrorEnvelope, FailureCategory};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Worker launch configuration.
#[derive(Debug, Clone)]
pub struct BrowserWorkerConfig {
    /// Command to run the worker, e.g. `node`.
    pub program: String,
    /// Script path, e.g. `workers/playwright/worker.js`.
    pub script: String,
    /// Extra program arguments.
    pub args: Vec<String>,
    /// Working directory for the child.
    pub working_dir: Option<std::path::PathBuf>,
    /// Session parameters for the first request.
    pub session: SessionParams,
}

impl BrowserWorkerConfig {
    /// Default config pointing at the shipped Playwright worker.
    #[must_use]
    pub fn playwright(script_path: impl Into<String>) -> Self {
        Self {
            program: "node".to_owned(),
            script: script_path.into(),
            args: Vec::new(),
            working_dir: None,
            session: SessionParams::default(),
        }
    }
}

/// Failures of the worker channel itself (not of browser operations).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerError {
    Spawn(String),
    Io(String),
    /// Response did not arrive within the timeout.
    Timeout,
    /// The worker exited before responding (crash path, spec 06 §6.15).
    WorkerExited,
    Protocol(String),
}

impl WorkerError {
    /// Maps channel failures to canonical envelopes for audit.
    #[must_use]
    pub fn to_envelope(&self) -> ErrorEnvelope {
        match self {
            Self::Spawn(detail) | Self::Io(detail) => {
                ErrorEnvelope::new(FailureCategory::UpstreamDriver, detail.clone())
            }
            Self::Timeout => {
                ErrorEnvelope::new(FailureCategory::UpstreamDriver, "worker response timeout")
            }
            Self::WorkerExited => ErrorEnvelope::new(
                FailureCategory::UpstreamDriver,
                "browser worker exited unexpectedly",
            ),
            Self::Protocol(detail) => {
                ErrorEnvelope::new(FailureCategory::UpstreamDriver, detail.clone())
            }
        }
    }
}

struct Shared {
    stdin: Option<ChildStdin>,
    child: Option<Child>,
}

/// A live browser worker handle.
#[derive(Clone)]
pub struct BrowserWorkerHandle {
    shared: Arc<SharedHandle>,
    next_id: Arc<std::sync::atomic::AtomicU64>,
}

struct SharedHandle {
    inner: Mutex<Shared>,
    responses: Arc<Mutex<HashMap<String, WorkerResponse>>>,
}

impl std::fmt::Debug for BrowserWorkerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserWorkerHandle").finish()
    }
}

impl BrowserWorkerHandle {
    /// Spawns the worker process and starts the reader thread.
    ///
    /// # Errors
    /// [`WorkerError::Spawn`] when the child cannot start.
    pub fn spawn(config: &BrowserWorkerConfig) -> Result<Self, WorkerError> {
        let working_dir = config
            .working_dir
            .clone()
            .map(Ok)
            .unwrap_or_else(std::env::current_dir)
            .map_err(|e| WorkerError::Spawn(e.to_string()))?;
        let mut command = Command::new(&config.program);
        command.env_clear();
        for name in [
            "PATH",
            "HOME",
            "USERPROFILE",
            "LOCALAPPDATA",
            "SystemRoot",
            "SYSTEMROOT",
            "TEMP",
            "TMP",
            "DISPLAY",
            "WAYLAND_DISPLAY",
            "XDG_RUNTIME_DIR",
            "PLAYWRIGHT_BROWSERS_PATH",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        command.arg(&config.script);
        for arg in &config.args {
            command.arg(arg);
        }
        let mut child = command
            .current_dir(working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                WorkerError::Spawn(format!("{} {}: {e}", config.program, config.script))
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| WorkerError::Spawn("no stdin".to_owned()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| WorkerError::Spawn("no stdout".to_owned()))?;

        // Shared response map: the reader thread inserts, request calls
        // remove by correlation id.
        let responses: Arc<Mutex<HashMap<String, WorkerResponse>>> =
            Arc::new(Mutex::new(HashMap::new()));
        {
            let sink = Arc::clone(&responses);
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(response) = serde_json::from_str::<WorkerResponse>(&line) {
                        if let Ok(mut map) = sink.lock() {
                            map.insert(response.id.clone(), response);
                        }
                    }
                }
            });
        }

        Ok(Self {
            shared: Arc::new(SharedHandle {
                inner: Mutex::new(Shared {
                    stdin: Some(stdin),
                    child: Some(child),
                }),
                responses,
            }),
            next_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        })
    }

    fn request_id(&self) -> String {
        let n = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("r{n}")
    }

    /// Sends one operation and waits for the correlated response.
    ///
    /// # Errors
    /// [`WorkerError`] on IO/timeout/worker exit.
    pub fn request(
        &self,
        op: crate::protocol::BrowserOp,
        timeout: Duration,
    ) -> Result<WorkerResponse, WorkerError> {
        self.request_inner(op, None, timeout)
    }

    /// Sends a request carrying session params (the first request).
    ///
    /// # Errors
    /// [`WorkerError`] on IO/timeout/worker exit.
    pub fn request_with_session(
        &self,
        op: crate::protocol::BrowserOp,
        session: SessionParams,
        timeout: Duration,
    ) -> Result<WorkerResponse, WorkerError> {
        self.request_inner(op, Some(session), timeout)
    }

    fn request_inner(
        &self,
        op: crate::protocol::BrowserOp,
        session: Option<SessionParams>,
        timeout: Duration,
    ) -> Result<WorkerResponse, WorkerError> {
        let request = WorkerRequest {
            id: self.request_id(),
            op,
            session,
        };
        let id = request.id.clone();
        let line =
            serde_json::to_string(&request).map_err(|e| WorkerError::Protocol(e.to_string()))?;

        {
            let mut guard = self
                .shared
                .inner
                .lock()
                .map_err(|_| WorkerError::Protocol("poisoned handle".to_owned()))?;
            let stdin = guard.stdin.as_mut().ok_or(WorkerError::WorkerExited)?;
            stdin
                .write_all(line.as_bytes())
                .and_then(|_| stdin.write_all(b"\n"))
                .and_then(|_| stdin.flush())
                .map_err(|e| WorkerError::Io(e.to_string()))?;
        }

        // Poll for the correlated response; the reader thread owns stdout.
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if let Some(response) = self
                .shared
                .responses
                .lock()
                .ok()
                .and_then(|mut m| m.remove(&id))
            {
                return Ok(response);
            }
            let exited = {
                let mut guard = self
                    .shared
                    .inner
                    .lock()
                    .map_err(|_| WorkerError::Protocol("poisoned handle".to_owned()))?;
                guard
                    .child
                    .as_mut()
                    .and_then(|c| c.try_wait().ok())
                    .map(|s| s.is_some())
                    .unwrap_or(true)
            };
            if exited {
                // Give the reader a beat to flush the final response.
                std::thread::sleep(Duration::from_millis(20));
                if let Some(response) = self
                    .shared
                    .responses
                    .lock()
                    .ok()
                    .and_then(|mut m| m.remove(&id))
                {
                    return Ok(response);
                }
                return Err(WorkerError::WorkerExited);
            }
            if std::time::Instant::now() >= deadline {
                return Err(WorkerError::Timeout);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// True when the child process has exited.
    #[must_use]
    pub fn has_exited(&self) -> bool {
        self.shared
            .inner
            .lock()
            .ok()
            .and_then(|mut guard| {
                guard
                    .child
                    .as_mut()
                    .and_then(|c| c.try_wait().ok())
                    .map(|s| s.is_some())
            })
            .unwrap_or(true)
    }

    /// Cancels in-flight work by killing the worker.
    pub fn kill(&self) {
        if let Ok(mut guard) = self.shared.inner.lock() {
            if let Some(child) = guard.child.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
            guard.child = None;
            guard.stdin = None;
        }
    }
}

impl Drop for BrowserWorkerHandle {
    fn drop(&mut self) {
        if Arc::strong_count(&self.shared) == 1 {
            self.kill();
        }
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    #[test]
    #[ignore = "subprocess fixture invoked only by the lifecycle regression"]
    fn sleeping_child() {
        std::thread::sleep(Duration::from_secs(60));
    }
    #[test]
    fn dropping_a_clone_does_not_stop_the_shared_worker() {
        let config = BrowserWorkerConfig {
            program: std::env::current_exe()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            script: "--ignored".into(),
            args: vec![
                "--exact".into(),
                "worker::lifecycle_tests::sleeping_child".into(),
            ],
            working_dir: None,
            session: SessionParams::default(),
        };
        let handle = BrowserWorkerHandle::spawn(&config).unwrap();
        drop(handle.clone());
        std::thread::sleep(Duration::from_millis(40));
        assert!(!handle.has_exited());
        handle.kill();
        assert!(handle.has_exited());
    }
}
