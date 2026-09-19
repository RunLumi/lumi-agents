//! Evidence records: minimal, privacy-aware proof data (spec 11 §11.4–11.5).
//!
//! Evidence prefers structured state over screenshots (the enum order
//! mirrors the preference ranking). Evidence is tenant-scoped, redacted on
//! append, and supports retention-based purging: purging replaces the
//! payload with a tombstone while the audit events that referenced it stay
//! intact.

use lumi_protocol::{
    redact_value, EvidenceId, EvidenceRequirement, RedactionRule, SensitivityLabel, TenantId,
    Timestamp,
};
use serde::{Deserialize, Serialize};

/// A stored evidence record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub evidence_id: EvidenceId,
    pub tenant_id: TenantId,
    pub kind: EvidenceKind,
    pub sensitivity: SensitivityLabel,
    /// Redacted, structured payload (JSON object).
    pub payload: serde_json::Value,
    /// Rules applied before storage (recorded for reproducibility).
    pub redaction_rules: Vec<RedactionRule>,
    pub created_at: Timestamp,
    /// When retention requires the payload to be purged.
    pub retention_until: Option<Timestamp>,
    /// Set when the payload has been purged; the record remains as a
    /// tombstone so audit references stay resolvable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purged_at: Option<Timestamp>,
}

/// Evidence kind, ordered by preference (spec 11 §11.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceKind {
    StructuredState,
    ResourceReference,
    Checksum,
    Diff,
    LogExcerpt,
    SelectiveScreenshot,
    FullScreenshot,
}

impl EvidenceKind {
    #[must_use]
    pub const fn from_requirement(requirement: EvidenceRequirement) -> Self {
        match requirement {
            EvidenceRequirement::StructuredState => Self::StructuredState,
            EvidenceRequirement::ResourceReference => Self::ResourceReference,
            EvidenceRequirement::Checksum => Self::Checksum,
            EvidenceRequirement::Diff => Self::Diff,
            EvidenceRequirement::LogExcerpt => Self::LogExcerpt,
            EvidenceRequirement::SelectiveScreenshot => Self::SelectiveScreenshot,
            EvidenceRequirement::FullScreenshot => Self::FullScreenshot,
        }
    }

    /// Screenshots are surveillance-adjacent; they are permitted only when
    /// the action explicitly required them (spec 11 §11.5: continuous
    /// recording is never default evidence).
    #[must_use]
    pub const fn requires_explicit_requirement(self) -> bool {
        matches!(self, Self::SelectiveScreenshot | Self::FullScreenshot)
    }
}

/// Everything needed to append one evidence record.
#[derive(Debug, Clone)]
pub struct EvidenceRequest {
    pub tenant_id: TenantId,
    pub kind: EvidenceKind,
    pub sensitivity: SensitivityLabel,
    pub payload: serde_json::Value,
    pub redaction_rules: Vec<RedactionRule>,
    pub created_at: Timestamp,
    pub retention_until: Option<Timestamp>,
    /// The action's declared evidence requirements, which gate screenshot
    /// evidence.
    pub action_requirements: Vec<EvidenceRequirement>,
}

/// Errors surfaced by the evidence store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceError {
    /// Screenshot evidence was not explicitly required by the action.
    ScreenshotNotRequired,
    /// Evidence belongs to another tenant.
    CrossTenant,
    AlreadyPurged,
}

/// Tenant-scoped evidence store.
#[derive(Debug, Default)]
pub struct EvidenceStore {
    records: Vec<EvidenceRecord>,
}

