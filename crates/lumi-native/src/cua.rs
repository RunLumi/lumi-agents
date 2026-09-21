//! Cua upstream configuration and pinning (spec 07 §7.10).
//!
//! The bundled upstream binary MUST be exact-version, checksum-verified,
//! license-recorded, and update-owner-recorded. The runtime never
//! downloads "latest". The adapter speaks the Lumi DesktopDriver
//! interface; workflow definitions never see Cua tool names (§7.2).

use crate::driver::{
    CollateralReport, DesktopDriver, EffectOracle, NativeError, NativeOperation, NativeOutcome,
    NativeResult, RefusalReason,
};
use crate::mcp::{tool_result, CuaTransport, ProcessMcpTransport};
use crate::session::{
    PermissionRequirement, PermissionState, RuntimeGeneration, SessionHandle, SessionState,
};
use crate::target::{DeliveryMode, SemanticTarget};
use lumi_protocol::canonical::sha256_hex;
use lumi_protocol::FailureCategory;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

/// A pinned upstream binary (§7.10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedBinary {
    /// Exact upstream version, e.g. `0.3.10`.
    pub version: String,
    /// Platform triple the binary targets.
    pub target: String,
    /// SHA-256 of the exact artifact.
    pub sha256: String,
    /// Path the pinned artifact is installed at.
    pub path: String,
    /// Upstream license identity (recorded, reviewed per AGENTS.md).
    pub license: String,
    /// Who owns updates for this binary (team/role).
    pub update_owner: String,
}

impl PinnedBinary {
    /// Verifies the installed artifact's checksum against the pin.
    ///
    /// # Errors
    /// [`NativeError`] when the file is missing or the digest differs —
    /// the runtime fails closed rather than executing an unpinned
    /// binary.
    pub fn verify(&self) -> Result<(), NativeError> {
        let bytes = std::fs::read(Path::new(&self.path)).map_err(|e| {
            NativeError::new(
                lumi_protocol::FailureCategory::UpstreamDriver,
                format!("pinned upstream missing at {}: {e}", self.path),
            )
        })?;
        let actual = sha256_hex(&bytes);
        if actual != self.sha256 {
            return Err(NativeError::new(
                lumi_protocol::FailureCategory::SecurityViolation,
                format!(
                    "upstream checksum mismatch at {}: expected {}, found {actual}",
                    self.path, self.sha256
                ),
            ));
        }
        Ok(())
    }
}

/// The Cua upstream configuration: pinned per-platform binaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CuaUpstreamConfig {
    pub macos: Option<PinnedBinary>,
    pub windows: Option<PinnedBinary>,
}

impl CuaUpstreamConfig {
    /// Verifies every configured pin (fail closed on any mismatch).
    ///
    /// # Errors
    /// First verification failure.
    pub fn verify_all(&self) -> Result<(), NativeError> {
        for pin in [&self.macos, &self.windows].into_iter().flatten() {
            pin.verify()?;
        }
        Ok(())
    }
}

/// The Cua-backed DesktopDriver adapter.
///
/// v1 ships the contract, pinning, and session/generation discipline; the
/// wire protocol to the Cua Driver process follows the secrets-broker
/// slice (the adapter resolves the driver endpoint from instance config,
/// not from workflow code).
#[derive(Clone)]
pub struct CuaDriverAdapter {
    pub config: CuaUpstreamConfig,
    pub driver_version: &'static str,
    /// The adapter's current runtime generation; handles from earlier
    /// generations fail closed (§7.18).
    pub current_generation: RuntimeGeneration,
    transport: Option<Arc<dyn CuaTransport>>,
}

impl CuaDriverAdapter {
    #[must_use]
    pub const fn new(config: CuaUpstreamConfig, current_generation: RuntimeGeneration) -> Self {
        Self {
            config,
            driver_version: "cua-adapter/0.1.0",
            current_generation,
            transport: None,
        }
    }

