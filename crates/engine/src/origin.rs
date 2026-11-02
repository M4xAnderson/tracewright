//! The origin map (NS §6.7): where each qualified ID of a scope's flattened net comes from (an
//! element's place or transition, a fused transition, a relationship's generated place), and the
//! conversion of a checked verdict into the element terms of the IPC contract. The explorers never
//! see it.

use std::collections::BTreeMap;

use tracewright_net::Certificate;
use tracewright_net::Counterexample;
use tracewright_net::Evidence;
use tracewright_net::Marking;
use tracewright_net::OpenNets;
use tracewright_net::QualifiedId;
use tracewright_net::Step;
use tracewright_net::UnknownReason;
use tracewright_net::Violation;

use crate::check::Verdict;
use crate::dto;
use crate::dto::DeadlockDto;
use crate::dto::DisprovedDto;
use crate::dto::NoOverflowPlaceDto;
use crate::dto::Outcome;
use crate::dto::OverflowDto;
use crate::dto::PlaceOrigin;
use crate::dto::PlaceRef;
use crate::dto::ProvedDto;
use crate::dto::ReplayFrame;
use crate::dto::TransitionOrigin;
use crate::dto::TransitionRef;
use crate::dto::UnknownDto;
use crate::dto::UnknownVerdict;
use crate::dto::VerdictDto;
use crate::dto::ViolationDto;
use crate::dto::WaitDto;
use crate::dto::WaitingDto;
use crate::lower::Lowered;
use crate::lower::fused_id;

struct OriginMap {
    places: BTreeMap<QualifiedId, PlaceOrigin>,
    transitions: BTreeMap<QualifiedId, TransitionOrigin>,
}

// An own ID is `element/name` (NS §6.1); lowering builds no other shape for a part.
fn split(id: &QualifiedId) -> (String, String) {
    let (element, name) =
        id.0.split_once('/')
            .expect("lowering gives every own ID the shape element/name");
    (element.to_owned(), name.to_owned())
}

fn transition_ref(id: &QualifiedId) -> TransitionRef {
    let (element, transition) = split(id);
    TransitionRef {
        element,
        transition,
    }
}

fn place_ref(id: &QualifiedId) -> PlaceRef {
    let (element, place) = split(id);
    PlaceRef { element, place }
}

impl OriginMap {
    fn new(open: &OpenNets) -> OriginMap {
        let mut places = BTreeMap::new();
        let mut transitions = BTreeMap::new();
        for link in &open.links {
            places.insert(
                QualifiedId(link.relationship.clone()),
                PlaceOrigin::Generated {
                    relationship: link.relationship.clone(),
                },
            );
        }
        for part in &open.parts {
            for place in &part.places {
                let previous =
                    places.insert(place.id.clone(), PlaceOrigin::Own(place_ref(&place.id)));
                assert!(
                    previous.is_none(),
                    "origin: an own ID never equals a generated ID (it contains '/')"
                );
            }
            for transition in &part.transitions {
                transitions.insert(
                    transition.id.clone(),
                    TransitionOrigin::Own(transition_ref(&transition.id)),
                );
            }
        }
        for fusion in &open.fusions {
            let previous = transitions.insert(
                fused_id(fusion),
                TransitionOrigin::Sync {
                    relationship: fusion.relationship.clone(),
                    caller: transition_ref(&fusion.caller),
                    callee: transition_ref(&fusion.callee),
                },
            );
            assert!(
                previous.is_none(),
                "origin: a fused ID never equals an own ID (it contains '+')"
            );
        }
        OriginMap {
            places,
            transitions,
        }
    }

    fn transition(&self, id: &QualifiedId) -> TransitionOrigin {
        self.transitions
            .get(id)
            .cloned()
            .expect("origin: every transition of the flattened net comes from a part or a fusion")
    }

    fn place(&self, id: &QualifiedId) -> PlaceOrigin {
        self.places
            .get(id)
            .cloned()
            .expect("origin: every place of the flattened net comes from a part or a binding")
    }

    fn frames(&self, steps: &[Step]) -> Vec<ReplayFrame> {
        steps
            .iter()
            .enumerate()
            .map(|(index, step)| ReplayFrame {
                step: u32::try_from(index + 1)
                    .expect("a counterexample is shorter than its budget"),
                fired: self.transition(&step.fired),
            })
            .collect()
    }
}

/// The checked `verdict` of a check on `lowered`, in element terms.
pub(crate) fn verdict_dto(verdict: &Verdict, lowered: &Lowered) -> VerdictDto {
    let outcome = match verdict {
        Verdict::Proved(proof) => {
            let markings = proof.markings();
            assert!(
                markings <= dto::JS_SAFE_INTEGER_MAX,
                "verdict_dto: a marking count stays below 2^53 (IPC-2)"
            );
            let technique = match proof.technique() {
                tracewright_net::Technique::Exhaustive => dto::Technique::Exhaustive,
                tracewright_net::Technique::Certificate(Certificate::NoOverflowPlace {
                    drop_places,
                }) => {
                    let origin = OriginMap::new(&lowered.open);
                    dto::Technique::NoOverflowPlace(NoOverflowPlaceDto {
                        drop_places: drop_places.iter().map(|id| origin.place(id)).collect(),
                    })
                }
            };
            Outcome::Proved(ProvedDto {
                technique,
                markings,
            })
        }
        Verdict::Disproved(violation) => {
            let Evidence::Counterexample(cx) = violation.evidence();
            Outcome::Disproved(disproved(cx, lowered))
        }
        Verdict::Unknown(UnknownReason::CapacityExceededFirst { place, steps }) => {
            Outcome::Unknown(UnknownVerdict {
                reason: capacity_exceeded_first(place, steps, lowered),
            })
        }
        Verdict::Unknown(reason) => Outcome::Unknown(UnknownVerdict {
            reason: unknown_dto(reason),
        }),
    };
    VerdictDto {
        assumptions: lowered
            .assumptions
            .iter()
            .map(ToString::to_string)
            .collect(),
        outcome,
    }
}

