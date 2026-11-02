//! Domain types of a Tracewright project: elements, relationships, behavior models, ports,
//! properties and layout. Pure: no IO (ADR 0004).
//!
//! So far the model holds what slice 0 checks: elements with Petri nets, their relationships with
//! binding pairs, and properties. State machines (slice 2), layout, commands and `validate` join it
//! with the tickets that use them.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

pub mod id;

pub use id::ElementId;
pub use id::Name;
pub use id::PropertyId;
pub use id::RelationshipId;

/// The whole model of one project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Model {
    /// Every element, by ID.
    pub elements: BTreeMap<ElementId, Element>,
    /// Every relationship, by ID. A relationship is stored in its source element's file.
    pub relationships: BTreeMap<RelationshipId, Relationship>,
}

/// The kinds of format 1 (file-format §6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    /// A person.
    Person,
    /// A software system.
    SoftwareSystem,
    /// A container.
    Container,
    /// A database.
    Database,
    /// A queue.
    Queue,
    /// A component.
    Component,
    /// A code element.
    Code,
}

/// One element (file-format §6.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// The stable ID.
    pub id: ElementId,
    /// What sort of element it is; never part of a net.
    pub kind: ElementKind,
    /// Display name; never part of a net or a key.
    pub name: String,
    /// The element whose view shows this one; it is a child of that element's combined scope.
    pub parent: Option<ElementId>,
    /// Display only.
    pub technology: Option<String>,
    /// Display only.
    pub description: Option<String>,
    /// The behavior model, if the element has one.
    pub behavior: Option<BehaviorModel>,
    /// Properties whose scope is this element, in file order.
    pub properties: Vec<Property>,
    /// The `[final]` texts, kept verbatim; slice 0 never parses them (D-25).
    pub final_predicates: FinalPredicates,
}

/// A behavior model (glossary). State machines arrive in slice 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BehaviorModel {
    /// A Petri net.
    PetriNet(PetriNet),
}

/// A Petri net inside an element (file-format §6.4). Places and transitions share one namespace;
/// a name in both maps is a name clash that lowering reports (L10).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PetriNet {
    /// Places by name.
    pub places: BTreeMap<Name, Place>,
    /// Transitions by name.
    pub transitions: BTreeMap<Name, Transition>,
}

/// What a place does at its capacity (ADR 0014).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhenFull {
    /// The sender waits: back-pressure.
    Block,
    /// The token is discarded: load shedding.
    Drop,
    /// Exceeding the capacity is a violation.
    Overflow,
}

/// One place. A missing capacity or `when_full` is a model problem, not a load failure
/// (file-format §3.3), so both are optional here and lowering reports them (L2, L10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// Display name.
    pub name: String,
    /// The capacity, at least 1.
    pub capacity: Option<u32>,
    /// When full; places of a behavior model have no default.
    pub when_full: Option<WhenFull>,
    /// The initial token count.
    pub initial: u32,
    /// Whether the place is a place port.
    pub port: bool,
}

/// One transition with its arcs, by place name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    /// Display name.
    pub name: String,
    /// Whether the transition is a transition port.
    pub port: bool,
    /// Arcs from places, place name to weight.
    pub pre: BTreeMap<Name, u32>,
    /// Arcs to places, place name to weight.
    pub post: BTreeMap<Name, u32>,
}

/// A relationship and its binding pairs (file-format §6.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    /// The stable ID.
    pub id: RelationshipId,
    /// The element whose file holds it.
    pub source: ElementId,
    /// The element it points to.
    pub target: ElementId,
    /// Display text.
    pub meaning: Option<String>,
    /// Display text.
    pub technology: Option<String>,
    /// Scan attribute, kept as written; the scan reads it from slice 5.
    pub interaction: Option<String>,
    /// Scan attribute, kept as written; the scan reads it from slice 5.
    pub timeout: Option<String>,
    /// The binding pairs, in file order.
    pub bindings: Vec<BindingPair>,
}

/// A port of an element, written `element/name` in the files.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PortRef {
    /// The element that owns the port.
    pub element: ElementId,
    /// The transition (or place) name inside its behavior model.
    pub name: Name,
}

/// One binding pair with its own direction (ADR 0016).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingPair {
    /// Sync: the caller. Async: the sender.
    pub from: PortRef,
    /// Sync: the callee. Async: the receiver.
    pub to: PortRef,
    /// The mode of the pair.
    pub binding: Binding,
}

/// The mode of a binding pair (glossary Binding).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// The two transitions fire as one fused transition.
    Sync,
    /// Messages wait in the relationship's generated place.
    Async {
        /// The capacity of the generated place; missing is L13.
        capacity: Option<u32>,
        /// When full; `overflow` when the file states none (ADR 0014).
        when_full: WhenFull,
    },
}

/// Which behavior of its scope element a property is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Behavior {
    /// The element's own behavior model, ports closed.
    Own,
    /// The combined behavior of its children.
    Combined,
}

/// The scope of a property: its element and behavior (D-32).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Scope {
    /// The scope element.
    pub element: ElementId,
    /// Own or combined behavior.
    pub behavior: Behavior,
}

/// One `[[property]]` entry (file-format §6.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    /// Unique in the project.
    pub id: PropertyId,
    /// Display text; never the headline.
    pub label: Option<String>,
    /// Own or combined.
    pub behavior: Behavior,
    /// What the property states.
    pub kind: PropertyKind,
}

/// The property kinds of file-format §6.8. Predicate texts are kept verbatim (D-25).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyKind {
    /// Deadlock-free.
    DeadlockFree,
    /// No capacity exceeded.
    NoCapacityExceeded,
    /// No dead transitions.
    NoDeadTransitions,
    /// Never p.
    Never {
        /// The predicate as written.
        predicate: String,
    },
    /// Always reachable p.
    AlwaysReachable {
        /// The predicate as written.
        predicate: String,
    },
}

/// The `[final]` table of a scope element, kept verbatim (file-format §6.8).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FinalPredicates {
    /// The final predicate of the combined behavior.
    pub combined: Option<String>,
    /// The final predicate of the own behavior.
    pub own: Option<String>,
}
