//! Append-only audit ledger with tenant-scoped access (spec 11 §11.3,
//! §11.5, §11.13).
//!
//! Events can only be appended; the chain links each event to its
//! predecessor so removal or reordering is detectable. All queries are
//! tenant-scoped: there is no API to read another tenant's events.

use crate::event::AuditEvent;
use lumi_protocol::TenantId;
use std::collections::HashMap;

/// Result of verifying the whole chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainVerification {
    Intact { events: usize },
    Broken { at_event_id: String, reason: String },
}

/// In-memory append-only audit ledger.
///
/// The v1 runtime persists ledgers via [`JsonlAuditLog`]; this store is
/// the reference implementation and the unit-test surface.
#[derive(Debug, Default)]
pub struct AuditLedger {
    events: Vec<AuditEvent>,
    /// tenant -> indices into `events`, kept so tenant-scoped reads never
    /// touch other tenants' records.
    by_tenant: HashMap<TenantId, Vec<usize>>,
}

impl AuditLedger {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an event if its chain link is valid.
    ///
    /// # Errors
    /// Returns the reason when the event's hash is invalid or its
    /// `prev_event_hash` does not match the current chain head.
    pub fn append(&mut self, event: AuditEvent) -> Result<(), String> {
        if !event.hash_is_valid() {
            return Err("event hash does not match content".to_owned());
        }
        let expected_prev = self
            .events
            .last()
            .map(|e| e.event_hash.as_str())
            .unwrap_or("genesis");
        if event.prev_event_hash != expected_prev {
            return Err(format!(
                "chain link mismatch: expected prev {expected_prev}, got {}",
                event.prev_event_hash
            ));
        }
        let tenant = event.tenant_id.clone();
        let index = self.events.len();
        self.events.push(event);
        self.by_tenant.entry(tenant).or_default().push(index);
        Ok(())
    }

    /// Tenant-scoped events, oldest first. Other tenants' events are
    /// structurally unreachable.
    #[must_use]
    pub fn events_for(&self, tenant_id: &TenantId) -> Vec<&AuditEvent> {
        self.by_tenant
            .get(tenant_id)
            .map(|indices| indices.iter().map(|&i| &self.events[i]).collect())
            .unwrap_or_default()
    }

    /// Current chain head hash (`"genesis"` when empty).
    #[must_use]
    pub fn head_hash(&self) -> &str {
        self.events
            .last()
            .map(|e| e.event_hash.as_str())
            .unwrap_or("genesis")
    }

    /// Verifies the full chain integrity.
    #[must_use]
    pub fn verify_chain(&self) -> ChainVerification {
        let mut prev = "genesis".to_owned();
        for event in &self.events {
            if event.prev_event_hash != prev {
                return ChainVerification::Broken {
                    at_event_id: event.event_id.clone(),
                    reason: "prev_event_hash does not match predecessor".to_owned(),
                };
            }
            if !event.hash_is_valid() {
                return ChainVerification::Broken {
                    at_event_id: event.event_id.clone(),
                    reason: "event hash does not match content".to_owned(),
                };
            }
            prev = event.event_hash.clone();
        }
        ChainVerification::Intact {
            events: self.events.len(),
        }
    }
}

/// Append-only JSONL persistence (one JSON event per line).
///
/// Loading verifies the envelope, per-event hash, and chain links; a
/// corrupted or reordered file fails loudly instead of silently
/// continuing from tampered state.
pub struct JsonlAuditLog {
    path: std::path::PathBuf,
}

