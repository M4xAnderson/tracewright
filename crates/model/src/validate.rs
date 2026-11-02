//! `validate`: the problems of a Model (file-format §10.3), and `blocks`: what each one stops
//! (§10.5). Problems are a function of the Model, never stored state, so they come out the same
//! after open, edit, undo and reload (design §3). The checks mirror the lowering preconditions of
//! NS §6.6 with the same detail lines; `engine::lower` keeps its own as a second line of defense.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::Behavior;
use crate::BehaviorModel;
use crate::Binding;
use crate::BindingPair;
use crate::Element;
use crate::ElementId;
use crate::Model;
use crate::PetriNet;
use crate::PortRef;
use crate::Relationship;
use crate::Scope;
use crate::problem::Blocks;
use crate::problem::ParentOfNotLoaded;
use crate::problem::Problem;
use crate::problem::ProblemKind;
use crate::problem::Severity;
use crate::problem::element_file;

/// Every problem of `model`, sorted by file, then line, then message (flows §4.5).
///
/// # Panics
///
/// Only on a bug: a problem without a cause.
#[must_use]
pub fn validate(model: &Model) -> Vec<Problem> {
    let mut problems: Vec<Problem> = model
        .files_not_loaded
        .values()
        .map(|file| file.problem.clone())
        .collect();
    problems.extend(model.load_problems.iter().cloned());
    let bound = bound_ports(model);
    let cycles = parent_cycles(model);
    for element in model.elements.values() {
        parent_problems(model, element, &cycles, &mut problems);
        match &element.behavior {
            Some(BehaviorModel::PetriNet(net)) => {
                net_problems(element, net, &mut problems);
                unbound_ports(element, net, &bound, &mut problems);
            }
            Some(BehaviorModel::Unloaded { problem }) => problems.push(problem.clone()),
            None => no_behavior(model, element, &mut problems),
        }
        for property in &element.properties {
            problems.extend(property.problems.iter().cloned());
        }
        let has_final =
            element.final_predicates.combined.is_some() || element.final_predicates.own.is_some();
        if has_final {
            let mut problem = Problem::error(
                ProblemKind::FinalNotChecked,
                Some("D-25"),
                format!(
                    "{} has a [final] table; done when is checked from slice 1",
                    element_file(&element.id)
                ),
            )
            .on_element(&element.id);
            problem.severity = Severity::Warning;
            problems.push(problem);
        }
    }
    for relationship in model.relationships.values() {
        relationship_problems(model, relationship, &mut problems);
    }
    problems.sort_by(|a, b| {
        (&a.file, a.line, a.kind, &a.message).cmp(&(&b.file, b.line, b.kind, &b.message))
    });
    problems.dedup();
    assert!(
        problems.iter().all(|problem| !problem.message.is_empty()),
        "validate: every problem states its cause"
    );
    problems
}

/// What `problem` stops in `model` (file-format §10.3 "Blocks", §10.5).
#[must_use]
pub fn blocks(model: &Model, problem: &Problem) -> Blocks {
    match problem.kind {
        ProblemKind::Parse | ProblemKind::Schema | ProblemKind::TooLarge => {
            let not_loaded = problem
                .file
                .as_ref()
                .and_then(|file| model.files_not_loaded.get(file));
            match not_loaded.map(|file| &file.parent) {
                None | Some(ParentOfNotLoaded::TopLevel) => Blocks::Nothing,
                Some(ParentOfNotLoaded::Unknown) => Blocks::EveryCombinedScope,
                Some(ParentOfNotLoaded::Element(parent)) => Blocks::Scopes(vec![combined(parent)]),
            }
        }
        ProblemKind::BehaviorNotLoaded
        | ProblemKind::NameClash
        | ProblemKind::NoWhenFull
        | ProblemKind::BadInitial => element_scopes(model, problem),
        ProblemKind::NoCapacity => {
            if problem.relationship.is_some() {
                relationship_scopes(model, problem)
            } else {
                element_scopes(model, problem)
            }
        }
        ProblemKind::Dangling => {
            if problem.relationship.is_some() {
                relationship_scopes(model, problem)
            } else if problem.place.is_some() {
                element_scopes(model, problem)
            } else {
                // A `parent` that names nothing: only that parent's combined scope could change.
                let parent = problem
                    .element
                    .as_ref()
                    .and_then(|id| model.elements.get(id))
                    .and_then(|element| element.parent.as_ref());
                Blocks::Scopes(parent.map(combined).into_iter().collect())
            }
        }
        ProblemKind::NoBehavior | ProblemKind::Unbound => Blocks::Scopes(
            parent_of(model, problem.element.as_ref())
                .map(combined)
                .into_iter()
                .collect(),
        ),
        ProblemKind::UnknownPort
        | ProblemKind::NotAPort
        | ProblemKind::BindingSelf
        | ProblemKind::BindingAsync
        | ProblemKind::BindingDirection
        | ProblemKind::BindingDuplicate
        | ProblemKind::BadBinding
        | ProblemKind::DuplicateId => relationship_scopes(model, problem),
        ProblemKind::ParentCycle => Blocks::Scopes(
            problem
                .element
                .iter()
                .flat_map(|id| [own(id), combined(id)])
                .collect(),
        ),
        ProblemKind::Property => Blocks::Property,
        ProblemKind::FinalNotChecked => Blocks::Nothing,
    }
}

