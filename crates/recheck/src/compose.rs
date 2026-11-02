//! recheck's own composition of open nets (NS §6.3–§6.5): it never sees the engine's flattened
//! net, so the byte comparison of the two encodings checks engine's flattening (ADR 0010). It
//! re-checks what it can see without the model: IDs resolve, L6, L7, L8 and L12, every
//! asynchronous binding set is non-empty, sorted and without duplicates, and NS §2.3 holds on
//! every part and on the result (NS §6.6).

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use tracewright_net::AsyncLink;
use tracewright_net::Net;
use tracewright_net::OpenNets;
use tracewright_net::Place;
use tracewright_net::QualifiedId;
use tracewright_net::Transition;
use tracewright_net::validate;
use tracewright_net::validate::StructuralError;

/// Why recheck refuses to compose an `OpenNets` value. The worker maps any of these to Unknown
/// "internal disagreement": engine accepted what recheck rejects.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ComposeError {
    /// A part fails NS §2.3.
    #[error("part {part} is not structurally valid: {}", lines(.errors))]
    InvalidPart {
        /// Index of the part in `OpenNets::parts`.
        part: usize,
        /// What is wrong with it.
        errors: Vec<StructuralError>,
    },
    /// Two parts have a transition with the same ID.
    #[error("transition {transition} is in more than one part")]
    TransitionInTwoParts {
        /// The transition.
        transition: String,
    },
    /// A fusion or an asynchronous binding names a transition no part has.
    #[error("{relationship} binds {transition}, which is not a transition of any part")]
    UnknownTransition {
        /// The relationship of the fusion.
        relationship: String,
        /// The missing transition.
        transition: String,
    },
    /// L6: caller and callee belong to one part.
    #[error("{relationship} binds {caller} to {callee} (same element)")]
    SameElement {
        /// The relationship of the fusion.
        relationship: String,
        /// The caller.
        caller: String,
        /// The callee.
        callee: String,
    },
    /// L7: a transition is caller in one fusion and callee in another.
    #[error("{transition} is both caller and callee")]
    CallerAndCallee {
        /// The transition.
        transition: String,
    },
    /// L7: a transition is fused and also asynchronously bound.
    #[error("{transition} is both fused and asynchronously bound")]
    FusedAndLinked {
        /// The transition.
        transition: String,
    },
    /// L7: a transition is a member of two asynchronous bindings.
    #[error("{transition} is in more than one asynchronous binding")]
    InTwoLinks {
        /// The transition.
        transition: String,
    },
    /// L8: two asynchronous bindings of one relationship.
    #[error("duplicate generated place {relationship}")]
    DuplicateGeneratedPlace {
        /// The relationship, which is also the generated place's ID.
        relationship: String,
    },
    /// An asynchronous binding with no sender or no receiver.
    #[error("asynchronous binding {relationship} has an empty {side} set")]
    EmptyLinkSet {
        /// The relationship of the binding.
        relationship: String,
        /// `from` or `to`.
        side: &'static str,
    },
    /// An asynchronous binding set that is not strictly sorted by ID (NS §6.1).
    #[error("asynchronous binding {relationship} has a set that is not sorted without duplicates")]
    UnsortedLinkSet {
        /// The relationship of the binding.
        relationship: String,
    },
    /// Adding a generated place's arc to a transition left u32.
    #[error("the arcs of {transition} exceed u32")]
    LinkWeightOverflow {
        /// The transition.
        transition: String,
    },
    /// A bound transition is not own-shaped (`element/name`), so it has no bare name.
    #[error("{transition} has no bare name for a fused ID")]
    NotOwnShaped {
        /// The transition.
        transition: String,
    },
    /// L12: two fusions give the same fused ID.
    #[error("duplicate fused ID {id}")]
    DuplicateFusedId {
        /// The fused ID.
        id: String,
    },
    /// Adding the weights of a caller and a callee left u32.
    #[error("the arcs of {id} exceed u32")]
    WeightOverflow {
        /// The fused ID.
        id: String,
    },
    /// The composed net fails NS §2.3.
    #[error("the composed net is not structurally valid: {}", lines(.errors))]
    InvalidResult {
        /// What is wrong with it.
        errors: Vec<StructuralError>,
    },
}

fn lines(errors: &[StructuralError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// Composes `open` into one net (NS §6.5): every part's places and transitions, one generated
/// place per asynchronous binding with an arc from every sender and to every receiver, then every
/// fusion as one fused transition whose arcs add up the caller's and the callee's, and no fused
/// transition on its own.
///
/// # Errors
///
/// The first rule the input breaks, in the order the module documentation lists them.
#[expect(
    clippy::similar_names,
    reason = "caller and callee are the terms of NS §6.3"
)]
pub fn compose(open: &OpenNets) -> Result<Net, ComposeError> {
    let mut owner = index_transitions(open)?;
    let (generated, linked) = link(open, &mut owner)?;
    let mut callers = BTreeSet::new();
    let mut callees = BTreeSet::new();
    let mut fused: BTreeMap<String, Transition> = BTreeMap::new();
    for fusion in &open.fusions {
        let Some((caller_part, caller)) = owner.get(&fusion.caller) else {
            return Err(unknown(&fusion.relationship, &fusion.caller));
        };
        let Some((callee_part, callee)) = owner.get(&fusion.callee) else {
            return Err(unknown(&fusion.relationship, &fusion.callee));
        };
        if caller_part == callee_part {
            return Err(ComposeError::SameElement {
                relationship: fusion.relationship.clone(),
                caller: fusion.caller.0.clone(),
                callee: fusion.callee.0.clone(),
            });
        }
        callers.insert(&fusion.caller);
        callees.insert(&fusion.callee);
        let id = format!(
            "{}/{}+{}",
            fusion.relationship,
            bare(&fusion.caller)?,
            bare(&fusion.callee)?
        );
        if fused.contains_key(&id) {
            return Err(ComposeError::DuplicateFusedId { id });
        }
        let transition = fuse(QualifiedId(id.clone()), caller, callee)?;
        fused.insert(id, transition);
    }
    if let Some(both) = callers.intersection(&callees).next() {
        return Err(ComposeError::CallerAndCallee {
            transition: both.0.clone(),
        });
    }
    if let Some(both) = callers.union(&callees).find(|id| linked.contains(**id)) {
        return Err(ComposeError::FusedAndLinked {
            transition: both.0.clone(),
        });
    }
    let mut net = Net::default();
    net.places.extend(generated);
    for part in &open.parts {
        net.places.extend(part.places.iter().cloned());
        net.initial.0.extend(
            part.initial
                .0
                .iter()
                .map(|(id, count)| (id.clone(), *count)),
        );
        let unbound = part
            .transitions
            .iter()
            .filter(|t| !callers.contains(&t.id) && !callees.contains(&t.id))
            .map(|t| owner[&t.id].1.clone());
        net.transitions.extend(unbound);
    }
    net.transitions.extend(fused.into_values());
    validate::net(&net).map_err(|errors| ComposeError::InvalidResult { errors })?;
    Ok(net)
}