impl JsonlAuditLog {
    #[must_use]
    pub const fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }

    /// Appends one event as a JSON line (with newline).
    ///
    /// # Errors
    /// Propagates I/O errors.
    pub fn append(&self, event: &AuditEvent) -> std::io::Result<()> {
        use std::io::Write;
        let line = event
            .to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        Ok(())
    }

    /// Loads and verifies the full chain into an in-memory ledger.
    ///
    /// # Errors
    /// Fails closed on I/O errors, malformed lines, hash mismatch, or
    /// broken chain links.
    pub fn load(&self) -> Result<AuditLedger, String> {
        use std::io::BufRead;
        let file =
            std::fs::File::open(&self.path).map_err(|e| format!("opening audit log: {e}"))?;
        let mut ledger = AuditLedger::new();
        for (line_no, line) in std::io::BufReader::new(file).lines().enumerate() {
            let line = line.map_err(|e| format!("reading line {}: {e}", line_no + 1))?;
            if line.trim().is_empty() {
                continue;
            }
            let event =
                AuditEvent::from_json(&line).map_err(|e| format!("line {}: {e}", line_no + 1))?;
            ledger
                .append(event)
                .map_err(|e| format!("line {}: {e}", line_no + 1))?;
        }
        Ok(ledger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::tests::fixture_principal;
    use crate::event::{ActionEventDetails, AuditEventKind, AuditScope, PolicyOutcome};
    use crate::verify::VerificationStatus;
    use lumi_protocol::{ActionId, Capability, ResourceRef, ResourceType, RiskClass, Target};

    fn event(prev: &str, digest: &str) -> AuditEvent {
        AuditEvent::new(
            AuditScope {
                tenant_id: TenantId::parse("t-1").unwrap(),
                task_id: lumi_protocol::TaskId::parse("task-1").unwrap(),
                run_id: lumi_protocol::RunId::parse("run-1").unwrap(),
                workflow: None,
            },
            AuditEventKind::Action(Box::new(ActionEventDetails {
                action_id: ActionId::parse("a-1").unwrap(),
                step_id: None,
                principal: fixture_principal("t-1"),
                capability: Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND).0,
                operation: "send_customer_email".to_owned(),
                resource: ResourceRef {
                    resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                    id: "outbound/x".to_owned(),
                    sensitivity: None,
                },
                target: Target::canonical("mailto:c@example.test"),
                risk_class: RiskClass::Communication,
                execution_tier: None,
                adapter: None,
                adapter_version: None,
                policy: PolicyOutcome::Allowed {
                    rule_id: "r".to_owned(),
                },
                approval_id: None,
                action_digest: digest.to_owned(),
                execution_status: None,
                verification: VerificationStatus::NotRequired,
                failure: None,
                failure_category: None,
            })),
            vec![],
            prev,
            lumi_protocol::Timestamp::UNIX_EPOCH,
        )
    }

    #[test]
    fn append_chain_links_each_event() {
        let mut ledger = AuditLedger::new();
        ledger.append(event("genesis", "d1")).unwrap();
        ledger.append(event(ledger.head_hash(), "d2")).unwrap();
        ledger.append(event(ledger.head_hash(), "d3")).unwrap();
        assert_eq!(
            ledger.verify_chain(),
            ChainVerification::Intact { events: 3 }
        );
    }

    #[test]
    fn broken_link_is_detected() {
        let mut ledger = AuditLedger::new();
        ledger.append(event("genesis", "d1")).unwrap();
        // Skip the head: pretend the middle event never happened.
        ledger.append(event("genesis", "d2")).unwrap_err();
        // Force a wrong link through raw construction:
        let ok = event(ledger.head_hash(), "d2");
        let mut tampered = ok.clone();
        tampered.prev_event_hash = "genesis".to_owned();
        // Note: content hash no longer matches after mutation, which the
        // ledger also rejects.
        assert!(ledger.append(tampered).is_err());
        // The untampered event appends fine.
        ledger.append(ok).unwrap();
    }

    #[test]
    fn tenant_scoping_hides_other_tenants() {
        let mut ledger = AuditLedger::new();
        let t2_event = AuditEvent::new(
            AuditScope {
                tenant_id: TenantId::parse("t-2").unwrap(),
                task_id: lumi_protocol::TaskId::parse("task-2").unwrap(),
                run_id: lumi_protocol::RunId::parse("run-2").unwrap(),
                workflow: None,
            },
            AuditEventKind::Action(Box::new(ActionEventDetails {
                action_id: ActionId::parse("a-2").unwrap(),
                step_id: None,
                principal: fixture_principal("t-2"),
                capability: "email.send".to_owned(),
                operation: "send".to_owned(),
                resource: ResourceRef {
                    resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                    id: "x".to_owned(),
                    sensitivity: None,
                },
                target: Target::canonical("mailto:x@y.test"),
                risk_class: RiskClass::Communication,
                execution_tier: None,
                adapter: None,
                adapter_version: None,
                policy: PolicyOutcome::Allowed {
                    rule_id: "r".to_owned(),
                },
                approval_id: None,
                action_digest: "d2".to_owned(),
                execution_status: None,
                verification: VerificationStatus::NotRequired,
                failure: None,
                failure_category: None,
            })),
            vec![],
            "genesis",
            lumi_protocol::Timestamp::UNIX_EPOCH,
        );
        ledger.append(t2_event).unwrap();
        assert_eq!(ledger.events_for(&TenantId::parse("t-1").unwrap()).len(), 0);
        assert_eq!(ledger.events_for(&TenantId::parse("t-2").unwrap()).len(), 1);
    }

    #[test]
    fn jsonl_round_trip_and_tamper_detection() {
        let dir = std::env::temp_dir().join(format!("lumi-audit-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("audit.jsonl");
        let _ = std::fs::remove_file(&path);
        let log = JsonlAuditLog::new(path.clone());
        let mut ledger = AuditLedger::new();
        ledger.append(event("genesis", "d1")).unwrap();
        ledger.append(event(ledger.head_hash(), "d2")).unwrap();
        for e in ledger.events_for(&TenantId::parse("t-1").unwrap()) {
            log.append(e).unwrap();
        }
        let reloaded = log.load().unwrap();
        assert_eq!(
            reloaded.verify_chain(),
            ChainVerification::Intact { events: 2 }
        );

        // Tamper with the file: modifying content breaks hash validation.
        let content = std::fs::read_to_string(&path).unwrap();
        let tampered = content.replace("send_customer_email", "send_spam");
        std::fs::write(&path, tampered).unwrap();
        assert!(log.load().is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
