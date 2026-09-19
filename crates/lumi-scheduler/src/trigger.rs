//! Trigger normalization (spec 13 §13.2–§13.4, §13.9–§13.10).
//!
//! Any event source (cron, webhook, connector event, manual enqueue)
//! normalizes into ONE request shape before it can create a task.
//! Payloads carry references, not raw sensitive content (§13.10);
//! deduplication is enforced per key + window before a run starts (§13.4).

use lumi_protocol::{TaskId, TenantId, Timestamp};
use serde::{Deserialize, Serialize};

/// Where the trigger came from (§13.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TriggerKind {
    Cron,
    Webhook,
    ConnectorEvent,
    FileEvent,
    InboxEvent,
    DeviceLocalEvent,
    ManualEnqueue,
}

/// Original source identifier for audit (e.g. webhook URL hash, cron
/// expression, connector id).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerSource {
    pub kind: TriggerKind,
    /// Source identity, e.g. `cron:0 2 * * *`, `webhook:acme-inbox`,
    /// `connector:expense-connector`.
    pub identity: String,
}

/// What to do when the device that should run the task is unavailable
/// (§13.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceAvailability {
    /// Wait until the bound device is available (never reroute).
    WaitForDevice,
    /// Route to an eligible cloud worker IF policy permits. Local-only
    /// constraints make this equivalent to WaitForDevice (§13.9: locality
    /// is never weakened).
    MayRerouteIfPolicyPermits,
}

/// Deduplication contract (§13.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeduplicationWindow {
    /// Stable key derived from the event identity + business key (e.g.
    /// `webhook:acme-inbox:invoice-42`), NOT from arrival time.
    pub key: String,
    /// Window length in seconds: a duplicate within the window is
    /// dropped.
    pub window_seconds: u64,
}

/// Normalized trigger request (§13.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerRequest {
    pub tenant_id: TenantId,
    /// Principal under which the resulting task runs (e.g. the SCHEDULE
    /// principal bound to the unattended lease).
    pub principal_id: String,
    /// Workflow/pack id or task template ref.
    pub template_id: String,
    /// Payload REFERENCES (§13.10: refs, not raw sensitive content),
    /// e.g. `{"invoice_ref": "webhook/payloads/42"}`.
    pub payload_refs: serde_json::Value,
    pub triggered_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deduplication: Option<DeduplicationWindow>,
    pub source: TriggerSource,
    /// Schedule timezone for DST-explicit schedules (§13.11), e.g.
    /// `Europe/Berlin`. Webhook/inbox triggers may omit it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    pub device_availability: DeviceAvailability,
    /// Deadline + budgets for the resulting run (§13.6). Carried on the
    /// task at creation.
    pub deadline: Option<Timestamp>,
    pub max_actions: u32,
    /// The unattended lease authorizing this trigger (§13.5).
    pub lease_id: String,
}

impl TriggerRequest {
    /// The deduplication key including the source identity, so two
    /// sources with the same business key do not collide accidentally.
    #[must_use]
    pub fn deduplication_key(&self) -> Option<String> {
        self.deduplication
            .as_ref()
            .map(|d| format!("{}::{}", self.source.identity, d.key))
    }

    /// True when local-only privacy constraints mean cloud rerouting is
    /// forbidden regardless of the availability preference (§13.9).
    #[must_use]
    pub fn cloud_reroute_forbidden(&self, local_only: bool) -> bool {
        local_only
    }
}

/// Outcome of admitting a trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerAdmission {
    /// Create and start a task.
    Admit { task_id: TaskId },
    /// A duplicate within the dedup window: dropped, no task created.
    DuplicateDropped { key: String },
    /// Quiet hours suppressed a non-critical start (§13.8).
    QuietHours { until: Timestamp },
    /// The lease does not admit this trigger (§13.5).
    LeaseRefused { reason: String },
}

/// Dedup + admission ledger (§13.4). Record of keys with their admission
/// timestamps; a duplicate within the window is dropped BEFORE any task
/// is created.
#[derive(Debug, Default)]
pub struct DeduplicationLedger {
    admitted: std::collections::BTreeMap<String, Timestamp>,
}

impl DeduplicationLedger {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true when this (key, now) is a DUPLICATE within the
    /// window; otherwise records the admission and returns false.
    pub fn is_duplicate(&mut self, key: &str, window_seconds: u64, now: Timestamp) -> bool {
        if let Some(admitted_at) = self.admitted.get(key) {
            let elapsed = now.epoch_seconds() - admitted_at.epoch_seconds();
            if elapsed >= 0 && u64::try_from(elapsed).map_or(true, |e| e < window_seconds) {
                return true;
            }
        }
        self.admitted.insert(key.to_owned(), now);
        false
    }

    /// Explicit release (e.g. a run failed before creating side effects —
    /// the key may be re-admitted).
    pub fn release(&mut self, key: &str) {
        self.admitted.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> TriggerRequest {
        TriggerRequest {
            tenant_id: TenantId::parse("t-1").unwrap(),
            principal_id: "u-schedule".to_owned(),
            template_id: "invoice-reconciliation".to_owned(),
            payload_refs: serde_json::json!({"invoice_ref": "webhook/payloads/42"}),
            triggered_at: Timestamp::from_epoch(1000, 0).unwrap(),
            deduplication: Some(DeduplicationWindow {
                key: "invoice-42".to_owned(),
                window_seconds: 3600,
            }),
            source: TriggerSource {
                kind: TriggerKind::Webhook,
                identity: "webhook:acme-inbox".to_owned(),
            },
            timezone: Some("Europe/Berlin".to_owned()),
            device_availability: DeviceAvailability::WaitForDevice,
            deadline: Some(Timestamp::from_epoch(4600, 0).unwrap()),
            max_actions: 20,
            lease_id: "lease-1".to_owned(),
        }
    }

    #[test]
    fn dedup_key_includes_source_identity() {
        let r = request();
        assert_eq!(
            r.deduplication_key().unwrap(),
            "webhook:acme-inbox::invoice-42"
        );
    }

    #[test]
    fn duplicates_within_window_are_dropped() {
        let mut ledger = DeduplicationLedger::new();
        let key = r#"webhook:acme-inbox::invoice-42"#;
        let now = Timestamp::from_epoch(1000, 0).unwrap();
        assert!(!ledger.is_duplicate(key, 3600, now), "first arrival admits");
        // Same event re-delivered 10 minutes later: duplicate.
        assert!(ledger.is_duplicate(key, 3600, Timestamp::from_epoch(1600, 0).unwrap()));
        // Same event 2 hours later: window elapsed, admits again.
        assert!(!ledger.is_duplicate(key, 3600, Timestamp::from_epoch(5600, 0).unwrap()));
    }

    #[test]
    fn local_only_blocks_cloud_reroute() {
        let r = request();
        assert!(
            r.cloud_reroute_forbidden(true),
            "local-only must never reroute"
        );
        assert!(!r.cloud_reroute_forbidden(false));
    }
}
