//! The check pipeline (NS §9.2, design §4) and the only verdict the app stores, caches, renders
//! or converts to a DTO (VER-1, D-53).
//!
//! `verify` explores the engine's flattened net; `recheck` composes the open nets itself,
//! compares the two encodings, re-explores to confirm a Proved and replays a counterexample to
//! confirm a Disproved. `CheckedProof` and `CheckedViolation` have private fields and are built
//! only in this module, after that confirmation: there is no other path to Proved or Disproved.

use tracewright_net::Budget;
use tracewright_net::Evidence;
use tracewright_net::NetProperty;
use tracewright_net::NetVerdict;
use tracewright_net::Phase;
use tracewright_net::Technique;
use tracewright_net::UnknownReason;
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

/// Checks `property` on `lowered` within `budget`: verify gets ⌊N/2⌋ markings and recheck the rest
/// (NS §10.1).
///
/// # Panics
///
/// Only on a bug: the budget split gave recheck less than verify.
#[must_use]
pub fn run(lowered: &Lowered, property: &NetProperty, budget: &Budget) -> Checked {
    let verdict = pipeline(lowered, property, budget.max_markings);
    Checked {
        key: check_key(&lowered.flat, property),
        verdict,
    }
}

fn pipeline(lowered: &Lowered, property: &NetProperty, max_markings: u64) -> Verdict {
    // Step 0: recheck's own composition must encode to the same bytes as the engine's.
    let Ok(own) = tracewright_recheck::compose(&lowered.open) else {
        return Verdict::Unknown(UnknownReason::InternalDisagreement);
    };
    if encode::canonical(&own) != encode::canonical(&lowered.flat) {
        return Verdict::Unknown(UnknownReason::InternalDisagreement);
    }
    let main = Budget {
        max_markings: max_markings / 2,
    };
    let rest = Budget {
        max_markings: max_markings - main.max_markings,
    };
    assert!(
        rest.max_markings >= main.max_markings,
        "run: recheck gets at least verify's half (NS §10.1)"
    );
    match tracewright_verify::check(&lowered.flat, property, &main) {
        NetVerdict::Proved {
            technique,
            markings,
        } => match technique {
            // Step 2: agreement of the plain BFS on recheck's own composition (NS §9.4).
            Technique::Exhaustive => match tracewright_recheck::explore(&own, property, &rest) {
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
                NetVerdict::Unknown(UnknownReason::Cancelled) => {
                    Verdict::Unknown(UnknownReason::Cancelled)
                }
                NetVerdict::Proved { .. } | NetVerdict::Disproved(_) | NetVerdict::Unknown(_) => {
                    Verdict::Unknown(UnknownReason::InternalDisagreement)
                }
            },
        },
        // Step 3: replay on recheck's own composition (NS §9.3); a failed replay is an internal
        // error, not a disagreement (D-24).
        NetVerdict::Disproved(evidence) => match &evidence {
            Evidence::Counterexample(cx) => match tracewright_recheck::replay(&own, cx, property) {
                Ok(()) => Verdict::Disproved(CheckedViolation { evidence }),
                Err(_replay) => Verdict::Unknown(UnknownReason::InternalError),
            },
        },
        // Step 4: an Unknown from verify is final.
        NetVerdict::Unknown(reason) => Verdict::Unknown(reason),
    }
}
