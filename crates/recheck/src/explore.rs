//! The plain breadth-first exploration of NS §12, the deadlock-free decision of NS §8.1 with
//! `final none` and no capacity exceeded (NS §8.2). No reductions, ever (ADR 0010). recheck never
//! emits a certificate: on a net without an overflow place it explores, and its Proved is the
//! exhaustive one.

use std::collections::BTreeMap;
use std::collections::VecDeque;

use tracewright_net::Budget;
use tracewright_net::Counterexample;
use tracewright_net::Evidence;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::NetVerdict;
use tracewright_net::Phase;
use tracewright_net::Step;
use tracewright_net::Technique;
use tracewright_net::UnknownReason;
use tracewright_net::Violation;
use tracewright_net::validate;

use crate::semantics::Dense;

/// Decides `property` on `net` (normally recheck's own composition), storing at most
/// `budget.max_markings` markings. Its outcome must agree with `verify::check` (NS §9.4).
#[must_use]
pub fn explore(net: &Net, property: &NetProperty, budget: &Budget) -> NetVerdict {
    if validate::net(net).is_err() {
        return NetVerdict::Unknown(UnknownReason::InternalError);
    }
    let dense = Dense::of(net);
    Search::new(&dense, property, budget.max_markings).run()
}

struct Search<'a> {
    net: &'a Dense,
    property: &'a NetProperty,
    max_markings: u64,
    /// Discovery number of every stored marking: R and O together.
    seen: BTreeMap<Vec<u64>, usize>,
    /// The stored markings by discovery number.
    order: Vec<Found>,
    /// Discovery numbers of the overflow markings O, in discovery order.
    overflows: Vec<usize>,
}

struct Found {
    marking: Vec<u64>,
    /// The predecessor's discovery number and the transition that led here; `None` for M0.
    parent: Option<(usize, usize)>,
}

enum Outcome {
    Stored(usize),
    Ends(NetVerdict),
}

impl<'a> Search<'a> {
    fn new(net: &'a Dense, property: &'a NetProperty, max_markings: u64) -> Search<'a> {
        Search {
            net,
            property,
            max_markings,
            seen: BTreeMap::new(),
            order: Vec::new(),
            overflows: Vec::new(),
        }
    }

    fn run(mut self) -> NetVerdict {
        let mut queue = VecDeque::new();
        match self.discover(self.net.initial.clone(), None) {
            Outcome::Stored(i) => queue.push_back(i),
            Outcome::Ends(verdict) => return verdict,
        }
        // Bounded: each stored marking is queued once and `discover` caps the stored ones (LIM-1).
        while let Some(i) = queue.pop_front() {
            let m = self.order[i].marking.clone();
            for t in 0..self.net.transitions.len() {
                if !self.net.enabled(&m, t) {
                    continue;
                }
                let Some(successor) = self.net.fire(&m, t) else {
                    return NetVerdict::Unknown(UnknownReason::InternalError);
                };
                if self.seen.contains_key(&successor) {
                    continue;
                }
                // An overflow marking is stored and tested, never enqueued (NS §12 step 2).
                let expand = !self.net.is_overflow(&successor);
                match self.discover(successor, Some((i, t))) {
                    Outcome::Stored(j) if expand => queue.push_back(j),
                    Outcome::Stored(_) => {}
                    Outcome::Ends(verdict) => return verdict,
                }
            }
        }
        let stored = self.order.len();
        let reachable = stored - self.overflows.len();
        let Some(&first) = self.overflows.first() else {
            let markings = u64::try_from(reachable).expect("a marking count fits in u64");
            return NetVerdict::Proved {
                technique: Technique::Exhaustive,
                markings,
            };
        };
        // NS §8.1: complete, no deadlock, O ≠ ∅. No capacity exceeded never gets here: it ends
        // at its first overflow marking.
        assert!(
            matches!(self.property, NetProperty::DeadlockFree),
            "explore: only deadlock-free completes with an overflow marking"
        );
        let place = self
            .net
            .smallest_exceeded(&self.order[first].marking)
            .map(|(p, _, _)| self.net.places[p].clone());
        match (place, self.run_to(first)) {
            (Some(place), Some(steps)) => {
                NetVerdict::Unknown(UnknownReason::CapacityExceededFirst { place, steps })
            }
            _ => NetVerdict::Unknown(UnknownReason::InternalError),
        }
    }

    // NS §10.1 (b) and §12 step 3: check the budget, store, then test at discovery.
    fn discover(&mut self, m: Vec<u64>, parent: Option<(usize, usize)>) -> Outcome {
        let stored = u64::try_from(self.order.len()).expect("a marking count fits in u64");
        if stored == self.max_markings {
            let reason = UnknownReason::BudgetExhausted {
                phase: Phase::Recheck,
                markings: stored,
            };
            return Outcome::Ends(NetVerdict::Unknown(reason));
        }
        assert!(
            stored < self.max_markings,
            "discover: stored past the budget"
        );
        let i = self.order.len();
        self.seen.insert(m.clone(), i);
        self.order.push(Found { marking: m, parent });
        let m = &self.order[i].marking;
        let violation = if self.net.is_overflow(m) {
            self.overflows.push(i);
            match self.property {
                // NS §3.2: an overflow marking is never a deadlock.
                NetProperty::DeadlockFree => return Outcome::Stored(i),
                NetProperty::CapacityNeverExceeded => {
                    let Some((p, capacity, count)) = self.net.smallest_exceeded(m) else {
                        return Outcome::Ends(NetVerdict::Unknown(UnknownReason::InternalError));
                    };
                    Violation::Overflow {
                        place: self.net.places[p].clone(),
                        capacity,
                        count,
                    }
                }
            }
        } else {
            match self.property {
                NetProperty::CapacityNeverExceeded => return Outcome::Stored(i),
                NetProperty::DeadlockFree => {
                    if (0..self.net.transitions.len()).any(|t| self.net.enabled(m, t)) {
                        return Outcome::Stored(i);
                    }
                    let Some(waiting) = self.net.waiting(m) else {
                        return Outcome::Ends(NetVerdict::Unknown(UnknownReason::InternalError));
                    };
                    Violation::Deadlock { waiting }
                }
            }
        };
        match self.run_to(i) {
            Some(steps) => Outcome::Ends(NetVerdict::Disproved(Evidence::Counterexample(
                Counterexample { steps, violation },
            ))),
            None => Outcome::Ends(NetVerdict::Unknown(UnknownReason::InternalError)),
        }
    }

    /// The run from M0 to the marking discovered as number `last`.
    fn run_to(&self, last: usize) -> Option<Vec<Step>> {
        let mut steps = Vec::new();
        let mut at = last;
        // Parent pointers lead strictly to earlier discoveries, so this ends at M0 (LIM-3).
        while let Some((parent, t)) = self.order[at].parent {
            assert!(
                parent < at,
                "counterexample: a parent is discovered before its child"
            );
            steps.push(Step {
                fired: self.net.transitions[t].clone(),
                marking: self.net.to_marking(&self.order[at].marking)?,
            });
            at = parent;
        }
        steps.reverse();
        Some(steps)
    }
}
