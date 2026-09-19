//! Report-only scorecards for bounded digital roles.
//!
//! A role scorecard is an evidence contract, not an execution or policy
//! authority.  It deliberately keeps the raw identities and time windows in
//! the record so that a caller cannot turn a short, synthetic, or mixed
//! cohort into a production certification by changing a label.

use serde::{de, Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Seconds in an exact seven-day window.
pub const SECONDS_PER_WEEK: i64 = 7 * 24 * 60 * 60;

/// Mature-role verified completion target.
pub const MATURE_MIN_VERIFIED_NUMERATOR: u128 = 99;
pub const MATURE_MIN_VERIFIED_DENOMINATOR: u128 = 100;

/// Mature-role baseline-minute displacement target.
pub const MATURE_MIN_MINUTES_REMOVED_NUMERATOR: u128 = 80;
pub const MATURE_MIN_MINUTES_REMOVED_DENOMINATOR: u128 = 100;

/// Mature-role eligible-unit-without-rescue target.
pub const MATURE_MIN_RESCUE_FREE_NUMERATOR: u128 = 80;
pub const MATURE_MIN_RESCUE_FREE_DENOMINATOR: u128 = 100;

/// Mature-role maximum operating-cost share of displaced labor value.
pub const MATURE_MAX_OPERATING_COST_NUMERATOR: u128 = 30;
pub const MATURE_MAX_OPERATING_COST_DENOMINATOR: u128 = 100;

/// Customer-canary minimum verified completion target.
pub const CANARY_MIN_VERIFIED_NUMERATOR: u128 = 95;
pub const CANARY_MIN_VERIFIED_DENOMINATOR: u128 = 100;

/// Customer-canary maximum unexpected rescue rate.  The boundary is strict:
/// exactly five percent does not pass.
pub const CANARY_MAX_RESCUE_NUMERATOR: u128 = 5;
pub const CANARY_MAX_RESCUE_DENOMINATOR: u128 = 100;

const CANARY_MIN_WORK_UNITS: u64 = 100;
const MATURE_MIN_WEEKLY_WINDOWS: usize = 4;

/// Where the observations came from.  Fixture and synthetic evidence can
/// support engineering decisions but can never satisfy a live gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceProvenance {
    Fixture,
    Synthetic,
    Staging,
    Production,
}

impl EvidenceProvenance {
    #[must_use]
    pub const fn is_live(self) -> bool {
        matches!(self, Self::Staging | Self::Production)
    }

    #[must_use]
    pub const fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }
}

/// Whether a source is merely asserted by an importer or was verified by a
/// runtime/reviewer path.  Imported assertions remain visible in reports but
/// do not satisfy live certification gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceTrust {
    ImportedAssertion,
    RuntimeVerified,
}

impl<'de> Deserialize<'de> for EvidenceTrust {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "imported_assertion" => Ok(Self::ImportedAssertion),
            "runtime_verified" => Err(de::Error::custom(
                "runtime_verified evidence must be attached by a trusted verifier",
            )),
            _ => Err(de::Error::custom("unknown evidence trust value")),
        }
    }
}

/// A separate reviewer proof reference.  This is deliberately a reference,
/// not an authority token: the scorecard records that a reviewer supplied
/// proof, while policy/runtime code remains responsible for authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewerProof {
    pub reviewer_id: String,
    pub proof_reference: String,
    pub proof_digest: String,
    pub reviewed_at_unix_seconds: i64,
}

/// Stable tenant, role, deployment, and version identity for one scorecard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleIdentity {
    pub tenant_id: String,
    pub role_id: String,
    pub deployment_id: String,
    pub role_version: String,
    pub runtime_version: String,
}

/// An exact half-open time window `[start, end)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeWindow {
    pub window_id: String,
    pub start_unix_seconds: i64,
    pub end_unix_seconds: i64,
}

impl TimeWindow {
    #[must_use]
    pub const fn duration_seconds(&self) -> Option<i64> {
        self.end_unix_seconds.checked_sub(self.start_unix_seconds)
    }

    #[must_use]
    pub const fn contains(&self, timestamp_unix_seconds: i64) -> bool {
        timestamp_unix_seconds >= self.start_unix_seconds
            && timestamp_unix_seconds < self.end_unix_seconds
    }
}

/// A source reference attached to a scorecard or a particular window/unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceReference {
    pub reference: String,
    /// Runtime-verified evidence must provide a non-empty digest. Imported
    /// legacy assertions may omit it and remain non-certifying.
    #[serde(default)]
    pub evidence_digest: String,
    pub provenance: EvidenceProvenance,
    pub trust: EvidenceTrust,
    pub tenant_id: String,
    pub role_id: String,
    pub deployment_id: String,
    pub role_version: String,
    pub runtime_version: String,
    pub window_id: Option<String>,
    pub reviewer_proof: Option<ReviewerProof>,
}

/// Human-time categories for one received work unit.
///
/// The categories are disjoint.  When all operator-time categories are
/// present, approvals + exceptions + rescue + review must equal
/// `residual_human_minutes`. Support is tracked separately because its loaded
/// dollar cost is already included in the window operating-cost ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanTimeMeasurement {
    pub baseline_human_minutes: Option<u64>,
    pub residual_human_minutes: Option<u64>,
    pub expected_approval_minutes: Option<u64>,
    pub expected_exception_minutes: Option<u64>,
    pub unexpected_rescue_minutes: Option<u64>,
    pub support_minutes: Option<u64>,
    pub review_minutes: Option<u64>,
}

/// Work-unit outcomes and safety observations.  A record represents one
/// received/attempted unit; its runtime cost is therefore included even when
/// the unit fails or is ineligible for the routine denominator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkUnitMeasurement {
    pub work_unit_id: String,
    pub tenant_id: String,
    pub role_id: String,
    pub deployment_id: String,
    pub role_version: String,
    pub runtime_version: String,
    pub window_id: String,
    pub observed_at_unix_seconds: i64,
    pub eligible: bool,
    pub completed: bool,
    pub verified: bool,
    pub human_time: HumanTimeMeasurement,
    #[serde(default)]
    pub cycle_time_before_minutes: Option<u64>,
    #[serde(default)]
    pub cycle_time_after_minutes: Option<u64>,
    pub runtime_cost_micro_usd: Option<u64>,
    pub wrong_target_events: Option<u64>,
    pub ambiguous_state_events: Option<u64>,
    pub unauthorized_side_effects: Option<u64>,
    pub unexpected_rescue_count: Option<u64>,
    pub exception_count: Option<u64>,
    pub escalation_count: Option<u64>,
    pub correctly_escalated_exceptions: Option<u64>,
    pub incorrectly_handled_exceptions: Option<u64>,
    pub sla_met: Option<bool>,
    pub safety_regression: Option<bool>,
    pub privacy_regression: Option<bool>,
}

/// Window-scoped costs.  Runtime cost belongs to each attempted unit; these
/// fields cover the remaining fully loaded operating costs exactly once per
/// window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowCostMeasurement {
    pub window_id: String,
    pub support_cost_micro_usd: Option<u64>,
    pub deployment_cost_micro_usd: Option<u64>,
}

/// Deployment/reuse measurements kept in the report so customer-specific
/// effort is not hidden behind a reliability score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReuseMeasurement {
    pub deployment_hours: Option<u64>,
    pub customer_specific_code_percent: Option<u8>,
    pub shared_role_logic_percent: Option<u8>,
}

/// The coverage claim used by the report.  A true claim must point to an
/// evidence reference in the same scorecard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepresentativeCoverage {
    pub representative: bool,
    pub population_description: String,
    pub evidence_reference: String,
}

/// Raw role scorecard input.  This is data for assessment, never an executable
/// permission or certification token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleScorecard {
    pub identity: RoleIdentity,
    pub provenance: EvidenceProvenance,
    pub coverage: RepresentativeCoverage,
    pub windows: Vec<TimeWindow>,
    pub work_units: Vec<WorkUnitMeasurement>,
    pub window_costs: Vec<WindowCostMeasurement>,
    pub reuse: ReuseMeasurement,
    pub loaded_labor_cost_per_hour_micro_usd: Option<u64>,
    pub evidence: Vec<EvidenceReference>,
}

/// Exact ratio with the denominator kept visible in reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactRatio {
    pub numerator: u128,
    pub denominator: u128,
}

impl ExactRatio {
    #[must_use]
    pub const fn new(numerator: u128, denominator: u128) -> Option<Self> {
        if denominator == 0 {
            None
        } else {
            Some(Self {
                numerator,
                denominator,
            })
        }
    }

    #[must_use]
    pub fn as_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    #[must_use]
    pub const fn at_least(self, required_numerator: u128, required_denominator: u128) -> bool {
        self.numerator.saturating_mul(required_denominator)
            >= required_numerator.saturating_mul(self.denominator)
    }

    #[must_use]
    pub const fn at_most(self, required_numerator: u128, required_denominator: u128) -> bool {
        self.numerator.saturating_mul(required_denominator)
            <= required_numerator.saturating_mul(self.denominator)
    }
}

/// A ratio whose measured numerator may be negative.  This is used for
/// human-time displacement so extra human work remains visible and fails a
/// replacement gate instead of becoming an unknown or zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedRatio {
    pub numerator: i128,
    pub denominator: u128,
}

impl SignedRatio {
    #[must_use]
    pub const fn new(numerator: i128, denominator: u128) -> Option<Self> {
        if denominator == 0 {
            None
        } else {
            Some(Self {
                numerator,
                denominator,
            })
        }
    }

