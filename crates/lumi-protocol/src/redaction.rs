//! Field-level redaction (spec 03 §3.10) and secret references.
//!
//! Action/observation payloads MUST support redaction before leaving the
//! device or entering long-term audit. Redaction is deterministic: the
//! same input and rules always produce the same output, so redacted values
//! remain digestable within one policy configuration.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Marker substituted for redacted values. Contains no original data.
pub const REDACTED_MARKER: &str = "[REDACTED:lumi-v1]";

/// One redaction rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RedactionRule {
    /// Redact every occurrence of this object key at any depth
    /// (case-sensitive exact match).
    KeyName { key: String },
    /// Redact a specific JSON Pointer path (RFC 6901), e.g.
    /// `/arguments/password`.
    Pointer { pointer: String },
}

impl RedactionRule {
    #[must_use]
    pub fn key(key: impl Into<String>) -> Self {
        Self::KeyName { key: key.into() }
    }

    #[must_use]
    pub fn pointer(pointer: impl Into<String>) -> Self {
        Self::Pointer {
            pointer: pointer.into(),
        }
    }
}

/// Applies redaction rules to `value` (recursively for key rules) and
/// returns the redacted copy. The input is not mutated.
#[must_use]
pub fn redact_value(value: &Value, rules: &[RedactionRule]) -> Value {
    let pointers: Vec<&String> = rules
        .iter()
        .filter_map(|r| match r {
            RedactionRule::Pointer { pointer } => Some(pointer),
            _ => None,
        })
        .collect();
    let keys: Vec<&String> = rules
        .iter()
        .filter_map(|r| match r {
            RedactionRule::KeyName { key } => Some(key),
            _ => None,
        })
        .collect();
    let mut out = value.clone();
    for pointer in &pointers {
        if let Some(target) = pointer_target_mut(&mut out, pointer) {
            *target = Value::String(REDACTED_MARKER.to_owned());
        }
    }
    redact_keys_recursive(&mut out, &keys);
    out
}

fn redact_keys_recursive(value: &mut Value, keys: &[&String]) {
    match value {
        Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                if keys.contains(&k) {
                    *v = Value::String(REDACTED_MARKER.to_owned());
                } else {
                    redact_keys_recursive(v, keys);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_keys_recursive(item, keys);
            }
        }
        _ => {}
    }
}

/// Resolves a JSON Pointer to a mutable target, creating nothing.
fn pointer_target_mut<'a>(value: &'a mut Value, pointer: &str) -> Option<&'a mut Value> {
    if pointer.is_empty() {
        return Some(value);
    }
    if !pointer.starts_with('/') {
        return None;
    }
    let tokens: Vec<String> = pointer[1..]
        .split('/')
        .map(|t| t.replace("~1", "/").replace("~0", "~"))
        .collect();
    let mut current = value;
    for token in tokens {
        current = match current {
            Value::Object(map) => map.get_mut(&token)?,
            Value::Array(items) => {
                let index: usize = token.parse().ok()?;
                items.get_mut(index)?
            }
            _ => return None,
        };
    }
    Some(current)
}

/// A reference to a credential held in the runtime's secret broker.
///
/// Secret VALUES never appear in prompts, audit logs, traces, screenshots,
/// or workflow definitions (AGENTS.md). Only the reference name does; the
/// broker resolves it at the narrowest executor boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SecretRef(pub String);

impl SecretRef {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl std::fmt::Display for SecretRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Displaying the *reference* is safe; values are never held here.
        write!(f, "secret_ref({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn key_rules_redact_at_any_depth() {
        let v = json!({
            "arguments": {
                "password": "hunter2",
                "nested": {"api_key": "sk-123", "visible": "ok"},
                "keep": 1
            },
            "password": "top",
        });
        let redacted = redact_value(
            &v,
            &[
                RedactionRule::key("password"),
                RedactionRule::key("api_key"),
            ],
        );
        assert_eq!(
            redacted,
            json!({
                "arguments": {
                    "password": REDACTED_MARKER,
                    "nested": {"api_key": REDACTED_MARKER, "visible": "ok"},
                    "keep": 1
                },
                "password": REDACTED_MARKER,
            })
        );
        // Input not mutated.
        assert_eq!(v["arguments"]["password"], "hunter2");
    }

    #[test]
    fn pointer_rules_redact_exact_paths() {
        let v = json!({"a": {"b": [{"secret": 1}, {"secret": 2}]}});
        let redacted = redact_value(&v, &[RedactionRule::pointer("/a/b/0/secret")]);
        assert_eq!(redacted["a"]["b"][0]["secret"], REDACTED_MARKER);
        assert_eq!(redacted["a"]["b"][1]["secret"], 2);
    }

    #[test]
    fn pointer_escapes_are_honored() {
        let v = json!({"a/b": {"c~d": "s"}});
        let redacted = redact_value(&v, &[RedactionRule::pointer("/a~1b/c~0d")]);
        assert_eq!(redacted["a/b"]["c~d"], REDACTED_MARKER);
    }

    #[test]
    fn arrays_are_walked_by_key_rules() {
        let v = json!({"items": [{"token": "t1"}, {"token": "t2"}]});
        let redacted = redact_value(&v, &[RedactionRule::key("token")]);
        assert_eq!(redacted["items"][0]["token"], REDACTED_MARKER);
        assert_eq!(redacted["items"][1]["token"], REDACTED_MARKER);
    }

    #[test]
    fn secret_ref_display_leaks_nothing_but_the_name() {
        let r = SecretRef::new("erp-production");
        assert_eq!(r.to_string(), "secret_ref(erp-production)");
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, "\"erp-production\"");
    }
}