    /// Connects to the exact pinned platform binary over one persistent MCP
    /// stdio lease. This never downloads, installs, grants OS permissions, or
    /// widens the driver's launch-time authorization mode.
    pub fn connect(
        config: CuaUpstreamConfig,
        current_generation: RuntimeGeneration,
    ) -> Result<Self, NativeError> {
        let pin = platform_pin(&config)?;
        if pin.version != "0.28.2" {
            return Err(NativeError::new(
                FailureCategory::VersionIncompatible,
                format!(
                    "unsupported Cua Driver version {}; expected 0.28.2",
                    pin.version
                ),
            ));
        }
        pin.verify()?;
        let transport: Arc<dyn CuaTransport> = Arc::new(ProcessMcpTransport::spawn(&pin.path)?);
        verify_discovery(transport.as_ref(), &pin.version)?;
        Ok(Self {
            config,
            driver_version: "cua-driver/0.28.2",
            current_generation,
            transport: Some(transport),
        })
    }

    /// Connect to the reviewed macOS 0.28.2 installation locations only.
    /// The digest is anchored in source from the published release asset; a
    /// user-controlled path or checksum is never accepted by this helper.
    #[cfg(target_os = "macos")]
    pub fn connect_installed(
        current_generation: RuntimeGeneration,
    ) -> Result<Option<Self>, NativeError> {
        const SHA256: &str = "af30d29cf33bd3bbda1330be7225b18881ea4c5af6df374e08627914b5ac334d";
        let mut candidates =
            vec![Path::new("/Applications/CuaDriver.app/Contents/MacOS/cua-driver").to_path_buf()];
        if let Some(home) = std::env::var_os("HOME") {
            candidates.push(Path::new(&home).join(".local/bin/cua-driver"));
        }
        let Some(path) = candidates.into_iter().find(|path| path.is_file()) else {
            return Ok(None);
        };
        let config = CuaUpstreamConfig {
            macos: Some(PinnedBinary {
                version: "0.28.2".into(),
                target: "aarch64-apple-darwin".into(),
                sha256: SHA256.into(),
                path: path.to_string_lossy().into_owned(),
                license: "MIT".into(),
                update_owner: "runtime-team".into(),
            }),
            windows: None,
        };
        Self::connect(config, current_generation).map(Some)
    }

    fn transport(&self) -> Result<&dyn CuaTransport, NativeError> {
        self.transport.as_deref().ok_or_else(|| {
            NativeError::new(
                FailureCategory::VersionIncompatible,
                "Cua wire client is not connected",
            )
        })
    }

    fn call(&self, name: &str, arguments: Value) -> Result<Value, NativeError> {
        let result = self
            .transport()?
            .request("tools/call", json!({"name": name, "arguments": arguments}))?;
        tool_result(result)
    }

    fn start_session(&self, handle: &SessionHandle) -> Result<Value, NativeError> {
        self.call("start_session", json!({"session": handle.session_id}))
    }

