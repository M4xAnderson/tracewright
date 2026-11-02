# Property language

Specification `docs/specs/property-language.md`. Status: **Accepted** (applies `docs/specs/decisions.md`; final for implementation once every spec has applied that file), revision 3, 2026-11-02. Author: software architect. It implements ADR 0003 as amended by ADR 0015, 0017, 0018 and 0019 on top of `docs/specs/net-semantics.md` revision 3 (cited below as **NS §n**). Revision 3 applies the decisions this spec owns (D-01–D-05, D-08, D-10, D-11, D-12, D-14, D-20, D-25, D-26, D-28, D-30, D-32, D-33, D-40, D-41, D-42, D-45); §17 maps every former open question. The review fixes of 2026-11-02 (D-60–D-68) are applied in place: T5 ships in slice 1 (§8.3), exact accumulation with a symmetric range (§3.2, E117), the mirrored W501 case, the renderer and detail-line rules, and the vectors P-22–P-27, E-33–E-43, T-16–T-27, V-47, V-48, S-06, S-07. This spec defines no second grammar, canonical form or overflow rule: it cites NS for all three.

Every verdict, length, stored count and marking count in §14.3–§14.5 was recomputed for revision 3 with a throwaway reference implementation of NS §4, §8 and §12 (Python, in a temp directory, not in the repo), with transitions in canonical ID order and E5 case 1 mapped as in NS §13.1 (D-07). It reproduces NS case 1 P1 (6 steps, same marking) and the E5 counts (84 and 216 markings). The parse and error vectors (§14.1, §14.2) are unchanged from revision 2 except P-01, P-09, E-24 and E-31.

Slice tags: **[S0]** walking skeleton (predicate strings opaque), **[S1]** parser and final predicate, plus T5 "never holds the whole pool" alone (D-60), **[S2]** state machines, **[S4]** user properties and the other templates (D-05, D-25).

---

## 1. Ownership: what this spec defines and what it cites

| Topic | Normative text | This spec |
|---|---|---|
| Predicate lexer, grammar, tree shape and atom normalization | **NS §5.3** | transcribes it as EBNF (§3.2), adds the property-kind prefix (§3.1) and the error codes and messages (§10) |
| Predicate abstract syntax and evaluation | **NS §5.1, §5.2** | cites them |
| Meaning of each property kind, including overflow markings | **NS §8**, ADR 0017 | cites it (§6) |
| Evidence, replay, agreement and certificates | **NS §9** | cites it (§6) |
| Unknown reasons | **NS §10.2**, ADR 0017 | maps every property problem to `PropertyNotCheckable` (§10.1) |
| Canonical encoding and CheckKey | **NS §11**, produced by `net::encode` | no second form. §5 shows what the encoding gets from the text |
| Which places a combined-scope property may name | **NS §6.6 L9**, ADR 0015 | resolution procedure and messages (§4) |
| Final predicate inside the deadlock-free property | **NS §3.3, §8.1**, ADR 0018 | text, resolution, storage, errors (§7) |
| Property text, scope and reference resolution, templates, TOML storage, validation messages, what v1 cannot say, property-level vectors | **this spec** (templates: ADR 0019) | — |

Where this spec and NS disagree, NS wins. Terms follow `GLOSSARY.md` with the merged block of `consistency-review.md` as resolved by D-28–D-33 (§16). "Predicate" always means ADR 0003's linear marking predicate.

---

## 2. Property kinds

One spelling per layer (D-33):

