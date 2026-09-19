//! Model capability contracts (spec 09 §9.2).
//!
//! Unsupported capabilities MUST be explicit: an adapter declares exactly
//! what its models support, and routing fails closed on a missing required
//! capability instead of silently degrading.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Capability dimensions a model/adapter may declare (spec 09 §9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    Text,
    Reasoning,
    Vision,
    ToolUse,
    StructuredOutput,
    Embeddings,
    NativeComputerUse,
    Streaming,
    LongContext,
    ParallelToolCalls,
    UsageReporting,
}

/// A model offered by a provider instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Provider-side model identifier, e.g. `gpt-4o-mini`.
    pub id: String,
    pub capabilities: BTreeSet<ModelCapability>,
    /// Advertised maximum context window in tokens (0 = unknown).
    pub max_context_tokens: u64,
    /// Data regions this model is served from, e.g. `us`, `eu`.
    pub data_regions: BTreeSet<String>,
}

impl ModelInfo {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            capabilities: BTreeSet::new(),
            max_context_tokens: 0,
            data_regions: BTreeSet::new(),
        }
    }

    pub fn with_capabilities(
        mut self,
        capabilities: impl IntoIterator<Item = ModelCapability>,
    ) -> Self {
        self.capabilities.extend(capabilities);
        self
    }

    pub fn with_context(mut self, tokens: u64) -> Self {
        self.max_context_tokens = tokens;
        self
    }

    pub fn with_regions(mut self, regions: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.data_regions
            .extend(regions.into_iter().map(Into::into));
        self
    }

    /// True when every required capability is declared. Unsupported = not
    /// matched; there is no silent degradation.
    #[must_use]
    pub fn supports(&self, required: &BTreeSet<ModelCapability>) -> bool {
        required.iter().all(|cap| self.capabilities.contains(cap))
    }
}

/// Convenience constructor sets for common capability bundles.
#[must_use]
pub fn text_model(id: &str) -> ModelInfo {
    ModelInfo::new(id)
        .with_capabilities([ModelCapability::Text, ModelCapability::UsageReporting])
        .with_context(8_192)
}

#[must_use]
pub fn tool_model(id: &str) -> ModelInfo {
    text_model(id).with_capabilities([
        ModelCapability::ToolUse,
        ModelCapability::StructuredOutput,
        ModelCapability::Reasoning,
    ])
}
