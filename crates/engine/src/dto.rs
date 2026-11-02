//! The IPC contract (design §3, flows §9): the events the Session emits and the view of the open
//! project, in element terms. ts-rs writes the TypeScript bindings on `cargo test` (IPC-2), and
//! `check` fails when they drift. Everything here carries IDs, never display names: the UI
//! resolves names through the `View`, so a rename never shows an old label.
//!
//! A `VerdictDto` is built only from the checked `engine::check::Verdict` (VER-1, D-53).

use std::collections::BTreeMap;

use serde::Serialize;
use ts_rs::TS;

use crate::key::CheckKey;

/// The largest integer a JavaScript `number` holds exactly (IPC-2).
pub const JS_SAFE_INTEGER_MAX: u64 = (1 << 53) - 1;

/// An event of the Session, sent over the one Channel to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "engine-event.ts")]
pub enum EngineEvent {
    /// The complete key set of one revision, sent before any verdict for it.
    CurrentKeys(CurrentKeys),
    /// A check for this key is queued or running: the row shows `V-RUN`.
    CheckQueued(CheckQueued),
    /// The verdict of one check.
    Verdict(VerdictEvent),
}

/// The complete key set of revision `rev` (design §4: `keysFresh` compares it with the view's).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "current-keys.ts")]
pub struct CurrentKeys {
    /// The revision the keys were derived for.
    #[ts(type = "number")]
    pub rev: u64,
    /// One entry per property of the project, by property ID.
    pub keys: BTreeMap<String, KeyState>,
}

/// What a property's check is keyed by, before any check runs (design §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "key-state.ts")]
pub enum KeyState {
    /// The property lowered; a check with this key decides it.
    Key(CheckKey),
    /// The property ends as Unknown without a check: model incomplete or not checkable.
    Unknown(UnknownDto),
    /// No behavior model to check: no verdict at all (D-13).
    NothingToCheck,
}

/// A check was queued for `key`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "check-queued.ts")]
pub struct CheckQueued {
    /// The property ID.
    pub property: String,
    /// The key of the net the check explores.
    pub key: CheckKey,
}

/// The verdict of one check, with the key of the net that was checked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "verdict-event.ts")]
pub struct VerdictEvent {
    /// The property ID.
    pub property: String,
    /// Recomputed by the worker from the net it explored.
    pub key: CheckKey,
    /// The verdict.
    pub verdict: VerdictDto,
}

/// A checked verdict as the UI shows it: exactly three outcomes (VER-3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "verdict-dto.ts")]
pub struct VerdictDto {
    /// Relationship IDs of the scope that bind no ports, so they take no part in the check
    /// (NS §6.6, `V-PRO-ASM`).
    pub assumptions: Vec<String>,
    /// Proved, Disproved or Unknown.
    #[serde(flatten)]
    #[ts(flatten)]
    pub outcome: Outcome,
}

/// The three verdicts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "outcome.ts")]
pub enum Outcome {
    /// Confirmed by the second explorer.
    Proved(ProvedDto),
    /// With evidence replayed on the second explorer's composition.
    Disproved(DisprovedDto),
    /// Undecided, for exactly one reason.
    Unknown(UnknownVerdict),
}

/// A Proved: how and over how many markings (VER-5, `V-PRO-1`). A certificate has 0 markings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "proved-dto.ts")]
pub struct ProvedDto {
    /// The evidence path of ADR 0008.
    pub technique: Technique,
    /// Reachable markings both explorers counted.
    #[ts(type = "number")]
    pub markings: u64,
}

/// How a Proved was reached: the two paths of ADR 0008.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "technique.ts")]
pub enum Technique {
    /// Both explorers explored every reachable marking and agreed.
    Exhaustive,
    /// No place overflows when full; the second explorer checked the certificate (NS §9.5).
    NoOverflowPlace(NoOverflowPlaceDto),
}

