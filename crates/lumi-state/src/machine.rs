//! Validated task state transitions (spec 02 §2.4).
//!
//! The runtime cannot corrupt task lifecycle state by construction:
//! transitions outside the allowed graph are refused.

use lumi_protocol::TaskStatus;

/// Error returned for a disallowed transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionError {
    /// The target state is not reachable from the current state.
    Disallowed { from: TaskStatus, to: TaskStatus },
    /// Terminal states accept no further transitions.
    Terminal { state: TaskStatus },
    /// AMBIGUOUS -> COMPLETED requires explicit verification evidence.
    AmbiguousCompletionRequiresVerification,
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disallowed { from, to } => {
                write!(f, "transition {from:?} -> {to:?} is not allowed")
            }
            Self::Terminal { state } => write!(f, "state {state:?} is terminal"),
            Self::AmbiguousCompletionRequiresVerification => write!(
                f,
                "AMBIGUOUS -> COMPLETED is only allowed after explicit verification"
            ),
        }
    }
}

impl std::error::Error for TransitionError {}

/// Validates a task-status transition per spec 02 §2.4.
///
/// `verified` marks that explicit verification evidence exists; it is
/// required for AMBIGUOUS -> COMPLETED (never for RUNNING -> COMPLETED,
/// where the caller is responsible for having run required verification).
///
/// # Errors
/// Returns the specific [`TransitionError`] when disallowed.
pub fn transition_allowed(
    from: TaskStatus,
    to: TaskStatus,
    verified: bool,
) -> Result<(), TransitionError> {
    use TaskStatus::*;
    if from.is_terminal() {
        return Err(TransitionError::Terminal { state: from });
    }
    let allowed = match from {
        Created => matches!(to, Queued | Cancelled | Failed),
        Queued => matches!(to, Running | Cancelled | Failed),
        Running => matches!(
            to,
            WaitingApproval
                | WaitingUser
                | WaitingExternal
                | Paused
                | Completed
                | Failed
                | Ambiguous
                | Cancelled
        ),
        WaitingApproval | WaitingUser | WaitingExternal => {
            matches!(to, Running | Cancelled | Failed)
        }
        Paused => matches!(to, Running | Cancelled),
        Ambiguous => match to {
            WaitingUser | Failed => true,
            Completed if verified => true,
            Completed => {
                return Err(TransitionError::AmbiguousCompletionRequiresVerification);
            }
            _ => false,
        },
        Completed | Failed | Cancelled => false,
    };
    if allowed {
        Ok(())
    } else {
        Err(TransitionError::Disallowed { from, to })
    }
}

/// A task bound to the transition rules.
#[derive(Debug, Clone)]
pub struct TaskStateMachine {
    pub status: TaskStatus,
}

impl TaskStateMachine {
    #[must_use]
    pub const fn new(status: TaskStatus) -> Self {
        Self { status }
    }

    /// Attempts a transition; on success the internal status moves.
    ///
    /// # Errors
    /// [`TransitionError`] when the transition is not allowed.
    pub fn transition(
        &mut self,
        to: TaskStatus,
        verified: bool,
    ) -> Result<TaskStatus, TransitionError> {
        transition_allowed(self.status, to, verified)?;
        self.status = to;
        Ok(to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TaskStatus::*;

    #[test]
    fn happy_path_is_allowed() {
        let mut sm = TaskStateMachine::new(Created);
        sm.transition(Queued, false).unwrap();
        sm.transition(Running, false).unwrap();
        sm.transition(Completed, false).unwrap();
    }

    #[test]
    fn waiting_states_return_to_running_or_terminate() {
        for waiting in [WaitingApproval, WaitingUser, WaitingExternal] {
            let mut sm = TaskStateMachine::new(Running);
            sm.transition(waiting, false).unwrap();
            sm.transition(Running, false).unwrap();
            let mut sm2 = TaskStateMachine::new(waiting);
            sm2.transition(Cancelled, false).unwrap();
            let mut sm3 = TaskStateMachine::new(waiting);
            sm3.transition(Paused, false).unwrap_err();
        }
    }

    #[test]
    fn ambiguous_completion_requires_verification() {
        let mut sm = TaskStateMachine::new(Running);
        sm.transition(Ambiguous, false).unwrap();
        assert_eq!(
            sm.transition(Completed, false).unwrap_err(),
            TransitionError::AmbiguousCompletionRequiresVerification
        );
        sm.transition(Completed, true).unwrap();
    }

    #[test]
    fn ambiguous_cannot_go_straight_to_running() {
        let mut sm = TaskStateMachine::new(Running);
        sm.transition(Ambiguous, false).unwrap();
        assert!(sm.transition(Running, false).is_err());
        // But it can route to a human or fail.
        let mut sm2 = TaskStateMachine::new(Ambiguous);
        sm2.transition(WaitingUser, false).unwrap();
        let mut sm3 = TaskStateMachine::new(Ambiguous);
        sm3.transition(Failed, false).unwrap();
    }

    #[test]
    fn terminal_states_are_frozen() {
        for terminal in [Completed, Failed, Cancelled] {
            let mut sm = TaskStateMachine::new(terminal);
            assert!(matches!(
                sm.transition(Running, false),
                Err(TransitionError::Terminal { .. })
            ));
        }
    }

    #[test]
    fn paused_only_resumes_or_cancels() {
        let mut sm = TaskStateMachine::new(Paused);
        sm.transition(Completed, true).unwrap_err();
        sm.transition(Running, false).unwrap();
        let mut sm2 = TaskStateMachine::new(Paused);
        sm2.transition(Cancelled, false).unwrap();
    }

    #[test]
    fn created_cannot_skip_queue_into_completed() {
        let mut sm = TaskStateMachine::new(Created);
        assert!(sm.transition(Completed, true).is_err());
    }
}
