# State machines

Specification `docs/specs/state-machines.md`. Status: **Accepted** (applies `docs/specs/decisions.md`), revision 3, 2026-11-02. Author: senior backend developer. Revision 3 applies the decisions owned by this spec (D-01–D-07, D-09, D-10, D-12, D-14–D-16, D-23, D-24, D-26, D-28–D-31, D-33, D-36, D-37, D-41–D-44, D-46) and rebases the text on `docs/specs/net-semantics.md` revision 3 ("the net semantics" below; section references with a leading **NS** point there, for example NS 6.4). Every count, size, counterexample, waiting payload and canonical encoding in section 11 was computed with a throwaway reference implementation of NS 2–8, 11, 12 plus the compilation rule of section 4 (Python, in a temp directory, not in the repo); the vectors new in revision 3 (V-03 under the slice 2 default, V-13c, V-20b, V-25, K-07) were run again on 2026-11-02.

This text defines flat state machines, the second notation of ADR 0002: their structure, their ports, how they bind to other elements, their compilation to a 1-safe part of `OpenNets`, the runtime checks that keep a compiler bug from becoming a wrong Proved, the model-level replay check, validation, and forward compatibility. `crates/model` and `crates/format` implement sections 1, 2 and 8; `crates/engine` implements sections 4–7 and 9. `net`, `verify` and `recheck` see only Petri nets (ADR 0002) and implement nothing from this text: set-valued asynchronous bindings are in the net semantics from slice 0 (ADR 0016, D-06).

Where this text and an ADR or the net semantics disagree, the ADR and the net semantics win. The amendments this spec proposed in revision 2 are decided and recorded in ADRs 0015–0018 and NS revision 3 (section 13). Terms follow `GLOSSARY.md` (section 14). Every question of revision 2 is closed (section 15).

**Normative words.** "must", "must not", "may" bind `model`, `format` and `engine`. A code block is a sketch of shape unless it says "normative".

**Slice tags.** **[S2]** is needed by slice 2, the slice that brings state machines. Final predicates exist from slice 1 (D-05), so they are [S2] here. **[S4]** marks what needs slice 4 (user properties, templates, no dead transitions; NS 15). Slice 2 checks exactly two properties: deadlock-free (with the final predicate) and no capacity exceeded.

---

## 1. Structure [S2]

A **state machine** is a behavior model made of states and transitions between them, triggered by events. v1 state machines are flat and have no data (ADR 0002).

### 1.1 What a v1 state machine contains

| Part | Rule (normative) |
|---|---|
| **States** | A non-empty set. Each state has an ID and a display name (defaults to the ID). |
| **Initial state** | Exactly one, and it is one of the states. |
| **Final states** | A subset of the states, possibly empty. A final state may have outgoing transitions: "final" means "stopping here counts as proper termination", not "nothing happens after". Effect on checks: section 3.6. |
| **Events** | A set, possibly empty. Each event has an ID, a display name (defaults to the ID) and a **direction**: `input`, `output` or `internal`. |
| **Transitions** | A set, possibly empty. Each transition has its own ID and exactly three fields: `from` (a state), `event` (an event) and `to` (a state). `from == to` (a self-loop) is allowed. |

This text says **state machine transition** for an element of the Transitions set and **net transition** for a transition of the compiled or flattened net, wherever the difference matters.

### 1.2 What is not allowed in v1

Each of these is a load error (SM-E10) and cannot be built in the editor. None of them is ever ignored, because ignoring any of them changes the behavior and can turn a Disproved into a Proved.

- Nested (composite) states, orthogonal regions, history, entry or exit actions.
- Guards of any kind (on data, on other elements, on event parameters), `else` branches, priorities.
- Data variables, event parameters, counters.
- Effects on a transition: a transition cannot send an event in addition to its own event. To react to an input with an output, use an intermediate state (`idle --request--> working --reply--> idle`).
- Transitions without an event (completion transitions), and pseudostates (choice, junction, fork, join).
- Time events (`after 5 s`). Expiry is modeled as an `internal` event (often named `timeout`) that may happen at any time the machine is in its source state (the "timeouts as nondeterministic choice" abstraction of E5). Such an event is not the relationship attribute Timeout of the scan and not the Budget of a check (D-33).
- A per-machine event pool, deferral lists or run-to-completion queues. The only place where a message waits is the generated place of an asynchronous binding (section 3.4).

### 1.3 IDs

- State, event and transition IDs follow `CODING_STANDARDS.md` FMT-3 (created once with a collision suffix, never changed by a rename, used by every reference).
- **Name rule (normative; NS 2.2 narrowed by the Name alphabet of file-format §4.1, D-12, ADR 0020):** every ID matches `[A-Za-z_](?:[A-Za-z0-9_-]{0,46}[A-Za-z0-9_])?`, so it has 1 to 48 characters, keeps its case and does not end in `-`; it is not one of `and`, `or`, `not`; and it **does not contain `__`** (two underscores; reserved, section 10.1 F1). Anything else is SM-E05. This keeps every compiled ID a valid qualified ID (NS 2.3 rule 1) and nameable by the predicate lexer (NS 5.3, which reads a trailing `-` as a minus).
- The state, event and transition IDs of one state machine are **pairwise disjoint** (SM-E04). So `element/x` names exactly one thing: a state (a place of the compiled net), an event (a port) or a state machine transition (a net transition). This also satisfies the one-namespace rule of NS 2.2, since events never appear in the net. A clash is a model problem, never a load failure: the file loads, the editor rejects the gesture, and every scope containing the element fails L14 (D-26).
- The editor generates a transition ID from the source state and the event (`closed-failure`, then `closed-failure-2`). After that it is an opaque, stable ID.

### 1.4 Rust shape (sketch) and limits

```rust
// crates/model
pub struct StateMachine {
    states: OrdMap<StateId, State>,                  // State { name: String }
    initial: StateId,
    finals: OrdSet<StateId>,
    events: OrdMap<EventId, Event>,                  // Event { name: String, direction: Direction }
    transitions: OrdMap<SmTransitionId, SmTransition>,
}
pub struct SmTransition { from: StateId, event: EventId, to: StateId }
pub enum Direction { Input, Output, Internal }       // three values; never a bool (F7)
pub enum BehaviorModel { PetriNet(PetriNet), StateMachine(StateMachine), Unloaded { problem: Problem } }   // design §3
```

`BehaviorModel::Unloaded` is the placeholder for an element whose behavior section could not be loaded (SM-E10, SM-E12; file-format §3.3). The element stays in the Model with it, so a scope that contains it fails lowering (L1, section 8.2) instead of being checked without that element's behavior. A TOML syntax error, or a defect at the element level, makes the whole element file not load instead; file-format §10.5 then blocks every scope the element could change, so no scope is checked without it either.

Limits (`CODING_STANDARDS.md` LIM-1, LIM-2; checked before allocation, on load and in every command):

| Name | Value | Checked | Exceeded |
|---|---|---|---|
| `SM_STATES_MAX` | 1,024 per state machine | load, `apply` | SM-E12 (load error) / command Rejected |
| `SM_EVENTS_MAX` | 1,024 per state machine | load, `apply` | same |
| `SM_TRANSITIONS_MAX` | 4,096 per state machine | load, `apply` | same |
| `SCOPE_TRANSITIONS_MAX` | 65,536 net transitions per flattened net | `lower`, computed with checked arithmetic as (own and internal net transitions) + Σ over synchronous bindings of \|port(a)\| × \|port(b)\| **before any `Fusion` is allocated** | L16 (`ModelIncomplete`) |

`SCOPE_TRANSITIONS_MAX` exists because a synchronous binding is a cross product: two machines at `SM_TRANSITIONS_MAX` on one event would produce about 16.7 million fused transitions.

### 1.5 File format (content here; syntax owned by file-format §6.5)

The element file holds states, events and transitions, never the compiled net (slice 2 acceptance criterion). The stored syntax is file-format §6.5: a `[state_machine]` table with one multi-line table per state, event and transition, keyed by ID and written in sorted order (FMT-3), so that each part is its own block in a diff (file-format A6). Excerpt:

```toml
[state_machine]
initial = "closed"
final = []                       # list of state IDs; the writer omits it when empty; effect per section 3.6

[state_machine.states.half_open]
name = "HalfOpen"

[state_machine.events.timeout]
direction = "internal"

[state_machine.transitions.cool-down]
from = "open"
event = "timeout"
to = "half_open"
```

Any key not listed in file-format §6.5 inside `[state_machine]`, a missing `direction`, and any `direction` value other than the three, is a load error (SM-E10), never dropped (FMT-1).

---

## 2. Ports [S2]

### 2.1 Which parts are ports

These rules are in force (ADR 0015, D-01; NS 6.2).

- **Input and output events are transition ports.** For an input or output event `e` of element `E`, `port(E/e)` is the set of net transitions `{E/τ | τ ∈ Δ, τ.event = e}`, in canonical ID order. **Every member of `port(E/e)` is a transition port for the purposes of NS L4, L5, L7 and L11.** A Petri net transition port `E/t` is the one-element set `{E/t}`.
- **Internal events are not ports.** Their transitions fire on their own in every scope.
- **Every state is an implicit place port.** The compiled place `E/s` is a place port: it is never bound (NS 6.2, no place fusion in v1) and cannot be made a non-port, and it is what a property, template parameter or final predicate of the parent scope may name under NS L9. It is named `element/state` in the one namespace of section 1.3; there is no `element/state:` prefix. This is what lets "Breaker is in Open" be asked in a combined scope.

| Direction | Port? | Meaning | Own scope | Combined scope |
|---|---|---|---|---|
| `input` | transition port (set) | Something outside makes the machine take a transition: a call it accepts (sync) or a message it receives (async). | Fires on its own whenever its source state is active (closed port, NS 6.2). | Must be bound (L4). Fires only with its partner (sync) or by taking a message (async). |
| `output` | transition port (set) | The machine calls something (sync) or sends a message (async) while taking the transition. | Fires on its own (closed port). | Must be bound (L4). Fires only with its partner (sync), or alone, putting a message (async). |
| `internal` | no | Something inside the element happens: an expiry, a failure, a decision. | Fires on its own. | Fires on its own. Cannot be bound (L5). |

In a combined scope an input or output event must label at least one transition (SM-E11). An empty `port(E/e)` would produce no `Fusion` and no arc, so the binding would silently do nothing; SM-E11 reports that as a clear cause instead (vector V-16).

