//! Postcondition verification (spec 11 §11.6–11.10).
//!
//! Executor SUCCESS is not success. A required postcondition that is
//! FAILED or AMBIGUOUS means the step is NOT verified-successful,
//! regardless of what any model or executor claims. Verification should
//! use an observation path independent of the executing path when possible
//! (spec 11 §11.9); model-only judgment never verifies high-risk side
//! effects (spec 11 §11.10).

use lumi_protocol::{ArtifactId, Postcondition, PostconditionCheck, ResourceRef, SensitivityLabel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Canonical verification statuses (spec 11 §11.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    Passed,
    Failed,
    Ambiguous,
    NotRequired,
}

/// Result of evaluating one postcondition.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VerificationOutcome {
    pub postcondition_id: String,
    pub status: VerificationStatus,
    /// Secret-free, human-readable detail for evidence/exception queues.
    pub detail: String,
}

/// What the verifier may consult. Every capability is optional; a required
/// check whose environment capability is missing resolves to AMBIGUOUS
/// (fail closed) — never PASSED.
pub trait VerificationEnvironment {
    /// Resolves the current state of a resource as canonical JSON
    /// (connector query, DOM query, DB read…). `None` = unresolvable.
    fn resolve_record(&self, resource: &ResourceRef) -> Option<serde_json::Value> {
        let _ = resource;
        None
    }

    /// Workspace root for file checks; `None` = files not available.
    fn workspace_root(&self) -> Option<&Path> {
        None
    }

    /// Fetches a registered artifact; `None` = unknown artifact.
    fn artifact(&self, id: &ArtifactId) -> Option<lumi_protocol::Artifact> {
        let _ = id;
        None
    }

    /// Runs a named probe (independent observation path) returning
    /// canonical JSON; `None` = probe unavailable.
    fn run_probe(&self, probe_id: &str) -> Option<serde_json::Value> {
        let _ = probe_id;
        None
    }

    /// Runs a registered custom verifier; `None` = verifier unavailable
    /// (which fails closed to AMBIGUOUS, not PASSED).
    fn run_custom(
        &self,
        verifier_id: &str,
        params: &serde_json::Value,
    ) -> Option<VerificationStatus> {
        let _ = (verifier_id, params);
        None
    }
}

/// Reference environment used when nothing is wired: every check resolves
/// to AMBIGUOUS.
#[derive(Debug, Default, Clone, Copy)]
pub struct UnavailableEnvironment;

impl VerificationEnvironment for UnavailableEnvironment {}

/// In-memory environment backing fixture/test and local-run verification.
#[derive(Debug, Default, Clone)]
pub struct FixtureEnvironment {
    pub records: HashMap<ResourceRef, serde_json::Value>,
    pub workspace_root: Option<PathBuf>,
    pub artifacts: HashMap<ArtifactId, lumi_protocol::Artifact>,
    pub probes: HashMap<String, serde_json::Value>,
    pub custom: HashMap<String, VerificationStatus>,
}

impl FixtureEnvironment {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_record(mut self, resource: ResourceRef, state: serde_json::Value) -> Self {
        self.records.insert(resource, state);
        self
    }

    pub fn with_workspace(mut self, root: PathBuf) -> Self {
        self.workspace_root = Some(root);
        self
    }

    pub fn with_probe(mut self, probe_id: impl Into<String>, value: serde_json::Value) -> Self {
        self.probes.insert(probe_id.into(), value);
        self
    }
}

impl VerificationEnvironment for FixtureEnvironment {
    fn resolve_record(&self, resource: &ResourceRef) -> Option<serde_json::Value> {
        self.records.get(resource).cloned()
    }

    fn workspace_root(&self) -> Option<&Path> {
        self.workspace_root.as_deref()
    }

    fn artifact(&self, id: &ArtifactId) -> Option<lumi_protocol::Artifact> {
        self.artifacts.get(id).cloned()
    }

    fn run_probe(&self, probe_id: &str) -> Option<serde_json::Value> {
        self.probes.get(probe_id).cloned()
    }

    fn run_custom(
        &self,
        verifier_id: &str,
        _params: &serde_json::Value,
    ) -> Option<VerificationStatus> {
        self.custom.get(verifier_id).copied()
    }
}

/// The deterministic postcondition verifier.
#[derive(Debug, Default, Clone, Copy)]
pub struct PostconditionVerifier;