    fn resolve_window(&self, target: &SemanticTarget) -> Result<ResolvedWindow, NativeError> {
        let apps = self.call("list_apps", json!({}))?;
        let pid = apps
            .get("apps")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|app| app.get("running").and_then(Value::as_bool) == Some(true))
            .find(|app| application_matches(app, &target.application))
            .and_then(|app| app.get("pid").and_then(Value::as_u64))
            .and_then(|pid| u32::try_from(pid).ok())
            .ok_or_else(|| {
                NativeError::new(
                    FailureCategory::NativeSession,
                    format!("target application {:?} is not running", target.application),
                )
            })?;
        let windows = self.call("list_windows", json!({"pid": pid, "on_screen_only": false}))?;
        let mut candidates: Vec<_> = windows
            .get("windows")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|window| window.get("pid").and_then(Value::as_u64) == Some(u64::from(pid)))
            .filter(|window| {
                target.window.as_ref().is_none_or(|expected| {
                    window
                        .get("title")
                        .and_then(Value::as_str)
                        .is_some_and(|actual| actual == expected)
                })
            })
            .collect();
        if candidates.len() != 1 {
            return Err(NativeError::new(
                FailureCategory::NativeSession,
                format!(
                    "target window resolution was not unique ({} matches)",
                    candidates.len()
                ),
            ));
        }
        let window = candidates.pop().expect("one candidate");
        let window_id = window
            .get("window_id")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                NativeError::new(
                    FailureCategory::VersionIncompatible,
                    "Cua window record omitted window_id",
                )
            })?;
        Ok(ResolvedWindow { pid, window_id })
    }

    fn snapshot_element(
        &self,
        handle: &SessionHandle,
        window: ResolvedWindow,
        target: &SemanticTarget,
    ) -> Result<ResolvedElement, NativeError> {
        let state = self.call(
            "get_window_state",
            json!({
                "session": handle.session_id,
                "pid": window.pid,
                "window_id": window.window_id,
                "include_screenshot": false,
                "max_elements": 2000,
                "max_depth": 25
            }),
        )?;
        let mut matches: Vec<_> = state
            .get("elements")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|element| element_matches(element, target))
            .collect();
        if matches.len() != 1 {
            return Err(NativeError::new(
                FailureCategory::NativeElement,
                format!(
                    "semantic target resolution was not unique ({} matches)",
                    matches.len()
                ),
            ));
        }
        let element = matches.pop().expect("one element");
        let token = element
            .get("element_token")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                NativeError::new(
                    FailureCategory::VersionIncompatible,
                    "Cua element omitted snapshot-bound token",
                )
            })?
            .to_owned();
        Ok(ResolvedElement {
            window,
            token,
            value: element
                .get("value")
                .and_then(Value::as_str)
                .map(str::to_owned),
        })
    }

    fn result(
        outcome: NativeOutcome,
        observation: Option<String>,
        mode: DeliveryMode,
    ) -> NativeResult {
        NativeResult {
            outcome,
            failure: None,
            refusal: None,
            oracle_observation: observation,
            collateral: CollateralReport {
                focus_preserved: Some(mode.preserves_focus_and_cursor()),
                cursor_preserved: Some(mode.preserves_focus_and_cursor()),
                no_input_leak: Some(!mode.is_global_input()),
                clipboard_unchanged: Some(true),
            },
        }
    }
}

impl std::fmt::Debug for CuaDriverAdapter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CuaDriverAdapter")
            .field("config", &self.config)
            .field("driver_version", &self.driver_version)
            .field("current_generation", &self.current_generation)
            .field("connected", &self.transport.is_some())
            .finish()
    }
}