**Port kind for the UI** (inspector Ports list): the backend supplies `PortKind::{Transition, Place, InputEvent, OutputEvent, State}`; copy belongs to UX.

### 2.2 Which pairs can be bound

A relationship holds zero or more **bindings** (glossary Binding, D-29), each a pair of ports with **its own direction** (ADR 0016, D-07): one port of each of the relationship's two elements, either way around. A sync binding's `from` is the caller and its `to` the callee; an async binding's `from` is the sender and its `to` the receiver. So one relationship, drawn from the caller to the callee, carries a call and its reply (NS L11; vectors NS C3, C21; V-13c). Below, "pair" is short for binding. Every pair is checked against this table; rows are the pair's `from` port, columns its `to` port, whichever end of the relationship each sits at.

| `from` ↓ / `to` → | Petri net transition port | SM input event | SM output event | SM internal event | any place port (incl. states) |
|---|---|---|---|---|---|
| Petri net transition port | allowed (NS) | allowed | SM-B01 | L5 | never (NS 6.2) |
| SM output event | allowed | allowed | SM-B01 | L5 | never (NS 6.2) |
| SM input event | SM-B01 | SM-B01 | SM-B01 | L5 | never (NS 6.2) |
| SM internal event | L5 | L5 | L5 | L5 | never (NS 6.2) |
| any place port | never (NS 6.2) | never | never | never | never |

The table holds for synchronous and asynchronous pairs alike. Further rules per event:

- **One mode per event (SM-B04).** An event is bound only synchronously or only asynchronously.
- **Synchronous: any number of pairs.** A synchronously bound event may be bound by several pairs on several relationships; each pair yields its own fused transitions (section 4.1).
- **Asynchronous: exactly one pair (SM-B05).** An asynchronously bound event is in exactly one asynchronous pair, so it has one generated place. This is a deliberate v1 restriction, and NS L7 applies the same rule to Petri net transition ports (D-16, ADR 0016). Under the composition rule of NS 6.5 a net transition in several asynchronous bindings would get one arc from or to each generated place: as a receiver it would wait for a message **in every one of them** (an AND-join), and as a sender it would put a message into every one of them (a broadcast). Neither is what a modeler means by "receives from two senders" or "sends to two receivers"; NS C20 keeps that meaning on record for a later relaxation. To receive from two senders, declare one input event per relationship.
- **No self-binding (L6).** Both ports of a pair belong to two distinct elements of the scope.
- **Direction (SM-B01, D-07).** An output event is bound only as a pair's `from` and an input event only as its `to`, at either end of the relationship. With SM-B01, NS L7 (caller and callee at once) cannot occur for a state machine transition; with SM-B04 neither can "fused and asynchronously bound"; with SM-B05 neither can "in two asynchronous bindings".
- **Unique fused IDs (L12).** No two pairs of one relationship may produce the same fused transition ID `r/caller+callee`. For a state machine the bare names in that ID are **state machine transition** IDs, not event IDs, so the check runs on the cross products of section 4.1, not on the event names of the pairs: two pairs in opposite directions whose transitions happen to carry the same names collide (V-25).

---

## 3. Behavior with other elements (normative)

The meaning of a state machine in a scope is the meaning of its compiled net (section 4) under the net semantics. This section states the same meaning in state machine terms, so that users, the model-level replay check (section 7.4), `check_lowering` (section 5) and the reference explorer (section 12) can be written from it without reading section 4.

### 3.1 Reference semantics

A **reference tuple** of a scope is a pair `(σ, m)`:

- `σ` maps each state machine element of the scope to one of its states (its **active state**);
- `m` maps every place that is not a state place (Petri net places and generated places) to a token count.

The initial reference tuple has `σ(E) = initial(E)` and `m` = the net semantics initial marking on those places. A reference tuple whose `m` exceeds a capacity is an **overflow tuple**: it is terminal, exactly like an overflow marking (NS 3.2).

A step is the firing of one net transition label `t` (an own net transition or a fused one, NS 2.2). For each state machine transition `τ` of `E` taking part in `t`, the step requires `σ(E) = τ.from` and sets `σ(E) := τ.to`; all remaining conditions and effects (Petri net partners, generated places, capacities, `block`/`drop`/`overflow`) are those of the net semantics on `m`. Sections 3.2–3.5 spell out what that means per kind of event.

### 3.2 Own scope

All transitions fire alone. A transition `τ` can be taken iff `τ.from` is active; taking it makes `τ.to` active. Event directions do not matter: the environment may offer any input and accept any output at any time. So in its own scope a state machine deadlocks iff it can reach a state with no outgoing transition that is not final (section 3.6).

**Closed-port note (NS 6.2, 9.6; D-30), state machine wording:** every own-scope verdict of a state machine carries "every input and output event of `<element>` can happen whenever its active state has a transition on it". For an element whose state machine has no input or output event the note is omitted. It is not an assumption: that word is reserved for relationships that take no part in a check.

### 3.3 Synchronous binding (rendezvous)

For a pair of relationship `r` whose `from` is port `a` of `A` (the caller) and whose `to` is port `b` of `B` (the callee), at either end of `r`: a step of the pair is one pair `(α, β)` with `α ∈ port(A/a)` and `β ∈ port(B/b)`. Both fire together, as one step, or neither fires. A synchronously bound event never fires alone.

- **Input event.** The caller waits until the machine is in a state with a transition on that event. "Breaker accepts request only in Closed or HalfOpen": a caller that calls while Breaker is in Open waits; it is not rejected and not dropped.
- **Output event.** The machine leaves its state only when the callee takes the call at the same moment.
- **Event in several states.** An event used by k transitions offers k alternatives; exactly one state is active (Lemma 1), so at most one of them is possible at a time.

### 3.4 Asynchronous binding (bag)

For an asynchronous pair of relationship `r` whose `from` is port `a` of `A` (the sender) and whose `to` is port `b` of `B` (the receiver) there is one generated place `r` with the relationship's capacity and `when_full` (default `overflow`, ADR 0014, NS 6.4). Every transition in `port(A/a)` puts one message into `r`; every transition in `port(B/b)` takes one message from `r` (set-valued `AsyncLink`, ADR 0016, D-06). Messages are indistinguishable (ADR 0002).

- **Input event (decided, D-15).** A transition on the event can be taken iff its source state is active and `r` holds at least one message. A message that arrives while the machine is in a state with no transition on that event **waits in `r`**; it is not discarded. To ignore an event in a state, add a self-loop on it there (vector V-12). Reason: a design that silently loses messages must never be Proved by accident, and a later guard that is false means "not accepted", so it keeps the same meaning (F5). UML discards an unhandled, non-deferred event by default; that is confirmed only through secondary copies of the UML 2.x run-to-completion text, and citing the OMG UML 2.5.1 clause is a research record item (D-44), not part of this rule.
- **Output event.** The transition can be taken iff its source state is active and `r` allows it under its `when_full`: `block` needs room; `drop` and `overflow` always allow it (an `overflow` firing may produce an overflow marking).

### 3.5 Internal events in a combined scope

They fire alone whenever their source state is active. This is the only way a state machine moves without a partner in a combined scope.

**Deadlock is global (NS 7).** One element that can always move masks elements that are stuck forever: a state machine with an internal self-loop makes the whole scope deadlock-free even when two workers hold each other's locks (vector V-21). "Always reachable" [S4] catches that case.

### 3.6 Final states and the final predicate [S2]

A scope element has one **final predicate** per behavior, own and combined (`[final] own | combined`; ADR 0003, ADR 0018, D-28; UI label "done when"). It is not part of the net: lowering copies it into `NetProperty::DeadlockFree { final }` of that behavior (NS 8.1, D-04), so it changes only the deadlock-free key of that behavior and, from slice 4, the key of a "can always get back to: done" template property, whose expansion is the final predicate (ADR 0022, D-48); never the net encoding or the key of "no capacity exceeded" or of any other user property. Final predicates exist from slice 1 (D-05); a state machine's **final states** contribute to them from slice 2, when state machines arrive (D-42):

- **Own scope.** If `final` is non-empty and the user states no own final predicate for the element, the own-scope final predicate is the single atom `Σ_{f ∈ final} E/f = 1` ("E is in a final state"; one linear atom because exactly one state place holds a token, Lemma 1), for one final state as for several. This is exactly the sum atom that the slice 1 minimal form admits for state machines (D-05). A stated own final predicate replaces the default; a property cannot override either (D-41). With `final` empty and nothing stated, F = none.
- **Combined scope.** The final predicate is what the user states for the parent element; there is no default. The "done when" editor offers to prefill it with the conjunction, over the child state machines with a non-empty `final`, of their atoms above, which it may name because states are place ports (section 2.1, D-01). The prefill is a suggestion the user accepts as an ordinary Command, never applied on its own.

A deadlock-free Disproved with F = none on a terminated workload says "no transition is enabled and no final predicate is defined" and points to "done when" (NS 9.6, D-23); the panel never advises a transition back to the initial state, because a reset hides real deadlocks. `final` is validated by SM-E09 in every slice.

### 3.7 Property atoms on states [S2 in final predicates; S4 in user properties and templates]

A predicate may use `E/s` for a state `s` of state machine `E`. It denotes **1 if `s` is active and 0 otherwise** (it is about the state, not about a place; F3). In v1 it is the place `E/s`. In an own scope it is always allowed; in a combined scope it is allowed because every state is a place port (section 2.1, NS L9). Example: `breaker/open >= 1` (NS 5.3 syntax). The predicate is hashed as parsed (D-10): `breaker/open >= 1` and `breaker/open > 0` are different keys.

---

## 4. Compilation to a 1-safe part (normative)

`engine::lower` compiles each state machine into one part of `OpenNets` (NS 6.1). Bindings then become `Fusion` and `AsyncLink` values exactly as for Petri nets. The compiler is a pure function of the `StateMachine` and the element ID, uses no recursion (LIM-3) and is linear in the size of the machine.

### 4.1 Rule

For state machine `A` of element `E` with states `S`, initial `s0`, events `Ev` and transitions `Δ`:

1. **Places.** For every `s ∈ S`: place `E/s`, capacity 1, `when_full = overflow`, initial 1 if `s = s0`, else 0.
2. **Transitions.** For every `τ ∈ Δ`: net transition `E/τ` with pre `{E/τ.from ↦ 1}` and post `{E/τ.to ↦ 1}`. For a self-loop both are `{E/s ↦ 1}`.
3. **Nothing else** in the part: no complement places, no event places, no extra transitions. Events do not appear in the net.