    #[must_use]
    pub fn as_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    #[must_use]
    pub fn at_least(self, required_numerator: u128, required_denominator: u128) -> bool {
        self.numerator >= 0
            && (self.numerator as u128).saturating_mul(required_denominator)
                >= required_numerator.saturating_mul(self.denominator)
    }
}

/// Cost per verified unit with both numerator and denominator preserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostPerVerifiedUnit {
    pub attempted_cost_micro_usd: u128,
    pub verified_work_units: u64,
}

impl CostPerVerifiedUnit {
    #[must_use]
    pub fn floor_micro_usd(self) -> Option<u128> {
        (self.verified_work_units != 0)
            .then_some(self.attempted_cost_micro_usd / u128::from(self.verified_work_units))
    }
}

/// Aggregated measurements.  Optional derived values remain `None` when any
/// required source measurement is absent; missing data never becomes zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMetrics {
    pub received_work_units: u64,
    pub eligible_work_units: u64,
    pub routine_coverage_received_work_units: u64,
    pub routine_coverage_eligible_work_units: u64,
    pub routine_coverage_rate: Option<ExactRatio>,
    pub completed_work_units: u64,
    pub verified_work_units: u64,
    pub eligible_work_units_without_unexpected_rescue: Option<u64>,
    /// Eligible units with at least one unexpected rescue; the canary rescue
    /// rate uses this unit count, not the all-attempt event count.
    pub eligible_work_units_with_unexpected_rescue: Option<u64>,
    pub sla_compliant_work_units: Option<u64>,
    pub sla_compliance_rate: Option<ExactRatio>,
    pub verified_completion_rate: Option<ExactRatio>,
    pub rescue_free_eligible_rate: Option<ExactRatio>,
    pub baseline_human_minutes: Option<u128>,
    pub residual_human_minutes: Option<u128>,
    pub expected_approval_minutes: Option<u128>,
    pub expected_exception_minutes: Option<u128>,
    pub unexpected_rescue_minutes: Option<u128>,
    pub support_minutes: Option<u128>,
    pub review_minutes: Option<u128>,
    pub displaced_human_minutes: Option<i128>,
    pub minutes_removed_rate: Option<SignedRatio>,
    pub runtime_cost_micro_usd: Option<u128>,
    pub support_cost_micro_usd: Option<u128>,
    pub deployment_cost_micro_usd: Option<u128>,
    pub total_attempted_cost_micro_usd: Option<u128>,
    pub cost_per_verified_work_unit: Option<CostPerVerifiedUnit>,
    pub displaced_labor_value_micro_usd: Option<i128>,
    /// Positive means loss; negative means net value created.
    pub signed_net_loss_micro_usd: Option<i128>,
    pub operating_cost_share: Option<ExactRatio>,
    pub wrong_target_events: Option<u128>,
    pub ambiguous_state_events: Option<u128>,
    pub unauthorized_side_effects: Option<u128>,
    pub unexpected_rescue_events: Option<u128>,
    pub exception_count: Option<u128>,
    pub exception_work_units: Option<u64>,
    pub routine_exception_unit_rate: Option<ExactRatio>,
    pub escalation_count: Option<u128>,
    pub correctly_escalated_exceptions: Option<u128>,
    pub incorrectly_handled_exceptions: Option<u128>,
    pub safety_regressions: Option<u128>,
    pub privacy_regressions: Option<u128>,
    pub total_human_attention_minutes: Option<u128>,
    pub human_attention_minutes_per_100_received: Option<ExactRatio>,
    pub cycle_time_before_minutes_per_unit: Option<ExactRatio>,
    pub cycle_time_after_minutes_per_unit: Option<ExactRatio>,
    pub cycle_time_reduction_minutes_per_unit: Option<SignedRatio>,
    pub cycle_time_reduction_rate: Option<SignedRatio>,
    pub signed_net_value_created_micro_usd: Option<i128>,
    pub recurring_net_value_created_micro_usd: Option<i128>,
    /// Deployment cost divided by recurring net value, measured in the same
    /// observed cohort-window units. This is a hypothesis, never a calendar
    /// payback promise.
    pub payback_hypothesis_observed_windows: Option<ExactRatio>,
    /// Longest consecutive seven-day streak with eligible work in every
    /// window and trusted source coverage for every window.
    pub exact_weekly_window_count: usize,
    pub deployment_hours: Option<u64>,
    pub customer_specific_code_percent: Option<u8>,
    pub shared_role_logic_percent: Option<u8>,
}

/// Validation error category.  These are data-integrity errors, not runtime
/// policy decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScorecardErrorCode {
    EmptyCohort,
    MissingIdentity,
    DuplicateWorkUnit,
    DuplicateWindow,
    DuplicateEvidence,
    DuplicateWindowCost,
    InvalidWindow,
    GappedWindows,
    OverlappingWindows,
    OutOfWindow,
    UnknownWindow,
    MixedTenant,
    MixedRole,
    MixedDeployment,
    MixedRoleVersion,
    MixedRuntimeVersion,
    MixedProvenance,
    InconsistentMeasurement,
    InvalidEvidence,
    InvalidCoverage,
    UnknownCostWindow,
}

/// One deterministic validation error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScorecardError {
    pub code: ScorecardErrorCode,
    pub detail: String,
}

impl ScorecardError {
    fn new(code: ScorecardErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ScorecardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.detail)
    }
}

impl std::error::Error for ScorecardError {}

/// Gate category used in an assessment report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Pass,
    Fail,
    Unknown,
}

/// Why a gate did not pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateReasonCode {
    InvalidScorecard,
    UnsupportedProvenance,
    ImportedEvidenceOnly,
    MissingReviewerProof,
    NotRepresentative,
    InsufficientWorkUnits,
    NoEligibleWorkUnits,
    NoVerifiedWorkUnits,
    LowVerifiedCompletion,
    RescueRateTooHigh,
    UnauthorizedSideEffects,
    WrongTargetEffects,
    AmbiguousState,
    MinutesRemovedTooLow,
    RescueFreeUnitsTooLow,
    OperatingCostTooHigh,
    MissingCost,
    MissingHumanTime,
    MissingMeasurement,
    IncorrectExceptionHandling,
    MissingExceptionMeasurement,
    SafetyRegression,
    PrivacyRegression,
    InsufficientConsecutiveWeeklyWindows,
}

/// A structured gate reason suitable for a human review report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateReason {
    pub code: GateReasonCode,
    pub detail: String,
}

/// Canary or mature assessment result.  A pass is a report conclusion only;
/// it does not grant permissions, alter policy, or mark a deployment live.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateResult {
    pub status: GateStatus,
    pub reasons: Vec<GateReason>,
}

/// Internal maturity language used by the report.  The enum is deliberately
/// a recommendation, not an authority or an executable certification state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecommendedMaturity {
    Assisted,
    AutomatedWorkflow,
    AutonomousRoutine,
    RoleCapacityReplacementCandidate,
    HardenedDigitalRole,
}

/// Full scorecard assessment.  Consumers must still perform their own policy,
/// approval, release, and deployment checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleAssessment {
    pub identity: RoleIdentity,
    pub provenance: EvidenceProvenance,
    pub valid: bool,
    pub validation_errors: Vec<ScorecardError>,
    pub metrics: Option<RoleMetrics>,
    pub canary: GateResult,
    pub mature: GateResult,
    pub recommended_maturity: RecommendedMaturity,
}

