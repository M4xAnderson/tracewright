//! The vocabulary both explorers speak: nets, markings, properties, evidence and verdicts.
//!
//! Data only (ADR 0009): constructors, structural validation in `validate` and the canonical
//! encoding in `encode`. No firing rule, no predicate evaluation, no composition; CI fails on any
//! other `impl` block. Every place and transition is named by its qualified ID
//! (`docs/specs/net-semantics.md` §2.2), never by a position, so markings, keys and
//! counterexamples mean the same thing on the engine's net and on recheck's own composition.
//!
//! Slice 0 so far covers block places and synchronous bindings. `WhenFull`, `OpenNets`,
//! `NetProperty`, `Technique`, `Evidence` and `Violation` gain their other variants and fields
//! (drop and overflow, asynchronous bindings, no capacity exceeded, the `NoOverflowPlace`
//! certificate, the final predicate) as the later slice 0 tickets and slice 1 land; every match on
//! them is exhaustive, so the compiler lists each place that has to learn the new case.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

pub mod encode;
pub mod validate;

/// The string that names one place or transition of a net: `element/name` for an element's own
/// place or transition, `relationship/caller+callee` (bare names) for a fused transition (NS §2.2).
/// Ordered bytewise, which is the canonical order of NS §1.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QualifiedId(pub String);

/// What a place does at its capacity (ADR 0014). Only `block` is in the net class so far; `drop`
/// and `overflow` arrive with their firing rule (NS §4.2) in a later slice 0 ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WhenFull {
    /// A transition whose firing would put the place above its capacity is not enabled.
    Block,
}

/// One place: its capacity and what happens when it is full. Its initial count is in
/// [`Net::initial`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// Qualified ID, unique across the places and transitions of the net.
    pub id: QualifiedId,
    /// At least 1 (NS §2.3 rule 3).
    pub capacity: u32,
    /// When full (ADR 0014).
    pub when_full: WhenFull,
}

/// One transition with its arcs. A map key is a place ID and its value the arc weight (≥ 1); a
/// read arc is the same place in `pre` and `post` (NS §2.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    /// Qualified ID, unique across the places and transitions of the net.
    pub id: QualifiedId,
    /// Arcs from places into the transition.
    pub pre: BTreeMap<QualifiedId, u32>,
    /// Arcs from the transition into places.
    pub post: BTreeMap<QualifiedId, u32>,
}

/// Token count per place. A place that is missing holds 0 tokens, so two markings are equal iff
/// they agree on every place of the net (NS §3.1), not necessarily iff the maps are equal.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Marking(pub BTreeMap<QualifiedId, u32>);

/// A plain net (NS §2.1). The final predicate is not part of it (ADR 0018).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Net {
    /// Places, in any order; the encoding and both explorers sort by ID.
    pub places: Vec<Place>,
    /// Transitions, in any order.
    pub transitions: Vec<Transition>,
    /// The initial marking; it is in bounds on a valid net (NS §2.3 rule 4).
    pub initial: Marking,
}

/// One synchronous binding pair: the caller and the callee fire as one fused transition named
/// `relationship/caller+callee` with the bare transition names (NS §6.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fusion {
    /// The ID of the relationship that holds the pair.
    pub relationship: String,
    /// Qualified ID of the caller transition in its part.
    pub caller: QualifiedId,
    /// Qualified ID of the callee transition in another part.
    pub callee: QualifiedId,
}

/// The behavior models of a scope's children and the bindings between them, before composition
/// (NS §6.1). `engine::flatten` and `recheck::compose` each turn it into a [`Net`]. Asynchronous
/// bindings (`links`) arrive with their generated places in a later slice 0 ticket.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpenNets {
    /// One net per child, with own-shaped IDs and its own initial marking.
    pub parts: Vec<Net>,
    /// The synchronous binding pairs.
    pub fusions: Vec<Fusion>,
}

/// The property a check decides on a net (NS §8, ADR 0018).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetProperty {
    /// No reachable marking is a deadlock (NS §8.1). Slice 0 checks it with `final none`: no
    /// marking is final. The final predicate joins this variant in slice 1 (ADR 0018).
    DeadlockFree,
}

