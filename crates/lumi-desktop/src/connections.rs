//! Project connections (Spec 29): named per-project integrations whose
//! credentials are REFERENCES into the secrets broker — the credential
//! value lives only in the backend (OS keyring in production, in-memory
//! in tests) and never in project files, records, logs, or the UI.

use lumi_protocol::Timestamp;
use lumi_secrets::{SecretBackend, SecretBroker, SecretValue};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One registered project connection (metadata only — no credential).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionRecord {
    pub connection_id: String,
    pub project_id: String,
    pub name: String,
    /// Integration kind: `api_key` | `basic` | `custom`.
    pub kind: String,
    pub endpoint: String,
    /// Broker reference; resolution happens at the narrowest boundary.
    pub credential_ref: String,
    pub created_at: Timestamp,
}

/// Connection metadata lives in its own JSON store beside the runtime
/// state; secrets live in the broker. Atomic write (temp + rename).
pub fn load_connection_records(path: &Path) -> Result<Vec<ConnectionRecord>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("read connections: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("corrupt connections file: {e}"))
}

pub fn save_connection_records(path: &Path, records: &[ConnectionRecord]) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    let json =
        serde_json::to_string_pretty(records).map_err(|e| format!("serialize connections: {e}"))?;
    std::fs::write(&tmp, json).map_err(|e| format!("write connections: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("persist connections: {e}"))
}

fn records_for<'a>(records: &'a [ConnectionRecord], project_id: &str) -> Vec<&'a ConnectionRecord> {
    records
        .iter()
        .filter(|record| record.project_id == project_id)
        .collect()
}

/// Verifies a connection's credential is still present in the broker
/// WITHOUT exposing it (Spec 29: status breadth). Returns false when
/// the connection is unknown to this project.
///
/// # Errors
/// Store or broker failures other than absence.
pub fn verify_connection<B: SecretBackend>(
    broker: &SecretBroker<B>,
    records_path: &Path,
    project_id: &str,
    connection_id: &str,
) -> Result<bool, String> {
    let records = load_connection_records(records_path)?;
    let Some(record) = records
        .iter()
        .find(|r| r.project_id == project_id && r.connection_id == connection_id)
    else {
        return Ok(false);
    };
    broker
        .exists(&lumi_protocol::SecretRef(record.credential_ref.clone()))
        .map_err(|e| format!("verify credential: {e}"))
}

/// Registers a connection: validates input, stores the credential in
/// the broker under a project-scoped reference, and persists metadata
/// (never the credential value).
///
/// # Errors
/// Empty name/endpoint/credential, duplicate name for the project, or
/// store failures.
pub fn connect_connection<B: SecretBackend>(
    broker: &SecretBroker<B>,
    records_path: &Path,
    project_id: &str,
    name: &str,
    kind: &str,
    endpoint: &str,
    credential: &str,
) -> Result<ConnectionRecord, String> {
    let name = name.trim();
    let kind = kind.trim().to_lowercase();
    let endpoint = endpoint.trim().to_owned();
    let credential = credential.trim();
    if name.is_empty() || endpoint.is_empty() || credential.is_empty() {
        return Err("name, endpoint, and credential are all required".to_owned());
    }
    let allowed = ["api_key", "basic", "custom"];
    if !allowed.contains(&kind.as_str()) {
        return Err(format!("unsupported connection kind {kind:?}"));
    }

    let mut records = load_connection_records(records_path)?;
    if records_for(&records, project_id)
        .iter()
        .any(|r| r.name == name)
    {
        return Err(format!(
            "connection {name:?} already exists for this project"
        ));
    }

    let connection_id = format!(
        "conn-{}",
        lumi_protocol::canonical::sha256_hex(format!("{project_id}/{name}/{endpoint}").as_bytes())
            [..12]
            .to_owned()
    );
    let credential_ref = format!("connections/{project_id}/{connection_id}");

    broker
        .store(
            &lumi_protocol::SecretRef(credential_ref.clone()),
            &SecretValue::new(credential),
        )
        .map_err(|e| format!("store credential: {e}"))?;

    let record = ConnectionRecord {
        connection_id: connection_id.clone(),
        project_id: project_id.to_owned(),
        name: name.to_owned(),
        kind,
        endpoint,
        credential_ref,
        created_at: Timestamp::now(),
    };
    records.push(record.clone());
    save_connection_records(records_path, &records)?;
    Ok(record)
}

/// Removes a connection: revokes the broker secret (best-effort) and
/// deletes the metadata record. Returns false when nothing matched.
///
/// # Errors
/// Store failures.
pub fn disconnect_connection<B: SecretBackend>(
    broker: &SecretBroker<B>,
    records_path: &Path,
    project_id: &str,
    connection_id: &str,
) -> Result<bool, String> {
    let records = load_connection_records(records_path)?;
    let Some(position) = records
        .iter()
        .position(|r| r.project_id == project_id && r.connection_id == connection_id)
    else {
        return Ok(false);
    };
    let record = &records[position];
    let _ = broker.revoke(&lumi_protocol::SecretRef(record.credential_ref.clone()));
    let mut rest: Vec<ConnectionRecord> = records;
    rest.remove(position);
    save_connection_records(records_path, &rest)?;
    Ok(true)
}

/// Lists a project's connections, project-scoped. Values never appear.
///
/// # Errors
/// Store failures.
pub fn list_connections(
    records_path: &Path,
    project_id: &str,
) -> Result<Vec<ConnectionRecord>, String> {
    let records = load_connection_records(records_path)?;
    Ok(records_for(&records, project_id)
        .into_iter()
        .cloned()
        .collect())
}

/// Convenience handle bundling the broker with the records path for the
/// desktop shell (records beside the runtime state file).
#[derive(Debug)]
pub struct ConnectionsStore {
    pub records_path: PathBuf,
}

impl ConnectionsStore {
    #[must_use]
    pub fn new(records_path: PathBuf) -> Self {
        Self { records_path }
    }

    /// # Errors
    /// Store failures.
    pub fn connect<B: SecretBackend>(
        &self,
        broker: &SecretBroker<B>,
        project_id: &str,
        name: &str,
        kind: &str,
        endpoint: &str,
        credential: &str,
    ) -> Result<ConnectionRecord, String> {
        connect_connection(
            broker,
            &self.records_path,
            project_id,
            name,
            kind,
            endpoint,
            credential,
        )
    }

    /// # Errors
    /// Store failures.
    pub fn disconnect<B: SecretBackend>(
        &self,
        broker: &SecretBroker<B>,
        project_id: &str,
        connection_id: &str,
    ) -> Result<bool, String> {
        disconnect_connection(broker, &self.records_path, project_id, connection_id)
    }

    #[must_use]
    pub fn list(&self, project_id: &str) -> Vec<ConnectionRecord> {
        list_connections(&self.records_path, project_id).unwrap_or_default()
    }

    /// # Errors
    /// Broker failures other than absence.
    pub fn verify<B: SecretBackend>(
        &self,
        broker: &SecretBroker<B>,
        project_id: &str,
        connection_id: &str,
    ) -> Result<bool, String> {
        verify_connection(broker, &self.records_path, project_id, connection_id)
    }
}
