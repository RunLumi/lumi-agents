//! Provider instances: concrete account/configuration boundaries
//! (spec 09 §9.16–9.17).

use crate::capabilities::ModelInfo;
use crate::driver::ProviderDriver;
use crate::error::ModelError;
use crate::request::ModelRequest;
use crate::response::ModelResponse;
use crate::transport::HttpTransport;
use lumi_protocol::SecretRef;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;

/// Lifecycle status of an instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstanceStatus {
    /// Admitted for new work.
    Active,
    /// Exists but temporarily not admitting new work.
    Paused,
    /// Revoked: MUST block admission of new tasks/sessions (spec 09
    /// §9.19). Existing durable state is cleared before new admission
    /// could ever be considered.
    Revoked,
}

/// One concrete provider account/endpoint boundary.
///
/// Credentials are REFERENCED (`credential_ref`), never stored here;
/// resolution happens at the narrowest executor boundary right before the
/// transport call (AGENTS.md secrets rule).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderInstance {
    pub instance_id: String,
    /// Driver family, e.g. `openai`, `openai-compatible`.
    pub driver: String,
    /// Human label, e.g. `acme-prod-openai`.
    pub label: String,
    /// Base URL, e.g. `https://api.openai.com` or
    /// `http://localhost:11434` for a local Ollama endpoint.
    pub endpoint: String,
    /// Reference into the secret broker. Never the credential itself.
    pub credential_ref: SecretRef,
    /// Provider name as matched against tenant allowlists.
    pub provider_name: String,
    /// True when the endpoint runs on-device (local-only eligible).
    pub local_execution: bool,
    /// Data regions served, e.g. `us`, `eu`, `local`.
    pub data_regions: Vec<String>,
    pub status: InstanceStatus,
    /// Cached catalog per model id (does NOT grant authorization).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub catalog: BTreeMap<String, ModelInfo>,
}

impl ProviderInstance {
    #[must_use]
    pub fn new(
        instance_id: impl Into<String>,
        driver: &'static str,
        label: impl Into<String>,
        endpoint: impl Into<String>,
        credential_ref: SecretRef,
        provider_name: impl Into<String>,
    ) -> Self {
        Self {
            instance_id: instance_id.into(),
            driver: driver.to_owned(),
            label: label.into(),
            endpoint: endpoint.into(),
            credential_ref,
            provider_name: provider_name.into(),
            local_execution: false,
            data_regions: Vec::new(),
            status: InstanceStatus::Active,
            catalog: BTreeMap::new(),
        }
    }

    pub fn local(mut self) -> Self {
        self.local_execution = true;
        self.data_regions = vec!["local".to_owned()];
        self
    }

    pub fn with_regions(mut self, regions: Vec<&'static str>) -> Self {
        self.data_regions = regions.into_iter().map(str::to_owned).collect();
        self
    }

    pub fn with_catalog(mut self, models: Vec<ModelInfo>) -> Self {
        self.catalog = models.into_iter().map(|m| (m.id.clone(), m)).collect();
        self
    }

    /// True when the instance may admit NEW work.
    #[must_use]
    pub const fn admits_new_work(&self) -> bool {
        matches!(self.status, InstanceStatus::Active)
    }

    /// Health probe: lists models (read-only GET). MUST NOT create
    /// sessions, launch login flows, or mutate credential state (spec 09
    /// §9.17).
    ///
    /// # Errors
    /// [`ModelError`] normalized from the transport/provider failure.
    pub fn health_probe(
        &self,
        driver: &dyn ProviderDriver,
        transport: &dyn HttpTransport,
    ) -> Result<Vec<String>, ModelError> {
        driver.list_models(transport, &self.endpoint, &[])
    }

    /// Sign-out/revocation: prevents new admission BEFORE clearing
    /// mutable state (spec 09 §9.17 ordering).
    pub fn revoke(&mut self) {
        self.status = InstanceStatus::Revoked;
        self.catalog.clear();
    }
}

/// Resolved credential headers for a call. Built at the executor boundary
/// from the secret broker; never logged, never persisted.
pub type AuthHeaders = Vec<(String, String)>;

/// Executes one request against a specific instance (routing has already
/// chosen it).
///
/// # Errors
/// [`ModelError`] normalized onto the taxonomy.
pub fn complete_on_instance(
    driver: &dyn ProviderDriver,
    instance: &ProviderInstance,
    auth: &AuthHeaders,
    transport: &dyn HttpTransport,
    request: &ModelRequest,
) -> Result<ModelResponse, ModelError> {
    if !instance.admits_new_work() {
        return Err(ModelError::new(
            lumi_protocol::FailureCategory::SecurityViolation,
            format!("provider instance {} is not active", instance.instance_id),
            false,
        ));
    }
    driver.complete(transport, &instance.endpoint, auth, request)
}

/// Probe timeout: short, explicit.
pub const HEALTH_PROBE_TIMEOUT: Duration = Duration::from_secs(10);
