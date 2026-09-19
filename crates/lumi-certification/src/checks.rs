//! Release certification checks (spec 21).
//!
//! Each check is a named gate with pass/fail and evidence. Checks cover:
//! signing config, updater/rollback config, SBOM presence, dependency
//! policy, adversarial corpus green, fixture certification green.

use serde::{Deserialize, Serialize};

/// One certification check result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckResult {
    pub check_name: String,
    pub passed: bool,
    /// Evidence or failure detail.
    pub detail: String,
}

/// Signing configuration requirements (§21: signed builds).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningConfig {
    pub macos_signing: bool,
    pub macos_notarization: bool,
    pub windows_code_signing: bool,
    /// Signing identity/certificate refs (not the credentials themselves).
    pub macos_identity_ref: Option<String>,
    pub windows_cert_ref: Option<String>,
}

/// Updater/rollback configuration requirements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdaterConfig {
    pub updater_enabled: bool,
    pub rollback_supported: bool,
    pub update_manifest_url: Option<String>,
}

/// The certification report: all check results aggregated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CertificationReport {
    pub checks: Vec<CheckResult>,
    /// All supplied configuration declarations passed this checklist.
    pub configuration_ready: bool,
    /// Configuration booleans are not artifact or live-canary evidence.
    /// This checker cannot authorize a release and always reports false.
    pub release_approved: bool,
}

/// Runs the release certification checks.
#[must_use]
pub fn run_certification(
    signing: &SigningConfig,
    updater: &UpdaterConfig,
    adversarial_corpus_green: bool,
    fixture_certification_green: bool,
    sbom_generated: bool,
    dependency_policy_green: bool,
) -> CertificationReport {
    let mut checks = Vec::new();

    // Signing checks (spec 22.8).
    checks.push(CheckResult {
        check_name: "macos_code_signing".to_owned(),
        passed: signing.macos_signing,
        detail: if signing.macos_signing {
            "macOS signing configured".to_owned()
        } else {
            "macOS signing identity required (Apple Developer)".to_owned()
        },
    });
    checks.push(CheckResult {
        check_name: "macos_notarization".to_owned(),
        passed: signing.macos_notarization,
        detail: if signing.macos_notarization {
            "macOS notarization configured".to_owned()
        } else {
            "macOS notarization required for Gatekeeper".to_owned()
        },
    });
    checks.push(CheckResult {
        check_name: "windows_code_signing".to_owned(),
        passed: signing.windows_code_signing,
        detail: if signing.windows_code_signing {
            "Windows code signing configured".to_owned()
        } else {
            "Windows code signing certificate required".to_owned()
        },
    });

    // Updater/rollback (spec 22.8).
    checks.push(CheckResult {
        check_name: "updater_enabled".to_owned(),
        passed: updater.updater_enabled,
        detail: if updater.updater_enabled {
            "updater configured".to_owned()
        } else {
            "updater must be configured for release".to_owned()
        },
    });
    checks.push(CheckResult {
        check_name: "rollback_supported".to_owned(),
        passed: updater.rollback_supported,
        detail: if updater.rollback_supported {
            "rollback path documented and tested".to_owned()
        } else {
            "rollback path required (spec 20 §20.13)".to_owned()
        },
    });

    // SBOM / provenance (spec 21).
    checks.push(CheckResult {
        check_name: "sbom_generated".to_owned(),
        passed: sbom_generated,
        detail: if sbom_generated {
            "SBOM generated".to_owned()
        } else {
            "SBOM required for supply-chain provenance".to_owned()
        },
    });

    // Dependency policy.
    checks.push(CheckResult {
        check_name: "dependency_policy".to_owned(),
        passed: dependency_policy_green,
        detail: if dependency_policy_green {
            "cargo-deny clean".to_owned()
        } else {
            "cargo-deny violations detected".to_owned()
        },
    });

    // Test corpus gates.
    checks.push(CheckResult {
        check_name: "adversarial_corpus".to_owned(),
        passed: adversarial_corpus_green,
        detail: if adversarial_corpus_green {
            "adversarial corpus green".to_owned()
        } else {
            "adversarial corpus has failures".to_owned()
        },
    });
    checks.push(CheckResult {
        check_name: "fixture_certification".to_owned(),
        passed: fixture_certification_green,
        detail: if fixture_certification_green {
            "fixture certification green".to_owned()
        } else {
            "fixture certification has failures".to_owned()
        },
    });

    let configuration_ready = checks.iter().all(|c| c.passed);
    CertificationReport {
        checks,
        configuration_ready,
        release_approved: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signing_config() -> SigningConfig {
        SigningConfig {
            macos_signing: true,
            macos_notarization: true,
            windows_code_signing: true,
            macos_identity_ref: Some("Apple Development".to_owned()),
            windows_cert_ref: Some("cert-store-ref".to_owned()),
        }
    }

    fn updater_config() -> UpdaterConfig {
        UpdaterConfig {
            updater_enabled: true,
            rollback_supported: true,
            update_manifest_url: Some("https://updates.lumi.test".to_owned()),
        }
    }

    #[test]
    fn all_declared_checks_pass_but_cannot_approve_a_release() {
        let report = run_certification(
            &signing_config(),
            &updater_config(),
            true, // adversarial green
            true, // fixture green
            true, // SBOM
            true, // dep policy
        );
        assert!(report.configuration_ready, "{report:?}");
        assert!(
            !report.release_approved,
            "declarations are not artifact proof"
        );
        assert_eq!(report.checks.len(), 9);
    }

    #[test]
    fn missing_signing_blocks_release() {
        let signing = SigningConfig {
            macos_signing: false,
            macos_notarization: false,
            windows_code_signing: false,
            macos_identity_ref: None,
            windows_cert_ref: None,
        };
        let report = run_certification(&signing, &updater_config(), true, true, true, true);
        assert!(!report.release_approved);
        // All three signing checks must fail.
        let signing_failures: Vec<&CheckResult> = report
            .checks
            .iter()
            .filter(|c| c.check_name.contains("signing") || c.check_name.contains("notarization"))
            .collect();
        assert_eq!(signing_failures.len(), 3);
        assert!(!signing_failures.iter().any(|c| c.passed));
    }

    #[test]
    fn missing_sbom_blocks_release() {
        let report = run_certification(
            &signing_config(),
            &updater_config(),
            true,
            true,
            false, // SBOM missing
            true,
        );
        assert!(!report.release_approved);
        assert!(report
            .checks
            .iter()
            .any(|c| c.check_name == "sbom_generated" && !c.passed));
    }

    #[test]
    fn certification_report_serializes() {
        let report =
            run_certification(&signing_config(), &updater_config(), true, true, true, true);
        let json = serde_json::to_string(&report).unwrap();
        let back: CertificationReport = serde_json::from_str(&json).unwrap();
        assert!(back.configuration_ready);
        assert!(!back.release_approved);
    }
}