impl DesktopDriver for CuaDriverAdapter {
    fn name(&self) -> &'static str {
        "cua-driver"
    }

    fn version(&self) -> &'static str {
        self.driver_version
    }

    fn session_state(&self, handle: &SessionHandle) -> Result<SessionState, NativeError> {
        if !handle.is_current(self.current_generation) {
            return Err(NativeError::new(
                lumi_protocol::FailureCategory::UpstreamDriver,
                "stale session generation",
            ));
        }
        let session = self.start_session(handle)?;
        Ok(
            if session.get("desktop_unlocked").and_then(Value::as_bool) == Some(true) {
                SessionState::Unlocked
            } else {
                SessionState::Locked
            },
        )
    }

    fn permissions(
        &self,
        handle: &SessionHandle,
    ) -> Result<Vec<PermissionRequirement>, NativeError> {
        if !handle.is_current(self.current_generation) {
            return Err(NativeError::new(
                FailureCategory::UpstreamDriver,
                "stale session generation",
            ));
        }
        let status = self.call("check_permissions", json!({"prompt": false}))?;
        Ok(vec![PermissionRequirement {
            capability: if cfg!(target_os = "windows") {
                "windows.uia".to_owned()
            } else {
                "macos.accessibility".to_owned()
            },
            state: if status.get("accessibility").and_then(Value::as_bool) == Some(true) {
                PermissionState::Granted
            } else {
                PermissionState::NotGranted
            },
        }])
    }

    fn execute(
        &self,
        handle: &SessionHandle,
        operation: NativeOperation,
        oracle: EffectOracle<'_>,
    ) -> Result<NativeResult, NativeError> {
        if !handle.is_current(self.current_generation) {
            return Err(NativeError::new(
                FailureCategory::UpstreamDriver,
                "stale session generation",
            ));
        }
        self.start_session(handle)?;
        let (target, mode) = match &operation {
            NativeOperation::FocusWindow { target }
            | NativeOperation::ReadValue { target }
            | NativeOperation::SetValue { target, .. }
            | NativeOperation::PressAction { target, .. } => {
                (target, DeliveryMode::SemanticBackground)
            }
            NativeOperation::Click { target, mode } => (target, *mode),
        };
        let window = self.resolve_window(target)?;
        match operation {
            NativeOperation::FocusWindow { .. } => {
                self.call(
                    "bring_to_front",
                    json!({"pid": window.pid, "window_id": window.window_id}),
                )?;
                Ok(Self::result(
                    NativeOutcome::Delivered,
                    Some("exact window verified frontmost by Cua".to_owned()),
                    DeliveryMode::SemanticForeground,
                ))
            }
            NativeOperation::ReadValue { target } => {
                let element = self.snapshot_element(handle, window, &target)?;
                Ok(Self::result(
                    NativeOutcome::Delivered,
                    element.value,
                    DeliveryMode::SemanticBackground,
                ))
            }
            NativeOperation::SetValue { target, value } => {
                let element = self.snapshot_element(handle, window, &target)?;
                self.call(
                    "set_value",
                    json!({
                        "session": handle.session_id,
                        "pid": element.window.pid,
                        "element_token": element.token,
                        "value": value
                    }),
                )?;
                let observed = self.snapshot_element(handle, window, &target)?.value;
                let outcome = if observed.as_deref() == Some(value.as_str()) {
                    NativeOutcome::Delivered
                } else {
                    NativeOutcome::NoEffect
                };
                Ok(Self::result(outcome, observed, mode))
            }
            NativeOperation::Click { target, mode } => {
                if mode.is_global_input() {
                    return Ok(NativeResult {
                        outcome: NativeOutcome::Refused,
                        failure: None,
                        refusal: Some(RefusalReason::GlobalInputNotAuthorized),
                        oracle_observation: None,
                        collateral: CollateralReport::default(),
                    });
                }
                let element = self.snapshot_element(handle, window, &target)?;
                self.call(
                    "click",
                    json!({
                        "session": handle.session_id,
                        "pid": element.window.pid,
                        "window_id": element.window.window_id,
                        "element_token": element.token,
                        "delivery_mode": if mode == DeliveryMode::SemanticForeground {
                            "foreground"
                        } else {
                            "background"
                        }
                    }),
                )?;
                let observation = oracle();
                Ok(Self::result(
                    if observation.is_some() {
                        NativeOutcome::Delivered
                    } else {
                        NativeOutcome::NoEffect
                    },
                    observation,
                    mode,
                ))
            }
            NativeOperation::PressAction { target, action } => {
                let element = self.snapshot_element(handle, window, &target)?;
                let key = match action.as_str() {
                    "confirm" | "return" => "return",
                    "cancel" | "escape" => "escape",
                    "space" => "space",
                    _ => {
                        return Ok(NativeResult {
                            outcome: NativeOutcome::Refused,
                            failure: None,
                            refusal: Some(RefusalReason::UnsupportedRoute),
                            oracle_observation: Some(format!(
                                "unsupported semantic key action {action:?}"
                            )),
                            collateral: CollateralReport::default(),
                        })
                    }
                };
                self.call(
                    "press_key",
                    json!({
                        "session": handle.session_id,
                        "pid": element.window.pid,
                        "window_id": element.window.window_id,
                        "element_token": element.token,
                        "key": key,
                        "delivery_mode": "background"
                    }),
                )?;
                let observation = oracle();
                Ok(Self::result(
                    if observation.is_some() {
                        NativeOutcome::Delivered
                    } else {
                        NativeOutcome::NoEffect
                    },
                    observation,
                    mode,
                ))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ResolvedWindow {
    pid: u32,
    window_id: u64,
}

#[derive(Debug, Clone)]
struct ResolvedElement {
    window: ResolvedWindow,
    token: String,
    value: Option<String>,
}

fn platform_pin(config: &CuaUpstreamConfig) -> Result<&PinnedBinary, NativeError> {
    #[cfg(target_os = "macos")]
    let pin = config.macos.as_ref();
    #[cfg(target_os = "windows")]
    let pin = config.windows.as_ref();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let pin: Option<&PinnedBinary> = None;
    pin.ok_or_else(|| {
        NativeError::new(
            FailureCategory::VersionIncompatible,
            "no reviewed Cua binary is configured for this platform",
        )
    })
}

fn verify_discovery(transport: &dyn CuaTransport, expected: &str) -> Result<(), NativeError> {
    let result = transport.request("server/discover", json!({}))?;
    let actual = result
        .pointer("/_meta/io.modelcontextprotocol~1serverInfo/version")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            NativeError::new(
                FailureCategory::VersionIncompatible,
                "Cua discovery omitted server version",
            )
        })?;
    if actual != expected {
        return Err(NativeError::new(
            FailureCategory::VersionIncompatible,
            format!("Cua discovery version {actual} differs from pin {expected}"),
        ));
    }
    Ok(())
}

fn application_matches(app: &Value, expected: &str) -> bool {
    ["bundle_id", "name", "process_name", "executable"]
        .into_iter()
        .filter_map(|field| app.get(field).and_then(Value::as_str))
        .any(|actual| actual.eq_ignore_ascii_case(expected))
}

fn normalized_role(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn element_matches(element: &Value, target: &SemanticTarget) -> bool {
    if let Some(expected) = &target.name {
        let matches_name = ["label", "name", "title"]
            .into_iter()
            .filter_map(|field| element.get(field).and_then(Value::as_str))
            .any(|actual| actual.eq_ignore_ascii_case(expected));
        if !matches_name {
            return false;
        }
    }
    if let Some(expected) = &target.role {
        let Some(actual) = element.get("role").and_then(Value::as_str) else {
            return false;
        };
        let expected = normalized_role(expected);
        let actual = normalized_role(actual);
        if actual != expected && !actual.ends_with(&expected) && !expected.ends_with(&actual) {
            return false;
        }
    }
    if let Some((property, expected)) = &target.stable_property {
        if element.get(property).and_then(Value::as_str) != Some(expected.as_str()) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    #[derive(Default)]
    struct ScriptedTransport {
        responses: Mutex<VecDeque<Value>>,
        requests: Mutex<Vec<(String, Value)>>,
    }

    impl ScriptedTransport {
        fn new(responses: impl IntoIterator<Item = Value>) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().collect()),
                requests: Mutex::new(Vec::new()),
            }
        }
    }

    impl CuaTransport for ScriptedTransport {
        fn request(&self, method: &str, params: Value) -> Result<Value, NativeError> {
            self.requests
                .lock()
                .unwrap()
                .push((method.to_owned(), params));
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| NativeError::new(FailureCategory::UpstreamDriver, "no fixture"))
        }
    }

    fn tool(value: Value) -> Value {
        json!({"structuredContent": value})
    }

    fn connected(responses: impl IntoIterator<Item = Value>) -> CuaDriverAdapter {
        CuaDriverAdapter {
            config: CuaUpstreamConfig {
                macos: None,
                windows: None,
            },
            driver_version: "cua-driver/0.28.2",
            current_generation: RuntimeGeneration(7),
            transport: Some(Arc::new(ScriptedTransport::new(responses))),
        }
    }

    fn target() -> SemanticTarget {
        SemanticTarget {
            application: "com.example.Editor".to_owned(),
            window: Some("Quarterly Report".to_owned()),
            role: Some("text_field".to_owned()),
            name: Some("Body".to_owned()),
            stable_property: None,
        }
    }

    fn handle() -> SessionHandle {
        SessionHandle {
            session_id: "run-123".to_owned(),
            generation: RuntimeGeneration(7),
        }
    }

    #[test]
    fn discovery_must_match_the_pinned_version() {
        let good = ScriptedTransport::new([json!({
            "_meta": {"io.modelcontextprotocol/serverInfo": {"version": "0.28.2"}}
        })]);
        assert!(verify_discovery(&good, "0.28.2").is_ok());
        let bad = ScriptedTransport::new([json!({
            "_meta": {"io.modelcontextprotocol/serverInfo": {"version": "0.29.0"}}
        })]);
        assert_eq!(
            verify_discovery(&bad, "0.28.2").unwrap_err().category,
            FailureCategory::VersionIncompatible
        );
    }

    #[test]
    fn set_value_uses_fresh_snapshot_token_and_readback() {
        let adapter = connected([
            tool(json!({"desktop_unlocked": true})),
            tool(json!({"apps":[{"bundle_id":"com.example.Editor","running":true,"pid":42}]})),
            tool(json!({"windows":[{"pid":42,"window_id":9,"title":"Quarterly Report"}]})),
            tool(
                json!({"elements":[{"element_token":"token-before","role":"AXTextField","label":"Body","value":"old"}]}),
            ),
            tool(json!({"effect":"attempted"})),
            tool(
                json!({"elements":[{"element_token":"token-after","role":"AXTextField","label":"Body","value":"new"}]}),
            ),
        ]);
        let result = adapter
            .execute(
                &handle(),
                NativeOperation::SetValue {
                    target: target(),
                    value: "new".to_owned(),
                },
                &|| None,
            )
            .unwrap();
        assert_eq!(result.outcome, NativeOutcome::Delivered);
        assert_eq!(result.oracle_observation.as_deref(), Some("new"));
    }

    #[test]
    fn stale_generation_refuses_before_contacting_upstream() {
        let adapter = connected([]);
        let stale = SessionHandle {
            session_id: "run-old".to_owned(),
            generation: RuntimeGeneration(6),
        };
        let error = adapter
            .execute(
                &stale,
                NativeOperation::ReadValue { target: target() },
                &|| None,
            )
            .unwrap_err();
        assert_eq!(error.category, FailureCategory::UpstreamDriver);
    }

    #[test]
    fn ambiguous_window_resolution_fails_closed() {
        let adapter = connected([
            tool(json!({"desktop_unlocked": true})),
            tool(json!({"apps":[{"bundle_id":"com.example.Editor","running":true,"pid":42}]})),
            tool(json!({"windows":[
                {"pid":42,"window_id":9,"title":"Quarterly Report"},
                {"pid":42,"window_id":10,"title":"Quarterly Report"}
            ]})),
        ]);
        let error = adapter
            .execute(
                &handle(),
                NativeOperation::ReadValue { target: target() },
                &|| None,
            )
            .unwrap_err();
        assert_eq!(error.category, FailureCategory::NativeSession);
    }

    #[test]
    fn checksum_mismatch_fails_closed() {
        let dir = std::env::temp_dir().join(format!("lumi-cua-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cua.bin");
        std::fs::write(&path, b"cua-binary-contents").unwrap();
        let pin = PinnedBinary {
            version: "0.3.10".to_owned(),
            target: "aarch64-apple-darwin".to_owned(),
            sha256: lumi_protocol::canonical::sha256_hex(b"cua-binary-contents"),
            path: path.to_string_lossy().to_string(),
            license: "Apache-2.0".to_owned(),
            update_owner: "runtime-team".to_owned(),
        };
        assert!(pin.verify().is_ok());

        // One byte changed: refuse to run the binary at all.
        std::fs::write(&path, b"cua-binary-contents!").unwrap();
        let err = pin.verify().unwrap_err();
        assert_eq!(
            err.category,
            lumi_protocol::FailureCategory::SecurityViolation
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_binary_fails_closed() {
        let pin = PinnedBinary {
            version: "0.3.10".to_owned(),
            target: "x86_64-pc-windows-msvc".to_owned(),
            sha256: "abc".to_owned(),
            path: "/nonexistent/cua.bin".to_owned(),
            license: "MIT".to_owned(),
            update_owner: "runtime-team".to_owned(),
        };
        assert!(pin.verify().is_err());
    }
}