fn own(element: &ElementId) -> Scope {
    Scope {
        element: element.clone(),
        behavior: Behavior::Own,
    }
}

fn combined(element: &ElementId) -> Scope {
    Scope {
        element: element.clone(),
        behavior: Behavior::Combined,
    }
}

fn parent_of<'m>(model: &'m Model, element: Option<&ElementId>) -> Option<&'m ElementId> {
    element
        .and_then(|id| model.elements.get(id))
        .and_then(|element| element.parent.as_ref())
}

// For an element: its own scope and its parent's combined scope (file-format §10.5).
fn element_scopes(model: &Model, problem: &Problem) -> Blocks {
    let Some(element) = &problem.element else {
        return Blocks::Nothing;
    };
    let mut scopes = vec![own(element)];
    scopes.extend(parent_of(model, Some(element)).map(combined));
    Blocks::Scopes(scopes)
}

// For a relationship: the combined scope of its ends' common parent. One whose target is missing,
// or that did not enter the model, blocks its source's parent's combined scope.
fn relationship_scopes(model: &Model, problem: &Problem) -> Blocks {
    let relationship = problem
        .relationship
        .as_ref()
        .and_then(|id| model.relationships.get(id));
    let Some(relationship) = relationship else {
        return Blocks::Scopes(
            parent_of(model, problem.element.as_ref())
                .map(combined)
                .into_iter()
                .collect(),
        );
    };
    if let Some(parent) = contributing_scope(model, relationship) {
        return Blocks::Scopes(vec![combined(parent)]);
    }
    if model.elements.contains_key(&relationship.target) {
        Blocks::Nothing
    } else {
        Blocks::Scopes(
            parent_of(model, Some(&relationship.source))
                .map(combined)
                .into_iter()
                .collect(),
        )
    }
}

// NS §6.6: a relationship contributes to the combined scope of P iff both its elements are
// children of P. An end whose file did not load counts with the parent the loader read
// leniently, so its ports are not reported a second time as unbound.
fn contributing_scope<'m>(model: &'m Model, relationship: &Relationship) -> Option<&'m ElementId> {
    let source = parent_of(model, Some(&relationship.source))?;
    let target = match model.elements.get(&relationship.target) {
        Some(target) => target.parent.as_ref()?,
        None => match &model
            .files_not_loaded
            .get(&element_file(&relationship.target))?
            .parent
        {
            ParentOfNotLoaded::Element(parent) => parent,
            ParentOfNotLoaded::Unknown | ParentOfNotLoaded::TopLevel => return None,
        },
    };
    if source == target { Some(source) } else { None }
}

fn qualified(port: &PortRef) -> String {
    format!("{}/{}", port.element, port.name)
}

// Every port named in a pair of a contributing relationship (L4).
fn bound_ports(model: &Model) -> BTreeSet<String> {
    let mut bound = BTreeSet::new();
    for relationship in model.relationships.values() {
        if contributing_scope(model, relationship).is_some() {
            for pair in &relationship.bindings {
                bound.insert(qualified(&pair.from));
                bound.insert(qualified(&pair.to));
            }
        }
    }
    bound
}

// The elements on a parent cycle, found in one pass over the parent chains (LIM-3: no
// recursion; every element is walked at most once).
fn parent_cycles(model: &Model) -> BTreeSet<ElementId> {
    let mut done: BTreeSet<&ElementId> = BTreeSet::new();
    let mut on_cycle = BTreeSet::new();
    for start in model.elements.keys() {
        let mut path: Vec<&ElementId> = Vec::new();
        let mut at = Some(start);
        while let Some(id) = at {
            if done.contains(id) {
                break;
            }
            if let Some(position) = path.iter().position(|seen| *seen == id) {
                on_cycle.extend(path[position..].iter().map(|id| (*id).clone()));
                break;
            }
            path.push(id);
            at = model
                .elements
                .get(id)
                .and_then(|element| element.parent.as_ref());
        }
        done.extend(path);
    }
    on_cycle
}

