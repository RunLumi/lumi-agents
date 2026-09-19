//! Provider-neutral model contracts and routing (spec 09).
//!
//! Business/workflow logic MUST NOT branch on provider names (spec 09
//! §9.4): workflows speak [`ModelRequest`]/[`ModelResponse`]; provider
//! adapters translate. Routing enforces the spec 09 §9.7 order where
//! privacy constraints outrank cost, and fallback (§9.8) can never weaken
//! local-only, region, allowlist, or capability constraints.
//!
//! Architecture layers (spec 09 §9.16):
//!
//! ```text
//! ProviderDriver (protocol family: OpenAI/Anthropic/Gemini/compatible)
//!   -> ProviderInstance (one account/endpoint/credential boundary)
//!     -> ModelCatalog (discovered models/capabilities)
//!       -> RouteSnapshot (per task/turn routing record, persisted)
//! ```
//!
//! "OpenAI-compatible" endpoints are a distinct driver and MUST pass the
//! same shared contract suite (spec 09 §9.13); compatibility is proven,
//! not assumed. The shared contract suite runs against fixture transports
//! playing recorded provider wire shapes, so it is hermetic and
//! deterministic.

pub mod capabilities;
pub mod driver;
pub mod error;
pub mod instance;
pub mod pricing;
pub mod request;
pub mod response;
pub mod routing;
pub mod snapshot;
pub mod transport;

#[cfg(feature = "http")]
pub mod ureq_transport;

pub mod adapters;

pub use capabilities::{ModelCapability, ModelInfo};
pub use driver::ProviderDriver;
pub use error::{ModelError, TransportError};
pub use instance::{InstanceStatus, ProviderInstance};
pub use pricing::{ModelPrice, PriceTable};
pub use request::{ModelMessage, ModelRequest, ModelRole, ToolSpec};
pub use response::{ModelResponse, ToolCall, Usage};
pub use routing::{ensure_admission, route, Candidate, RouteChoice, RouteError, TenantModelPolicy};
pub use snapshot::RouteSnapshot;
#[cfg(feature = "http")]
pub use ureq_transport::UreqTransport;
