//! The main explorer. It has its own firing rule, capacity rule and property evaluation and
//! depends only on `tracewright-net` (ADR 0009). Written from `docs/specs/net-semantics.md`
//! §4, §7, §8.1, §10.1 and §12, never from `recheck`.
//!
//! It explores without reductions in the normative order of NS §12: breadth-first from the
//! initial marking, transitions in canonical ID order, the violation test at discovery. So the
//! first deadlock it finds has a shortest counterexample, and its marking count must equal
//! recheck's (NS §9.4).
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::collections::VecDeque;

use tracewright_net::Budget;
use tracewright_net::Counterexample;
use tracewright_net::Evidence;
use tracewright_net::Marking;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::NetVerdict;
use tracewright_net::Phase;
use tracewright_net::QualifiedId;
use tracewright_net::Step;
use tracewright_net::Technique;
use tracewright_net::TransitionWaits;
use tracewright_net::UnknownReason;
use tracewright_net::Violation;
use tracewright_net::Wait;
use tracewright_net::WaitReason;
use tracewright_net::WhenFull;
use tracewright_net::validate;

/// Decides `property` on `net`, storing at most `budget.max_markings` markings.
///
/// An invalid net is Unknown "internal error": engine validates before it hands a net over, so an
/// invalid one here is a bug upstream (NS §2.3).
#[must_use]
pub fn check(net: &Net, property: &NetProperty, budget: &Budget) -> NetVerdict {
    if validate::net(net).is_err() {
        return NetVerdict::Unknown(UnknownReason::InternalError);
    }
    match property {
        NetProperty::DeadlockFree => deadlock_free(&Compiled::new(net), budget.max_markings),
    }
}

// The net with places and transitions sorted by ID and arcs as indexes into the place order.
struct Compiled {
    place_ids: Vec<QualifiedId>,
    capacities: Vec<u32>,
    transitions: Vec<CompiledTransition>,
    initial: Vec<u32>,
}

struct CompiledTransition {
    id: QualifiedId,
    // One entry per place in the pre or post set, sorted by place: (place, pre weight, post
    // weight). Sorted so the waiting list comes out in canonical order (NS §7).
    arcs: Vec<PlaceArcs>,
}

#[derive(Clone, Copy)]
struct PlaceArcs {
    place: usize,
    pre: u32,
    post: u32,
    blocks: bool,
}

impl Compiled {
    fn new(net: &Net) -> Compiled {
        let mut places: Vec<_> = net.places.iter().collect();
        places.sort_by(|a, b| a.id.cmp(&b.id));
        let index: HashMap<&QualifiedId, usize> = places
            .iter()
            .enumerate()
            .map(|(i, place)| (&place.id, i))
            .collect();
        let mut transitions: Vec<_> = net.transitions.iter().collect();
        transitions.sort_by(|a, b| a.id.cmp(&b.id));
        let transitions = transitions
            .into_iter()
            .map(|transition| {
                let mut arcs: Vec<PlaceArcs> = Vec::new();
                let touched = transition.pre.keys().chain(transition.post.keys());
                for place in touched {
                    let i = *index.get(place).expect("validate: every arc names a place");
                    if arcs.iter().all(|arc| arc.place != i) {
                        arcs.push(PlaceArcs {
                            place: i,
                            pre: transition.pre.get(place).copied().unwrap_or(0),
                            post: transition.post.get(place).copied().unwrap_or(0),
                            blocks: match places[i].when_full {
                                WhenFull::Block => true,
                            },
                        });
                    }
                }
                arcs.sort_by_key(|arc| arc.place);
                CompiledTransition {
                    id: transition.id.clone(),
                    arcs,
                }
            })
            .collect();
        let initial = places
            .iter()
            .map(|place| net.initial.0.get(&place.id).copied().unwrap_or(0))
            .collect();
        Compiled {
            place_ids: places.iter().map(|place| place.id.clone()).collect(),
            capacities: places.iter().map(|place| place.capacity).collect(),
            transitions,
            initial,
        }
    }

    // NS §4.1: E2 (enough tokens in every pre place) and E3 (no blocking post place above its
    // capacity after firing). E1 never applies: without overflow places no marking exceeds a
    // capacity.
    fn enabled(&self, marking: &[u32], transition: &CompiledTransition) -> bool {
        transition.arcs.iter().all(|arc| {
            let count = marking[arc.place];
            if count < arc.pre {
                return false;
            }
            if arc.blocks && arc.post > 0 {
                let tentative = u64::from(count) - u64::from(arc.pre) + u64::from(arc.post);
                tentative <= u64::from(self.capacities[arc.place])
            } else {
                true
            }
        })
    }

    // NS §4.2 for block places: remove the pre weights, add the post weights.
    fn fire(&self, marking: &[u32], transition: &CompiledTransition) -> Option<Vec<u32>> {
        let mut next = marking.to_vec();
        for arc in &transition.arcs {
            let count = next[arc.place].checked_sub(arc.pre)?;
            next[arc.place] = count.checked_add(arc.post)?;
            assert!(
                next[arc.place] <= self.capacities[arc.place],
                "fire: block place {} above its capacity",
                self.place_ids[arc.place].0
            );
        }
        Some(next)
    }

