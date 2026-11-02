//! Lowering: Model → `OpenNets` → flattened `Net` (NS §6). Total or fails: every failing
//! precondition becomes one line of Unknown "model incomplete", and a scope that does not lower
//! never gets a smaller net (design §6).

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use tracewright_model::Behavior;
use tracewright_model::BehaviorModel;
use tracewright_model::Binding;
use tracewright_model::Element;
use tracewright_model::Model;
use tracewright_model::PetriNet;
use tracewright_model::PortRef;
use tracewright_model::RelationshipId;
use tracewright_model::Scope;
use tracewright_model::WhenFull;
use tracewright_net::Fusion;
use tracewright_net::Net;
use tracewright_net::OpenNets;
use tracewright_net::Place;
use tracewright_net::QualifiedId;
use tracewright_net::Transition;
use tracewright_net::validate;
use tracewright_net::validate::StructuralError;

/// A scope lowered to the input of a check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lowered {
    /// The parts and bindings; recheck composes them itself (ADR 0010).
    pub open: OpenNets,
    /// The engine's composition of `open`; verify explores it and the `CheckKey` hashes it.
    pub flat: Net,
    /// Relationships of the scope's children that take no part in the check (NS §6.6). Never
    /// part of the key.
    pub assumptions: Vec<RelationshipId>,
}

/// Why a scope does not lower.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LowerError {
    /// No child (or, for an own scope, the element) has a behavior model: no verdict at all.
    NothingToCheck,
    /// A precondition of NS §6.6 fails; one line per cause, ordered by rule, then by ID.
    ModelIncomplete(Vec<String>),
    /// The scope uses net features this build does not check yet (drop and overflow places,
    /// asynchronous bindings); their tickets replace this with the real lowering.
    NotCheckedYet(Vec<String>),
}

/// Why `flatten` refuses an `OpenNets` value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FlattenError {
    /// A fusion names a transition no part has.
    #[error("{transition} not found")]
    UnknownTransition {
        /// The transition.
        transition: String,
    },
    /// Adding a caller's and a callee's weights left u32.
    #[error("the arcs of {transition} exceed u32")]
    WeightOverflow {
        /// The fused transition.
        transition: String,
    },
    /// The flattened net fails NS §2.3.
    #[error("the flattened net is not structurally valid")]
    Invalid(Vec<StructuralError>),
}

// Precondition failures, kept with their rule number and sort key (NS §6.6: ordered by rule,
// then by canonical ID).
#[derive(Default)]
struct Lines {
    incomplete: BTreeSet<(u8, String, String)>,
    not_checked: BTreeSet<String>,
}

impl Lines {
    fn add(&mut self, rule: u8, id: &str, line: String) {
        self.incomplete.insert((rule, id.to_owned(), line));
    }
}

fn qualified(element: &Element, name: &str) -> String {
    format!("{}/{name}", element.id)
}

/// Lowers one scope of `model` (NS §6.6).
///
/// # Errors
///
/// `NothingToCheck`, `ModelIncomplete` with every failing precondition, or `NotCheckedYet`.
///
/// # Panics
///
/// Only on a bug: a scope that passed every precondition flattened to an invalid net, or one
/// that failed without a line.
pub fn lower(model: &Model, scope: &Scope) -> Result<Lowered, LowerError> {
    let Some(element) = model.elements.get(&scope.element) else {
        return Err(LowerError::ModelIncomplete(vec![format!(
            "{} not found",
            scope.element
        )]));
    };
    let mut lines = Lines::default();
    let (open, assumptions) = match scope.behavior {
        Behavior::Own => {
            let Some(BehaviorModel::PetriNet(net)) = &element.behavior else {
                return Err(LowerError::NothingToCheck);
            };
            let part = part(element, net, &mut lines);
            (
                OpenNets {
                    parts: vec![part],
                    fusions: Vec::new(),
                },
                Vec::new(),
            )
        }
        Behavior::Combined => combined(model, element, &mut lines)?,
    };
    if lines.incomplete.is_empty() {
        match flatten(&open) {
            Ok(flat) => {
                if !lines.not_checked.is_empty() {
                    return Err(LowerError::NotCheckedYet(
                        lines.not_checked.into_iter().collect(),
                    ));
                }
                assert_eq!(
                    validate::net(&flat),
                    Ok(()),
                    "lower: flatten returned an invalid net"
                );
                return Ok(Lowered {
                    open,
                    flat,
                    assumptions,
                });
            }
            Err(FlattenError::Invalid(errors)) => {
                for error in errors {
                    lines.add(10, "", error.to_string());
                }
            }
            Err(error @ FlattenError::UnknownTransition { .. }) => {
                lines.add(3, "", error.to_string());
            }
            Err(error @ FlattenError::WeightOverflow { .. }) => {
                lines.add(10, "", error.to_string());
            }
        }
    }
    assert!(
        !lines.incomplete.is_empty(),
        "lower: failing without a line"
    );
    Err(LowerError::ModelIncomplete(
        lines
            .incomplete
            .into_iter()
            .map(|(_, _, line)| line)
            .collect(),
    ))
}

