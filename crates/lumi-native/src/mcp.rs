//! Minimal modern MCP stdio client for the pinned Cua Driver.
//!
//! This module is deliberately private to the native adapter: workflow and
//! policy code continue to speak Lumi's [`DesktopDriver`](crate::DesktopDriver)
//! contract, never upstream tool names.

use crate::driver::NativeError;
use lumi_protocol::FailureCategory;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

pub(crate) const MCP_PROTOCOL_VERSION: &str = "2026-07-28";
const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
const MAX_SKIPPED_MESSAGES: usize = 64;

pub(crate) trait CuaTransport: Send + Sync {
    fn request(&self, method: &str, params: Value) -> Result<Value, NativeError>;
}

pub(crate) fn with_protocol_meta(params: Value) -> Result<Value, NativeError> {
    let mut object = match params {
        Value::Object(object) => object,
        _ => {
            return Err(NativeError::new(
                FailureCategory::ModelFormat,
                "MCP request params must be an object",
            ))
        }
    };
    object.insert(
        "_meta".to_owned(),
        json!({
            "io.modelcontextprotocol/protocolVersion": MCP_PROTOCOL_VERSION,
            "io.modelcontextprotocol/clientCapabilities": {}
        }),
    );
    Ok(Value::Object(object))
}

struct ProcessState {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

/// One persistent stdio transport. Session and snapshot ownership therefore
/// remain bound to one authenticated process lease instead of being recreated
/// for every action.
pub(crate) struct ProcessMcpTransport {
    state: Mutex<ProcessState>,
}

impl ProcessMcpTransport {
    pub(crate) fn spawn_with_args(binary: &str, args: &[&str]) -> Result<Self, NativeError> {
        let mut child = Command::new(binary)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                NativeError::new(
                    FailureCategory::UpstreamDriver,
                    format!("start pinned Cua MCP process: {error}"),
                )
            })?;
        let stdin = child.stdin.take().ok_or_else(|| {
            NativeError::new(FailureCategory::UpstreamDriver, "Cua MCP stdin unavailable")
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            NativeError::new(
                FailureCategory::UpstreamDriver,
                "Cua MCP stdout unavailable",
            )
        })?;
        Ok(Self {
            state: Mutex::new(ProcessState {
                child,
                stdin,
                stdout: BufReader::new(stdout),
                next_id: 1,
            }),
        })
    }
}

impl CuaTransport for ProcessMcpTransport {
    fn request(&self, method: &str, params: Value) -> Result<Value, NativeError> {
        let mut state = self.state.lock().map_err(|_| {
            NativeError::new(FailureCategory::UpstreamDriver, "Cua MCP lock poisoned")
        })?;
        if state
            .child
            .try_wait()
            .map_err(|error| {
                NativeError::new(
                    FailureCategory::UpstreamDriver,
                    format!("inspect Cua MCP process: {error}"),
                )
            })?
            .is_some()
        {
            return Err(NativeError::new(
                FailureCategory::NativeSession,
                "Cua MCP process exited",
            ));
        }
        let id = state.next_id;
        state.next_id = state.next_id.saturating_add(1);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": with_protocol_meta(params)?,
        });
        serde_json::to_writer(&mut state.stdin, &request).map_err(|error| {
            NativeError::new(
                FailureCategory::UpstreamDriver,
                format!("encode Cua MCP request: {error}"),
            )
        })?;
        state
            .stdin
            .write_all(b"\n")
            .and_then(|_| state.stdin.flush())
            .map_err(|error| {
                NativeError::new(
                    FailureCategory::NativeSession,
                    format!("write Cua MCP request: {error}"),
                )
            })?;

        for _ in 0..MAX_SKIPPED_MESSAGES {
            let mut line = String::new();
            let read = state.stdout.read_line(&mut line).map_err(|error| {
                NativeError::new(
                    FailureCategory::NativeSession,
                    format!("read Cua MCP response: {error}"),
                )
            })?;
            if read == 0 {
                return Err(NativeError::new(
                    FailureCategory::NativeSession,
                    "Cua MCP closed stdout",
                ));
            }
            if line.len() > MAX_RESPONSE_BYTES {
                return Err(NativeError::new(
                    FailureCategory::SecurityViolation,
                    "Cua MCP response exceeded the bounded payload size",
                ));
            }
            let response: Value = serde_json::from_str(line.trim()).map_err(|error| {
                NativeError::new(
                    FailureCategory::VersionIncompatible,
                    format!("invalid Cua MCP JSON response: {error}"),
                )
            })?;
            if response.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = response.get("error") {
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Cua MCP request failed");
                return Err(NativeError::new(FailureCategory::UpstreamDriver, message));
            }
            let result = response.get("result").cloned().ok_or_else(|| {
                NativeError::new(
                    FailureCategory::VersionIncompatible,
                    "Cua MCP response omitted result",
                )
            })?;
            if result
                .get("resultType")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind != "complete")
            {
                return Err(NativeError::new(
                    FailureCategory::VersionIncompatible,
                    "Cua MCP returned a non-complete result",
                ));
            }
            return Ok(result);
        }
        Err(NativeError::new(
            FailureCategory::VersionIncompatible,
            "Cua MCP did not return the matching response id",
        ))
    }
}

impl Drop for ProcessMcpTransport {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            let _ = state.child.kill();
            let _ = state.child.wait();
        }
    }
}

pub(crate) fn tool_result(result: Value) -> Result<Value, NativeError> {
    if result.get("isError").and_then(Value::as_bool) == Some(true) {
        let message = result
            .get("content")
            .and_then(Value::as_array)
            .and_then(|items| items.iter().find_map(|item| item.get("text")))
            .and_then(Value::as_str)
            .unwrap_or("Cua tool refused or failed");
        return Err(NativeError::new(FailureCategory::UpstreamDriver, message));
    }
    if let Some(value) = result.get("structuredContent") {
        return Ok(value.clone());
    }
    let text = result
        .get("content")
        .and_then(Value::as_array)
        .and_then(|items| items.iter().find_map(|item| item.get("text")))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            NativeError::new(
                FailureCategory::VersionIncompatible,
                "Cua tool result omitted structured content",
            )
        })?;
    serde_json::from_str(text).map_err(|error| {
        NativeError::new(
            FailureCategory::VersionIncompatible,
            format!("Cua tool result text was not JSON: {error}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_metadata_is_added_without_accepting_non_objects() {
        let value = with_protocol_meta(json!({"name":"list_apps"})).unwrap();
        assert_eq!(
            value["_meta"]["io.modelcontextprotocol/protocolVersion"],
            MCP_PROTOCOL_VERSION
        );
        assert!(with_protocol_meta(Value::Null).is_err());
    }

    #[test]
    fn structured_results_are_preferred_and_errors_fail_closed() {
        assert_eq!(
            tool_result(json!({"structuredContent":{"ok":true}})).unwrap(),
            json!({"ok":true})
        );
        assert!(tool_result(json!({
            "isError": true,
            "content": [{"type":"text","text":"permission denied"}]
        }))
        .is_err());
    }
}
