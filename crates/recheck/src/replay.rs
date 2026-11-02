//! Counterexample replay, the evidence check of NS §9.3 (D-33): every step enabled from the
//! marking before it, every recorded marking right, and the last marking a violation of the
//! property, exactly as recorded.

use tracewright_net::Counterexample;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::Violation;
use tracewright_net::validate;

use crate::semantics::Dense;

/// Why a counterexample does not replay. Steps count from 1, as in NS §9.3.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReplayError {
    /// The net fails NS §2.3.
    #[error("the net is not structurally valid")]
    InvalidNet,
    /// Rule (a): the marking before the step is an overflow marking.
    #[error("step {step}: fires from an overflow marking")]
    FromOverflowMarking {
        /// The step.
        step: usize,
    },
    /// Rule (b): the fired transition is not a transition of the net.
    #[error("step {step}: {transition} is not a transition of the net")]
    UnknownTransition {
        /// The step.
        step: usize,
        /// The recorded transition.
        transition: String,
    },
    /// Rule (c): the fired transition is not enabled.
    #[error("step {step}: {transition} is not enabled")]
    NotEnabled {
        /// The step.
        step: usize,
        /// The recorded transition.
        transition: String,
    },
    /// Rule (e): the marking after the step differs from the recorded one.
    #[error("step {step}: the recorded marking differs")]
    MarkingDiffers {
        /// The step.
        step: usize,
    },
    /// Rule 3: the last marking is not a deadlock.
    #[error("the last marking is not a deadlock")]
    NotADeadlock,
    /// Rule 3: the recorded waiting list is not waits(M, t) for every transition.
    #[error("the recorded waiting list differs")]
    WaitingDiffers,
    /// A count left the range the evidence can carry.
    #[error("step {step}: a token count left u32")]
    Arithmetic {
        /// The step.
        step: usize,
    },
}

/// Replays `cx` on `net` (normally recheck's own composition) for `property`.
///
/// # Errors
///
/// The first rule of NS §9.3 the counterexample breaks.
pub fn replay(net: &Net, cx: &Counterexample, property: &NetProperty) -> Result<(), ReplayError> {
    if validate::net(net).is_err() {
        return Err(ReplayError::InvalidNet);
    }
    let dense = Dense::of(net);
    let mut m = dense.initial.clone();
    for (i, step) in cx.steps.iter().enumerate() {
        let number = i + 1;
        if dense.is_overflow(&m) {
            return Err(ReplayError::FromOverflowMarking { step: number });
        }
        let Some(t) = dense.transitions.iter().position(|id| *id == step.fired) else {
            return Err(ReplayError::UnknownTransition {
                step: number,
                transition: step.fired.0.clone(),
            });
        };
        if !dense.enabled(&m, t) {
            return Err(ReplayError::NotEnabled {
                step: number,
                transition: step.fired.0.clone(),
            });
        }
        m = dense
            .fire(&m, t)
            .ok_or(ReplayError::Arithmetic { step: number })?;
        let recorded = dense
            .to_marking(&m)
            .ok_or(ReplayError::Arithmetic { step: number })?;
        // Equal on every place: `to_marking` omits zeros, so a recorded zero would not compare
        // equal; NS §3.1 lets a marking omit zeros, so drop them before comparing.
        let mut given = step.marking.clone();
        given.0.retain(|_, count| *count > 0);
        if given != recorded {
            return Err(ReplayError::MarkingDiffers { step: number });
        }
    }
    match property {
        NetProperty::DeadlockFree => last_is_deadlock(&dense, &m, &cx.violation),
    }
}

// NS §9.3 rule 3 for deadlock-free with `final none`.
fn last_is_deadlock(dense: &Dense, m: &[u64], violation: &Violation) -> Result<(), ReplayError> {
    if dense.is_overflow(m) {
        return Err(ReplayError::NotADeadlock);
    }
    if (0..dense.transitions.len()).any(|t| dense.enabled(m, t)) {
        return Err(ReplayError::NotADeadlock);
    }
    let Violation::Deadlock { waiting } = violation;
    let expected = dense.waiting(m).ok_or(ReplayError::WaitingDiffers)?;
    if *waiting == expected {
        Ok(())
    } else {
        Err(ReplayError::WaitingDiffers)
    }
}
