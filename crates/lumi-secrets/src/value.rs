//! Secret values with redacted diagnostics and zeroize-on-drop.

use zeroize::Zeroizing;

/// A resolved secret value.
///
/// - `Debug`/`Display` never print the value (redacted marker), so an
///   accidental `println!("{value:?}")` or log line cannot leak it.
/// - Memory is zeroized when the value is dropped (via `zeroize`).
/// - Reading the raw string is an explicit, greppable [`SecretValue::expose`]
///   call at the executor boundary.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretValue {
    inner: Zeroizing<String>,
}

impl SecretValue {
    /// Wraps a raw secret. The input string is copied into zeroizing
    /// storage; the caller's original buffer is its own responsibility.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            inner: Zeroizing::new(value.into()),
        }
    }

    /// Explicit read of the secret value — used at the narrowest executor
    /// boundary only. Grep for `expose(` when auditing value flow.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.inner
    }

    /// Length of the secret in bytes (safe to expose; useful for policy
    /// checks like "non-empty credential").
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// True when the secret is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl std::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretValue([REDACTED])")
    }
}

impl std::fmt::Display for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_and_display_redact() {
        let value = SecretValue::new("hunter2-do-not-leak");
        assert_eq!(format!("{value:?}"), "SecretValue([REDACTED])");
        assert_eq!(format!("{value}"), "[REDACTED]");
        assert!(!format!("{value:?}").contains("hunter2"));
    }

    #[test]
    fn expose_and_length() {
        let value = SecretValue::new("abc");
        assert_eq!(value.expose(), "abc");
        assert_eq!(value.len(), 3);
        assert!(!value.is_empty());
        assert!(SecretValue::new("").is_empty());
    }

    #[test]
    fn clone_writes_zeroizing_copy() {
        let a = SecretValue::new("shared");
        let b = a.clone();
        assert_eq!(a.expose(), b.expose());
    }
}
