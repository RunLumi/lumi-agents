//! Cua upstream configuration and pinning (spec 07 §7.10).
//!
//! The bundled upstream binary MUST be exact-version, checksum-verified,
//! license-recorded, and update-owner-recorded. The runtime never
//! downloads "latest". The adapter speaks the Lumi DesktopDriver
//! interface; workflow definitions never see Cua tool names (§7.2).

use crate::driver::{DesktopDriver, EffectOracle, NativeError, NativeOperation, NativeResult};
use crate::session::{SessionHandle, SessionState};
use lumi_protocol::canonical::sha256_hex;
use serde::{Deserialize, Serialize};
use std::path::Path;

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
#[derive(Debug, Clone)]
pub struct CuaDriverAdapter {
    pub config: CuaUpstreamConfig,
    pub driver_version: &'static str,
    /// The adapter's current runtime generation; handles from earlier
    /// generations fail closed (§7.18).
    pub current_generation: crate::session::RuntimeGeneration,
}

impl CuaDriverAdapter {
    #[must_use]
    pub const fn new(
        config: CuaUpstreamConfig,
        current_generation: crate::session::RuntimeGeneration,
    ) -> Self {
        Self {
            config,
            driver_version: "cua-adapter/0.1.0",
            current_generation,
        }
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
        // v1 adapter runs only in interactive sessions; anything else is a
        // hard failure before any operation is attempted (§7.7).
        if !handle.is_current(self.current_generation) {
            return Err(NativeError::new(
                lumi_protocol::FailureCategory::UpstreamDriver,
                "stale session generation",
            ));
        }
        Ok(SessionState::Unlocked)
    }

    fn permissions(
        &self,
        _handle: &SessionHandle,
    ) -> Result<Vec<crate::session::PermissionRequirement>, NativeError> {
        // Detection is delegated to the platform helper process; the
        // adapter never grants or escalates (§7.6).
        Ok(vec![
            crate::session::PermissionRequirement {
                capability: "accessibility".to_owned(),
                state: crate::session::PermissionState::NotGranted,
            },
            crate::session::PermissionRequirement {
                capability: "screen_recording".to_owned(),
                state: crate::session::PermissionState::NotGranted,
            },
        ])
    }

    fn execute(
        &self,
        _handle: &SessionHandle,
        _operation: NativeOperation,
        _oracle: EffectOracle<'_>,
    ) -> Result<NativeResult, NativeError> {
        // The executable Cua wire client lands with the secrets-broker
        // slice; until then the adapter refuses (stable, pre-mutation)
        // instead of pretending (§7.15 REFUSED semantics).
        Err(NativeError::new(
            lumi_protocol::FailureCategory::VersionIncompatible,
            "cua wire client not enabled in this build",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