fn parent_problems(
    model: &Model,
    element: &Element,
    cycles: &BTreeSet<ElementId>,
    problems: &mut Vec<Problem>,
) {
    if cycles.contains(&element.id) {
        problems.push(
            Problem::error(
                ProblemKind::ParentCycle,
                Some("L3"),
                format!("{}: the parent chain loops", element.id),
            )
            .on_element(&element.id),
        );
    }
    let Some(parent) = &element.parent else {
        return;
    };
    if model.elements.contains_key(parent) {
        return;
    }
    let what = if model.files_not_loaded.contains_key(&element_file(parent)) {
        "did not load"
    } else {
        "not found"
    };
    problems.push(
        Problem::error(
            ProblemKind::Dangling,
            Some("L3"),
            format!("{}: parent {parent} {what}", element.id),
        )
        .on_element(&element.id),
    );
}

// L2, L3 for arcs and L10 inside one Petri net, in the wording of `engine::lower`.
fn net_problems(element: &Element, net: &PetriNet, problems: &mut Vec<Problem>) {
    let id = |name: &str| format!("{}/{name}", element.id);
    let mut push = |kind, code, place: String, message: String| {
        let mut problem = Problem::error(kind, Some(code), message).on_element(&element.id);
        problem.place = Some(place);
        problems.push(problem);
    };
    for name in net.places.keys() {
        if net.transitions.contains_key(name) {
            push(
                ProblemKind::NameClash,
                "L10",
                id(name.as_str()),
                format!(
                    "{} has both a place and a transition named {name}",
                    element.id
                ),
            );
        }
    }
    for (name, place) in &net.places {
        let place_id = id(name.as_str());
        match place.capacity {
            None => push(
                ProblemKind::NoCapacity,
                "L10",
                place_id.clone(),
                format!("{place_id} has no capacity"),
            ),
            Some(capacity) => {
                if place.initial > capacity {
                    push(
                        ProblemKind::BadInitial,
                        "L10",
                        place_id.clone(),
                        format!(
                            "{place_id}: initial {} exceeds capacity {capacity}",
                            place.initial
                        ),
                    );
                }
            }
        }
        if place.when_full.is_none() {
            push(
                ProblemKind::NoWhenFull,
                "L2",
                place_id.clone(),
                format!("{place_id} has no when_full"),
            );
        }
    }
    for transition in net.transitions.values() {
        for place in transition.pre.keys().chain(transition.post.keys()) {
            if !net.places.contains_key(place) {
                let place_id = id(place.as_str());
                push(
                    ProblemKind::Dangling,
                    "L3",
                    place_id.clone(),
                    format!("{place_id} not found"),
                );
            }
        }
    }
}

// L4: a transition port of a child that no contributing relationship binds. A top-level element
// is a child of no scope.
fn unbound_ports(
    element: &Element,
    net: &PetriNet,
    bound: &BTreeSet<String>,
    problems: &mut Vec<Problem>,
) {
    if element.parent.is_none() {
        return;
    }
    for (name, transition) in &net.transitions {
        let port = format!("{}/{name}", element.id);
        if transition.port && !bound.contains(&port) {
            let mut problem = Problem::error(
                ProblemKind::Unbound,
                Some("L4"),
                format!("{port} is a port but no relationship inside this scope binds it"),
            )
            .on_element(&element.id);
            problem.port = Some(port);
            problems.push(problem);
        }
    }
}

// L1, listed only when another child of the same parent has a behavior model (D-13).
fn no_behavior(model: &Model, element: &Element, problems: &mut Vec<Problem>) {
    let Some(parent) = &element.parent else {
        return;
    };
    let sibling_has_one = model.elements.values().any(|other| {
        other.id != element.id && other.parent.as_ref() == Some(parent) && other.behavior.is_some()
    });
    if sibling_has_one {
        problems.push(
            Problem::error(
                ProblemKind::NoBehavior,
                Some("L1"),
                format!("{} has no behavior model", element.id),
            )
            .on_element(&element.id),
        );
    }
}

// A problem of `relationship`, in its source element's file.
fn on_relationship(
    relationship: &Relationship,
    kind: ProblemKind,
    code: &'static str,
    message: String,
) -> Problem {
    let mut problem = Problem::error(kind, Some(code), message).on_element(&relationship.source);
    problem.relationship = Some(relationship.id.clone());
    problem
}

