//! Cooperative cancellation (spec 02 §2.9).
//!
//! Cancellation MUST be prompt, cooperative, and state-preserving: it
//! stops new model/tool work, interrupts cancellable executor work, and
//! never corrupts durable state. The token is a plain atomic flag so any
//! scheduler (sync or async) can observe it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A shareable cancellation token.
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation. Idempotent.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// True when cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Convenience guard for executor loops: returns `Err` when cancelled.
    ///
    /// # Errors
    /// Returns `"cancelled"` when the token has fired.
    pub fn check(&self) -> Result<(), &'static str> {
        if self.is_cancelled() {
            Err("cancelled")
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_shares_state_across_clones() {
        let token = CancelToken::new();
        let observer = token.clone();
        assert!(!observer.is_cancelled());
        token.cancel();
        assert!(observer.is_cancelled());
        assert_eq!(observer.check(), Err("cancelled"));
    }

    #[test]
    fn cancel_is_idempotent() {
        let token = CancelToken::new();
        token.cancel();
        token.cancel();
        assert!(token.is_cancelled());
    }
}