impl RoleScorecard {
    /// Validate structural identity, windows, evidence, and outcome
    /// consistency.  Missing measurements are intentionally not errors; they
    /// become unknown gate results during [`Self::assess`].
    pub fn validate(&self) -> Result<(), Vec<ScorecardError>> {
        let mut errors = Vec::new();
        self.collect_validation_errors(&mut errors);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Produce a report-only assessment.  This method cannot authorize work
    /// or mutate any runtime state.  It intentionally treats every source as
    /// imported unless a trusted runtime verifier is supplied through
    /// [`Self::assess_with_verifier`].
    #[must_use]
    pub fn assess(&self) -> RoleAssessment {
        self.assess_with_verifier(|_, _| false)
    }

    /// Produce a report after an application-owned verifier has checked the
    /// evidence reference, tenant/deployment identity, and audit digest.  The
    /// callback is an observation hook only; this method still cannot grant
    /// permissions or mutate runtime state.
    #[must_use]
    pub fn assess_with_verifier<F>(&self, verifier: F) -> RoleAssessment
    where
        F: Fn(&EvidenceReference, &RoleIdentity) -> bool,
    {
        let mut validation_errors = Vec::new();
        self.collect_validation_errors(&mut validation_errors);
        if !validation_errors.is_empty() {
            let invalid_canary = GateResult {
                status: GateStatus::Unknown,
                reasons: vec![GateReason {
                    code: GateReasonCode::InvalidScorecard,
                    detail: "structural validation failed; no gate can pass".to_owned(),
                }],
            };
            let invalid_mature = invalid_canary.clone();
            return RoleAssessment {
                identity: self.identity.clone(),
                provenance: self.provenance,
                valid: false,
                validation_errors,
                metrics: None,
                canary: invalid_canary,
                mature: invalid_mature,
                recommended_maturity: RecommendedMaturity::Assisted,
            };
        }

        let verifier: &dyn Fn(&EvidenceReference, &RoleIdentity) -> bool = &verifier;
        let metrics = self.derive_metrics(verifier);
        let canary = self.evaluate_canary(&metrics, verifier);
        let mature = self.evaluate_mature(&metrics, verifier);
        let recommended_maturity = if mature.status == GateStatus::Pass {
            RecommendedMaturity::HardenedDigitalRole
        } else if canary.status == GateStatus::Pass {
            RecommendedMaturity::AutonomousRoutine
        } else {
            RecommendedMaturity::Assisted
        };
        RoleAssessment {
            identity: self.identity.clone(),
            provenance: self.provenance,
            valid: true,
            validation_errors,
            metrics: Some(metrics),
            canary,
            mature,
            recommended_maturity,
        }
    }

    fn collect_validation_errors(&self, errors: &mut Vec<ScorecardError>) {
        self.validate_identity(errors);
        if self.work_units.is_empty() {
            errors.push(ScorecardError::new(
                ScorecardErrorCode::EmptyCohort,
                "a scorecard with no received work units cannot pass any gate",
            ));
        }

        let windows_by_id = self.validate_windows(errors);
        self.validate_evidence(errors, &windows_by_id);
        self.validate_coverage(errors);
        self.validate_window_costs(errors, &windows_by_id);
        self.validate_work_units(errors, &windows_by_id);
        self.validate_reuse(errors);
    }

    fn validate_identity(&self, errors: &mut Vec<ScorecardError>) {
        let fields = [
            ("tenant_id", self.identity.tenant_id.as_str()),
            ("role_id", self.identity.role_id.as_str()),
            ("deployment_id", self.identity.deployment_id.as_str()),
            ("role_version", self.identity.role_version.as_str()),
            ("runtime_version", self.identity.runtime_version.as_str()),
        ];
        for (name, value) in fields {
            if value.trim().is_empty() {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::MissingIdentity,
                    format!("{name} is required"),
                ));
            }
        }
    }

    fn validate_windows(&self, errors: &mut Vec<ScorecardError>) -> BTreeMap<String, TimeWindow> {
        let mut windows_by_id = BTreeMap::new();
        let mut ordered = self.windows.clone();
        ordered.sort_by_key(|window| (window.start_unix_seconds, window.end_unix_seconds));
        for window in &ordered {
            if window.window_id.trim().is_empty()
                || window
                    .duration_seconds()
                    .is_none_or(|duration| duration <= 0)
            {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::InvalidWindow,
                    format!(
                        "window {:?} must have an id and start < end",
                        window.window_id
                    ),
                ));
            }
            if windows_by_id
                .insert(window.window_id.clone(), window.clone())
                .is_some()
            {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::DuplicateWindow,
                    format!("window {} appears more than once", window.window_id),
                ));
            }
        }
        for pair in ordered.windows(2) {
            let previous = &pair[0];
            let next = &pair[1];
            if next.start_unix_seconds < previous.end_unix_seconds {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::OverlappingWindows,
                    format!(
                        "window {} overlaps window {}",
                        previous.window_id, next.window_id
                    ),
                ));
            } else if next.start_unix_seconds != previous.end_unix_seconds {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::GappedWindows,
                    format!(
                        "window {} ends at {}, but window {} starts at {}",
                        previous.window_id,
                        previous.end_unix_seconds,
                        next.window_id,
                        next.start_unix_seconds
                    ),
                ));
            }
        }
        windows_by_id
    }

    fn validate_evidence(
        &self,
        errors: &mut Vec<ScorecardError>,
        windows_by_id: &BTreeMap<String, TimeWindow>,
    ) {
        let mut references = BTreeSet::new();
        for evidence in &self.evidence {
            if evidence.reference.trim().is_empty() {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::InvalidEvidence,
                    "evidence reference cannot be empty",
                ));
            }
            if !references.insert(evidence.reference.clone()) {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::DuplicateEvidence,
                    format!(
                        "evidence reference {} appears more than once",
                        evidence.reference
                    ),
                ));
            }
            if evidence.provenance != self.provenance {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::MixedProvenance,
                    format!(
                        "evidence {} is {:?}, scorecard is {:?}",
                        evidence.reference, evidence.provenance, self.provenance
                    ),
                ));
            }
            self.validate_evidence_identity(evidence, errors);
            if let Some(window_id) = &evidence.window_id {
                if !windows_by_id.contains_key(window_id) {
                    errors.push(ScorecardError::new(
                        ScorecardErrorCode::UnknownWindow,
                        format!(
                            "evidence {} names unknown window {window_id}",
                            evidence.reference
                        ),
                    ));
                }
            }
            if evidence.trust == EvidenceTrust::RuntimeVerified {
                match &evidence.reviewer_proof {
                    Some(proof)
                        if !proof.reviewer_id.trim().is_empty()
                            && !proof.proof_reference.trim().is_empty()
                            && !proof.proof_digest.trim().is_empty()
                            && !evidence.evidence_digest.trim().is_empty()
                            && proof.proof_reference != evidence.reference
                            && proof.reviewed_at_unix_seconds > 0 => {}
                    Some(_) => errors.push(ScorecardError::new(
                        ScorecardErrorCode::InvalidEvidence,
                        format!(
                            "runtime-verified evidence {} needs separate reviewer proof",
                            evidence.reference
                        ),
                    )),
                    None => errors.push(ScorecardError::new(
                        ScorecardErrorCode::InvalidEvidence,
                        format!(
                            "runtime-verified evidence {} needs reviewer proof",
                            evidence.reference
                        ),
                    )),
                }
            }
        }
    }

    fn validate_evidence_identity(
        &self,
        evidence: &EvidenceReference,
        errors: &mut Vec<ScorecardError>,
    ) {
        if evidence.tenant_id != self.identity.tenant_id {
            errors.push(ScorecardError::new(
                ScorecardErrorCode::MixedTenant,
                format!("evidence {} belongs to another tenant", evidence.reference),
            ));
        }
        if evidence.role_id != self.identity.role_id {
            errors.push(ScorecardError::new(
                ScorecardErrorCode::MixedRole,
                format!("evidence {} belongs to another role", evidence.reference),
            ));
        }
        if evidence.deployment_id != self.identity.deployment_id {
            errors.push(ScorecardError::new(
                ScorecardErrorCode::MixedDeployment,
                format!(
                    "evidence {} belongs to another deployment",
                    evidence.reference
                ),
            ));
        }
        if evidence.role_version != self.identity.role_version {
            errors.push(ScorecardError::new(
                ScorecardErrorCode::MixedRoleVersion,
                format!("evidence {} uses another role version", evidence.reference),
            ));
        }
        if evidence.runtime_version != self.identity.runtime_version {
            errors.push(ScorecardError::new(
                ScorecardErrorCode::MixedRuntimeVersion,
                format!(
                    "evidence {} uses another runtime version",
                    evidence.reference
                ),
            ));
        }
    }

    fn validate_coverage(&self, errors: &mut Vec<ScorecardError>) {
        if self.coverage.population_description.trim().is_empty() {
            errors.push(ScorecardError::new(
                ScorecardErrorCode::InvalidCoverage,
                "representative coverage needs a population description",
            ));
        }
        if self.coverage.representative
            && !self
                .evidence
                .iter()
                .any(|evidence| evidence.reference == self.coverage.evidence_reference)
        {
            errors.push(ScorecardError::new(
                ScorecardErrorCode::InvalidCoverage,
                format!(
                    "representative coverage references missing evidence {}",
                    self.coverage.evidence_reference
                ),
            ));
        }
    }

    fn validate_window_costs(
        &self,
        errors: &mut Vec<ScorecardError>,
        windows_by_id: &BTreeMap<String, TimeWindow>,
    ) {
        let mut seen = BTreeSet::new();
        for costs in &self.window_costs {
            if !windows_by_id.contains_key(&costs.window_id) {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::UnknownCostWindow,
                    format!("costs name unknown window {}", costs.window_id),
                ));
            }
            if !seen.insert(costs.window_id.clone()) {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::DuplicateWindowCost,
                    format!("costs for window {} appear more than once", costs.window_id),
                ));
            }
        }
    }

    fn validate_work_units(
        &self,
        errors: &mut Vec<ScorecardError>,
        windows_by_id: &BTreeMap<String, TimeWindow>,
    ) {
        let mut work_unit_ids = BTreeSet::new();
        for unit in &self.work_units {
            if unit.work_unit_id.trim().is_empty() {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::InconsistentMeasurement,
                    "work unit id cannot be empty",
                ));
            }
            if !work_unit_ids.insert(unit.work_unit_id.clone()) {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::DuplicateWorkUnit,
                    format!("work unit {} appears more than once", unit.work_unit_id),
                ));
            }
            self.validate_work_unit_identity(unit, errors);
            let Some(window) = windows_by_id.get(&unit.window_id) else {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::UnknownWindow,
                    format!(
                        "work unit {} names unknown window {}",
                        unit.work_unit_id, unit.window_id
                    ),
                ));
                continue;
            };
            if !window.contains(unit.observed_at_unix_seconds) {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::OutOfWindow,
                    format!(
                        "work unit {} timestamp {} is outside window {}",
                        unit.work_unit_id, unit.observed_at_unix_seconds, unit.window_id
                    ),
                ));
            }
            if unit.verified && !unit.completed {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::InconsistentMeasurement,
                    format!(
                        "work unit {} is verified without being completed",
                        unit.work_unit_id
                    ),
                ));
            }
            if unit.verified && !unit.eligible {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::InconsistentMeasurement,
                    format!(
                        "ineligible work unit {} cannot be verified",
                        unit.work_unit_id
                    ),
                ));
            }
            self.validate_human_time(unit, errors);
            self.validate_exception_counts(unit, errors);
        }
    }

    fn validate_reuse(&self, errors: &mut Vec<ScorecardError>) {
        for (label, value) in [
            (
                "customer_specific_code_percent",
                self.reuse.customer_specific_code_percent,
            ),
            (
                "shared_role_logic_percent",
                self.reuse.shared_role_logic_percent,
            ),
        ] {
            if value.is_some_and(|percent| percent > 100) {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::InconsistentMeasurement,
                    format!("{label} must be between 0 and 100"),
                ));
            }
        }
    }

    fn validate_work_unit_identity(
        &self,
        unit: &WorkUnitMeasurement,
        errors: &mut Vec<ScorecardError>,
    ) {
        let identity_fields = [
            (
                ScorecardErrorCode::MixedTenant,
                "tenant",
                unit.tenant_id.as_str(),
                self.identity.tenant_id.as_str(),
            ),
            (
                ScorecardErrorCode::MixedRole,
                "role",
                unit.role_id.as_str(),
                self.identity.role_id.as_str(),
            ),
            (
                ScorecardErrorCode::MixedDeployment,
                "deployment",
                unit.deployment_id.as_str(),
                self.identity.deployment_id.as_str(),
            ),
            (
                ScorecardErrorCode::MixedRoleVersion,
                "role version",
                unit.role_version.as_str(),
                self.identity.role_version.as_str(),
            ),
            (
                ScorecardErrorCode::MixedRuntimeVersion,
                "runtime version",
                unit.runtime_version.as_str(),
                self.identity.runtime_version.as_str(),
            ),
        ];
        for (code, label, actual, expected) in identity_fields {
            if actual != expected {
                errors.push(ScorecardError::new(
                    code,
                    format!("work unit {} has mixed {label}", unit.work_unit_id),
                ));
            }
        }
    }

    fn validate_human_time(&self, unit: &WorkUnitMeasurement, errors: &mut Vec<ScorecardError>) {
        let time = &unit.human_time;
        let categories = [
            time.expected_approval_minutes,
            time.expected_exception_minutes,
            time.unexpected_rescue_minutes,
            time.review_minutes,
        ];
        if let (Some(residual), true) = (
            time.residual_human_minutes,
            categories.iter().all(Option::is_some),
        ) {
            let category_total: u64 = categories.iter().flatten().copied().sum();
            if residual != category_total {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::InconsistentMeasurement,
                    format!(
                        "work unit {} residual minutes do not equal disjoint human-time categories",
                        unit.work_unit_id
                    ),
                ));
            }
        }
    }

    fn validate_exception_counts(
        &self,
        unit: &WorkUnitMeasurement,
        errors: &mut Vec<ScorecardError>,
    ) {
        if let (Some(exception_count), Some(correct), Some(incorrect), Some(escalations)) = (
            unit.exception_count,
            unit.correctly_escalated_exceptions,
            unit.incorrectly_handled_exceptions,
            unit.escalation_count,
        ) {
            if correct.saturating_add(incorrect) != exception_count
                || correct > escalations
                || escalations > exception_count
            {
                errors.push(ScorecardError::new(
                    ScorecardErrorCode::InconsistentMeasurement,
                    format!(
                        "work unit {} exception/escalation counts are inconsistent",
                        unit.work_unit_id
                    ),
                ));
            }
        }
    }

    fn derive_metrics(
        &self,
        verifier: &dyn Fn(&EvidenceReference, &RoleIdentity) -> bool,
    ) -> RoleMetrics {
        let received = self.work_units.len() as u64;
        let eligible = self.work_units.iter().filter(|unit| unit.eligible).count() as u64;
        let completed = self
            .work_units
            .iter()
            .filter(|unit| unit.eligible && unit.completed)
            .count() as u64;
        let verified = self
            .work_units
            .iter()
            .filter(|unit| unit.eligible && unit.verified)
            .count() as u64;
        let rescue_free = count_eligible_without_rescue(&self.work_units);
        let rescued_eligible = count_eligible_with_rescue(&self.work_units);
        let sla_compliant = count_sla_compliant(&self.work_units);
        let exception_work_units = count_eligible_with_exception(&self.work_units);

        let baseline = sum_eligible(&self.work_units, |unit| {
            unit.human_time.baseline_human_minutes
        });
        let residual = sum_eligible(&self.work_units, |unit| {
            unit.human_time.residual_human_minutes
        });
        let approvals = sum_eligible(&self.work_units, |unit| {
            unit.human_time.expected_approval_minutes
        });
        let exceptions = sum_eligible(&self.work_units, |unit| {
            unit.human_time.expected_exception_minutes
        });
        let rescues = sum_eligible(&self.work_units, |unit| {
            unit.human_time.unexpected_rescue_minutes
        });
        let support = sum_eligible(&self.work_units, |unit| unit.human_time.support_minutes);
        let review = sum_eligible(&self.work_units, |unit| unit.human_time.review_minutes);
        let all_residual = sum_all(&self.work_units, |unit| {
            unit.human_time.residual_human_minutes
        });
        let all_support = sum_all(&self.work_units, |unit| unit.human_time.support_minutes);
        let total_human_attention = all_residual
            .zip(all_support)
            .and_then(|(residual, support)| residual.checked_add(support));
        let cycle_before = sum_eligible(&self.work_units, |unit| unit.cycle_time_before_minutes);
        let cycle_after = sum_eligible(&self.work_units, |unit| unit.cycle_time_after_minutes);
        let cycle_reduction = cycle_before.zip(cycle_after).and_then(|(before, after)| {
            Some(i128::try_from(before).ok()? - i128::try_from(after).ok()?)
        });
        let displaced = baseline.zip(residual).and_then(|(baseline, residual)| {
            Some(i128::try_from(baseline).ok()? - i128::try_from(residual).ok()?)
        });
        let runtime_cost = sum_all(&self.work_units, |unit| unit.runtime_cost_micro_usd);
        let support_cost = sum_window_costs(&self.window_costs, &self.windows, |cost| {
            cost.support_cost_micro_usd
        });
        let deployment_cost = sum_window_costs(&self.window_costs, &self.windows, |cost| {
            cost.deployment_cost_micro_usd
        });
        let total_cost = runtime_cost
            .zip(support_cost)
            .zip(deployment_cost)
            .map(|((runtime, support), deployment)| runtime + support + deployment);
        let displaced_value = displaced
            .zip(self.loaded_labor_cost_per_hour_micro_usd.map(i128::from))
            .and_then(|(minutes, hourly_cost)| minutes.checked_mul(hourly_cost)?.checked_div(60));
        let signed_net_loss = total_cost.zip(displaced_value).and_then(|(cost, value)| {
            let cost = i128::try_from(cost).ok()?;
            Some(cost - value)
        });
        let signed_net_value_created = signed_net_loss.and_then(i128::checked_neg);
        let recurring_net_value = displaced_value
            .zip(runtime_cost)
            .zip(support_cost)
            .and_then(|((value, runtime), support)| {
                Some(value - i128::try_from(runtime).ok()? - i128::try_from(support).ok()?)
            });
        let payback_hypothesis =
            recurring_net_value
                .zip(deployment_cost)
                .and_then(|(net, deployment)| {
                    (net > 0).then_some(ExactRatio::new(deployment, u128::try_from(net).ok()?))?
                });
        let operating_cost_share = total_cost.zip(displaced_value).and_then(|(cost, value)| {
            (value > 0).then_some(ExactRatio::new(cost, u128::try_from(value).ok()?))?
        });
        let weekly_count = consecutive_measured_weekly_windows(self, verifier);

        RoleMetrics {
            received_work_units: received,
            eligible_work_units: eligible,
            routine_coverage_received_work_units: received,
            routine_coverage_eligible_work_units: eligible,
            routine_coverage_rate: ExactRatio::new(u128::from(eligible), u128::from(received)),
            completed_work_units: completed,
            verified_work_units: verified,
            eligible_work_units_without_unexpected_rescue: rescue_free,
            eligible_work_units_with_unexpected_rescue: rescued_eligible,
            sla_compliant_work_units: sla_compliant,
            sla_compliance_rate: sla_compliant
                .and_then(|count| ExactRatio::new(u128::from(count), u128::from(eligible))),
            verified_completion_rate: ExactRatio::new(u128::from(verified), u128::from(eligible)),
            rescue_free_eligible_rate: rescue_free
                .and_then(|count| ExactRatio::new(u128::from(count), u128::from(eligible))),
            baseline_human_minutes: baseline,
            residual_human_minutes: residual,
            expected_approval_minutes: approvals,
            expected_exception_minutes: exceptions,
            unexpected_rescue_minutes: rescues,
            support_minutes: support,
            review_minutes: review,
            displaced_human_minutes: displaced,
            minutes_removed_rate: displaced.and_then(|removed| {
                baseline.and_then(|baseline| SignedRatio::new(removed, baseline))
            }),
            runtime_cost_micro_usd: runtime_cost,
            support_cost_micro_usd: support_cost,
            deployment_cost_micro_usd: deployment_cost,
            total_attempted_cost_micro_usd: total_cost,
            cost_per_verified_work_unit: total_cost.and_then(|cost| {
                (verified != 0).then_some(CostPerVerifiedUnit {
                    attempted_cost_micro_usd: cost,
                    verified_work_units: verified,
                })
            }),
            displaced_labor_value_micro_usd: displaced_value,
            signed_net_loss_micro_usd: signed_net_loss,
            operating_cost_share,
            wrong_target_events: sum_all(&self.work_units, |unit| unit.wrong_target_events),
            ambiguous_state_events: sum_all(&self.work_units, |unit| unit.ambiguous_state_events),
            unauthorized_side_effects: sum_all(&self.work_units, |unit| {
                unit.unauthorized_side_effects
            }),
            unexpected_rescue_events: sum_all(&self.work_units, |unit| {
                unit.unexpected_rescue_count
            }),
            exception_count: sum_eligible(&self.work_units, |unit| unit.exception_count),
            exception_work_units,
            routine_exception_unit_rate: exception_work_units
                .and_then(|count| ExactRatio::new(u128::from(count), u128::from(eligible))),
            escalation_count: sum_eligible(&self.work_units, |unit| unit.escalation_count),
            correctly_escalated_exceptions: sum_eligible(&self.work_units, |unit| {
                unit.correctly_escalated_exceptions
            }),
            incorrectly_handled_exceptions: sum_eligible(&self.work_units, |unit| {
                unit.incorrectly_handled_exceptions
            }),
            safety_regressions: sum_flags(&self.work_units, |unit| unit.safety_regression),
            privacy_regressions: sum_flags(&self.work_units, |unit| unit.privacy_regression),
            total_human_attention_minutes: total_human_attention,
            human_attention_minutes_per_100_received: total_human_attention
                .and_then(|total| total.checked_mul(100))
                .and_then(|total| ExactRatio::new(total, u128::from(received))),
            cycle_time_before_minutes_per_unit: cycle_before
                .and_then(|total| ExactRatio::new(total, u128::from(eligible))),
            cycle_time_after_minutes_per_unit: cycle_after
                .and_then(|total| ExactRatio::new(total, u128::from(eligible))),
            cycle_time_reduction_minutes_per_unit: cycle_reduction
                .and_then(|total| SignedRatio::new(total, u128::from(eligible))),
            cycle_time_reduction_rate: cycle_reduction.and_then(|reduction| {
                cycle_before.and_then(|before| SignedRatio::new(reduction, before))
            }),
            signed_net_value_created_micro_usd: signed_net_value_created,
            recurring_net_value_created_micro_usd: recurring_net_value,
            payback_hypothesis_observed_windows: payback_hypothesis,
            exact_weekly_window_count: weekly_count,
            deployment_hours: self.reuse.deployment_hours,
            customer_specific_code_percent: self.reuse.customer_specific_code_percent,
            shared_role_logic_percent: self.reuse.shared_role_logic_percent,
        }
    }

    fn evaluate_canary(
        &self,
        metrics: &RoleMetrics,
        verifier: &dyn Fn(&EvidenceReference, &RoleIdentity) -> bool,
    ) -> GateResult {
        let mut reasons = Vec::new();
        if !self.provenance.is_live() {
            reasons.push(reason(
                GateReasonCode::UnsupportedProvenance,
                "canary evidence must be staging or production",
            ));
        }
        if !self.coverage.representative {
            reasons.push(reason(
                GateReasonCode::NotRepresentative,
                "representative coverage is not asserted",
            ));
        }
        if !has_runtime_verified_coverage(self, verifier) {
            reasons.push(reason(
                GateReasonCode::ImportedEvidenceOnly,
                "canary requires runtime-verified evidence for the coverage claim",
            ));
        }
        if metrics.eligible_work_units < CANARY_MIN_WORK_UNITS {
            reasons.push(reason(
                GateReasonCode::InsufficientWorkUnits,
                format!(
                    "{} eligible units; at least {CANARY_MIN_WORK_UNITS} are required",
                    metrics.eligible_work_units
                ),
            ));
        }
        match metrics.verified_completion_rate {
            Some(rate)
                if rate.at_least(
                    CANARY_MIN_VERIFIED_NUMERATOR,
                    CANARY_MIN_VERIFIED_DENOMINATOR,
                ) => {}
            Some(rate) => reasons.push(reason(
                GateReasonCode::LowVerifiedCompletion,
                format!("verified completion is {:.2}%", rate.as_f64() * 100.0),
            )),
            None => reasons.push(reason(
                GateReasonCode::NoEligibleWorkUnits,
                "verified completion has no eligible denominator",
            )),
        }
        if metrics.baseline_human_minutes.is_none() || metrics.residual_human_minutes.is_none() {
            reasons.push(reason(
                GateReasonCode::MissingHumanTime,
                "canary requires measured baseline and residual human minutes",
            ));
        }
        if metrics.runtime_cost_micro_usd.is_none()
            || metrics.support_cost_micro_usd.is_none()
            || metrics.deployment_cost_micro_usd.is_none()
            || metrics.total_attempted_cost_micro_usd.is_none()
        {
            reasons.push(reason(
                GateReasonCode::MissingCost,
                "canary requires runtime, support, and deployment costs for all attempted work",
            ));
        }
        match (
            metrics.eligible_work_units_with_unexpected_rescue,
            metrics.eligible_work_units,
        ) {
            (Some(rescues), eligible) => {
                if eligible == 0 {
                    reasons.push(reason(
                        GateReasonCode::MissingMeasurement,
                        "unexpected rescue measurement has no eligible denominator",
                    ));
                } else {
                    let below_strict_limit = u128::from(rescues)
                        .saturating_mul(CANARY_MAX_RESCUE_DENOMINATOR)
                        < CANARY_MAX_RESCUE_NUMERATOR.saturating_mul(u128::from(eligible));
                    if !below_strict_limit {
                        reasons.push(reason(
                            GateReasonCode::RescueRateTooHigh,
                            format!(
                                "unexpected rescue affects {:.2}% of eligible units",
                                rescues as f64 / eligible as f64 * 100.0
                            ),
                        ));
                    }
                }
            }
            (None, _) => reasons.push(reason(
                GateReasonCode::MissingMeasurement,
                "unexpected rescue measurement has no eligible denominator",
            )),
        }
        add_zero_reason(
            &mut reasons,
            metrics.unauthorized_side_effects,
            GateReasonCode::UnauthorizedSideEffects,
            "unauthorized side effects",
        );
        add_zero_reason(
            &mut reasons,
            metrics.safety_regressions,
            GateReasonCode::SafetyRegression,
            "safety regressions",
        );
        add_zero_reason(
            &mut reasons,
            metrics.privacy_regressions,
            GateReasonCode::PrivacyRegression,
            "privacy regressions",
        );
        live_gate_result(reasons)
    }

    fn evaluate_mature(
        &self,
        metrics: &RoleMetrics,
        verifier: &dyn Fn(&EvidenceReference, &RoleIdentity) -> bool,
    ) -> GateResult {
        let mut reasons = Vec::new();
        if !self.provenance.is_production() {
            reasons.push(reason(
                GateReasonCode::UnsupportedProvenance,
                "mature role evidence must be production evidence",
            ));
        }
        if !self.coverage.representative {
            reasons.push(reason(
                GateReasonCode::NotRepresentative,
                "representative coverage is not asserted",
            ));
        }
        if !has_runtime_verified_coverage(self, verifier) {
            reasons.push(reason(
                GateReasonCode::MissingReviewerProof,
                "mature assessment requires runtime-verified evidence with reviewer proof",
            ));
        }
        if metrics.exact_weekly_window_count < MATURE_MIN_WEEKLY_WINDOWS {
            reasons.push(reason(
                GateReasonCode::InsufficientConsecutiveWeeklyWindows,
                format!(
                    "{} exact weekly windows; at least {MATURE_MIN_WEEKLY_WINDOWS} consecutive windows are required",
                    metrics.exact_weekly_window_count
                ),
            ));
        }
        match metrics.verified_completion_rate {
            Some(rate)
                if rate.at_least(
                    MATURE_MIN_VERIFIED_NUMERATOR,
                    MATURE_MIN_VERIFIED_DENOMINATOR,
                ) => {}
            Some(rate) => reasons.push(reason(
                GateReasonCode::LowVerifiedCompletion,
                format!("verified completion is {:.2}%", rate.as_f64() * 100.0),
            )),
            None => reasons.push(reason(
                GateReasonCode::NoEligibleWorkUnits,
                "verified completion has no eligible denominator",
            )),
        }
        match (metrics.minutes_removed_rate, metrics.displaced_human_minutes) {
            (Some(rate), _)
                if rate.at_least(
                    MATURE_MIN_MINUTES_REMOVED_NUMERATOR,
                    MATURE_MIN_MINUTES_REMOVED_DENOMINATOR,
                ) => {}
            (Some(rate), _) => reasons.push(reason(
                GateReasonCode::MinutesRemovedTooLow,
                format!("baseline minutes removed is {:.2}%", rate.as_f64() * 100.0),
            )),
            (None, Some(removed)) => reasons.push(reason(
                GateReasonCode::MinutesRemovedTooLow,
                format!("net human minutes released is {removed}; a positive baseline denominator is required"),
            )),
            (None, None) => reasons.push(reason(
                GateReasonCode::MissingHumanTime,
                "baseline/residual human minutes are incomplete",
            )),
        }
        match metrics.rescue_free_eligible_rate {
            Some(rate)
                if rate.at_least(
                    MATURE_MIN_RESCUE_FREE_NUMERATOR,
                    MATURE_MIN_RESCUE_FREE_DENOMINATOR,
                ) => {}
            Some(rate) => reasons.push(reason(
                GateReasonCode::RescueFreeUnitsTooLow,
                format!(
                    "eligible units without unexpected rescue are {:.2}%",
                    rate.as_f64() * 100.0
                ),
            )),
            None => reasons.push(reason(
                GateReasonCode::MissingHumanTime,
                "eligible rescue denominator is missing",
            )),
        }
        match (
            metrics.displaced_labor_value_micro_usd,
            metrics.operating_cost_share,
        ) {
            (Some(value), _) if value <= 0 => reasons.push(reason(
                GateReasonCode::OperatingCostTooHigh,
                format!("displaced labor value is non-positive ({value} micro-USD)"),
            )),
            (Some(_), Some(rate))
                if rate.at_most(
                    MATURE_MAX_OPERATING_COST_NUMERATOR,
                    MATURE_MAX_OPERATING_COST_DENOMINATOR,
                ) => {}
            (_, Some(rate)) => reasons.push(reason(
                GateReasonCode::OperatingCostTooHigh,
                format!("operating cost share is {:.2}%", rate.as_f64() * 100.0),
            )),
            (None, _) | (Some(_), None) => reasons.push(reason(
                GateReasonCode::MissingCost,
                "all attempted runtime/support/deployment costs and displaced value are required",
            )),
        }
        add_zero_reason(
            &mut reasons,
            metrics.unauthorized_side_effects,
            GateReasonCode::UnauthorizedSideEffects,
            "unauthorized side effects",
        );
        add_zero_reason(
            &mut reasons,
            metrics.wrong_target_events,
            GateReasonCode::WrongTargetEffects,
            "wrong-target effects",
        );
        add_zero_reason(
            &mut reasons,
            metrics.ambiguous_state_events,
            GateReasonCode::AmbiguousState,
            "ambiguous states",
        );
        add_zero_reason(
            &mut reasons,
            metrics.incorrectly_handled_exceptions,
            GateReasonCode::IncorrectExceptionHandling,
            "incorrectly handled exceptions",
        );
        if metrics.exception_count.is_none()
            || metrics.escalation_count.is_none()
            || metrics.correctly_escalated_exceptions.is_none()
            || metrics.incorrectly_handled_exceptions.is_none()
        {
            reasons.push(reason(
                GateReasonCode::MissingExceptionMeasurement,
                "exception and escalation categories must be measured for every eligible unit",
            ));
        }
        add_zero_reason(
            &mut reasons,
            metrics.safety_regressions,
            GateReasonCode::SafetyRegression,
            "safety regressions",
        );
        add_zero_reason(
            &mut reasons,
            metrics.privacy_regressions,
            GateReasonCode::PrivacyRegression,
            "privacy regressions",
        );
        live_gate_result(reasons)
    }
}

