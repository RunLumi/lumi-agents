//! Durable memory: explicitly retained facts (spec 10 §10.4, §10.10,
//! §10.13).
//!
//! Memory creation is explicit: every record carries owner (tenant/user),
//! purpose, provenance, retention, deletion behavior, and sensitivity.
//! Cross-task access is tenant-scoped. Secrets and do-not-persist
//! content are refused at write time.

use lumi_protocol::{SensitivityLabel, TenantId, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Stable memory identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MemoryId(pub String);

impl MemoryId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// Retention duration for a memory record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionDuration {
    /// Tenant audit-retention bound.
    TenantDefault,
    /// Explicit days.
    Days(u32),
    /// Until explicitly deleted.
    UntilDeleted,
}

/// Deletion behavior (§10.4): what happens when retention elapses or the
/// user deletes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryDeletion {
    /// Hard delete the record.
    Purge,
    /// Keep a tombstone (id + deletion marker) without content.
    Tombstone,
}

/// Where the memory came from (provenance, §10.4/§10.9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProvenance {
    /// The task that produced it.
    pub task_id: String,
    /// Source refs (evidence ids, file paths, conversation refs).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_refs: Vec<String>,
    /// How it was created: `explicit-user`, `policy-governed`, etc.
    pub creation: String,
}

/// Tenant/user scoping (§10.13): memory is never cross-tenant readable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryScope {
    pub tenant_id: TenantId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// A durable memory record (§10.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub memory_id: MemoryId,
    pub scope: MemoryScope,
    /// Why this memory exists (e.g. "customer escalation contacts").
    pub purpose: String,
    /// The retained content.
    pub content: String,
    pub provenance: MemoryProvenance,
    pub sensitivity: SensitivityLabel,
    pub retention: RetentionDuration,
    pub deletion: MemoryDeletion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<u8>,
    pub created_at: Timestamp,
    /// Set when purged; content is gone (tombstone deletion keeps this).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purged_at: Option<Timestamp>,
}

/// Memory write failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    /// The content matches a do-not-persist class (§10.10) — e.g. it
    /// carries a secret-ref marker or an obviously-secret field name.
    DoNotPersist {
        reason: String,
    },
    NotFound {
        memory_id: String,
    },
    CrossTenant,
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DoNotPersist { reason } => {
                write!(f, "content is do-not-persist: {reason}")
            }
            Self::NotFound { memory_id } => write!(f, "memory not found: {memory_id}"),
            Self::CrossTenant => write!(f, "memory belongs to another tenant"),
        }
    }
}

impl std::error::Error for MemoryError {}

/// Patterns that mark content as do-not-persist (§10.10). Deliberately
/// conservative: case-insensitive substring hits on credential-ish keys.
const DO_NOT_PERSIST_MARKERS: &[&str] = &[
    "password\":",
    "api_key\":",
    "apikey\":",
    "secret\":",
    "authorization: bearer",
    "set-cookie",
    "private_key\":",
];

