//! Deterministic canonical JSON and SHA-256 digests.
//!
//! Lumi digests (action material digests, error digests, audit hash
//! chains) use `lumi-canonical-json/v1`: RFC 8259 JSON with
//! lexicographically (byte-order) sorted object keys, no insignificant
//! whitespace, minimal string escaping, and shortest-round-trip number
//! formatting.
//!
//! This is deliberately implemented here instead of relying on
//! `serde_json`'s map ordering: the map type can change with the
//! `preserve_order` feature across the dependency graph, which would
//! silently change every digest. Our serializer sorts explicitly. It is
//! not strict RFC 8785 (JCS) — number formatting differs — which is fine
//! for v1 because digests are computed and compared by the same runtime;
//! the algorithm is versioned so any future change re-baselines.

use serde_json::Value;
use sha2::{Digest, Sha256};

/// Serializes `value` to canonical JSON (sorted keys, no insignificant
/// whitespace) regardless of `serde_json`'s map ordering mode.
#[must_use]
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => write_json_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
            out.push('{');
            for (i, (key, val)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json_string(key, out);
                out.push(':');
                write_canonical(val, out);
            }
            out.push('}');
        }
    }
}

fn write_json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// SHA-256 of raw bytes, lowercase hex-encoded.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex_lower(&digest)
}

/// SHA-256 digest over the canonical JSON form of `value`.
#[must_use]
pub fn sha256_canonical(value: &Value) -> String {
    sha256_hex(canonical_json(value).as_bytes())
}

/// Lowercase hex encoding without pulling an extra dependency.
#[must_use]
pub fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // NOTE: dev-dependencies enable serde_json/preserve_order, so these
    // tests deliberately run with insertion-ordered maps. If canonical
    // output stays stable here, the explicit sort is doing its job.

    #[test]
    fn canonical_json_sorts_object_keys_recursively() {
        let v = json!({
            "zebra": 1,
            "alpha": {"y": true, "a": null},
            "mid": [3, {"b": 2, "a": 1}, 2],
        });
        // Arrays preserve order (semantically ordered); only object keys
        // are sorted.
        assert_eq!(
            canonical_json(&v),
            r#"{"alpha":{"a":null,"y":true},"mid":[3,{"a":1,"b":2},2],"zebra":1}"#
        );
    }

    #[test]
    fn canonical_json_escapes_strings_minimally() {
        let v = json!({"s": "quote\" back\\ newline\n tab\t ctrl\u{1}"});
        assert_eq!(
            canonical_json(&v),
            "{\"s\":\"quote\\\" back\\\\ newline\\n tab\\t ctrl\\u0001\"}"
        );
    }

    #[test]
    fn sha256_matches_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn canonical_digest_ignores_key_order() {
        let a = json!({"b": 1, "a": {"d": 2, "c": 3}});
        let b = json!({"a": {"c": 3, "d": 2}, "b": 1});
        assert_eq!(sha256_canonical(&a), sha256_canonical(&b));
    }

    #[test]
    fn canonical_digest_matches_pinned_golden() {
        // Golden value computed independently (python hashlib) over the
        // canonical form {"action":"send_customer_email","risk":"COMMUNICATION"}.
        // An accidental canonicalization change fails loudly here instead
        // of silently invalidating persisted digests.
        let v = json!({"action": "send_customer_email", "risk": "COMMUNICATION"});
        assert_eq!(
            sha256_canonical(&v),
            "df462a91c650e0110b06586778d40565c097e86752e328ac4fe5811ba5b5f0b3"
        );
    }
}
