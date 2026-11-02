# Net semantics

Specification `docs/specs/net-semantics.md`. Status: **Accepted** (applies `docs/specs/decisions.md`; final for implementation once every spec has applied that file), revision 3, 2026-11-02. Author: software architect. Revision 3 applies `docs/specs/decisions.md` (D-01 to D-59) to revision 2: set-valued asynchronous bindings, binding pairs with their own direction, the final predicate inside the deadlock-free property, `never p` on overflow markings, the `NoOverflowPlace` certificate, the closed list of seven Unknown reasons, children of a combined scope, L7 with at most one asynchronous binding per port, and the budget split. Every number below was recomputed with a throwaway reference implementation of sections 2–8, 10.1, 11 and 12 (Python, in a temp directory, not in the repo); the E5 case 1 figures (84 and 99 markings, the 6-step and 13-step deadlocks, P4 in 3 steps) were re-run on the four-relationship mapping of D-07 with transitions in canonical ID order.

This is the one written definition of the Petri net class the tool checks, of how open nets compose, of every built-in property, of the evidence a verdict needs, of budgets and Unknown reasons, and of the canonical encoding behind the CheckKey. `crates/verify` and `crates/recheck` implement it **independently, from this text, never from each other** (ADR 0009, ADR 0010). `crates/engine` implements the lowering and flattening half (section 6). `crates/net` implements only the data types, structural validation and the canonical encoding (section 11).

Where this text and an ADR disagree, the ADR wins. The amendments this revision relies on are in force: ADR 0015 (places a property may name; state machine ports), ADR 0016 (composition rules), ADR 0017 (overflow markings, certificates, Unknown reasons), ADR 0018 (final predicate). Section 16 lists them. Terms follow `GLOSSARY.md`; section 17 lists the additions. Section 18 records the closed questions.

Slice tags: **[S0]** is needed by slice 0 (walking skeleton). **[S1]**, **[S2]**, **[S4]** and **[later]** mark parts a later slice needs; they are specified now so that nothing in slice 0 changes shape when they arrive. Section 15 is the slice map.

---

## 1. Notation

- ℕ = {0, 1, 2, …}. ℕ⁺ = ℕ \ {0}. ℤ the integers. u32 = {0, …, 2³² − 1}.
- For a finite set X, a function v: X → ℕ is written as a vector; v(x) is its value at x.
- IDs are strings; the order used everywhere ("canonical order") is the bytewise order of their UTF-8 encoding.
- A **run** is a sequence of firings from a marking (section 4.3). The glossary word **counterexample** is reserved for the evidence object of section 9; a counterexample contains a run, but not every run is a counterexample.
- Plain English follows every formula. Where the two seem to differ, the formula is normative.

---

## 2. Net structure [S0]

### 2.1 Definition

A **net** is a tuple

N = (P, T, W⁻, W⁺, cap, wf, M₀)

where

- P is a finite set of **places**, T a finite set of **transitions**, P ∩ T = ∅.
- W⁻: P × T → ℕ is the **pre weight** (arc from place to transition) and W⁺: T × P → ℕ the **post weight** (arc from transition to place). A weight of 0 means "no arc". Every arc has a weight in ℕ⁺. Arc weights **are** part of the net class (ADR 0010 "at least the arc weight", ADR 0013 "arc weights" in the key). E5 needed a weight-2 read arc for load shedding (`control_plane/shed_a`).
- cap: P → ℕ⁺ is the **capacity** (glossary). Every place has one. cap ≥ 1.
- wf: P → {block, drop, overflow} is **when full** (ADR 0014). Every place of a behavior model states it (no default); the default `overflow` exists only for generated places (6.4). State places of a compiled state machine are `overflow` (SM §4.1, D-14).
- M₀ is the **initial marking** (section 3).

The **final predicate** F (section 3.3) is **not** part of the net: it is a parameter of the deadlock-free property (8.1, ADR 0018), so that editing it changes only the deadlock-free key of that behavior and, from slice 4, the key of a T7 "done" template property, whose expansion is F (ADR 0022, D-48; ADR 0013).

The pre set of t is •t = {p | W⁻(p, t) > 0}, the post set is t• = {p | W⁺(t, p) > 0}. A transition may have •t = ∅ (a source: it fires freely), t• = ∅ (a sink), or both (it fires freely and changes nothing). All three are allowed; the model-level `validate` may warn, lowering does not fail.

**Not in the net class:** inhibitor arcs, reset arcs, read arcs as a primitive (a read arc is written as equal pre and post weight on the same place), priorities, time, colors, place fusion. A later extension needs an ADR and an amendment of this spec; both explorers must then change together (ADR 0009).

### 2.2 Qualified IDs

Every place and transition carries a **qualified ID**, a string. **Places and transitions share one namespace**: within one element, and therefore within the net, no place and transition may have the same ID. A clash is the model problem `nameClash` (file-format §6.4, D-26): the file loads, the net editor rejects the gesture ("Billing has both a place and a transition named `ret_a`"), and every scope that contains the element fails L10. For a state machine, states, events and transitions share the element's namespace (SM-E04). Markings, properties, counterexamples, evidence and the encoding use these IDs, never positions (ADR 0010).

Three shapes, decided by the characters `/` and `+`:

| shape | pattern | meaning |
|---|---|---|
| own | `element/name` | a place, transition or state of one element's behavior model |
| fused | `relationship/caller+callee` | the fused transition of a synchronous binding pair; `caller` and `callee` are the **bare** transition names (not qualified), since the pair fixes both elements (L11) |
| generated | `relationship` | the generated place of an asynchronous binding |

**Slugs.** `element`, `relationship` and `name` are slugs: `[A-Za-z_][A-Za-z0-9_-]*`, never one of `and`, `or`, `not`. This spec depends on exactly four facts about slugs: no `/`, no `+`, no whitespace, and a first character that is not a digit; the reserved words and the first-character rule exist for the predicate lexer (5.3). The file format narrows the rest (file-format §4.1, D-12): element, relationship, view and property IDs are lowercase and letter-first; names inside a behavior model keep their case and contain no `__`; nothing ends in `-`. Every ID in this document follows the narrowed rule.

**No collisions.** An own ID contains exactly one `/` and no `+`; a fused ID contains exactly one `/` and exactly one `+`; a generated ID contains neither. So the three shapes never collide with each other. Within a shape: own IDs are unique by the one-namespace rule above; generated IDs are unique because relationship IDs are unique in the model and L8 allows one AsyncLink, and therefore one generated place, per relationship, whatever the size of its sets; fused IDs are unique because L12 forbids two pairs on one relationship that produce the same fused ID (same bare caller and callee transition names), whatever their direction, so `r/x+y` names exactly one pair; for a state machine the names are those of the transitions of the cross product (SM §2.2, D-50).

### 2.3 Structural validity (`net::validate`)

A net is **structurally valid** iff

1. all IDs are unique across places and transitions together and well formed (2.2);
2. every arc refers to an existing place and transition and has weight ≥ 1;
3. cap(p) ≥ 1 for every p;
4. M₀(p) ≤ cap(p) for every p (the initial marking is never an overflow marking);
5. every property, including the final predicate carried by a deadlock-free property, refers only to existing places;
6. arc weights, capacities and token counts fit in u32; predicate coefficients and constants fit in i32;
7. **cap(p) + max_t W⁺(t, p) ≤ u32::MAX for every p** (the largest tentative count, section 4.1, fits in u32, so no arithmetic in either explorer can overflow on a valid net; explorers still use checked arithmetic per `CODING_STANDARDS.md` LIM-4 and map a `None` to Unknown `InternalError`).

`engine` validates every part of an `OpenNets` and the flattened net before it hands anything to a worker (L10); `verify` and `recheck` validate again on entry (they treat their input as untrusted). A net that fails validation in lowering ends the property as Unknown `ModelIncomplete` (section 6.6); a net that fails validation inside a worker ends as Unknown `InternalError` (engine let an invalid net through).

### 2.4 Compact notation used in this document

- `x/p(2, block, 1)` = place `x/p` with cap 2, when-full block, initial 1. The initial count is omitted when 0.
- `x/t: x/p, 2·x/q → x/r` = transition `x/t` with W⁻(x/p) = 1, W⁻(x/q) = 2, W⁺(x/r) = 1. `∅` is the empty side.

---

## 3. Markings [S0]

### 3.1 Marking

A **marking** is a function M: P → ℕ. Its data form (`net::Marking`) is a map from qualified ID to count in which places with count 0 may be omitted; two markings are equal iff they agree on every place of P. A marking is **in bounds** iff M(p) ≤ cap(p) for all p.

### 3.2 Overflow marking

M is an **overflow marking** iff ∃p ∈ P: M(p) > cap(p).

Since M₀ is in bounds (2.3) and only firing into an `overflow` place can raise a count above its capacity (4.2), every overflow marking has at least one exceeded place p with wf(p) = overflow, and the last step of any run that reaches it starts from an in-bounds marking. An overflow marking is a **violation marking** and terminal: it has no successors (4.4), it is not a deadlock and it is not final (7). Its counts are defined by the firing rule and two properties read them: "no capacity exceeded" (8.2) and "never p" (8.3, D-02); no other property looks at them.

### 3.3 Initial and final markings

M₀ is model data (ADR 0003) and in bounds. The **final predicate** F is model data of the scope element (`[final] own | combined`, one per behavior; ADR 0003, ADR 0018) and is lowered into the deadlock-free property of that behavior (8.1). Given F, M is **final** iff F ≠ none and F(M) holds. **[S1]** Slice 0 always lowers F = none, so no marking is final; slice 1 brings the minimal form (D-05: the editor offers a conjunction of `place = n`, and for a state machine with several final states the single sum atom `Σ E/f = 1`; hand-written `[final]` text may be any predicate of section 5). A final marking may still have enabled transitions; being final does not stop exploration (it only decides whether a marking without enabled transitions is a deadlock or proper termination). Overflow markings are never final.

---

## 4. Enabling and firing [S0]

### 4.1 Enabling

For transition t and marking M define the **tentative count**

M⁺ₜ(p) = M(p) − W⁻(p, t) + W⁺(t, p)      (for every p; this is M(p) when t does not touch p)