fn reason(code: GateReasonCode, detail: impl Into<String>) -> GateReason {
    GateReason {
        code,
        detail: detail.into(),
    }
}

fn live_gate_result(reasons: Vec<GateReason>) -> GateResult {
    if reasons.is_empty() {
        GateResult {
            status: GateStatus::Pass,
            reasons,
        }
    } else if reasons.iter().any(|reason| {
        matches!(
            reason.code,
            GateReasonCode::UnsupportedProvenance
                | GateReasonCode::ImportedEvidenceOnly
                | GateReasonCode::NotRepresentative
        )
    }) {
        GateResult {
            status: GateStatus::Fail,
            reasons,
        }
    } else if reasons.iter().any(|reason| {
        matches!(
            reason.code,
            GateReasonCode::MissingCost
                | GateReasonCode::MissingHumanTime
                | GateReasonCode::MissingMeasurement
                | GateReasonCode::MissingExceptionMeasurement
                | GateReasonCode::MissingReviewerProof
                | GateReasonCode::NoEligibleWorkUnits
        )
    }) {
        GateResult {
            status: GateStatus::Unknown,
            reasons,
        }
    } else {
        GateResult {
            status: GateStatus::Fail,
            reasons,
        }
    }
}

fn add_zero_reason(
    reasons: &mut Vec<GateReason>,
    value: Option<u128>,
    code: GateReasonCode,
    label: &str,
) {
    match value {
        Some(0) => {}
        Some(value) => reasons.push(reason(code, format!("{value} {label}"))),
        None => reasons.push(reason(
            GateReasonCode::MissingMeasurement,
            format!("{label} measurement is missing"),
        )),
    }
}