// L3 for the target and, for a contributing relationship, L8, L12 and L13 across its pairs plus
// the per-pair rules. Pairs on a relationship that contributes nowhere never enter a net.
fn relationship_problems(model: &Model, relationship: &Relationship, problems: &mut Vec<Problem>) {
    let rel = relationship.id.as_str();
    let target = &relationship.target;
    if !model.elements.contains_key(target) {
        // A target whose file did not load is reported once, as that file's problem.
        if !model.files_not_loaded.contains_key(&element_file(target)) {
            problems.push(on_relationship(
                relationship,
                ProblemKind::Dangling,
                "L3",
                format!("{rel}: target {target} not found"),
            ));
        }
        return;
    }
    if contributing_scope(model, relationship).is_none() {
        return;
    }
    let mut fused: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut asynchronous = 0_usize;
    for pair in &relationship.bindings {
        pair_problems(model, relationship, pair, problems);
        match pair.binding {
            Binding::Sync => {
                let names = (pair.from.name.to_string(), pair.to.name.to_string());
                *fused.entry(names).or_default() += 1;
            }
            Binding::Async { capacity, .. } => {
                asynchronous += 1;
                if capacity.is_none() {
                    problems.push(on_relationship(
                        relationship,
                        ProblemKind::NoCapacity,
                        "L13",
                        format!("{rel} is asynchronous but has no capacity"),
                    ));
                }
            }
        }
    }
    for ((from, to), count) in fused {
        if count > 1 {
            problems.push(on_relationship(
                relationship,
                ProblemKind::BindingDuplicate,
                "L12",
                format!("{rel} binds {from} + {to} twice"),
            ));
        }
    }
    if asynchronous > 1 {
        problems.push(on_relationship(
            relationship,
            ProblemKind::BindingAsync,
            "L8",
            format!("{rel} binds two asynchronous port pairs"),
        ));
    }
}

// L3 and L5 for each end of one pair, L6 and L11.
fn pair_problems(
    model: &Model,
    relationship: &Relationship,
    pair: &BindingPair,
    problems: &mut Vec<Problem>,
) {
    let (from, to) = (qualified(&pair.from), qualified(&pair.to));
    for end in [&pair.from, &pair.to] {
        if let Some((kind, message)) = resolve(model, end) {
            let mut problem = on_relationship(relationship, kind, kind_code(kind), message);
            problem.port = Some(qualified(end));
            problems.push(problem);
        }
    }
    if pair.from.element == pair.to.element {
        let mut problem = on_relationship(
            relationship,
            ProblemKind::BindingSelf,
            "L6",
            format!("{from} is bound to {to} (same element)"),
        );
        problem.port = Some(from.clone());
        problems.push(problem);
    }
    let (s, t) = (&relationship.source, &relationship.target);
    let forward = pair.from.element == *s && pair.to.element == *t;
    let backward = pair.from.element == *t && pair.to.element == *s;
    if !forward && !backward {
        problems.push(on_relationship(
            relationship,
            ProblemKind::BindingDirection,
            "L11",
            format!("{} ({s} ↔ {t}) binds {from} + {to}", relationship.id),
        ));
    }
}

fn kind_code(kind: ProblemKind) -> &'static str {
    if kind == ProblemKind::NotAPort {
        "L5"
    } else {
        "L3"
    }
}

// L3 and L5 for one end of a pair: it names a transition port of its element. An element whose
// behavior did not load is already reported (`BehaviorNotLoaded`), so its ports are not judged.
fn resolve(model: &Model, port: &PortRef) -> Option<(ProblemKind, String)> {
    let id = qualified(port);
    let not_found = Some((ProblemKind::UnknownPort, format!("{id} not found")));
    let not_a_port = Some((
        ProblemKind::NotAPort,
        format!("{id} is bound but is not a port"),
    ));
    let behavior = model
        .elements
        .get(&port.element)
        .and_then(|element| element.behavior.as_ref());
    match behavior {
        None => not_found,
        Some(BehaviorModel::Unloaded { .. }) => None,
        Some(BehaviorModel::PetriNet(net)) => match net.transitions.get(&port.name) {
            Some(transition) => {
                if transition.port {
                    None
                } else {
                    not_a_port
                }
            }
            None => {
                if net.places.contains_key(&port.name) {
                    not_a_port
                } else {
                    not_found
                }
            }
        },
    }
}
