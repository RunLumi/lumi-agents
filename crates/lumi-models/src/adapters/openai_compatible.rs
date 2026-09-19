//! OpenAI-compatible adapter.
//!
//! A distinct driver identity for third-party OpenAI-shaped endpoints
//! (Ollama, vLLM, OpenRouter, enterprise gateways). Compatibility with
//! OpenAI semantics is NOT assumed (spec 09 §9.13): this driver carries
//! its own conservative catalog and MUST pass the same shared contract
//! suite, which additionally asserts behaviors compatible endpoints are
//! known to get wrong (usage metadata presence, strict JSON objects in
//! tool arguments).

use crate::adapters::openai::{OpenAIDriver, OpenAIWire};
use crate::capabilities::ModelInfo;
use crate::driver::ProviderDriver;
use crate::error::ModelError;
use crate::request::ModelRequest;
use crate::response::ModelResponse;
use crate::transport::HttpTransport;
use std::time::Instant;

/// The generic OpenAI-compatible driver family.
#[derive(Debug, Default, Clone, Copy)]
pub struct OpenAICompatibleDriver;

impl OpenAICompatibleDriver {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    pub(crate) const WIRE: OpenAIWire = OpenAIWire {
        name: "openai-compatible",
    };
}

impl ProviderDriver for OpenAICompatibleDriver {
    fn name(&self) -> &'static str {
        "openai-compatible"
    }

    fn describe(&self, model: &str) -> ModelInfo {
        // Conservative: text + tool/structured capabilities are opt-in per
        // deployment catalog; streaming is NOT declared (compatible
        // endpoints frequently diverge on SSE semantics).
        let mut info = OpenAIWire::describe(&Self::WIRE, model);
        info.capabilities
            .remove(&crate::capabilities::ModelCapability::Streaming);
        info
    }

    fn complete(
        &self,
        transport: &dyn HttpTransport,
        endpoint: &str,
        auth_headers: &[(String, String)],
        request: &ModelRequest,
    ) -> Result<ModelResponse, ModelError> {
        let start = Instant::now();
        let mut response = Self::WIRE.complete(transport, endpoint, auth_headers, request)?;
        response.latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        Ok(response)
    }

    fn list_models(
        &self,
        transport: &dyn HttpTransport,
        endpoint: &str,
        auth_headers: &[(String, String)],
    ) -> Result<Vec<String>, ModelError> {
        Self::WIRE.list_models(transport, endpoint, auth_headers)
    }
}

/// Re-exported for the shared contract suite: the suite runs the same
/// scenarios against both the OpenAI driver and this one, proving
/// compatibility rather than assuming it.
#[must_use]
pub fn openai_driver_for_contract_suite() -> OpenAIDriver {
    OpenAIDriver::new()
}
