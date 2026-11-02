//! The main explorer. It has its own firing rule, capacity rule and property evaluation and
//! depends only on `tracewright-net` (ADR 0009). Written from `docs/specs/net-semantics.md`
//! §3.2, §4, §7, §8.1, §8.2, §9.5, §10.1 and §12, never from `recheck`.
//!
//! It explores without reductions in the normative order of NS §12: breadth-first from the
//! initial marking, transitions in canonical ID order, the violation test at discovery. So the
//! first violation it finds has a shortest counterexample, and its marking count must equal
//! recheck's (NS §9.4). Overflow markings are stored (for de-duplication and the budget) but
//! never expanded: they have no successors (NS §4.4).
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::atomic::Ordering;

use tracewright_net::Budget;
use tracewright_net::Certificate;
use tracewright_net::Counterexample;
use tracewright_net::Evidence;
use tracewright_net::Marking;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::NetVerdict;
use tracewright_net::POLL_EXPANSIONS;
use tracewright_net::Phase;
use tracewright_net::Progress;
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

/// Decides `property` on `net` within `budget`: at most `budget.max_markings` stored markings,
/// and the deadline and the cancel flag polled every [`POLL_EXPANSIONS`] expansions (NS §10.1).
/// No capacity exceeded on a net without an overflow place is Proved by the `NoOverflowPlace`
/// certificate without exploring (NS §8.2), so it never polls.
///
/// An invalid net is Unknown "internal error": engine validates before it hands a net over, so an
/// invalid one here is a bug upstream (NS §2.3).
#[must_use]
pub fn check(net: &Net, property: &NetProperty, budget: &Budget) -> NetVerdict {
    check_reporting(net, property, budget, &mut |_progress| {})
}

/// [`check`], calling `progress` at every poll with the markings stored so far.
#[must_use]
pub fn check_reporting(
    net: &Net,
    property: &NetProperty,
    budget: &Budget,
    progress: &mut dyn FnMut(Progress),
) -> NetVerdict {
    if validate::net(net).is_err() {
        return NetVerdict::Unknown(UnknownReason::InternalError);
    }
    let compiled = Compiled::new(net);
    match property {
        NetProperty::DeadlockFree => explore(&compiled, property, budget, progress),
        NetProperty::CapacityNeverExceeded => {
            if compiled.when_full.contains(&WhenFull::Overflow) {
                explore(&compiled, property, budget, progress)
            } else {
                no_overflow_place(&compiled)
            }
        }
    }
}

// NS §8.2, §9.5: without an overflow place no marking exceeds a capacity (§3.2), so the property
// holds without exploring; the certificate names the drop places (D-03).
fn no_overflow_place(net: &Compiled) -> NetVerdict {
    assert!(
        !net.when_full.contains(&WhenFull::Overflow),
        "no_overflow_place: the net has an overflow place"
    );
    let drop_places: Vec<QualifiedId> = net
        .place_ids
        .iter()
        .zip(&net.when_full)
        .filter(|(_, when_full)| **when_full == WhenFull::Drop)
        .map(|(id, _)| id.clone())
        .collect();
    assert!(
        drop_places.is_sorted(),
        "no_overflow_place: places are in canonical order"
    );
    NetVerdict::Proved {
        technique: Technique::Certificate(Certificate::NoOverflowPlace { drop_places }),
        markings: 0,
    }
}