t is **enabled in M**, written M [t⟩, iff

(E1) M is not an overflow marking, and
(E2) M(p) ≥ W⁻(p, t) for every p ∈ •t, and
(E3) M⁺ₜ(p) ≤ cap(p) for every p ∈ t• with wf(p) = block.

In English: the input places hold enough tokens, and no blocking output place would end up over its capacity. (E3) uses the count *after* removing the pre weight, so a self-loop on a full blocking place (`x/p(1, block, 1)`, `x/t: x/p → x/p`) is enabled: 1 − 1 + 1 = 1 ≤ 1. A blocking output with weight 3 into a place of capacity 2 is never enabled. (E1) is what makes overflow markings terminal. On a valid net (2.3 rule 7) every tentative count of an in-bounds marking fits in u32.

### 4.2 Firing

If M [t⟩, firing t yields M' = M [t⟩ M', defined per place:

- wf(p) ∈ {block, overflow}:   M'(p) = M⁺ₜ(p)
- wf(p) = drop:                 M'(p) = min(cap(p), M⁺ₜ(p))

So `block` only ever changes whether t is enabled; `drop` fires and discards the surplus above the capacity (subtract first, then add, then clamp); `overflow` fires and may produce a count above the capacity, in which case M' is an overflow marking. Nothing else differs between the three; in particular all three remove the pre weights normally.

Worked micro-examples (all are conformance vectors in section 14):

| net | from | t | result |
|---|---|---|---|
| `x/p(2, block)`, `x/t: ∅ → x/p` | p = 2 | x/t | not enabled (2 + 1 > 2): back-pressure |
| `x/p(2, drop)`, same t | p = 2 | x/t | enabled, p stays 2: load shedding |
| `x/p(2, overflow)`, same t | p = 2 | x/t | enabled, p = 3: overflow marking |
| `x/p(2, drop)`, `x/t: ∅ → 3·x/p` | p = 0 | x/t | p = min(2, 3) = 2 |
| `x/p(1, drop, 1)`, `x/t: x/p → 2·x/p` | p = 1 | x/t | p = min(1, 1 − 1 + 2) = 1 |

### 4.3 Runs

M₀ [t₁⟩ M₁ [t₂⟩ … [tₙ⟩ Mₙ is a **run** of length n. By (E1) no tᵢ fires from an overflow marking, so at most Mₙ can be an overflow marking.

### 4.4 Reachable markings

- **Reach(M)** = the set of markings reachable from M by a run (M included).
- **R** = Reach(M₀) \ {overflow markings} — the **reachable markings**. **O** = Reach(M₀) ∩ {overflow markings} — the **reachable overflow markings**. Every element of O is a successor of some element of R and has no successors itself.
- **Finiteness.** For every p and every reachable M: M(p) ≤ cap(p) + max_t W⁺(t, p). Hence |R ∪ O| ≤ ∏ₚ (cap(p) + 1 + maxW⁺(p)) < ∞. Every property in section 8 is therefore decidable by exhaustive exploration; the budget (section 10) is about cost, not decidability.
- **Marking count.** The number reported with a Proved by exhaustion is |R| ("markings"). |O| is reported separately in evidence. Both explorers must compute the same |R| and the same |O| on the same net (section 9.4).

---

## 5. Linear marking predicates [S1 for the final predicate and the T5 `pool-share` expansion; S4 for user properties] (evaluation code is shared with nothing)

### 5.1 Abstract syntax (what explorers receive)

```
Pred  ::= Atom | And(Pred, Pred, …) | Or(Pred, Pred, …) | Not(Pred)
Atom  ::= { terms: [(QualifiedId, i32)…] sorted by ID, no duplicates, no zero coefficient, may be empty; cmp; constant: i32 }
cmp   ::= lt | le | eq | ne | ge | gt
```

### 5.2 Semantics

For marking M: ⟦Atom⟧(M) = (Σᵢ aᵢ · M(pᵢ)) cmp c, evaluated in 128-bit signed integers (counts < 2³², coefficients < 2³¹, at most 4,096 terms per atom (property-language §3.2): no overflow possible). An atom with no terms compares 0 with c. And/Or/Not are the Boolean connectives. An empty `And` is true, an empty `Or` is false (the parser never produces them). A predicate is evaluated on in-bounds and on overflow markings alike; which markings a property evaluates it on is the property's business (section 8).

### 5.3 Concrete syntax (the user writes; `model::property` parses from slice 1)

Lexer, applied left to right with maximal munch:

```
WS     := [ \t]+                                  skipped
INT    := [0-9]+                                  decimal, no sign, no leading zeros required
ID     := [A-Za-z_][A-Za-z0-9_/-]*                maximal run; `and`, `or`, `not` are keywords, not IDs
OP     := '==' | '!=' | '<=' | '>=' | '=' | '<' | '>' | '*' | '+' | '-' | '(' | ')'
```

Because an ID is a maximal run and may contain `-`, **a `-` directly after an ID character belongs to the ID**: `p/in-flight - r >= 1` is the place `p/in-flight` minus `r`; `x/p-1 >= 0` names a place `x/p-1`, which is a parse error ("unknown place `x/p-1`") if no such place exists, never `x/p` minus 1; `rel-7 - 2 >= 0` is the generated place `rel-7` minus 2. Binary `-` therefore needs whitespace before it when it follows an ID. There is no unary minus; a negative coefficient or constant arises only from subtraction. An ID that starts with a digit cannot occur (2.2), so `3` is always an INT.

Grammar:

```
pred   := disj
disj   := conj ('or' conj)*
conj   := unary ('and' unary)*
unary  := 'not' unary | '(' pred ')' | linear cmp linear
linear := term (('+' | '-') term)*
term   := INT | INT '*' ID | ID
cmp    := '=' | '==' | '!=' | '<' | '<=' | '>' | '>='
```

Normalization: for `L cmp R` with L = Σ aᵢpᵢ + c and R = Σ bᵢpᵢ + d, the atom is Σ (aᵢ − bᵢ)·pᵢ cmp (d − c): every place moves to the left, every constant to the right, duplicate places are merged, zero coefficients dropped, terms sorted by ID, `=`/`==` mapped to `eq`. Coefficients and the constant are accumulated exactly (in i64, which cannot overflow within 4,096 bytes of text) and only the **final** values are checked: every final coefficient and the final constant must lie in the symmetric range [−(2³¹−1), 2³¹−1], otherwise the text is a parse error (PL E117), never wrapped or clamped. An intermediate sum outside the range is not an error (`2147483647*a/x + a/x - a/x >= 0` is valid), and −2³¹ is excluded so that every value can be negated and written back as a literal (D-62). An atom whose terms all cancel is kept (`a/x - a/x >= 0` is `0 ≥ 0`, true). Every ID must name a place of the net (own, generated, or a place port; L9 decides which in a combined scope); an unknown ID is a parse error.

**Tree shape (PL A1).** The parser produces one n-ary `And` per chain of two or more `and`, one n-ary `Or` per chain of two or more `or`, and one `Not` per `not`; parentheses nest; nothing is flattened, reordered, deduplicated or rewritten. Error codes and messages are those of `docs/specs/property-language.md` §10.

The parser does **not** rewrite `<` into `<=` or similar (D-10); two syntactically different but equivalent predicates give different CheckKeys, which is allowed (ADR 0013 requires that semantic changes change the key, not that equivalent texts share one).

**Slices (D-25).** Slice 0 never parses: predicate and `[final]` strings are stored and written back verbatim; a `[final]` table lowers as `final none` with the warning `finalNotChecked`, and a user property or template is Unknown `PropertyNotCheckable`. Slice 1 ships the lexer, parser, normalization and resolution in `model::property` and the evaluation of 5.2 in both explorers, used only by the final predicate. Slice 4 uses the same parser for user properties and templates.

Examples: `control_plane/in_a + control_plane/wait_a + control_plane/ret_a >= 3`; `not (p/items = 0 and r = 0)`; `2*w/free - c/running <= 4`.

Parser vectors (section 14, "Predicate vectors").

---

## 6. Ports, bindings, composition and lowering

### 6.1 Open nets [S0]

An **OpenNets** value is what `engine::lower` produces and what both `engine::flatten` and `recheck::compose` consume (ADR 0010):

```
OpenNets  = { parts: [Net…], fusions: [Fusion…], links: [AsyncLink…] }
Fusion    = { relationship: Id, caller: QualifiedId, callee: QualifiedId }        -- both transitions; the pair fixes the elements (L11)
AsyncLink = { relationship: Id, from: [QualifiedId…], to: [QualifiedId…], capacity: u32, when_full: WhenFull }
            -- from and to: non-empty, sorted by canonical ID, no duplicates, every member a transition (D-06)
```

Each part is the behavior model of one child element, already carrying own-shaped IDs (`element/name`) and its own M₀. The final predicate does not travel here: it is inside the deadlock-free `NetProperty` (8.1, ADR 0018), so `recheck::compose` needs nothing beyond the parts and the bindings. Parts carry no port flags: which transitions and places are ports is a Model fact; everything that depends on it is a **lowering precondition** (6.6, engine, shared by both explorers and golden-tested), not a composition rule. In prose this document says "synchronous binding" for a `Fusion` and "asynchronous binding" for an `AsyncLink`; these are the code names of the two kinds of binding (glossary **Binding**, D-29), and the field `links` keeps its name.

### 6.2 Ports [S0]

A **port** is a place or transition that a behavior model offers to the outside (glossary).

- A **transition port** is what relationships bind. In a combined scope it must be bound (L4). For a state machine, an input or output event is a transition port that stands for every transition on it; an internal event is not a port (SM §2.1, ADR 0015).
- A **place port** is never bound by a relationship in v1 (no place fusion). It is the only kind of place a property or final predicate of the *parent* scope may name (ADR 0015; L9). Every state of a state machine is an implicit place port (`element/state`, meaning "this state is active"). Place ports take part in nothing else; inside the net they are plain places.
- In an element's **own scope** its ports are **closed**: a transition port fires on its own as if its partner were always ready (a port with an empty pre set is a free source), and a place port is a plain place. Own-scope properties may name any place of the element. **Every own-scope verdict carries the closed-port note "every port of `<element>` can fire at any time"** (verdict text, section 9.6; the word "assumption" is reserved for relationships, D-30).

### 6.3 Synchronous binding: transition fusion [S0]

A Fusion (r, a, c) with caller transition a of part A and callee transition c of part C produces one **fused transition** with ID `r/name(a)+name(c)`:

W⁻(p, r/a+c) = W⁻(p, a) + W⁻(p, c)    W⁺(r/a+c, p) = W⁺(a, p) + W⁺(c, p)    for every p

(weights add; in practice the pre sets are disjoint because a and c belong to different elements, but the rule is total). A transition that appears in any Fusion, as caller or as callee, is **removed** from the composed net: a bound port transition never fires on its own (ADR 0010). One fused transition exists per Fusion, so a callee with k callers yields k fused transitions and a caller with k callees (over k relationships) yields k alternatives (nondeterministic choice of partner). A and C are the two elements of relationship r in either order (L11, D-07): a call pair and its reply pair sit on one relationship.

### 6.4 Asynchronous binding: generated place [S0]

An AsyncLink (r, S, D, cap, wf) adds a **generated place** with ID `r`, capacity cap, when-full wf, initial 0, and for every s ∈ S an arc W⁺(s, r) += 1 and for every d ∈ D an arc W⁻(r, d) += 1. Every sender in S and every receiver in D stays in the net as its own transition (it is still enabled by its own pre set and now also produces into or consumes from `r`). A Petri net transition port is the one-element case; a state machine event contributes every transition on that event (D-06). The generated place is a bag: no order, no identity (ADR 0002). Message weight is fixed at 1 per firing. Any member of S puts a message; any member of D takes one (a choice, not a join: L7 keeps every transition in at most one asynchronous binding, so no transition ever needs a message from two places).

**Defaults (ADR 0014).** `AsyncLink.capacity` is the relationship's capacity; a relationship that binds asynchronously without one fails lowering (L13). `AsyncLink.when_full` is the relationship's `when_full` and **defaults to `overflow`** when the relationship states none; lowering fills the default in, so an `OpenNets` value is always explicit and the generated place line in the encoding never depends on a default.

### 6.5 Composition (flattening) [S0]

flatten(O) for O = (parts, fusions, links), implemented once in `engine` and once in `recheck::compose`:

1. P = ⋃ P(part) ∪ {r | (r, S, D, …) ∈ links}; cap, wf, M₀ from the parts, and (cap, wf, 0) for generated places.
2. T₀ = ⋃ T(part), with the parts' arcs.
3. Apply every asynchronous binding (6.4) to the arcs of T₀, once per member of its `from` and `to` sets.
4. Build one fused transition per fusion (6.3) from the arcs after step 3 (so a sender that is also a caller would carry its generated-place arc into the fusion; L7 forbids that mix in v1, but the rule is stated so that both implementations agree if it is ever relaxed; vector C8 records the meaning).
5. T = (T₀ \ {a, c | (r, a, c) ∈ fusions}) ∪ {fused transitions}.
6. Validate (2.3).

The result is independent of the order of parts, fusions and links; the canonical encoding (section 11) is sorted, so `encode::canonical(engine::flatten(O)) == encode::canonical(recheck::compose(O))` must hold bytewise for every valid O. A difference ends the check as Unknown `InternalDisagreement` before any exploration (ADR 0010).

**Own scope.** For a property on an element's own behavior model, O has one part, no fusions and no links; flattening is the identity plus validation.

### 6.6 Lowering preconditions (engine; total or fail) [S0]

`lower(model, scope)` with `scope = Scope { element, behavior: Own | Combined }` (D-32) either returns `Lowered { open: OpenNets, flat: Net, origin: OriginMap, assumptions: [RelationshipId…], snapshot }` or fails with `ModelIncomplete(details)`; it never returns a smaller net (design §6). Each failing condition contributes one line to `details`; lines are ordered by rule number, then by canonical ID; **lines use qualified IDs, never display names** (the examples below show IDs).

**Children (D-13).** A **child** of a combined scope is an element whose `parent` is the scope element (file-format §6.1). In a valid project Persons and Software Systems have no parent and are never children; lowering reads `parent`, never `kind`, so a hand-written Person or Software System that carries a `parent` is a child (file-format §6.2, D-51). **Nothing to check:** if the scope has no children, or no child has a behavior model, or an own scope's element has no behavior model, lowering does not run and the property's key state is `nothingToCheck` (design §3 `KeyState::NothingToCheck`; no verdict, no Unknown). L1 lines are produced only when at least one child has a behavior model.

A relationship **contributes** iff both of its elements are children of the scope. A relationship that does not contribute, or that contributes but binds no ports, adds nothing to the net and is listed in `assumptions`, which the verdict panel shows next to every verdict of the scope ("not part of this check: rel-3 (api → external_dns)"); `assumptions` is not part of the CheckKey (a cached verdict shows the current assumptions). For a **combined scope**:

| # | condition | detail text (example) |
|---|---|---|
| L1 | every child element has a behavior model (that loaded) | `shipping has no behavior model` |
| L2 | every place of every child has a `when_full` | `billing/queue has no when_full` |
| L3 | every relationship endpoint and port reference resolves. A dangling reference in a property or in the final predicate is not an L3 line: it ends only that property (PL §4.3, E2xx) as `PropertyNotCheckable` (D-47) | `billing/take_lock not found` |
| L4 | every transition port of every child is bound by at least one contributing relationship, that is, named in one of its pairs (for an event, even if the pair's cross product is empty: that case is SM-E11). A port bound only by relationships to elements outside the scope is **not** closed (closing it would add behavior and could hide a deadlock); it fails L4 | `billing/reply is a port but no relationship inside this scope binds it` |
| L5 | every bound transition is a transition port (not an internal transition, not a place, not an internal event) | `billing/retry is bound but is not a port` |
| L6 | the two ports of a pair belong to two distinct children of the scope | `billing/ping is bound to billing/pong (same element)` |
| L7 | a transition port has exactly one role: caller in ≥ 1 fusions, or callee in ≥ 1 fusions, or member of the `from` or `to` set of **exactly one** asynchronous binding (D-16; SM-B05 for Petri net ports too). Never caller and callee; never fused and asynchronously bound; never in two asynchronous bindings | `control_plane/accept_a is both caller and callee`; `fe1/receive_info is in 2 asynchronous bindings: r2, r5` |
| L8 | a relationship carries at most one AsyncLink (its generated place is `relationship`); its `from` and `to` sets hold every transition of the two bound ports; it may carry any number of fusions | `rel-7 binds two asynchronous port pairs` |
| L9 | every place named by a property stored in the model, by a template parameter or by the final predicate of the scope is a place port of a child (states included) or a generated place of the scope. A template expansion computed at lowering from the model (T7 "its initial marking") may name every place (ADR 0015). **Property-level (D-47, ADR 0017):** a failure ends only the property that names the place, or, for the final predicate, the deadlock-free and T7 "done" properties of that behavior, as Unknown `PropertyNotCheckable` with the E208 line; it never fails the lowering of the net, so the other properties of the scope run | `billing/inflight is not a port` (PL E208 text) |
| L10 | every part and the flattened net pass 2.3 (in particular M₀ ≤ cap, rule 7 and one namespace per element) | `billing/pool: initial 5 exceeds capacity 3` |
| L11 | each pair has its own direction: its caller (sync) or `from` members (async) belong to one of the relationship's two elements and its callee or `to` members to the other, either way around (D-07). For a state machine, an output event is bound only as `from` and an input event only as `to` (SM-B01) | `rel-2 (api ↔ billing) binds api/call + external/serve` |
| L12 | no two pairs on one relationship produce the same fused ID `r/caller+callee`, whatever their direction: the bare names are the transition names of the fused ID, which for a Petri net port are the port's name and for a state machine event the names of the state machine transitions of the cross product (SM §2.2, SM V-25; D-50) | `rel-2 binds call + serve twice` |
| L13 | an asynchronous binding has a capacity ≥ 1 | `rel-4 is asynchronous but has no capacity` |
| L14 | state machine structure (SM-E01–SM-E09, SM-E11; SM §8.2) | see SM §8.3 |
| L15 | state machine binding rules (SM-B01, SM-B04, SM-B05; SM §8.2) | see SM §8.3 |
| L16 | the flattened net stays within `SCOPE_TRANSITIONS_MAX` (SM §8.2) | `scope sys would have 65792 transitions; the limit is 65536` |

For an **own scope**, L1, L2, L3, L10 and L14 apply and a property may name any place of the element.

L6–L8 and L11–L13 are v1 restrictions chosen because they make the fused-ID rule and the composition total and unambiguous (chained fusions in particular have no single obvious meaning: "A calls B calls C" as one three-way step or as two alternatives; asynchronous fan-in would be an AND-join, not what "receives from two senders" means). Relaxing any of them later is additive.

`recheck::compose` re-checks what it can see without the Model (IDs resolve; L6, L7, L8, L12 by inspection of the parts and the binding lists; every AsyncLink set is non-empty, sorted, without duplicates, and its members are transitions of the parts; 2.3 on every part and on the result) and returns `ComposeError`, which the worker maps to Unknown `InternalDisagreement` (engine accepted what recheck rejects). It cannot see a relationship's elements, so L11 is engine-only and golden-tested (ADR 0010: the cross-check covers composition, not lowering).

### 6.7 Origin map (engine only) [S0]

Lowering also returns, for every qualified ID, where it came from: own IDs map to (element, place, transition or state); fused IDs to (relationship, [(caller element, caller transition), (callee element, callee transition)]); generated IDs to the relationship. `ReplayFrame.fired` (design §3, `{element, transition}[]`) and the aria text "Billing fires take_lock" are rendered through this map; the explorers never see it. For state machines the map also carries `Origin::State` and `Origin::SmTransition` (SM §7.1).

**Highlight rule for the last frame of a deadlock** (slice 0 rule, D-34; UX may replace it before slice 0 UI code with V13 as the test): highlight every element that owns a transition t with waits(M, t) ≠ ∅ and at least one pre place of t holding ≥ 1 token (the element has started something it cannot finish), plus the elements that own the places those transitions wait for. For LOCKS(yx) this is `a`, `b` and `l`, which meets the slice 0 acceptance criterion "both elements highlighted". The rule only renders frames; it never touches evidence.

---

## 7. Deadlock, termination, overflow [S0]

For a marking M with M ∈ R (so M is not an overflow marking), and the final predicate F of the deadlock-free property under check:

- **enabled(M)** = {t ∈ T | M [t⟩}.
- M is a **deadlock** iff enabled(M) = ∅ and M is not final.
- M is **proper termination** iff enabled(M) = ∅ and M is final **[S1]**.
- An overflow marking is **neither** (ADR 0014): it is a violation marking, it has no successors and it is excluded from both definitions.

Note what counts as "enabled": a transition whose only obstacle is a full `block` output place is **not** enabled, so a producer waiting on a full blocking buffer with nothing else to do is a deadlock. That is the intended reading of back-pressure (E5 case 2, Cognito). A transition whose firing would overflow an `overflow` place **is** enabled, so such a marking is never a deadlock; its successor is an overflow marking. (E5's `explore.py` already reads it this way: its deadlock test skips markings that have an overflow successor.)

**Deadlock is global.** "Deadlock-free" means the scope never comes to a complete stop. One element that can always fire (a heartbeat, a self-loop) masks two elements that are stuck forever: LOCKS(yx) plus an independent `h/tick: h/s → h/s` is Proved deadlock-free (vector V43). The partial-stuck case is what "always reachable" (S4) catches (V44). Section 9.6 fixes the verdict text that says so.

**Waiting-for information.** For a deadlock M and every t ∈ T:

waits(M, t) = [(p, reason) for p ∈ •t ∪ t• in canonical order, where reason is `NeedsTokens { needed: W⁻(p, t), holds: M(p) }` if p ∈ •t and M(p) < W⁻(p, t), or `OutputFull { capacity: cap(p), would_hold: M⁺ₜ(p) }` if p ∈ t•, wf(p) = block and M⁺ₜ(p) > cap(p); places with neither reason are not listed]

This is the `Violation::Deadlock { waiting }` payload (one entry per transition of the net, in canonical order of transition ID, each with its full list) and is what the replay panel shows ("which transitions wait for which tokens", slice 0 acceptance). The reason field keeps entries such as V24's `c/fail ← c/idle (full)` from reading like a missing token.

**Consequence for slice 0 (F = none):** a workload that terminates (every token consumed, nothing left to do) **is a deadlock** until the final predicate arrives in slice 1 (D-05). Both E5 fixes come out Disproved in slice 0 (13.1, 13.2; vectors V49, V50); slice 0 shows the defects, and slice 1 shows that the fixes work. The verdict text says "no transition is enabled and no final predicate is defined" (9.6, D-23) and never advises a loop back to the initial marking.

---

## 8. Properties: definitions and decision procedures

All procedures run on the fully explored (R, O) unless they can stop early on a violation. "Discovery order" is the order of the normative exploration (section 12). Unknown `CapacityExceededFirst` is only ever produced after a **complete** exploration; if the budget runs out first, the verdict is Unknown `BudgetExhausted` (section 10). It names the **smallest-ID exceeded place of the first overflow marking in discovery order** and carries the run to that marking (10.2; D-22).

Common pattern (ADR 0014, ADR 0017): a local violation reached through in-bounds markings is Disproved; Proved needs the whole explored region overflow-free; "overflow reachable, no violation found" is Unknown `CapacityExceededFirst`, suggesting `block`, `drop` or a larger capacity.

Every property has a normative **"Proved means"** line; the verdict panel must show it (section 9.6).

### 8.1 Deadlock-free [S0; final predicate S1]

`NetProperty::DeadlockFree { final: Option<Pred> }` (ADR 0018). Holds iff no reachable marking is a deadlock: ∀M ∈ R: enabled(M) ≠ ∅ ∨ final(M), with final(M) per 3.3 from the property's own `final`.

**Proved means:** "the scope never comes to a complete stop; parts of it can still be stuck forever, which 'always reachable' catches."

| outcome | condition | evidence |
|---|---|---|
| Disproved | a deadlock M ∈ R is discovered | counterexample M₀ → M (the first deadlock in discovery order; by BFS it has minimal length), `Violation::Deadlock { waiting }` |
| Proved | exploration complete, no deadlock, O = ∅ | exhaustive, markings |R| |
| Unknown `CapacityExceededFirst` | exploration complete, no deadlock, O ≠ ∅ | — |

The Disproved stands even when O ≠ ∅ (vectors V17, V25): the deadlock was reached without passing through an overflow marking, since no run ever passes through one.

### 8.2 No capacity exceeded [S0]

Holds iff O = ∅.

**Proved means:** "no place of this scope can ever hold more than its capacity; tokens may still be discarded at drop places: `<list>`."

| outcome | condition | evidence |
|---|---|---|
| Proved (certificate) | no place has wf = overflow | `Technique::Certificate(NoOverflowPlace { drop_places })`, markings 0; `recheck::check_certificate` accepts it on its own composition (9.5) |
| Disproved | an overflow marking M is discovered | counterexample M₀ → M (ends in the overflow marking; minimal length by BFS), `Violation::Overflow { place, capacity, count }` where `place` is the **smallest-ID** exceeded place of M |
| Proved (exhaustive) | exploration complete, O = ∅ | exhaustive, markings |R| |

There is no Unknown of kind `CapacityExceededFirst` for this property. A net with no `overflow` place cannot violate it (3.2); `verify` emits the certificate without exploring, and the verdict names the drop places so "the queue never overflows" is never read as "no message is ever lost" (D-03). A scope that contains a state machine has `overflow` state places and therefore explores (D-14).

### 8.3 Never p [S1 for T5 `pool-share` only (D-60); S4 for user properties]

Holds iff ∀M ∈ R ∪ O: ¬p(M). p **is** evaluated on overflow markings (D-02, ADR 0017): their counts are defined by the firing rule, and a p-marking reached as an overflow marking is Disproved by its run, which ADR 0010's replay rule allows (a run may end in an overflow marking, never start a step from one).

**Proved means:** "no reachable marking satisfies p."

| outcome | condition | evidence |
|---|---|---|
| Disproved | some M ∈ R ∪ O with p(M) is discovered | counterexample M₀ → M (zero steps if p(M₀); V47), `Violation::Predicate { value }`; when M ∈ O the panel also says that the run exceeds a capacity (9.6) |
| Proved | exploration complete, no such M, O = ∅ | exhaustive |
| Unknown `CapacityExceededFirst` | exploration complete, no such M, O ≠ ∅ | — |

Because overflow markings are tested at discovery (section 12), the shortest counterexample may end in one (V34: 3 steps).

### 8.4 Always reachable p [S4]

Holds iff from every reachable marking some p-marking is reachable: ∀M ∈ R ∃M' ∈ Reach(M) ∩ R: p(M'). (AG EF p.)

**Proved means (normative wording, north-star non-goal, E5 implication 5):** "from every reachable marking the scope *can* get back to a marking where p holds, under some order of steps. It never means 'will recover'."

Decision procedure, after complete exploration:

1. G₀ = {M ∈ R | p(M)} ∪ {M ∈ R | some successor of M is in O}.
2. Good = all M ∈ R from which G₀ is reachable (backward closure of G₀ over the successor relation restricted to R).
3. Bad = R \ Good.

| outcome | condition | evidence |
|---|---|---|
| Disproved | Bad ≠ ∅ | `CounterexampleThenExhaustive { cx: M₀ → M for the first M ∈ Bad in discovery order with Violation::Unreachable, markings: |Reach(M)| }`; the exhaustive part: Reach(M) contains no p-marking **and no overflow marking** |
| Proved | Bad = ∅ and O = ∅ | exhaustive |
| Unknown `CapacityExceededFirst` | Bad = ∅ and O ≠ ∅ | — |

Including "reaches an overflow marking" in G₀ is what makes a Disproved sound under ADR 0014: a marking from which an overflow marking is reachable has undefined behavior beyond it, so the tool makes no claim about it. ADR 0017 states the confirmation as "neither a p-marking nor an overflow marking is reachable from its last marking" (D-09).

### 8.5 No dead transitions [S4]

t is **dead** iff ∀M ∈ R: ¬(M [t⟩). A transition whose firing produces an overflow marking is enabled in the source marking and therefore **not** dead. Holds iff no transition is dead.

**Proved means:** "every transition of the scope is enabled in at least one reachable marking."

| outcome | condition | evidence |
|---|---|---|
| Disproved | exploration complete, O = ∅, Dead ≠ ∅ | `Evidence::Exhaustive { dead: Dead in canonical order, markings: |R| }` |
| Proved | exploration complete, O = ∅, Dead = ∅ | exhaustive |
| Unknown `CapacityExceededFirst` | exploration complete, O ≠ ∅ | — |

A dead-set claim is global over R, so unlike a deadlock it cannot be "reached without passing through an overflow marking"; with O ≠ ∅ the verdict is Unknown even if Dead ≠ ∅ (vector V46; D-09, ADR 0017).

### 8.6 Summary of outcome rules

| property | local violation (Disproved despite O ≠ ∅) | needs O = ∅ for Proved | Disproved needs O = ∅ |
|---|---|---|---|
| deadlock-free | yes | yes | no |
| no capacity exceeded | yes (O ≠ ∅ is the violation) | n/a (certificate or O = ∅) | no |
| never p | yes (the violating marking may itself be in O) | yes | no |
| always reachable p | partially: Reach(M) ∩ O = ∅ for the counterexample M | yes | only locally |
| no dead transitions | no | yes | yes |

---

## 9. Verdicts and evidence

### 9.1 Verdict and evidence types (design §3) [S0]

```
NetProperty = DeadlockFree { final: Option<Pred> } | CapacityNeverExceeded | NoDeadTransitions | Never(Pred) | AlwaysReachable(Pred)
NetVerdict  = Proved { technique, markings } | Disproved(Evidence) | Unknown(UnknownReason)
Technique   = Exhaustive                                     -- markings = |R|; confirmed by agreement (9.4)
            | Certificate(Certificate)                       -- markings = 0; recheck checks the certificate (9.5)
Certificate = NoOverflowPlace { drop_places: [QualifiedId…] }   -- [S0] no capacity exceeded only
            | PInvariantBound([(y: Marking, b: u64)…])          -- [later]
Evidence    = Counterexample(cx)                              -- deadlock-free, no capacity exceeded, never p
            | Exhaustive { dead, markings }                   -- no dead transitions; markings = |R|
            | CounterexampleThenExhaustive { cx, markings }   -- always reachable p; markings = |Reach(last marking)|
Counterexample = { steps: [Step…], violation: Violation }
Step        = { fired: QualifiedId, marking: Marking }        -- one transition of the flattened net; marking after it
Violation   = Deadlock { waiting: [(QualifiedId, [(QualifiedId, WaitReason)…])…] }
            | Overflow { place, capacity: u32, count: u32 }
            | Predicate { value: i128 (informational) } | DeadTransitions([QualifiedId…]) | Unreachable
WaitReason  = NeedsTokens { needed: u32, holds: u32 } | OutputFull { capacity: u32, would_hold: u32 }
```

`Step.fired` is the qualified ID of exactly one transition of the flattened net (an own or a fused transition). The constituents of a fused step (the two elements, for highlighting) are derived by `engine` through the origin map; they are not part of the evidence.

A counterexample may have **zero steps** (M₀ itself is a deadlock or satisfies p; vectors V01, V10, V47). It cannot have zero steps for "no capacity exceeded" (M₀ is in bounds). An evidence kind that does not match the property kind (for example `Exhaustive` for deadlock-free), or a certificate kind that does not match it, is rejected by the worker as Unknown `InternalError`.

### 9.2 The check pipeline (worker, design §4) [S0]

For one property on one scope, with budget (N markings, T time) and cancel flag:

0. `own = recheck::compose(open)`; `ComposeError` or `canonical(own) ≠ canonical(flat)` → Unknown `InternalDisagreement`. Property-independent (the final predicate is in the property, not in the net); costs one encoding.
1. `verify::check(flat, p)` with at most ⌊N/2⌋ stored markings and until t₀ + T/2. verify drops its visited set before step 2.
2. Proved by exhaustion → `recheck::explore(own, p)` with the remaining budget (section 10) must **agree** (9.4): agreement → Proved; disagreement → Unknown `InternalDisagreement` (logged); budget out → Unknown `BudgetExhausted { phase: Recheck }`. Proved by certificate → `recheck::check_certificate(own, p, cert)` must accept (9.5); rejection → Unknown `InternalDisagreement` (logged).
3. Disproved with `Counterexample` → `recheck::replay(own, cx, p)` (9.3); failure → Unknown `InternalError` (logged; D-24). `Exhaustive` → `recheck::explore(own, p)` must agree on the dead set **and on `markings`**. `CounterexampleThenExhaustive` → replay to the last marking, then `recheck::explore` from it must find no p-marking and no overflow marking and the same `markings`; budget out → Unknown, never Disproved.
3b. [S2] For a scope that contains a state machine, `engine::sm_replay` (SM §7.4) then checks every counterexample (and the counterexample part of `CounterexampleThenExhaustive`) in state machine terms; failure → Unknown `InternalError`.
4. Unknown from verify → final; recheck does not run, except `CapacityExceededFirst`, whose run is replayed (10.2) and, [S2], passed through `sm_replay`.
5. The worker recomputes the CheckKey from `flat` and `p` (section 11) and returns it with the verdict.

A panic anywhere in the worker → Unknown `InternalError` (ADR 0012).

### 9.3 Counterexample replay (`recheck::replay`) [S0]

Given recheck's own net N, a counterexample cx and property p (the glossary's **Replay** is the canvas act; this function is the evidence check, D-33):

1. M := M₀.
2. For each step i = 1..n: (a) M must not be an overflow marking; (b) `fired` must be a transition of N; (c) M [fired⟩ must hold by section 4.1; (d) M := M [fired⟩ M' by 4.2; (e) M must equal `steps[i].marking` on every place.
3. The last marking M (M₀ if n = 0) must violate p:
   - deadlock-free: M in bounds, enabled(M) = ∅, ¬final(M) with `final` taken from p, and `waiting` equals waits(M, t) for every t, entry for entry (7).
   - no capacity exceeded: M is an overflow marking and `Overflow.place` is its smallest-ID exceeded place with the recorded capacity and count.
   - never p: p(M); M may be an overflow marking (D-02).
   - always reachable p: M is in bounds; then the exhaustive part (9.2 step 3).
4. Any failure is a `ReplayError` naming the step and the rule (a, b, c, e, or 3) that failed.

Replay stores no markings and cannot exhaust the budget; it honors only the cancel flag, checked once before it starts.

### 9.4 Agreement (`recheck::explore` vs `verify::check`) [S0]

Two results **agree** iff they have the same verdict kind and the same technique or evidence kind, and additionally:

- Proved by exhaustion: equal |R| and equal |O| **while `verify` explores without reductions** (all of slice 0 and until the spec is amended for stubborn sets; then the count shown is recheck's and the fitness test on counts moves to the differential corpus).
- Proved by certificate: `recheck::check_certificate` accepts the certificate on recheck's composition (9.5).
- Disproved `Exhaustive`: identical dead sets and equal `markings`.
- `CounterexampleThenExhaustive`: recheck's exploration from the replayed last marking finds no p-marking, no overflow marking, and equal `markings`.

Anything else is `InternalDisagreement`; in particular a recheck result of Unknown `CapacityExceededFirst`, or of Disproved, against a verify Proved (verify missed an overflow marking or a violation). The only recheck outcomes passed through as themselves are Unknown `BudgetExhausted { Recheck }` and `Cancelled`.

### 9.5 Certificates [S0 for NoOverflowPlace; later for the rest]

A **certificate** is a small object that an independent checker can verify in time polynomial in the net size without exploring. Only the kinds listed here are valid evidence; a new kind needs an amendment of this section before `verify` may emit it. `Proved { technique: Certificate(cert), markings: 0 }` carries the certificate so that `recheck::check_certificate(own, p, cert) -> bool` can run on recheck's own composition. This is the first of the two Proved paths of VER-1 and ADR 0008 (certificate accepted by the independent checker; agreement of the second explorer is the other); there is no third constructor (D-03).

**NoOverflowPlace { drop_places } [S0].** Valid for `CapacityNeverExceeded` only. `check_certificate` accepts it iff no place of recheck's net has wf = overflow and `drop_places` equals the sorted list of the net's places with wf = drop. Then O = ∅ by 3.2, so the property holds. The verdict text names the drop places (9.6).

**P-invariant bound [later].** C is the incidence matrix, C(p, t) = W⁺(t, p) − W⁻(p, t). A certificate is a list of pairs (y, b) with y: P → ℕ such that

(I1) yᵀC = 0 (for every t: Σₚ y(p)·C(p, t) = 0), and
(I2) y(p) = 0 for every p with wf(p) = drop (clamping breaks the invariant on drop places only), and
(I3) b = yᵀM₀.

Then yᵀM = b for every M ∈ R ∪ O (block only disables; overflow fires by the incidence vector), and for every p with y(p) > 0: M(p) ≤ ⌊b / y(p)⌋.

What a certificate proves (sufficient conditions; "Proved if", not "iff"):

- **No capacity exceeded**: Proved if for every p with wf(p) = overflow some (y, b) in the list has y(p) > 0 and ⌊b / y(p)⌋ ≤ cap(p). (This is what restores a cheap Proved for scopes with state machines: Lemma 1 of SM §6 is such an invariant.)
- **Never p** for exactly these atom shapes, and no other: a single atom with all coefficients aᵢ ≥ 1 and cmp ∈ {ge, gt}; `gt c` is checked as `ge c + 1`. Proved if some (y, b) has y(pᵢ) ≥ aᵢ for every term and b < c (for ge), **and** the no-capacity-exceeded condition above holds for the same list (ADR 0014: Proved requires that no overflow marking is reachable). Compound predicates, `le`/`lt`/`eq`/`ne` atoms and negative coefficients are not certificate-provable in v1; `verify` must not emit a certificate for them.
- No certificate kind exists for deadlock-free, always reachable or no dead transitions in v1; those are Proved by exhaustion only.

`recheck::check_certificate` checks (I1)–(I3) and the use conditions with exact integer arithmetic on its own composition. ILP/Farkas and inductive-invariant certificates are reserved names, not yet specified.

### 9.6 Verdict text the UI spec must carry [S0]

- Proved: the property's "Proved means" line (8.1–8.5), the technique and markings, and for an own scope the closed-port note "every port of `<element>` can fire at any time".
- Proved by `NoOverflowPlace` (8.2): "no place in `<scope>` can overflow: every place blocks or drops. Tokens may be discarded at: `<drop_places>`."
- Disproved deadlock-free with F = none: "no transition is enabled and no final predicate is defined" when the deadlock marking has no token in any pre set of any transition (a terminated workload), otherwise "no transition is enabled". The panel never advises a loop back to the initial marking (a reset transition hides real deadlocks); its next step is "define done when" (slice 1; in slice 0: "done when comes with slice 1") (D-23).
- Disproved never p whose last marking is an overflow marking: "… and this run also exceeds the capacity of `<place>` (`<count>` of `<capacity>`)" (D-02).
- Always reachable: "can always get back to `<p>`", never "recovers".
- Unknown `CapacityExceededFirst`: "`<place>` can exceed its capacity (`<count>` of `<capacity>`) before any `<property>` violation; set it to block or drop, or raise its capacity", with its run replayable.
- Unknown `PropertyNotCheckable`: headline "Property cannot be checked", with the detail lines (PL §10).
- Every verdict of a combined scope lists `assumptions` (6.6).

---

## 10. Budget semantics and Unknown reasons [S0]

### 10.1 Budget

`Budget { max_markings: N, deadline: Option<Instant>, cancel: AtomicBool }`. The engine derives the deadline from its Clock as t₀ + T; tests use markings only, or an injected fake Clock for deadline tests.

**Marking accounting (normative; both explorers; Appendix A follows it):**

(a) **stored** = |R ∪ O| discovered so far; both kinds are stored for de-duplication. M₀ counts as 1.
(b) Before **every** insertion into R or into O the explorer checks `stored < max`; if `stored = max`, the new marking is **neither stored nor tested** and the phase ends as Unknown `BudgetExhausted { markings: stored }`.
(c) Hence a phase with maximum m completes an exploration iff m ≥ |R ∪ O|, and fails iff m ≤ |R ∪ O| − 1 (vectors B5–B8 pin both sides).
(d) A violation test (8.1–8.3) runs on a marking right after it is stored, so a Disproved discovered as the k-th stored marking needs m ≥ k (vectors B1–B4).

**Time and cancel.** The deadline and the cancel flag are polled at least once every 1,024 expansions (an expansion is popping one marking from the queue). A phase that finds the deadline passed ends as `BudgetExhausted`; one that finds the flag set ends as `Cancelled` (if both hold at one poll, `Cancelled`). A violation found between two polls is reported as Disproved even if the deadline has passed meanwhile; there is no poll "before reporting a result". Replay and certificate checks store nothing, are bounded by the size of the evidence, and ignore the deadline; they check the cancel flag once before they start.

**Split (ADR 0010; D-17):** main phase ⌊N/2⌋ markings and deadline t₀ + T/2; recheck phase N − ⌊N/2⌋ markings and deadline t₀ + T (the time the main phase did not use carries over; the marking half does not). **While `verify` uses no reductions, the recheck phase can never run out of markings**: it explores the same R ∪ O and N − ⌊N/2⌋ ≥ ⌊N/2⌋ ≥ |R ∪ O|. `BudgetExhausted { phase: Recheck }` can therefore only come from the deadline (vector B9) or, once `verify` has reductions, from recheck's larger R.

**Memory estimate:** bytes ≥ stored markings × |P| × 4 is a **lower bound** (parent pointers, queue and hash-set overhead come on top; E1 calibrates the factor); the registry keeps the sum of running N × |P| × 4 × factor under its RAM share (design §5).

### 10.2 The complete enumeration of Unknown reasons

```
UnknownReason =
  | ModelIncomplete(details: [String…])                                           -- lowering failed (6.6); no worker ran
  | PropertyNotCheckable(details: [String…])                                      -- the property has an error, a template precondition fails, or the property kind is not checked yet in this slice (D-25); no worker ran. Headline "Property cannot be checked"
  | InternalDisagreement                                                          -- encodings differ, compose failed, a certificate was rejected, or results do not agree (9.4)
  | InternalError                                                                 -- a worker panicked, a replay failed (recheck::replay or sm_replay), arithmetic overflowed, or an explorer rejected its input
  | Cancelled                                                                     -- the cancel flag was set (polled every ≤ 1,024 expansions). Produced in S0 by the key-change cancel of design §4; user-visible from S1
  | BudgetExhausted { phase: Main | Recheck, markings: u64, elapsed: Duration }   -- "budget exhausted" / "budget exhausted while re-checking"
  | CapacityExceededFirst { place: QualifiedId, steps: [Step…] }                  -- complete exploration, no violation, O ≠ ∅; `place` = smallest-ID exceeded place of the first overflow marking in discovery order; `steps` = the run to it, replayed by recheck as a "no capacity exceeded" counterexample (a failed replay → InternalError)
```

There are no other reasons (D-08, ADR 0017); in particular "no counterexample up to depth N" and "unsupported construct" are not reasons this tool produces. **The reason is the event that ended the pipeline.** `ModelIncomplete` and `PropertyNotCheckable` end it before any worker runs (`ModelIncomplete` first when both apply: a scope whose net cannot be lowered gives every property `ModelIncomplete`, whatever that property's own errors; a property or final-predicate error alone never fails the lowering, D-47). `InternalDisagreement` and `InternalError` end it the moment they are detected, whatever the clock says afterwards, so a detected bug is never reported as a budget outcome. `Cancelled` and `BudgetExhausted` end a phase at a poll (`Cancelled` wins at the same poll). `CapacityExceededFirst` needs a complete exploration. The list above is also the display order when a log aggregates reasons. Each reason maps to one `UnknownDto.category`; `tried` names the technique and phase, `reached` carries markings and elapsed time, `details` the lines, and `suggestion` is filled for `CapacityExceededFirst` (9.6), `ModelIncomplete` and `PropertyNotCheckable` (the first detail's fix). An Unknown is never cached (ADR 0013) and never rendered as success (design §3). An initial marking above capacity is `ModelIncomplete` (D-08).

**Not an Unknown:** a scope with nothing to check (6.6) has the key state `nothingToCheck` and no verdict at all; the panel shows "no element in this view has a behavior model" (flows R9).

---

## 11. Canonical encoding and CheckKey [S0]

`net::encode::canonical(net) -> Vec<u8>` is the only behavior in `net` (ADR 0009). It is UTF-8 text; **every line, including the last, ends in LF**; no indentation, no trailing spaces; produced as follows (every list sorted by canonical ID order):

```
petrinet-net 1
place <id> <capacity> <block|drop|overflow> <initial>        -- one line per place, sorted by id
transition <id>                                               -- one block per transition, sorted by id; a transition with no arcs is just this line
pre <place-id> <weight>                                       -- all pre lines of the transition, sorted by place id, weight ≥ 1, omitted when no arc
post <place-id> <weight>                                      -- then all post lines, sorted by place id
```

There is no `final` line in the net (ADR 0018). `canonical_property(p)` is one more LF-terminated line:

```
property deadlock-free final none
property deadlock-free final <pred>
property capacity-never-exceeded
property no-dead-transitions
property never <pred>
property always-reachable <pred>
```

`<pred>` is the S-expression `(atom <cmp> <constant> <id> <coef> <id> <coef> …)` with terms sorted by ID (an atom with no terms is `(atom <cmp> <constant>)`), `(and e…)`, `(or e…)`, `(not e)`; `<cmp>` ∈ lt le eq ne ge gt; integers in decimal without leading zeros, with a leading `-` if negative.

**CheckKey** = BLAKE3-256 of `canonical(flat) ++ canonical_property(p)`, rendered as 64 lowercase hex characters; `key8` is its first 8 characters (thread names). The key covers IDs, capacities, when-full, arcs with weights, initial marking and the property, including the final predicate of a deadlock-free property; never layout, display names, element kinds, the scope element's ID, `assumptions` or the budget (ADR 0013). Editing the final predicate therefore changes only the deadlock-free key of that behavior and, from slice 4, the key of every T7 "done" template property of that behavior (its expansion is F; ADR 0022, D-48), never the keys of "no capacity exceeded" or "never p" (D-04). The header line gives domain separation; bumping `petrinet-net 1` invalidates every cached verdict, which is the intended effect of a semantics change. The header stays `1` in revision 3 although the `final` line moved into the property line: no cached verdict exists before slice 0 code (D-39).

Fitness (ADR 0013): every semantic mutation of a net or property changes the key; insertion order, undo-redo, rebuild from files, renames of display names and layout changes do not.

---

## 12. Normative exploration order [S0 for both; recheck forever]

`recheck::explore`, and `verify::check` as long as it uses no reductions, explore as follows:

1. FIFO queue initialized with M₀; R = {M₀}; O = ∅; parent(M₀) = none; stored = 1; test(M₀).
2. Pop M (one expansion). For each t ∈ T in canonical ID order with M [t⟩: compute M'. If M' is an overflow marking: if M' ∉ O, check the budget (10.1 b), add to O with parent (M, t), test; do not enqueue. Else if M' ∉ R: check the budget, add to R with parent (M, t), test, enqueue.
3. Violation tests for 8.1–8.3 run **at discovery** of a marking (M₀ included), so the first violating marking has minimal depth and the counterexample is the parent chain. For "never p" the test also runs on overflow markings (D-02); for deadlock-free and "no capacity exceeded" the test on an overflow marking is what 8.1 and 8.2 say (never a deadlock; the violation of 8.2).
4. Repeat until the queue is empty or the budget is exhausted.

This fixes the counterexample of every slice-0 and slice-4 vector to a definite length, makes "the k-th stored marking" well defined, and lets the "first overflow marking" be the same in both explorers. When `verify` later gains stubborn sets, its counterexamples need only be valid (9.3), and recheck's BFS supplies the shortest one for display (ADR 0010).

---

## 13. Worked examples: the three E5 cases

All E5 nets (`docs/product/experiments/e5/`) were re-run under this spec with the reference implementation; the counts and lengths below reproduce `e5/results.txt` wherever E5 made a choice (E5 had only `overflow` places and no `block`/`drop`). Mapping from the E5 files to this spec: every place is `overflow`; a call and its reply sit on one relationship (D-07), so case 1 has four relationships `r1`–`r4` instead of E5's eight bindings; fused transitions use the `r/caller+callee` shape; the places named by properties and final predicates (`api_b/done`, `control_plane/in_a`, `control_plane/wait_a`, `control_plane/ret_a`, `fe1/map_entries`, `users/signed_in`, `storage/serving`) are declared as place ports (L9); and, because case 1 has both a place and a transition named `control_plane/ret_a` (and `ret_b`), which 2.2 forbids, the **transitions are renamed `return_a` and `return_b`** (the places keep their names, so P4 is unchanged). Every result carries its slice; where the slice-0 verdict (F = none) differs from the slice-1 verdict, both are given. The final predicates of the E5 files are minimal-form conjunctions, so they are slice 1 results (D-05).

### 13.1 Case 1: EBS thread starvation (combined scope of 5 elements)

Elements and transition ports: `api_a` {call, reply}, `api_b` {call, reply}, `control_plane` {accept_a, accept_b, fwd_a, fwd_b, done_a, done_b, return_a, return_b}, `cluster_a` {create, respond}, `cluster_b` {create, respond}. Four synchronous relationships with two pairs each: `r1` api_a → control_plane (call + accept_a; return_a + reply), `r2` api_b → control_plane (call + accept_b; return_b + reply), `r3` control_plane → cluster_a (fwd_a + create; respond + done_a), `r4` control_plane → cluster_b (fwd_b + create; respond + done_b). The reply pairs run from the target to the source (L11). After flattening: 8 fused transitions, 0 own transitions remain, **17 places** (3 + 3 + 7 + 2 + 2), |R| = 84, |O| = 0. Example fused transition:

`r1/call+accept_a: api_a/pending, control_plane/threads_free → api_a/waiting, control_plane/in_a`

- **P1 deadlock-free**, F = `api_b/done = 2` [S1] and F = none [S0], same verdict: **Disproved, 6 steps**: r1/call+accept_a ×3, r3/fwd_a+create ×3 → marking `api_a/waiting = 3, api_b/pending = 2, control_plane/wait_a = 3, cluster_a/working = 3, cluster_b/healthy = 1`. Full waiting payload: `r1/call+accept_a ← api_a/pending (needs 1, holds 0), control_plane/threads_free (needs 1, holds 0)`; `r1/return_a+reply ← control_plane/ret_a`; `r2/call+accept_b ← control_plane/threads_free` (the AZ-B callers wait for the pool); `r2/return_b+reply ← api_b/waiting, control_plane/ret_b`; `r3/fwd_a+create ← control_plane/in_a`; `r3/respond+done_a ← cluster_a/healthy` (the degraded cluster never responds); `r4/fwd_b+create ← control_plane/in_b`; `r4/respond+done_b ← cluster_b/working, control_plane/wait_b`.
- **P2 no capacity exceeded** [S0]: **Proved**, exhaustive, 84 markings.
- **P4 never `control_plane/in_a + control_plane/wait_a + control_plane/ret_a >= 3`** [S4]: **Disproved, 3 steps** (three AZ-A calls accepted).
- Fixed variant (`case1_ebs_fixed`, the same four relationships with a third pair `shed_a + rejected` on `r1`, **20 places**, weight-2 read arc `r1/shed_a+rejected: 2·control_plane/a_used, api_a/pending → 2·control_plane/a_used, api_a/failed`): [S1] P1 (F = `api_b/done = 2`) **Proved**, 99 markings; [S4] P3 always-reachable `api_b/done = 2`, P4 and [S0] P2 all **Proved**, 99 markings. [S0] **P1 with F = none is Disproved, 13 steps** (vector V49): r1/call+accept_a ×2, r1/shed_a+rejected, r2/call+accept_b, r3/fwd_a+create ×2, r4/fwd_b+create, r4/respond+done_b, r2/return_b+reply, r2/call+accept_b, r4/fwd_b+create, r4/respond+done_b, r2/return_b+reply → the terminal marking `api_a/failed = 1, api_a/waiting = 2, api_b/done = 2, cluster_a/working = 2, cluster_b/healthy = 1, control_plane/a_used = 2, control_plane/threads_free = 1, control_plane/wait_a = 2`, which is proper termination by design (AZ-B finished, AZ-A is stuck on purpose) but a deadlock without F. **So slice 0 shows the EBS defect and slice 1 shows that its fix works** (J3; D-05).

### 13.2 Case 2: Kinesis thread limit and Cognito blocking buffer (two scopes)

E5 checks two scopes inside one file. In the tool a combined scope is *all* children of one parent element, so these are **two parent elements with one view each (slice 3)**; in slice 0 (one view) the six elements would form one combined scope and both halves would be checked as one union net.

Scope (a) `membership`, `fe_peers`, `fe1`; two asynchronous relationships: `r1` (membership/publish → fe1/open_peer_thread, cap 8) and `r2` (fe_peers/send_info → fe1/receive_info, cap 8), both generated places `overflow`. Own transitions stay: `membership/publish: membership/members → membership/listed, r1`; `fe1/open_peer_thread: r1 → fe1/peer_threads`; etc. 8 places, |R| = 330, |O| = 21.

- **P1 no capacity exceeded** [S0]: **Disproved, 10 steps** (publish, open_peer_thread, alternating ×5) ending in `fe1/peer_threads = 5 > 4`; `Violation::Overflow { place: fe1/peer_threads, capacity: 4, count: 5 }`.
- **P2 always reachable `fe1/map_entries = 5`** [S4]: **Unknown `CapacityExceededFirst { fe1/peer_threads, … }`**: Bad = ∅ (every marking either reaches a p-marking or reaches an overflow marking) and O ≠ ∅.
- `fe1` alone, own scope (vector V48): its closed port `open_peer_thread` is a free source, so **no capacity exceeded is Disproved in 5 steps** (`fe1/peer_threads` 5 of 4); this is the closed-port note at work.

Scope (b) `users`, `cognito_web`, `kinesis_api`; synchronous `r3` (users/login + cognito_web/auth) and `r4` (cognito_web/flush + kinesis_api/put); `kinesis_api/fail` is an internal transition and stays as its own. 6 places, |R| = 24.

- **P3 deadlock-free**, F = `users/signed_in = 4` [S1] and F = none [S0], same verdict: **Disproved, 3 steps**: kinesis_api/fail, r3/login+auth ×2 → `users/want = 2, users/signed_in = 2, cognito_web/buf_used = 2, kinesis_api/down = 1`; waiting: `kinesis_api/fail ← kinesis_api/up`, `r3/login+auth ← cognito_web/buf_free`, `r4/flush+put ← kinesis_api/up`. This is the Cognito "blocks on backlogged buffers" root cause. With ADR 0014 the same design is written as `cognito_web/buf(2, block)` and one place fewer.
- Fixed variant: [S1] P3 **Proved** (24 markings); [S0] P1 and [S4] P2 **Proved** (76 / 76 markings); [S0] **P3 with F = none is Disproved, 9 steps** at proper termination `users/signed_in = 4, cognito_web/buf_free = 2, kinesis_api/down = 1` (vector V50).

### 13.3 Case 3: DynamoDB retry amplification (combined scope of 3 elements)

`r1` synchronous (network/blip + storage/lose); `r2` asynchronous (storage/request → metadata/accept, cap 3); `r3` asynchronous (metadata/respond → storage/inbox, cap 3). Request and reply are two asynchronous relationships because L8 allows one generated place per relationship; the user states `interaction = "synchronous"` on `r2` for the scan (SR §2.5). 9 places, |R| = 1,919, |O| = 966.

- **P1 no capacity exceeded** [S0]: **Disproved, 8 steps**: r1/blip+lose ×3, storage/request ×3, storage/timeout, storage/request → `r2 = 4 > 3`; `Violation::Overflow { place: r2, capacity: 3, count: 4 }`. The replay frame shows the generated place of relationship r2 with "4 of 3" (slice 0 acceptance criterion 2 wording).
- **P2 always reachable `storage/serving = 3`** [S4]: **Unknown `CapacityExceededFirst { r2, … }`**.
- With a retry budget (`case3_dynamodb_fixed`, 10 places): P1 [S0] and P2 [S4] **Proved**, 1,970 markings (P2 "can always get back to serving = 3", not "recovers"). With the post-mortem's capacity-only fix (`case3_dynamodb_pmfix`): P1 **Disproved, 14 steps** (`r2` 7 of 6), |R| = 17,149, |O| = 5,528, which is the "unbudgeted retries overrun any capacity" verdict E5 predicted.

---

## 14. Conformance test vectors

Both explorers must produce these outcomes (golden tests in `verify` and `recheck`, and the E1 corpus). "len" is the counterexample length under the normative order (section 12). **For Disproved-by-counterexample rows the `#R` and `#O` columns are oracle facts from a full exploration, for information; the explorers stop at discovery and the evidence carries no count. The `stored` column is the number of stored markings at the moment of discovery (the violating marking included), which a test may assert and which the budget vectors use.** For Proved and Unknown rows `#R` and `#O` are what the explorers must report. All results were computed with the reference implementation. Nets use the notation of 2.4; qualified IDs are given as they appear in the flattened net. "Proved (certificate)" means `Technique::Certificate(NoOverflowPlace { drop_places })`, markings 0, accepted by `recheck::check_certificate`.

Shared nets:

- **SRC(k)**: `x/p(2, k)`, `x/t: ∅ → x/p` for k ∈ {block, drop, overflow}.
- **LOCKS(order)**: `a/idle(1, block, 1)`, `a/has_1(1, block)`, `a/has_2(1, block)`, same for `b`, `l/x(1, block, 1)`, `l/y(1, block, 1)`. Fused: `ra1/take_x+acq_x: a/idle, l/x → a/has_1`; `ra2/take_y+acq_y: a/has_1, l/y → a/has_2`; `ra3/release+rel_xy: a/has_2 → a/idle, l/x, l/y`. For `b`, order **yx**: `rb1/take_y+acq_y: b/idle, l/y → b/has_1`, `rb2/take_x+acq_x: b/has_1, l/x → b/has_2`, `rb3` as for a. Order **xy**: b mirrors a. (Two callers per callee; the callee `l/acq_x` never fires alone.)
- **PC(k)**: `p/items(5, block, 5)`, `c/got(5, block)`, generated `r(3, k)`; `p/produce: p/items → r`; `c/consume: r → c/got`.
- **POOL**: `c/idle(3, block, 3)`, `c/running(3, block)`, `w/free(2, block, 2)`; `r1/start+acquire: c/idle, w/free → c/running`; `r2/finish+release: c/running → c/idle, w/free`. **POOL-LEAK** adds internal `c/fail: c/running → c/idle`.
- **BRANCH**: `x/c(1, block, 1)`, `x/d(1, block)`, `x/p(1, overflow)`; `x/t1: x/c → x/d`; `x/t2: x/c → x/p`; `x/t3: x/p → 2·x/p`. **BRANCH-4** adds `x/t4: x/d, x/p → x/c` (never enabled: `x/d` and `x/p` are never both 1).
- **MASK**: LOCKS(yx) plus `h/s(1, block, 1)`, `h/tick: h/s → h/s`.
- **FE1**: `fe1/peer_threads(4, overflow)`, `fe1/map_entries(8, overflow)`; `fe1/open_peer_thread: ∅ → fe1/peer_threads`; `fe1/receive_info: fe1/peer_threads → fe1/peer_threads, fe1/map_entries` (E5 `fe1` in its own scope, ports closed).
- **MAX** = 4294967295 (u32::MAX).

### 14.1 Property vectors

| # | net | property | expected verdict | len / detail | stored | #R | #O | slice |
|---|---|---|---|---|---|---|---|---|
| V01 | `x/p(1, block, 1)`, no transitions | deadlock-free | Disproved | 0 (M₀ is a deadlock); waiting = [] | 1 | 1 | 0 | S0 |
| V02 | same | no capacity exceeded | Proved (certificate) | drop_places = []; markings 0 | — | 1 | 0 | S0 |
| V03 | SRC(block) | deadlock-free | Disproved | 2; waiting `x/t ← x/p OutputFull { 2, 3 }` | 3 | 3 | 0 | S0 |
| V04 | SRC(drop) | deadlock-free | Proved | — | — | 3 | 0 | S0 |
| V05 | SRC(drop) | no capacity exceeded | Proved (certificate) | drop_places = [`x/p`] | — | 3 | 0 | S0 |
| V06 | SRC(overflow) | no capacity exceeded | Disproved | 3; `Overflow { x/p, 2, 3 }` | 4 | 3 | 1 | S0 |
| V07 | SRC(overflow) | deadlock-free | Unknown CapacityExceededFirst { x/p, 3 steps } | — | — | 3 | 1 | S0 |
| V08 | `x/p(1, block, 1)`, `x/t: x/p → x/p` | deadlock-free | Proved | — (self-loop at capacity is enabled) | — | 1 | 0 | S0 |
| V09 | `x/p(3, block, 3)`, `x/q(1, block)`, `x/t: 2·x/p → x/q` | deadlock-free | Disproved | 1; waiting `x/t ← x/p NeedsTokens { 2, 1 }, x/q OutputFull { 1, 2 }` | 2 | 2 | 0 | S0 |
| V10 | `x/p(2, block)`, `x/t: ∅ → 3·x/p` | deadlock-free | Disproved | 0 (t never enabled); waiting `x/t ← x/p OutputFull { 2, 3 }` | 1 | 1 | 0 | S0 |
| V11 | `x/p(2, drop)`, `x/t: ∅ → 3·x/p` | deadlock-free | Proved | — (p = min(2, 3)) | — | 2 | 0 | S0 |
| V12 | `x/p(1, drop, 1)`, `x/t: x/p → 2·x/p` | deadlock-free | Proved | — | — | 1 | 0 | S0 |
| V13 | LOCKS(yx) | deadlock-free | Disproved | 2 (ra1, rb1); full waiting: `ra1 ← a/has_1 OutputFull{1,2}, a/idle Needs{1,0}, l/x Needs{1,0}`; `ra2 ← l/y Needs{1,0}`; `ra3 ← a/has_2 Needs{1,0}`; `rb1 ← b/has_1 OutputFull{1,2}, b/idle Needs{1,0}, l/y Needs{1,0}`; `rb2 ← l/x Needs{1,0}`; `rb3 ← b/has_2 Needs{1,0}` | 5 | 6 | 0 | S0 |
| V14 | LOCKS(xy) | deadlock-free | Proved | — | — | 5 | 0 | S0 |
| V15 | LOCKS(xy) | no capacity exceeded | Proved (certificate) | drop_places = [] | — | 5 | 0 | S0 |
| V16 | PC(overflow) | no capacity exceeded | Disproved | 4 (produce ×4); `Overflow { r, 3, 4 }` | 9 | 18 | 2 | S0 |
| V17 | PC(overflow) | deadlock-free | Disproved | 10 (terminal `c/got = 5`, F = none; overflow reachable elsewhere) | 20 | 18 | 2 | S0 |
| V18 | PC(block) | no capacity exceeded | Proved (certificate) | drop_places = [] | — | 18 | 0 | S0 |
| V19 | PC(block) | deadlock-free | Disproved | 10 (`c/got = 5`); waiting `c/consume ← c/got OutputFull{5,6}, r Needs{1,0}`; `p/produce ← p/items Needs{1,0}` | 18 | 18 | 0 | S0 |
| V20 | PC(drop) | deadlock-free | Disproved | 8 (produce ×5, consume ×3; `c/got = 3`) | 27 | 30 | 0 | S0 |
| V21 | PC(drop) | no capacity exceeded | Proved (certificate) | drop_places = [`r`] | — | 30 | 0 | S0 |
| V22 | POOL | deadlock-free | Proved | — | — | 3 | 0 | S0 |
| V23 | POOL | no capacity exceeded | Proved (certificate) | drop_places = [] | — | 3 | 0 | S0 |
| V24 | POOL-LEAK | deadlock-free | Disproved | 4 (start, fail, start, fail; `c/idle = 3`); waiting `c/fail ← c/idle OutputFull{3,4}, c/running Needs{1,0}`; `r1/start+acquire ← w/free Needs{1,0}`; `r2/finish+release ← c/idle OutputFull{3,4}, c/running Needs{1,0}` | 6 | 6 | 0 | S0 |
| V25 | BRANCH | deadlock-free | Disproved | 1 (deadlock at `x/d = 1`; overflow reachable on the other branch) | 2 | 3 | 1 | S0 |
| V26 | BRANCH | no capacity exceeded | Disproved | 2 (t2, t3); `Overflow { x/p, 1, 2 }` | 4 | 3 | 1 | S0 |
| V27 | `x/a(1, overflow, 1)`, `x/b(1, overflow, 1)`, `x/t: ∅ → x/a, x/b` | no capacity exceeded | Disproved | 1; `Overflow.place = x/a` (smallest ID of two exceeded) | 2 | 1 | 1 | S0 |
| V28 | POOL with r2 removed | deadlock-free | Unknown ModelIncomplete, details exactly [`c/finish is a port but no relationship inside this scope binds it`, `w/release is a port but no relationship inside this scope binds it`] | lowering fails; no worker | — | — | — | S0 |
| V29 | `x/p(3, block, 5)` | any | Unknown ModelIncomplete [`x/p: initial 5 exceeds capacity 3`] | — | — | — | — | S0 |
| V30 | PC(block) | deadlock-free, final `c/got = 5` | Proved | — | — | 18 | 0 | S1 |
| V31 | `x/p(1, block, 1)` | deadlock-free, final `x/p = 1` | Proved | — (M₀ is proper termination) | — | 1 | 0 | S1 |
| V32 | PC(block) | never `r >= 3` | Disproved | 3 | 6 | 18 | 0 | S4 |
| V33 | PC(block) | never `r >= 4` | Proved | — | — | 18 | 0 | S4 |
| V34 | SRC(overflow) | never `x/p >= 3` | Disproved | 3; the last marking is the overflow marking `x/p = 3` (D-02); `Predicate { value: 3 }`; panel adds "this run also exceeds the capacity of x/p (3 of 2)" | 4 | 3 | 1 | S4 |
| V35 | SRC(overflow) | never `x/p >= 2` | Disproved | 2 | 3 | 3 | 1 | S4 |
| V36 | POOL | always reachable `w/free = 2` | Proved | — | — | 3 | 0 | S4 |
| V37 | POOL-LEAK | always reachable `w/free = 2` | Disproved | 2 (start, fail); `markings` = 3 (Reach of `c/idle = 3, w/free = 1`) | 6 | 6 | 0 | S4 |
| V38 | BRANCH | always reachable `x/c = 1` | Disproved | 1 (from `x/d = 1` nothing is reachable; no overflow from there); `markings` = 1 | 4 | 3 | 1 | S4 |
| V39 | BRANCH | always reachable `x/d = 1` | Unknown CapacityExceededFirst { x/p } | Bad = ∅ because `x/p = 1` reaches an overflow marking | — | 3 | 1 | S4 |
| V40 | POOL | no dead transitions | Proved | — | — | 3 | 0 | S4 |
| V41 | `x/p(2, block)`, `x/t: ∅ → 3·x/p` | no dead transitions | Disproved | dead = [`x/t`]; `markings` = 1 | 1 | 1 | 0 | S4 |
| V42 | BRANCH | no dead transitions | Unknown CapacityExceededFirst { x/p } | (no dead transition here; V46 pins the Dead ≠ ∅ case) | — | 3 | 1 | S4 |
| V43 | MASK | deadlock-free | Proved | — (a and b are stuck forever; the scope never stops) | — | 6 | 0 | S0 |
| V44 | MASK | always reachable `a/idle = 1` | Disproved | 2 (ra1, rb1); `markings` = 1 | 6 | 6 | 0 | S4 |
| V45 | MASK | no capacity exceeded | Proved (certificate) | drop_places = [] | — | 6 | 0 | S0 |
| V46 | BRANCH-4 | no dead transitions | Unknown CapacityExceededFirst { x/p } | `x/t4` is dead, but O ≠ ∅ wins (8.5) | — | 3 | 1 | S4 |
| V47 | `x/p(1, block, 1)`, no transitions | never `x/p >= 1` | Disproved | 0 | 1 | 1 | 0 | S4 |
| V48 | FE1 (own scope, closed ports) | no capacity exceeded | Disproved | 5 (open_peer_thread ×5); `Overflow { fe1/peer_threads, 4, 5 }` | 12 | 37 | 13 | S0 |
| V49 | E5 `case1_ebs_fixed`, F = none | deadlock-free | Disproved | 13 (see 13.1) | 99 | 99 | 0 | S0 |
| V50 | E5 `case2_kinesis_fixed` scope (b), F = none | deadlock-free | Disproved | 9 (fail, login+auth ×2, discard+error, login+auth, discard+error, login+auth, discard+error ×2) | 24 | 24 | 0 | S0 |
| V51 | `x/p(MAX − 1, overflow, MAX − 1)`, `x/t: ∅ → x/p` | no capacity exceeded | Disproved | 1; `Overflow { x/p, 4294967294, 4294967295 }` (count = u32::MAX, no overflow of the arithmetic) | 2 | 1 | 1 | S0 |
| V52 | `x/p(MAX − 1, block, MAX − 1)`, `x/t: ∅ → x/p` | deadlock-free | Disproved | 0; waiting `x/t ← x/p OutputFull { 4294967294, 4294967295 }` | 1 | 1 | 0 | S0 |
| V53 | `x/p(MAX − 1, drop, MAX − 1)`, `x/t: ∅ → x/p` | deadlock-free | Proved | — (clamp at the limit) | — | 1 | 0 | S0 |
| V54 | `x/p(MAX − 1, overflow)`, `x/t: ∅ → 2·x/p` | any | Unknown ModelIncomplete [`x/p: capacity 4294967294 plus largest post weight 2 exceeds u32`] | rule 7 | — | — | — | S0 |
| V55 | E5 `case1_ebs_fixed` | deadlock-free, final `api_b/done = 2` | Proved | — (the V49 marking is proper termination) | — | 99 | 0 | S1 |
| V56 | PC(block), own scope of `c` without a behavior model, or a combined scope whose children all lack one | any | key state `nothingToCheck`; no verdict | 6.6 | — | — | — | S0 |
| V57 | PC(block) with `[[property]] kind = "never"` in slice 0 | never `r >= 3` | Unknown PropertyNotCheckable [`user properties are checked from slice 4`] | D-25; no worker. A `pool-share` template entry instead gets [`never holds the whole pool is checked from slice 1`] in slice 0 and is checked from slice 1 (PL S-06, D-60) | — | — | — | S0 |

### 14.2 Budget vectors [S0]

Budgets are markings only unless a fake Clock is named. "main max" = ⌊N/2⌋.

| # | net, property | N | main max | expected |
|---|---|---|---|---|
| B1 | PC(overflow), no capacity exceeded | 16 | 8 | Unknown `BudgetExhausted { Main, markings: 8 }` (the overflow marking is the 9th stored; `c/consume` sorts before `p/produce`) |
| B2 | same | 18 | 9 | Disproved, 4 steps; replay stores nothing |
| B3 | PC(block), deadlock-free | 34 | 17 | Unknown `BudgetExhausted { Main, 17 }` (the deadlock is the 18th stored marking) |
| B4 | same | 36 | 18 | Disproved (V19); replay only, no recheck exploration |
| B5 | POOL, deadlock-free | 5 | 2 | Unknown `BudgetExhausted { Main, 2 }` (|R ∪ O| = 3) |
| B6 | same | 6 | 3 | Proved; recheck gets 3 and completes |
| B7 | LOCKS(xy), deadlock-free | 9 | 4 | Unknown `BudgetExhausted { Main, 4 }` (|R ∪ O| = 5) |
| B8 | same | 10 | 5 | Proved |
| B9 | PC(block), never `r >= 4` (|R| = 18), N = 100, fake Clock: t₀ + T/2 not reached during the main phase, deadline t₀ + T passed at the first poll of the recheck phase | — | — | Unknown `BudgetExhausted { Recheck, markings: 1 }` |
| B10 | POOL, deadlock-free, N = 100, cancel flag set before the main phase starts | — | — | Unknown `Cancelled` |
| B11 | PC(overflow), deadlock-free | 38 | 19 | Unknown `BudgetExhausted { Main, 19 }` (deadlock is the 20th stored marking) |
| B12 | same | 40 | 20 | Disproved (V17) |
| B13 | PC(block), no capacity exceeded | 1 | 0 | Proved (certificate): no exploration happens, so the budget is irrelevant |

The E1 three-example corpus (V16, V13, V22) must each answer within 2 s.

### 14.3 Composition vectors [S0]

Inputs are `OpenNets` values; the expected result is the exact canonical encoding (section 11) from **both** `engine::flatten` and `recheck::compose`, or a `ComposeError` from recheck and the matching L-rule failure from engine. Parts are given in 2.4 notation; a part's transitions that are ports are marked (p). AsyncLink sets are written in brackets.

- **C1 fusion removes both bound transitions.** Parts `a`: `a/idle(1, block, 1)`, `a/busy(1, block)`, `a/call (p): a/idle → a/busy`, `a/done: a/busy → a/idle`; `b`: `b/free(1, block, 1)`, `b/serve (p): b/free → b/free`. Fusion (`r`, `a/call`, `b/serve`). Expected encoding:

  ```
  petrinet-net 1
  place a/busy 1 block 0
  place a/idle 1 block 1
  place b/free 1 block 1
  transition a/done
  pre a/busy 1
  post a/idle 1
  transition r/call+serve
  pre a/idle 1
  pre b/free 1
  post a/busy 1
  post b/free 1
  ```
  Deadlock-free (final none): Proved, 2 markings.
- **C2 caller with two callees (two relationships).** C1's parts plus `c`: `c/free(1, block, 1)`, `c/serve (p): c/free → c/free`; fusions (`r1`, `a/call`, `b/serve`), (`r2`, `a/call`, `c/serve`). Expected: places `a/busy`, `a/idle`, `b/free`, `c/free`; transitions `a/done`, `r1/call+serve` (pre `a/idle 1`, `b/free 1`; post `a/busy 1`, `b/free 1`), `r2/call+serve` (pre `a/idle 1`, `c/free 1`; post `a/busy 1`, `c/free 1`). `a/call` does not appear.
- **C3 callee with two callers, two fusions on one relationship.** Parts `a`: `a/idle(1, block, 1)`, `a/has(1, block)`, `a/take (p): a/idle → a/has`, `a/give (p): a/has → a/idle`; `b` the same with `b/`; `l`: `l/x(1, block, 1)`, `l/acq (p): l/x → ∅`, `l/rel (p): ∅ → l/x`. Fusions (`ra`, `a/take`, `l/acq`), (`ra`, `a/give`, `l/rel`), (`rb`, `b/take`, `l/acq`), (`rb`, `b/give`, `l/rel`). Expected transitions, in this order: `ra/give+rel` (pre `a/has 1`; post `a/idle 1`, `l/x 1`), `ra/take+acq` (pre `a/idle 1`, `l/x 1`; post `a/has 1`), `rb/give+rel`, `rb/take+acq` (mirrored). No `l/acq`, `l/rel`, `a/take`, … remain. Deadlock-free: Proved, 3 markings.
- **C4 asynchronous binding, default when_full.** Parts `p`: `p/items(5, block, 5)`, `p/produce (p): p/items → ∅`; `c`: `c/got(5, block)`, `c/consume (p): ∅ → c/got`; AsyncLink (`r`, [`p/produce`], [`c/consume`], 3, overflow) (lowering filled in the default). Expected:

  ```
  petrinet-net 1
  place c/got 5 block 0
  place p/items 5 block 5
  place r 3 overflow 0
  transition c/consume
  pre r 1
  post c/got 1
  transition p/produce
  pre p/items 1
  post r 1
  ```
  This is PC(overflow).
- **C5 asynchronous binding with block.** As C4 with when_full = block: identical except `place r 3 block 0`. The CheckKey differs from C4's.
- **C6 own scope, closed ports.** One part FE1 (14), no fusions, no links: the encoding lists `fe1/open_peer_thread` with only `post fe1/peer_threads 1` (a free source), and `fe1/receive_info` with `pre fe1/peer_threads 1`, `post fe1/map_entries 1`, `post fe1/peer_threads 1`.
- **C7 duplicate (caller, callee) pair.** C1 with the fusion (`r`, `a/call`, `b/serve`) listed twice: engine fails L12; recheck `ComposeError` ("duplicate fused ID `r/call+serve`").
- **C8 sender that is also a caller (L7 case, meaning on record).** C2's parts with fusion (`r1`, `a/call`, `b/serve`) and AsyncLink (`r2`, [`a/call`], [`c/serve`], 2, overflow): engine fails L7; recheck `ComposeError`. For the record, the composition rule of 6.5 would give `r1/call+serve` with `post r2 1` and `c/serve` with `pre r2 1`; both compositions must produce exactly that if L7 is ever relaxed.
- **C9 self-binding (L6).** Part `a` with ports `a/ping`, `a/pong` and a fusion (`r`, `a/ping`, `a/pong`): engine fails L6; recheck `ComposeError`.
- **C10 caller and callee (L7).** Three parts `a`, `b`, `c`; fusions (`r1`, `a/call`, `b/serve`), (`r2`, `b/serve`, `c/serve`): engine fails L7; recheck `ComposeError`.
- **C11 two asynchronous bindings on one relationship (L8).** Two AsyncLinks with relationship `r`: engine fails L8; recheck `ComposeError` ("duplicate generated place `r`").
- **C12 relationship that binds nothing.** C1 with an extra relationship `r9` between `a` and `b` that binds no ports: encoding identical to C1; `Lowered.assumptions = [r9]`.
- **C13 port bound only from outside the scope (L4).** C1 where `b` is not a child of the scope: engine fails L4 for `a/call`; no worker runs.
- **C14 missing capacity (L13).** C4 with no capacity on the relationship: engine fails L13 ("`r` is asynchronous but has no capacity").
- **C15 final predicate is in the property, not in the net.** PC(block) (C5's `OpenNets`) with `DeadlockFree { final: c/got = 5 }`: both encodings of the net are C5's; the property line is `property deadlock-free final (atom eq 5 c/got 1)`; with `final: None` it is `property deadlock-free final none` and the two keys differ. The key of `capacity-never-exceeded` on the same net is the same in both cases (D-04).
- **C16 property proptest.** `canonical(engine::flatten(O)) == canonical(recheck::compose(O))` for every net above and for 1,000 random valid `OpenNets` (proptest; random AsyncLinks have set sizes 1–3), and both reject the same random invalid ones.
- **C17 asynchronous input event with two transitions** (SM V-22). Parts PRODUCER (`producer/p(1, block, 1)`, `producer/send (p): producer/p → producer/p`) and the compiled PAUSER (`consumer/ready(1, overflow, 1)`, `consumer/paused(1, overflow)`, `consumer/pause: ready → paused`, `consumer/resume: paused → ready`, `consumer/run: ready → ready`, `consumer/skip: paused → paused`, the last two on input event `msg`); AsyncLink (`jobs`, [`producer/send`], [`consumer/run`, `consumer/skip`], 2, block). Expected:

  ```
  petrinet-net 1
  place consumer/paused 1 overflow 0
  place consumer/ready 1 overflow 1
  place jobs 2 block 0
  place producer/p 1 block 1
  transition consumer/pause
  pre consumer/ready 1
  post consumer/paused 1
  transition consumer/resume
  pre consumer/paused 1
  post consumer/ready 1
  transition consumer/run
  pre consumer/ready 1
  pre jobs 1
  post consumer/ready 1
  transition consumer/skip
  pre consumer/paused 1
  pre jobs 1
  post consumer/paused 1
  transition producer/send
  pre producer/p 1
  post jobs 1
  post producer/p 1
  ```
- **C18 state machine to state machine, sets on both ends** (SM V-23). Parts compiled PINGER (`pinger/a(1, overflow, 1)`, `pinger/b(1, overflow)`, `pinger/p1: a → b`, `pinger/p2: b → a`, both on output event `ping`) and PONGER (`ponger/x(1, overflow, 1)`, `ponger/y(1, overflow)`, `ponger/q1: x → y`, `ponger/q2: y → x`, both on input event `ping`); AsyncLink (`pings`, [`pinger/p1`, `pinger/p2`], [`ponger/q1`, `ponger/q2`], 1, block). Expected:

  ```
  petrinet-net 1
  place pinger/a 1 overflow 1
  place pinger/b 1 overflow 0
  place pings 1 block 0
  place ponger/x 1 overflow 1
  place ponger/y 1 overflow 0
  transition pinger/p1
  pre pinger/a 1
  post pinger/b 1
  post pings 1
  transition pinger/p2
  pre pinger/b 1
  post pinger/a 1
  post pings 1
  transition ponger/q1
  pre pings 1
  pre ponger/x 1
  post ponger/y 1
  transition ponger/q2
  pre pings 1
  pre ponger/y 1
  post ponger/x 1
  ```
- **C19 empty set.** C17 with `to = []`: engine never produces it (SM-E11); recheck `ComposeError` ("asynchronous binding jobs has an empty to set").
- **C20 member in two sets (L7, meaning on record).** Two AsyncLinks on different relationships that both list `consumer/run` in `to`: engine fails L7 (D-16); recheck `ComposeError`. For the record, the composition rule of 6.5 would give `consumer/run` one pre arc from each generated place (an AND-join); both compositions must produce exactly that if L7 is ever relaxed.
- **C21 call and reply on one relationship (D-07).** Parts `a`: `a/idle(1, block, 1)`, `a/busy(1, block)`, `a/call (p): a/idle → a/busy`, `a/reply (p): a/busy → a/idle`; `b`: `b/free(1, block, 1)`, `b/work(1, block)`, `b/serve (p): b/free → b/work`, `b/ret (p): b/work → b/free`. Relationship `r` from `a` to `b` with fusions (`r`, `a/call`, `b/serve`) and (`r`, `b/ret`, `a/reply`) (the reply pair runs from the target to the source). Expected:

  ```
  petrinet-net 1
  place a/busy 1 block 0
  place a/idle 1 block 1
  place b/free 1 block 1
  place b/work 1 block 0
  transition r/call+serve
  pre a/idle 1
  pre b/free 1
  post a/busy 1
  post b/work 1
  transition r/ret+reply
  pre a/busy 1
  pre b/work 1
  post a/idle 1
  post b/free 1
  ```
  Deadlock-free: Proved, 2 markings. The scan sees one relationship and no RUN-SYNC-CYCLE (SR RS9).
- **C22 same bare names in opposite directions (L12).** Parts `a` with ports `a/x`, `a/y` and `b` with ports `b/x`, `b/y`; fusions (`r`, `a/x`, `b/y`) and (`r`, `b/x`, `a/y`): engine fails L12 ("`r` binds x + y twice"); recheck `ComposeError` ("duplicate fused ID `r/x+y`").

### 14.4 Encoding vectors [S0]

(a) PC(block) built with places in reverse order encodes to the same bytes; (b) changing `r` from block to drop changes the key; (c) changing `c/consume`'s weight on `r` from 1 to 2 changes the key; (d) renaming the display name of element `p` does not change the key (IDs unchanged); (e) changing `assumptions` does not change the key; (f) the last byte of `canonical(flat)` is LF, so `canonical(flat) ++ canonical_property(p)` has the last `post` (or `transition`) line and `property …` on separate lines; (g) changing `[final] combined` changes the key of `deadlock-free` and, from slice 4, of a T7 "done" property of the combined behavior, and of no other property of the scope (D-04, D-48); (h) an AsyncLink with `to = [c/consume]` and one with `to = [c/consume, c/consume2]` (a second receiving transition) encode differently (one more `pre r 1`).

### 14.5 Predicate vectors [S1 parser; S4 user properties]

| input | places present | result |
|---|---|---|
| `p/in-flight - r >= 1` | `p/in-flight`, `r` | atom `ge 1` with terms `p/in-flight 1`, `r -1` |
| `x/p-1 >= 0` | `x/p` only | parse error "unknown place `x/p-1`" |
| `x/p -1 >= 0` | `x/p` | atom `ge 1` with term `x/p 1` (the constant moved: `x/p − 1 ≥ 0` is `x/p ≥ 1`) |
| `rel-7 - 2 >= 0` | `rel-7` | atom `ge 2`, term `rel-7 1` |
| `2*w/free - c/running <= 4` | both | atom `le 4`, terms `c/running -1`, `w/free 2` |
| `not (p/items = 0 and r = 0)` | both | `(not (and (atom eq 0 p/items 1) (atom eq 0 r 1)))` |
| `a/x - a/x >= 0` | `a/x` | atom `ge 0` with no terms (true) |
| `3 = 3` | — | atom `eq 0` with no terms after normalization (`0 = 0`, true) |
| `2147483647*a/x + 2147483647*a/x >= 0` | `a/x` | parse error (coefficient leaves i32) |
| `-1 * a/x >= 0` | `a/x` | parse error (no unary minus) |
| `and >= 1` | — | parse error (keyword is not an ID) |
| `a/x = 1 and b/y = 0 and c/z = 2` | all | one `(and …)` with three atoms, not nested (PL A1) |
| `e/done + e/failed = 1` | both | atom `eq 1` with terms `e/done 1`, `e/failed 1` (the state machine final-state form, D-05) |

---

## 15. Slice map

| part | slice 0 | later |
|---|---|---|
| 2 structure, weights, IDs, one namespace, validation incl. rule 7 | all | — |
| 3 markings | initial, overflow; F = none | final predicate, minimal form in the editor (S1, D-05) |
| 4 enabling and firing, block/drop/overflow | all three (TOML authors must pick `when_full`) | — |
| 5 predicates | strings opaque (D-25) | lexer, parser, resolution, evaluation for the final predicate and the T5 `pool-share` expansion (S1, D-60); user properties and the other templates (S4) |
| 6 ports, bindings (sets, per-pair direction), flattening, L1–L13, assumptions, nothingToCheck | all except L9 for user properties and L14–L16 | L9 for the final predicate (S1); L14–L16 (S2) |
| 7 deadlock, global-stop wording | with F = none | final predicate (S1) |
| 8 properties | 8.1 (F = none), 8.2 (certificate and exhaustive) | 8.1 with F and 8.3 for T5 `pool-share` only (S1, D-60); 8.3 for user properties, 8.4, 8.5 (S4) |
| 9 evidence | 9.1–9.4, 9.5 NoOverflowPlace, 9.6 | sm_replay step 3b (S2); P-invariant certificates (later; pulled forward if E9 shows I2 < 80 %) |
| 10 budget, Unknown | all seven reasons (Cancelled produced internally, shown from S1; PropertyNotCheckable for user properties present in files, and for a `pool-share` entry in slice 0 only) | — |
| 11 encoding, CheckKey | all (`final none` in every deadlock-free line) | — |
| 12 exploration order | both explorers | verify may deviate once it has reductions |
| 14 vectors | V01–V29, V43, V45, V48–V54, V56, V57, B1–B13, C1–C16, C21, C22, encoding (a)–(h) | V30, V31, V55 and the parser vectors (S1); C17–C20 (S2); V32–V42, V44, V46, V47 (S4) |

State machines (S2) lower to 1-safe parts of `OpenNets`; an input or output event is the set of its transitions, which is why `AsyncLink` carries sets; their states are place ports; nothing else in this spec changes for them (ADR 0002 as amended by ADR 0015, SM spec). `v1-slices.md` is updated by the product manager (D-38).

---

## 16. Amendments in force

Revision 2's proposed amendments were decided in `decisions.md` and are recorded in ADRs; this spec no longer proposes anything.

1. **ADR 0016** (amends ADR 0002, 0010): in a combined scope every transition port must be bound, place ports are never bound; fused IDs use bare names; `AsyncLink` carries sets; binding pairs have their own direction and a relationship carries a call and its reply; L12 regardless of direction; L7 with at most one asynchronous binding per port; children are the elements whose parent is the scope element. (Revision 2 items 1, 4, 11; D-06, D-07, D-13, D-16.)
2. **ADR 0017** (amends ADR 0008, 0010, 0014): never p on overflow markings; always reachable confirmed with "and no overflow marking"; no dead transitions Unknown whenever O ≠ ∅; CapacityExceededFirst with one place and its run; `NoOverflowPlace` as a certificate; the closed list of seven Unknown reasons; a failed replay is "internal error". (Revision 2 items 2, 9, 10; D-02, D-03, D-08, D-09, D-22, D-24.)
3. **ADR 0015** (amends ADR 0002, 0003): L9 as written, with lowering-computed template expansions exempt; states as place ports; events as transition ports; integer weights, negative allowed. (Revision 2 item 3; D-01.)
4. **ADR 0018** (amends ADR 0010, 0013): the final predicate in `DeadlockFree { final }`; slice 1 minimal form; slice 0 keeps predicate strings opaque. (Revision 2 items 6, 8; D-04, D-05, D-25.)
5. **Design §3** carries `Step { fired: QualifiedId, marking }`, `Violation::Predicate { value: i128 }`, `WaitReason`, `CapacityExceededFirst.steps`, `Technique::Certificate`, `Lowered.assumptions` (revision 2 items 5, 7; D-37).
6. **Slug rule** (revision 2 item 12): in ADR 0020 and the file format (D-12).

---

## 17. Glossary additions (edits to `GLOSSARY.md`; the merged block of `consistency-review.md` applies with D-28 to D-33)

Under **Behavior**:

- **Reachable marking**: A marking the net can reach from its initial marking by firing transitions without passing through an overflow marking. The count of reachable markings is what a Proved by exhaustion reports. _Avoid_: state, state space, state count.
- **Overflow marking** (sharpened): A marking in which a place holds more tokens than its capacity. It is reached by exactly one firing into an `overflow` place from an in-bounds marking, violates "no capacity exceeded", and nothing happens after it: it is neither a deadlock nor final. Only "no capacity exceeded" and "never p" look at its counts. _Avoid_: error state, bad marking, terminal state.
- **Run**: A sequence of firings from one marking to another; the formal object inside a counterexample. _Avoid_: firing sequence, trace, path.
- **Qualified ID**: The string that names one place or transition of a flattened net: `element/name` for an element's own place, transition or state, `relationship/caller+callee` (bare names) for a fused transition, `relationship` for a generated place. Places, transitions, states and events of one element share one namespace. _Avoid_: path, key, address.
- **Fused transition**: The one transition a synchronous binding makes from a caller transition and a callee transition; the two bound ports no longer fire on their own. _Avoid_: merged transition, joint step, synchronized transition.
- **Port** (sharpened): A place or transition that a behavior model offers to the outside. A transition port is what relationships bind; a place port is never bound in v1 and exists so that a property of the parent scope can name it. For a state machine, an input or output event is a transition port standing for every transition on it, and every state is a place port; internal events are not ports. _Avoid_: interface, pin, endpoint, hook, input/output port, event port.
- **Binding**: One pair of ports that a relationship connects, with its own direction: synchronous (the two transitions fire as one fused transition) or asynchronous (messages wait in the relationship's generated place). A relationship holds zero or more bindings, at most one of them asynchronous; a call and its reply are two bindings of one relationship. `Fusion` and `AsyncLink` are the code names. Not the relationship's interaction. _Avoid_: link, wire, connection, port mapping.
- **Final predicate**: The linear marking predicate of a scope element (one for its own behavior, one for its combined behavior) that says which markings count as proper termination; checked as part of deadlock-free; the UI calls it "done when"; a state machine's final states contribute to it. Overflow markings are never final. _Avoid_: final state, final markings (as a list), final condition, end state, goal, accept condition, done condition.
- **Dead transition**: A transition that is enabled in no reachable marking. _Avoid_: unreachable transition, unused transition.

Under **Checking**:

- **Unknown** (sharpened): The tool could not decide the property. An Unknown verdict always carries a reason from a closed list: budget exhausted, budget exhausted while re-checking, capacity exceeded first, model incomplete, property cannot be checked, cancelled, internal error or internal disagreement. _Avoid_: inconclusive, open, partial, bounded-proved.
- **Assumption**: A relationship of a scope that takes no part in a check because it binds no ports or leads outside the scope; every verdict of the scope lists them. (The own-scope line "every port can fire at any time" is the closed-port note, not an assumption.) _Avoid_: ignored relationship, exclusion.
- **Canonical encoding**: The sorted text form of a flattened net and a property from which the CheckKey is computed; two nets with the same behavior-relevant data encode to the same bytes. _Avoid_: serialization, hash input, fingerprint.
- **CheckKey**: The hash of the canonical encoding of a net plus a property; the identity of a check for caching and for deciding whether a verdict is current. _Avoid_: check hash, cache key, verdict ID.
- **Step**: One fired transition of the flattened net together with the marking after it; a counterexample is a list of steps from the initial marking. _Avoid_: frame (reserved for the rendered replay), move, tick, event.
- **Certificate**: A small object that proves a property without exploring and that the independent checker verifies on its own composition; in v1 "no overflow place" (with the drop places named) and, later, P-invariant bounds. _Avoid_: proof object, witness.

---

## 18. Closed questions

Revision 2's ten open questions are decided; the table maps each to its decision.

| # | question (revision 2) | decision |
|---|---|---|
| 1 | Places a combined-scope property may name | D-01: L9 kept, with PL A3 and states as place ports; ADR 0015 |
| 2 | L7 fused + asynchronously bound | D-16: kept forbidden; at most one asynchronous binding per port; C8/C20 keep the meaning |
| 3 | Slice of the final predicate | D-05: slice 1, minimal form; slice 0 lowers `final none`; ADR 0018 |
| 4 | Children of a combined scope | D-13: elements whose parent is the scope element; `nothingToCheck` when none has a behavior model |
| 5 | No dead transitions Unknown whenever O ≠ ∅ | D-09: yes; ADR 0017 |
| 6 | Budget time split | D-17: remaining wall clock |
| 7 | Initial marking above capacity | D-08: `ModelIncomplete` |
| 8 | Structural Proved for no capacity exceeded | D-03: yes, as the `NoOverflowPlace` certificate checked by recheck; ADR 0017 |
| 9 | Deadlock replay highlight | D-34: the 6.7 rule is the slice 0 rule; UX may replace it with V13 as the test |
| 10 | Differential corpus encoding | D-35: adopted as written; checked by spike E1 against TAPAAL verifypn 4.2.3 (`docs/product/experiments/e1-state-space.md`): the net-effect complement is required (V08), drop nets stay excluded (V12). Queries: on a net with an overflow place, deadlock-free is `E[inb U (inb ∧ deadlock)]` (a deadlock reachable through in-bounds markings only) followed by `AG inb`, never a plain `EF deadlock`, which explores past overflow markings; no capacity exceeded is `AG inb`, where `inb` is M(p) ≤ cap(p) for every overflow place. The export emits no inhibitor arcs (a verifypn 4.2.3 defect), skips nets with counts near 2³¹ (TAPAAL counts are signed 32-bit), and CI pins the verifypn build by checksum |

---

## Appendix A: Reference pseudocode for the decision procedures

```
explore(N, max):
  R := {M0 -> none}; O := {}; stored := 1; queue := [M0]; test(M0)
  while queue not empty:
    M := pop_front(queue)                       -- one expansion; poll deadline and cancel every <= 1,024 expansions
    for t in T sorted by id:
      if enabled(M, t):
        M' := fire(M, t)
        if overflow(M'):
          if M' in O: continue
          if stored = max: return Exhausted(stored)
          O[M'] := (M, t); stored += 1; test(M')         -- test matters for "no capacity exceeded" and "never p"
        else:
          if M' in R: continue
          if stored = max: return Exhausted(stored)
          R[M'] := (M, t); stored += 1; test(M'); push_back(queue, M')
  return Complete(R, O)

deadlock_free(F): test(M) = in_bounds(M) and enabled(M) = {} and not final(F, M)  -> Disproved(path(M), waits)
               Complete and O = {} -> Proved(|R|); Complete and O != {} -> Unknown(CEF(first O, path))
capacity:      no overflow place -> Proved(Certificate(NoOverflowPlace(drop places)))
               test(M) = overflow(M) -> Disproved(path(M), smallest exceeded place); Complete -> Proved(|R|)
never p:       test(M) = p(M) (in bounds or overflow) -> Disproved(path(M)); otherwise as deadlock_free
always p:      Complete; G0 := {M in R | p(M)} ∪ {M in R | M has a successor in O}; Good := pre*(G0); Bad := R \ Good
               first M in Bad by discovery -> Disproved(path(M), |Reach(M)|); Bad = {} and O = {} -> Proved; else Unknown(CEF)
no_dead:       Complete; O != {} -> Unknown(CEF); Dead := {t | no M in R with enabled(M, t)}; Dead != {} -> Disproved(Dead, |R|); else Proved
```

`enabled`, `fire`, `overflow`, `waits` and `p(M)` are sections 4.1, 4.2, 3.2, 7 and 5.2. `path(M)` follows parent pointers to M₀ and records `Step { fired, marking }` for every edge. `Exhausted(stored)` becomes `BudgetExhausted { phase, markings: stored, elapsed }`.