// One child's Petri net with own-shaped IDs (L2, L3 for arcs, L10).
fn part(element: &Element, net: &PetriNet, lines: &mut Lines) -> Net {
    let mut part = Net::default();
    let mut clash = false;
    for name in net
        .places
        .keys()
        .filter(|name| net.transitions.contains_key(*name))
    {
        clash = true;
        let id = qualified(element, name.as_str());
        lines.add(
            10,
            &id,
            format!(
                "{} has both a place and a transition named {name}",
                element.id
            ),
        );
    }
    place_nodes(element, net, lines, &mut part);
    for (name, transition) in &net.transitions {
        let mut arcs = |side: &BTreeMap<tracewright_model::Name, u32>| {
            let mut out = BTreeMap::new();
            for (place, weight) in side {
                let id = qualified(element, place.as_str());
                if !net.places.contains_key(place) {
                    lines.add(3, &id, format!("{id} not found"));
                }
                out.insert(QualifiedId(id), *weight);
            }
            out
        };
        let pre = arcs(&transition.pre);
        let post = arcs(&transition.post);
        part.transitions.push(Transition {
            id: QualifiedId(qualified(element, name.as_str())),
            pre,
            post,
        });
    }
    if !clash {
        if let Err(errors) = validate::net(&part) {
            for error in errors {
                // Arcs to missing places are already L3 lines.
                if !matches!(error, StructuralError::UnknownArcPlace { .. }) {
                    lines.add(10, "", error.to_string());
                }
            }
        }
    }
    part
}

// The places of one part, with L2 and the L10 capacity line.
fn place_nodes(element: &Element, net: &PetriNet, lines: &mut Lines, part: &mut Net) {
    for (name, place) in &net.places {
        let id = qualified(element, name.as_str());
        let capacity = place.capacity.unwrap_or_else(|| {
            lines.add(10, &id, format!("{id} has no capacity"));
            1
        });
        match place.when_full {
            None => lines.add(2, &id, format!("{id} has no when_full")),
            Some(WhenFull::Block) => {}
            Some(other) => {
                let what = if other == WhenFull::Drop {
                    "drop"
                } else {
                    "overflow"
                };
                lines.not_checked.insert(format!(
                    "{id} is a {what} place; drop and overflow places are not checked yet"
                ));
            }
        }
        // A place that is not `block` is lowered as `block` only so that L10 can still be judged;
        // a scope with one never reaches a worker (`NotCheckedYet`).
        part.places.push(Place {
            id: QualifiedId(id.clone()),
            capacity,
            when_full: tracewright_net::WhenFull::Block,
        });
        if place.initial > 0 {
            part.initial.0.insert(QualifiedId(id), place.initial);
        }
    }
}