// The net with places and transitions sorted by ID and arcs as indexes into the place order.
struct Compiled {
    place_ids: Vec<QualifiedId>,
    capacities: Vec<u32>,
    when_full: Vec<WhenFull>,
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
                            blocks: places[i].when_full == WhenFull::Block,
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
            when_full: places.iter().map(|place| place.when_full).collect(),
            transitions,
            initial,
        }
    }

    // NS §3.2: some place holds more than its capacity.
    fn is_overflow(&self, marking: &[u32]) -> bool {
        marking
            .iter()
            .zip(&self.capacities)
            .any(|(count, capacity)| count > capacity)
    }

    // The smallest-ID place above its capacity, with its capacity and count (NS §8.2).
    fn exceeded(&self, marking: &[u32]) -> Option<(QualifiedId, u32, u32)> {
        let place = (0..marking.len()).find(|p| marking[*p] > self.capacities[*p])?;
        Some((
            self.place_ids[place].clone(),
            self.capacities[place],
            marking[place],
        ))
    }

    // NS §4.1: E1 (not an overflow marking), E2 (enough tokens in every pre place) and E3 (no
    // blocking post place above its capacity after firing).
    fn enabled(&self, marking: &[u32], transition: &CompiledTransition) -> bool {
        if self.is_overflow(marking) {
            return false;
        }
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

    // NS §4.2: remove the pre weights, add the post weights; a drop place then keeps at most its
    // capacity, an overflow place may end above it. None only if a count leaves u32, which rule 7
    // of NS §2.3 rules out on a valid net.
    fn fire(&self, marking: &[u32], transition: &CompiledTransition) -> Option<Vec<u32>> {
        let mut next = marking.to_vec();
        for arc in &transition.arcs {
            let tentative = next[arc.place]
                .checked_sub(arc.pre)?
                .checked_add(arc.post)?;
            let capacity = self.capacities[arc.place];
            next[arc.place] = match self.when_full[arc.place] {
                WhenFull::Block => {
                    assert!(
                        tentative <= capacity,
                        "fire: block place {} above its capacity",
                        self.place_ids[arc.place].0
                    );
                    tentative
                }
                WhenFull::Drop => tentative.min(capacity),
                WhenFull::Overflow => tentative,
            };
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

// The markings discovered so far (R ∪ O), in discovery order, with the step that discovered each.
struct Discovered {
    markings: Vec<Vec<u32>>,
    parents: Vec<Option<(usize, usize)>>,
    index: HashMap<Vec<u32>, usize>,
    // The first overflow marking in discovery order, and how many there are (|O|).
    first_overflow: Option<usize>,
    overflow_count: u64,
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

// NS §10.1: the cancel flag first (it wins at the same poll), then the deadline. Returns the
// verdict that ends the phase, if any.
fn poll(budget: &Budget, stored: u64) -> Option<NetVerdict> {
    assert!(
        stored <= budget.max_markings,
        "poll: stored past the budget"
    );
    if budget.cancel.load(Ordering::Relaxed) {
        return Some(NetVerdict::Unknown(UnknownReason::Cancelled));
    }
    let deadline = budget.deadline.as_ref()?;
    if deadline.clock.now() >= deadline.at {
        return Some(exhausted(stored));
    }
    None
}

// The breadth-first exploration of NS §12 for deadlock-free (NS §8.1, `final none`) and no
// capacity exceeded (NS §8.2). The first violation in discovery order ends it; a complete
// exploration is Proved if no overflow marking was found, and for deadlock-free otherwise Unknown
// "capacity exceeded first" with the first overflow marking.
fn explore(
    net: &Compiled,
    property: &NetProperty,
    budget: &Budget,
    progress: &mut dyn FnMut(Progress),
) -> NetVerdict {
    let max_markings = budget.max_markings;
    let mut found = Discovered {
        markings: Vec::new(),
        parents: Vec::new(),
        index: HashMap::new(),
        first_overflow: None,
        overflow_count: 0,
    };
    if let Some(verdict) = discover(
        net,
        property,
        &mut found,
        net.initial.clone(),
        None,
        max_markings,
    ) {
        return verdict;
    }
    let mut queue = VecDeque::from([0]);
    let mut expansions: u64 = 0;
    // Every in-bounds marking enters the queue once and the budget bounds the stored markings
    // (LIM-1).
    while let Some(at) = queue.pop_front() {
        if expansions.is_multiple_of(POLL_EXPANSIONS) {
            if let Some(verdict) = poll(budget, found.stored()) {
                return verdict;
            }
            progress(Progress {
                markings: found.stored(),
            });
        }
        expansions += 1;
        let marking = found.markings[at].clone();
        assert!(
            !net.is_overflow(&marking),
            "explore: an overflow marking is never expanded"
        );
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
            let overflow = net.is_overflow(&successor);
            if let Some(verdict) = discover(
                net,
                property,
                &mut found,
                successor,
                Some((at, t)),
                max_markings,
            ) {
                return verdict;
            }
            if !overflow {
                queue.push_back(found.markings.len() - 1);
            }
        }
    }
    complete(net, property, &found)
}

// The verdict of a complete exploration without a violation: Proved with |R| if no overflow
// marking was found, else (deadlock-free only) capacity exceeded first at the first one (NS §8.1).
fn complete(net: &Compiled, property: &NetProperty, found: &Discovered) -> NetVerdict {
    let reachable = found.stored() - found.overflow_count;
    match found.first_overflow {
        None => NetVerdict::Proved {
            technique: Technique::Exhaustive,
            markings: reachable,
        },
        Some(first) => {
            assert!(
                matches!(property, NetProperty::DeadlockFree),
                "complete: no capacity exceeded stops at the first overflow marking"
            );
            let (place, _, _) = net
                .exceeded(&found.markings[first])
                .expect("complete: an overflow marking exceeds a capacity");
            NetVerdict::Unknown(UnknownReason::CapacityExceededFirst {
                place,
                steps: found.run_to(net, first),
            })
        }
    }
}

// Stores a new marking and runs the violation test at discovery (NS §12 step 3). Returns the
// verdict that ends the exploration, if any.
fn discover(
    net: &Compiled,
    property: &NetProperty,
    found: &mut Discovered,
    marking: Vec<u32>,
    parent: Option<(usize, usize)>,
    max_markings: u64,
) -> Option<NetVerdict> {
    assert!(
        !found.index.contains_key(&marking),
        "discover: {marking:?} is already stored"
    );
    // NS §10.1 (b): the budget is checked before every insertion, M0 and overflow markings
    // included.
    if found.stored() >= max_markings {
        return Some(exhausted(found.stored()));
    }
    let at = found.markings.len();
    found.index.insert(marking.clone(), at);
    found.markings.push(marking);
    found.parents.push(parent);
    let current = &found.markings[at];
    if let Some((place, capacity, count)) = net.exceeded(current) {
        // NS §3.2: an overflow marking is neither a deadlock nor final; it violates only no
        // capacity exceeded.
        found.overflow_count += 1;
        if found.first_overflow.is_none() {
            found.first_overflow = Some(at);
        }
        return match property {
            NetProperty::DeadlockFree => None,
            NetProperty::CapacityNeverExceeded => {
                let steps = found.run_to(net, at);
                assert!(
                    !steps.is_empty(),
                    "discover: the initial marking is in bounds (NS §2.3 rule 4)"
                );
                let violation = Violation::Overflow {
                    place,
                    capacity,
                    count,
                };
                Some(NetVerdict::Disproved(Evidence::Counterexample(
                    Counterexample { steps, violation },
                )))
            }
        };
    }
    match property {
        NetProperty::CapacityNeverExceeded => return None,
        NetProperty::DeadlockFree => {}
    }
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