    // waits(M, t) of NS §7 for every transition, in canonical order.
    fn waiting(&self, marking: &[u32]) -> Option<Vec<TransitionWaits>> {
        let mut waiting = Vec::with_capacity(self.transitions.len());
        for transition in &self.transitions {
            let mut waits = Vec::new();
            for arc in &transition.arcs {
                let count = marking[arc.place];
                let tentative = i64::from(count) - i64::from(arc.pre) + i64::from(arc.post);
                let capacity = self.capacities[arc.place];
                let reason = if count < arc.pre {
                    Some(WaitReason::NeedsTokens {
                        needed: arc.pre,
                        holds: count,
                    })
                } else if arc.blocks && arc.post > 0 && tentative > i64::from(capacity) {
                    let would_hold = u32::try_from(tentative).ok()?;
                    Some(WaitReason::OutputFull {
                        capacity,
                        would_hold,
                    })
                } else {
                    None
                };
                if let Some(reason) = reason {
                    waits.push(Wait {
                        place: self.place_ids[arc.place].clone(),
                        reason,
                    });
                }
            }
            assert!(
                !waits.is_empty(),
                "waiting: {} is enabled at a deadlock",
                transition.id.0
            );
            waiting.push(TransitionWaits {
                transition: transition.id.clone(),
                waits,
            });
        }
        Some(waiting)
    }

    fn to_marking(&self, counts: &[u32]) -> Marking {
        Marking(
            self.place_ids
                .iter()
                .zip(counts)
                .filter(|(_, count)| **count > 0)
                .map(|(id, count)| (id.clone(), *count))
                .collect(),
        )
    }
}

// The markings discovered so far, in discovery order, with the step that discovered each.
struct Discovered {
    markings: Vec<Vec<u32>>,
    parents: Vec<Option<(usize, usize)>>,
    index: HashMap<Vec<u32>, usize>,
}

impl Discovered {
    fn stored(&self) -> u64 {
        u64::try_from(self.markings.len()).expect("a stored count fits in u64")
    }

    // Follows parent pointers back to the initial marking (iteratively, LIM-3).
    fn run_to(&self, net: &Compiled, mut at: usize) -> Vec<Step> {
        let mut steps = Vec::new();
        while let Some((parent, transition)) = self.parents[at] {
            steps.push(Step {
                fired: net.transitions[transition].id.clone(),
                marking: net.to_marking(&self.markings[at]),
            });
            at = parent;
        }
        assert_eq!(at, 0, "run_to: every run starts at the initial marking");
        steps.reverse();
        steps
    }
}

fn exhausted(markings: u64) -> NetVerdict {
    NetVerdict::Unknown(UnknownReason::BudgetExhausted {
        phase: Phase::Main,
        markings,
    })
}

// NS §8.1 with `final none`: the first marking without an enabled transition is a deadlock.
fn deadlock_free(net: &Compiled, max_markings: u64) -> NetVerdict {
    let mut found = Discovered {
        markings: Vec::new(),
        parents: Vec::new(),
        index: HashMap::new(),
    };
    if let Some(verdict) = discover(net, &mut found, net.initial.clone(), None, max_markings) {
        return verdict;
    }
    let mut queue = VecDeque::from([0]);
    // Every marking enters the queue once and the budget bounds the stored markings (LIM-1).
    while let Some(at) = queue.pop_front() {
        let marking = found.markings[at].clone();
        for (t, transition) in net.transitions.iter().enumerate() {
            if !net.enabled(&marking, transition) {
                continue;
            }
            let Some(successor) = net.fire(&marking, transition) else {
                return NetVerdict::Unknown(UnknownReason::InternalError);
            };
            if found.index.contains_key(&successor) {
                continue;
            }
            if let Some(verdict) = discover(net, &mut found, successor, Some((at, t)), max_markings)
            {
                return verdict;
            }
            queue.push_back(found.markings.len() - 1);
        }
    }
    NetVerdict::Proved {
        technique: Technique::Exhaustive,
        markings: found.stored(),
    }
}

// Stores a new marking and runs the violation test at discovery (NS §12 step 3). Returns the
// verdict that ends the exploration, if any.
fn discover(
    net: &Compiled,
    found: &mut Discovered,
    marking: Vec<u32>,
    parent: Option<(usize, usize)>,
    max_markings: u64,
) -> Option<NetVerdict> {
    assert!(
        !found.index.contains_key(&marking),
        "discover: {marking:?} is already stored"
    );
    // NS §10.1 (b): the budget is checked before every insertion, M0 included.
    if found.stored() >= max_markings {
        return Some(exhausted(found.stored()));
    }
    let at = found.markings.len();
    found.index.insert(marking.clone(), at);
    found.markings.push(marking);
    found.parents.push(parent);
    let current = &found.markings[at];
    if net.transitions.iter().any(|t| net.enabled(current, t)) {
        return None;
    }
    let Some(waiting) = net.waiting(current) else {
        return Some(NetVerdict::Unknown(UnknownReason::InternalError));
    };
    let steps = found.run_to(net, at);
    let violation = Violation::Deadlock { waiting };
    Some(NetVerdict::Disproved(Evidence::Counterexample(
        Counterexample { steps, violation },
    )))
}
