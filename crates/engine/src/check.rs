//! The check pipeline (NS §9.2, design §4) and the only verdict the app stores, caches, renders
//! or converts to a DTO (VER-1, D-53).
//!
//! `verify` explores the engine's flattened net; `recheck` composes the open nets itself,
//! compares the two encodings, re-explores or checks the certificate to confirm a Proved, and
//! replays a counterexample to confirm a Disproved or the run of a capacity exceeded first.
//! `CheckedProof` and `CheckedViolation` have private fields and are built only in this module,
//! after that confirmation: there is no other path to Proved or Disproved.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tracewright_net::Budget;
use tracewright_net::Clock;
use tracewright_net::Counterexample;
use tracewright_net::Deadline;
use tracewright_net::Evidence;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::NetVerdict;
use tracewright_net::Phase;
use tracewright_net::Progress;
use tracewright_net::QualifiedId;
use tracewright_net::Step;
use tracewright_net::Technique;
use tracewright_net::UnknownReason;
use tracewright_net::Violation;
use tracewright_net::encode;

use crate::key::CheckKey;
use crate::key::check_key;
use crate::lower::Lowered;

/// A Proved whose evidence recheck confirmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedProof {
    technique: Technique,
    markings: u64,
}

impl CheckedProof {
    /// How the property was proved.
    #[must_use]
    pub fn technique(&self) -> &Technique {
        &self.technique
    }

    /// The number of reachable markings both explorers counted.
    #[must_use]
    pub fn markings(&self) -> u64 {
        self.markings
    }
}

/// A Disproved whose evidence recheck replayed on its own composition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedViolation {
    evidence: Evidence,
}

impl CheckedViolation {
    /// The replayed evidence.
    #[must_use]
    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }
}

/// The outcome of a check: exactly three verdicts (VER-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The property holds, confirmed by the second explorer.
    Proved(CheckedProof),
    /// The property does not hold, with replayed evidence.
    Disproved(CheckedViolation),
    /// The property could not be decided, for exactly one reason.
    Unknown(UnknownReason),
}

/// What one check returns: the key of the net it explored and its verdict (NS §9.2 step 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// Recomputed from the net the worker explored.
    pub key: CheckKey,
    /// The verdict.
    pub verdict: Verdict,
}

/// The engine's markings budget per check until `arch.toml`'s `[budget]` is read: the
/// `max_markings` of the file-format §5 example. Spike E1 measured 0.2–1.3 million markings per
/// second per thread, so it stays in the seconds range.
pub const DEFAULT_MAX_MARKINGS: u64 = 2_000_000;

/// The engine's time budget per check: the 2 s target of the E1 spike and the `time_ms` of the
/// file-format §5 example.
pub const DEFAULT_TIME: Duration = Duration::from_secs(2);

/// What one check may spend before the split (NS §10.1): N markings and T of time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckBudget {
    /// N: the main phase gets ⌊N/2⌋ markings, the recheck phase the rest.
    pub max_markings: u64,
    /// T: the main phase ends at t₀ + T/2, the recheck phase at t₀ + T.
    pub time: Duration,
}

impl Default for CheckBudget {
    fn default() -> CheckBudget {
        CheckBudget {
            max_markings: DEFAULT_MAX_MARKINGS,
            time: DEFAULT_TIME,
        }
    }
}

/// Checks `property` on `lowered` within `budget`, measured on `clock` from the moment it starts
/// (t₀): verify gets ⌊N/2⌋ markings until t₀ + T/2, recheck N − ⌊N/2⌋ markings until t₀ + T, so
/// the time verify does not use carries over and its markings do not (NS §10.1). `progress`
/// hears each phase's stored markings at every poll.
///
/// # Panics
///
/// Only on a bug: the budget split gave recheck less than verify.
#[must_use]
pub fn run(
    lowered: &Lowered,
    property: &NetProperty,
    budget: &CheckBudget,
    clock: &Arc<dyn Clock>,
    progress: &mut dyn FnMut(Phase, Progress),
) -> Checked {
    let verdict = pipeline(lowered, property, &split(budget, clock), progress);
    Checked {
        key: check_key(&lowered.flat, property),
        verdict,
    }
}

