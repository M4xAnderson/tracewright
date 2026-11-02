//! The plain breadth-first exploration of NS §12 and the deadlock-free decision of NS §8.1 with
//! `final none`. No reductions, ever (ADR 0010).

use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::sync::atomic::Ordering;

use tracewright_net::Budget;
use tracewright_net::Counterexample;
use tracewright_net::Evidence;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::NetVerdict;
use tracewright_net::POLL_EXPANSIONS;
use tracewright_net::Phase;
use tracewright_net::Progress;
use tracewright_net::Step;
use tracewright_net::Technique;
use tracewright_net::UnknownReason;
use tracewright_net::Violation;
use tracewright_net::validate;

use crate::semantics::Dense;

/// Decides `property` on `net` (normally recheck's own composition) within `budget` (NS §10.1).
/// Its outcome must agree with `verify::check` (NS §9.4).
#[must_use]
pub fn explore(net: &Net, property: &NetProperty, budget: &Budget) -> NetVerdict {
    explore_reporting(net, property, budget, &mut |_progress| {})
}

/// [`explore`], calling `progress` with the stored markings each time it polls the budget.
#[must_use]
pub fn explore_reporting(
    net: &Net,
    property: &NetProperty,
    budget: &Budget,
    progress: &mut dyn FnMut(Progress),
) -> NetVerdict {
    if validate::net(net).is_err() {
        return NetVerdict::Unknown(UnknownReason::InternalError);
    }
    match property {
        NetProperty::DeadlockFree => {
            let dense = Dense::of(net);
            Search::new(&dense, budget).deadlock_free(progress)
        }
    }
}

struct Search<'a> {
    net: &'a Dense,
    budget: &'a Budget,
    max_markings: u64,
    /// Discovery number of every stored marking; the reachable set R.
    seen: BTreeMap<Vec<u64>, usize>,
    /// The stored markings by discovery number.
    order: Vec<Found>,
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
    fn new(net: &'a Dense, budget: &'a Budget) -> Search<'a> {
        Search {
            net,
            budget,
            max_markings: budget.max_markings,
            seen: BTreeMap::new(),
            order: Vec::new(),
        }
    }

    fn stored(&self) -> u64 {
        u64::try_from(self.order.len()).expect("a marking count fits in u64")
    }

    // NS §10.1, time and cancel: the flag wins when both hold at one poll.
    fn poll(&self) -> Option<NetVerdict> {
        assert!(
            self.stored() <= self.max_markings,
            "poll: more markings stored than the budget allows"
        );
        if self.budget.cancel.load(Ordering::Relaxed) {
            return Some(NetVerdict::Unknown(UnknownReason::Cancelled));
        }
        if let Some(deadline) = &self.budget.deadline {
            if deadline.clock.now() >= deadline.at {
                let reason = UnknownReason::BudgetExhausted {
                    phase: Phase::Recheck,
                    markings: self.stored(),
                };
                return Some(NetVerdict::Unknown(reason));
            }
        }
        None
    }

    fn deadlock_free(mut self, progress: &mut dyn FnMut(Progress)) -> NetVerdict {
        let mut queue = VecDeque::new();
        match self.discover(self.net.initial.clone(), None) {
            Outcome::Stored(i) => queue.push_back(i),
            Outcome::Ends(verdict) => return verdict,
        }
        let mut expansions: u64 = 0;
        // Bounded: each stored marking is queued once and `discover` caps the stored ones (LIM-1).
        while let Some(i) = queue.pop_front() {
            // The first expansion polls, then every POLL_EXPANSIONS-th one.
            if expansions.is_multiple_of(POLL_EXPANSIONS) {
                if let Some(verdict) = self.poll() {
                    return verdict;
                }
                progress(Progress {
                    markings: self.stored(),
                });
            }
            expansions += 1;
            let m = self.order[i].marking.clone();
            for t in 0..self.net.transitions.len() {
                if !self.net.enabled(&m, t) {
                    continue;
                }
                let Some(successor) = self.net.fire(&m, t) else {
                    return NetVerdict::Unknown(UnknownReason::InternalError);
                };
                assert!(
                    !self.net.is_overflow(&successor),
                    "explore: a block-only net overflowed"
                );
                if self.seen.contains_key(&successor) {
                    continue;
                }
                match self.discover(successor, Some((i, t))) {
                    Outcome::Stored(j) => queue.push_back(j),
                    Outcome::Ends(verdict) => return verdict,
                }
            }
        }
        let markings = u64::try_from(self.order.len()).expect("a marking count fits in u64");
        NetVerdict::Proved {
            technique: Technique::Exhaustive,
            markings,
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
        let is_deadlock = (0..self.net.transitions.len()).all(|t| !self.net.enabled(m, t));
        if !is_deadlock {
            return Outcome::Stored(i);
        }
        match self.counterexample(i) {
            Some(cx) => Outcome::Ends(NetVerdict::Disproved(Evidence::Counterexample(cx))),
            None => Outcome::Ends(NetVerdict::Unknown(UnknownReason::InternalError)),
        }
    }

    fn counterexample(&self, last: usize) -> Option<Counterexample> {
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
        let waiting = self.net.waiting(&self.order[last].marking)?;
        Some(Counterexample {
            steps,
            violation: Violation::Deadlock { waiting },
        })
    }
}