fn has_runtime_verified_coverage(
    scorecard: &RoleScorecard,
    verifier: &dyn Fn(&EvidenceReference, &RoleIdentity) -> bool,
) -> bool {
    scorecard
        .evidence
        .iter()
        .find(|evidence| evidence.reference == scorecard.coverage.evidence_reference)
        .is_some_and(|evidence| {
            evidence.trust == EvidenceTrust::RuntimeVerified
                && evidence.reviewer_proof.is_some()
                && verifier(evidence, &scorecard.identity)
        })
}

fn sum_eligible<F>(units: &[WorkUnitMeasurement], getter: F) -> Option<u128>
where
    F: Fn(&WorkUnitMeasurement) -> Option<u64>,
{
    let mut total = 0_u128;
    for unit in units.iter().filter(|unit| unit.eligible) {
        total = total.checked_add(u128::from(getter(unit)?))?;
    }
    Some(total)
}

fn sum_all<F>(units: &[WorkUnitMeasurement], getter: F) -> Option<u128>
where
    F: Fn(&WorkUnitMeasurement) -> Option<u64>,
{
    let mut total = 0_u128;
    for unit in units {
        total = total.checked_add(u128::from(getter(unit)?))?;
    }
    Some(total)
}

fn sum_window_costs<F>(
    costs: &[WindowCostMeasurement],
    windows: &[TimeWindow],
    getter: F,
) -> Option<u128>
where
    F: Fn(&WindowCostMeasurement) -> Option<u64>,
{
    if costs.len() != windows.len() {
        return None;
    }
    let mut total = 0_u128;
    for cost in costs {
        total = total.checked_add(u128::from(getter(cost)?))?;
    }
    Some(total)
}