/// One fired transition and the marking after it (NS §9.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// Exactly one transition of the flattened net.
    pub fired: QualifiedId,
    /// The marking after the firing.
    pub marking: Marking,
}

/// Why a transition cannot fire at a deadlock, for one place (NS §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitReason {
    /// An input place holds fewer tokens than the arc weight.
    NeedsTokens {
        /// The pre weight.
        needed: u32,
        /// The count in the deadlock marking.
        holds: u32,
    },
    /// A blocking output place would end up above its capacity.
    OutputFull {
        /// The capacity of the place.
        capacity: u32,
        /// The tentative count after firing (NS §4.1).
        would_hold: u32,
    },
}

/// One place a waiting transition waits for, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wait {
    /// The place.
    pub place: QualifiedId,
    /// Why the transition cannot fire because of it.
    pub reason: WaitReason,
}

/// Everything one transition waits for at a deadlock: `waits(M, t)` of NS §7, places in canonical
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionWaits {
    /// The transition.
    pub transition: QualifiedId,
    /// Its waits, sorted by place ID.
    pub waits: Vec<Wait>,
}

/// What the last marking of a counterexample violates (NS §9.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// No transition is enabled and the marking is not final. One entry per transition of the
    /// net, sorted by transition ID.
    Deadlock {
        /// What each transition waits for.
        waiting: Vec<TransitionWaits>,
    },
}

/// A run from the initial marking to a violation (NS §9.1). It may have zero steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counterexample {
    /// The run.
    pub steps: Vec<Step>,
    /// What its last marking violates.
    pub violation: Violation,
}

/// The evidence of a Disproved (NS §9.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evidence {
    /// A run to a violating marking; confirmed by `recheck::replay`.
    Counterexample(Counterexample),
}

/// How a Proved was reached (ADR 0008). The certificate path arrives with `NoOverflowPlace`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Technique {
    /// Every reachable marking was explored; confirmed by agreement of the second explorer.
    Exhaustive,
}

/// The phase of a check that ran out of budget (NS §10.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// `verify`, the main explorer.
    Main,
    /// `recheck`, the second explorer.
    Recheck,
}

/// Why the tool could not decide a property: the closed list of ADR 0017, in display order
/// (NS §10.2). There are no other reasons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownReason {
    /// Lowering failed; one line per cause, with qualified IDs (NS §6.6).
    ModelIncomplete(Vec<String>),
    /// The property has an error or is not checked in this slice; one line per cause.
    PropertyNotCheckable(Vec<String>),
    /// The two compositions or the two explorers disagree.
    InternalDisagreement,
    /// A worker panicked, a replay failed, arithmetic overflowed or an explorer rejected its input.
    InternalError,
    /// The cancel flag was set.
    Cancelled,
    /// A phase ran out of markings before it finished.
    BudgetExhausted {
        /// Which explorer ran out.
        phase: Phase,
        /// Markings stored when it stopped.
        markings: u64,
    },
    /// Exploration finished, found no violation and reached an overflow marking (NS §8).
    CapacityExceededFirst {
        /// The smallest-ID exceeded place of the first overflow marking.
        place: QualifiedId,
        /// The run to that marking.
        steps: Vec<Step>,
    },
}

/// What an explorer may spend on one phase (NS §10.1). The deadline and the cancel flag join it
/// with the supervised workers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// The most markings the phase may store, the initial marking included.
    pub max_markings: u64,
}

/// A candidate verdict, as `verify::check` and `recheck::explore` return it. It is never cached,
/// shown or converted to a DTO: only `engine::check` turns it into a verdict, after the evidence
/// was checked (VER-1, D-53).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetVerdict {
    /// The property holds.
    Proved {
        /// How it was proved.
        technique: Technique,
        /// The number of reachable markings |R| for an exhaustive Proved.
        markings: u64,
    },
    /// The property does not hold.
    Disproved(Evidence),
    /// The property could not be decided.
    Unknown(UnknownReason),
}