// The combined scope of `scope`: its children's parts and the fusions of contributing
// relationships (NS §6.5, §6.6).
fn combined(
    model: &Model,
    scope: &Element,
    lines: &mut Lines,
) -> Result<(OpenNets, Vec<RelationshipId>), LowerError> {
    let children: Vec<&Element> = model
        .elements
        .values()
        .filter(|element| element.parent.as_ref() == Some(&scope.id))
        .collect();
    if children.iter().all(|child| child.behavior.is_none()) {
        return Err(LowerError::NothingToCheck);
    }
    let mut open = OpenNets::default();
    for child in &children {
        match &child.behavior {
            None => lines.add(
                1,
                child.id.as_str(),
                format!("{} has no behavior model", child.id),
            ),
            Some(BehaviorModel::PetriNet(net)) => open.parts.push(part(child, net, lines)),
        }
    }
    let child_ids: BTreeSet<_> = children.iter().map(|child| &child.id).collect();
    let mut bindings = Bindings::default();
    let mut assumptions = Vec::new();
    for relationship in model.relationships.values() {
        let source_inside = child_ids.contains(&relationship.source);
        let target_inside = child_ids.contains(&relationship.target);
        if !(source_inside && target_inside) || relationship.bindings.is_empty() {
            if source_inside || target_inside {
                assumptions.push(relationship.id.clone());
            }
            continue;
        }
        bindings.relationship(model, relationship, lines, &mut open.fusions);
    }
    // L4: every transition port of a child is named in a pair of a contributing relationship.
    for child in &children {
        if let Some(BehaviorModel::PetriNet(net)) = &child.behavior {
            for (name, transition) in &net.transitions {
                let id = qualified(child, name.as_str());
                if transition.port && !bindings.bound.contains(&id) {
                    lines.add(
                        4,
                        &id,
                        format!("{id} is a port but no relationship inside this scope binds it"),
                    );
                }
            }
        }
    }
    // L7, for synchronous pairs: never caller and callee.
    for id in bindings.callers.intersection(&bindings.callees) {
        lines.add(7, id, format!("{id} is both caller and callee"));
    }
    Ok((open, assumptions))
}

#[derive(Default)]
struct Bindings {
    bound: BTreeSet<String>,
    callers: BTreeSet<String>,
    callees: BTreeSet<String>,
}

impl Bindings {
    // The pairs of one contributing relationship: L3, L5, L6, L11, L12, and a Fusion per valid
    // synchronous pair.
    fn relationship(
        &mut self,
        model: &Model,
        relationship: &tracewright_model::Relationship,
        lines: &mut Lines,
        fusions: &mut Vec<Fusion>,
    ) {
        let rel = relationship.id.as_str();
        let mut fused_names = BTreeSet::new();
        for pair in &relationship.bindings {
            let from = format!("{}/{}", pair.from.element, pair.from.name);
            let to = format!("{}/{}", pair.to.element, pair.to.name);
            self.bound.insert(from.clone());
            self.bound.insert(to.clone());
            match pair.binding {
                Binding::Sync => {}
                Binding::Async { .. } => {
                    lines.not_checked.insert(format!(
                        "{rel} binds asynchronously; asynchronous bindings are not checked yet"
                    ));
                    continue;
                }
            }
            let from_resolves = resolve(model, &pair.from, lines);
            let to_resolves = resolve(model, &pair.to, lines);
            let mut valid = from_resolves && to_resolves;
            if pair.from.element == pair.to.element {
                lines.add(6, &from, format!("{from} is bound to {to} (same element)"));
                valid = false;
            }
            let (s, t) = (&relationship.source, &relationship.target);
            let forward = pair.from.element == *s && pair.to.element == *t;
            let backward = pair.from.element == *t && pair.to.element == *s;
            if !forward && !backward {
                lines.add(11, rel, format!("{rel} ({s} ↔ {t}) binds {from} + {to}"));
                valid = false;
            }
            if !fused_names.insert((pair.from.name.clone(), pair.to.name.clone())) {
                lines.add(
                    12,
                    rel,
                    format!("{rel} binds {} + {} twice", pair.from.name, pair.to.name),
                );
                valid = false;
            }
            self.callers.insert(from.clone());
            self.callees.insert(to.clone());
            if valid {
                fusions.push(Fusion {
                    relationship: rel.to_owned(),
                    caller: QualifiedId(from),
                    callee: QualifiedId(to),
                });
            }
        }
    }
}

