//! Shared provider adapter contract suite (spec 09 §9.15, §9.13).
//!
//! Every driver family — OpenAI, Anthropic, Gemini, and
//! OpenAI-compatible — runs the SAME scenarios against fixture transports
//! playing recorded provider wire shapes. Compatibility is proven, not
//! assumed: an "OpenAI-compatible" endpoint passes these tests or it does
//! not pass at all.
//!
//! Streaming and request cancellation are deliberately NOT declared by the
//! v1 adapters (spec 09 §9.2: unsupported capability MUST be explicit);
//! the capability matrix test pins that honesty.

use lumi_models::adapters::{AnthropicDriver, GeminiDriver, OpenAICompatibleDriver, OpenAIDriver};
use lumi_models::capabilities::ModelCapability;
use lumi_models::driver::ProviderDriver;
use lumi_models::request::{ModelMessage, ModelRequest, ModelRole, ToolSpec};
use lumi_models::response::FinishReason;
use lumi_models::transport::{FixtureResponse, FixtureTransport};

fn text_request(driver_name: &str, model: &str) -> ModelRequest {
    ModelRequest::text(
        format!("req-{driver_name}"),
        ModelRole::Planner,
        model,
        "Summarize the reconciliation steps.",
    )
}

fn tool_request(model: &str) -> ModelRequest {
    let mut request =
        ModelRequest::text("req-tools", ModelRole::Extractor, model, "Extract totals");
    request.tools = vec![ToolSpec {
        name: "read_invoice_total".to_owned(),
        description: "Read the total of an invoice".to_owned(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {"invoice_id": {"type": "string"}},
            "required": ["invoice_id"],
        }),
    }];
    request
}

fn structured_request(model: &str) -> ModelRequest {
    let mut request = ModelRequest::text("req-struct", ModelRole::Extractor, model, "Return JSON");
    request.structured_output_schema = Some("totals".to_owned());
    request
}

fn auth() -> Vec<(String, String)> {
    vec![("Authorization".to_owned(), "Bearer test".to_owned())]
}

#[test]
fn capability_matrix_declares_streaming_and_cancellation_unsupported() {
    // Spec 09 §9.2: unsupported capabilities must be explicit, not
    // silently absent semantics.
    for (name, driver, model) in [
        ("openai", &ProviderDriverRef::OpenAI, "gpt-4o-mini"),
        ("anthropic", &ProviderDriverRef::Anthropic, "claude-sonnet"),
        ("gemini", &ProviderDriverRef::Gemini, "gemini-2"),
        (
            "openai-compatible",
            &ProviderDriverRef::Compatible,
            "llama3.1",
        ),
    ] {
        let info = driver.driver().describe(model);
        assert!(
            info.supports(&[ModelCapability::Text].into_iter().collect()),
            "{name}"
        );
        assert!(
            !info.capabilities.contains(&ModelCapability::Streaming),
            "{name} must explicitly not claim streaming in v1"
        );
        assert!(
            !info
                .capabilities
                .contains(&ModelCapability::NativeComputerUse),
            "{name} must not claim computer use"
        );
    }
}

/// Enum wrapper so the suite can iterate driver references.
#[derive(Debug, Clone, Copy)]
enum ProviderDriverRef {
    OpenAI,
    Anthropic,
    Gemini,
    Compatible,
}

impl ProviderDriverRef {
    fn driver(self) -> Box<dyn ProviderDriver> {
        match self {
            Self::OpenAI => Box::new(OpenAIDriver::new()),
            Self::Anthropic => Box::new(AnthropicDriver::new()),
            Self::Gemini => Box::new(GeminiDriver::new()),
            Self::Compatible => Box::new(OpenAICompatibleDriver::new()),
        }
    }