/// The `NoOverflowPlace` certificate of a Proved no capacity exceeded (D-03).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "no-overflow-place-dto.ts")]
pub struct NoOverflowPlaceDto {
    /// The places that drop when full: messages can still be lost there.
    pub drop_places: Vec<PlaceOrigin>,
}

/// A Disproved with its counterexample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "disproved-dto.ts")]
pub struct DisprovedDto {
    /// One frame per step of the counterexample, from step 1.
    pub frames: Vec<ReplayFrame>,
    /// What is wrong in the last marking.
    pub violation: ViolationDto,
}

/// One step of a counterexample, in element terms (NS §6.7). The replay ticket adds the tokens
/// and highlights.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "replay-frame.ts")]
pub struct ReplayFrame {
    /// 1-based.
    pub step: u32,
    /// The transition of the flattened net that fired.
    pub fired: TransitionOrigin,
}

/// Where a transition of the flattened net comes from (NS §6.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "transition-origin.ts")]
pub enum TransitionOrigin {
    /// A transition of one element.
    Own(TransitionRef),
    /// The fused transition of a synchronous binding pair.
    Sync {
        /// The relationship that holds the pair.
        relationship: String,
        /// The calling element's transition.
        caller: TransitionRef,
        /// The called element's transition.
        callee: TransitionRef,
    },
}

/// A transition of an element's Petri net.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "transition-ref.ts")]
pub struct TransitionRef {
    /// The element ID.
    pub element: String,
    /// The transition's name in the element's net.
    pub transition: String,
}

/// A place of an element's Petri net.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "place-ref.ts")]
pub struct PlaceRef {
    /// The element ID.
    pub element: String,
    /// The place's name in the element's net.
    pub place: String,
}

/// Where a place of the flattened net comes from (NS §6.7): a place of an element's Petri net, or
/// the generated place of a relationship's asynchronous binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "place-origin.ts")]
pub enum PlaceOrigin {
    /// A place of one element.
    Own(PlaceRef),
    /// The queue of an asynchronous binding; its ID is the relationship's.
    Generated {
        /// The relationship.
        relationship: String,
    },
}

/// What the last marking of a counterexample violates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "violation-dto.ts")]
pub enum ViolationDto {
    /// No transition is enabled (NS §7).
    Deadlock(DeadlockDto),
    /// A place holds more than its capacity (NS §8.2, `V-DIS-CAP-*`).
    Overflow(OverflowDto),
}

/// An overflow marking: the smallest-ID place above its capacity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "overflow-dto.ts")]
pub struct OverflowDto {
    /// The place.
    pub place: PlaceOrigin,
    /// Its capacity.
    pub capacity: u32,
    /// What it holds in the last marking.
    pub count: u32,
}

/// A deadlock: who waits for what (flows R1, `V-DIS-DL-*`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "deadlock-dto.ts")]
pub struct DeadlockDto {
    /// One entry per transition that waits, in the order of the net.
    pub waiting: Vec<WaitingDto>,
    /// No place in any transition's pre set holds a token: the run finished rather than got
    /// stuck, and the panel adds `V-DIS-NOFINAL` (NS §9.6, D-23, flows R18).
    pub finished: bool,
}

/// One waiting transition and the places it waits for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "waiting-dto.ts")]
pub struct WaitingDto {
    /// The transition.
    pub transition: TransitionOrigin,
    /// One entry per place that holds it back.
    pub waits: Vec<WaitDto>,
}

/// One place a waiting transition is held back by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "wait-dto.ts")]
pub struct WaitDto {
    /// The place.
    pub place: PlaceOrigin,
    /// Why.
    pub reason: WaitReason,
}

/// Why a place holds a transition back (NS §7, flows R1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "wait-reason.ts")]
pub enum WaitReason {
    /// A pre place holds fewer tokens than the arc takes.
    NeedsTokens {
        /// The arc weight.
        needed: u32,
        /// Tokens in the place.
        holds: u32,
    },
    /// A `block` post place has no room for what the transition would add.
    OutputFull {
        /// The place's capacity.
        capacity: u32,
        /// Tokens in the place.
        holds: u32,
    },
}