fn sum_flags<F>(units: &[WorkUnitMeasurement], getter: F) -> Option<u128>
where
    F: Fn(&WorkUnitMeasurement) -> Option<bool>,
{
    let mut total = 0_u128;
    for unit in units {
        if getter(unit)? {
            total = total.checked_add(1)?;
        }
    }
    Some(total)
}

fn count_eligible_without_rescue(units: &[WorkUnitMeasurement]) -> Option<u64> {
    let mut count = 0_u64;
    for unit in units.iter().filter(|unit| unit.eligible) {
        if unit.unexpected_rescue_count? == 0 {
            count = count.checked_add(1)?;
        }
    }
    Some(count)
}

fn count_eligible_with_rescue(units: &[WorkUnitMeasurement]) -> Option<u64> {
    let mut count = 0_u64;
    for unit in units.iter().filter(|unit| unit.eligible) {
        if unit.unexpected_rescue_count? > 0 {
            count = count.checked_add(1)?;
        }
    }
    Some(count)
}

fn count_eligible_with_exception(units: &[WorkUnitMeasurement]) -> Option<u64> {
    let mut count = 0_u64;
    for unit in units.iter().filter(|unit| unit.eligible) {
        if unit.exception_count? > 0 {
            count = count.checked_add(1)?;
        }
    }
    Some(count)
}

fn count_sla_compliant(units: &[WorkUnitMeasurement]) -> Option<u64> {
    let mut count = 0_u64;
    for unit in units.iter().filter(|unit| unit.eligible) {
        if unit.sla_met? {
            count = count.checked_add(1)?;
        }
    }
    Some(count)
}

fn has_runtime_verified_window_coverage(
    scorecard: &RoleScorecard,
    window_id: &str,
    verifier: &dyn Fn(&EvidenceReference, &RoleIdentity) -> bool,
) -> bool {
    scorecard.evidence.iter().any(|evidence| {
        evidence.trust == EvidenceTrust::RuntimeVerified
            && evidence.reviewer_proof.is_some()
            && (evidence.window_id.as_deref() == Some(window_id) || evidence.window_id.is_none())
            && verifier(evidence, &scorecard.identity)
    })
}

