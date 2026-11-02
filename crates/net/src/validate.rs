//! Structural validity of a net (NS §2.3). Engine validates every part and the flattened net
//! before a check (L10); `verify` and `recheck` validate again on entry, because they treat their
//! input as untrusted.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::Net;
use crate::QualifiedId;

/// One way a net is not structurally valid. The messages are the NS §6.6 L10 detail lines, with
/// qualified IDs.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StructuralError {
    /// Rule 1: the ID is none of the three shapes of NS §2.2.
    #[error("{id} is not a well-formed qualified ID")]
    MalformedId {
        /// The offending ID.
        id: String,
    },
    /// Rule 1: places and transitions share one namespace.
    #[error("{id} names more than one place or transition")]
    DuplicateId {
        /// The ID used twice.
        id: String,
    },
    /// Rule 2: an arc names a place the net does not have.
    #[error("{transition} has an arc to {place}, which is not a place of this net")]
    UnknownArcPlace {
        /// The transition with the arc.
        transition: String,
        /// The missing place.
        place: String,
    },
    /// Rule 2: every arc has a weight of at least 1.
    #[error("{transition} has an arc of weight 0 to {place}")]
    ZeroWeight {
        /// The transition with the arc.
        transition: String,
        /// The place of the arc.
        place: String,
    },
    /// Rule 3: every capacity is at least 1.
    #[error("{place}: capacity 0")]
    ZeroCapacity {
        /// The place.
        place: String,
    },
    /// The initial marking names a place the net does not have.
    #[error("the initial marking names {place}, which is not a place of this net")]
    UnknownInitialPlace {
        /// The missing place.
        place: String,
    },
    /// Rule 4: the initial marking is in bounds (V29).
    #[error("{place}: initial {initial} exceeds capacity {capacity}")]
    InitialAboveCapacity {
        /// The place.
        place: String,
        /// Its initial count.
        initial: u32,
        /// Its capacity.
        capacity: u32,
    },
    /// Rule 7: the largest tentative count fits in u32 (V54).
    #[error("{place}: capacity {capacity} plus largest post weight {weight} exceeds u32")]
    CapacityPlusWeight {
        /// The place.
        place: String,
        /// Its capacity.
        capacity: u32,
        /// The largest post weight into it.
        weight: u32,
    },
}

/// Checks rules 1–4 and 7 of NS §2.3 (rule 5 has nothing to check while the only property is
/// deadlock-free with `final none`; rule 6 holds by the types).
///
/// # Errors
///
/// Every violated rule, ordered by rule and then by ID.
pub fn net(net: &Net) -> Result<(), Vec<StructuralError>> {
    let mut errors = Vec::new();
    check_ids(net, &mut errors);
    let capacities: BTreeMap<&QualifiedId, u32> = net
        .places
        .iter()
        .map(|place| (&place.id, place.capacity))
        .collect();
    let mut transitions: Vec<_> = net.transitions.iter().collect();
    transitions.sort_by(|a, b| a.id.cmp(&b.id));
    let mut largest_post: BTreeMap<&QualifiedId, u32> = BTreeMap::new();
    for transition in transitions {
        for (place, weight) in transition.pre.iter().chain(&transition.post) {
            if !capacities.contains_key(place) {
                errors.push(StructuralError::UnknownArcPlace {
                    transition: transition.id.0.clone(),
                    place: place.0.clone(),
                });
            } else if *weight == 0 {
                errors.push(StructuralError::ZeroWeight {
                    transition: transition.id.0.clone(),
                    place: place.0.clone(),
                });
            }
        }
        for (place, weight) in &transition.post {
            let largest = largest_post.entry(place).or_insert(0);
            *largest = (*largest).max(*weight);
        }
    }
    for (place, capacity) in &capacities {
        if *capacity == 0 {
            errors.push(StructuralError::ZeroCapacity {
                place: place.0.clone(),
            });
        }
    }
    for (place, initial) in &net.initial.0 {
        match capacities.get(place) {
            None => errors.push(StructuralError::UnknownInitialPlace {
                place: place.0.clone(),
            }),
            Some(capacity) => {
                if initial > capacity {
                    errors.push(StructuralError::InitialAboveCapacity {
                        place: place.0.clone(),
                        initial: *initial,
                        capacity: *capacity,
                    });
                }
            }
        }
    }
    for (place, weight) in largest_post {
        if let Some(capacity) = capacities.get(place) {
            if capacity.checked_add(weight).is_none() {
                errors.push(StructuralError::CapacityPlusWeight {
                    place: place.0.clone(),
                    capacity: *capacity,
                    weight,
                });
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// Rule 1.
fn check_ids(net: &Net, errors: &mut Vec<StructuralError>) {
    let mut seen: BTreeSet<&QualifiedId> = BTreeSet::new();
    let mut duplicates: BTreeSet<&QualifiedId> = BTreeSet::new();
    let all = net.places.iter().map(|place| &place.id);
    for id in all.chain(net.transitions.iter().map(|transition| &transition.id)) {
        if !seen.insert(id) {
            duplicates.insert(id);
        }
    }
    for id in &seen {
        if !is_qualified_id(&id.0) {
            errors.push(StructuralError::MalformedId { id: id.0.clone() });
        }
    }
    for id in duplicates {
        errors.push(StructuralError::DuplicateId { id: id.0.clone() });
    }
}

/// Whether `id` has one of the three shapes of NS §2.2: `slug` (generated), `slug/slug` (own) or
/// `slug/slug+slug` (fused).
#[must_use]
pub fn is_qualified_id(id: &str) -> bool {
    let Some((prefix, rest)) = id.split_once('/') else {
        return is_slug(id);
    };
    if !is_slug(prefix) {
        return false;
    }
    match rest.split_once('+') {
        Some((left, right)) => is_slug(left) && is_slug(right),
        None => is_slug(rest),
    }
}

/// Whether `s` is a slug of NS §2.2: `[A-Za-z_][A-Za-z0-9_-]*` and not `and`, `or` or `not`.
#[must_use]
pub fn is_slug(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return false;
    }
    !matches!(s, "and" | "or" | "not")
}