/// An Unknown verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "unknown-verdict.ts")]
pub struct UnknownVerdict {
    /// The one reason.
    pub reason: UnknownDto,
}

/// The seven Unknown reasons (ADR 0017, VER-3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "category", rename_all = "camelCase")]
#[ts(export, export_to = "unknown-dto.ts")]
pub enum UnknownDto {
    /// Lowering failed; one line per cause.
    ModelIncomplete {
        /// The causes.
        details: Vec<String>,
    },
    /// The property cannot be checked (yet); one line per cause.
    PropertyNotCheckable {
        /// The causes.
        details: Vec<String>,
    },
    /// The two compositions or explorers disagreed.
    InternalDisagreement,
    /// A bug in the tool: a failed replay or a panicked worker.
    InternalError,
    /// The user cancelled the check.
    Cancelled,
    /// The budget ran out.
    BudgetExhausted {
        /// Which explorer ran out.
        phase: Phase,
        /// Markings stored when it stopped.
        #[ts(type = "number")]
        markings: u64,
    },
    /// An overflow place exceeded its capacity before the property was decided (`U-CEF-*`).
    CapacityExceededFirst {
        /// The smallest-ID exceeded place of the first overflow marking (contract R4).
        place: PlaceOrigin,
        /// Its capacity.
        capacity: u32,
        /// What it holds at the end of the run.
        count: u32,
        /// The run to that overflow marking, replayed by the second explorer.
        frames: Vec<ReplayFrame>,
    },
}

/// The two explorers of a check (NS §10.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "phase.ts")]
pub enum Phase {
    /// `verify`.
    Main,
    /// `recheck`.
    Recheck,
}

/// The view of the open project (design §3 projection): the software system, its children and
/// their relationships, and the properties whose scope is in the view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "view.ts")]
pub struct View {
    /// The revision of the model this view shows.
    #[ts(type = "number")]
    pub rev: u64,
    /// The scope of the Container view; `None` when the project has no software system.
    pub system: Option<ElementView>,
    /// The system's children, by ID.
    pub elements: Vec<ElementView>,
    /// Relationships with at least one end among the children, by ID.
    pub relationships: Vec<RelationshipView>,
    /// Properties of the system and of its children, by scope element, then file order.
    pub properties: Vec<PropertyView>,
}

/// An element as the view shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "element-view.ts")]
pub struct ElementView {
    /// The element ID.
    pub id: String,
    /// The display name.
    pub name: String,
}

/// A relationship as the view shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "relationship-view.ts")]
pub struct RelationshipView {
    /// The relationship ID.
    pub id: String,
    /// The source element ID.
    pub source: String,
    /// The target element ID.
    pub target: String,
    /// What it means, for example "calls".
    pub meaning: Option<String>,
}

/// A property as the verdict panel lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "property-view.ts")]
pub struct PropertyView {
    /// The property ID.
    pub id: String,
    /// The user's label, shown next to the kind's name.
    pub label: Option<String>,
    /// What it states.
    pub kind: PropertyKind,
    /// Its scope.
    pub scope: ScopeView,
}

/// The property kinds of file-format §6.8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "property-kind.ts")]
pub enum PropertyKind {
    /// Deadlock-free.
    DeadlockFree,
    /// No capacity exceeded.
    NoCapacityExceeded,
    /// No dead transitions.
    NoDeadTransitions,
    /// Never p.
    Never,
    /// Always reachable p.
    AlwaysReachable,
}

/// The scope of a property: its element and behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "scope-view.ts")]
pub struct ScopeView {
    /// The scope element ID.
    pub element: String,
    /// Own or combined.
    pub behavior: Behavior,
}

/// Which behavior of its scope element a property is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "behavior.ts")]
pub enum Behavior {
    /// The element's own behavior model, ports closed.
    Own,
    /// The combined behavior of its children.
    Combined,
}
