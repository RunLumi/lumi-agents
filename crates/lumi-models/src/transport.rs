//! HTTP transport abstraction.
//!
//! Adapters speak provider HTTP APIs through this trait so the shared
//! contract suite can run hermetically against fixture transports, and the
//! real transport (`UreqTransport`, OS-backed TLS) is swappable.

use crate::error::TransportError;
use std::time::Duration;

/// One HTTP response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// Minimal JSON-over-HTTP client surface used by adapters.
pub trait HttpTransport: std::fmt::Debug + Send + Sync {
    /// Issues a POST with a JSON body and the given headers.
    ///
    /// # Errors
    /// [`TransportError`] on timeout/connect/malformed framing.
    fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &str,
        timeout: Duration,
    ) -> Result<HttpResponse, TransportError>;

    /// Issues a GET with the given headers (health/catalog probes).
    ///
    /// # Errors
    /// [`TransportError`] on timeout/connect/malformed framing.
    fn get_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        timeout: Duration,
    ) -> Result<HttpResponse, TransportError> {
        let _ = (url, headers, timeout);
        Err(TransportError::NoTransport)
    }
}

/// A scripted transport for contract tests: records requests, replays
/// canned responses, and can simulate transport-level failures.
#[derive(Debug, Default, Clone)]
pub struct FixtureTransport {
    pub responses: std::sync::Arc<std::sync::Mutex<Vec<FixtureResponse>>>,
    pub seen_requests: std::sync::Arc<std::sync::Mutex<Vec<SeenRequest>>>,
}

/// One scripted response entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixtureResponse {
    /// HTTP response with status + JSON body.
    Http { status: u16, body: String },
    /// Transport-level timeout.
    Timeout,
    /// Connection failure.
    Connect(String),
    /// Malformed response framing.
    Malformed(String),
}

/// A recorded incoming request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeenRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl FixtureTransport {
    #[must_use]
    pub fn serving(responses: Vec<FixtureResponse>) -> Self {
        Self {
            responses: std::sync::Arc::new(std::sync::Mutex::new(responses)),
            seen_requests: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    #[must_use]
    pub fn requests(&self) -> Vec<SeenRequest> {
        self.seen_requests
            .lock()
            .map(|r| r.clone())
            .unwrap_or_default()
    }
}

impl HttpTransport for FixtureTransport {
    fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &str,
        timeout: Duration,
    ) -> Result<HttpResponse, TransportError> {
        let _ = timeout;
        self.seen_requests
            .lock()
            .map(|mut seen| {
                seen.push(SeenRequest {
                    method: "POST".to_owned(),
                    url: url.to_owned(),
                    headers: headers.to_vec(),
                    body: body.to_owned(),
                });
            })
            .map_err(|_| TransportError::Connect("poisoned fixture".to_owned()))?;
        self.next_response()
    }

    fn get_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        timeout: Duration,
    ) -> Result<HttpResponse, TransportError> {
        let _ = timeout;
        self.seen_requests
            .lock()
            .map(|mut seen| {
                seen.push(SeenRequest {
                    method: "GET".to_owned(),
                    url: url.to_owned(),
                    headers: headers.to_vec(),
                    body: String::new(),
                });
            })
            .map_err(|_| TransportError::Connect("poisoned fixture".to_owned()))?;
        self.next_response()
    }
}

impl FixtureTransport {
    fn next_response(&self) -> Result<HttpResponse, TransportError> {
        let mut guard = self
            .responses
            .lock()
            .map_err(|_| TransportError::Connect("poisoned fixture".to_owned()))?;
        if guard.is_empty() {
            return Err(TransportError::Connect(
                "fixture exhausted: no scripted response".to_owned(),
            ));
        }
        match guard.remove(0) {
            FixtureResponse::Http { status, body } => Ok(HttpResponse { status, body }),
            FixtureResponse::Timeout => Err(TransportError::Timeout),
            FixtureResponse::Connect(e) => Err(TransportError::Connect(e)),
            FixtureResponse::Malformed(e) => Err(TransportError::Malformed(e)),
        }
    }
}