// The two phase budgets of NS §10.1, sharing one cancel flag.
struct Split {
    main: Budget,
    rest: Budget,
}

fn split(budget: &CheckBudget, clock: &Arc<dyn Clock>) -> Split {
    let t0 = clock.now();
    let deadline = |after: Duration| Deadline {
        at: t0
            .checked_add(after)
            .expect("a time budget of at most 2^31 ms fits after any Instant"),
        clock: Arc::clone(clock),
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let main = Budget {
        max_markings: budget.max_markings / 2,
        deadline: Some(deadline(budget.time / 2)),
        cancel: Arc::clone(&cancel),
    };
    let rest = Budget {
        max_markings: budget.max_markings - main.max_markings,
        deadline: Some(deadline(budget.time)),
        cancel,
    };
    assert!(
        rest.max_markings >= main.max_markings,
        "split: recheck gets at least verify's half of the markings (NS §10.1)"
    );
    Split { main, rest }
}

fn pipeline(
    lowered: &Lowered,
    property: &NetProperty,
    budgets: &Split,
    progress: &mut dyn FnMut(Phase, Progress),
) -> Verdict {
    // Step 0: recheck's own composition must encode to the same bytes as the engine's.
    let Ok(own) = tracewright_recheck::compose(&lowered.open) else {
        return Verdict::Unknown(UnknownReason::InternalDisagreement);
    };
    if encode::canonical(&own) != encode::canonical(&lowered.flat) {
        return Verdict::Unknown(UnknownReason::InternalDisagreement);
    }
    let candidate = tracewright_verify::check_reporting(
        &lowered.flat,
        property,
        &budgets.main,
        &mut |stored| progress(Phase::Main, stored),
    );
    confirm(&own, property, candidate, &budgets.rest, progress)
}

// Steps 2 to 4 of NS §9.2: recheck confirms the main explorer's candidate on its own composition
// `own`, or the check ends as Unknown.
fn confirm(
    own: &Net,
    property: &NetProperty,
    candidate: NetVerdict,
    rest: &Budget,
    progress: &mut dyn FnMut(Phase, Progress),
) -> Verdict {
    // Replay and the certificate check store nothing and ignore the deadline; they check the
    // cancel flag once before they start (NS §10.1).
    let cancelled = rest.cancel.load(Ordering::Relaxed);
    match candidate {
        NetVerdict::Proved {
            technique: Technique::Exhaustive,
            markings,
        } => agree(own, property, markings, rest, progress),
        NetVerdict::Proved { .. }
        | NetVerdict::Disproved(_)
        | NetVerdict::Unknown(UnknownReason::CapacityExceededFirst { .. })
            if cancelled =>
        {
            Verdict::Unknown(UnknownReason::Cancelled)
        }
        // Step 2, the certificate path of ADR 0008: recheck checks it on its own composition; a
        // rejection is a disagreement (NS §9.4).
        NetVerdict::Proved {
            technique: Technique::Certificate(certificate),
            markings,
        } => {
            if markings == 0 && tracewright_recheck::check_certificate(own, property, &certificate)
            {
                Verdict::Proved(CheckedProof {
                    technique: Technique::Certificate(certificate),
                    markings,
                })
            } else {
                Verdict::Unknown(UnknownReason::InternalDisagreement)
            }
        }
        // Step 3: replay on recheck's own composition (NS §9.3); a failed replay is an internal
        // error, not a disagreement (D-24).
        NetVerdict::Disproved(evidence) => match &evidence {
            Evidence::Counterexample(cx) => match tracewright_recheck::replay(own, cx, property) {
                Ok(()) => Verdict::Disproved(CheckedViolation { evidence }),
                Err(_replay) => Verdict::Unknown(UnknownReason::InternalError),
            },
        },
        // Step 4: an Unknown from verify is final, except that the run of a capacity exceeded
        // first is replayed like a no capacity exceeded counterexample (NS §10.2, VER-2).
        NetVerdict::Unknown(UnknownReason::CapacityExceededFirst { place, steps }) => {
            let replayed = overflow_run(own, place.clone(), steps.clone()).is_some_and(|cx| {
                tracewright_recheck::replay(own, &cx, &NetProperty::CapacityNeverExceeded).is_ok()
            });
            if replayed {
                Verdict::Unknown(UnknownReason::CapacityExceededFirst { place, steps })
            } else {
                Verdict::Unknown(UnknownReason::InternalError)
            }
        }
        NetVerdict::Unknown(reason) => Verdict::Unknown(reason),
    }
}

// Step 2 for a Proved by exhaustion: the plain BFS on recheck's own composition must agree on the
// marking count (NS §9.4).
fn agree(
    own: &Net,
    property: &NetProperty,
    markings: u64,
    rest: &Budget,
    progress: &mut dyn FnMut(Phase, Progress),
) -> Verdict {
    let second = tracewright_recheck::explore_reporting(own, property, rest, &mut |stored| {
        progress(Phase::Recheck, stored);
    });
    match second {
        NetVerdict::Proved {
            technique: Technique::Exhaustive,
            markings: confirmed,
        } if confirmed == markings => Verdict::Proved(CheckedProof {
            technique: Technique::Exhaustive,
            markings,
        }),
        NetVerdict::Unknown(
            reason @ UnknownReason::BudgetExhausted {
                phase: Phase::Recheck,
                ..
            },
        ) => Verdict::Unknown(reason),
        NetVerdict::Unknown(UnknownReason::Cancelled) => Verdict::Unknown(UnknownReason::Cancelled),
        NetVerdict::Proved { .. } | NetVerdict::Disproved(_) | NetVerdict::Unknown(_) => {
            Verdict::Unknown(UnknownReason::InternalDisagreement)
        }
    }
}

// The run of a capacity exceeded first as the no capacity exceeded counterexample it is: its last
// marking exceeds `place`. The capacity is read from recheck's composition, so a run that names a
// place the composition lacks does not replay.
fn overflow_run(own: &Net, place: QualifiedId, steps: Vec<Step>) -> Option<Counterexample> {
    let capacity = own.places.iter().find(|p| p.id == place)?.capacity;
    let count = *steps.last()?.marking.0.get(&place)?;
    Some(Counterexample {
        steps,
        violation: Violation::Overflow {
            place,
            capacity,
            count,
        },
    })
}

// The confirmation step with a made-up main-explorer candidate: the only way to show what happens
// when verify is wrong, since the real one agrees with recheck on every net the tests can build.
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tracewright_net::Certificate;
    use tracewright_net::Marking;
    use tracewright_net::Place;
    use tracewright_net::Transition;
    use tracewright_net::WhenFull;

    use super::*;

    fn q(id: &str) -> QualifiedId {
        QualifiedId(id.to_owned())
    }

    // POOL of NS §14: three clients share two workers; 3 reachable markings, no deadlock.
    fn pool() -> Net {
        let place = |id: &str, capacity| Place {
            id: q(id),
            capacity,
            when_full: WhenFull::Block,
        };
        Net {
            places: vec![
                place("c/idle", 3),
                place("c/running", 3),
                place("w/free", 2),
            ],
            transitions: vec![
                Transition {
                    id: q("r1/start+acquire"),
                    pre: BTreeMap::from([(q("c/idle"), 1), (q("w/free"), 1)]),
                    post: BTreeMap::from([(q("c/running"), 1)]),
                },
                Transition {
                    id: q("r2/finish+release"),
                    pre: BTreeMap::from([(q("c/running"), 1)]),
                    post: BTreeMap::from([(q("c/idle"), 1), (q("w/free"), 1)]),
                },
            ],
            initial: Marking(BTreeMap::from([(q("c/idle"), 3), (q("w/free"), 2)])),
        }
    }

    fn confirmed(candidate: NetVerdict) -> Verdict {
        let rest = Budget {
            max_markings: 1_000,
            ..Budget::default()
        };
        confirm(
            &pool(),
            &NetProperty::DeadlockFree,
            candidate,
            &rest,
            &mut |_, _| {},
        )
    }

    fn proved(markings: u64) -> NetVerdict {
        NetVerdict::Proved {
            technique: Technique::Exhaustive,
            markings,
        }
    }

    #[test]
    fn a_proved_candidate_that_recheck_counts_the_same_is_proved() {
        let Verdict::Proved(proof) = confirmed(proved(3)) else {
            panic!("recheck agrees on 3 markings");
        };
        assert_eq!(proof.markings(), 3);
    }

    #[test]
    fn explorers_that_count_different_markings_are_an_internal_disagreement() {
        assert_eq!(
            confirmed(proved(4)),
            Verdict::Unknown(UnknownReason::InternalDisagreement)
        );
    }

    #[test]
    fn a_counterexample_that_fails_replay_is_an_internal_error() {
        // D-24: the initial marking of POOL is no deadlock, so replay rejects this run.
        let cx = Counterexample {
            steps: Vec::new(),
            violation: Violation::Deadlock {
                waiting: Vec::new(),
            },
        };
        assert_eq!(
            confirmed(NetVerdict::Disproved(Evidence::Counterexample(cx))),
            Verdict::Unknown(UnknownReason::InternalError)
        );
    }

    #[test]
    fn a_counterexample_with_a_wrong_marking_is_an_internal_error() {
        let cx = Counterexample {
            steps: vec![Step {
                fired: q("r1/start+acquire"),
                marking: Marking(BTreeMap::from([(q("c/idle"), 3)])),
            }],
            violation: Violation::Deadlock {
                waiting: Vec::new(),
            },
        };
        assert_eq!(
            confirmed(NetVerdict::Disproved(Evidence::Counterexample(cx))),
            Verdict::Unknown(UnknownReason::InternalError)
        );
    }

    #[test]
    fn replay_checks_the_cancel_flag_once_before_it_starts() {
        let cx = Counterexample {
            steps: Vec::new(),
            violation: Violation::Deadlock {
                waiting: Vec::new(),
            },
        };
        let rest = Budget {
            max_markings: 1_000,
            cancel: Arc::new(AtomicBool::new(true)),
            ..Budget::default()
        };
        let verdict = confirm(
            &pool(),
            &NetProperty::DeadlockFree,
            NetVerdict::Disproved(Evidence::Counterexample(cx)),
            &rest,
            &mut |_, _| {},
        );
        assert_eq!(verdict, Verdict::Unknown(UnknownReason::Cancelled));
    }

    fn certificate(drop_places: &[&str]) -> NetVerdict {
        NetVerdict::Proved {
            technique: Technique::Certificate(Certificate::NoOverflowPlace {
                drop_places: drop_places.iter().map(|id| q(id)).collect(),
            }),
            markings: 0,
        }
    }

    fn confirmed_capacity(candidate: NetVerdict) -> Verdict {
        let rest = Budget {
            max_markings: 1_000,
            ..Budget::default()
        };
        confirm(
            &pool(),
            &NetProperty::CapacityNeverExceeded,
            candidate,
            &rest,
            &mut |_, _| {},
        )
    }

    #[test]
    fn a_certificate_recheck_accepts_is_proved_with_no_markings() {
        let Verdict::Proved(proof) = confirmed_capacity(certificate(&[])) else {
            panic!("POOL has no overflow place");
        };
        assert_eq!(proof.markings(), 0);
    }

    #[test]
    fn a_certificate_recheck_rejects_is_an_internal_disagreement() {
        // POOL has no drop place, so a certificate that names one is wrong.
        assert_eq!(
            confirmed_capacity(certificate(&["w/free"])),
            Verdict::Unknown(UnknownReason::InternalDisagreement)
        );
    }

    #[test]
    fn a_capacity_exceeded_first_whose_run_does_not_replay_is_an_internal_error() {
        // POOL never exceeds a capacity, so no run of it ends in an overflow marking.
        let candidate = NetVerdict::Unknown(UnknownReason::CapacityExceededFirst {
            place: q("c/running"),
            steps: vec![Step {
                fired: q("r1/start+acquire"),
                marking: Marking(BTreeMap::from([
                    (q("c/idle"), 2),
                    (q("c/running"), 1),
                    (q("w/free"), 1),
                ])),
            }],
        });
        assert_eq!(
            confirmed(candidate),
            Verdict::Unknown(UnknownReason::InternalError)
        );
    }
}
