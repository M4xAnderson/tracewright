//! The origin map (NS §6.7): where each qualified ID of a scope's flattened net comes from, and
//! the conversion of a checked verdict into the element terms of the IPC contract. The explorers
//! never see it.

use std::collections::BTreeMap;

use tracewright_net::Counterexample;
use tracewright_net::Evidence;
use tracewright_net::Marking;
use tracewright_net::OpenNets;
use tracewright_net::QualifiedId;
use tracewright_net::UnknownReason;
use tracewright_net::Violation;

use crate::check::Verdict;
use crate::dto;
use crate::dto::DeadlockDto;
use crate::dto::DisprovedDto;
use crate::dto::Outcome;
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
        let mut transitions = BTreeMap::new();
        for part in &open.parts {
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
        OriginMap { transitions }
    }

    fn transition(&self, id: &QualifiedId) -> TransitionOrigin {
        self.transitions
            .get(id)
            .cloned()
            .expect("origin: every transition of the flattened net comes from a part or a fusion")
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
            Outcome::Proved(ProvedDto {
                technique: match proof.technique() {
                    tracewright_net::Technique::Exhaustive => dto::Technique::Exhaustive,
                },
                markings,
            })
        }
        Verdict::Disproved(violation) => {
            let Evidence::Counterexample(cx) = violation.evidence();
            Outcome::Disproved(disproved(cx, lowered))
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

fn disproved(cx: &Counterexample, lowered: &Lowered) -> DisprovedDto {
    let origin = OriginMap::new(&lowered.open);
    let frames = cx
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| ReplayFrame {
            step: u32::try_from(index + 1).expect("a counterexample is shorter than its budget"),
            fired: origin.transition(&step.fired),
        })
        .collect();
    let last = cx
        .steps
        .last()
        .map_or(&lowered.flat.initial, |step| &step.marking);
    let Violation::Deadlock { waiting } = &cx.violation;
    let waiting: Vec<WaitingDto> = waiting
        .iter()
        .map(|w| WaitingDto {
            transition: origin.transition(&w.transition),
            waits: w
                .waits
                .iter()
                .map(|wait| WaitDto {
                    place: place_ref(&wait.place),
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

/// The Unknown reason in the terms of the IPC contract.
pub(crate) fn unknown_dto(reason: &UnknownReason) -> UnknownDto {
    match reason {
        UnknownReason::ModelIncomplete(details) => UnknownDto::ModelIncomplete {
            details: details.clone(),
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
        UnknownReason::CapacityExceededFirst { place, .. } => UnknownDto::CapacityExceededFirst {
            place: place.0.clone(),
        },
    }
}