// The run of a capacity exceeded first, replayed by recheck, with the place in element terms.
fn capacity_exceeded_first(place: &QualifiedId, steps: &[Step], lowered: &Lowered) -> UnknownDto {
    let origin = OriginMap::new(&lowered.open);
    let capacity = lowered
        .flat
        .places
        .iter()
        .find(|p| p.id == *place)
        .expect("capacity_exceeded_first: the place is a place of the checked net")
        .capacity;
    let count = steps
        .last()
        .and_then(|step| step.marking.0.get(place))
        .copied()
        .expect("capacity_exceeded_first: the replayed run ends above the capacity");
    assert!(
        count > capacity,
        "capacity_exceeded_first: the replayed run ends above the capacity"
    );
    UnknownDto::CapacityExceededFirst {
        place: origin.place(place),
        capacity,
        count,
        frames: origin.frames(steps),
    }
}

fn disproved(cx: &Counterexample, lowered: &Lowered) -> DisprovedDto {
    let origin = OriginMap::new(&lowered.open);
    let frames = origin.frames(&cx.steps);
    let waiting = match &cx.violation {
        Violation::Deadlock { waiting } => waiting,
        Violation::Overflow {
            place,
            capacity,
            count,
        } => {
            assert!(
                count > capacity,
                "disproved: an overflow marking holds more than the capacity"
            );
            return DisprovedDto {
                frames,
                violation: ViolationDto::Overflow(OverflowDto {
                    place: origin.place(place),
                    capacity: *capacity,
                    count: *count,
                }),
            };
        }
    };
    let last = cx
        .steps
        .last()
        .map_or(&lowered.flat.initial, |step| &step.marking);
    let waiting: Vec<WaitingDto> = waiting
        .iter()
        .map(|w| WaitingDto {
            transition: origin.transition(&w.transition),
            waits: w
                .waits
                .iter()
                .map(|wait| WaitDto {
                    place: origin.place(&wait.place),
                    reason: match wait.reason {
                        tracewright_net::WaitReason::NeedsTokens { needed, holds } => {
                            dto::WaitReason::NeedsTokens { needed, holds }
                        }
                        tracewright_net::WaitReason::OutputFull { capacity, .. } => {
                            dto::WaitReason::OutputFull {
                                capacity,
                                holds: last.0.get(&wait.place).copied().unwrap_or(0),
                            }
                        }
                    },
                })
                .collect(),
        })
        .collect();
    assert_eq!(
        waiting.len(),
        lowered.flat.transitions.len(),
        "disproved: in a deadlock every transition waits (NS §7)"
    );
    DisprovedDto {
        frames,
        violation: ViolationDto::Deadlock(DeadlockDto {
            waiting,
            finished: finished(lowered, last),
        }),
    }
}

// NS §9.6, D-23: no place in any transition's pre set holds a token.
fn finished(lowered: &Lowered, marking: &Marking) -> bool {
    lowered.flat.transitions.iter().all(|transition| {
        transition
            .pre
            .keys()
            .all(|place| marking.0.get(place).copied().unwrap_or(0) == 0)
    })
}

/// The Unknown reason in the terms of the IPC contract, for every reason that needs no net:
/// capacity exceeded first is converted by `verdict_dto`, with the checked net.
pub(crate) fn unknown_dto(reason: &UnknownReason) -> UnknownDto {
    match reason {
        UnknownReason::ModelIncomplete(details) => UnknownDto::ModelIncomplete {
            details: details.clone(),
            causes: Vec::new(),
        },
        UnknownReason::PropertyNotCheckable(details) => UnknownDto::PropertyNotCheckable {
            details: details.clone(),
        },
        UnknownReason::InternalDisagreement => UnknownDto::InternalDisagreement,
        UnknownReason::InternalError => UnknownDto::InternalError,
        UnknownReason::Cancelled => UnknownDto::Cancelled,
        UnknownReason::BudgetExhausted { phase, markings } => {
            assert!(
                *markings <= dto::JS_SAFE_INTEGER_MAX,
                "unknown_dto: a marking count stays below 2^53 (IPC-2)"
            );
            UnknownDto::BudgetExhausted {
                phase: match phase {
                    tracewright_net::Phase::Main => dto::Phase::Main,
                    tracewright_net::Phase::Recheck => dto::Phase::Recheck,
                },
                markings: *markings,
            }
        }
        UnknownReason::CapacityExceededFirst { .. } => {
            panic!("unknown_dto: a capacity exceeded first comes only from a check (verdict_dto)")
        }
    }
}