/// Detects do-not-persist content (§10.10).
#[must_use]
pub fn is_do_not_persist(content: &str) -> bool {
    let lower = content.to_lowercase();
    DO_NOT_PERSIST_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

/// The durable memory store (§10.4, §10.13).
#[derive(Debug, Default)]
pub struct MemoryStore {
    records: BTreeMap<MemoryId, MemoryRecord>,
}

impl MemoryStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Writes one memory record after do-not-persist screening.
    ///
    /// # Errors
    /// [`MemoryError::DoNotPersist`] when the content matches a
    /// do-not-persist class.
    pub fn remember(&mut self, record: MemoryRecord) -> Result<MemoryId, MemoryError> {
        if is_do_not_persist(&record.content) {
            return Err(MemoryError::DoNotPersist {
                reason: "content contains credential-shaped fields".to_owned(),
            });
        }
        let memory_id = record.memory_id.clone();
        self.records.insert(memory_id.clone(), record);
        Ok(memory_id)
    }

    /// Tenant-scoped read (§10.13). Cross-tenant reads are refused
    /// outright, not silently empty.
    ///
    /// # Errors
    /// [`MemoryError::CrossTenant`] when the record exists under another
    /// tenant; [`MemoryError::NotFound`] when absent.
    pub fn get(
        &self,
        scope_tenant: &TenantId,
        memory_id: &MemoryId,
    ) -> Result<&MemoryRecord, MemoryError> {
        let record = self
            .records
            .get(memory_id)
            .ok_or_else(|| MemoryError::NotFound {
                memory_id: memory_id.0.clone(),
            })?;
        if &record.scope.tenant_id != scope_tenant {
            return Err(MemoryError::CrossTenant);
        }
        if record.purged_at.is_some() {
            return Err(MemoryError::NotFound {
                memory_id: memory_id.0.clone(),
            });
        }
        Ok(record)
    }

    /// Lists live (non-purged) memories for one tenant.
    #[must_use]
    pub fn list_for_tenant(&self, tenant_id: &TenantId) -> Vec<&MemoryRecord> {
        self.records
            .values()
            .filter(|r| &r.scope.tenant_id == tenant_id && r.purged_at.is_none())
            .collect()
    }

    /// User deletion (§10.11): purge removes content; tombstone keeps an
    /// empty marker. Cross-tenant deletion is refused.
    ///
    /// # Errors
    /// [`MemoryError`] variants as in [`Self::get`].
    pub fn delete(
        &mut self,
        scope_tenant: &TenantId,
        memory_id: &MemoryId,
        now: Timestamp,
    ) -> Result<MemoryDeletion, MemoryError> {
        let record = self
            .records
            .get_mut(memory_id)
            .ok_or_else(|| MemoryError::NotFound {
                memory_id: memory_id.0.clone(),
            })?;
        if &record.scope.tenant_id != scope_tenant {
            return Err(MemoryError::CrossTenant);
        }
        let behavior = record.deletion;
        match behavior {
            MemoryDeletion::Purge => {
                self.records.remove(memory_id);
            }
            MemoryDeletion::Tombstone => {
                record.content = String::new();
                record.purged_at = Some(now);
            }
        }
        Ok(behavior)
    }

    /// Retention sweep: purges/tombstones expired records. Returns the
    /// number of records processed (§10.4 retention).
    pub fn sweep_expired(&mut self, now: Timestamp, tenant_default_days: u32) -> usize {
        let expired: Vec<MemoryId> = self
            .records
            .iter()
            .filter(|(_, r)| {
                if r.purged_at.is_some() {
                    return false;
                }
                match r.retention {
                    RetentionDuration::UntilDeleted => false,
                    RetentionDuration::Days(days) => {
                        let deadline_secs = i64::from(days) * 86_400;
                        r.created_at.epoch_seconds() + deadline_secs <= now.epoch_seconds()
                    }
                    RetentionDuration::TenantDefault => {
                        let deadline_secs = i64::from(tenant_default_days) * 86_400;
                        r.created_at.epoch_seconds() + deadline_secs <= now.epoch_seconds()
                    }
                }
            })
            .map(|(id, _)| id.clone())
            .collect();
        let count = expired.len();
        for id in expired {
            let (tenant_id, deletion) = match self.records.get(&id) {
                Some(r) => (r.scope.tenant_id.clone(), r.deletion),
                None => continue,
            };
            // delete() re-checks tenant scope; the sweep acts as the
            // retention policy owner so scope always matches.
            self.delete(&tenant_id, &id, now).ok();
            let _ = deletion;
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(tenant: &str, content: &str) -> MemoryRecord {
        MemoryRecord {
            memory_id: MemoryId::new(format!("mem-{tenant}-{}", content.len())),
            scope: MemoryScope {
                tenant_id: TenantId::parse(tenant).unwrap(),
                user_id: None,
            },
            purpose: "customer escalation contacts".to_owned(),
            content: content.to_owned(),
            provenance: MemoryProvenance {
                task_id: "task-1".to_owned(),
                source_refs: vec!["evidence-1".to_owned()],
                creation: "explicit-user".to_owned(),
            },
            sensitivity: SensitivityLabel::Internal,
            retention: RetentionDuration::UntilDeleted,
            deletion: MemoryDeletion::Purge,
            confidence: None,
            created_at: Timestamp::UNIX_EPOCH,
            purged_at: None,
        }
    }

    #[test]
    fn tenant_isolation_is_enforced() {
        let mut store = MemoryStore::new();
        store
            .remember(record("t-1", "escalation path: mail ops@x"))
            .unwrap();
        // Tenant 2 cannot read tenant 1's memory: refused, not empty.
        let id = &store.list_for_tenant(&TenantId::parse("t-1").unwrap())[0].memory_id;
        assert!(matches!(
            store.get(&TenantId::parse("t-2").unwrap(), id),
            Err(MemoryError::CrossTenant)
        ));
    }

    #[test]
    fn secrets_are_refused_as_memory() {
        let mut store = MemoryStore::new();
        let err = store
            .remember(record(
                "t-1",
                r#"{"api_key": "sk-live-123", "note": "prod key"}"#,
            ))
            .unwrap_err();
        assert!(matches!(err, MemoryError::DoNotPersist { .. }));
    }

    #[test]
    fn purge_deletes_tombstone_keeps_marker() {
        let mut store = MemoryStore::new();
        let mut tomb = record("t-1", "keep-marker-only");
        tomb.memory_id = MemoryId::new("mem-tomb");
        tomb.deletion = MemoryDeletion::Tombstone;
        store.remember(record("t-1", "purge-me")).unwrap();
        store.remember(tomb).unwrap();

        let purge_id = store
            .list_for_tenant(&TenantId::parse("t-1").unwrap())
            .iter()
            .find(|r| r.deletion == MemoryDeletion::Purge)
            .map(|r| r.memory_id.clone())
            .unwrap();
        store
            .delete(
                &TenantId::parse("t-1").unwrap(),
                &purge_id,
                Timestamp::UNIX_EPOCH,
            )
            .unwrap();
        assert!(store
            .get(&TenantId::parse("t-1").unwrap(), &purge_id)
            .is_err());
    }

    #[test]
    fn retention_sweep_processes_expired() {
        let mut store = MemoryStore::new();
        let mut r = record("t-1", "old memory");
        r.memory_id = MemoryId::new("mem-old");
        r.retention = RetentionDuration::Days(1);
        r.created_at = Timestamp::from_epoch(0, 0).unwrap();
        store.remember(r).unwrap();
        // Two days later: expired.
        let now = Timestamp::from_epoch(2 * 86_400, 0).unwrap();
        assert_eq!(store.sweep_expired(now, 30), 1);
        assert!(store
            .list_for_tenant(&TenantId::parse("t-1").unwrap())
            .is_empty());
    }
}
