//! The vocabulary both explorers speak: nets, markings, properties, evidence and verdicts.
//!
//! Data only (ADR 0009): constructors, structural validation in `validate` and the canonical
//! encoding in `encode`. No firing rule, no predicate evaluation, no composition; CI fails on any
//! other `impl` block. Every place and transition is named by its qualified ID
//! (`docs/specs/net-semantics.md` §2.2), never by a position, so markings, keys and
//! counterexamples mean the same thing on the engine's net and on recheck's own composition.
//!
//! Slice 0 covers block, drop and overflow places, synchronous and asynchronous bindings, the
//! properties deadlock-free (`final none`) and no capacity exceeded, and the `NoOverflowPlace`
//! certificate. `NetProperty`, `Evidence` and `Violation` gain their later variants (the final
//! predicate, never p, always reachable, no dead transitions) with slices 1 and 4; every match on
//! them is exhaustive, so the compiler lists each place that has to learn the new case.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

pub mod encode;
pub mod validate;

/// The string that names one place or transition of a net: `element/name` for an element's own
/// place or transition, `relationship/caller+callee` (bare names) for a fused transition, and
/// `relationship` for the generated place of an asynchronous binding (NS §2.2).
/// Ordered bytewise, which is the canonical order of NS §1.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QualifiedId(pub String);

/// What a place does at its capacity (ADR 0014, NS §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WhenFull {
    /// A transition whose firing would put the place above its capacity is not enabled.
    Block,
    /// The transition fires and the surplus above the capacity is discarded.
    Drop,
    /// The transition fires and the place may exceed its capacity: an overflow marking, which is
    /// a violation and has no successors (NS §3.2).
    Overflow,
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

/// One asynchronous binding: a generated place named `relationship` with this capacity and when
/// full, initially empty. Every sender puts one message into it, every receiver takes one
/// (NS §6.4). The sets hold transitions; a Petri net port is a one-element set, a state machine
/// event every transition on that event (D-06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsyncLink {
    /// The ID of the relationship that holds the binding; also the generated place's ID.
    pub relationship: String,
    /// The senders: non-empty, sorted by ID, without duplicates.
    pub from: Vec<QualifiedId>,
    /// The receivers: non-empty, sorted by ID, without duplicates.
    pub to: Vec<QualifiedId>,
    /// The generated place's capacity, at least 1 (L13).
    pub capacity: u32,
    /// Always explicit: lowering fills in the default `overflow` (ADR 0014).
    pub when_full: WhenFull,
}

/// The behavior models of a scope's children and the bindings between them, before composition
/// (NS §6.1). `engine::flatten` and `recheck::compose` each turn it into a [`Net`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpenNets {
    /// One net per child, with own-shaped IDs and its own initial marking.
    pub parts: Vec<Net>,
    /// The synchronous binding pairs.
    pub fusions: Vec<Fusion>,
    /// The asynchronous bindings, at most one per relationship (L8).
    pub links: Vec<AsyncLink>,
}

/// The property a check decides on a net (NS §8, ADR 0018).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetProperty {
    /// No reachable marking is a deadlock (NS §8.1). Slice 0 checks it with `final none`: no
    /// marking is final. The final predicate joins this variant in slice 1 (ADR 0018).
    DeadlockFree,
    /// No reachable marking is an overflow marking (NS §8.2): "no capacity exceeded".
    CapacityNeverExceeded,
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
    /// The marking is an overflow marking (NS §8.2).
    Overflow {
        /// The smallest-ID place above its capacity.
        place: QualifiedId,
        /// Its capacity.
        capacity: u32,
        /// Its count in the marking.
        count: u32,
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

/// A small object an independent checker verifies without exploring (NS §9.5). Only the kinds
/// listed here are valid evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Certificate {
    /// No place overflows when full, so no overflow marking exists (NS §3.2). Valid for no
    /// capacity exceeded only.
    NoOverflowPlace {
        /// Every place that drops when full, sorted by ID: messages can still be lost there
        /// (D-03).
        drop_places: Vec<QualifiedId>,
    },
}

/// How a Proved was reached: the two paths of ADR 0008 as amended by ADR 0017.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Technique {
    /// Every reachable marking was explored; confirmed by agreement of the second explorer.
    Exhaustive,
    /// A certificate, accepted by `recheck::check_certificate`; `markings` is 0.
    Certificate(Certificate),
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

/// Expansions (pops of one marking from the queue) between two polls of the deadline and the
/// cancel flag; an explorer polls at its first expansion and then at least this often (NS §10.1).
pub const POLL_EXPANSIONS: u64 = 1024;

/// The time a deadline is measured against: the engine's Clock port, a fake one in tests
/// (design §3). Explorers read it only when they poll.
pub trait Clock: fmt::Debug + Send + Sync {
    /// The current instant.
    fn now(&self) -> Instant;
}

/// The moment a phase must end, with the clock that tells it (NS §10.1).
#[derive(Debug, Clone)]
pub struct Deadline {
    /// The phase has run out of time once the clock reads this instant or later.
    pub at: Instant,
    /// Shared with the engine and the other phase of the same check, which read the same clock.
    pub clock: Arc<dyn Clock>,
}

/// What an explorer may spend on one phase (NS §10.1): markings, time and a cancel flag, polled
/// together every [`POLL_EXPANSIONS`] expansions.
#[derive(Debug, Clone, Default)]
pub struct Budget {
    /// The most markings the phase may store, the initial marking included.
    pub max_markings: u64,
    /// `None` for a budget of markings only.
    pub deadline: Option<Deadline>,
    /// Set by the engine to stop the check; shared with it, so the flag is behind an `Arc`.
    pub cancel: Arc<AtomicBool>,
}

/// What an explorer reports at a poll while it runs (design §3, `checkProgress`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Markings stored so far, the initial marking included.
    pub markings: u64,
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
        /// The number of reachable markings |R| for an exhaustive Proved; 0 for a certificate.
        markings: u64,
    },
    /// The property does not hold.
    Disproved(Evidence),
    /// The property could not be decided.
    Unknown(UnknownReason),
}