// L3 and L5 for one end of a pair: it names a transition port of its element.
fn resolve(model: &Model, port: &PortRef, lines: &mut Lines) -> bool {
    let id = format!("{}/{}", port.element, port.name);
    let net = model
        .elements
        .get(&port.element)
        .and_then(|element| element.behavior.as_ref());
    let Some(BehaviorModel::PetriNet(net)) = net else {
        lines.add(3, &id, format!("{id} not found"));
        return false;
    };
    match net.transitions.get(&port.name) {
        Some(transition) if transition.port => true,
        Some(_) => {
            lines.add(5, &id, format!("{id} is bound but is not a port"));
            false
        }
        None => {
            if net.places.contains_key(&port.name) {
                lines.add(5, &id, format!("{id} is bound but is not a port"));
            } else {
                lines.add(3, &id, format!("{id} not found"));
            }
            false
        }
    }
}

/// The engine's composition of `open` (NS §6.5): the parts' places and unbound transitions, plus
/// one fused transition per fusion whose arcs add up the caller's and the callee's. recheck
/// composes the same `open` on its own, and the check compares the two encodings (ADR 0010).
///
/// # Errors
///
/// A fusion that names no transition, or a result that fails NS §2.3.
#[expect(
    clippy::similar_names,
    reason = "caller and callee are the terms of NS §6.3"
)]
pub fn flatten(open: &OpenNets) -> Result<Net, FlattenError> {
    let bound: BTreeSet<&QualifiedId> = open
        .fusions
        .iter()
        .flat_map(|fusion| [&fusion.caller, &fusion.callee])
        .collect();
    let mut transitions: BTreeMap<&QualifiedId, &Transition> = BTreeMap::new();
    let mut flat = Net::default();
    for part in &open.parts {
        flat.places.extend(part.places.iter().cloned());
        flat.initial.0.extend(
            part.initial
                .0
                .iter()
                .map(|(id, count)| (id.clone(), *count)),
        );
        for transition in &part.transitions {
            transitions.insert(&transition.id, transition);
            if !bound.contains(&transition.id) {
                flat.transitions.push(transition.clone());
            }
        }
    }
    for fusion in &open.fusions {
        let find = |id: &QualifiedId| {
            transitions
                .get(id)
                .copied()
                .ok_or_else(|| FlattenError::UnknownTransition {
                    transition: id.0.clone(),
                })
        };
        let (caller, callee) = (find(&fusion.caller)?, find(&fusion.callee)?);
        let id = fused_id(fusion);
        let pre = add_weights(&caller.pre, &callee.pre);
        let post = add_weights(&caller.post, &callee.post);
        let (Some(pre), Some(post)) = (pre, post) else {
            return Err(FlattenError::WeightOverflow { transition: id.0 });
        };
        flat.transitions.push(Transition { id, pre, post });
    }
    validate::net(&flat).map_err(FlattenError::Invalid)?;
    Ok(flat)
}

/// The qualified ID of a fusion's transition in the flattened net (NS §6.1):
/// `relationship/caller+callee` with the bare transition names.
#[must_use]
pub fn fused_id(fusion: &Fusion) -> QualifiedId {
    let bare = |id: &QualifiedId| {
        id.0.split_once('/')
            .map_or(id.0.clone(), |(_, name)| name.to_owned())
    };
    QualifiedId(format!(
        "{}/{}+{}",
        fusion.relationship,
        bare(&fusion.caller),
        bare(&fusion.callee)
    ))
}

// Pointwise sum of two arc maps, or None if a weight leaves u32.
fn add_weights(
    a: &BTreeMap<QualifiedId, u32>,
    b: &BTreeMap<QualifiedId, u32>,
) -> Option<BTreeMap<QualifiedId, u32>> {
    let mut sum = a.clone();
    for (place, weight) in b {
        let total = sum.get(place).copied().unwrap_or(0).checked_add(*weight)?;
        sum.insert(place.clone(), total);
    }
    Some(sum)
}