impl EvidenceStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends evidence after enforcing screenshot policy and applying
    /// redaction. The payload passed in is NEVER stored as-is; it is
    /// redacted with `request.redaction_rules` first (defense in depth:
    /// callers should not rely on this and must not hand raw secrets to
    /// evidence anyway).
    pub fn record(&mut self, request: EvidenceRequest) -> Result<EvidenceId, EvidenceError> {
        if request.kind.requires_explicit_requirement()
            && !request
                .action_requirements
                .iter()
                .any(|r| EvidenceKind::from_requirement(*r) == request.kind)
        {
            return Err(EvidenceError::ScreenshotNotRequired);
        }
        let evidence_id = EvidenceId::generate();
        let payload = redact_value(&request.payload, &request.redaction_rules);
        self.records.push(EvidenceRecord {
            evidence_id: evidence_id.clone(),
            tenant_id: request.tenant_id,
            kind: request.kind,
            sensitivity: request.sensitivity,
            payload,
            redaction_rules: request.redaction_rules,
            created_at: request.created_at,
            retention_until: request.retention_until,
            purged_at: None,
        });
        Ok(evidence_id)
    }

    /// Tenant-scoped read.
    #[must_use]
    pub fn get(&self, tenant_id: &TenantId, evidence_id: &EvidenceId) -> Option<&EvidenceRecord> {
        self.records
            .iter()
            .find(|r| &r.tenant_id == tenant_id && &r.evidence_id == evidence_id)
    }

    /// Purges payloads whose retention window has elapsed. Returns the
    /// count purged. Tombstones remain (payload replaced by a marker) so
    /// audit references stay structurally valid without retaining data.
    pub fn purge_expired(&mut self, now: Timestamp) -> usize {
        let mut purged = 0;
        for record in &mut self.records {
            let due = record.retention_until.is_some_and(|until| now > until);
            if due && record.purged_at.is_none() {
                record.payload = serde_json::json!({
                    "purged": true,
                    "reason": "retention_elapsed",
                });
                record.purged_at = Some(now);
                purged += 1;
            }
        }
        purged
    }

    /// Number of records held (including tombstones).
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// True when the store holds no records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ts(secs: i64) -> Timestamp {
        Timestamp::from_epoch(secs, 0).unwrap()
    }

    #[test]
    fn evidence_is_redacted_on_append() {
        let mut store = EvidenceStore::new();
        let id = store
            .record(EvidenceRequest {
                tenant_id: TenantId::parse("t-1").unwrap(),
                kind: EvidenceKind::StructuredState,
                sensitivity: SensitivityLabel::Confidential,
                payload: json!({"record_id": "q-1", "total": "42.00", "api_key": "sk-xyz"}),
                redaction_rules: vec![RedactionRule::key("api_key")],
                created_at: ts(10),
                retention_until: Some(ts(1_000)),
                action_requirements: vec![],
            })
            .unwrap();
        let record = store.get(&TenantId::parse("t-1").unwrap(), &id).unwrap();
        assert_eq!(record.payload["api_key"], lumi_protocol::REDACTED_MARKER);
        assert_eq!(record.payload["record_id"], "q-1");
    }

    #[test]
    fn cross_tenant_reads_find_nothing() {
        let mut store = EvidenceStore::new();
        let id = store
            .record(EvidenceRequest {
                tenant_id: TenantId::parse("t-1").unwrap(),
                kind: EvidenceKind::Checksum,
                sensitivity: SensitivityLabel::Internal,
                payload: json!({"sha256": "abc"}),
                redaction_rules: vec![],
                created_at: ts(0),
                retention_until: None,
                action_requirements: vec![],
            })
            .unwrap();
        assert!(
            store.get(&TenantId::parse("t-2").unwrap(), &id).is_none(),
            "tenant 2 must not see tenant 1 evidence"
        );
    }

    #[test]
    fn screenshots_require_explicit_action_requirement() {
        let mut store = EvidenceStore::new();
        let err = store.record(EvidenceRequest {
            tenant_id: TenantId::parse("t-1").unwrap(),
            kind: EvidenceKind::FullScreenshot,
            sensitivity: SensitivityLabel::Internal,
            payload: json!({"png": "..."}),
            redaction_rules: vec![],
            created_at: ts(0),
            retention_until: None,
            action_requirements: vec![EvidenceRequirement::StructuredState],
        });
        assert_eq!(err.unwrap_err(), EvidenceError::ScreenshotNotRequired);

        let ok = store.record(EvidenceRequest {
            tenant_id: TenantId::parse("t-1").unwrap(),
            kind: EvidenceKind::SelectiveScreenshot,
            sensitivity: SensitivityLabel::Internal,
            payload: json!({"png": "cropped"}),
            redaction_rules: vec![],
            created_at: ts(0),
            retention_until: None,
            action_requirements: vec![EvidenceRequirement::SelectiveScreenshot],
        });
        assert!(ok.is_ok());
    }

    #[test]
    fn retention_purge_tombstones_payload() {
        let mut store = EvidenceStore::new();
        let id = store
            .record(EvidenceRequest {
                tenant_id: TenantId::parse("t-1").unwrap(),
                kind: EvidenceKind::StructuredState,
                sensitivity: SensitivityLabel::PersonalData,
                payload: json!({"email": "person@example.test"}),
                redaction_rules: vec![],
                created_at: ts(0),
                retention_until: Some(ts(100)),
                action_requirements: vec![],
            })
            .unwrap();
        assert_eq!(store.purge_expired(ts(50)), 0);
        assert_eq!(store.purge_expired(ts(101)), 1);
        let record = store.get(&TenantId::parse("t-1").unwrap(), &id).unwrap();
        assert!(record.purged_at.is_some());
        assert_eq!(record.payload["purged"], true);
        assert!(record.payload.get("email").is_none());
        // Second purge is a no-op.
        assert_eq!(store.purge_expired(ts(200)), 0);
    }
}