| Prose and text form | TOML `kind` | Canonical line (NS §11) | Rust (`net::NetProperty`) | Meaning (NS) | Slice |
|---|---|---|---|---|---|
| `deadlock-free` | `deadlock_free` | `property deadlock-free final none` or `… final <pred>` | `DeadlockFree { final }` | §8.1 | S0 (`final none`); final predicate S1 |
| `no capacity exceeded` | `no_capacity_exceeded` | `property capacity-never-exceeded` | `CapacityNeverExceeded` | §8.2 | S0 |
| `no dead transitions` | `no_dead_transitions` | `property no-dead-transitions` | `NoDeadTransitions` | §8.5 | S4 |
| `never <predicate>` | `never` | `property never <pred>` | `Never(Pred)` | §8.3 | S4 (S1 as T5's expansion only, D-60) |
| `always reachable <predicate>` | `always_reachable` | `property always-reachable <pred>` | `AlwaysReachable(Pred)` | §8.4 | S4 |

Every property has a **scope element** (glossary: scope) and a **behavior**: `own` (the scope element's own behavior model, with its ports closed, NS §6.2) or `combined` (the children in its view, combined through their bindings, NS §6.5). An element that has both a behavior model and children can carry properties of both kinds, so `behavior` is always explicit (§9). Prose says own scope and combined scope; code says `Scope { element, behavior: Own | Combined }` (D-32).

---

## 3. Grammar

### 3.1 Property text

The text form is what the Add property dialog's "Custom" field accepts, what the vectors use and what the Checks line shows (§5). TOML stores `kind` and `predicate` separately (§9), so the prefix never appears in a file.

```ebnf
(* ISO 14977. Whitespace (space, tab) separates words and is otherwise skipped. *)
property      = builtin | user_property ;
builtin       = "deadlock-free"
              | "no" , "capacity" , "exceeded"
              | "no" , "dead" , "transitions" ;
user_property = "never" , predicate
              | "always" , "reachable" , predicate ;
```

- The prefix words are recognized only at the start of the text. They are not reserved inside a predicate. A relationship may be called `never`, and then `never never > 0` is valid.
- A property text is at most 4,096 bytes (LIM-2). A longer text is error E114.
- Columns in messages are 1-based and count Unicode characters of the whole property text. For a TOML `predicate` or `[final]` string they count characters of that string.

### 3.2 Predicate (transcription of NS §5.3, which is normative)

```ebnf
predicate   = disjunction ;
disjunction = conjunction , { "or" , conjunction } ;
conjunction = unary , { "and" , unary } ;
unary       = "not" , unary
            | "(" , predicate , ")"
            | linear , cmp , linear ;          (* exactly one cmp: no chains *)
linear      = term , { ( "+" | "-" ) , term } ; (* no unary minus *)
term        = INT | INT , "*" , ID | ID ;
cmp         = "=" | "==" | "!=" | "<" | "<=" | ">" | ">=" ;

(* lexer, left to right, maximal munch *)
WS  = ( " " | "\t" ) , { " " | "\t" } ;               (* skipped *)
INT = digit , { digit } ;                             (* value ≤ 2,147,483,647 *)
ID  = ( letter | "_" ) , { letter | digit | "_" | "/" | "-" } ;   (* "and", "or", "not" are keywords *)
letter = "A" | … | "Z" | "a" | … | "z" ;              (* ASCII only, as NS §5.3 [A-Za-z] *)
digit  = "0" | … | "9" ;                              (* ASCII only *)
```

Any other character outside a token, including a non-ASCII letter such as `é`, is E101 (E-34).

The consequences below are all NS §5.3:

- The lexer accepts upper case. Element and relationship IDs are lowercase (file-format §4.1, ADR 0020, D-12), so `Billing/inflight` lexes but fails resolution (E201, "Did you mean billing?"); names inside a behavior model keep their case (`api/Ready` is a valid place). `==` means `=`.
- A `-` directly after an ID character belongs to the ID. `api/ready-1` is one ID, and `x/p- 1` is the ID `x/p-` followed by `1` (E111). The fix is a space before a binary minus. No ID or name ends in `-` (file-format §4.1), so `x/p-` never resolves.
- There is no unary minus and no `ID * INT`. The only product is `INT * ID`.
- A comparison whose places all cancel stays an atom with no terms: `3 = 3` is `(atom eq 0)`. This spec only warns about it (W501).

**Tree shape (NS §5.3, D-43).** The parser produces one n-ary `And` per chain of two or more `and`, one n-ary `Or` per chain of two or more `or`, and one `Not` per `not`. Parentheses create nesting. Nothing is flattened, reordered, deduplicated or rewritten. So `(a and b) and c` and `a and b and c` are different trees with different CheckKeys (P-11, P-12), and so are `not not x` and `x`.

**Limits (LIM-1, LIM-2).**

| Limit | Value | Error |
|---|---|---|
| Property text, `predicate` or `[final]` string | 4,096 bytes | E114 |
| Nesting of `(` and `not` | 32 | E113 |
| Integer literal | 2,147,483,647 | E109 |
| Coefficient or constant after combining (NS §2.3 rule 6, §5.3) | final value in [−(2³¹−1), 2³¹−1] (symmetric; −2³¹ excluded) | E117 |
| Atoms per predicate, terms per atom (also for template expansions) | 4,096 each | E307 for templates. Hand-written text cannot reach the limit within 4,096 bytes |

**Exact accumulation (NS §5.3, D-62).** Normalization sums coefficients and constants exactly in i64 (a 4,096-byte text holds fewer than 2,048 literals of at most 2³¹−1, so every intermediate sum is below 2⁴²) and checks only the final coefficient of each place and the final constant. An intermediate value outside the range is not an error (P-24); a final value outside it is E117 (P-19, P-22, P-23). The range is symmetric so that the renderer (§5) can always write |c| or |a| as an INT literal of at most 2,147,483,647, and `parse(render(P)) == P` holds for every atom the parser accepts.

With counts below 2³², coefficients below 2³¹ and at most 4,096 terms, every sum stays below 2⁷⁵, so the i128 evaluation of NS §5.2 cannot overflow. NS §5.2 carries the same 4,096-term limit (D-43).

The parser uses an explicit stack (precedence climbing over an operator stack), never recursion (LIM-3). It runs in time linear in the text length.

---

## 4. Scope, references and typing

### 4.1 Reference forms

| Written | Resolves to | Qualified ID (NS §2.2) | Slice |
|---|---|---|---|
| `element/name`, where `name` is a place | that place of `element`'s Petri net | `element/name` | S1 (final predicate, T5 parameters), S4 (properties, other templates) |
| `element/name`, where `name` is a state | the place compiled for that state (1 token while the state is active); every state is an implicit place port (ADR 0015) | `element/name` | S2 |
| `relationship` (no `/`) | the generated place of that relationship's asynchronous binding (messages in transit) | `relationship` | S1 (final predicate, T5 parameters), S4 |

Places, transitions, states and events of one element share one namespace (NS §2.2, ADR 0015); there is no `element/state:<name>` form. A clash is a **model problem**, never a load failure (D-26, ADR 0020): a place and a transition with the same name is `nameClash` (file-format §6.4, NS L10); a state and an event with the same name is SM-E04 (NS L14). The file loads, the editor rejects the gesture, and every scope containing the element is blocked.

### 4.2 Typing

The grammar already guarantees that every comparison is between two linear sums with integer coefficients, and that only `and`, `or` and `not` combine comparisons. Resolution assigns each ID a kind: place, state, transition, event, element, synchronous relationship or asynchronous relationship. A predicate is well typed iff every ID resolves to a **place** (a state counts as a place) that the property may name. There are no other types.

### 4.3 Resolution procedure (normative; `model::validate`, re-checked per property in `engine::lower_property` with the E2xx codes below, including NS L9; property references are not NS L3, D-47)

References are resolved in text order. **Every** reference is checked and every failing one is reported. For each reference the first failing rule below decides its error, so each reference gets exactly one error. A failing reference ends only the property (or final predicate, §7) that contains it, as `PropertyNotCheckable` (§10.1); it never fails the lowering of the scope's net.

ID containing `/`: split at the first `/` into `e` and `n`.

1. `e` is an element of the project. Otherwise **E201**.
2. Own behavior: `e` is the scope element. Combined: `e` is a child of the scope element, that is, an element whose `parent` is the scope element (NS §6.6, D-13). Otherwise **E206**.
3. `n` is a transition or event of `e`: **E203**.
4. `n` is a place or state of `e`. Otherwise **E202** (this also covers `a/b/c`).
5. Combined behavior: `n` is a place port of `e`; every state is one (NS L9, ADR 0015). Otherwise **E208**.

ID without `/`:

1. It is a relationship of the project. Otherwise **E204** if it is an element ID, else **E207**.
2. Combined behavior, and both ends of the relationship are children of the scope element. Otherwise **E206** (an own-behavior property can never name a generated place).
3. The relationship carries an asynchronous binding (NS L8: at most one). Otherwise **E205**.

Template parameters that name places (§8) resolve by the same rules. Their messages name the parameter instead of a column.

### 4.4 Which places a combined-scope property may name (D-01, ADR 0015)

In a combined scope a property, a template parameter or a final predicate names only **place ports** of children (states included) and **generated places** of the scope (NS L9). A place port is never bound in v1 (no place fusion), so promoting a place to a port is one attribute and adds no binding obligation; the template picker offers "make it a port" inline. Port flags are not part of the CheckKey (NS §6.1). In an own scope any place of the element may be named. The one exception is a template expansion that lowering computes from the model and does not store (T7 "its initial marking", §8.2): it may name every place.

E5 case 1 is ported with `control_plane/in_a`, `control_plane/wait_a`, `control_plane/ret_a` and `control_plane/threads_free` as place ports. The slice 4 acceptance example is `never billing/inflight + shipping/inflight > 2` with both places declared as ports (P-09).

### 4.5 Dangling references and ID reuse

- A deleted element, place, state or relationship leaves a dangling reference. The loader reports it (FMT-2). The property keeps its text, and its key state is Unknown `PropertyNotCheckable` with the E2xx line (§10.1). The delete gesture warns first ("2 properties refer to Billing / inflight"). That is a UX item.
- **No silent retargeting** (research F1). The slug generator never issues an ID that any property, final predicate or template parameter in the project still references, whether the reference is dangling or not (file-format §4.3 adopts this). A place created after `api/waiting` was deleted therefore gets `waiting-2`, and the old property stays E202 instead of silently naming the new place (vector E-32).

---

## 5. From text to `NetProperty` and the CheckKey

1. **[S1]** `model::property` parses the text (§3) into NS §5.1 `Pred`. It normalizes **atoms only**, as NS §5.3 specifies: places move left and the constant moves right, equal places are merged, zero coefficients are dropped, terms are sorted by ID, and `=`/`==` becomes `eq`. No other rewriting happens. `<` stays `lt`, `not` stays. In slice 0 every predicate and `[final]` string is opaque: stored and written back verbatim, never parsed (D-25, §10.1).
2. `engine::lower(model, scope: Scope)` builds the scope's net (`flat`). Then, per property, `engine::lower_property` (design §3, D-64) resolves and type-checks (§4), copies the scope element's final predicate of that behavior into the deadlock-free property (§7), expands templates and checks their preconditions (§8), and returns `Result<(NetProperty, Expansion), Diags>`. `Err(Diags)` is the property's key state Unknown `PropertyNotCheckable` with one line per diagnostic (§10.1); it never touches `flat` or the other properties of the scope. `Expansion { cap, g, n, k }` holds the values a template read from the model (all empty for built-ins and custom predicates); it is carried with the verdict for the headline and W10 (§8.1 rule 3), never hashed:

   ```rust
   // design §3, NS §9.1, ADR 0018
   pub enum NetProperty { DeadlockFree { final: Option<Pred> }, CapacityNeverExceeded, NoDeadTransitions, Never(Pred), AlwaysReachable(Pred) }
   ```
3. The worker computes the CheckKey from the flattened net it explored and from `net::encode::canonical_property(&NetProperty)` (NS §11, ADR 0009, ADR 0013). Neither `engine` nor the UI ever prints a canonical line.

**What the key covers.** The canonical property line plus `canonical(flat)`. Behavior `own` and `combined` lower to different nets, so they get different keys. The final predicate is in the key of the deadlock-free property and of a T7 "done" property, whose expansion is the final predicate (§7), and of no other property. The property `id`, `label`, the template kind, the `holder` label, the stored spelling and the port flags of places are **not** in the key. Two spellings with the same `Pred` share a key (`a/x == 1` and `a/x = 1`; `x/p -1 >= 0` and `x/p >= 1`). The predicate is hashed **as parsed** (D-10, ADR 0019): equivalent but different trees do not share a key (`p > 50` and `p >= 51`, V-02 against NS V33).

**"Checks:" line (wrong-Proved defense, catalog §2.3 b).** The verdict panel shows `Checks:` followed by the §3.1 text form of the property exactly as checked, rendered from the `NetProperty` the worker hashed, never from the stored text or template parameters (design `VerdictDto.checks`). For deadlock-free a second line `Done when: <predicate>` or `Done when: none` is rendered from the hashed `final`. The tables in §14 show only the predicate part.

- Atom: the positive terms on the left, the negative terms (with their absolute coefficient) on the right, each side in ID order. A constant c > 0 goes last on the right and c < 0 last on the left: written `+ |c|` after a term, and `|c|` alone when that side has no terms (`never 3 >= 0` renders `3 >= 0`, not `0 + 3 >= 0`; P-25, P-26). A side with neither terms nor constant is `0`. A coefficient of 1 is omitted, and any other coefficient is written `k*id`. Every rendered number is at most 2,147,483,647 because the range is symmetric (§3.2).
- `And` and `Or` children are joined with ` and ` or ` or `. A child that is an `And` or `Or` is parenthesized. `Not` renders as `not x` for an atom or a `Not`, and as `not (x)` otherwise.
- **Round trip (golden property, §12):** `parse(render(P)) == P` for every `P` the parser or a template can produce. P-04 renders `p/in-flight >= r + 1`. P-13 renders `a/x >= 1 or (b/y >= 1 and c/z >= 1)`.
- The panel shows the same rendering with display names (W8 of the catalog). Qualified IDs appear only in the Checks line.

**Headline.** The headline is generated, never free text:

- built-ins: the T1/T2/T8 sentence of the catalog;
- templates: the template sentence, with values (capacity, N, K) taken from the `Expansion` the verdict carries (D-64), never from the current model. T7 "done" names the predicate it checked, rendered from the hashed `AlwaysReachable(F)` with display names: "<Scope> can always reach done when: <F>" (catalog T7);
- custom predicates: "Never: …" or "Can always get back to: …" followed by the rendered predicate with display names.

The optional `label` (§9) is shown only as a secondary line under the headline. It can never replace the headline or the Checks line. So a stale `label` over an edited predicate cannot make a Proved read as a different question.

---

## 6. Meaning and evidence per kind

The definitions, outcome tables and "Proved means" lines are NS §8.1–§8.5. The evidence types are NS §9.1, replay is NS §9.3, agreement is NS §9.4 and certificates are NS §9.5. Summary (normative only through NS):

| Kind | Proved in v1 (technique) | Disproved evidence | `recheck` confirms Disproved by | Certificates later (NS §9.5 only) | Unknown "capacity exceeded first" |
|---|---|---|---|---|---|
| deadlock-free | exhaustive (both explorers agree on \|R\| and \|O\|) | `Counterexample`, `Violation::Deadlock { waiting }` | replay. The last marking is in bounds, enables nothing and is not final under the property's own `final` | none in v1 | complete exploration, no deadlock, O ≠ ∅ |
| no capacity exceeded | certificate `NoOverflowPlace { drop_places }` if no place overflows when full, accepted by `recheck::check_certificate` on its own composition (D-03); otherwise exhaustive | `Counterexample`, `Violation::Overflow` | replay. The last marking is an overflow marking, and `place` is its smallest-ID exceeded place | P-invariant bound | never |
| no dead transitions | exhaustive | `Exhaustive { dead, markings }` | its own exploration agrees on the dead set and on `markings` | none | whenever O ≠ ∅, even with dead transitions (D-09) |
| never p | exhaustive | `Counterexample`, `Violation::Predicate { value }` | replay. The last marking satisfies p and may be an overflow marking (D-02) | P-invariant, only for one atom with all coefficients ≥ 1 and `ge`/`gt`, together with the no-capacity condition | complete exploration, no p-marking in R ∪ O, O ≠ ∅ |
| always reachable p | exhaustive | `CounterexampleThenExhaustive { cx, markings }` | replay, then its own exploration from the last marking finds no p-marking, no overflow marking, and the same `markings` | none | Bad = ∅ and O ≠ ∅ |

- **Withdrawn since revision 1:** "ILP infeasibility of the state equation" for `never p`, and "siphon/trap" for deadlock-free. The state equation M = M₀ + C·σ does not hold on `drop` places (clamping; NS §9.5 I2), so that certificate could produce a wrong Proved. Neither is a certificate kind of NS §9.5; adding one needs an NS amendment first.
- **Overflow markings (NS §3.2, §8.3, ADR 0017).** `never p` is evaluated on overflow markings: it holds iff no marking of R ∪ O satisfies p. A p-marking reached as an overflow marking is Disproved by the run to it, and the panel adds that the run also exceeds a capacity (NS §9.6). The shortest counterexample may therefore end in an overflow marking (V-23: 3 steps instead of the 4-step in-bounds run). No other property reads an overflow marking's counts: always reachable and no dead transitions keep NS §8.4 and §8.5 (D-09).
- **"Capacity exceeded first" names** the smallest-ID exceeded place of the first overflow marking in discovery order, and carries the run to it, which recheck replays (NS §10.2, D-22).
- **`Violation::Predicate { value: i128 }` (NS §9.1).** `value` is the left-hand sum Σ aᵢ·M(pᵢ) of the **first atom in pre-order** of the `Pred`. It is informational: NS §9.3 does not compare it, so it is never displayed. **Every** predicate value the panel shows, the first atom's included, is computed by `engine` from the last marking of the counterexample after `recheck::replay` accepted it (NS §9.3 step 2e makes that marking the replayed one; M₀ for zero steps), one value per atom in pre-order (D-64). The DTO carries each value as a decimal string (IPC-2: it may exceed 2⁵³).
- **Own behavior.** Every own-behavior verdict carries the **closed-port note** "every port of `<element>` can fire at any time" (NS §6.2, §9.6). It is not an assumption: that word is reserved for relationships that take no part in a check (D-30).
- **Always reachable wording** (NS §8.4, E5 implication 5): "can always get back to …", never "recovers" or "will".
- **Counterexamples are definite.** Both explorers explore in the normative order of NS §12, so every counterexample in §14 has a fixed length and fixed steps, and a test may assert them.

---

## 7. Final predicates (ADR 0018)

- A **final predicate** (UI label: **done when**) is model data of the scope element (ADR 0003): one per behavior, so at most two per element. TOML: `[final]` with optional keys `own` and `combined`, each holding one predicate (D-28). A disjunction states several ways to terminate properly.
- It uses the grammar of §3.2 and the resolution of §4.3, including L9 for `combined`.
- **It lives in the deadlock-free property, not in the net** (D-04). Lowering copies F of the property's behavior into `NetProperty::DeadlockFree { final }`; with no F it lowers `final none`. The canonical line is `property deadlock-free final none` or `property deadlock-free final <pred>`; `canonical(flat)` has no `final` line (NS §11). Editing done when therefore changes the key of the deadlock-free property of that behavior and of every T7 "done" property of that behavior (whose expansion is F, §8.2), and of nothing else: "no capacity exceeded" and `never p` never go stale because of it (ADR 0013).
- **No per-property override** (D-41). `[final]` is the only source. The key `final` inside a `[[property]]` entry is reserved: E407, kept verbatim on save, property not checked (E-31).
- **State machines.** A state machine's final states contribute to its element's final predicate. From slice 2, an element whose state machine has a non-empty `final` list and no stated `own` final predicate gets `Σ e/f = 1` over its final states (one atom, one or several states) as its own-scope final predicate (D-42, SM §3.6).
- **Errors.** A final predicate with an E1xx or E2xx error makes every deadlock-free and T7 "done" property of that behavior Unknown `PropertyNotCheckable`, each line prefixed "done when: " (S-05). Other properties of the scope are unaffected. A final predicate with an error is never lowered as `final none` (§17 C3).
- Overflow markings are never final (NS §7).
- **Slices (D-05, D-25).** S0: `[final]` strings are opaque; a `[final]` table present in slice 0 is the warning `finalNotChecked` ("not checked until slice 1") and every deadlock-free property lowers `final none` (S-04). S1: the parser, resolution and evaluation in both explorers; the editor offers the minimal form, a conjunction of `place = n` atoms and, for a state machine with several final states, the single sum atom `Σ e/f = 1`; hand-written `[final]` text may be any §3.2 predicate (one grammar, no second parser). The TOML shape is the same in every slice. The E5 files put `final` on each property; the E1 port moves it to `[final] combined`.

---

## 8. Templates (ADR 0019)

### 8.1 Rules for every template

1. A template is a stable kind ID and version plus parameters. It is stored that way and expanded in `engine::lower` at every check into **exactly one** `NetProperty`; it adds no semantics (catalog §2). A template is never a group of properties with a summary (D-11).
2. Expansion runs in shared lowering, which ADR 0010 does not cross-check. Each template and each preset therefore has a lowering golden test, and the panel's Checks line is rendered from the hashed `NetProperty` (§5).
3. **Values follow edits.** `cap`, `M₀`, `g` and `F` are read from the model at every lowering. An edit changes the expansion, hence the key, hence the verdict goes stale. The verdict keeps its `Expansion { cap, g, n, k }` in memory for display ("capacity 50 then, 30 now", catalog W10; never persisted, ADR 0005; design §3 `Verdict`, `VerdictDto.expansion`, D-64).
4. **Fixed forms.** Each expansion has the exact `Pred` given in §8.2, and that `Pred` round-trips through its Checks line (§5). "Edit as predicate" therefore produces a custom property with the same key, frozen at today's values (catalog §2.4).
5. Parameter IDs resolve by §4.3, including L9. The one exception is the T7 "its initial marking" preset: it is computed at lowering, stores no IDs, and may name every place (NS L9, ADR 0015).
6. A failed precondition never yields a vacuous verdict. The property ends Unknown `PropertyNotCheckable` with the E3xx line of §10.4, and the picker applies the same check before it adds the property (catalog §6, W9).
7. Changing an expansion rule is a format change: version bump and migration (FMT-1, ADR 0019, catalog §2.3 c).

### 8.2 Catalog templates

`cap(p)`, `wf(p)` and `M₀(p)` are taken from the flattened net. In `g(p)`, t ranges over the transitions of the flattened net with W⁺(t, p) > 0, so fused transitions count as one step. F is the scope element's final predicate for the property's behavior (§7).

| ID (catalog) | Stored as | Parameters | Expands to (`NetProperty`, NS §5.1 atoms) | Preconditions (§10.4) | "Does not mean" (catalog §5, exact) | Slice |
|---|---|---|---|---|---|---|
| T1 No deadlock | `kind = "deadlock_free"` | none | `DeadlockFree { final: F }` (`final none` without F) | — | "No deadlock does not mean done: a timeout or retry loop can run forever." Headline: Proved "<Scope> never deadlocks", Disproved "<Scope> can deadlock" (GLOSSARY Deadlock) | S0 (F from S1) |
| T2 No capacity exceeded | `kind = "no_capacity_exceeded"` | none | `CapacityNeverExceeded` | — | "Places that block or drop can still make senders wait or lose messages." | S0 |
| T3 Never overflows | `template = "never-overflows"`, v1 | `place` | `Never(atom gt cap(p) [(p, 1)])` | wf(p) = overflow (E301). cap(p) ≤ 2³¹−1 (E308) | "Holds for this initial marking only; more callers may overflow it." | S4 (its Disproved ends at an overflow marking, D-02, T-08) |
| T4 Never more than | `template = "never-more-than"`, v1 | `terms` = [{`place`, `weight` ≥ 1, default 1}] (1–256, distinct), `at_most` N ≥ 0 | `Never(atom gt N [merged, sorted terms])` | not decided by when-full alone: E302 if every term's place blocks or drops and N ≥ Σ weightᵢ·cap(pᵢ). N ≤ 2³¹−1 (E308) | "A bound for this initial marking only; more callers may exceed it." | S4 (D-60: no evidence gate) |
| T4 preset "never fills up" | `template = "never-fills-up"`, v1 | `place` | g = max over t of (W⁺(t, p) − W⁻(p, t)); `Never(atom gt (cap(p) − g) [(p, 1)])` | g ≥ 1 and cap(p) − g ≥ 0 (E303) | as T4 | S4 |
| T5 Never holds the whole pool | `template = "pool-share"`, v1 | `pool` P, `held_by` H (1–256, distinct), `holder` (display only), `k` (optional) | K = `k` if given, else **M₀(P) + Σ_{h∈H} M₀(h)**; `Never(atom ge K [(h, 1) for h in H, sorted])` | K ≥ 1 (E304, default or given). P ∉ H and H ≠ ∅ (E305). K ≤ 2³¹−1 (E308) | "Others may still hold the rest; waiting calls may never be served." | **S1**, alone and unconditionally (D-60, §8.3) |
| T6 Never at the same time | held back (D-11) | — | would be `Never(And(atoms ge 1))` | — | — | after two journal lock questions |
| T7 Can always get back to | `template = "get-back-to"`, v1 | `target` = [{`place`, `cmp` ∈ `=`, `>=`, `<=`, `value`}] (1–256, no repeated (place, cmp)); `target` and `preset` are never both given (E412, T-25) | one row: `AlwaysReachable(row atom)`. Several: `AlwaysReachable(And(row atoms sorted by place ID, then eq < ge < le))` | values ≤ 2³¹−1 | "A way back exists; under load or with timeouts it may never be taken." | S4 |
| T7 preset "its initial marking" | `template = "get-back-to"`, `preset = "initial"` | none | rows = every place of the flattened net, `eq M₀(p)`, in place ID order | at most 4,096 places (E307) | as T7 | S4 |
| T7 preset "done" ("can always still finish") | `template = "get-back-to"`, `preset = "done"` | none | `AlwaysReachable(F)` | F exists (E306) | as T7. Headline names F: "<Scope> can always reach done when: <F>" (§5) | S4 |
| T8 Every transition can fire | `kind = "no_dead_transitions"` | none | `NoDeadTransitions` | — | "Each can fire in some run, not in every run or the way you intend." | S4 |
| Custom | `kind = "never"` or `"always_reachable"`, `predicate` | text | parsed (§5) | — | custom `always reachable` shows T7's line. Custom `never` shows NS §8.3's "Proved means" line; any further line is UX copy (§17 OQ11) | S4 |

Notes:

- **T5's default K** is the pool's initial tokens **plus** the held-by places' initial tokens (D-11). It asks "can the holder hold the whole pool", also when a held-by place starts non-empty (T-03: pool 2 and held 1 give K = 3).
- **T4 "never fills up" with g > 1.** N = cap − g is sound: a Proved means no step ever waits or drops at p. A Disproved means only that p can exceed cap − g, so for g > 1 the sentence names N instead of "reach the capacity" (catalog T4; §17 OQ10).
- **T7 "can always clear pending"** (research §4) is T7 with the target `pending = 0`. Its help text must state the research precondition: `pending` is not `drop`, and only answer transitions consume from it. Otherwise a design that sheds every request is Proved. The tool cannot infer which transitions answer, so v1 only shows this text. It does not enforce the precondition.
- **CheckKey sharing (D-10).** T3 on a place with capacity 50 shares its key with a hand-written `never p > 50`, but not with `never p >= 51` (§5). Catalog acceptance criterion 13 says so; T-16 against V-24 pins it on a fixture.

### 8.3 T5 in slice 1 (D-60, ADR 0023)

T5 "never holds the whole pool" ships in slice 1, alone and unconditionally; the D-11 evidence gate is removed. T4 and its "never fills up" preset stay in slice 4, as do T3, T7, T8 and custom predicates. Exactly this moves into slice 1:

1. **Template parameters for `pool-share` v1 only.** `model::validate` parses and resolves `pool` and `held_by` by §4.3 (E2xx, including L9 and E208 in a combined scope) and type-checks `holder` and `k` (E412). Parameters of every other template stay verbatim until slice 4 (D-49).
2. **Expansion and its preconditions** in `engine::lower_property`: default K, `Never(atom ge K [(h, 1) for h in H, sorted])`, and E304, E305 and E308. No other E3xx code is reachable in slice 1.
3. **The picker entry for T5**, with the place picker of §4.4 ("make it a port" inline) and the same precondition checks before the property is added (§8.1 rule 6). The picker offers no other template before slice 4.
4. **`never p` in both explorers**: NS §8.3 (evaluated on R ∪ O, D-02), `Evidence::Counterexample` with `Violation::Predicate`, its `recheck::replay` rule (NS §9.3 step 3), and the `property never <pred>` line of `canonical_property` (NS §11). NS §5.2 evaluation already ships in slice 1 for the final predicate; only T5 lowers to `Never` before slice 4.
5. **The S-02 exemption**: from slice 1 a `pool-share` entry is not gated (S-06). In slice 0 it is Unknown `PropertyNotCheckable` with the line "never holds the whole pool is checked from slice 1".
6. **Panel**: the T5 headline, "does not mean" line, Covers line with the held-by places, Disproved help, the Checks line rendered from the hashed property (§5), and the `Expansion` value K for W10 (D-64).

Not in slice 1: the T5 hint about steps that take from the pool (catalog T5, slice 4), "edit as predicate" (slice 4) and the vacuity guard, which needs "no dead transitions" (slice 4, D-20, D-61). Vectors T-01–T-03, T-17, T-18, T-20–T-22 and T-24 are slice 1 vectors.

---

## 9. Storage in TOML (the file format spec adopts this)

### 9.1 Shape

Properties live in the file of their scope element (ADR 0001, ADR 0003).

```toml
# elements/ebs.toml: scope element of E5 case 1 (children api_a, api_b, control_plane, cluster_a, cluster_b)
[[property]]
id = "az-a-pool-share"
label = "AZ-A must not starve AZ-B"
behavior = "combined"
template = "pool-share"
template_version = 1
held_by = ["control_plane/in_a", "control_plane/wait_a", "control_plane/ret_a"]
holder = "AZ-A calls"
pool = "control_plane/threads_free"

[[property]]
id = "deadlock-free"
behavior = "combined"
kind = "deadlock_free"

[[property]]
id = "pool-share-by-hand"
behavior = "combined"
kind = "never"
predicate = "control_plane/in_a + control_plane/wait_a + control_plane/ret_a >= 3"

[final]
combined = "api_b/done = 2"
```

The four `control_plane` places and `api_b/done` are place ports of their elements (§4.4).

### 9.2 Field rules

| Key | Required | Rule |
|---|---|---|
| `id` | yes | ID alphabet (file-format §4.1, D-12: lowercase, letter-first, at most 48 characters), unique in the project (FMT-3, §4.5). It is the `PropertyId` |
| `label` | no | secondary display text, ≤ 200 characters, never the headline (§5) |
| `behavior` | yes | `"own"` or `"combined"`. There is no default: a default would change meaning when children are added. The key is not named `scope`, because the glossary's scope is the element |
| `kind` | exactly one of `kind` and `template` (E410) | `deadlock_free`, `no_capacity_exceeded`, `no_dead_transitions`, `never`, `always_reachable` |
| `predicate` | for `never` and `always_reachable` only (E401, E402) | stored verbatim. The engine never rewrites it, because IDs never change (FMT-3) |
| `template`, `template_version` | together | §8.2 IDs, version 1. An unknown pair is E411 |
| template parameters | per §8.2 | `place`, `terms`, `at_most`, `pool`, `held_by`, `holder`, `k`, `target`, `preset`. A missing or ill-typed one, a repeated place, or `target` together with `preset` is E412. The names and TOML types of every template's parameters (the §8.2 parameter schema) are needed from slice 0, because E411 and E412 are file rules checked in every slice; place IDs inside them are not parsed or resolved before the slice that checks the template (§10.1) |

- **Writer order (FMT-3):** keys in the order `id, label, behavior, kind, predicate, template, template_version`, then the template parameters alphabetically. Entries sorted by `id`. Existing files are edited in place (ADR 0011).
- **Limits (LIM-1):** at most 256 properties per element and 4,096 per project (E409). Exceeding them is a load problem, never a truncation.
- **Unknown keys** in a `[[property]]` entry are kept verbatim on save and reported as E407. The property is then not checked (FMT-1: never dropped silently). `final` is such a key (D-41).
- **Built-ins are explicit entries** (D-20, ADR 0019). What is checked is exactly what is in the files (J6). The New project flow seeds `deadlock_free` and `no_capacity_exceeded` with `behavior = "combined"` on the system element (slice 0).

### 9.3 Expansion rules are format

A template's stored form is its kind, version and parameters (D-11, ADR 0019). Changing an expansion rule (for example T5's default K) is a format change: version bump and migration (FMT-1). Storing the expanded predicate with a template tag and recognizing templates by shape were rejected (ADR 0019): both freeze values, so "never fills up", "its initial marking" and "done" could not follow edits.

### 9.4 Commands (design §3)

`Command::SetProperty { property: PropertyDraft }` (id, `Scope`, kind or template + version + parameters, predicate text), `Command::DeleteProperty { id }` and `Command::SetFinal { scope: Scope, predicate: Option<String> }`. `model::apply` rejects a draft with any E1xx, E2xx or E4xx error, and returns the first message as `Rejected` (SEC-2: WebView text is untrusted). In slice 0 `apply` checks only E4xx and stores predicate text without parsing it (D-25). E3xx preconditions are not checked at apply time, because a later model edit can break them; lowering checks them. A hand-written file with errors still loads (FMT-2).

---

## 10. Validation

### 10.1 Layers and outcome

| Layer | Checks | Codes |
|---|---|---|
| `model::property` (pure text) [S1] | lexing, parsing, atom normalization, i32 | E1xx |
| `model::validate` (Model) | resolution and typing (§4) [S1 for `[final]` and the `pool-share` parameters (D-60), S4 for `predicate` and the other templates' parameters; D-49], TOML field rules including the §8.2 parameter schema [S0], warnings | E2xx, E4xx, W5xx |
| `engine::lower_property` (per property, after `lower` built `flat`; D-64) | re-resolution by §4.3 (E2xx, including NS L9; never NS L3, D-47), template expansion and preconditions, value ranges | E2xx, E3xx |

- **Errors** (E) prevent the check of the property that contains them. Its key state is Unknown **`PropertyNotCheckable(details)`**, headline "Property cannot be checked" (NS §10.2, ADR 0017, D-08), with one detail line per error, ordered by code and then by column (a template parameter error, which has no column, sorts by parameter name after the errors with a column of the same code). A line without a code, such as the slice gate line below, comes after every coded line, so an E4xx line and the gate line appear in that order (S-07). The "done when: " prefix of S-05 does not change the order. A property error never fails the lowering of the scope's net, so other properties of the scope still run; `ModelIncomplete` is reserved for the net (NS §6.6 L1, L2, L4–L8, L10–L16, and L3 for relationship and port references) and wins when both apply (NS §10.2).
- **Warnings** (W) let the check run and are shown next to the property.
- **Nothing to check.** A scope without children, a combined scope where no child has a behavior model, and an own scope whose element has none have the key state `nothingToCheck`: no verdict, no Unknown (NS §6.6, D-13). This replaces revision 2's E404 and E405, which are retired and not reused.
- **Slice gating (D-25, D-60).** Before slice 4 a `[[property]]` of kind `never`, `always_reachable` or `no_dead_transitions`, or one with a `template` other than `pool-share`, is Unknown `PropertyNotCheckable` with the line "user properties are checked from slice 4" (NS V57; S-01 to S-03). A `pool-share` entry is gated only in slice 0, with the line "never holds the whole pool is checked from slice 1" (S-06); from slice 1 it is checked (§8.3). A string is parsed only in the slice that first checks it (D-49, FF §6.8): slice 1 parses and resolves `[final]` and the `pool-share` parameters; `predicate` strings and the other templates' parameters stay verbatim and are parsed from slice 4. The S0 reader still knows the §8.2 parameter schema (names, TOML types, at most 256 list entries, no repeated place, `target` versus `preset`; an empty list is left to the E305 precondition), because E411 and E412 are E4xx file rules reported from slice 0 (§9.2). The one exception is E114, a length check that needs no parser and is reported in every slice, including slice 0. No other E1xx or E2xx is reported for a string the slice does not parse. A `[final]` table in slice 0 is the warning `finalNotChecked` (§7).
- Messages use display names where they name existing model objects (`<Element> / <place>`), and the user's text verbatim where it names something unknown. Suggestions pick one ID of the same kind in scope with edit distance ≤ 2, ties by ID order. The copy is draft for slice 4; the senior UX designer may reword it without changing the facts (D-46).

### 10.2 Syntax errors (E1xx)

| Code | When | Message |
|---|---|---|
| E101 | character outside the lexer | "Unexpected character “<c>” at column <col>. A predicate uses place IDs, whole numbers, + - *, comparisons, parentheses and the words and, or, not." |
| E102 | known misspelling at the error position: `&&`, `&`, `\|\|`, `\|`, `!` (not `!=`), `<>`, `=<`, `=>`, `≥`, `≤`, `≠` | "Use <right> instead of <wrong> (column <col>)." `<col>` is the column of the first character of the misspelled token (`&` of `&&`, `=` of `=>`; E-01–E-03) |
| E104 | a sum where a comparison is required | "<text> is a count, not a predicate. Compare it with a number, for example <text> > 0." |
| E105 | second comparison after `linear cmp linear` | "Compare two sides at a time. Write <a cmp b> and <b cmp c>." |
| E106 | unbalanced parenthesis | "The parenthesis at column <col> is not closed." / "The closing parenthesis at column <col> has no opening one." |
| E107 | `(` … `)` around a sum | "Parentheses group predicates, not sums. Remove them: <text without them>." |
| E108 | `*` not in the form `INT * ID` | "Multiply a place only by a whole number written first, for example 2 * <place>." |
| E109 | literal > 2,147,483,647 | "<number> is too large. Numbers in a predicate can be at most 2,147,483,647." |
| E110 | text ends inside a predicate | "The predicate ends too early after <last token>. Add <expected>." |
| E111 | unexpected token | "Unexpected <token> at column <col>." If the previous ID ends in `-`, add: "<id> ends with a minus sign. Put a space before a minus sign: <a> - <b>." If the token is `and`, `or` or `not`, add: "and, or and not are words of the language, not IDs." |
| E112 | text starts with no property word | "Start with never or always reachable, or use deadlock-free, no capacity exceeded or no dead transitions." For `always <pred>`: "To say a predicate always holds, write never not (<pred>)." For `eventually`, `until`, `leads`: "What eventually happens cannot be stated. Use always reachable for 'can always get back to'." |
| E113 | nesting > 32 | "Parentheses and not are nested more than 32 levels deep. Simplify the predicate." |
| E114 | > 4,096 bytes | "The predicate is too long (more than 4,096 bytes)." |
| E115 | built-in followed by text | "<kind> takes no predicate. Remove <text>, or start with never or always reachable." |
| E116 | `-` before the first term of a sum | "A sum cannot start with a minus sign (column <col>). Move <term> to the other side of the comparison." |
| E117 | a **final** coefficient or the final constant lies outside [−(2³¹−1), 2³¹−1] after exact accumulation (§3.2, D-62). Column: the last occurrence of that place in the text, or for the constant the last non-zero integer literal that is not a coefficient (P-19, P-22, P-23) | "After combining terms, <place or the constant> comes to <n>. A predicate can hold whole numbers from −2,147,483,647 to 2,147,483,647." |

E103 (upper case) stays retired: case is a resolution matter (E201), not a syntax one.

### 10.3 Resolution errors (E2xx), in the order of §4.3

| Code | Message |
|---|---|
| E201 | "There is no element with the ID <id>." + "Did you mean <suggestion>?" |
| E202 | "<Element> has no place <name>." + "Did you mean <suggestion>?" If `<name>` contains `-` and splitting at the last `-` gives an existing place or a number: "Did you mean <e>/<a> - <b>? Put spaces around a minus sign." |
| E203 | "<Element> / <name> is a transition. A predicate counts tokens in places." / "<Element> / <name> is an event. A predicate can name the states of <Element>." |
| E204 | "<Element> is an element. Name one of its places, for example <id>/<first place>." |
| E205 | "<relationship> is synchronous, so no messages wait in it." / "<relationship> binds no ports, so it has no messages in transit." |
| E206 | "<ref> is outside this property's scope: <Scope element>, <its own behavior \| the children in its view>." |
| E207 | "There is no relationship with the ID <id>. Places of an element are written element/place." |
| E208 | "<Element> / <place> is not a port. Here a predicate can name place ports and messages in transit. Make <place> a port, or check it in <Element>'s own behavior." It is the user-facing text of NS L9's detail line (ADR 0015). |

### 10.4 Template preconditions (E3xx, `engine::lower`; catalog §6 lines)

| Code | Template | Line |
|---|---|---|
| E301 | T3 | "T3 needs a place that overflows when full; <place> now <blocks \| drops>." |
| E302 | T4 | "Every place in the sum blocks or drops, so <N> can never be exceeded." |
| E303 | T4 never fills up | "Capacity <c> is smaller than the <g> tokens one step adds." / "No step adds tokens to <place>, so it cannot fill up." |
| E304 | T5 | default K = 0: "The pool starts with no tokens. Set K." / given `k` < 1: "K must be at least 1." |
| E305 | T5 | "Choose at least one held-by place." / "The pool cannot also be a held-by place." |
| E306 | T7 done | "This scope has no 'done when' yet." |
| E307 | T7 initial | "This scope has more than 4,096 places. Name the target places instead." |
| E308 | T3, T4, T5, T7 | "<capacity \| K \| N \| value> is <n>, more than a predicate can hold (2,147,483,647)." |

### 10.5 File errors (E4xx)

| Code | Message |
|---|---|
| E401 | "<kind> needs a predicate." |
| E402 | "<kind> takes no predicate. Remove the predicate line." |
| E403 | "Say whether this property is about <Element>'s own behavior (behavior = \"own\") or its children (behavior = \"combined\")." |
| E404, E405 | retired (key state `nothingToCheck`, §10.1) |
| E406 | "Unknown kind <kind>. Use deadlock_free, no_capacity_exceeded, no_dead_transitions, never or always_reachable." |
| E407 | "Unknown key <key> in property <id>. It is kept in the file but the property is not checked." For `final`: "A property cannot set its own done when. Use the element's done when." |
| E408 | "Two properties have the ID <id>." |
| E409 | "<Element> has more than 256 properties." / "The project has more than 4,096 properties." |
| E410 | "A property has either kind or template, not both." / "…needs a kind or a template." |
| E411 | "Unknown template <t> version <v>." |
| E412 | "Template <t> needs <parameter>." / "<parameter> must be <type>." / "<place> is listed twice." / "Template get-back-to takes either target or preset, not both." Checked against the §8.2 parameter schema from slice 0; `<place>` is the stored text, compared verbatim |

### 10.6 Warnings (W5xx)

| Code | When | Message |
|---|---|---|
| W501 | an atom whose truth is fixed: (a) no terms; (b) all coefficients ≥ 0 and (`ge c` with c ≤ 0, `gt c` with c < 0, `ne c` with c < 0 → always true; `le c` with c < 0, `lt c` with c ≤ 0, `eq c` with c < 0 → never true); (c) the mirrored case, all coefficients ≤ 0 and (`le c` with c ≥ 0, `lt c` with c > 0, `ne c` with c > 0 → always true; `ge c` with c > 0, `gt c` with c ≥ 0, `eq c` with c > 0 → never true). P-14 is (b), P-17 is (c), P-15 and P-16 are (a) (D-63) | "<comparison> is always true, because token counts are never negative." / "…is never true…" / "<comparison> names no place, so it is always <true \| false>." |
| W502 | same canonical property line, scope element and behavior as another property; shown on the property whose `id` sorts later, naming the earlier one (V-48) | "This property already exists: <headline>." |
| W503 | a hand-written atom with all coefficients > 0 on places that all block or drop, with max = Σ aᵢ·cap(pᵢ): `ge c` with c > max, `gt c` with c ≥ max, `eq c` with c > max (never true); `le c` with c ≥ max, `lt c` with c > max, `ne c` with c > max (always true) | "<places> never hold more than <max> together (they block or drop when full), so <comparison> is never true." / "…always true." |

Templates do not warn in these cases: the precondition stops them (E302). Hand-written predicates only warn, because their headline is the predicate itself, so the vacuity is visible (D-20, ADR 0019).

---

## 11. Not expressible in v1 (ADR 0002, ADR 0003, ADR 0019, north-star non-goals)

| Question | Why not | Instead |
|---|---|---|
| "eventually", "leads to", "until", fairness, LTL/CTL beyond AG and AG EF | liveness (ADR 0003, research §4) | T7 "can always get back to", stated with its "does not mean" line. Later export to TLA+, Quint or P |
| "always p" as a keyword | not an ADR 0003 kind | `never not (p)` (E112 says so) |
| "can reach p" (EF p) | not a v1 kind (D-40, ADR 0019): it would change Proved, Disproved, Evidence and Replay | a Disproved `never p` shows a way to p; the vacuity guard and exploration counts (D-20). An `expected: reachable` flag may follow after v1 |
| `enabled(t)`, "t never fires", "t can always fire again" | predicates count tokens; an `enabled` atom needs a new atom kind, its encoding and evaluation in both explorers (D-45) | `no dead transitions` |
| `capacity(p)` or `initial(p)` inside a hand-written predicate | removed in revision 2 (one grammar, NS §5.3) | the number itself, or a template, whose values follow edits |
| products, division, min, max, modulo | not linear (ADR 0003) | — |
| two kinds in one property (`never a and always reachable b`) | one property, one verdict | two properties |
| "no capacity exceeded" for one place | the built-in covers the scope | T3 "never overflows" (D-02) |
| a done when per property | one final predicate per behavior (D-41) | the element's `[final]` |
| quantifiers ("for every worker"), parameters ("for any K") | data or parametric proofs | one place per instance. P-invariant certificates later |
| time, rates, timeouts as durations, load, metastability | untimed nets (E5) | "slow = never responds" modeling. Out of scope |
| message order, request/reply matching, token data, identity (per-term leaders) | bag places (ADR 0002) | TLA+, Quint, P. Counting variants are not offered under those names (research §3.3) |
| places of grandchildren or other scopes | one view per composition (ADR 0002) | a property in the child's view |
| internal places of a child in a combined scope | ADR 0015 (L9) | make the place a port (one attribute), or check it in the child's own scope |
| display names in text | names change, IDs do not (FMT-3) | IDs. The UI renders names |

The vacuity guard (research F4 i) ships in slice 4 as a product feature: "no dead transitions" runs next to user properties, and also next to deadlock-free whenever the scope has a final predicate for that behavior, and exploration counts are shown (D-20, D-61, ADR 0019, ADR 0023). It needs nothing from the language; V-46 shows what it catches next to V-42 (deadlock-free with done when) and V-44. Before slice 4 no guard runs, because "no dead transitions" is not checked (S-03); T5 in slices 1–3 shows its exploration count only.

---

## 12. Placement and tests

- `model::property` [S1]: lexer, explicit-stack parser, `Pred` construction with atom normalization, and the E1xx codes. `model::validate`: resolution, typing, file rules and warnings. `engine::lower(m, scope: Scope)` (design §3, D-32) builds `flat`; `engine::lower_property` (D-64) re-resolves, copies F into `DeadlockFree { final }`, expands templates, checks preconditions and returns `(NetProperty, Expansion)` or the diagnostics. `net::encode::canonical_property` is the only producer of canonical lines (ADR 0009). `verify` and `recheck` each implement NS §5.2 evaluation from the text, from slice 1, used only by the final predicate and by T5's `Never` until slice 4; NS §8.3 ships in both explorers in slice 1 for T5 (§8.3, D-60). The UI never parses: it sends text and renders engine diagnostics, the Checks line and the headline.
- **Golden tests:** every vector in §14, at the layer and from the slice it names. The NS §14.5 predicate vectors run against the same parser.
- **Round trip (proptest):** for a random `Pred` the parser can produce, `parse(render(P)) == P`. For every template vector, the parse of its Checks line equals its expansion (catalog's "edit as predicate").
- **Lowering preserves meaning (proptest):** a test-only evaluator over the raw parse tree (before atom normalization) agrees with `recheck`'s evaluator on the lowered `NetProperty`, for random predicates and markings. This guards the shared, unchecked lowering path.
- **Evaluator agreement (proptest):** `verify` and `recheck` agree on random predicates and markings, including overflow markings, atoms with no terms and coefficients and constants of ±(2³¹−1).
- **Normalization (proptest, D-62):** for random texts, the i64 accumulation agrees with an arbitrary-precision reference, E117 is reported exactly when a final value lies outside [−(2³¹−1), 2³¹−1], and never for an intermediate value.
- **Fuzzing:** the parser never panics, runs in linear time, and respects §3.2 limits.
- **CheckKey fitness (ADR 0013):** changing any coefficient, constant, cmp, ID, connective or tree shape changes the key; changing `[final]` changes only the keys of deadlock-free and T7 "done" of that behavior. Changing `id`, `label`, `holder`, a port flag or spelling variants that give the same `Pred` does not.
- **Differential corpus:** `interop` writes `never p` as `AG ¬p` and `always reachable p` as `AG EF p`. Only nets whose places all block enter the corpus for user properties. They are encoded with complement places (D-35, NS §18), so O = ∅ and TAPAAL never explores past an overflow marking. Nets with drop or overflow places are excluded.

---

## 13. Fixtures

NS shared nets are used unchanged (NS §14): **SRC(k)**, **PC(k)**, **LOCKS(order)**, **POOL**, **POOL-LEAK**, **BRANCH**, **MASK**. For combined-scope use, their scope element is `sys` with the parts as children, and **every place a vector names is a place port** (§4.4). BRANCH and SRC are own-behavior nets of element `x`.

- **shop** (combined, scope `sys`, children `api` and `worker`):
  - `api/ready(3, block, 3)` and `api/waiting(3, block)` are place ports. Transition ports: `api/submit: api/ready → api/waiting`, `api/ack: api/waiting → api/ready`.
  - `worker/idle(1, block, 1)` is **internal**. `worker/busy(1, block)` is a place port. Transition ports: `worker/take: worker/idle → worker/busy`, `worker/finish: worker/busy → worker/idle`.
  - `api-jobs` is asynchronous (`api/submit` → `worker/take`, capacity 2). `worker-replies` is synchronous (caller `worker/finish`, callee `api/ack`).
  - Flattened: `api/submit: api/ready → api/waiting, api-jobs`; `worker/take: api-jobs, worker/idle → worker/busy`; `worker-replies/finish+ack: api/waiting, worker/busy → api/ready, worker/idle`. Canonical transition order: `api/submit`, `worker-replies/finish+ack`, `worker/take`.
  - **N1**: `api-jobs` overflow (default): |R| = 6, |O| = 1. **N2**: `api-jobs` block: |R| = 6, |O| = 0.
- **gate** (own): `gate/closed(1, block, 1)`, `gate/open(1, block)`, `gate/count(2, drop)`; `gate/arrive: ∅ → gate/count`; `gate/admit: gate/open, gate/count → gate/open`; `gate/unlock: gate/open → gate/closed`. |R| = 3.
- **job** (own): `job/start(1, block, 1)`, `job/done(1, block)`; `job/run: job/start → job/done`. |R| = 2. **job-F** adds `[final] own = "job/done = 1"`.
- **y** (own): `y/p(4, block)`; `y/t: ∅ → 2·y/p`; `y/u: y/p → ∅`. |R| = 5.
- **breaker** (own, state machine, S2): states `closed` (initial), `open`, `half_open`, each compiled to a place `(1, overflow)` (D-14; SM LC-05); internal events `trip` closed→open, `probe` open→half_open, `reset` half_open→closed, `fail` half_open→open. |R| = 3, |O| = 0.
- **ebs-bad** and **ebs-timeout**: E5 `case1_ebs_bad` and `case1_ebs_timeout`, mapped as in NS §13.1 (D-07). All places overflow. Four relationships, each carrying a call and its reply: `r1` api_a → control_plane (`call + accept_a`, `return_a + reply`), `r2` api_b → control_plane (`call + accept_b`, `return_b + reply`), `r3` control_plane → cluster_a (`fwd_a + create`, `respond + done_a`), `r4` control_plane → cluster_b (`fwd_b + create`, `respond + done_b`). ebs-timeout adds a third pair on `r1`, `timeout_a + fail` (caller in control_plane). The transitions `ret_a` and `ret_b` are renamed `return_a` and `return_b` (NS §2.2). Place ports: `api_b/done`, `control_plane/in_a`, `control_plane/wait_a`, `control_plane/ret_a` and `control_plane/threads_free`. 17 and 18 places; |R| = 84 and 216, |O| = 0.

---

## 14. Test vectors

"len" is the counterexample length under NS §12. "stored" is the number of stored markings at discovery, the violating one included. For always reachable and no dead transitions it is |R ∪ O|, because they decide after a complete exploration (NS §14 convention). #R and #O come from a full exploration. Canonical lines in §14.3 and §14.4 omit the leading `property `.

### 14.1 Parse vectors (`model::property`, no net) [S1]

| # | Text | Result: canonical line (NS §11) or error | Checks line (predicate part) |
|---|---|---|---|
| P-01 | `deadlock-free` | `property deadlock-free final none` (scope without `[final]`) | — |
| P-02 | `no capacity exceeded` | `property capacity-never-exceeded` | — |
| P-03 | `no dead transitions` | `property no-dead-transitions` | — |
| P-04 | `never p/in-flight - r >= 1` | `property never (atom ge 1 p/in-flight 1 r -1)` | `p/in-flight >= r + 1` |
| P-05 | `never x/p -1 >= 0` | `property never (atom ge 1 x/p 1)` | `x/p >= 1` |
| P-06 | `never x/p- 1 >= 0` | E111 at column 12, with the minus-sign hint (`x/p-` is one ID) | — |
| P-07 | `never 2*w/free - c/running <= 4` | `property never (atom le 4 c/running -1 w/free 2)` | `2*w/free <= c/running + 4` |
| P-08 | `never a/x == 1` | `property never (atom eq 1 a/x 1)` (same key as `a/x = 1`) | `a/x = 1` |
| P-09 | `never billing/inflight + shipping/inflight > 2` | `property never (atom gt 2 billing/inflight 1 shipping/inflight 1)` (slice 4 acceptance text, D-01, D-12) | same as text |
| P-10 | `never not not a/x >= 1` | `property never (not (not (atom ge 1 a/x 1)))` | `not not a/x >= 1` |
| P-11 | `never a/x >= 1 and b/y >= 1 and c/z >= 1` | `property never (and (atom ge 1 a/x 1) (atom ge 1 b/y 1) (atom ge 1 c/z 1))` | same as text |
| P-12 | `never (a/x >= 1 and b/y >= 1) and c/z >= 1` | `property never (and (and (atom ge 1 a/x 1) (atom ge 1 b/y 1)) (atom ge 1 c/z 1))` (key differs from P-11) | same as text |
| P-13 | `never a/x >= 1 or b/y >= 1 and c/z >= 1` | `property never (or (atom ge 1 a/x 1) (and (atom ge 1 b/y 1) (atom ge 1 c/z 1)))` | `a/x >= 1 or (b/y >= 1 and c/z >= 1)` |
| P-14 | `never a/x + 3 >= 0` | `property never (atom ge -3 a/x 1)`, W501 (always true) | `a/x + 3 >= 0` |
| P-15 | `never 3 = 3` | `property never (atom eq 0)`, W501 | `0 = 0` |
| P-16 | `never a/x - a/x >= 0` | `property never (atom ge 0)`, W501 | `0 >= 0` |
| P-17 | `never 0 >= a/x + 3` | `property never (atom ge 3 a/x -1)`, W501 (never true; mirrored case (c), all coefficients ≤ 0) | `0 >= a/x + 3` |
| P-18 | `always reachable a/x = 1` | `property always-reachable (atom eq 1 a/x 1)` | `a/x = 1` |
| P-19 | `never 2147483647*a/x + 2147483647*a/x >= 0` | E117 at column 35 | — |
| P-20 | `never -1 * a/x >= 0` | E116 at column 7 | — |
| P-21 | `never and >= 1` | E111 at column 7 (keyword hint) | — |
| P-22 | `never 0 >= 2147483647*a/x + a/x` | E117 at column 29: "After combining terms, a/x comes to −2,147,483,648. …" (the final coefficient fits i32 but not the symmetric range, D-62) | — |
| P-23 | `never a/x + 2147483647 + 1 >= 0` | E117 at column 26: "After combining terms, the constant comes to −2,147,483,648. …" | — |
| P-24 | `never 2147483647*a/x + a/x - a/x >= 0` | `property never (atom ge 0 a/x 2147483647)`, W501 (always true). The intermediate 2,147,483,648 is not an error: only final values are checked (D-62) | `2147483647*a/x >= 0` |
| P-25 | `never 3 >= 0` | `property never (atom ge -3)`, W501 (names no place) | `3 >= 0` (left side without terms: `\|c\|` alone, §5) |
| P-26 | `never 0 >= a/x - 3` | `property never (atom ge -3 a/x -1)`, no warning | `3 >= a/x` |
| P-27 | `never 0 <= a/x + 1` | `property never (atom le 1 a/x -1)`, W501 (always true; mirrored case (c)) | `0 <= a/x + 1` |

### 14.2 Syntax, resolution and file errors (context: shop, scope `sys`, behavior combined, unless stated) [E1xx, E2xx: S1 for `[final]` and `pool-share` parameters, S4 for `predicate` and the other templates' parameters; E114 and E4xx S0]

Every error row ends as Unknown `PropertyNotCheckable` with that line (§10.1).

| # | Input | Result |
|---|---|---|
| E-01 | `never api/ready > 0 && worker/busy = 1` | E102 at column 21: "Use and instead of && (column 21)." |
| E-02 | `never api/ready => 1` | E102 at column 17: "Use >= instead of =>…" |
| E-03 | `never api/ready ≥ 1` | E102 at column 17: "Use >= instead of ≥…" |
| E-04 | `never (api/ready + api/waiting) > 2` | E107 at column 7: "…Remove them: api/ready + api/waiting > 2." |
| E-05 | `never api/ready * worker/busy > 0` | E108 at column 17 |
| E-06 | `never api/ready * 2 > 0` | E108 at column 17: "…for example 2 * Api / ready." |
| E-07 | `never api/ready` | E104 at column 16 |
| E-08 | `never 1 <= api/ready <= 2` | E105 at column 22: "…Write 1 <= api/ready and api/ready <= 2." |
| E-09 | `never api/ready > 0 and` | E110 at column 24 |
| E-10 | `never (api/ready > 0` | E106: "The parenthesis at column 7 is not closed." |
| E-11 | `never api/ready > 0)` | E106: "The closing parenthesis at column 20 has no opening one." |
| E-12 | `always api/ready + api/waiting = 3` | E112: "To say a predicate always holds, write never not (api/ready + api/waiting = 3)." |
| E-13 | `eventually api/ready = 3` | E112 (liveness message) |
| E-14 | `deadlock-free api/ready > 0` | E115 at column 15 |
| E-15 | `never -api/ready > -1` | E116 at column 7 |
| E-16 | `never api/ready > 2147483648` | E109 at column 19 |
| E-17 | `never api/submit > 0` | E203 at column 7 (transition) |
| E-18 | `never worker-replies > 0` | E205 at column 7 (synchronous) |
| E-19 | `never api > 0` | E204 at column 7 |
| E-20 | `never api/redy > 0` | E202 at column 7: "Api has no place redy. Did you mean ready?" |
| E-21 | `never api/ready-1 > 0` | E202 at column 7: "…Did you mean api/ready - 1? Put spaces around a minus sign." |
| E-22 | `never billing/inflight > 0` | E201 at column 7 |
| E-23 | `never jobs > 0` | E207 at column 7 |
| E-24 | `never worker/idle = 0` | E208 at column 7 (`worker/idle` is internal; NS L9) |
| E-25 | scope `api`, behavior own: `never api-jobs > 0` | E206 (rule 2 of the bare-ID path: an own-behavior property never names a generated place) |
| E-26 | scope `api`, behavior own: `never worker/busy > 0` | E206 (`worker` exists in the project but is not the scope element; rule 2 comes before rule 4) |
| E-27 | `never api/ready + worker/busy > api-jobs` | valid: `property never (atom gt 0 api-jobs -1 api/ready 1 worker/busy 1)`. Checks line `api/ready + worker/busy > api-jobs` |
| E-28 | TOML `kind = "never"` without `predicate` | E401 |
| E-29 | TOML `kind = "deadlock_free"`, `predicate = "api/ready > 0"` | E402 |
| E-30 | TOML entry without `behavior` | E403 |
| E-31 | TOML entry with `final = "api/ready = 3"` | E407 (reserved key, D-41). The key is kept on save |
| E-32 | `never api/waiting >= 3` saved. Then place `api/waiting` is deleted and a new place "waiting" is added to `api` | the new place gets ID `waiting-2` (§4.5). The property stays E202 "Api has no place waiting." and never names the new place |
| E-33 | `never api/ready > 0 # 1` | E101 at column 21: "Unexpected character “#” at column 21. …" |
| E-34 | `never café > 0` | E101 at column 10 (`é` is not an ASCII letter, §3.2) |
| E-35 | `never ` followed by 33 × `not ` and `api/ready > 0` | E113 at column 135 (the 33rd `not`) |
| E-36 | TOML `kind = "eventually"` | E406: "Unknown kind eventually. Use deadlock_free, …" [S0] |
| E-37 | TOML: `id = "load"` in `elements/sys.toml` and in `elements/api.toml` | E408 "Two properties have the ID load." on both entries; neither is checked [S0] |
| E-38 | TOML: 257 `[[property]]` entries in `elements/sys.toml` | E409 "Sys has more than 256 properties." on every property of `sys`; none is dropped or checked [S0] |
| E-39 | TOML: `kind = "never"` and `template = "pool-share"` in one entry; a second entry with neither | E410 "A property has either kind or template, not both." / "…needs a kind or a template." [S0] |
| E-40 | TOML: `template = "pool-share"`, `template_version = 2` | E411 "Unknown template pool-share version 2." [S0: needs the §8.2 schema, §10.1] |
| E-41 | TOML: `pool-share` v1 without `pool`; a second entry with `held_by = "api/waiting"` (a string) | E412 "Template pool-share needs pool." / "held_by must be a list of place IDs." [S0] |
| E-42 | TOML: `pool-share` v1, `pool = "api/ready"`, `held_by = ["api/waiting", "api/waiting"]` | E412 "api/waiting is listed twice." (compared verbatim) [S0] |
| E-43 | `pool-share` v1, `pool = "api/ready"`, `held_by = ["worker/idle"]` | E208 for parameter `held_by`: "Worker / idle is not a port. …" (`worker/idle` is internal; NS L9) [S1, D-60] |

### 14.3 Verdict vectors (NS revision 3)

| # | Net | Property | Canonical line | Expected verdict | len / detail | stored | #R | #O | Slice |
|---|---|---|---|---|---|---|---|---|---|
| V-01 | PC(block) | `never r >= 3` | `never (atom ge 3 r 1)` | Disproved | 3 (p/produce ×3) = NS V32 | 6 | 18 | 0 | S4 |
| V-02 | PC(block) | `never r > 3` | `never (atom gt 3 r 1)` | Proved, 18 markings | key differs from NS V33 `r >= 4` (D-10) | — | 18 | 0 | S4 |
| V-03 | PC(block) | `never p/items + r + c/got != 5` | `never (atom ne 5 c/got 1 p/items 1 r 1)` | Proved, 18 | conservation | — | 18 | 0 | S4 |
| V-04 | PC(block) | `never not (p/items + r + c/got = 5)` | `never (not (atom eq 5 c/got 1 p/items 1 r 1))` | Proved, 18 | key differs from V-03 | — | 18 | 0 | S4 |
| V-05 | PC(block) | `always reachable c/got = 5` | `always-reachable (atom eq 5 c/got 1)` | Proved, 18 | — | — | 18 | 0 | S4 |
| V-06 | PC(block) | `always reachable p/items = 5` | `always-reachable (atom eq 5 p/items 1)` | Disproved (CounterexampleThenExhaustive) | 1 (p/produce); markings 17 | 18 | 18 | 0 | S4 |
| V-07 | PC(block) | `never 3 = 3` | `never (atom eq 0)` | Disproved, W501 | 0 (M₀) | 1 | 18 | 0 | S4 |
| V-08 | PC(overflow) | `never r >= 3` | `never (atom ge 3 r 1)` | Disproved | 3, in-bounds marking | 6 | 18 | 2 | S4 |
| V-09 | PC(overflow) | `never r >= 4` | `never (atom ge 4 r 1)` | Disproved | 4 (p/produce ×4), ending in the overflow marking `p/items = 1, r = 4`; panel adds the capacity line (r, 4 of 3) | 9 | 18 | 2 | S4 |
| V-10 | PC(overflow) | `always reachable c/got = 5` | `always-reachable (atom eq 5 c/got 1)` | Unknown CapacityExceededFirst { r, 4 steps } | — | — | 18 | 2 | S4 |
| V-11 | POOL | `never c/idle < c/running` | `never (atom lt 0 c/idle 1 c/running -1)` | Disproved | 2 (r1/start+acquire ×2): idle 1, running 2 | 3 | 3 | 0 | S4 |
| V-12 | POOL | `never c/running + w/free != 2` | `never (atom ne 2 c/running 1 w/free 1)` | Proved, 3 | — | — | 3 | 0 | S4 |
| V-13 | POOL-LEAK | `never w/free = 0 and c/running = 0` | `never (and (atom eq 0 w/free 1) (atom eq 0 c/running 1))` (written order kept) | Disproved | 4 (start, fail, start, fail) | 6 | 6 | 0 | S4 |
| V-14 | POOL-LEAK | `no dead transitions` | `no-dead-transitions` | Proved, 6 | — | — | 6 | 0 | S4 |
| V-15 | LOCKS(yx) | `never a/has_1 + b/has_1 = 2` | `never (atom eq 2 a/has_1 1 b/has_1 1)` | Disproved | 2 (ra1, rb1) | 5 | 6 | 0 | S4 |
| V-16 | MASK | `always reachable a/idle = 1 and b/idle = 1` | `always-reachable (and (atom eq 1 a/idle 1) (atom eq 1 b/idle 1))` | Disproved | 2 (ra1, rb1); markings 1 | 6 | 6 | 0 | S4 |
| V-17 | BRANCH | `never x/d = 1` | `never (atom eq 1 x/d 1)` | Disproved | 1 (x/t1) | 2 | 3 | 1 | S4 |
| V-18 | BRANCH | `never x/c = 0 and x/d = 0` | `never (and (atom eq 0 x/c 1) (atom eq 0 x/d 1))` | Disproved | 1 (x/t2), marking `x/p = 1` | 3 | 3 | 1 | S4 |
| V-19 | BRANCH | `never x/p >= 2` | `never (atom ge 2 x/p 1)` | Disproved | 2 (x/t2, x/t3), ending in the overflow marking `x/p = 2` | 4 | 3 | 1 | S4 |
| V-20 | SRC(drop) | `never x/p >= 2` | `never (atom ge 2 x/p 1)` | Disproved | 2 | 3 | 3 | 0 | S4 |
| V-21 | SRC(block) | `always reachable x/p = 0` | `always-reachable (atom eq 0 x/p 1)` | Disproved | 1; markings 2 | 3 | 3 | 0 | S4 |
| V-22 | SRC(overflow) | `always reachable x/p = 0` | `always-reachable (atom eq 0 x/p 1)` | Unknown CapacityExceededFirst { x/p, 3 steps } | Bad = ∅, O ≠ ∅ | — | 3 | 1 | S4 |
| V-23 | shop N1 | `never api/ready = 0` | `never (atom eq 0 api/ready 1)` | Disproved | 3 (api/submit ×3), ending in the overflow marking `api-jobs = 3, api/waiting = 3, worker/idle = 1`; panel adds "this run also exceeds the capacity of api-jobs (3 of 2)" | 5 | 6 | 1 | S4 |
| V-24 | shop N1 | `never api-jobs >= 3` | `never (atom ge 3 api-jobs 1)` | Disproved | 3 (api/submit ×3), same overflow marking as V-23 | 5 | 6 | 1 | S4 |
| V-25 | shop N1 | `deadlock-free` | `deadlock-free final none` | Unknown CapacityExceededFirst { api-jobs, 3 steps } | — | — | 6 | 1 | S0 |
| V-26 | shop N1 | `no capacity exceeded` | `capacity-never-exceeded` | Disproved | 3 (api/submit ×3); Overflow { api-jobs, 2, 3 } | 5 | 6 | 1 | S0 |
| V-27 | shop N2 | `never api-jobs + worker/busy >= 3` | `never (atom ge 3 api-jobs 1 worker/busy 1)` | Disproved | 4 (submit, submit, take, submit) | 6 | 6 | 0 | S4 |
| V-28 | shop N2 | `never api/ready + api/waiting != 3` | `never (atom ne 3 api/ready 1 api/waiting 1)` | Proved, 6 | — | — | 6 | 0 | S4 |
| V-29 | shop N2 | `always reachable api/ready = 3` | `always-reachable (atom eq 3 api/ready 1)` | Proved, 6 | — | — | 6 | 0 | S4 |
| V-30 | shop N2 | `deadlock-free` | `deadlock-free final none` | Proved, 6 | — | — | 6 | 0 | S0 |
| V-31 | shop N2 | `no capacity exceeded` | `capacity-never-exceeded` | Proved, certificate `NoOverflowPlace { drop_places: [] }` | markings 0; accepted by `recheck::check_certificate` | — | 6 | 0 | S0 |
| V-32 | shop N2 | `never worker/idle = 0` | `never (atom eq 0 worker/idle 1)` | Unknown PropertyNotCheckable [E208 line] | with `worker/idle` declared a place port (key unchanged: port flags are not encoded): Disproved, 2 (api/submit, worker/take), stored 4 | — | 6 | 0 | S4 |
| V-33 | gate | `no dead transitions` | `no-dead-transitions` | Disproved (Exhaustive) | dead = [gate/admit, gate/unlock]; markings 3 | 3 | 3 | 0 | S4 |
| V-34 | gate | `never gate/count >= 3` | `never (atom ge 3 gate/count 1)` | Proved, 3; W503 | — | — | 3 | 0 | S4 |
| V-35 | gate | `deadlock-free` | `deadlock-free final none` | Proved, 3 | `arrive` always enabled (drop) | — | 3 | 0 | S0 |
| V-36 | job | `deadlock-free` | `deadlock-free final none` | Disproved | 1 (job/run): "no transition is enabled and no final predicate is defined" (no pre place holds a token, NS §9.6) | 2 | 2 | 0 | S0 |
| V-37 | job-F | `deadlock-free` | `deadlock-free final (atom eq 1 job/done 1)` | Proved, 2 | — | — | 2 | 0 | S1 |
| V-38 | breaker | `always reachable breaker/closed = 1` | `always-reachable (atom eq 1 breaker/closed 1)` | Proved, 3 | — | — | 3 | 0 | S2+S4 |
| V-39 | breaker | `never breaker/closed + breaker/open + breaker/half_open != 1` | `never (atom ne 1 breaker/closed 1 breaker/half_open 1 breaker/open 1)` | Proved, 3 | — | — | 3 | 0 | S2+S4 |
| V-40 | ebs-bad | P4 `never control_plane/in_a + control_plane/wait_a + control_plane/ret_a >= 3` | `never (atom ge 3 control_plane/in_a 1 control_plane/ret_a 1 control_plane/wait_a 1)` | Disproved | 3 (r1/call+accept_a ×3) | 9 | 84 | 0 | S4 |
| V-41 | ebs-timeout | P4 (as V-40) | same line | Disproved | 3 (r1/call+accept_a ×3) | 9 | 216 | 0 | S4 |
| V-42 | ebs-timeout, `[final] combined = "api_b/done = 2"` | `deadlock-free` (T1) | `deadlock-free final (atom eq 2 api_b/done 1)` | Proved, 216 | the "does not mean" line of T1 cannot be collapsed | — | 216 | 0 | S1 |
| V-43 | ebs-timeout, no final | `deadlock-free` | `deadlock-free final none` | Disproved | 17: r1/call+accept_a ×3, r3/fwd_a+create, r1/timeout_a+fail, r2/call+accept_b, r3/fwd_a+create, r1/timeout_a+fail, r2/call+accept_b, r3/fwd_a+create, r1/timeout_a+fail, r4/fwd_b+create ×2, r4/respond+done_b, r2/return_b+reply, r4/respond+done_b, r2/return_b+reply; ends at `api_a/failed = 3, api_b/done = 2, cluster_a/working = 3, cluster_b/healthy = 1, control_plane/threads_free = 3`; "no transition is enabled" (pre places hold tokens) | 216 | 216 | 0 | S0 |
| V-44 | ebs-timeout | `always reachable api_b/done = 2` | `always-reachable (atom eq 2 api_b/done 1)` | Proved, 216 | — | — | 216 | 0 | S4 |
| V-45 | breaker | `no capacity exceeded` | `capacity-never-exceeded` | Proved, exhaustive, 3 | no certificate: state places overflow (D-14) | — | 3 | 0 | S2 |
| V-46 | ebs-timeout | `no dead transitions` (vacuity guard, D-20) | `no-dead-transitions` | Disproved (Exhaustive) | dead = [r1/return_a+reply, r3/respond+done_a]; markings 216. Next to V-42 and V-44 it shows that no AZ-A call is ever answered. It runs next to V-42 also when no user property exists, because the scope has a final predicate (D-61) | 216 | 216 | 0 | S4 |
| V-47 | shop N2 | `never api/ready + api/waiting >= 7` | `never (atom ge 7 api/ready 1 api/waiting 1)` | Proved, 6; W503 on a sum: "Api / ready and Api / waiting never hold more than 6 together (they block or drop when full), so … is never true." | max = 3 + 3 = 6 < 7 | — | 6 | 0 | S4 |
| V-48 | shop N1 | T3 `never-overflows: api-jobs` (id `a-overflow`) and custom `never api-jobs > 2` (id `b-custom`) | both `never (atom gt 2 api-jobs 1)` | Disproved as T-16, one check for the shared key; W502 on `b-custom` only, naming `a-overflow`'s headline | 3 | 5 | 6 | 1 | S4 |

### 14.4 Template vectors (`engine::lower_property` golden tests, then verdicts) [S4; T5 vectors T-01–T-03, T-17, T-18, T-20–T-22, T-24 from S1 (D-60); T-25 is an E4xx file rule, S0]

| # | Net | Template and parameters | Expansion (canonical line) | Checks line | Expected verdict |
|---|---|---|---|---|---|
| T-01 | ebs-timeout | `pool-share`: pool `control_plane/threads_free`, held_by in_a, wait_a, ret_a, default K | K = 3 + 0 = 3: `never (atom ge 3 control_plane/in_a 1 control_plane/ret_a 1 control_plane/wait_a 1)` (same key as V-41) | `control_plane/in_a + control_plane/ret_a + control_plane/wait_a >= 3` | Disproved, 3 steps, stored 9 (catalog acceptance criterion) |
| T-02 | shop N2 | `pool-share`: pool `api/ready`, held_by `api/waiting` | K = 3: `never (atom ge 3 api/waiting 1)` | `api/waiting >= 3` | Disproved, 4 (submit, submit, take, submit), stored 6 |
| T-03 | any | `pool-share`: pool with M₀ = 2, one held-by place with M₀ = 1 | K = 3 | — | expansion only |
| T-04 | shop N2 | `never-more-than`: terms api-jobs, worker/busy; at_most 2 | `never (atom gt 2 api-jobs 1 worker/busy 1)` (key differs from V-27) | `api-jobs + worker/busy > 2` | Disproved, 4, ending with sum 3, stored 6 |
| T-05 | shop N2 | `never-fills-up`: api-jobs | g = 1, N = 1: `never (atom gt 1 api-jobs 1)` | `api-jobs > 1` | Disproved, 2 (api/submit ×2), stored 3 |
| T-06 | y | `never-fills-up`: y/p | g = 2, N = 2: `never (atom gt 2 y/p 1)` | `y/p > 2` | Disproved, 2 (y/t ×2) at `y/p = 4`, stored 3; #R 5. Headline names N = 2 (§17 OQ10) |
| T-07 | shop N2 | `never-more-than`: terms api/ready; at_most 3 | — | — | Unknown PropertyNotCheckable: E302 "Every place in the sum blocks or drops, so 3 can never be exceeded." |
| T-08 | PC(overflow) | `never-overflows`: r | `never (atom gt 3 r 1)` | `r > 3` | Disproved, 4 (p/produce ×4) at the overflow marking `r = 4`, stored 9 (D-02) |
| T-09 | PC(block) | `never-overflows`: r | — | — | Unknown PropertyNotCheckable: E301 "T3 needs a place that overflows when full; messages in transit on p → c now block." |
| T-10 | shop N2 | `get-back-to`, preset initial | `always-reachable (and (atom eq 0 api-jobs 1) (atom eq 3 api/ready 1) (atom eq 0 api/waiting 1) (atom eq 0 worker/busy 1) (atom eq 1 worker/idle 1))` | `api-jobs = 0 and api/ready = 3 and api/waiting = 0 and worker/busy = 0 and worker/idle = 1` | Proved, 6 (names the internal `worker/idle`, allowed for this preset by NS L9) |
| T-11 | gate | `get-back-to`, preset initial | `always-reachable (and (atom eq 1 gate/closed 1) (atom eq 0 gate/count 1) (atom eq 0 gate/open 1))` | `gate/closed = 1 and gate/count = 0 and gate/open = 0` | Disproved, 1 (gate/arrive); markings 2; stored 3 |
| T-12 | ebs-timeout with `[final] combined = "api_b/done = 2"` | `get-back-to`, preset done | `always-reachable (atom eq 2 api_b/done 1)` | `api_b/done = 2` | Proved, 216 (= V-44, same key). Editing `[final]` changes this key and V-42's, not V-44's or T-01's. Without `[final]`: Unknown PropertyNotCheckable, E306 |
| T-13 | POOL / POOL-LEAK | `get-back-to`: target w/free = 2 | `always-reachable (atom eq 2 w/free 1)` | `w/free = 2` | POOL: Proved, 3 (NS V36). POOL-LEAK: Disproved, 2 (r1/start+acquire, c/fail), markings 3, stored 6 (NS V37) |
| T-14 | gate | T8 | `no-dead-transitions` | — | as V-33. No replay is offered (D-33) |
| T-15 | every T vector with an expansion | round trip | parse(`never ` or `always reachable ` + Checks line) equals the expansion | — | — |
| T-16 | shop N1 | `never-overflows`: api-jobs (capacity 2) | `never (atom gt 2 api-jobs 1)` (key differs from V-24 `>= 3`, D-10; equals V-48's custom) | `api-jobs > 2` | Disproved, 3 (api/submit ×3) at the overflow marking `api-jobs = 3`, stored 5; the panel says the run exceeds the capacity (3 of 2). Catalog T3 criterion |
| T-17 | shop N2 | `pool-share`: pool `api/ready`, held_by `api/waiting`, `k = 2` | K = 2 (given; the default would be 3): `never (atom ge 2 api/waiting 1)`; `Expansion.k = 2` | `api/waiting >= 2` | Disproved, 2 (api/submit ×2), stored 3 |
| T-18 | shop N2 | `pool-share`: pool `api/ready`, held_by `api/waiting`, `k = 0` | — | — | Unknown PropertyNotCheckable: E304 "K must be at least 1." |
| T-19 | job | `never-fills-up`: job/start | — | — | Unknown PropertyNotCheckable: E303 "No step adds tokens to Job / start, so it cannot fill up." |
| T-20 | shop N2 | `pool-share`: pool `api/waiting`, held_by `api-jobs`, default K | K = 0 + 0 = 0 | — | Unknown PropertyNotCheckable: E304 "The pool starts with no tokens. Set K." |
| T-21 | shop N2 | `pool-share`: pool `api/ready`, `held_by = []` | — | — | Unknown PropertyNotCheckable: E305 "Choose at least one held-by place." |
| T-22 | shop N2 | `pool-share`: pool `api/ready`, held_by `api/ready`, `api/waiting` | — | — | Unknown PropertyNotCheckable: E305 "The pool cannot also be a held-by place." |
| T-23 | own `x/p(2, block)`, `x/t: ∅ → 3·x/p` (NS V41's net) | `never-fills-up`: x/p | g = 3 | — | Unknown PropertyNotCheckable: E303 "Capacity 2 is smaller than the 3 tokens one step adds." |
| T-24 | own `x/pool(2147483647, block, 2147483647)`, `x/held(1, block, 1)`, `x/take: x/pool → x/held` | `pool-share`: pool `x/pool`, held_by `x/held`, default K | K = 2,147,483,647 + 1 | — | Unknown PropertyNotCheckable: E308 "K is 2,147,483,648, more than a predicate can hold (2,147,483,647)." |
| T-25 | shop N2 | `get-back-to` with `preset = "initial"` and `target = [{ place = "api/ready", cmp = "=", value = 3 }]` | — | — | Unknown PropertyNotCheckable: E412 "Template get-back-to takes either target or preset, not both." |
| T-26 | shop N2 | `get-back-to`: target rows in this order: (worker/busy, `=`, 0), (api/ready, `<=`, 3), (api/ready, `>=`, 1), (api/ready, `=`, 3) | rows sorted by place ID, then eq < ge < le: `always-reachable (and (atom eq 3 api/ready 1) (atom ge 1 api/ready 1) (atom le 3 api/ready 1) (atom eq 0 worker/busy 1))` | `api/ready = 3 and api/ready >= 1 and api/ready <= 3 and worker/busy = 0` | Proved, 6 (as V-29: `api/ready = 3` forces `worker/busy = 0`). No W503: templates do not warn (§10.6) |
| T-27 | own element `big` with 4,097 places `(1, block)` and no transitions; then with 4,096 | `get-back-to`, preset initial | 4,096 places: one `eq` row per place, in ID order | — | 4,097: Unknown PropertyNotCheckable, E307. 4,096: Proved, 1 marking |

### 14.5 Slice vectors (D-25)

| # | Slice | Input | Result |
|---|---|---|---|
| S-01 | S0–S3 | PC(block), `[[property]] kind = "never"`, `predicate = "r >= 3"` | Unknown PropertyNotCheckable ["user properties are checked from slice 4"] (= NS V57); the predicate is stored verbatim and not parsed before S4 (D-49) |
| S-02 | S0–S3 | shop N2, `template = "never-fills-up"`, `place = "api-jobs"` | Unknown PropertyNotCheckable ["user properties are checked from slice 4"]. Every template except `pool-share` (S-06) |
| S-03 | S0–S3 | gate, `kind = "no_dead_transitions"` | Unknown PropertyNotCheckable ["user properties are checked from slice 4"] (§17 C1) |
| S-04 | S0 | job-F (`[final] own = "job/done = 1"`), `deadlock-free` | warning `finalNotChecked`; lowers `deadlock-free final none`; Disproved as V-36. From S1: Proved as V-37 |
| S-05 | S0, then S1 | job with `[final] own = "job/done = = 1"` | S0: stored verbatim, no E1xx (opaque), lowers `final none` with `finalNotChecked`. S1: E111 at column 12, and every deadlock-free property of job's own behavior is Unknown PropertyNotCheckable ["done when: Unexpected = at column 12."] |
| S-06 | S0, then S1 | shop N2, `template = "pool-share"`, `pool = "api/ready"`, `held_by = ["api/waiting"]` | S0: Unknown PropertyNotCheckable ["never holds the whole pool is checked from slice 1"]; parameters stored verbatim, not resolved. From S1: as T-02 (Disproved, 4, stored 6) (D-60) |
| S-07 | S0 | PC(block), `kind = "never"` without `predicate` | Unknown PropertyNotCheckable ["never needs a predicate.", "user properties are checked from slice 4"]: the coded E401 line first, the gate line last (§10.1) |

### 14.6 Vectors changed by the decisions (for reviewers of revision 2)

| Decision | Vectors |
|---|---|
| D-02 never p on overflow markings | V-09 (Unknown → Disproved, 4, stored 9), V-19 (→ Disproved, 2, stored 4), V-23 (4 steps in bounds, stored 7 → 3 steps at the overflow marking, stored 5), V-24 (→ Disproved, 3, stored 5), T-08 (→ Disproved, 4, stored 9). Unchanged: V-08, V-17, V-18, V-20 and every always reachable and no dead transitions vector |
| D-01 L9 kept | E-24 and V-32 stay E208; the "any place" outcome is withdrawn. T-10 is valid (preset exemption) |
| D-03 certificate | V-31: "structural" → certificate `NoOverflowPlace` |
| D-04 final in deadlock-free | every deadlock-free canonical line gains `final none` or `final <pred>`; changing `[final]` changes the V-37, V-42 and T-12 keys only |
| D-05, D-25 slices | V-37, V-42 → S1; S-01 to S-05 added |
| D-07 call and reply on one relationship | ebs fixtures re-mapped; V-40, V-41, V-43 fused IDs |
| D-08 seven reasons | every "Unknown (E…)" row is `PropertyNotCheckable` |
| D-14 state places overflow | breaker fixture; V-45 added |
| D-20 vacuity guard | V-46 added |
| D-60 T5 in slice 1 | T-01–T-03, T-17, T-18, T-20–T-22, T-24 and E-43 become S1; S-02 exempts `pool-share`; S-06 added |
| D-61 guard next to deadlock-free with a final predicate | V-46 note |
| D-62 exact accumulation, symmetric range | P-22, P-23, P-24 added; P-19 unchanged (column 35) |
| D-63 mirrored W501 | P-17 now matches the rule; P-27 added |
| D-64, D-65 renderer, columns, line order | P-25, P-26, E-33–E-35, S-07 added; E102 column rule pins E-01–E-03 |
| D-68 missing vectors | E-36–E-42, V-47, V-48, T-16, T-19, T-23, T-25–T-27 added |

---

## 15. Amendments proposed in revision 2: where each landed

| Revision 2 | Landed in | Decision |
|---|---|---|
| A1 tree shape, error codes | NS §5.3 | D-43 |
| A2 4,096 terms per atom | NS §5.2 | D-43 |
| A3 L9 with the lowering-computed preset exempt | NS §6.6 L9, ADR 0015 | D-01 |
| A4 never p on overflow markings | NS §3.2, §8.3, §9.3, §12, V34; ADR 0017 | D-02 |
| A5 `PropertyNotCheckable` | NS §10.2, design §3, ADR 0017 | D-08 |
| A6 final predicate in `DeadlockFree` | NS §3.3, §8.1, §11; ADR 0018 | D-04 |
| A7 `net::Pred`, `Scope`, commands, `VerdictDto.checks` | design §3 | D-32, D-37 |
| A8 catalog: T2 certificate, as-parsed key, one namespace, single CEF place, T5 K | `docs/product/property-templates.md` (same revision) | D-01, D-03, D-10, D-11, D-22 |
| A9 Disproved without replay for no dead transitions | GLOSSARY Disproved, UX_STANDARDS VRD-2 (GSS owner) | D-33 |
| A10 slug and no-retarget constraints | file-format §4.1, §4.3 | D-12 |
| A11 integer weights, negative allowed | ADR 0015 | D-43 |
| A12 flows §4.13: seven templates, place combobox per §4.3, T7 note | `docs/ux/flows.md` (UX owner) | D-46 |

The three ADR drafts of revision 2 became ADR 0015, ADR 0017 and ADR 0019.

---

## 16. Glossary

This spec uses the merged block of `consistency-review.md` as resolved by D-28–D-33; the GSS owner applies it to `GLOSSARY.md`. Terms this spec relies on:

- **Linear marking predicate** (short: predicate): integer weights, negative allowed.
- **Final predicate** (UI label "done when"): one per behavior of a scope element; checked as part of deadlock-free (D-28). "Final markings" as a list and "final condition" are withdrawn.
- **Own scope / combined scope**: stored as `behavior = "own" | "combined"`.
- **Property template**: expands to exactly one property; a **preset** is one fixed choice inside a template (D-33).
- **Closed-port note**: the own-scope line "every port of `<element>` can fire at any time"; not an assumption (D-30).
- **Disproved**: a counterexample or, for no dead transitions, an exhaustive exploration confirmed by the second explorer (D-33).

"Condition", "final condition" and "scope kind" from revision 1 stay withdrawn.

---

## 17. Closed questions and calls made in this revision

| # | Question (revision 2) | Decision |
|---|---|---|
| OQ1 | Place ports only, or any place of a child? | D-01: place ports and generated places (L9), lowering-computed preset exempt; ADR 0015 |
| OQ2 | `never p` on overflow markings? | D-02: evaluated; ADR 0017 |
| OQ3 | Hash as parsed or normalize equivalent spellings? | D-10: as parsed; ADR 0019 |
| OQ4 | Template storage | D-11: kind, version, parameters; one property per template; ADR 0019 |
| OQ5 | Structural Proved for no capacity exceeded? | D-03: certificate `NoOverflowPlace` checked by recheck; ADR 0017 |
| OQ6 | New Unknown reason for property problems? | D-08: `PropertyNotCheckable`; ADR 0017 |
| OQ7 | Final predicate in the net or in deadlock-free? | D-04: in `DeadlockFree { final }`; ADR 0018 |
| OQ8 | Slice of final predicates | D-05: slice 1, minimal form; ADR 0018 |
| OQ9 | Hand-written predicates decided by when-full | D-20: warning W503 |
| OQ10 | Wording of "never fills up" when g > 1 | Decided here (catalog T4): for g > 1 the sentence states N, "never holds more than N (capacity c)". Revision 2's proposal "never gets within g of its capacity" was off by one: a Disproved means p ≥ c − g + 1, within g − 1 of c (T-06: c = 4, g = 2, violation at p ≥ 3). Naming N cannot read as more than the expansion checks |
| OQ11 | "Does not mean" line for a custom `never` | Routed to UX copy (D-46). Normative fact: the panel shows NS §8.3's "Proved means" line, and no added line may claim more |
| OQ12 | Vacuity guard; seeded built-ins | D-20: both ship (guard in slice 4, seeding in slice 0) |
| OQ13 | One namespace for states and events | D-01, D-26, ADR 0015: one namespace; a clash is a model problem |

Calls made here under the conservative rule (never risk a wrong Proved), recorded for D-21 review:

- **C1.** `no_dead_transitions` before slice 4 is Unknown `PropertyNotCheckable` like `never`, `always_reachable` and templates. D-25's list omits it; NS §10.2 ("the property kind is not checked yet in this slice") covers it.
- **C2.** A property-reference failure (an E2xx of §4.3, including NS L9; never NS L3, which covers only relationship and port references, D-47) ends only that property as `PropertyNotCheckable`; it does not fail the scope's lowering as `ModelIncomplete` (§4.3, §10.1). D-08 classes property errors this way; otherwise one typo would block every property of the scope.
- **C3.** A final predicate with an error makes the deadlock-free and T7 "done" properties of its behavior `PropertyNotCheckable`; it is never lowered as `final none`, which would report the intended end as a deadlock.
- **C4.** The Checks line of deadlock-free carries `Done when: …` rendered from the hashed `final` (§5), so the panel shows the final predicate that was checked, not the one stored now.