Bindings of the scope, per bound pair (`P_a` = the port set of the pair's `from`, `P_b` = that of its `to`, section 2.1; either may sit at either end of the relationship, D-07):

- **Synchronous pair on relationship `r`.** For every `(α, β) ∈ P_a × P_b`, in canonical order, one `Fusion { relationship: r, caller: α, callee: β }`. Its fused transition ID is `r/name(α)+name(β)` with the bare state machine transition names (NS 2.2, D-31), for example `calls/call+pass`. Every transition in `P_a ∪ P_b` loses its own firing (NS 6.3). Before allocating, `lower` checks that no fused ID repeats within `r` (L12, section 2.2).
- **Asynchronous pair on relationship `r`.** One `AsyncLink { relationship: r, from: P_a, to: P_b, capacity, when_full }` with `from` and `to` non-empty sorted sets (NS 6.1, D-06). The generated place `r` gets a post arc of weight 1 from every member of `from` and a pre arc of weight 1 to every member of `to`.

**Why `overflow` for state places, and what it costs (decided, D-14).** A state place never holds more than 1 token (Lemma 1), so on a correct lowering the choice has no effect on any verdict. If lowering were wrong, `overflow` turns the defect into an overflow marking, which `sm_replay` (section 7.4) maps to Unknown `InternalError`; `block` would hide the same defect as a silent extra constraint and `drop` as a silent loss of a token. **Cost:** `verify` emits the certificate `NoOverflowPlace { drop_places }` for "no capacity exceeded" only when no place is `overflow` (NS 8.2, 9.5; D-03), so every scope that contains a state machine explores for that property (V-11a: Proved by exhaustion, 6 markings) and its Proved never carries the "every place blocks or drops" text. Scopes with state machines are small to explore, and `check_lowering` (section 5) makes the defect unlikely. Once P-invariant certificates exist (NS 9.5), the invariant of Lemma 1 is exactly such a certificate and can restore a Proved without exploring.

### 4.2 Size (normative; golden tests use it)

For one scope:

- places = Σ over state machines `|S|` + Σ over Petri nets `|P|` + one generated place per asynchronous pair;
- net transitions = (own net transitions whose event is internal, or asynchronously bound, or any own net transition in an own scope) + (Petri net transitions not in any `Fusion`) + **Σ over synchronous pairs `|P_a| × |P_b|`**.

Counting per pair, not per state machine transition, avoids double counting in state machine to state machine bindings (V-13: 2 fused transitions, not 4).

### 4.3 Lemma 1 postconditions

These are the premises of Lemma 1 (section 6). They are part of `check_lowering` (section 5, checks C1 and C4), so they hold on every lowering, not only in tests.

---

## 5. Runtime lowering check (normative; `engine::check_lowering`)

The compilation and the binding of ports are shared by both explorers (ADR 0010), so `recheck` cannot catch a lowering bug, and `sm_replay` sees only Disproved and `CapacityExceededFirst` evidence. A compiler that swaps `from` and `to`, puts the initial token on the wrong state, or leaves out a transition or a fusion would otherwise produce a **wrong Proved** (vectors LC-01 and LC-03 show it). `check_lowering(model, scope, lowered)` closes that path.

- It is written from sections 2 and 3 and the Model snapshot, **never calling `lower::state_machine`**; a module-boundary test enforces that it does not import it (the same test as for `sm_replay`).
- It runs on the derive thread's `lower-<rev>` child right after `lower` succeeds, for every scope that contains a state machine, and before any key is published (design §4, Edit step 2). It reads `Lowered.snapshot` (design §3). It is linear in the size of `OpenNets` (bounded by `SCOPE_TRANSITIONS_MAX`).
- A failure gives `KeyState::Unknown(InternalError)` for every property of the scope, is logged with the scope and the first failed check, and is never cached. A panic in it is a `lower-*` panic (ADR 0012) with the same outcome.

Checks, for every state machine element `E` of the scope:

| # | Check |
|---|---|
| C1 | The part of `E` has exactly the places `{E/s | s ∈ S}`, each with capacity 1, `overflow`, initial `[s = initial(E)]`, and exactly the net transitions `{E/τ | τ ∈ Δ}`, each with pre exactly `{E/τ.from ↦ 1}` and post exactly `{E/τ.to ↦ 1}`. |
| C2 | For every contributing synchronous pair that binds an event of `E`: the `Fusion` values with that relationship and a constituent in `port(E/e)` are exactly `port(from) × port(to)` of that pair, computed from the Model, with `E`'s output event as caller and its input event as callee (SM-B01, D-07). |
| C3 | For every contributing asynchronous pair that binds an event of `E`: there is exactly one `AsyncLink` with that relationship, its `from` and `to` are exactly the port sets of the pair's `from` and `to` computed from the Model, and its capacity and `when_full` are the relationship's (with the `overflow` default filled in). |
| C4 | No member of `port(E/e)` for an unbound event, and no net transition of an internal event, appears in any `Fusion` or `AsyncLink`; in an own scope there are no `Fusion` or `AsyncLink` values at all. |
| C5 | Every place and net transition of every part of the scope has the element prefix of that part (so no other part can touch `S_E`). |

C1, C4 and C5 are the premises of Lemma 1; C1–C3 are the premises of Lemma 2. The arcs of fused transitions and generated places are then fixed by the composition rules, which `recheck::compose` reimplements and compares bytewise (NS 6.5).

**Residual risk (ADR 0008).** `lower` and `check_lowering` are written from the same text. A misreading shared by both escapes both; the reference explorer oracle (section 12), written from section 3 rather than section 4, is the guard against that.

---

## 6. Correctness: proof sketch

Notation: `N` is the flattened net of a scope that contains state machine `A` of element `E`; `S_E = {E/s | s ∈ S}`.

**Lemma 1 (one active state).** In every marking `M ∈ R ∪ O` of `N`, `Σ_{p ∈ S_E} M(p) = 1`. Hence every state place holds 0 or 1 token and no state place is ever exceeded.

*Sketch.* `y = 1` on `S_E` (0 elsewhere) is a P-invariant: `Σ_p y(p)·(W⁺(t, p) − W⁻(p, t)) = 0` for every net transition `t`. Three kinds of `t`:
- (a) `E/τ`, possibly with arcs to generated places (not in `S_E`): exactly one pre and one post arc in `S_E`, both weight 1 (C1).
- (b) a fused transition of `E/τ` with a net transition of another element `B ≠ E` (L6): `B`'s places are disjoint from `S_E` (C5), and weights add (NS 6.3), so its restriction to `S_E` is that of `E/τ`.
- (c) any other net transition: no arc to `S_E` (C4, C5; state places are never bound, NS 6.2).

`y·M₀ = 1` (C1). `block` only disables and `overflow` fires by the incidence vector; `drop` clamps only `drop` places, and no state place is `drop`. So `y·M = 1` on every reachable marking, and non-negativity gives "exactly one place with 1".

**Lemma 2 (correspondence).** Let `h(σ, m) = M` with `M(E/s) = [σ(E) = s]` for every state machine element and `M = m` elsewhere. Then `h` is a bijection from reference tuples onto the markings of `N` that satisfy Lemma 1; `h(initial tuple) = M₀`; and `(σ, m)` steps to `(σ', m')` by label `t` iff `h(σ, m) [t⟩ h(σ', m')`.

*Sketch.* Case analysis on `t` as in Lemma 1. The state machine part of enabling, `M(E/τ.from) ≥ 1`, holds iff `σ(E) = τ.from`. The post place `E/τ.to` is `overflow`, so it adds no enabling condition (NS 4.1 E3 constrains only `block` places). Firing moves the single token from `E/τ.from` to `E/τ.to`, which is `σ'(E) = τ.to` (for a self-loop, the token stays). Overflow tuples and overflow markings correspond and are both terminal (NS 4.1 E1). Every other place, its capacity and its `when_full` are identical by construction (C2, C3). By induction on the length of runs, the reachability graphs of the reference semantics and of `N` are isomorphic with labels preserved.

**Theorem (built-in properties are preserved).** For every property, the verdict on `N` equals the verdict of the same property under the reference semantics:

- **Deadlock-free [S2].** A non-overflow marking with no enabled net transition that is not final corresponds to a reference tuple with no possible step that is not final; the final predicate's state atoms (section 3.6) map 1:1 under `h`. Deadlock stays global (section 3.5).
- **No capacity exceeded [S2].** By Lemma 1 no state place is exceeded, every other place is identical under `h`, so the sets of overflow markings, and with them `CapacityExceededFirst` (ADR 0014, ADR 0017), are the same.
- **No dead transitions [S4].** A net transition is dead iff its step is never possible in the reference semantics. Every state machine transition yields at least one net transition: exactly one in an own scope, for an internal event or an asynchronously bound event; at least one for a synchronously bound event, because L4 makes the event bound and SM-E11 makes both port sets non-empty. So no state machine transition can vanish from the check. The outcome is Unknown `CapacityExceededFirst` whenever O ≠ ∅ (D-09), and O is the same under `h`.
- **Never p, always reachable p [S4].** Predicates over markings whose state atoms map 1:1 under `h`. Lemma 2 covers overflow tuples too, so "never p", which is evaluated on overflow markings (D-02, ADR 0017), sees the same overflow markings; the isomorphism preserves reachability and therefore "from every reachable marking some p-marking or overflow marking is reachable" (NS 8.4, D-09).

**What the proof relies on at runtime.** `check_lowering` establishes its premises on every lowering (section 5). `sm_replay` checks every counterexample and every `CapacityExceededFirst` run in state machine terms (section 7.4). Golden lowering tests and the reference explorer oracle cover the rest (section 12). `check_lowering`, `sm_replay` and the oracle are the "implemented twice" of this spec.

---

## 7. Counterexamples and replay

### 7.1 Origin map additions (NS 6.7)

```rust
pub enum Origin {
    /* existing Petri net origins */
    State { element: ElementId, state: StateId },                                     // place E/s
    SmTransition { element: ElementId, transition: SmTransitionId, event: EventId },  // net transition E/τ
    // a fused ID maps to (relationship, [(caller element, caller name), (callee element, callee name)]) as in NS 6.7;
    // a constituent that is a state machine transition resolves through SmTransition
}
```

`Step.fired` is **one** qualified ID of the flattened net (NS 9.1, design §3). The constituents of a fused step are derived from the origin map at render time. The map is net → model and many-to-one; nothing may assume one net transition per state machine transition (F4).

### 7.2 Frames

For every frame `k` (frame 0 included), `ReplayFrame.activeState[E]` is the unique state `s` with `M_k(E/s) = 1`, computed from the marking, never from the transition definition, so it stays right for a stale verdict whose transitions were edited later. `M_0` is the initial marking of `Lowered.flat`.

Step text for a state machine constituent `E/τ` of `fired`:

- `from` is `activeState_{k-1}[E]` and `to` is `activeState_k[E]`, both from the markings.
- The event name comes from the current `Origin` of `E/τ`. If `E/τ` no longer exists (stale verdict), the step is dimmed with "(transition deleted)", as for other deleted parts.
- Copy (owned by UX, D-46; the facts above are normative, the wording is not): `R-STEP-SM` "Step 2 of 5: Breaker: Closed → Open on failure"; proposed `R-STEP-SM-SELF` "Step 1 of 5: Breaker: stays in Closed on request". A synchronous step adds the relationship line (`R-STEP-SYNC`), an asynchronous one the message line (`R-STEP-SEND`/`R-STEP-RECV`).
- A state machine step is never shown as place names (slice 2 acceptance criterion).

### 7.3 Violations in state machine terms

- **Deadlock waiting lines.** For a waiting net transition whose missing pre place is a state place `E/s` (`NeedsTokens` on `E/s`), the line names the state, not the place. Proposed copy `V-DIS-DL-SM` (UX, D-46): "Caller → Breaker (calls): request waits for Breaker to be in Closed; Breaker is in Open." Several waiting transitions of the same event are grouped ("…to be in Closed or HalfOpen…"). For an asynchronous input with a missing message, the message line is used instead ("waits for a message in Queue → Breaker (delivers)").
- **Dead transitions [S4].** The net-level set is mapped through the origin map: a state machine transition all of whose net transitions are dead is listed as "Breaker: HalfOpen → Closed on request never happens"; a fused transition that is dead while others of the same state machine transition are not is listed with its relationship ("…never happens through Caller B → Breaker (calls)").
- **Overflow** of a state place cannot happen (Lemma 1). If evidence claims it, `sm_replay` rejects it (section 7.4, step 3), so the user never sees "set Breaker: Closed to block or drop".

### 7.4 Model-level replay check (normative; `engine::sm_replay`)

ADR 0010 and design §7 require `engine` to check every counterexample against the state machines before frames reach the UI. `sm_replay` is an evidence check, not the canvas act the glossary calls Replay (D-33). It reads the `StateMachine` values and relationships of the **Model snapshot the check was lowered from** (`Lowered.snapshot`, design §3) and the initial marking `M_0` of `Lowered.flat` (the counterexample has no `M_0`, NS 9.1). It resolves `fired` by parsing the qualified ID against that snapshot, not through the origin map, and it must not call the compiler of section 4 (module-boundary test). It runs after `recheck::replay` has accepted the same evidence on recheck's own composition (NS 9.2 step 3b); a failure of either is Unknown `InternalError` (D-24).

It runs on:
- the steps of `Evidence::Counterexample`;
- the counterexample part of `Evidence::CounterexampleThenExhaustive`;
- the `steps` of `UnknownReason::CapacityExceededFirst`.

Input: scope, snapshot, the scope's contributing pairs, `M_0`, steps `1..n` (`fired_k`, `M_k`) and the violation (or the `CapacityExceededFirst` place). For every state machine element `E` of the scope:

1. `active_0 := initial(E)`; require `M_0(E/s) = [s = active_0]` for every state `s`.
2. For `k = 1..n`, parse `fired_k`:
   - **own shape `X/n`**: if `X` is a state machine element, `n` must be a state machine transition of `X`, and its event must be internal, asynchronously bound, or the scope must be `X`'s own scope (a synchronously bound transition never fires alone). The constituent set is `{(X, n)}`. If `X` is a Petri net element, the set has no state machine constituent.
   - **fused shape `r/a+c`**: `r` must be a contributing relationship of the scope, and exactly one synchronous pair of `r` in the snapshot must have `a` among the bare names of `port(from)` and `c` among those of `port(to)` (L12 makes it unique); its `from` element `A` and `to` element `C` are the two elements of `r` in either order (NS L11, D-07), and `(A, a)` and `(C, c)` are the constituents. For each constituent in a state machine, `a` (or `c`) must be a state machine transition whose event is that pair's port, with direction `output` as `from` and `input` as `to` (SM-B01).
   - **generated shape** (no `/`): reject (a place is not a transition).
   - For `E`: the constituents in `E` must number at most one. If one, `τ`: require `τ.from = active_{k-1}` and set `active_k := τ.to`. If none, `active_k := active_{k-1}`.
   - Require `M_k(E/s) = [s = active_k]` for every state `s`.
3. Check the end (last marking `M_n`, `M_0` if `n = 0`):
   - `Overflow { place }` and `CapacityExceededFirst { place }`: `place` must not be a state place.
   - `Deadlock`: for every state machine transition `τ` of `E` with `τ.from = active_n`, reject if it would be enabled by the section 3 rules using only `active_n` and `M_n`: own scope (any `τ`); internal event; asynchronous output to a generated place `r` with `when_full` `drop` or `overflow`, or `block` with `M_n(r) + 1 ≤ cap(r)`; asynchronous input with `M_n(r) ≥ 1`. Synchronous transitions are not rechecked here: their fused arcs are fixed by `check_lowering` (C2) and their waiting lines by `recheck::replay` against its own composition (NS 9.3).
   - Predicate violations [S4]: state atoms are already fixed by step 2; nothing more.

Any failed requirement gives Unknown `InternalError`, logged with the evidence and the snapshot, never cached.

**Placement in the check pipeline (NS 9.2 step 3b, design §4; in force, D-37).** Right after `recheck::replay` succeeds (and, for `CapacityExceededFirst`, right after its replay in step 4), on the `check-<key8>` worker. **Cache hits:** a cached Disproved served for a Model with the same CheckKey (for example after K-01) is passed through `sm_replay` again against the current snapshot before frames are built. This runs on a supervised `check-<key8>` thread spawned for it (ADR 0012), never under the Session lock. A failure is logged, evicts the cache entry and resubmits the key (design §4, Check), so the user sees the result of a full pipeline run, whose own `sm_replay` failure would be Unknown `InternalError`; the stale evidence is never shown.

---

## 8. Validation

### 8.1 Severities

- **Invariant**: `model::apply` never produces it (the command is Rejected with the message). A hand-written file can still contain it, and then it is also an Error.
- **Error**: reported by `model::validate` in the Problems list; `lower` fails with `ModelIncomplete` for every scope it affects (NS 6.6).
- **Warning**: Problems list only; never blocks lowering, never a verdict.
- **Load error**: the element's behavior does not load (SM-E10, SM-E12; file-format problem `behaviorNotLoaded`). The element stays in the Model as `BehaviorModel::Unloaded`, shown with "File has an error" (`I-FILE-ERR`), and every scope containing it fails L1. A TOML syntax error is a file-level `schema` problem (file-format §3.3), not a state machine code.

Two renderings of the same cause, never mixed: `ModelIncomplete.details` lines (NS 6.6) use **qualified IDs**, are ordered by rule number then canonical ID, and are what golden tests compare. `ProblemDto` (contract R2) carries the typed cause (`kind: 'stateMachine'`, `code`, and the element, state, event, transition and relationship IDs), and the UI renders the display-name message below from it.

### 8.2 Lowering rules for state machines (extends NS 6.6)

The state machine causes take these places in the NS 6.6 order:

| Rule | Condition | Detail line (IDs; example) |
|---|---|---|
| L1 (extended) | the element's behavior did not load | `breaker has a behavior model that did not load` |
| L4 (per event) | an input or output event is bound by no contributing relationship. **A port counts as bound when a contributing relationship names it in a pair, even if the pair's cross product is empty** (that case is SM-E11; NS L4 as amended, D-43) | `breaker/request is a port but no relationship inside this scope binds it` |
| L5 (per event) | an internal event is bound (SM-B02) | `breaker/timeout is bound but is not a port` |
| L6 (per event) | both ports of a pair are in the same element | `breaker/request is bound to breaker/notify (same element)` |
| L8 (per event) | as NS | as NS |
| L11 (per event) | the pair's `from` and `to` are not ports of the relationship's two elements, one each (either way around, D-07) | as NS |
| L12 (per fused ID) | two pairs of one relationship, in either direction, would produce the same fused ID `r/x+y`, where `x` and `y` are the bare names of the **state machine transitions** (section 2.2); one line per repeated ID | `r binds x + y twice` |
| L14 | state machine structure: SM-E01–SM-E09, SM-E11 (combined scope only) | see 8.3 |
| L15 | state machine binding rules: SM-B01, SM-B04, SM-B05 | see 8.3 |
| L16 | the flattened net would exceed `SCOPE_TRANSITIONS_MAX` | `scope sys would have 65792 transitions; the limit is 65536` |

For a state machine, L4–L8 and L11 are evaluated **per event**, never per net transition, so one unbound event gives one line, not one per transition (vector V-18); L12 alone is evaluated per fused ID, because that is what it protects (NS 2.2). An element with any L14 line is not compiled: rules that need its net transitions (L7, L10, L12, L16) skip it, and the event-level rules still run. The direction of an event (SM-B01) is reported once, as an L15 line, never also as L11. For a state machine event, every case NS L7 forbids is already an L15 line (SM-B01: caller and callee; SM-B04: fused and asynchronously bound; SM-B05: two asynchronous bindings), so L7 adds no line for it (section 2.2). For an own scope, L1, L2, L3, L10 and L14 (without SM-E11) apply.

### 8.3 Codes, details and messages

Messages follow the flows naming rules ("Element: name", relationships as "From → To (meaning)", display names). The code, condition, severity and detail line are normative; the message column is a proposal that the senior UX designer owns and may reword, never contradicting the condition (D-46). Strings that flows already has are reused verbatim.

| Code | Condition | Severity; scopes | Detail line | Message (proposal) |
|---|---|---|---|---|
| SM-E01 | No states | Invariant / Error (L14); every scope containing E | `breaker has no states` | "Breaker: the state machine has no states. Add a state and make it the initial state." |
| SM-E02 | No initial state | same | `breaker has no initial state` | "Breaker: there is no initial state. Make one state the initial state." |
| SM-E03 | Initial names a missing state | same | `breaker/halfopen (initial state) not found` | "Breaker: the initial state "halfopen" does not exist. Choose an existing state as the initial state." |
| SM-E04 | ID used more than once across states, events and transitions (a model problem; the file loads, D-26) | same | `breaker/open is used by more than one state, event or transition` | "Breaker: the ID "open" is used by more than one state, event or transition. Give each its own ID in elements/breaker.toml, then choose Reload." |
| SM-E05 | ID breaks the name rule of 1.3 | same | `breaker has an invalid ID "2fa"` | "Breaker: "2fa" is not an allowed ID. An ID starts with a letter or _, then letters, digits, - or _, up to 48 characters; it does not end in -, has no __, and is not and, or, not." |
| SM-E06 | Transition `from` or `to` names a missing state | same | `breaker/trip goes to opn, which is not a state` | "Breaker: transition trip goes to "opn", which is not a state of Breaker. Choose an existing state." |
| SM-E07 | Transition names an undeclared event | same | `breaker/trip uses falure, which is not an event` | "Breaker: transition trip uses "falure", which is not an event of Breaker. Choose an existing event or add it." |
| SM-E08 | Two transitions with the same `from`, `event` and `to` | same | `breaker/trip and breaker/trip-2 have the same from, event and to` | "Breaker: transitions trip and trip-2 both go from Closed to Open on failure. Delete one of them." |
| SM-E09 | `final` names a missing state | same | `breaker/done (final state) not found` | "Breaker: the final state "done" does not exist. Remove it from final or add the state." |
| SM-E10 | Unsupported key (`guard`, `parent`, `states` inside a state, `entry`, `exit`, `effect`, `history`, `variables`, `regions`, …), missing or unknown `direction` | Load error | (L1 line) | "elements/breaker.toml, line 22: transition trip has guard, which this version of the app does not support. Remove it, or open the project with the version that wrote it." / "…event reset has direction "both"; use input, output or internal." |
| SM-E11 | Input or output event used by no transition | Error (L14) in combined scopes containing E; Warning SM-W03 otherwise | `breaker/reset is an input event that no transition uses` | "Breaker: event reset is not used by any transition, so it can never happen. Add a transition on reset or delete the event." |
| SM-E12 | More states, events or transitions than 1.4 allows | Load error | (L1 line) | "elements/breaker.toml: Breaker has 5,000 transitions; the limit is 4,096." |
| SM-B01 | Input event as a pair's `from`, or output event as its `to`, at either end of the relationship (D-07) | Invariant (BindPorts Rejected) / Error (L15) | `calls binds breaker/request as from, but it is an input event` | `I-REL-INCOMPAT`: "Breaker: request and Caller: call cannot be bound this way: request is an input event, so Breaker cannot call or send with it. Swap the binding so that Caller: call calls Breaker: request, or use an output event." |
| SM-B02 | Internal event bound | same, as L5 | (L5 line) | `I-REL-INCOMPAT`: "…cannot be bound: timeout is an internal event of Breaker, not a port. Change its direction to input or output first." |
| SM-B03 | *Removed.* Binding any place port is impossible under NS 6.2; the editor never offers place ports as binding targets, and a hand-written file fails L5 ("is bound but is not a port"). | — | — | — |
| SM-B04 | Event bound both synchronously and asynchronously | same, L15 | `breaker/request is bound synchronously (calls) and asynchronously (deliveries)` | "Breaker: request is bound synchronously by Caller → Breaker (calls) and asynchronously by Queue → Breaker (delivers). Bind an event one way only; add a second event for the other." |
| SM-B05 | Asynchronously bound event in more than one asynchronous pair (the state machine form of NS L7, D-16) | same, L15 | `breaker/request is in 2 asynchronous bindings: deliveries, retries` | "Breaker: request receives from 2 asynchronous relationships. An event can receive from one asynchronous relationship only; add one input event per relationship." |
| SM-B06 | Pair binds two events of the same state machine | same, as L6 | (L6 line) | "A relationship cannot bind two events of Breaker." |
| SM-B07 | Input or output event not bound, combined scope | Error, as L4 (`U-INC-UNBOUND`) | (L4 line) | "Breaker: request is not bound by any relationship. Bind it in the relationship inspector, or make it internal if nothing in this view causes it." |
| SM-W01 | State not reachable from the initial state over the machine's own transitions, ignoring events | Warning | — | "M: state c cannot be reached from the initial state." (uses the display name, which defaults to the ID) |
| SM-W02 | Internal event used by no transition | Warning | — | "Breaker: event tick is not used by any transition." |
| SM-W03 | Input or output event used by no transition, outside a combined scope | Warning | — | as SM-E11 |
| SM-L16 | Scope too large | Error (L16) | (L16 line) | "This view would need 65,792 transitions to check; the limit is 65,536. Reduce the transitions on calls or split the view." |

Notes:

- SM-W01 is sound as a definite statement: bindings only remove possibilities, so a state unreachable in the machine alone is unreachable in every composition. It is computed with an explicit queue (LIM-3). The converse is not claimed; a state reachable in the machine may be unreachable in a scope, which "no dead transitions" [S4] finds.
- There is deliberately no problem for "state has no outgoing transition and is not final": that is the deadlock-free check's job, and a problem would be a verdict in disguise.
- There is no SM code for a fused-ID repeat: it is NS L12 (section 8.2). `model::apply` Rejects every gesture that would create one (binding a pair, adding a transition on a bound event, changing a transition's event), proposed copy "A → B (calls) would bind x + y twice; give one of the transitions another ID." A hand-written file reaches it only through lowering.
- **Editor rules that keep the invariants.** Choosing "State machine" creates one state, "State 1", as the initial state. Deleting a state deletes its transitions in the same gesture (one undo step). Deleting the initial state is Rejected: "Make another state the initial state first." Deleting an event that transitions use is Rejected: "Event failure is used by 2 transitions. Delete them first." Changing a bound event's direction in a way that breaks 2.2 is Rejected with the SM-B01 or SM-B02 reason.

---

## 9. CheckKey and currency

The CheckKey is BLAKE3 of `canonical(flat) ++ canonical_property(p)` (NS 11). **Fitness rule for state machines: a Model change changes the key iff it changes `canonical(flat)` or the property** (for deadlock-free, the property includes its final predicate, D-04). The order of the `parts`, `fusions` or `links` of `OpenNets` may change without changing the key.

| Change | Key changes? |
|---|---|
| Add or remove a state or transition; change `from`, `to` or the initial state | yes |
| Change an event's direction or binding in a combined scope (changes fusions or asynchronous bindings) | yes, or lowering fails |
| Draw the relationship the other way while every pair keeps its `from` and `to` (D-07) | no (V-20b) |
| Capacity or `when_full` of the asynchronous relationship | yes |
| Final states, own scope, no stated own final predicate | the own-scope deadlock-free key only, through the default final predicate of 3.6; the net encoding and every other key stay (K-07) |
| Final states, combined scope | no: the combined final predicate is what the user states (3.6) |
| Display name of a state, event or transition | no |
| Replace an event by another with a different ID, or change which event a transition uses, in the own scope | no: events do not appear in the net and all fire alone in the own scope |
| Event direction in the own scope | no, same reason |
| Layout of the state machine editor | no |

The "no" rows for the own scope are correct, not a loophole: the behavior is identical, and replay text resolves event names from the current Model (section 7.2), with `sm_replay` rerun on cache hits (section 7.4).

---

## 10. Forward compatibility: nested states and guards

### 10.1 Decided now, so that later is additive

- **F1. IDs are unique per machine**, not per parent, and `__` is reserved in every name inside a behavior model (1.3; file-format §4.1, D-12). A state moved into a composite keeps its ID, its place ID and every property reference. Per-leaf copies of a transition from a composite state can then be named inside the current slug grammar (for example `τ__leaf`) without renaming anything and without amending NS 2.2 or bumping `petrinet-net 1`. The exact naming scheme is not decided now.
- **F2. Transitions have explicit IDs.** With guards, two transitions with the same `from`, `event` and `to` but different guards are legitimate; SM-E08 is then relaxed, and every v1 file stays valid.
- **F3. A state atom means "this state is active"** (3.7), not "this place". A composite state's atom becomes the sum of its leaves: still one linear atom (ADR 0003).
- **F4. The origin map is many-to-one** (net → model). Nothing in `engine`, the DTOs or the UI assumes one net transition per state machine transition, or one place per state.
- **F5. An event that is not accepted waits** (3.3, 3.4; D-15). A guard that is false later means "not accepted", so it waits too. This must stay true, or adding guards would change the meaning of guard-free files.
- **F6. Unknown keys are rejected on load**, never ignored (SM-E10, FMT-1). An older app never checks a newer file with its guards dropped.
- **F7. Direction is a three-valued enum.** A later direction (for example a request/reply pair) is a new variant, not a reinterpretation of a bool.
- **F8. "Exactly one token per state machine" lives only in `engine`** (`check_lowering`, `sm_replay`). `net`, `verify` and `recheck` never assume it. Orthogonal regions later have one token per region and are still 1-safe.
- **F9. The TOML shape is keyed tables with inline fields.** `parent = "…"` on a state, `guard = "…"` on a transition, or `regions` are added keys behind a format version bump with a migration (FMT-1), not a restructure.
- **F10. `ReplayFrame.activeState` stays `Record<ElementId, StateId>` in v1.** With regions or nesting it becomes the set of active leaf states: an internal DTO change (ts-rs, one app), accepted now and not pre-built.
- **F11. Port sets.** An event is a set of net transition ports (set-valued `AsyncLink`, ADR 0016). Nested states only make the sets larger.

### 10.2 Deliberately left open (do not decide now)

- Semantics of nested states: UML-style entry and exit order and history, or plain leaf flattening.
- What guards read: bounded data variables (compiled to one place per value), other elements' states (read arcs, which still need ports, ADR 0002), or both.
- Whether transitions get effects (send in the same step). The v1 one-event-per-transition form stays valid either way.
- Event parameters.
- An explicit per-state "ignore" list as sugar for self-loops.
- A per-machine event pool with run-to-completion. It would change the meaning of existing files, so if it ever comes it is a new opt-in kind, never a default.
- Time events and timed analysis (E5: time is the real gap, out of v1).
- Lifting SM-B04 (both modes) and SM-B05 (asynchronous fan-in), together with NS L7 for Petri net ports (D-16); NS C8 and C20 keep the meaning on record, so the relaxation is additive.

---

## 11. Test vectors

All values were computed with the throwaway reference implementation named in the header, under the normative exploration order of NS 12, and both explorers and the reference explorer of section 12 must reproduce them. Columns follow NS 14: **len** is the counterexample length; **stored** is the number of stored markings at discovery (violating marking included); for Disproved-by-counterexample rows **#R** and **#O** are oracle facts from a full exploration; for Proved and Unknown rows they are what the explorers report. Technique "exhaustive" is stated for "no capacity exceeded" because no scope with a state machine gets the `NoOverflowPlace` certificate (4.1, D-14). Fused IDs use bare names. A deadlock-free row without "F =" has F = none.

### 11.1 Shared machines and nets

Notation: `a --e--> b` is a transition; directions `in`, `out`, `int`. Petri nets in NS 2.4 notation.

- **CALLER(E, ports)**: Petri net `E/idle(1, block, 1)`, each transition port `E/p: E/idle → E/idle`. CALLER(E) has the one port `call`.
- **PRODUCER**: `producer/p(1, block, 1)`, transition port `producer/send: producer/p → producer/p`.
- **SWITCH**: `switch`: off (initial), on; `press` in; `turn_on: off --press--> on`, `turn_off: on --press--> off`.
- **CONN**: `conn`: idle (initial), connected, closed; `open`, `close` in; `connect: idle --open--> connected`, `disconnect: connected --close--> closed`.
- **M**: `m`: a (initial), b, c; `x`, `y`, `z` int; `a_x: a --x--> b`, `b_y: b --y--> a`, `c_z: c --z--> a`.
- **BREAKER5**: `breaker`: closed (initial), open, half_open; `failure`, `success` in; `timeout` int; `trip: closed --failure--> open`, `cool_down: open --timeout--> half_open`, `recover: half_open --success--> closed`, `retrip: half_open --failure--> open`, `ok: closed --success--> closed`.
- **BREAKER2**: `breaker`: closed (initial), open; `request` in; `fail`, `cool` int; `accept: closed --request--> closed`, `trip: closed --fail--> open`, `reset: open --cool--> closed`. **BREAKER2-NR**: BREAKER2 without `reset` and without event `cool`.
- **BREAKER3**: `breaker`: closed (initial), open, half_open; `request` in; `fail`, `timeout` int; `pass: closed --request--> closed`, `trip: closed --fail--> open`, `cool_down: open --timeout--> half_open`, `probe: half_open --request--> closed`, `retrip: half_open --fail--> open`.
- **CONSUMER**: `consumer`: idle (initial), busy; `msg` in; `done` int; `take: idle --msg--> busy`, `finish: busy --done--> idle`.
- **STOPPER**: `consumer`: ready (initial), stopped; `msg` in; `stop` int; `handle: ready --msg--> ready`, `halt: ready --stop--> stopped`.
- **PAUSER**: `consumer`: ready (initial), paused; `msg` in; `hold`, `go` int; `run: ready --msg--> ready`, `skip: paused --msg--> paused`, `pause: ready --hold--> paused`, `resume: paused --go--> ready`.
- **CLIENT**: `client`: idle (initial), waiting; `req` out, `resp` in; `ask: idle --req--> waiting`, `got: waiting --resp--> idle`. **SERVER**: `server`: free (initial), working; `req` in, `resp` out; `take: free --req--> working`, `answer: working --resp--> free`.
- **LOCK(L)**: `L`: free (initial), held; `acquire`, `release` in; `take: free --acquire--> held`, `give: held --release--> free`.
- **WORKER(W, x, y)**: `W`: s0 (initial), s1, s2, s3; `get1`, `get2`, `rel1`, `rel2` out; `t1: s0 --get<x>--> s1`, `t2: s1 --get<y>--> s2`, `t3: s2 --rel<y>--> s3`, `t4: s3 --rel<x>--> s0`.
- **LOCKS-SM(order)**: LOCK(l1), LOCK(l2), WORKER(a, 1, 2), and WORKER(b, 2, 1) for order **yx** or WORKER(b, 1, 2) for **xy**; eight synchronous relationships `W-get-lK: W/getK → lK/acquire` and `W-rel-lK: W/relK → lK/release` for W ∈ {a, b}, K ∈ {1, 2}.
- **TICKER**: `ticker`: on (initial); `tick` int; `beat: on --tick--> on`.
- **PINGER / PONGER**: `pinger`: a (initial), b; `ping` out; `p1: a --ping--> b`, `p2: b --ping--> a`. `ponger`: x (initial), y; `ping` in; `q1: x --ping--> y`, `q2: y --ping--> x`.

### 11.2 Property vectors

| # | scope | places / net transitions | property | expected (technique) | len / detail | stored | #R | #O | slice |
|---|---|---|---|---|---|---|---|---|---|
| V-01a | SWITCH, own | 2 / 2 | deadlock-free | Proved (exhaustive) | — | — | 2 | 0 | S2 |
| V-01b | same | 2 / 2 | no capacity exceeded | Proved (exhaustive; no certificate) | — | — | 2 | 0 | S2 |
| V-01c | same | 2 / 2 | no dead transitions | Proved | — | — | 2 | 0 | S4 |
| V-02 | CONN, own | 3 / 2 | deadlock-free | Disproved | 2 (`conn/connect`, `conn/disconnect`); last `conn/closed = 1`; waiting `conn/connect ← conn/idle NeedsTokens{1,0}`, `conn/disconnect ← conn/connected NeedsTokens{1,0}`; text "no transition is enabled and no final predicate is defined" (no pre set holds a token; NS 9.6, D-23); steps "Conn: Idle → Connected on open", "Conn: Connected → Closed on close"; caveat `V-DIS-NOFINAL`, next step "define done when" | 3 | 3 | 0 | S2 |
| V-03 | CONN, `final = ["closed"]`, own, no stated own final predicate (F = default `conn/closed = 1`, 3.6) | 3 / 2 | deadlock-free | Proved | — | — | 3 | 0 | S2 |
| V-04a | M, own | 3 / 3 | deadlock-free | Proved | — | — | 2 | 0 | S2 |
| V-04b | same | 3 / 3 | no dead transitions | Disproved (`Exhaustive { dead: [m/c_z], markings: 2 }`) | warning SM-W01 "M: state c cannot be reached from the initial state." | — | 2 | 0 | S4 |
| V-05a | BREAKER5, own | 3 / 5 | deadlock-free | Proved | — | — | 3 | 0 | S2 |
| V-05b | same | 3 / 5 | no capacity exceeded | Proved (exhaustive) | — | — | 3 | 0 | S2 |
| V-05c | same | 3 / 5 | no dead transitions | Proved | — | — | 3 | 0 | S4 |
| V-05d | same | 3 / 5 | never `breaker/open >= 1` | Disproved | 1 (`breaker/trip`) | 2 | 3 | 0 | S4 |
| V-05e | same | 3 / 5 | always reachable `breaker/closed = 1` | Proved | — | — | 3 | 0 | S4 |
| V-06a | CALLER(caller) + BREAKER2; sync `calls`: `caller/call → breaker/request` | 3 / 3 (`breaker/reset`, `breaker/trip`, `calls/call+accept`) | deadlock-free | Proved | — | — | 2 | 0 | S2 |
| V-06b | same | 3 / 3 | no capacity exceeded | Proved (exhaustive) | — | — | 2 | 0 | S2 |
| V-06c | same | 3 / 3 | no dead transitions | Proved | — | — | 2 | 0 | S4 |
| V-06d | same | 3 / 3 | always reachable `breaker/closed = 1` (state place port, D-01) | Proved | — | — | 2 | 0 | S4 |
| V-07a | CALLER(caller) + BREAKER2-NR; sync `calls` | 3 / 2 (`breaker/trip`, `calls/call+accept`) | deadlock-free | Disproved | 1 (`breaker/trip`); last `breaker/open = 1, caller/idle = 1`; waiting `breaker/trip ← breaker/closed NeedsTokens{1,0}`, `calls/call+accept ← breaker/closed NeedsTokens{1,0}`; text "no transition is enabled"; step "Breaker: Closed → Open on fail"; waiting line "Caller → Breaker (calls): request waits for Breaker to be in Closed; Breaker is in Open." | 2 | 2 | 0 | S2 |
| V-07b | same | 3 / 2 | no dead transitions | Proved | — | — | 2 | 0 | S4 |
| V-07c | same | 3 / 2 | always reachable `breaker/closed = 1` | Disproved (`CounterexampleThenExhaustive`) | 1 (`breaker/trip`); `markings` = 1 | — | 2 | 0 | S4 |
| V-08a | CALLER(caller) + BREAKER3; sync `calls`: `caller/call → breaker/request` | 4 / 5 (`breaker/cool_down`, `breaker/retrip`, `breaker/trip`, `calls/call+pass`, `calls/call+probe`) | deadlock-free | Proved | — | — | 3 | 0 | S2 |
| V-08b | same | 4 / 5 | no dead transitions | Proved | — | — | 3 | 0 | S4 |
| V-09a | CALLER(caller_a) + CALLER(caller_b) + BREAKER3; sync `a_calls`, `b_calls` to `breaker/request` | 5 / 7 (`a_calls/call+pass`, `a_calls/call+probe`, `b_calls/call+pass`, `b_calls/call+probe`, 3 internal) | deadlock-free | Proved | — | — | 3 | 0 | S2 |
| V-09b | same | 5 / 7 | no dead transitions | Proved | — | — | 3 | 0 | S4 |
| V-10a | PRODUCER + CONSUMER; async `jobs`: `producer/send → consumer/msg`, capacity 2, `overflow` | 4 / 3 | no capacity exceeded | Disproved | 3 (`producer/send` ×3); `Overflow { jobs, 2, 3 }`; "Producer → Consumer (…) holds 3 messages; capacity 2." | 6 | 6 | 2 | S2 |
| V-10b | same | 4 / 3 | deadlock-free | Unknown `CapacityExceededFirst { jobs, 3 steps }` | `sm_replay` accepts the run | — | 6 | 2 | S2 |
| V-10c | same | 4 / 3 | no dead transitions | Unknown `CapacityExceededFirst { jobs }` | — | — | 6 | 2 | S4 |
| V-11a | V-10 with `jobs` `block` | 4 / 3 | no capacity exceeded | Proved (exhaustive; no certificate, state places are `overflow`) | — | — | 6 | 0 | S2 |
| V-11b | same | 4 / 3 | deadlock-free | Proved | — | — | 6 | 0 | S2 |
| V-11c | same | 4 / 3 | no dead transitions | Proved | — | — | 6 | 0 | S4 |
| V-11d | V-10 with `jobs` `drop` | 4 / 3 | no capacity exceeded | Proved (exhaustive) | — | — | 6 | 0 | S2 |
| V-11e | same | 4 / 3 | deadlock-free | Proved | — | — | 6 | 0 | S2 |
| V-12a | PRODUCER + STOPPER; async `jobs` capacity 1, `block` | 4 / 3 | deadlock-free | Disproved | 2 (`consumer/halt`, `producer/send`); last `consumer/stopped = 1, jobs = 1, producer/p = 1`: the message is kept, not discarded; waiting `consumer/halt ← consumer/ready NeedsTokens{1,0}`, `consumer/handle ← consumer/ready NeedsTokens{1,0}`, `producer/send ← jobs OutputFull{1,2}` | 4 | 4 | 0 | S2 |
| V-12b | same | 4 / 3 | no capacity exceeded | Proved (exhaustive) | — | — | 4 | 0 | S2 |
| V-13a | CLIENT + SERVER; sync `requests`: `client/req → server/req`; sync `replies`: `server/resp → client/resp` | 4 / 2 (`replies/answer+got`, `requests/ask+take`) | deadlock-free | Proved | — | — | 2 | 0 | S2 |
| V-13b | same | 4 / 2 | no dead transitions | Proved | — | — | 2 | 0 | S4 |
| V-13c | CLIENT + SERVER on **one** relationship `rpc` drawn from `client` to `server`, with sync pairs `client/req → server/req` (call) and `server/resp → client/resp` (reply, its `from` at the target; D-07) | 4 / 2 (`rpc/answer+got`, `rpc/ask+take`) | deadlock-free | Proved | — | — | 2 | 0 | S2 |
| V-14a | LOCKS-SM(yx) | 12 / 8 (all fused) | deadlock-free | Disproved | 2 (`a-get-l1/t1+take`, `b-get-l2/t1+take`); last `a/s1 = 1, b/s1 = 1, l1/held = 1, l2/held = 1`; waiting (all NeedsTokens{1,0}): `a-get-l1/t1+take ← a/s0, l1/free`; `a-get-l2/t2+take ← l2/free`; `a-rel-l1/t4+give ← a/s3`; `a-rel-l2/t3+give ← a/s2`; `b-get-l1/t2+take ← l1/free`; `b-get-l2/t1+take ← b/s0, l2/free`; `b-rel-l1/t3+give ← b/s2`; `b-rel-l2/t4+give ← b/s3`; both workers and both locks highlighted (NS 6.7 rule) | 5 | 10 | 0 | S2 |
| V-14b | same | 12 / 8 | no capacity exceeded | Proved (exhaustive) | — | — | 10 | 0 | S2 |
| V-15a | LOCKS-SM(xy) | 12 / 8 | deadlock-free | Proved | — | — | 7 | 0 | S2 |
| V-15b | same | 12 / 8 | no dead transitions | Proved | — | — | 7 | 0 | S4 |
| V-16 | CALLER(caller, [call, admin]) + `breaker`: closed (initial); `request`, `reset` in; only `accept: closed --request--> closed`; sync `calls: caller/call → breaker/request`, sync `resets: caller/admin → breaker/reset` | lowering fails | any | Unknown `ModelIncomplete`, details exactly [`breaker/reset is an input event that no transition uses`] | Without SM-E11 the binding would produce no `Fusion`, `caller/admin` would stay as its own net transition (2 places, 2 net transitions) and NS L4 would fail with the misleading "caller/admin is a port but no relationship inside this scope binds it". SM-E11 is the clearer cause, not a soundness guard; L4 counts the pair as binding `caller/admin` (8.2). | — | — | — | S2 |
| V-17 | CLIENT, own | 2 / 2 | deadlock-free | Proved (input and output events fire alone; closed-port note 3.2) | — | — | 2 | 0 | S2 |
| V-18a | V-06 with relationship `calls` deleted, combined scope | lowering fails | any | Unknown `ModelIncomplete`, details exactly [`breaker/request is a port but no relationship inside this scope binds it`, `caller/call is a port but no relationship inside this scope binds it`] (one line per event, not per net transition) | — | — | — | — | S2 |
| V-18b | BREAKER2, own scope | 2 / 3 | deadlock-free | Proved | — | — | 2 | 0 | S2 |
| V-19 | V-06 where `elements/breaker.toml` has `guard = "x > 0"` on a transition | breaker not loaded | any | load error SM-E10; Unknown `ModelIncomplete` [`breaker has a behavior model that did not load`]; never checked with the guard ignored | — | — | — | — | S2 |
| V-20 | V-06 with the pair reversed in a hand-written file: binding `breaker/request → caller/call` (`from` = the input event) on `calls`, whichever way `calls` is drawn | lowering fails | any | `BindPorts` is Rejected with the SM-B01 copy in the editor; for the file: Unknown `ModelIncomplete` [`calls binds breaker/request as from, but it is an input event`] | — | — | — | — | S2 |
| V-20b | V-06 with relationship `calls` drawn from `breaker` to `caller`, binding still `caller/call → breaker/request` (D-07) | 3 / 3 (`breaker/reset`, `breaker/trip`, `calls/call+accept`); encoding identical to V-06's | deadlock-free | Proved | — | — | 2 | 0 | S2 |
| V-21a | LOCKS-SM(yx) + TICKER (no ports, no bindings) | 13 / 9 | deadlock-free | Proved: a and b are stuck forever, the scope never stops (NS 7, V43) | — | — | 10 | 0 | S2 |
| V-21b | same | 13 / 9 | always reachable `a/s0 = 1` | Disproved | 2 (`a-get-l1/t1+take`, `b-get-l2/t1+take`); `markings` = 1 | — | 10 | 0 | S4 |
| V-22a | PRODUCER + PAUSER; async `jobs`: `producer/send → consumer/msg`, capacity 2, `block` (`to` = {`consumer/run`, `consumer/skip`}; NS C17) | 4 / 5 | deadlock-free | Proved | — | — | 6 | 0 | S2 |
| V-22b | same | 4 / 5 | no capacity exceeded | Proved (exhaustive) | — | — | 6 | 0 | S2 |
| V-23a | PINGER + PONGER; async `pings`: `pinger/ping → ponger/ping`, capacity 1, `block` (sets on both ends; NS C18) | 5 / 4 | deadlock-free | Proved | — | — | 4 | 0 | S2 |
| V-23b | same | 5 / 4 | no capacity exceeded | Proved (exhaustive) | — | — | 4 | 0 | S2 |
| V-23c | V-23 with `pings` `overflow` | 5 / 4 | no capacity exceeded | Disproved | 2 (`pinger/p1`, `pinger/p2`); `Overflow { pings, 1, 2 }` | 3 | 4 | 2 | S2 |
| V-23d | same | 5 / 4 | deadlock-free | Unknown `CapacityExceededFirst { pings, 2 steps }` | — | — | 4 | 2 | S2 |
| V-24 | scope `sys`: `a` with 257 states, each with a self-loop on output event `e`; `b` with 256 states, each with a self-loop on input event `f`; sync `r: a/e → b/f` | lowering fails before any `Fusion` is allocated | any | Unknown `ModelIncomplete` [`scope sys would have 65792 transitions; the limit is 65536`] | — | — | — | — | S2 |
| V-25 | `a`: s0 (initial), s1; `e` out, `h` in; `x: s0 --e--> s1`, `y: s1 --h--> s0`. `b`: t0 (initial), t1; `f` in, `g` out; `y: t0 --f--> t1`, `x: t1 --g--> t0`. Relationship `r` from `a` to `b` with sync pairs `a/e → b/f` and `b/g → a/h` | lowering fails | any | Unknown `ModelIncomplete`, details exactly [`r binds x + y twice`]: both pairs would produce `r/x+y` (L12 on transition names, 2.2). Without the engine check, `recheck::compose` rejects the duplicate ID and the outcome is `InternalDisagreement`, never a verdict | — | — | — | — | S2 |

### 11.3 Composition vectors (NS C17–C20)

The composition vectors of set-valued asynchronous bindings are NS vectors since revision 3 (D-06): **C17** (V-22: PRODUCER and the compiled PAUSER, `to` = [`consumer/run`, `consumer/skip`]), **C18** (V-23: PINGER and PONGER, sets on both ends), **C19** (an empty set: engine never produces it, SM-E11; recheck `ComposeError`) and **C20** (one transition in two `to` sets: engine fails L7, SM-B05 for a state machine event; recheck `ComposeError`; the AND-join meaning is on record). NS 14.3 holds their exact encodings, which have no `final` line (ADR 0018); this spec does not repeat them. The state machine golden tests assert them through both `engine::flatten` and `recheck::compose`.

### 11.4 Key vectors (ADR 0013 fitness)

Each asserts the canonical encoding (net lines, then the property line, NS 11), not only key equality.

| # | change | expected |
|---|---|---|
| K-01 | V-01: replace event `press` by an event with ID `toggle` (both transitions use it) | encoding identical to V-01's: `petrinet-net 1` / `place switch/off 1 overflow 1` / `place switch/on 1 overflow 0` / `transition switch/turn_off` / `pre switch/on 1` / `post switch/off 1` / `transition switch/turn_on` / `pre switch/off 1` / `post switch/on 1`; deadlock-free line `property deadlock-free final none` |
| K-02 | V-01: `press` direction `input` → `internal` | identical (own scope) |
| K-03 | V-01: initial state `on` | differs: `place switch/off 1 overflow 0`, `place switch/on 1 overflow 1` |
| K-04 | V-06: display names changed and states, events and transitions listed in another order in the file | identical to V-06's encoding (places `breaker/closed 1 overflow 1`, `breaker/open 1 overflow 0`, `caller/idle 1 block 1`; transitions `breaker/reset`, `breaker/trip`, `calls/call+accept`) |
| K-05 | V-08: retarget `probe` to `half_open → open` | differs: `calls/call+probe` posts `breaker/open 1` instead of `breaker/closed 1` |
| K-06 | V-22: `jobs` `block` → `drop` | differs: `place jobs 2 drop 0` |
| K-07 | CONN own scope: `final = ["closed"]` added, no stated own final predicate (V-02 → V-03) | net encoding identical (`petrinet-net 1` / `place conn/closed 1 overflow 0` / `place conn/connected 1 overflow 0` / `place conn/idle 1 overflow 1` / `transition conn/connect` / `pre conn/idle 1` / `post conn/connected 1` / `transition conn/disconnect` / `pre conn/connected 1` / `post conn/closed 1`); the deadlock-free line changes from `property deadlock-free final none` to `property deadlock-free final (atom eq 1 conn/closed 1)`; the `property capacity-never-exceeded` key is unchanged (D-04, D-42) |

### 11.5 Lowering check vectors (section 5)

Each uses a test-only faulty compiler; `check_lowering` must turn it into Unknown `InternalError` before any worker runs. The column "without check_lowering" is what both explorers would report on the faulty net (recomputed), showing why the check exists.

| # | fault, on scope | without `check_lowering` | expected |
|---|---|---|---|
| LC-01 | V-07: `breaker/trip` compiled with `from`/`to` swapped | deadlock-free **Proved**, 1 marking (wrong; truth is Disproved) | Unknown `InternalError` (C1) |
| LC-02 | V-07: initial token on `breaker/open` | Disproved in 0 steps | Unknown `InternalError` (C1); `sm_replay` would also reject (step 1) |
| LC-03 | V-07: `breaker/trip` omitted | deadlock-free **Proved**, 1 marking (wrong) | Unknown `InternalError` (C1) |
| LC-04 | V-09: fusion `b_calls/call+probe` omitted | — | Unknown `InternalError` (C2) |
| LC-05 | V-06: state place `breaker/open` compiled as `block` | — | Unknown `InternalError` (C1) |
| LC-06 | V-22: `consumer/skip` missing from `AsyncLink.to` | — | Unknown `InternalError` (C3) |
| LC-07 | V-06: internal `breaker/trip` added as callee of a second fusion on `calls` | — | Unknown `InternalError` (C4) |

### 11.6 Model-level replay check vectors (section 7.4)

The evidence is mutated, then fed to `sm_replay`; every mutation gives Unknown `InternalError`.

| # | mutation | expected |
|---|---|---|
| R-01 | V-07a with step 1 `fired` = `breaker/accept` | rejected (2: a synchronously bound transition fired alone) |
| R-02 | V-07a with `M_1` claiming `breaker/closed = 1, breaker/open = 1` | rejected (2: active state mismatch) |
| R-03 | V-02 with the two steps swapped | rejected (2: `from` ≠ active state) |
| R-04 | V-05 with a deadlock claimed after step `breaker/trip` | rejected (3: `breaker/cool_down` fires alone in the own scope) |
| R-05 | V-02 run with violation `Overflow { conn/closed, 1, 2 }` | rejected (3: state place) |
| R-06 | V-10b with `CapacityExceededFirst.place` = `consumer/idle` | rejected (3: state place), so the user never sees "set Consumer: Idle to block or drop" |
| R-07 | V-12 with a deadlock claimed after `producer/send` alone (`consumer/ready = 1, jobs = 1`) | rejected (3: asynchronous input with a message waiting) |
| R-08 | V-07a with `fired` = `callz/call+accept` | rejected (2: relationship not found) |
| R-09 | V-07a with `fired` = `calls/accept+call` | rejected (2: no synchronous pair of `calls` has `accept` in its `from` port and `call` in its `to` port) |
| R-10 | V-14a unchanged | accepted; frame 2 `activeState` = `a: s1, b: s1, l1: held, l2: held` |
| R-11 | V-02 cached Disproved, then K-01-style event replacement in CONN (key unchanged) | accepted on the cache hit; step text uses the new event names |
| R-12 | V-14a with all eight relationships drawn from the lock to the worker, every pair unchanged (D-07) | accepted with R-10's frames: constituents come from the pair, not from the drawing direction |

---

## 12. Test plan

- **`model`.** Proptest: `apply` never produces SM-E01–E09, SM-B01, SM-B02, SM-B04–B06. Deleting a state cascades its transitions in one undo step. Every Rejected carries its message. SM-E05 accepts exactly the 1.3 name rule (rejects `2fa`, `not`, `a__b`, `open-`, a 49-character ID; accepts `half_open`, `cool-down`, `HalfOpen`, a 48-character ID). `apply` Rejects every gesture that would repeat a fused ID (V-25).
- **`format`.** Round trip and canonical output (TEST-3 snapshots) for BREAKER5; every SM-E10 key, a missing and an unknown direction are load errors with file and line, and the element becomes `Unloaded`; SM-E12 limits are checked before tables are parsed; a golden fixture for the first format version with state machines.
- **`engine` golden lowering.** One test per section 4 rule (state places, initial marking, self-loop, port sets, sync cross product, async sets, internal events, own-scope closure) asserting the exact `OpenNets`, the 4.2 sizes, and the detail lists of V-16, V-18a, V-19, V-20, V-24, V-25; V-20b asserts that the drawing direction of a relationship does not change the encoding.
- **`check_lowering`.** LC-01–LC-07, plus a proptest that it accepts every lowering of the random scopes below. Module-boundary test: neither `check_lowering` nor `sm_replay` imports `lower::state_machine`.
- **Reference explorer oracle.** `engine/tests/sm_reference.rs` explores the reference semantics of section 3 directly (`σ` as an `ElementId → StateId` map, `m` as counts), never calling the compiler. Proptest over random small state machines (≤ 6 states, ≤ 12 transitions, random directions) composed with random small Petri nets and random sync and async pairs that pass section 8, asserting that the reachability graph is isomorphic to `verify`'s graph on the compiled net under `h`: same |R| and |O|, same deadlocks, same overflow markings, same dead sets mapped through the origin map.
- **Mutation check.** The LC faults, run through the oracle with `check_lowering` disabled, must be caught by the oracle too. This shows that both guards can fail.
- **`sm_replay`.** R-01–R-12, plus a proptest that every counterexample and every `CapacityExceededFirst` run that `verify` produces on the oracle's random scopes is accepted.
- **Composition.** NS C17–C20 and C21 (call and reply on one relationship) in the `engine` lowering tests, on top of their `verify`/`recheck` golden tests and the NS C16 proptest.
- **Check pipeline.** V-01–V-25 through the Session harness (design §8: `MemDisk`, inline `Spawner`, markings-only budgets); K-01–K-07; a lowering panic and a `check_lowering` failure each yield Unknown `InternalError` and the Session keeps working (ADR 0012); a cache hit whose `sm_replay` fails evicts the entry and resubmits the key.
- **UI fixtures.** Rust-written JSON fixtures for V-07a (frames with `activeState`) and V-18a (`modelIncomplete` with typed state machine causes), consumed by Vitest (design §8).

---

## 13. Amendments (decided and in force)

Every amendment that revision 2 proposed is decided in `docs/specs/decisions.md` and lives in its target document; this spec proposes nothing.

| # | revision 2 proposal | now in | decision |
|---|---|---|---|
| A1 | Set-valued `AsyncLink.from`/`to` (NS 6.1, 6.4, 6.5, L7, L8, L11, 2.2, recheck re-checks, C17–C20, §15; ADR 0010; design §3, §6); canonical encoding unchanged; shared-place alternative rejected | ADR 0016; NS revision 3 6.1–6.6, 14.3; design §3 | D-06 |
| A2 | ADR 0002: input and output events are transition ports, states are place ports, internal events are not ports | ADR 0015; NS 6.2 | D-01 |
| A3 | ADR 0003 and NS L9: a state of a child is a place port named `element/state`, meaning "this state is active" | ADR 0015; NS L9 | D-01 |
| A4 | NS L4 counts a port as bound when a contributing relationship names it in a pair; L14–L16 in the L-table | NS revision 3 6.6 | D-43 |
| A5 | `Lowered.snapshot`, `check_lowering` on `lower-<rev>`, `sm_replay` as pipeline step 3b with the cache-hit rerun, `Origin::State`/`SmTransition`, `BehaviorModel::Unloaded`, design §7 sentence | design §3, §4, §7 (`ScopeSnapshot`); NS 9.2 step 3b | D-37 |
| A6 | `CODING_STANDARDS.md` VER-2: a failed replay is "internal error" | GSS edit of VER-2 | D-24, D-36 |
| A7 | Contracts R2, R15: `ProblemDto.kind = 'stateMachine'` with `code` and optional `state`, `event`, `transition`, `relationship`; `PortKind` gains `InputEvent`, `OutputEvent`, `State`; copy keys `V-DIS-DL-SM`, `R-STEP-SM-SELF`, the closed-port note of 3.2, the SM-E*/SM-B* messages and `I-REL-INCOMPAT` reasons | UX, written against sections 3.2, 7.2, 7.3, 8.3; test cases V-07a, V-18a | D-46 |

Revision 3 adds two clarifications that need no other document to change: L12 for a state machine runs on the fused IDs, which carry transition names (section 2.2, V-25), and SM-B01 is read per pair, not per relationship end (D-07).

---

## 14. Terms

The state machine terms (Event, Input event, Output event, Internal event, Initial state, Final state, Active state, State machine transition, and the sharpened Port, Binding and Final predicate) are applied to `GLOSSARY.md` by the glossary owner from the merged block of `consistency-review.md` with D-28 to D-33. This spec uses them exactly. Two readings matter here:

- **Final state**: a state in which stopping counts as proper termination; from slice 2 it contributes to the element's own-scope final predicate (section 3.6) and may be named in the parent's.
- **Internal event**: may be named `timeout`, but it is not the scan's Timeout attribute and not the Budget (D-33).

Spec-internal words, never shown in the UI: reference tuple, constituent, port set.

---

## 15. Closed questions

| # | question (revision 2) | decision |
|---|---|---|
| 1 | Accept A1, set-valued asynchronous bindings, before slice 0 code | D-06: accepted; ADR 0016, NS revision 3 |
| 2 | States as implicit place ports nameable in parent-scope properties (A3) | D-01: accepted; ADR 0015 |
| 3 | `overflow` state places at the cost of the cheap Proved | D-14: kept; the cost is the missing `NoOverflowPlace` certificate (4.1) |
| 4 | Unaccepted asynchronous messages wait instead of being discarded | D-15: confirmed (3.4) |
| 5 | Own-scope final states if final predicates come early | D-05, D-42: final predicates in slice 1; the own-scope default `Σ E/f = 1` applies from slice 2 for one or several final states (3.6) |
| 6 | SM-B05 for Petri net ports | D-16: NS L7 applies it to Petri net ports too |
| 7 | New UX copy | D-46: routed to the senior UX designer, bound to sections 3.2, 7.2, 7.3, 8.3 |
| 8 | UML clause citation for "discard an unaccepted event" | D-44: a research record item; D-15 stands on its own reasoning |
