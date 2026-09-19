//! Versioning, compatibility & migrations (spec 20).
//!
//! Runtime, packs, policies, executors, and clients evolve independently
//! without unsafe silent breakage. Three mechanisms:
//!
//! 1. [`SemVer`] + [`CompatibilityRange`] — semantic versioning with
//!    range checks (§20.3, §20.5).
//! 2. [`CapabilitySet`] — typed capability advertisement, separate from
//!    product version (§20.15). Clients MUST NOT infer feature support
//!    from version number alone.
//! 3. [`RuntimeGeneration`] — process-identity generation that
//!    invalidates stale handles (§20.18). Already used by `lumi-native`.
//!
//! Schema migrations (§20.7) are versioned, deterministic, tested, and
//! backed up before destructive transformation.

pub mod migration;

pub use migration::{Migration, MigrationChain, MigrationError};

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// Semantic version (§20.3): `major.minor.patch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SemVer {
    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parses `major.minor.patch` (no pre-release tags in v1).
    ///
    /// # Errors
    /// Returns a human-readable message on malformed input.
    pub fn parse(input: &str) -> Result<Self, String> {
        let parts: Vec<&str> = input.trim().split('.').collect();
        if parts.len() != 3 {
            return Err(format!("expected major.minor.patch, got {input:?}"));
        }
        let mut nums = [0u32; 3];
        for (i, part) in parts.iter().enumerate() {
            nums[i] = part
                .parse()
                .map_err(|_| format!("invalid version component {part:?} in {input:?}"))?;
        }
        Ok(Self::new(nums[0], nums[1], nums[2]))
    }

    /// True when this version is in the same major (compatible within
    /// major per §20.3).
    #[must_use]
    pub const fn same_major(&self, other: &Self) -> bool {
        self.major == other.major
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A compatibility range: `>= min, < next_breaking` (§20.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityRange {
    /// Minimum supported version (inclusive).
    pub min: SemVer,
    /// First unsupported major (exclusive). `None` = all future majors
    /// compatible (rare; explicit opt-in).
    pub next_major: Option<u32>,
}

impl CompatibilityRange {
    #[must_use]
    pub const fn new(min: SemVer, next_major: Option<u32>) -> Self {
        Self { min, next_major }
    }

    #[must_use]
    pub fn contains(&self, version: &SemVer) -> bool {
        if version < &self.min {
            return false;
        }
        match self.next_major {
            Some(max) => version.major < max,
            None => true,
        }
    }
}

/// Typed capability advertisement (§20.15). Clients MUST use this for
/// feature detection, never version-number inference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CapabilitySet {
    caps: BTreeSet<String>,
}

impl CapabilitySet {
    #[must_use]
    pub fn new(caps: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            caps: caps.into_iter().map(Into::into).collect(),
        }
    }

    #[must_use]
    pub fn has(&self, capability: &str) -> bool {
        self.caps.contains(capability)
    }

    /// True when EVERY required capability is present. Missing = not
    /// supported; there is no graceful degradation to a weaker trust
    /// boundary (§20.16).
    #[must_use]
    pub fn supports_all(&self, required: &[&str]) -> bool {
        required.iter().all(|c| self.caps.contains(*c))
    }

    /// The difference: capabilities in `before` but not in `after`.
    /// Used to detect downgrade (§20.16: stale cached capability after
    /// downgrade must disable/reject the feature).
    #[must_use]
    pub fn lost_between(before: &Self, after: &Self) -> Vec<String> {
        before.caps.difference(&after.caps).cloned().collect()
    }

    #[must_use]
    pub fn as_set(&self) -> &BTreeSet<String> {
        &self.caps
    }
}

/// Well-known capability names (§20.15 examples).
pub mod capabilities {
    pub const TASK_RESUME: &str = "task.resume.v1";
    pub const TASK_FORK: &str = "task.fork.v1";
    pub const APPROVAL_DIGEST: &str = "approval.digest.v1";
    pub const BROWSER_SEMANTIC: &str = "browser.semantic.v1";
    pub const NATIVE_BACKGROUND_INPUT: &str = "native.background_input.v1";
    pub const WORKFLOW_PACK: &str = "workflow_pack.v1";
    pub const ARTIFACT_DOCX: &str = "artifact.docx.v1";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_parse_and_order() {
        let a = SemVer::parse("1.2.3").unwrap();
        let b = SemVer::parse("1.10.0").unwrap();
        assert!(a < b);
        assert!(SemVer::parse("2.0.0").unwrap() > b);
        assert!(SemVer::parse("1.2").is_err());
        assert!(SemVer::parse("a.b.c").is_err());
    }

    #[test]
    fn compatibility_range() {
        let range = CompatibilityRange::new(SemVer::new(0, 1, 0), Some(1));
        assert!(range.contains(&SemVer::new(0, 1, 0)));
        assert!(range.contains(&SemVer::new(0, 99, 0)));
        assert!(!range.contains(&SemVer::new(1, 0, 0)));
        assert!(!range.contains(&SemVer::new(0, 0, 9)));
    }

    #[test]
    fn capability_downgrade_detected() {
        let before = CapabilitySet::new([
            capabilities::TASK_RESUME,
            capabilities::BROWSER_SEMANTIC,
            capabilities::TASK_FORK,
        ]);
        let after = CapabilitySet::new([capabilities::TASK_RESUME, capabilities::BROWSER_SEMANTIC]);
        let lost = CapabilitySet::lost_between(&before, &after);
        assert_eq!(lost, vec![capabilities::TASK_FORK.to_owned()]);
        // §20.16: a feature that lost its capability must be disabled.
        assert!(!after.has(capabilities::TASK_FORK));
    }
}