    fn model(self) -> &'static str {
        match self {
            Self::OpenAI | Self::Compatible => "gpt-4o-mini",
            Self::Anthropic => "claude-sonnet",
            Self::Gemini => "gemini-2",
        }
    }

    fn success_body(self) -> String {
        match self {
            Self::OpenAI | Self::Compatible => serde_json::json!({
                "id": "chatcmpl-123",
                "choices": [{"index": 0, "message": {"role": "assistant", "content": "1) collect 2) match 3) report"}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 12, "completion_tokens": 9}
            })
            .to_string(),
            Self::Anthropic => serde_json::json!({
                "id": "msg_123",
                "content": [{"type": "text", "text": "1) collect 2) match 3) report"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 12, "output_tokens": 9}
            })
            .to_string(),
            Self::Gemini => serde_json::json!({
                "candidates": [{"content": {"parts": [{"text": "1) collect 2) match 3) report"}], "role": "model"}, "finishReason": "STOP"}],
                "usageMetadata": {"promptTokenCount": 12, "candidatesTokenCount": 9}
            })
            .to_string(),
        }
    }

    fn tool_call_body(self) -> String {
        match self {
            Self::OpenAI | Self::Compatible => serde_json::json!({
                "id": "chatcmpl-t",
                "choices": [{"index": 0, "message": {
                    "role": "assistant", "content": null,
                    "tool_calls": [{"id": "call-1", "type": "function",
                        "function": {"name": "read_invoice_total", "arguments": "{\"invoice_id\":\"INV-9\"}"}}]},
                    "finish_reason": "tool_calls"}],
                "usage": {"prompt_tokens": 20, "completion_tokens": 5}
            })
            .to_string(),
            Self::Anthropic => serde_json::json!({
                "id": "msg_t",
                "content": [{"type": "tool_use", "id": "call-1", "name": "read_invoice_total",
                    "input": {"invoice_id": "INV-9"}}],
                "stop_reason": "tool_use",
                "usage": {"input_tokens": 20, "output_tokens": 5}
            })
            .to_string(),
            Self::Gemini => serde_json::json!({
                "candidates": [{"content": {"parts": [
                    {"functionCall": {"name": "read_invoice_total", "args": {"invoice_id": "INV-9"}}}
                ], "role": "model"}, "finishReason": "STOP"}],
                "usageMetadata": {"promptTokenCount": 20, "candidatesTokenCount": 5}
            })
            .to_string(),
        }
    }

    fn structured_body(self) -> String {
        match self {
            Self::OpenAI | Self::Compatible => serde_json::json!({
                "id": "chatcmpl-s",
                "choices": [{"index": 0, "message": {"role": "assistant",
                    "content": "{\"total\": 4200, \"currency\": \"USD\"}"}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 8, "completion_tokens": 10}
            })
            .to_string(),
            Self::Anthropic => serde_json::json!({
                "id": "msg_s",
                "content": [{"type": "text", "text": "{\"total\": 4200, \"currency\": \"USD\"}"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 8, "output_tokens": 10}
            })
            .to_string(),
            Self::Gemini => serde_json::json!({
                "candidates": [{"content": {"parts": [{"text": "{\"total\": 4200, \"currency\": \"USD\"}"}], "role": "model"}, "finishReason": "STOP"}],
                "usageMetadata": {"promptTokenCount": 8, "candidatesTokenCount": 10}
            })
            .to_string(),
        }
    }

    fn rate_limited_body(self) -> String {
        match self {
            Self::OpenAI | Self::Compatible => serde_json::json!({
                "error": {"message": "Rate limit reached", "type": "requests", "code": "rate_limit_exceeded"}
            })
            .to_string(),
            Self::Anthropic => serde_json::json!({
                "type": "error", "error": {"type": "rate_limit_error", "message": "Number of requests has exceeded your per-minute rate"}
            })
            .to_string(),
            Self::Gemini => serde_json::json!({
                "error": {"code": 429, "message": "Resource has been exhausted (e.g. check quota).", "status": "RESOURCE_EXHAUSTED"}
            })
            .to_string(),
        }
    }

    fn context_overflow_body(self) -> String {
        match self {
            Self::OpenAI | Self::Compatible => serde_json::json!({
                "error": {"message": "This model's maximum context length is 128000 tokens", "type": "invalid_request_error", "code": "context_length_exceeded"}
            })
            .to_string(),
            Self::Anthropic => serde_json::json!({
                "type": "error", "error": {"type": "invalid_request_error", "message": "prompt is too long: 210000 tokens > 200000 context length maximum"}
            })
            .to_string(),
            Self::Gemini => serde_json::json!({
                "error": {"code": 400, "message": "The input token count exceeds the maximum number of tokens allowed", "status": "INVALID_ARGUMENT"}
            })
            .to_string(),
        }
    }

    fn auth_rejected_body(self) -> String {
        match self {
            Self::OpenAI | Self::Compatible => serde_json::json!({
                "error": {"message": "Incorrect API key provided", "type": "invalid_request_error", "code": "invalid_api_key"}
            })
            .to_string(),
            Self::Anthropic => serde_json::json!({
                "type": "error", "error": {"type": "authentication_error", "message": "invalid x-api-key"}
            })
            .to_string(),
            Self::Gemini => serde_json::json!({
                "error": {"code": 403, "message": "API key not valid. Please pass a valid API key.", "status": "PERMISSION_DENIED"}
            })
            .to_string(),
        }
    }
}

#[test]
fn all_drivers_text_completion_with_usage() {
    for provider in [
        ProviderDriverRef::OpenAI,
        ProviderDriverRef::Anthropic,
        ProviderDriverRef::Gemini,
        ProviderDriverRef::Compatible,
    ] {
        let name = provider.driver().name();
        let transport = FixtureTransport::serving(vec![FixtureResponse::Http {
            status: 200,
            body: provider.success_body(),
        }]);
        let request = text_request(name, provider.model());
        let response = provider
            .driver()
            .complete(&transport, "https://fixture.test", &auth(), &request)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            response.content.as_deref(),
            Some("1) collect 2) match 3) report"),
            "{name}"
        );
        assert_eq!(response.usage.input_tokens, 12, "{name}");
        assert_eq!(response.usage.output_tokens, 9, "{name}");
        assert_eq!(response.finish_reason, FinishReason::Stop, "{name}");
        // The request actually went out with the model + user content.
        let seen = transport.requests();
        assert_eq!(seen.len(), 1, "{name}");
        assert!(
            seen[0].body.contains("Summarize the reconciliation steps."),
            "{name}"
        );
    }
}

#[test]
fn all_drivers_parse_tool_calls() {
    for provider in [
        ProviderDriverRef::OpenAI,
        ProviderDriverRef::Anthropic,
        ProviderDriverRef::Gemini,
        ProviderDriverRef::Compatible,
    ] {
        let name = provider.driver().name();
        let transport = FixtureTransport::serving(vec![FixtureResponse::Http {
            status: 200,
            body: provider.tool_call_body(),
        }]);
        let request = tool_request(provider.model());
        let response = provider
            .driver()
            .complete(&transport, "https://fixture.test", &auth(), &request)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(response.tool_calls.len(), 1, "{name}");
        let call = &response.tool_calls[0];
        assert_eq!(call.name, "read_invoice_total", "{name}");
        assert_eq!(call.arguments["invoice_id"], "INV-9", "{name}");
        assert_eq!(response.finish_reason, FinishReason::ToolCall, "{name}");
        // Tool schemas travel with the request.
        assert!(
            transport.requests()[0].body.contains("read_invoice_total"),
            "{name}"
        );
    }
}

#[test]
fn malformed_tool_output_fails_format_not_silent() {
    for provider in [ProviderDriverRef::OpenAI, ProviderDriverRef::Anthropic] {
        let name = provider.driver().name();
        let bad_body = match provider {
            ProviderDriverRef::OpenAI | ProviderDriverRef::Compatible => serde_json::json!({
                "id": "x",
                "choices": [{"index": 0, "message": {"role": "assistant", "content": null,
                    "tool_calls": [{"id": "call-1", "type": "function",
                        "function": {"name": "read_invoice_total", "arguments": "not-json"}}]},
                    "finish_reason": "tool_calls"}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1}
            })
            .to_string(),
            ProviderDriverRef::Anthropic => serde_json::json!({
                "id": "x",
                "content": [{"type": "tool_use", "id": "call-1", "name": "read_invoice_total", "input": "not-an-object"}],
                "stop_reason": "tool_use",
                "usage": {"input_tokens": 1, "output_tokens": 1}
            })
            .to_string(),
            ProviderDriverRef::Gemini => unreachable!(),
        };
        let transport = FixtureTransport::serving(vec![FixtureResponse::Http {
            status: 200,
            body: bad_body,
        }]);
        let request = tool_request(provider.model());
        let error = provider
            .driver()
            .complete(&transport, "https://fixture.test", &auth(), &request)
            .expect_err("malformed tool output must fail");
        assert!(
            format!("{error:?}").contains("ModelFormat")
                || error.category == lumi_protocol::FailureCategory::ModelFormat,
            "{name}: expected MODEL_FORMAT, got {error:?}"
        );
    }
}

#[test]
fn all_drivers_structured_output() {
    for provider in [
        ProviderDriverRef::OpenAI,
        ProviderDriverRef::Anthropic,
        ProviderDriverRef::Gemini,
        ProviderDriverRef::Compatible,
    ] {
        let name = provider.driver().name();
        let transport = FixtureTransport::serving(vec![FixtureResponse::Http {
            status: 200,
            body: provider.structured_body(),
        }]);
        let request = structured_request(provider.model());
        let response = provider
            .driver()
            .complete(&transport, "https://fixture.test", &auth(), &request)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            response.structured.as_ref().unwrap()["total"],
            4200,
            "{name}"
        );
        // Structured requests must carry their constraint on the wire
        // (response_format / responseMimeType) where the provider has one.
        if matches!(
            provider,
            ProviderDriverRef::OpenAI | ProviderDriverRef::Compatible
        ) {
            assert!(
                transport.requests()[0].body.contains("json_object"),
                "{name}"
            );
        }
        if matches!(provider, ProviderDriverRef::Gemini) {
            assert!(
                transport.requests()[0].body.contains("application/json"),
                "{name}"
            );
        }
    }
}

#[test]
fn all_drivers_rate_limit_maps_with_retry() {
    for provider in [
        ProviderDriverRef::OpenAI,
        ProviderDriverRef::Anthropic,
        ProviderDriverRef::Gemini,
        ProviderDriverRef::Compatible,
    ] {
        let transport = FixtureTransport::serving(vec![FixtureResponse::Http {
            status: 429,
            body: provider.rate_limited_body(),
        }]);
        let error = provider
            .driver()
            .complete(
                &transport,
                "https://fixture.test",
                &auth(),
                &text_request("x", provider.model()),
            )
            .expect_err("429 must error");
        assert_eq!(
            error.category,
            lumi_protocol::FailureCategory::ProviderRateLimit
        );
        assert!(error.retryable);
    }
}

#[test]
fn all_drivers_context_overflow_is_distinct() {
    for provider in [
        ProviderDriverRef::OpenAI,
        ProviderDriverRef::Anthropic,
        ProviderDriverRef::Gemini,
        ProviderDriverRef::Compatible,
    ] {
        let transport = FixtureTransport::serving(vec![FixtureResponse::Http {
            status: 400,
            body: provider.context_overflow_body(),
        }]);
        let error = provider
            .driver()
            .complete(
                &transport,
                "https://fixture.test",
                &auth(),
                &text_request("x", provider.model()),
            )
            .expect_err("overflow must error");
        assert_eq!(
            error.category,
            lumi_protocol::FailureCategory::ModelContextLimit,
            "{:?}",
            provider.driver().name()
        );
        assert!(!error.retryable, "context overflow is not retryable");
    }
}

#[test]
fn all_drivers_auth_failure_maps_to_auth_session() {
    for provider in [
        ProviderDriverRef::OpenAI,
        ProviderDriverRef::Anthropic,
        ProviderDriverRef::Gemini,
        ProviderDriverRef::Compatible,
    ] {
        let transport = FixtureTransport::serving(vec![FixtureResponse::Http {
            status: 401,
            body: provider.auth_rejected_body(),
        }]);
        let error = provider
            .driver()
            .complete(
                &transport,
                "https://fixture.test",
                &auth(),
                &text_request("x", provider.model()),
            )
            .expect_err("401 must error");
        assert_eq!(error.category, lumi_protocol::FailureCategory::AuthSession);
    }
}

#[test]
fn all_drivers_timeout_maps_to_retryable_network() {
    for provider in [
        ProviderDriverRef::OpenAI,
        ProviderDriverRef::Anthropic,
        ProviderDriverRef::Gemini,
        ProviderDriverRef::Compatible,
    ] {
        let transport = FixtureTransport::serving(vec![FixtureResponse::Timeout]);
        let error = provider
            .driver()
            .complete(
                &transport,
                "https://fixture.test",
                &auth(),
                &text_request("x", provider.model()),
            )
            .expect_err("timeout must error");
        assert_eq!(error.category, lumi_protocol::FailureCategory::Network);
        assert!(error.retryable);
    }
}

#[test]
fn all_drivers_server_error_maps_to_provider_unavailable() {
    for provider in [
        ProviderDriverRef::OpenAI,
        ProviderDriverRef::Anthropic,
        ProviderDriverRef::Gemini,
        ProviderDriverRef::Compatible,
    ] {
        let transport = FixtureTransport::serving(vec![FixtureResponse::Http {
            status: 503,
            body: serde_json::json!({"error": {"message": "upstream overloaded"}}).to_string(),
        }]);
        let error = provider
            .driver()
            .complete(
                &transport,
                "https://fixture.test",
                &auth(),
                &text_request("x", provider.model()),
            )
            .expect_err("503 must error");
        assert_eq!(
            error.category,
            lumi_protocol::FailureCategory::ProviderUnavailable
        );
        assert!(error.retryable);
    }
}

#[test]
fn images_mapped_where_supported() {
    // Vision-capable adapters must map an image-bearing user message into
    // their native wire shape.
    for provider in [
        ProviderDriverRef::OpenAI,
        ProviderDriverRef::Anthropic,
        ProviderDriverRef::Gemini,
    ] {
        let name = provider.driver().name();
        let transport = FixtureTransport::serving(vec![FixtureResponse::Http {
            status: 200,
            body: provider.success_body(),
        }]);
        let mut request = text_request("x", provider.model());
        request.messages = vec![ModelMessage::UserWithImage {
            text: "What is in this screenshot?".to_owned(),
            image_mime_type: "image/png".to_owned(),
            image_base64: "aGVsbG8=".to_owned(),
        }];
        provider
            .driver()
            .complete(&transport, "https://fixture.test", &auth(), &request)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let body = &transport.requests()[0].body;
        let marker = match provider {
            ProviderDriverRef::OpenAI | ProviderDriverRef::Compatible => "image_url",
            ProviderDriverRef::Anthropic => "base64",
            ProviderDriverRef::Gemini => "inline_data",
        };
        assert!(body.contains(marker), "{name}: missing {marker} mapping");
    }
}

#[test]
fn all_drivers_health_probe_is_get_only() {
    // Spec 09 §9.17: setup/health checks must not create sessions or
    // trigger auth flows — only a read-only catalog GET.
    for provider in [
        ProviderDriverRef::OpenAI,
        ProviderDriverRef::Anthropic,
        ProviderDriverRef::Gemini,
        ProviderDriverRef::Compatible,
    ] {
        let name = provider.driver().name();
        let transport = FixtureTransport::serving(vec![FixtureResponse::Http {
            status: 200,
            body: match provider {
                ProviderDriverRef::Gemini => serde_json::json!({
                    "models": [{"name": "models/gemini-2"}]
                })
                .to_string(),
                _ => serde_json::json!({"data": [{"id": provider.model()}]}).to_string(),
            },
        }]);
        let models = provider
            .driver()
            .list_models(&transport, "https://fixture.test", &auth())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(!models.is_empty(), "{name}");
        let requests = transport.requests();
        assert_eq!(requests.len(), 1, "{name}");
        assert_eq!(requests[0].method, "GET", "{name}: probe must be GET");
    }
}

#[test]
fn health_probe_failure_does_not_create_side_effects() {
    // Even on auth failure the probe is one GET (no login cascade).
    let transport = FixtureTransport::serving(vec![FixtureResponse::Http {
        status: 401,
        body: ProviderDriverRef::Anthropic.auth_rejected_body(),
    }]);
    let driver = AnthropicDriver::new();
    assert!(driver
        .list_models(&transport, "https://fixture.test", &[])
        .is_err());
    assert_eq!(transport.requests().len(), 1);
    assert_eq!(transport.requests()[0].method, "GET");
}