impl PostconditionVerifier {
    /// Evaluates one postcondition against the environment.
    ///
    /// The result is deterministic: the same environment state yields the
    /// same outcome. Missing environment capability ⇒ AMBIGUOUS.
    #[must_use]
    pub fn verify(
        &self,
        postcondition: &Postcondition,
        env: &dyn VerificationEnvironment,
    ) -> VerificationOutcome {
        let outcome = match &postcondition.check {
            PostconditionCheck::RecordExists { resource } => match env.resolve_record(resource) {
                Some(state) if state.get("exists").and_then(|v| v.as_bool()) == Some(true) => {
                    VerificationOutcome {
                        postcondition_id: postcondition.id.0.clone(),
                        status: VerificationStatus::Passed,
                        detail: format!("record {} exists", resource.id),
                    }
                }
                Some(state) => VerificationOutcome {
                    postcondition_id: postcondition.id.0.clone(),
                    status: VerificationStatus::Failed,
                    detail: format!(
                        "record {} absent or not marked exists: {state}",
                        resource.id
                    ),
                },
                None => VerificationOutcome {
                    postcondition_id: postcondition.id.0.clone(),
                    status: VerificationStatus::Ambiguous,
                    detail: format!("resource {} unresolvable via independent path", resource.id),
                },
            },
            PostconditionCheck::RecordFieldEquals {
                resource,
                field,
                expected,
            } => match env.resolve_record(resource) {
                Some(state) => {
                    let actual = state.get(field);
                    // Canonical JSON comparison avoids type-surprise equalities.
                    let matches = actual.is_some_and(|actual| {
                        lumi_protocol::canonical::canonical_json(actual)
                            == lumi_protocol::canonical::canonical_json(expected)
                    });
                    if matches {
                        VerificationOutcome {
                            postcondition_id: postcondition.id.0.clone(),
                            status: VerificationStatus::Passed,
                            detail: format!("{}.{field} matches expected", resource.id),
                        }
                    } else {
                        VerificationOutcome {
                            postcondition_id: postcondition.id.0.clone(),
                            status: VerificationStatus::Failed,
                            detail: format!(
                                "{}.{field} is {:?}, expected {}",
                                resource.id,
                                actual,
                                lumi_protocol::canonical::canonical_json(expected)
                            ),
                        }
                    }
                }
                None => VerificationOutcome {
                    postcondition_id: postcondition.id.0.clone(),
                    status: VerificationStatus::Ambiguous,
                    detail: format!("resource {} unresolvable", resource.id),
                },
            },
            PostconditionCheck::FileChecksum { path, sha256 } => match env.workspace_root() {
                Some(root) => match file_sha256(&root.join(path)) {
                    Some(actual) if actual == *sha256 => VerificationOutcome {
                        postcondition_id: postcondition.id.0.clone(),
                        status: VerificationStatus::Passed,
                        detail: format!("{path} checksum matches"),
                    },
                    Some(actual) => VerificationOutcome {
                        postcondition_id: postcondition.id.0.clone(),
                        status: VerificationStatus::Failed,
                        detail: format!("{path} checksum {actual} != expected"),
                    },
                    None => VerificationOutcome {
                        postcondition_id: postcondition.id.0.clone(),
                        status: VerificationStatus::Failed,
                        detail: format!("{path} missing"),
                    },
                },
                None => VerificationOutcome {
                    postcondition_id: postcondition.id.0.clone(),
                    status: VerificationStatus::Ambiguous,
                    detail: "workspace unavailable for checksum".to_owned(),
                },
            },
            PostconditionCheck::FileExists { path } => match env.workspace_root() {
                Some(root) => {
                    let exists = root.join(path).is_file();
                    VerificationOutcome {
                        postcondition_id: postcondition.id.0.clone(),
                        status: if exists {
                            VerificationStatus::Passed
                        } else {
                            VerificationStatus::Failed
                        },
                        detail: format!("{path} exists: {exists}"),
                    }
                }
                None => VerificationOutcome {
                    postcondition_id: postcondition.id.0.clone(),
                    status: VerificationStatus::Ambiguous,
                    detail: "workspace unavailable".to_owned(),
                },
            },
            PostconditionCheck::ArtifactValid { artifact_id } => match env.artifact(artifact_id) {
                Some(artifact) => {
                    let valid =
                        artifact.validation_status == lumi_protocol::ValidationStatus::Valid;
                    VerificationOutcome {
                        postcondition_id: postcondition.id.0.clone(),
                        status: if valid {
                            VerificationStatus::Passed
                        } else {
                            VerificationStatus::Failed
                        },
                        detail: format!(
                            "artifact {} validation_status {:?}",
                            artifact.artifact_id, artifact.validation_status
                        ),
                    }
                }
                None => VerificationOutcome {
                    postcondition_id: postcondition.id.0.clone(),
                    status: VerificationStatus::Ambiguous,
                    detail: format!("artifact {} unknown to verifier", artifact_id),
                },
            },
            PostconditionCheck::RemoteStateMatches { probe_id, expected } => {
                match env.run_probe(probe_id) {
                    Some(actual) => {
                        let matches = lumi_protocol::canonical::canonical_json(&actual)
                            == lumi_protocol::canonical::canonical_json(expected);
                        VerificationOutcome {
                            postcondition_id: postcondition.id.0.clone(),
                            status: if matches {
                                VerificationStatus::Passed
                            } else {
                                VerificationStatus::Failed
                            },
                            detail: format!(
                                "probe {probe_id} returned {}",
                                lumi_protocol::canonical::canonical_json(&actual)
                            ),
                        }
                    }
                    None => VerificationOutcome {
                        postcondition_id: postcondition.id.0.clone(),
                        status: VerificationStatus::Ambiguous,
                        detail: format!("probe {probe_id} unavailable"),
                    },
                }
            }
            PostconditionCheck::Custom {
                verifier_id,
                params,
            } => match env.run_custom(verifier_id, params) {
                Some(status) => VerificationOutcome {
                    postcondition_id: postcondition.id.0.clone(),
                    status,
                    detail: format!("custom verifier {verifier_id} -> {status:?}"),
                },
                None => VerificationOutcome {
                    postcondition_id: postcondition.id.0.clone(),
                    status: VerificationStatus::Ambiguous,
                    detail: format!("custom verifier {verifier_id} not registered"),
                },
            },
        };
        outcome
    }

