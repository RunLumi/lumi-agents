//! Provider driver contract (spec 09 §9.2, §9.16).

use crate::capabilities::ModelInfo;
use crate::error::ModelError;
use crate::request::ModelRequest;
use crate::response::ModelResponse;
use crate::transport::HttpTransport;

/// One protocol/implementation family (OpenAI, Anthropic, Gemini,
/// OpenAI-compatible…). Stateless: all mutable configuration lives in the
/// [`crate::ProviderInstance`], so two instances of one driver never share
/// credential/session/catalog state (spec 09 §9.19).
pub trait ProviderDriver: std::fmt::Debug + Send + Sync {
    /// The driver family name, e.g. `openai`.
    fn name(&self) -> &'static str;

    /// Auth headers for one call. The `credential` value is resolved by
    /// the CALLER from the secret broker at this boundary and is never
    /// persisted or logged.
    fn auth_headers(&self, credential: &str) -> Vec<(String, String)> {
        vec![("Authorization".to_owned(), format!("Bearer {credential}"))]
    }

    /// Declared capabilities for a concrete model id on this driver.
    /// Unknown models return an empty (explicitly incapable) set.
    fn describe(&self, model: &str) -> ModelInfo;

    /// Executes one normalized request through `transport` for the given
    /// instance configuration (endpoint + credential reference resolved
    /// by the caller into header values).
    ///
    /// # Errors
    /// [`ModelError`] normalized onto the canonical failure taxonomy.
    fn complete(
        &self,
        transport: &dyn HttpTransport,
        endpoint: &str,
        auth_headers: &[(String, String)],
        request: &ModelRequest,
    ) -> Result<ModelResponse, ModelError>;

    /// Lists model ids advertised at the endpoint (catalog probe).
    ///
    /// Health/setup checks MUST only probe; they never trigger logins,
    /// durable sessions, or credential mutations (spec 09 §9.17).
    fn list_models(
        &self,
        transport: &dyn HttpTransport,
        endpoint: &str,
        auth_headers: &[(String, String)],
    ) -> Result<Vec<String>, ModelError> {
        let _ = (transport, endpoint, auth_headers);
        Err(ModelError::format("catalog listing not supported"))
    }
}
