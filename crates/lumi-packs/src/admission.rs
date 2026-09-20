//! Project skills/plugins admission (Spec 28): a project sees a pack
//! only when an exact version snapshot is PINNED for that project and
//! enabled. No floating versions, no ambient pack availability —
//! admission is explicit, durable, and checked at run start.

use crate::manifest::{PackManifest, PackStatus};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One project-scoped pin: the exact pack version a project admitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectPin {
    pub project_id: String,
    pub pack_id: String,
    /// Exact semantic version snapshot — never a range (Spec 28 §28.4:
    /// pinned snapshots, no floating versions).
    pub pinned_version: String,
    pub enabled: bool,
    pub pinned_at: String,
}

/// Registry of project pins, persisted as JSON (atomic write).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionRegistry {
    pub pins: Vec<ProjectPin>,
}

impl AdmissionRegistry {
    /// Loads the registry. A missing file is an empty registry.
    ///
    /// # Errors
    /// Corrupt JSON fails closed rather than silently dropping pins.
    pub fn load(path: &Path) -> Result<AdmissionRegistry, String> {
        if !path.exists() {
            return Ok(AdmissionRegistry::default());
        }
        let text = std::fs::read_to_string(path).map_err(|e| format!("read pins: {e}"))?;
        serde_json::from_str(&text).map_err(|e| format!("corrupt pins file: {e}"))
    }

    /// Persists the registry atomically (temp + rename).
    ///
    /// # Errors
    /// Filesystem failures.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&tmp, json).map_err(|e| format!("write pins: {e}"))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("persist pins: {e}"))
    }

    /// Pins (or re-pins) a pack version for a project. Re-pinning an
    /// existing (project, pack) pair upgrades the snapshot in place.
    pub fn pin(
        &mut self,
        project_id: &str,
        pack_id: &str,
        pinned_version: &str,
        pinned_at: &str,
    ) -> Result<(), String> {
        if project_id.trim().is_empty() || pack_id.trim().is_empty() {
            return Err("project_id and pack_id are required".to_owned());
        }
        let v = pinned_version.trim().to_owned();
        if v.split('.').count() != 3 || v.split('.').any(|p| p.parse::<u64>().is_err()) {
            return Err(format!(
                "pinned version {v:?} is not semantic (major.minor.patch)"
            ));
        }
        if let Some(pin) = self
            .pins
            .iter_mut()
            .find(|p| p.project_id == project_id && p.pack_id == pack_id)
        {
            pin.pinned_version = v;
            pin.enabled = true;
            pin.pinned_at = pinned_at.to_owned();
        } else {
            self.pins.push(ProjectPin {
                project_id: project_id.to_owned(),
                pack_id: pack_id.to_owned(),
                pinned_version: v,
                enabled: true,
                pinned_at: pinned_at.to_owned(),
            });
        }
        Ok(())
    }

    /// Removes the pin (project stops seeing the pack). True if removed.
    pub fn unpin(&mut self, project_id: &str, pack_id: &str) -> bool {
        let before = self.pins.len();
        self.pins
            .retain(|p| !(p.project_id == project_id && p.pack_id == pack_id));
        before != self.pins.len()
    }

    #[must_use]
    pub fn pin_for(&self, project_id: &str, pack_id: &str) -> Option<&ProjectPin> {
        self.pins
            .iter()
            .find(|p| p.project_id == project_id && p.pack_id == pack_id && p.enabled)
    }
}

/// Admission result for one pack on one project (Spec 28: admission is
/// enforced at run start, not just displayed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackAdmission {
    pub project_id: String,
    pub pack_id: String,
    pub pinned_version: String,
}