    /// Evaluates every postcondition; the aggregate status is the worst of
    /// the individual ones in the order FAILED > AMBIGUOUS > PASSED.
    #[must_use]
    pub fn verify_all(
        &self,
        postconditions: &[Postcondition],
        env: &dyn VerificationEnvironment,
    ) -> (VerificationStatus, Vec<VerificationOutcome>) {
        if postconditions.is_empty() {
            return (VerificationStatus::NotRequired, Vec::new());
        }
        let outcomes: Vec<VerificationOutcome> =
            postconditions.iter().map(|p| self.verify(p, env)).collect();
        let aggregate = if outcomes
            .iter()
            .any(|o| o.status == VerificationStatus::Failed)
        {
            VerificationStatus::Failed
        } else if outcomes
            .iter()
            .any(|o| o.status == VerificationStatus::Ambiguous)
        {
            VerificationStatus::Ambiguous
        } else {
            VerificationStatus::Passed
        };
        (aggregate, outcomes)
    }
}

fn file_sha256(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(lumi_protocol::canonical::sha256_hex(&bytes))
}

/// Sensitivity gate for model-assisted verification (spec 11 §11.10).
/// Model judgment may assist on unstructured artifacts but never stands in
/// for deterministic verification of high-risk side effects.
#[must_use]
pub fn model_verification_permitted(sensitivity: Option<SensitivityLabel>) -> bool {
    !matches!(
        sensitivity,
        Some(SensitivityLabel::Restricted) | Some(SensitivityLabel::PersonalData)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumi_protocol::{ArtifactProvenance, ArtifactType, PostconditionId, ResourceType, Target};
    use std::io::Write;

    fn check() -> Postcondition {
        Postcondition {
            id: PostconditionId::new("email-exists"),
            description: "sent message exists with expected subject".to_owned(),
            check: PostconditionCheck::RecordFieldEquals {
                resource: ResourceRef {
                    resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                    id: "msg-1".to_owned(),
                    sensitivity: None,
                },
                field: "subject".to_owned(),
                expected: serde_json::json!("Following up"),
            },
        }
    }

    #[test]
    fn executor_success_with_failed_verifier_is_failed() {
        // Spec 11.14: executor says SUCCESS; record state disagrees.
        let env = FixtureEnvironment::new().with_record(
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                id: "msg-1".to_owned(),
                sensitivity: None,
            },
            serde_json::json!({"exists": true, "subject": "Wrong subject"}),
        );
        let (status, outcomes) = PostconditionVerifier.verify_all(&[check()], &env);
        assert_eq!(status, VerificationStatus::Failed);
        assert!(outcomes[0].detail.contains("Wrong subject"));
    }

    #[test]
    fn unresolvable_state_is_ambiguous_never_passed() {
        let env = UnavailableEnvironment;
        let (status, _) = PostconditionVerifier.verify_all(&[check()], &env);
        assert_eq!(status, VerificationStatus::Ambiguous);
    }

    #[test]
    fn passing_record_passes() {
        let env = FixtureEnvironment::new().with_record(
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                id: "msg-1".to_owned(),
                sensitivity: None,
            },
            serde_json::json!({"exists": true, "subject": "Following up"}),
        );
        let (status, _) = PostconditionVerifier.verify_all(&[check()], &env);
        assert_eq!(status, VerificationStatus::Passed);
    }

    #[test]
    fn no_postconditions_is_not_required() {
        let (status, outcomes) = PostconditionVerifier.verify_all(&[], &UnavailableEnvironment);
        assert_eq!(status, VerificationStatus::NotRequired);
        assert!(outcomes.is_empty());
    }

    #[test]
    fn file_checksum_verification() {
        let dir = std::env::temp_dir().join(format!("lumi-verify-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.txt");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"verified content").unwrap();
        let expected = lumi_protocol::canonical::sha256_hex(b"verified content");
        let env = FixtureEnvironment::new().with_workspace(dir.clone());

        let good = Postcondition {
            id: PostconditionId::new("file-ok"),
            description: "output exists with expected content".to_owned(),
            check: PostconditionCheck::FileChecksum {
                path: "out.txt".to_owned(),
                sha256: expected,
            },
        };
        let (status, _) = PostconditionVerifier.verify_all(&[good], &env);
        assert_eq!(status, VerificationStatus::Passed);

        let bad = Postcondition {
            id: PostconditionId::new("file-bad"),
            description: "wrong checksum".to_owned(),
            check: PostconditionCheck::FileChecksum {
                path: "out.txt".to_owned(),
                sha256: "deadbeef".to_owned(),
            },
        };
        let (status, _) = PostconditionVerifier.verify_all(&[bad], &env);
        assert_eq!(status, VerificationStatus::Failed);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn aggregate_worst_status_wins() {
        let env = FixtureEnvironment::new()
            .with_probe("probe-a", serde_json::json!({"ok": true}))
            .with_probe("probe-b", serde_json::json!({"ok": false}));
        let postconditions = vec![
            Postcondition {
                id: PostconditionId::new("a"),
                description: "a".to_owned(),
                check: PostconditionCheck::RemoteStateMatches {
                    probe_id: "probe-a".to_owned(),
                    expected: serde_json::json!({"ok": true}),
                },
            },
            Postcondition {
                id: PostconditionId::new("b"),
                description: "b".to_owned(),
                check: PostconditionCheck::RemoteStateMatches {
                    probe_id: "probe-b".to_owned(),
                    expected: serde_json::json!({"ok": true}),
                },
            },
        ];
        let (status, _) = PostconditionVerifier.verify_all(&postconditions, &env);
        assert_eq!(status, VerificationStatus::Failed);
    }

    #[test]
    fn unknown_artifact_is_ambiguous() {
        let env = FixtureEnvironment::new();
        let postcondition = Postcondition {
            id: PostconditionId::new("art"),
            description: "artifact valid".to_owned(),
            check: PostconditionCheck::ArtifactValid {
                artifact_id: ArtifactId::parse("art-x").unwrap(),
            },
        };
        let (status, _) = PostconditionVerifier.verify_all(&[postcondition], &env);
        assert_eq!(status, VerificationStatus::Ambiguous);
    }

    #[test]
    fn unregistered_custom_verifier_fails_closed() {
        let env = FixtureEnvironment::new();
        let postcondition = Postcondition {
            id: PostconditionId::new("custom"),
            description: "custom".to_owned(),
            check: PostconditionCheck::Custom {
                verifier_id: "nope".to_owned(),
                params: serde_json::json!({}),
            },
        };
        let (status, _) = PostconditionVerifier.verify_all(&[postcondition], &env);
        assert_eq!(status, VerificationStatus::Ambiguous);
    }

    #[test]
    fn model_verification_gate() {
        assert!(model_verification_permitted(None));
        assert!(model_verification_permitted(Some(SensitivityLabel::Public)));
        assert!(!model_verification_permitted(Some(
            SensitivityLabel::Restricted
        )));
        assert!(!model_verification_permitted(Some(
            SensitivityLabel::PersonalData
        )));
    }

    #[test]
    fn artifact_valid_passes_only_when_validated() {
        let artifact_id = ArtifactId::parse("art-1").unwrap();
        let mut env = FixtureEnvironment::new();
        env.artifacts.insert(
            artifact_id.clone(),
            lumi_protocol::Artifact {
                artifact_id: artifact_id.clone(),
                task_id: lumi_protocol::TaskId::parse("task-1").unwrap(),
                run_id: None,
                artifact_type: ArtifactType::well_known(ArtifactType::JSON),
                location: "out/x.json".to_owned(),
                provenance: ArtifactProvenance {
                    generator: "fixture".to_owned(),
                    source_refs: vec![],
                    created_at: lumi_protocol::Timestamp::UNIX_EPOCH,
                },
                validation_status: lumi_protocol::ValidationStatus::Valid,
                publication_state: lumi_protocol::PublicationState::Draft,
                sha256: None,
            },
        );
        let postcondition = Postcondition {
            id: PostconditionId::new("art"),
            description: "artifact valid".to_owned(),
            check: PostconditionCheck::ArtifactValid {
                artifact_id: artifact_id.clone(),
            },
        };
        let (status, _) = PostconditionVerifier.verify_all(&[postcondition], &env);
        assert_eq!(status, VerificationStatus::Passed);
        let _ = Target::canonical("unused");
    }
}
