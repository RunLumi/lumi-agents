//! Real HTTP transport: `ureq` with OS-backed TLS (native-tls).
//!
//! Security.framework on macOS and schannel on Windows mean the local
//! trust runtime ships no bundled TLS/crypto implementation.

use crate::error::TransportError;
use crate::transport::{HttpResponse, HttpTransport};
use std::time::Duration;

/// Synchronous transport over `ureq`.
#[derive(Debug, Default, Clone, Copy)]
pub struct UreqTransport;

impl UreqTransport {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn agent(timeout: Duration) -> ureq::Agent {
        ureq::AgentBuilder::new()
            .timeout_connect(timeout)
            .timeout(timeout)
            .build()
    }

    fn map_err(e: ureq::Error) -> TransportError {
        match e {
            // Non-2xx statuses should arrive as responses (adapters map
            // provider error bodies); treat this path defensively.
            ureq::Error::Status(_, _) => {
                TransportError::Malformed("unexpected status path".to_owned())
            }
            ureq::Error::Transport(t) => {
                if t.to_string().contains("timed out") {
                    TransportError::Timeout
                } else {
                    TransportError::Connect(t.to_string())
                }
            }
        }
    }

    fn io_err(e: std::io::Error) -> TransportError {
        if e.kind() == std::io::ErrorKind::TimedOut || e.to_string().contains("timed out") {
            TransportError::Timeout
        } else {
            TransportError::Malformed(e.to_string())
        }
    }

    fn finish(result: Result<ureq::Response, ureq::Error>) -> Result<HttpResponse, TransportError> {
        let response = result.map_err(Self::map_err)?;
        let status = response.status();
        let body = response.into_string().map_err(Self::io_err)?;
        Ok(HttpResponse { status, body })
    }
}

impl HttpTransport for UreqTransport {
    fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &str,
        timeout: Duration,
    ) -> Result<HttpResponse, TransportError> {
        let mut request = Self::agent(timeout)
            .post(url)
            .set("Content-Type", "application/json");
        for (name, value) in headers {
            request = request.set(name, value);
        }
        Self::finish(request.send_string(body))
    }

    fn get_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        timeout: Duration,
    ) -> Result<HttpResponse, TransportError> {
        let mut request = Self::agent(timeout).get(url);
        for (name, value) in headers {
            request = request.set(name, value);
        }
        Self::finish(request.call())
    }
}