/// Checks that `manifest` may run for `project_id`: pinned (enabled,
/// exact version match), structurally valid, Certified (production
/// gate), runtime version within the declared range, and host OS
/// supported. Returns `Err` with every violation when refused.
///
/// # Errors
/// Refusals are reported as a violation list; empty list = admitted.
pub fn admit_pack_for_project(
    manifest: &PackManifest,
    registry: &AdmissionRegistry,
    project_id: &str,
    runtime_version: &str,
    host_os: &str,
) -> Result<PackAdmission, Vec<String>> {
    let mut violations: Vec<String> = Vec::new();

    if let Err(vs) = manifest.validate() {
        violations.extend(vs);
    }

    let pin = registry.pin_for(project_id, &manifest.pack_id);
    let Some(pin) = pin else {
        violations.push(format!(
            "pack {:?} is not pinned for this project; pin an exact version first",
            manifest.pack_id
        ));
        return Err(violations);
    };
    if !pin.enabled {
        violations.push(format!(
            "pack {:?} is pinned but disabled for this project",
            manifest.pack_id
        ));
    }
    if pin.pinned_version != manifest.version {
        violations.push(format!(
            "pinned version {} does not match manifest version {} (pinned snapshots only)",
            pin.pinned_version, manifest.version
        ));
    }

    if manifest.status == PackStatus::Deprecated {
        violations.push(format!("pack {:?} is deprecated", manifest.pack_id));
    }

    // Runtime compatibility: manifest.runtime_version_range is a floor
    // declaration (e.g. ">=0.1.0"); enforce >= against the caller's
    // current runtime version.
    let range = manifest.runtime_version_range.trim();
    if let Some(min) = range.strip_prefix(">=") {
        let cur: Vec<u64> = runtime_version
            .split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect();
        let need: Vec<u64> = min.split('.').map(|p| p.parse().unwrap_or(0)).collect();
        let below = (
            cur.first().copied().unwrap_or(0),
            cur.get(1).copied().unwrap_or(0),
            cur.get(2).copied().unwrap_or(0),
        ) < (
            *need.first().unwrap_or(&0),
            *need.get(1).unwrap_or(&0),
            *need.get(2).unwrap_or(&0),
        );
        if below {
            violations.push(format!(
                "runtime {runtime_version} below required floor {min}"
            ));
        }
    }
    if !manifest.supported_os.is_empty() && !manifest.supported_os.contains(host_os) {
        violations.push(format!("host OS {host_os:?} not in supported list"));
    }

    if violations.is_empty() {
        Ok(PackAdmission {
            project_id: project_id.to_owned(),
            pack_id: manifest.pack_id.clone(),
            pinned_version: manifest.version.clone(),
        })
    } else {
        Err(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_pin_unpin_round_trip() {
        let mut reg = AdmissionRegistry::default();
        reg.pin("p1", "crm-pack", "1.2.0", "2026-09-20").unwrap();
        assert!(reg.pin_for("p1", "crm-pack").is_some());
        // Re-pin upgrades in place, not a duplicate.
        reg.pin("p1", "crm-pack", "1.3.0", "2026-09-21").unwrap();
        assert_eq!(reg.pins.len(), 1);
        assert_eq!(
            reg.pin_for("p1", "crm-pack").unwrap().pinned_version,
            "1.3.0"
        );
        assert!(reg.unpin("p1", "crm-pack"));
        assert!(reg.pin_for("p1", "crm-pack").is_none());
    }

    #[test]
    fn pin_rejects_non_semver_and_empty_ids() {
        let mut reg = AdmissionRegistry::default();
        assert!(reg.pin("p1", "pack", "1.2", "d").is_err());
        assert!(reg.pin("p1", "pack", "abc", "d").is_err());
        assert!(reg.pin("", "pack", "1.0.0", "d").is_err());
    }

    #[test]
    fn registry_round_trips_through_json() {
        let mut reg = AdmissionRegistry::default();
        reg.pin("p1", "crm-pack", "1.2.0", "2026-09-20").unwrap();
        let path = std::env::temp_dir().join(format!("pins-{}.json", std::process::id()));
        reg.save(&path).unwrap();
        let loaded = AdmissionRegistry::load(&path).unwrap();
        assert_eq!(loaded, reg);
        let _ = std::fs::remove_file(&path);
    }
}