fn consecutive_measured_weekly_windows(
    scorecard: &RoleScorecard,
    verifier: &dyn Fn(&EvidenceReference, &RoleIdentity) -> bool,
) -> usize {
    let mut ordered = scorecard.windows.clone();
    ordered.sort_by_key(|window| (window.start_unix_seconds, window.end_unix_seconds));
    let mut longest = 0;
    let mut current = 0;
    for (index, window) in ordered.iter().enumerate() {
        let contiguous =
            index == 0 || ordered[index - 1].end_unix_seconds == window.start_unix_seconds;
        let measured_eligible_work = scorecard
            .work_units
            .iter()
            .any(|unit| unit.window_id == window.window_id && unit.eligible);
        let trusted_coverage =
            has_runtime_verified_window_coverage(scorecard, &window.window_id, verifier);
        if window.duration_seconds() == Some(SECONDS_PER_WEEK)
            && contiguous
            && measured_eligible_work
            && trusted_coverage
        {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    longest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> RoleIdentity {
        RoleIdentity {
            tenant_id: "tenant-a".to_owned(),
            role_id: "finance-ops".to_owned(),
            deployment_id: "deploy-a".to_owned(),
            role_version: "role-1".to_owned(),
            runtime_version: "runtime-1".to_owned(),
        }
    }

    fn proof() -> ReviewerProof {
        ReviewerProof {
            reviewer_id: "reviewer".to_owned(),
            proof_reference: "review://proof-1".to_owned(),
            proof_digest: "sha256:proof-1".to_owned(),
            reviewed_at_unix_seconds: 1_700_000_000,
        }
    }

    fn scorecard(provenance: EvidenceProvenance, weekly_windows: usize) -> RoleScorecard {
        let identity = identity();
        let mut windows = Vec::new();
        for index in 0..weekly_windows {
            let start = 1_700_000_000 + index as i64 * SECONDS_PER_WEEK;
            windows.push(TimeWindow {
                window_id: format!("w{index}"),
                start_unix_seconds: start,
                end_unix_seconds: start + SECONDS_PER_WEEK,
            });
        }
        let mut work_units = Vec::new();
        for index in 0..100 {
            let window_index = index * weekly_windows / 100;
            let window_index = window_index.min(weekly_windows.saturating_sub(1));
            let window = &windows[window_index];
            work_units.push(WorkUnitMeasurement {
                work_unit_id: format!("unit-{index}"),
                tenant_id: identity.tenant_id.clone(),
                role_id: identity.role_id.clone(),
                deployment_id: identity.deployment_id.clone(),
                role_version: identity.role_version.clone(),
                runtime_version: identity.runtime_version.clone(),
                window_id: window.window_id.clone(),
                observed_at_unix_seconds: window.start_unix_seconds + 1,
                eligible: true,
                completed: true,
                verified: true,
                human_time: HumanTimeMeasurement {
                    baseline_human_minutes: Some(10),
                    residual_human_minutes: Some(1),
                    expected_approval_minutes: Some(0),
                    expected_exception_minutes: Some(0),
                    unexpected_rescue_minutes: Some(0),
                    support_minutes: Some(0),
                    review_minutes: Some(1),
                },
                cycle_time_before_minutes: None,
                cycle_time_after_minutes: None,
                runtime_cost_micro_usd: Some(100),
                wrong_target_events: Some(0),
                ambiguous_state_events: Some(0),
                unauthorized_side_effects: Some(0),
                unexpected_rescue_count: Some(0),
                exception_count: Some(0),
                escalation_count: Some(0),
                correctly_escalated_exceptions: Some(0),
                incorrectly_handled_exceptions: Some(0),
                sla_met: Some(true),
                safety_regression: Some(false),
                privacy_regression: Some(false),
            });
        }
        let evidence = EvidenceReference {
            reference: "run://scorecard".to_owned(),
            evidence_digest: "sha256:scorecard".to_owned(),
            provenance,
            trust: EvidenceTrust::RuntimeVerified,
            tenant_id: identity.tenant_id.clone(),
            role_id: identity.role_id.clone(),
            deployment_id: identity.deployment_id.clone(),
            role_version: identity.role_version.clone(),
            runtime_version: identity.runtime_version.clone(),
            window_id: None,
            reviewer_proof: Some(proof()),
        };
        RoleScorecard {
            identity,
            provenance,
            coverage: RepresentativeCoverage {
                representative: true,
                population_description: "representative finance queue".to_owned(),
                evidence_reference: evidence.reference.clone(),
            },
            windows,
            work_units,
            window_costs: (0..weekly_windows)
                .map(|index| WindowCostMeasurement {
                    window_id: format!("w{index}"),
                    support_cost_micro_usd: Some(100),
                    deployment_cost_micro_usd: Some(if index == 0 { 100 } else { 0 }),
                })
                .collect(),
            reuse: ReuseMeasurement {
                deployment_hours: Some(8),
                customer_specific_code_percent: Some(10),
                shared_role_logic_percent: Some(90),
            },
            loaded_labor_cost_per_hour_micro_usd: Some(60_000_000),
            evidence: vec![evidence],
        }
    }

    #[test]
    fn production_four_consecutive_windows_can_pass_mature_gate() {
        let report = scorecard(EvidenceProvenance::Production, 4).assess_with_verifier(|_, _| true);
        assert_eq!(report.mature.status, GateStatus::Pass);
        assert_eq!(
            report.recommended_maturity,
            RecommendedMaturity::HardenedDigitalRole
        );
        let metrics = report.metrics.expect("valid scorecard");
        assert_eq!(metrics.verified_work_units, 100);
        assert_eq!(
            metrics
                .cost_per_verified_work_unit
                .unwrap()
                .verified_work_units,
            100
        );
        assert!(metrics.signed_net_loss_micro_usd.unwrap() < 0);
    }

    #[test]
    fn fixture_cannot_pass_live_gates_even_with_good_numbers() {
        let report = scorecard(EvidenceProvenance::Fixture, 4).assess();
        assert_ne!(report.canary.status, GateStatus::Pass);
        assert_ne!(report.mature.status, GateStatus::Pass);
        assert_ne!(
            report.recommended_maturity,
            RecommendedMaturity::HardenedDigitalRole
        );
        assert_eq!(report.recommended_maturity, RecommendedMaturity::Assisted);
    }

    #[test]
    fn production_label_without_live_verifier_is_unknown() {
        let report = scorecard(EvidenceProvenance::Production, 4).assess();
        assert_eq!(report.mature.status, GateStatus::Unknown);
        assert!(report
            .mature
            .reasons
            .iter()
            .any(|reason| reason.code == GateReasonCode::MissingReviewerProof));
    }

    #[test]
    fn extra_human_minutes_are_signed_and_fail_mature_gate() {
        let mut scorecard = scorecard(EvidenceProvenance::Production, 4);
        for unit in &mut scorecard.work_units {
            unit.human_time.residual_human_minutes = Some(15);
            unit.human_time.expected_approval_minutes = Some(0);
            unit.human_time.expected_exception_minutes = Some(0);
            unit.human_time.unexpected_rescue_minutes = Some(0);
            unit.human_time.support_minutes = Some(0);
            unit.human_time.review_minutes = Some(15);
        }
        let report = scorecard.assess_with_verifier(|_, _| true);
        let metrics = report.metrics.expect("valid scorecard");
        assert_eq!(metrics.displaced_human_minutes, Some(-500));
        assert!(metrics.displaced_labor_value_micro_usd.unwrap() < 0);
        assert!(metrics.signed_net_loss_micro_usd.unwrap() > 0);
        assert!(metrics
            .recurring_net_value_created_micro_usd
            .is_some_and(|value| value < 0));
        assert!(metrics.payback_hypothesis_observed_windows.is_none());
        assert_eq!(report.mature.status, GateStatus::Fail);
        assert_eq!(
            report.recommended_maturity,
            RecommendedMaturity::AutonomousRoutine
        );
    }

    #[test]
    fn duplicate_units_and_spoofed_evidence_are_rejected() {
        let mut scorecard = scorecard(EvidenceProvenance::Production, 4);
        scorecard.work_units.push(scorecard.work_units[0].clone());
        scorecard.evidence[0].tenant_id = "other-tenant".to_owned();
        let errors = scorecard.validate().expect_err("invalid scorecard");
        assert!(errors
            .iter()
            .any(|error| error.code == ScorecardErrorCode::DuplicateWorkUnit));
        assert!(errors
            .iter()
            .any(|error| error.code == ScorecardErrorCode::MixedTenant));
    }

    #[test]
    fn gapped_and_overlapping_windows_are_rejected() {
        let mut gapped = scorecard(EvidenceProvenance::Production, 4);
        gapped.windows[1].start_unix_seconds += 1;
        let errors = gapped.validate().expect_err("gap must be rejected");
        assert!(errors
            .iter()
            .any(|error| error.code == ScorecardErrorCode::GappedWindows));

        let mut overlapping = scorecard(EvidenceProvenance::Production, 4);
        overlapping.windows[1].start_unix_seconds -= 1;
        let errors = overlapping
            .validate()
            .expect_err("overlap must be rejected");
        assert!(errors
            .iter()
            .any(|error| error.code == ScorecardErrorCode::OverlappingWindows));
    }

    #[test]
    fn missing_cost_is_unknown_and_empty_cohort_never_passes() {
        let mut missing_cost = scorecard(EvidenceProvenance::Production, 4);
        missing_cost.window_costs[0].support_cost_micro_usd = None;
        let report = missing_cost.assess_with_verifier(|_, _| true);
        assert_eq!(report.mature.status, GateStatus::Unknown);
        assert!(report
            .mature
            .reasons
            .iter()
            .any(|reason| reason.code == GateReasonCode::MissingCost));

        let mut empty = scorecard(EvidenceProvenance::Production, 4);
        empty.work_units.clear();
        let report = empty.assess();
        assert!(!report.valid);
        assert_ne!(report.mature.status, GateStatus::Pass);
    }

    #[test]
    fn out_of_window_and_short_window_evidence_cannot_pass_mature() {
        let mut out_of_window_scorecard = scorecard(EvidenceProvenance::Production, 4);
        out_of_window_scorecard.work_units[0].observed_at_unix_seconds =
            out_of_window_scorecard.windows[0].end_unix_seconds;
        let report = out_of_window_scorecard.assess();
        assert!(!report.valid);
        assert!(report
            .validation_errors
            .iter()
            .any(|error| error.code == ScorecardErrorCode::OutOfWindow));

        let short = scorecard(EvidenceProvenance::Production, 3).assess_with_verifier(|_, _| true);
        assert_eq!(short.mature.status, GateStatus::Fail);
        assert!(short
            .mature
            .reasons
            .iter()
            .any(|reason| reason.code == GateReasonCode::InsufficientConsecutiveWeeklyWindows));
    }

    #[test]
    fn cost_per_verified_unit_includes_failed_attempts_and_denominator_is_exact() {
        let mut scorecard = scorecard(EvidenceProvenance::Production, 4);
        scorecard.work_units[99].verified = false;
        scorecard.work_units[99].completed = false;
        scorecard.work_units[99].runtime_cost_micro_usd = Some(9_999);
        let report = scorecard.assess_with_verifier(|_, _| true);
        let metrics = report.metrics.expect("valid scorecard");
        let cost = metrics
            .cost_per_verified_work_unit
            .expect("99 verified units");
        assert_eq!(cost.verified_work_units, 99);
        assert_eq!(cost.attempted_cost_micro_usd, 20_399);
    }

    #[test]
    fn expected_approvals_do_not_count_as_rescue() {
        let mut scorecard = scorecard(EvidenceProvenance::Production, 4);
        for unit in &mut scorecard.work_units {
            unit.human_time.expected_approval_minutes = Some(1);
            unit.human_time.review_minutes = Some(0);
            unit.human_time.residual_human_minutes = Some(1);
        }
        let report = scorecard.assess_with_verifier(|_, _| true);
        assert_eq!(report.mature.status, GateStatus::Pass);
        assert_eq!(report.metrics.unwrap().unexpected_rescue_events, Some(0));
    }

    #[test]
    fn canary_rescue_boundary_is_strictly_below_five_percent() {
        let mut scorecard = scorecard(EvidenceProvenance::Staging, 1);
        for unit in scorecard.work_units.iter_mut().take(5) {
            unit.human_time.expected_exception_minutes = Some(0);
            unit.human_time.unexpected_rescue_minutes = Some(1);
            unit.human_time.review_minutes = Some(0);
            unit.human_time.residual_human_minutes = Some(1);
            unit.unexpected_rescue_count = Some(1);
        }
        let report = scorecard.assess_with_verifier(|_, _| true);
        assert_eq!(report.canary.status, GateStatus::Fail);
        assert!(report
            .canary
            .reasons
            .iter()
            .any(|reason| reason.code == GateReasonCode::RescueRateTooHigh));
    }

    #[test]
    fn canary_requires_baseline_residual_and_complete_costs() {
        let mut scorecard = scorecard(EvidenceProvenance::Staging, 1);
        for unit in &mut scorecard.work_units {
            unit.human_time.baseline_human_minutes = None;
            unit.human_time.residual_human_minutes = None;
            unit.runtime_cost_micro_usd = None;
        }
        scorecard.window_costs[0].support_cost_micro_usd = None;
        scorecard.window_costs[0].deployment_cost_micro_usd = None;
        let report = scorecard.assess_with_verifier(|_, _| true);
        assert_eq!(report.canary.status, GateStatus::Unknown);
        assert!(report
            .canary
            .reasons
            .iter()
            .any(|reason| reason.code == GateReasonCode::MissingHumanTime));
        assert!(report
            .canary
            .reasons
            .iter()
            .any(|reason| reason.code == GateReasonCode::MissingCost));
    }

    #[test]
    fn ineligible_rescue_does_not_distort_eligible_rescue_rate_or_costs() {
        let mut scorecard = scorecard(EvidenceProvenance::Staging, 1);
        let mut ineligible = scorecard.work_units[0].clone();
        ineligible.work_unit_id = "ineligible-rescued".to_owned();
        ineligible.eligible = false;
        ineligible.completed = false;
        ineligible.verified = false;
        ineligible.human_time = HumanTimeMeasurement {
            baseline_human_minutes: None,
            residual_human_minutes: Some(4),
            expected_approval_minutes: None,
            expected_exception_minutes: None,
            unexpected_rescue_minutes: None,
            support_minutes: Some(3),
            review_minutes: None,
        };
        ineligible.runtime_cost_micro_usd = Some(99_999);
        ineligible.unexpected_rescue_count = Some(1);
        scorecard.work_units.push(ineligible);
        let report = scorecard.assess_with_verifier(|_, _| true);
        assert_eq!(report.canary.status, GateStatus::Pass);
        let metrics = report.metrics.expect("valid scorecard");
        assert_eq!(metrics.eligible_work_units_with_unexpected_rescue, Some(0));
        assert_eq!(metrics.unexpected_rescue_events, Some(1));
        assert_eq!(metrics.total_attempted_cost_micro_usd, Some(110_199));
        assert_eq!(metrics.total_human_attention_minutes, Some(107));
        assert_eq!(
            metrics.human_attention_minutes_per_100_received,
            Some(ExactRatio {
                numerator: 10_700,
                denominator: 101
            })
        );
    }

    #[test]
    fn cycle_time_coverage_exception_rate_and_payback_are_observed_metrics() {
        let mut scorecard = scorecard(EvidenceProvenance::Production, 4);
        for (index, unit) in scorecard.work_units.iter_mut().enumerate() {
            unit.cycle_time_before_minutes = Some(10);
            unit.cycle_time_after_minutes = Some(3);
            if index < 10 {
                unit.exception_count = Some(1);
                unit.escalation_count = Some(1);
                unit.correctly_escalated_exceptions = Some(1);
                unit.incorrectly_handled_exceptions = Some(0);
                unit.human_time.expected_exception_minutes = Some(1);
                unit.human_time.review_minutes = Some(0);
            }
        }
        let report = scorecard.assess_with_verifier(|_, _| true);
        let metrics = report.metrics.expect("valid scorecard");
        assert_eq!(
            metrics.routine_coverage_rate,
            Some(ExactRatio {
                numerator: 100,
                denominator: 100
            })
        );
        assert_eq!(
            metrics.routine_exception_unit_rate,
            Some(ExactRatio {
                numerator: 10,
                denominator: 100
            })
        );
        assert_eq!(
            metrics.cycle_time_before_minutes_per_unit,
            Some(ExactRatio {
                numerator: 1_000,
                denominator: 100
            })
        );
        assert_eq!(
            metrics.cycle_time_after_minutes_per_unit,
            Some(ExactRatio {
                numerator: 300,
                denominator: 100
            })
        );
        assert_eq!(
            metrics.cycle_time_reduction_rate,
            Some(SignedRatio {
                numerator: 700,
                denominator: 1_000
            })
        );
        assert_eq!(
            metrics.signed_net_value_created_micro_usd,
            Some(-metrics.signed_net_loss_micro_usd.unwrap())
        );
        assert!(metrics
            .recurring_net_value_created_micro_usd
            .is_some_and(|value| value > 0));
        assert!(metrics.payback_hypothesis_observed_windows.is_some());
    }

    #[test]
    fn empty_weekly_windows_do_not_count_toward_mature_streak() {
        let mut scorecard = scorecard(EvidenceProvenance::Production, 4);
        let first_window_start = scorecard.windows[0].start_unix_seconds;
        for unit in &mut scorecard.work_units {
            unit.window_id = "w0".to_owned();
            unit.observed_at_unix_seconds = first_window_start + 1;
        }
        let report = scorecard.assess_with_verifier(|_, _| true);
        let metrics = report.metrics.expect("valid scorecard");
        assert_eq!(metrics.exact_weekly_window_count, 1);
        assert_eq!(report.mature.status, GateStatus::Fail);
        assert!(report
            .mature
            .reasons
            .iter()
            .any(|reason| reason.code == GateReasonCode::InsufficientConsecutiveWeeklyWindows));
    }

    #[test]
    fn exact_ninety_nine_and_eighty_percent_boundaries_pass() {
        let mut scorecard = scorecard(EvidenceProvenance::Production, 4);
        scorecard.work_units[99].verified = false;
        scorecard.work_units[99].completed = false;
        for (index, unit) in scorecard.work_units.iter_mut().enumerate() {
            unit.human_time.residual_human_minutes = Some(2);
            unit.human_time.expected_approval_minutes = Some(0);
            unit.human_time.expected_exception_minutes = Some(0);
            unit.human_time.unexpected_rescue_minutes = Some(if index < 20 { 1 } else { 0 });
            unit.human_time.review_minutes = Some(if index < 20 { 1 } else { 2 });
            unit.unexpected_rescue_count = Some(if index < 20 { 1 } else { 0 });
        }
        let report = scorecard.assess_with_verifier(|_, _| true);
        assert_eq!(report.mature.status, GateStatus::Pass);
        let metrics = report.metrics.expect("valid scorecard");
        assert_eq!(
            metrics.verified_completion_rate.unwrap(),
            ExactRatio {
                numerator: 99,
                denominator: 100
            }
        );
        assert_eq!(
            metrics.minutes_removed_rate.unwrap(),
            SignedRatio {
                numerator: 800,
                denominator: 1000
            }
        );
        assert_eq!(
            metrics.rescue_free_eligible_rate.unwrap(),
            ExactRatio {
                numerator: 80,
                denominator: 100
            }
        );
    }

    #[test]
    fn imported_json_cannot_promote_runtime_verified_trust() {
        let parsed = serde_json::from_str::<EvidenceTrust>("\"runtime_verified\"");
        assert!(parsed.is_err());
    }
}