// Validates each part and maps every transition ID to its part and a copy of the transition,
// which `link` then extends by the generated places' arcs (NS §6.5 step 3).
fn index_transitions(
    open: &OpenNets,
) -> Result<BTreeMap<&QualifiedId, (usize, Transition)>, ComposeError> {
    let mut owner = BTreeMap::new();
    for (part, net) in open.parts.iter().enumerate() {
        validate::net(net).map_err(|errors| ComposeError::InvalidPart { part, errors })?;
        for transition in &net.transitions {
            if owner
                .insert(&transition.id, (part, transition.clone()))
                .is_some()
            {
                return Err(ComposeError::TransitionInTwoParts {
                    transition: transition.id.0.clone(),
                });
            }
        }
    }
    Ok(owner)
}

// NS §6.4: one generated place per binding, `relationship` with its capacity and when full,
// initially empty; W⁺(s, r) += 1 for every sender and W⁻(r, d) += 1 for every receiver. Returns
// the generated places and every transition that is a member of a binding.
fn link<'a>(
    open: &'a OpenNets,
    owner: &mut BTreeMap<&QualifiedId, (usize, Transition)>,
) -> Result<(Vec<Place>, BTreeSet<&'a QualifiedId>), ComposeError> {
    let mut generated: Vec<Place> = Vec::new();
    let mut linked = BTreeSet::new();
    for AsyncLink {
        relationship,
        from,
        to,
        capacity,
        when_full,
    } in &open.links
    {
        if generated.iter().any(|place| place.id.0 == *relationship) {
            return Err(ComposeError::DuplicateGeneratedPlace {
                relationship: relationship.clone(),
            });
        }
        let place = QualifiedId(relationship.clone());
        for (side, members) in [("from", from), ("to", to)] {
            if members.is_empty() {
                return Err(ComposeError::EmptyLinkSet {
                    relationship: relationship.clone(),
                    side,
                });
            }
            if members.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(ComposeError::UnsortedLinkSet {
                    relationship: relationship.clone(),
                });
            }
            for member in members {
                let Some((_, transition)) = owner.get_mut(member) else {
                    return Err(unknown(relationship, member));
                };
                if !linked.insert(member) {
                    return Err(ComposeError::InTwoLinks {
                        transition: member.0.clone(),
                    });
                }
                let arcs = if side == "from" {
                    &mut transition.post
                } else {
                    &mut transition.pre
                };
                let weight = arcs.get(&place).copied().unwrap_or(0).checked_add(1);
                let Some(weight) = weight else {
                    return Err(ComposeError::LinkWeightOverflow {
                        transition: member.0.clone(),
                    });
                };
                arcs.insert(place.clone(), weight);
            }
        }
        generated.push(Place {
            id: place,
            capacity: *capacity,
            when_full: *when_full,
        });
    }
    Ok((generated, linked))
}

fn unknown(relationship: &str, transition: &QualifiedId) -> ComposeError {
    ComposeError::UnknownTransition {
        relationship: relationship.to_owned(),
        transition: transition.0.clone(),
    }
}

// The bare name of an own-shaped ID: the part after `element/`.
fn bare(id: &QualifiedId) -> Result<&str, ComposeError> {
    match id.0.split_once('/') {
        Some((_, name)) if !name.contains('+') => Ok(name),
        _ => Err(ComposeError::NotOwnShaped {
            transition: id.0.clone(),
        }),
    }
}

// NS §6.3: W⁻(p, r/a+c) = W⁻(p, a) + W⁻(p, c) and the same for W⁺.
#[expect(
    clippy::similar_names,
    reason = "caller and callee are the terms of NS §6.3"
)]
fn fuse(
    id: QualifiedId,
    caller: &Transition,
    callee: &Transition,
) -> Result<Transition, ComposeError> {
    let mut fused = Transition {
        id,
        pre: caller.pre.clone(),
        post: caller.post.clone(),
    };
    for (sides, extra) in [
        (&mut fused.pre, &callee.pre),
        (&mut fused.post, &callee.post),
    ] {
        for (place, weight) in extra {
            let sum = sides.get(place).copied().unwrap_or(0).checked_add(*weight);
            let Some(sum) = sum else {
                return Err(ComposeError::WeightOverflow {
                    id: fused.id.0.clone(),
                });
            };
            sides.insert(place.clone(), sum);
        }
    }
    Ok(fused)
}
