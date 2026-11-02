# Property templates

As of 2026-11-02. Owner: product. Revised 2026-11-02 by the software architect to apply `docs/specs/decisions.md` (the PT lines of D-01–D-03, D-05, D-08–D-11, D-14, D-20, D-22, D-23, D-28, D-30, D-33, D-40–D-42), and again on 2026-11-02 for the review fixes D-60–D-68 (T5 in slice 1 without an evidence gate, the guard next to done when, T1/T4/T5/T7 wording, success metric). This is the catalog of properties the user picks and fills in instead of writing a predicate. Inputs: `north-star.md` (jobs J1–J6, north star, guardrails), `experiments/e5-paper-test.md` (E5), `v1-slices.md`, ADR 0001, 0002, 0003, 0008, 0010, 0013, 0014 and their amendments ADR 0015, 0017, 0018, 0019, `docs/specs/net-semantics.md` (NS), `docs/specs/property-language.md` (PL), `design/design.md`, `GLOSSARY.md`, `UX_STANDARDS.md` (VRD, TXT, WIN). Tags: **[decided]** settled elsewhere. **[assumption]** not yet backed by evidence. Every former proposal and ADR change is decided; section 12 maps each to its decision. PL §8 is the normative expansion of every template; this catalog owns the sentences, the "does not mean" lines and the acceptance criteria.

## 1. Problem and outcome

The solo designer of concurrent backends needs to ask design questions in their own words ("can the queue between Order API and Billing exceed 50?"). Writing linear marking predicates over qualified place IDs is slow and easy to get wrong. A Proved that reads as more than it says misleads as much as a wrong Proved. E5 showed both problems:

- **EBS case with a timeout.** "No deadlock" (with done when "every AZ-B call answered") and "can always get back to" were Proved, but the defect stayed hidden. Only the share-of-pool predicate was Disproved.
- **DynamoDB case.** A Proved "can always get back to" on the budgeted fix would mislead anyone who read it as "will recover".

- **Outcome:** more design questions answered per month (north star), at a lower cost per question (I1), with no Proved that the user reads as more than it says.
- **Success [assumption]:** from slice 4, at least 80 % of the properties the user adds for journal questions are templates rather than custom predicates. Only properties the user added count; the T1 and T2 entries that New project seeds are excluded (D-67).
- **Guardrails:**
  - Wrong Proved = 0. This is stop-ship **[decided]**. A template that expands to the wrong property gives a Proved of a different question, shown under the user's sentence. That counts as a wrong Proved (section 2).
  - Misread Proved = 0. If a journal note says a Proved meant less than the user read, that is a wording bug. It is fixed before the next slice.

## 2. What a template is

1. **Definition.** A **property template** is a named sentence with blanks. Once filled in, it expands to exactly one property of ADR 0003 and ADR 0010: deadlock-free, no capacity exceeded, no dead transitions, never p, or always reachable p. Templates add no semantics. A template is never a group of properties with a summary (ADR 0019). A **preset** is one fixed choice inside a template ("never fills up", "its initial marking", "done").
2. **Scope.** A filled-in template is a property. It has one scope element and is stored with it (ADR 0003), with `behavior` own or combined (PL §2). What its blanks can pick is section 2.6.
3. **Template expansion is in the trust path.** Expansion runs in engine lowering. Both explorers share lowering and do not cross-check it (ADR 0010). So the following are required:
   - (a) Each template and each preset has a lowering golden test, next to the binding-rule golden tests of ADR 0010 (PL §14.4).
   - (b) The "Checks:" line in the verdict panel is rendered from the canonical NetProperty that the worker explored and hashed into the CheckKey (PL §5). It is never derived again from the template parameters, in the engine or in the UI. Example: "Checks: never control_plane/in_a + control_plane/ret_a + control_plane/wait_a >= 3".
   - (c) Each template's expansion rule is part of what the file format means. Changing a rule, for example T5's default K, needs a format version bump and a migration under ADR 0001. Otherwise a project reopened months later (J6) would silently ask a different question.
4. **Stored by kind, version and parameters [decided, ADR 0019].** A template is stored by a stable kind ID and version (for example `pool-share`, v1) plus its parameters (PL §9). The UI name is not stored, so rewording the UI never breaks a file. The template is expanded during lowering at every check.
   - Values that the template reads from the model, such as a capacity, the tokens a pool starts with, or the final predicate, therefore follow edits. After the user resizes a place (J2, J4), the property asks about the new capacity and the old verdict is stale.
   - The CheckKey hashes the expansion **as parsed** (D-10). A template shares a key with a hand-written predicate only when the two trees are equal: T3 on a place with capacity 50 shares its key with "never p > 50", not with "never p >= 51". No equivalent spellings are normalized; the only cost is a cache miss.
   - The verdict that is shown carries the expanded property it was computed for. This copy is in memory and for display only, never persisted (ADR 0005). Headlines and W10 read their values from it.
   - "Edit as predicate" (slice 4) freezes today's values. Its dialog says: "The predicate keeps today's values (capacity 50) and no longer follows edits."
5. **Preconditions.** Lowering checks each template's preconditions (section 6). When one fails, the result is Unknown "property cannot be checked" (`PropertyNotCheckable`, ADR 0017) with a line that names the cause. It is never a vacuous verdict. The template picker applies the same checks before it adds a property.
6. **Which places a blank can name [decided, ADR 0015].** In a combined scope a blank names a **place port** of a child (every state of a state machine is one) or a relationship's generated place; in an own scope any place of the element. Blanks offer exactly these, and the picker offers "make it a port" inline for an internal place, because promoting a place is one attribute (PL §4.4). The "its initial marking" preset is the one exception: lowering computes it and it may name every place. States are written `element/state` in the one namespace of the element; there is no separate state ID shape.

## 3. Two kinds of question

The model has no time. Steps happen in any order, and a timeout is a step that can happen at any moment. Read every verdict through this table, because it is what keeps a Proved from saying more than it does.

| | "Never" questions (T1–T5) | "Can" questions (T7, T8) |
|---|---|---|
| **Proved** | The property holds for every order of the steps the model has. A step the model leaves out is not covered. For example, a slow service modeled as never responding loses its late response. | A way exists in the model. That way may need an order of steps that real timing never produces, and nothing makes the system take it. |
| **Disproved** | One possible order of steps breaks the property. It may need a timing that the real system rules out, so the user confirms it before counting it (north star, condition 3). | T7: after the steps of the counterexample, no continuation reaches the target. T8: the listed transitions fire in no run. |

Modeling help, shown with "never" questions: model a slow service as "may respond at any time" (keep the response step). That also covers slow. Use a "never responds" variant only to look for hangs with T1.

## 4. Catalog

| ID | Name | Sentence (example) | Expands to (PL §8.2) | Slice | Jobs | E5 |
|---|---|---|---|---|---|---|
| T1 | No deadlock | "Order system never deadlocks." | deadlock-free with the scope's final predicate (done when) | 0; done when from 1 (ADR 0018) | J1 J2 J3 | case 1 P1, case 2 P3 |
| T2 | No capacity exceeded | "Order system never goes over any capacity." | no capacity exceeded | 0 | J1 J3 J4 | case 2 P1, case 3 P1 |
| T3 | Never overflows | "Messages in transit on Order API → Billing never go over the capacity (50)." | never p > capacity(p) | 4 | J1 J4 | case 2 P1, case 3 P1, for one place |
| T4 | Never more than | "Billing / inflight and Shipping / inflight together never hold more than 2." | never Σ places > N | 4 (D-60; no evidence gate) | J4 J1 | north-star example; slice 4 criterion |
| T5 | Never holds the whole pool | "AZ-A calls never hold all 3 of Control plane / threads_free." | never Σ held by ≥ K | **1**, alone and unconditionally (D-60, ADR 0023; PL §8.3 lists what moves) | J1 J3 | case 1 P4 |
| T7 | Can always get back to | "Storage fleet can always get back to: Storage / serving = 3." | always reachable p | 4 | J1 J3 | case 1 P3, case 2 P2, case 3 P2 |
| T8 | Every transition can fire | "Every transition in Order system can fire at least once." | no dead transitions | 4 | J2 J6 | none |
| — | Custom predicate | "Never: …" / "Can always get back to: …" written by hand | never p / always reachable p (ADR 0003) | 4 | any | — |

**T6 "Never at the same time" is held back [decided, ADR 0019].** J1 names locks, but E5 had no such case. In slice 4 the custom predicate covers the question: "Never: Worker 1 / critical ≥ 1 and Worker 2 / critical ≥ 1". T6 becomes a template when the journal shows two lock questions. Its draft wording is kept in section 5 so it is cheap to add then.

From slice 2, the blanks also offer states of state machines ("Breaker is in Open"). A state is a place port named `breaker/open` that holds one token while the state is active (ADR 0015). From slice 3, every template works in any scope that has a view.

## 5. Templates

Every template's Unknown can carry any reason in section 7. Only reasons that are specific to a template are listed with it. Each template has one canonical "does not mean" line (shown in quotes and at most 75 characters). The UI and the tests use that exact string. A longer help text may follow it. Panel copy beyond these strings is the senior UX designer's (D-46) and may not contradict NS §9.6.

### T1 No deadlock
- **Parameters:** none. The scope's **done when** is its final predicate: model data of the scope element (ADR 0003), one for its own behavior and one for its combined behavior. T1 uses the one of its behavior; lowering copies it into the deadlock-free property (`DeadlockFree { final }`, ADR 0018), so editing done when re-checks only T1 and T7 "done" of that behavior. It is shared by T1 and T7's "done" preset; a property cannot override it. From slice 2, a state machine's final states give its own-scope default (`Σ e/f = 1`, SM §3.6). The Covers line shows the final predicate that was checked.
- **Proved:** "Proved: Order system never deadlocks." In every reachable marking, at least one step can happen, or the marking satisfies done when (GLOSSARY Deadlock).
- **Does not mean:** "No deadlock does not mean done: a timeout or retry loop can run forever."
  - Help (from slice 1, when done when is set): E5 case 1 with a timeout and done when `api_b/done = 2` is Proved, even though AZ-A calls can hold the whole pool until each timeout fires. Ask T5 for that.
  - The verdict also says nothing about other initial markings or about behavior outside this scope.
- **Disproved:** "Disproved: Order system never deadlocks." followed by "Order system can deadlock: after 6 steps, no step can happen." The replay ends there and shows which steps wait for which tokens (slice 0 criterion).
  - Without a final predicate, a design that is meant to end is reported as stuck at its end. The panel states NS §9.6's fact, "no transition is enabled and no final predicate is defined" when no transition's input holds a token, and points to done when: "Define done when" from slice 1, "done when comes with slice 1" in slice 0 (D-23).
  - The panel gives no advice to add a way back to the start. A reset transition changes every other property in the scope and can hide real deadlocks.
- **Typical Unknown:** capacity exceeded first, because generated places overflow by default (ADR 0014).
- **Also ask (depends on the slice; the panel never points to a template the slice does not check):**
  - Slice 0: modeling help only (section 3: keep the response step of a slow service; done when comes with slice 1).
  - Slices 1–3: T5 if a pool is shared; otherwise modeling help (section 3). T7 and T8 are not named.
  - From slice 4: T5 if a pool is shared, T7 "done" for "can always still finish", and T8 to see whether some steps never happen. With done when set, T8 already runs next to T1 (vacuity guard, D-61).

### T2 No capacity exceeded
- **Parameters:** none. The Covers line lists the places that overflow when full, and separately the places that block or drop. Places that block or drop cannot go over their capacity. State places of state machines are not listed among the places that overflow: each holds at most one token by construction and is compiled to overflow only so that a lowering defect would show up (D-14).
- **Proved:** in every reachable marking, every place that overflows when full holds at most its capacity.
  - If no place in the scope overflows when full, the Proved comes from the certificate `NoOverflowPlace`, which recheck checks on its own composition (NS §9.5, ADR 0017): a certificate that recheck checks, not a shortcut. Otherwise it comes from exploration confirmed by recheck. A scope with a state machine always explores.
  - The headline follows W2. For the certificate the panel appends NS §9.6's fact: no place in the scope can overflow, every place blocks or drops, and tokens may be discarded at the listed drop places.
- **Does not mean:** "Places that block or drop can still make senders wait or lose messages."
  - Help (from slice 4): ask T4 "never fills up" for that.
  - It also says nothing about other initial markings. E5 case 2 is Proved with 3 peers and Disproved with 5, and says nothing about 4.
- **Disproved:** "Messages in transit on A → B can go over the capacity (3)." The replay ends at the overflow marking (slice 0 criterion). If an element sends again before its earlier sends are done, the help text reads: "Retries can outnumber any capacity. Consider a retry budget." (E5 implication 6).
- **Typical Unknown:** budget exhausted.

### T3 Never overflows
- **Parameters:** one place that overflows when full. It is either a generated place, picked through its relationship, or a place inside a behavior model (in a combined scope, a place port). The capacity is read during lowering.
- **Precondition:** the place overflows when full. A place that blocks or drops is not offered, and the picker offers T4 "never fills up" instead (W9). If the user later switches the place to block or drop, the result is Unknown "property cannot be checked: T3 needs a place that overflows when full; Billing / inflight now blocks."
- **Proved:** this place never goes over its capacity. A never-p Proved needs no reachable overflow marking at all (ADR 0017), so this Proved also means that no other place in the scope overflows, which is the same as T2.
- **Does not mean:** "Holds for this initial marking only; more callers may overflow it."
- **Disproved:** the replay ends at the overflow marking where this place goes over its capacity, and the panel says the run exceeds that capacity. "Never p" is evaluated on overflow markings (ADR 0017, D-02); PL T-08 and T-16 pin it.
- **Typical Unknown:** "capacity exceeded first", naming another place. Next step: check T2, set that place to block or drop, or raise its capacity.

### T4 Never more than
- **Parameters:** one or more places, added up with weight 1 unless set otherwise, and N ≥ 0.
- **Presets:**
  - **"never fills up":** one place. It expands to N = capacity − g, where g ≥ 1 is the most tokens that any single step in the scope adds to the place (net of what it takes). For a generated place, g = 1. With g = 1 it renders as "Messages in transit on Order API → Billing never reach the capacity (50)". With g > 1 it names N: "Y / p never holds more than 2 (capacity 4)", because a Disproved then means only that the place got within g − 1 of its capacity, not that it reached it (PL §17 OQ10).
  - **"at most N in flight":** a sum over several places.
- **Preconditions:**
  - N ≥ 0. For "never fills up", capacity − g ≥ 0.
  - The sum must not be decided by "when full" alone, which happens when every place in the sum blocks or drops and N ≥ the sum of their capacities. The picker warns about this and does not add the property. After an edit, the result is Unknown "property cannot be checked".
- **Proved:** in every reachable marking, the sum is at most N.
  - "Never fills up" Proved: no step in this scope ever waits at this place or loses a message there because it is full.
  - **Resize statement [claim, confirmed by an E1 golden test before slice 4 ships]:** for one place, in this scope and with this initial marking, setting the capacity to N leaves every reachable marking unchanged. Verdicts in other scopes, and properties that read this capacity (T3, "never fills up"), go stale and are checked again.
- **Does not mean:** "A bound for this initial marking only; more callers may exceed it."
- **Disproved:** the replay ends where the sum is above N (slice 4 criterion: the sum is 3 for N = 2).
- **Typical Unknown:** capacity exceeded first.

### T5 Never holds the whole pool
- **Parameters:**
  - the **pool**, a place whose tokens are shared resources (for example `control_plane/threads_free`)
  - the places that count as **held by** the holder (for example `control_plane/in_a`, `wait_a`, `ret_a`)
  - in a combined scope the pool and the held-by places are place ports (section 2.6)
  - an optional label for the holder ("AZ-A calls"). It is for display only and is not part of the key.
  - K. By default, K is the tokens that the pool and the held-by places hold in the initial marking, which is the whole pool **[assumption: pools start full; K is editable]** (D-11).
- **Expands to:** never Σ held by ≥ K.
- **Slice:** 1, alone and unconditionally (D-60, ADR 0023). In slice 0 a T5 entry in a file is Unknown "property cannot be checked: never holds the whole pool is checked from slice 1" (PL S-06).
- **Preconditions:** K ≥ 1, at least one held-by place, and the pool is not one of the held-by places. A pool that starts empty gives the default K = 0. The picker then asks for K, and after an edit the result is Unknown "property cannot be checked: the pool starts with no tokens". A K the user sets below 1 gives "K must be at least 1" (PL T-18).
- **Proved:** the holder never holds K or more of the pool at once. With the default K, the holder never holds all of it.
- **Does not mean:** "Others may still hold the rest; waiting calls may never be served."
  - Help: "Tokens in places not marked as held by are not counted."
  - The headline uses the holder label, so the Covers line always names the held-by places.
- **Disproved:** the replay ends where the holder has K. E5 case 1: three AZ-A calls take all 3 threads in 3 steps. This was caught even with a timeout, where T1 and T7 were Proved. Help text: "This property catches one holder taking the whole pool, even with a timeout." It makes no claim about starvation (section 8).
- **Hint (slice 4, outside the trust path, display only):** "These steps take from the pool but put nothing into a held-by place: Control plane / accept_b." The hint helps the user notice a missing held-by place. It never changes the verdict.
- **Later:** suggesting the held-by places from a P-invariant through the pool (S11).

### T6 Never at the same time (held back, section 4)
- **Parameters:** two or more conditions, each "<place> holds a token" or "<element> is in <state>". **Expands to:** never (c1 and c2 …).
- **Proved:** no reachable marking satisfies all the conditions at once. **Does not mean:** "Says nothing about whose turn comes, or whether each side gets one."

### T7 Can always get back to
- **Parameters:** a target, given as conditions on places or states joined by "and" ("Storage / serving = 3").
- **Presets:**
  - **"its initial marking":** exactly the initial marking of the scope. Computed at lowering; it may name internal places (section 2.6).
  - **"done":** the final predicate of the property's behavior (done when). The picker names this preset "can always still finish"; the headline names the predicate that was checked, rendered from the hashed property with display names (PL §5): "EBS can always reach done when: API B / done = 2". Precondition: the scope has done when; otherwise "property cannot be checked: this scope has no 'done when' yet". Editing done when re-checks this property, because done when is its expansion.
- **Expands to:** always reachable p.
- **Proved:** from every reachable marking, at least one way to continue reaches the target.
- **Does not mean (always shown):** "A way back exists; under load or with timeouts it may never be taken."
  - Help: E5 case 3 is Proved for the fleet with a retry budget, and that fleet can still stay stuck when the service is slower than the timeout **[decided non-goal]**.
  - A Proved also does not mean that every request gets an answer.
- **Disproved:** "Storage fleet can never get back to: serving = 3, after these 8 steps." The replay leads to that point, and the panel adds: "No continuation from here reaches it (checked over N markings)."
  - Disproved only if no overflow marking can be reached from the counterexample's last marking; otherwise the verdict is Unknown "capacity exceeded first" (ADR 0017, D-09; NS §8.4).
- **Typical Unknown:** capacity exceeded first (E5 case 2 P2 and case 3 P2 before their fixes), or budget exhausted while checking all continuations.

### T8 Every transition can fire
- **Parameters:** none.
- **Proved:** each transition fires in at least one possible run.
- **Does not mean:** "Each can fire in some run, not in every run or the way you intend."
- **Disproved:** a list of the transitions that can never fire, with their elements highlighted.
  - A port transition is fused once per caller (ADR 0010). A dead copy is listed as "Billing / accept, when called by Order API". If every copy is dead, the list shows "Billing / accept (every caller)".
  - There is no replay. The evidence is an exhaustive exploration that the second explorer confirmed (ADR 0010). The glossary's Disproved and VRD-2 carry this exception (D-33).
  - The usual cause is a modeling mistake, such as a missing token in the initial marking or a wrong binding. It counts toward the north star only if the user confirms it as a defect of the design.
- **Typical Unknown:** capacity exceeded first. Whenever an overflow marking is reachable, the verdict is Unknown, even if some transition is clearly dead, because an overflow can hide a transition that would otherwise fire (ADR 0017, D-09; NS §8.5).
- **Vacuity guard (slice 4, D-20, D-61):** T8 runs next to user properties of the scope and next to T1 whenever the scope has done when for that behavior, and every Proved shows its exploration count, so a Proved over a model where the asked-about steps never happen is visible. E5 case 1 with a timeout: T8 lists `r1/return_a+reply` and `r3/respond+done_a`, so no AZ-A call is ever answered although T1 and T7 are Proved (PL V-46).

## 6. Preconditions, checked at lowering and in the picker

| Template | Precondition | Unknown "property cannot be checked" line (example) |
|---|---|---|
| T3 | The place overflows when full | "T3 needs a place that overflows when full; Billing / inflight now blocks." |
| T4 | N ≥ 0; the sum is not decided by "when full" alone | "Every place in the sum blocks or drops, so 60 can never be exceeded." |
| T4 "never fills up" | capacity − g ≥ 0 | "Capacity 1 is smaller than the 2 tokens one step adds." |
| T5 | K ≥ 1; at least one held-by place; the pool is not held | "The pool starts with no tokens. Set K." / "K must be at least 1." / "Choose at least one held-by place." |
| T7 "done" | The scope has done when | "This scope has no 'done when' yet." |
| All | Every named place, state or relationship exists and may be named in this scope (section 2.6) | "Billing has no place inflight." (PL E202) / "Billing / inflight is not a port. …" (PL E208) |

The lines are PL §10's E2xx and E3xx messages; PL is normative for their text.

## 7. Unknown, in plain words

One closed list of seven reasons (NS §10.2, ADR 0017). The words below are the catalog's; "no counterexample up to depth N" and "unsupported construct" are not reasons this tool produces.

| Reason | Plain words | Next step shown |
|---|---|---|
| Budget exhausted | "Ran out of time or memory after N markings. Nothing is known either way." | Fewer tokens in the initial marking, a smaller scope, or a larger budget |
| Budget exhausted while re-checking | "Ran out of budget while re-checking the answer independently. An answer that is not re-checked is not shown." | As above |
| Capacity exceeded first | "<Place> can go over its capacity first. What happens after that is not defined." | Set <place> to block or drop, raise its capacity, or check T2 |
| Model incomplete | One line per cause ("shipping has no behavior model", "billing/pool: initial 5 exceeds capacity 3") | Fix the listed item |
| Property cannot be checked | One line per cause from section 6 or PL §10; before slice 4, "user properties are checked from slice 4", except T5, which is checked from slice 1 (in slice 0: "never holds the whole pool is checked from slice 1") | Change the property or the model |
| Cancelled | "Cancelled after N markings." | Run again |
| Internal error, internal disagreement | "A bug in the tool, not in your design. It was logged." | Report it; nothing about the design is known |

A scope where no element has a behavior model has no verdict at all: its properties show "nothing to check" (NS §10.2, D-13), never an Unknown.

## 8. Not templates, and what to use instead

| Question the user asks | Why not in v1 | Closest v1 property | Use instead |
|---|---|---|---|
| "Will it recover once the trigger is gone, or under load?" (metastability) | No rates or time (E5 case 3; north-star non-goal) | T7 says only that a way back exists. T2 or T3 on a model with a retry budget shows whether retries can outnumber the capacity | Load test, queueing model or simulation. Timed analysis becomes a follow-up if the journal shows that timing questions dominate (E5 (c)) |
| "Does every request eventually get a reply?" | Liveness (ADR 0003) | T7 "can always get back to: no pending requests"; T1 | TLA+, Quint, P |
| "Can this ever happen at all?" (can reach p) | Not a v1 property kind (ADR 0019, D-40) | A Disproved "never" shows a way there; T8 and the exploration counts (vacuity guard) | After v1, possibly an "expected: reachable" flag |
| "Are messages handled in order? Does the reply match the request?" | Places are bags (ADR 0002) | None | TLA+, Quint, P |
| "Is the timeout shorter than the retry interval? Is latency under X ms?" | No time | None | Timed model, load test |
| "Does every caller get its turn?" (starvation, fairness) | Fairness is liveness | T5, T4 | TLA+ with fairness |
| "Does this hold for any fleet size?" | The initial marking fixes the token counts | Check two or three counts | Later: P-invariant certificates prove a bound for every count (E5 case 3, S11) |

## 9. Wording rules for the UI

- **W1** The only verdicts are Proved, Disproved and Unknown, in text with an icon. Unknown never looks like success (VRD-1). There is no combined success label such as "All properties proved". A neutral count ("3 Proved, 1 Disproved, 1 Unknown") is allowed.
- **W2** Headline per verdict:
  - Proved and Unknown: the verdict plus the template sentence exactly as asked.
  - Disproved: the verdict plus the sentence, then one "can" line (W6).
  - Names in the headline are current (ADR 0013). Values (capacity, N, K) always come from the expansion that the verdict was computed for, never from the current model.
  - No paraphrase may be stronger than the sentence. Notes such as T2's certificate line are appended under the headline, never in place of it.
- **W3** Sentence forms:
  - T1–T5: "never …"
  - T7: "can always get back to …"; with the "done" preset, "can always reach done when: <predicate>" (the picker label stays "can always still finish")
  - T8: "every transition … can fire at least once"
  - These words are banned in sentences, verdicts and help, except inside a "does not mean" line: will, eventually, recovers, self-heals, guaranteed, safe, correct, healthy, resilient, under load. The glossary's _Avoid_ words are banned too (passed, failed, verified, violated, green, bounded-proved, and "timeout" for the budget).
- **W4** Every Proved shows a **Covers** line with:
  - the scope and the combined children
  - the initial marking (places with tokens, up to 5, then "and N more")
  - for T1 and T7 "done", the final predicate that was checked (done when, or "none")
  - assumptions (relationships of the scope that take no part in the check)
  - for an own scope, the closed-port note "every port of <element> can fire at any time" (not an assumption, D-30)
  - for T2 and T3, the places that block or drop; for T5, the held-by places
  - the technique (exploration or certificate) and the number of markings
- **W5** Every Proved shows its canonical "does not mean" line from section 5 directly under the headline, not in a tooltip (WIN-3). For T1 and T7 it can never be collapsed.
- **W6** A Disproved's "can" line uses "can" ("… can go over the capacity (3)") and adds: "One possible order of steps, not necessarily a likely one." A "never" Disproved whose last marking is an overflow marking also says that the run exceeds that capacity (NS §9.6).
- **W7** An Unknown gives the reason, what was tried and the next step (section 7; VRD-3). It never shows the main explorer's answer before re-checking. It never says "no problems found", "probably" or "so far".
- **W8** Names in sentences:
  - a generated place: "messages in transit on <A> → <B>"
  - a place: "<Element> / <place>"
  - a state: "<Element> is in <State>"
  - Qualified IDs appear only in the "Checks:" line.
- **W9** A template whose answer "when full" alone would decide is not offered (T3 on a place that blocks or drops, T4 in section 6). After an edit, that case becomes Unknown "property cannot be checked", never a vacuous Proved. A hand-written predicate in the same situation gets a warning instead, because its headline is the predicate itself (PL W503, D-20).
- **W10** A stale verdict shows the values it was computed for when they differ from the current ones ("capacity 50 then, 30 now"). The values come from the expansion the verdict carries (section 2.4).
- **W11** The template picker also finds templates through the user's own words (queue, buffer, pool, lock, idle, finish, starvation, overflow). These words are search aliases only and never appear as labels.
- **W12** UI text uses US English and glossary terms (TXT-1), with at most about 75 characters per line (WIN-2).

## 10. Acceptance criteria

**Fixtures.** The E5 files are ported to the product format as E1 fixtures, mapped as in NS §13: every internal place overflows when full, to match E5's semantics; bags become generated places on relationships; a call and its reply sit on one relationship (`case1_*` has `r1`–`r4`, and the timeout pair `timeout_a + fail` sits on `r1`); the transitions `ret_a` and `ret_b` are renamed `return_a` and `return_b`; every place a property or final predicate names is a place port. A criterion names the ported fixture (for example `e5/case1_ebs_timeout`). Each criterion applies from the slice in its template's row; PL §14 holds the exact figures.

- Given `case1_ebs_timeout` with done when `api_b/done = 2` (slice 1), When T1 is checked, Then the verdict is Proved, the headline reads "… never deadlocks", and "No deadlock does not mean done: a timeout or retry loop can run forever." appears under the headline and cannot be collapsed (PL V-42).
- Given `case1_ebs_timeout` with done when `api_b/done = 2` (slice 4), When T1 is checked and no user property exists in the scope, Then T8 runs next to it and lists `r1/return_a+reply` and `r3/respond+done_a` (vacuity guard, D-61; PL V-46).
- Given `case1_ebs_timeout` without done when (slice 0), When T1 is checked, Then the verdict is Disproved after 17 steps with "… can deadlock", the replay ends where no step can happen, the panel points to done when, and it does not suggest a way back to the start (PL V-43).
- Given `case1_ebs_timeout` (slice 1), When T5 is checked with pool `control_plane/threads_free`, held by `in_a`, `wait_a`, `ret_a` (all place ports) and the default K, Then K is 3 and the verdict is Disproved with a 3-step replay ending where the held-by places add up to 3, and the help says "This property catches one holder taking the whole pool, even with a timeout." (PL T-01).
- Given a project file with a T5 entry (slice 0), When it is loaded, Then the property is Unknown "property cannot be checked" with "never holds the whole pool is checked from slice 1"; Given the same file in slice 1, Then it is checked (PL S-06).
- Given `case3_dynamodb_fixed`, When T7 "can always get back to: Storage / serving = 3" is Proved, Then the headline contains "can always get back to", contains none of the W3 words, and the "does not mean" line names load and timeouts.
- Given `case3_dynamodb_bad`, When T7 is checked, Then the verdict is Unknown "capacity exceeded first". The Unknown names messages in transit on the request relationship `r2`, suggests block, drop or a larger capacity, and shows no answer that was not re-checked.
- Given an asynchronous relationship whose "when full" is block, When the user opens the template picker for it, Then T3 is not offered and T4 "never fills up" is.
- Given T3 on a generated place with capacity 50, When the user changes the capacity to 30, Then the property asks about 30, and the earlier verdict is marked stale. Its headline still reads "(50)" and it shows "capacity 50 then, 30 now".
- Given T3 on a generated place, When the user switches that place to block, Then the check ends Unknown "property cannot be checked" with the T3 line from section 6, and no Proved is shown.
- Given T3 on a generated place with capacity 2 that three sends can overflow, When it is checked, Then the verdict is Disproved with a 3-step replay ending at the overflow marking, and the panel says the run exceeds the capacity (3 of 2) (PL T-16; V-24 is the hand-written `>= 3` with a different key).
- Given T4 "never fills up" on a block place where one step adds 2 tokens, When it is lowered, Then the "Checks:" line shows N = capacity − 2 and the sentence names N (PL T-06).
- Given T4 with N = 2 over billing/inflight and shipping/inflight (both place ports), When the sum can exceed 2, Then the verdict is Disproved and the replay ends where the sum is 3 (slice 4 criterion).
- Given a scope with a transition that can never fire and no reachable overflow marking, When T8 is checked, Then the verdict is Disproved with that transition listed and its element highlighted, and no replay is offered.
- Given a check that runs out of budget while re-checking, When it ends, Then the verdict is Unknown with that reason, and nothing in the panel reveals the main explorer's answer.
- Given T3 on place p with capacity 50, When a hand-written "never p > 50" exists in the same scope, Then both have the same CheckKey, a hand-written "never p >= 51" has a different one, and the "Checks:" line of each is rendered from the hashed canonical property (D-10).
- Given a scope with T1 and T7 "done" and a "never" property, When the user edits done when, Then T1 and T7 "done" go stale and the "never" property stays current (ADR 0018).
- Given a project file with a template property other than T5 (slices 0–3), When it is loaded, Then the property is Unknown "property cannot be checked" with "user properties are checked from slice 4" (PL S-02).

## 11. Measuring it (local only, no telemetry)

- **Dogfooding journal:** add two columns. **template** records T1–T8 or custom. **could not ask** records time, order, liveness or other, for questions that no template or predicate could express.
- **Local check log (opt-in, slice 1):** add the template ID next to the verdict and the Unknown reason, so I2 can be tallied per template, and mark whether the property was seeded or added by the user (section 1 counts only added ones). T1 and T2 ship in slice 0 and T5 in slice 1, so the ID is meaningful from slice 1.
- **Decision rules:**
  - If "could not ask: time" is at least 25 % of the journal's questions in any two consecutive months, timed analysis comes back as a discovery item (E5 (c)).
  - Two journal questions about locks make T6 a template.
  - No evidence gate for templates remains: T5 ships in slice 1 and T4 in slice 4 unconditionally (D-60, which removes D-11's gate). Moving T4 earlier needs a new decision.

## 12. Decisions applied (former proposals and ADR changes)

| Former item | Decision | Where it lives now |
|---|---|---|
| 12.1 ADR 0003: any place or state of a child, `element/state:<name>` | Withdrawn. Place ports (states included) and generated places; one namespace; the "its initial marking" preset is exempt (D-01) | ADR 0015, NS L9, PL §4.4, section 2.6 |
| 12.2(a) never p on overflow markings | Adopted (D-02) | ADR 0017, NS §8.3, T3 |
| 12.2(b) T7 Disproved only without a reachable overflow; 12.2(c) T8 Unknown whenever an overflow is reachable | Adopted; the duplicate amendment text here is withdrawn (D-09) | ADR 0017, NS §8.4, §8.5 |
| 12.2(d) "capacity exceeded first" names every overflow place | Withdrawn: it names the smallest-ID exceeded place of the first overflow marking and carries the run to it (D-22) | NS §10.2 |
| 12.3 done when in slice 1 | Adopted, minimal form; the final predicate lives in the deadlock-free property (D-04, D-05) | ADR 0018, NS §3.3, §8.1, PL §7 |
| 12.4 T4 and T5 to slice 1 behind an evidence gate | Superseded: T5 ships in slice 1 alone and unconditionally; T4 stays in slice 4; the gate is removed (D-60) | ADR 0023, PL §8.3, section 4, section 11 |
| Vacuity guard next to T1 with done when | Adopted (D-61) | ADR 0023, T8, PL §11 |
| T1 "never deadlocks" / "can deadlock"; T4 and T5 lines; T7 "done" headline; T1 "Also ask" by slice; success metric | Decided (D-67) | sections 1, 5, 9 |
| 12.5 templates ADR | Adopted: kind, version, parameters; one property per template. The "canonical predicate form" (Σ w·p ≥ c) is withdrawn: predicates are hashed as parsed (D-10, D-11) | ADR 0019, PL §8, §9 |
| T2 "no shortcut" | Replaced: the certificate `NoOverflowPlace` that recheck checks (D-03) | ADR 0017, NS §9.5 |
| "Property does not apply" | Replaced by "property cannot be checked" (`PropertyNotCheckable`, D-08) | ADR 0017, NS §10.2 |
| 12.6 Disproved without replay for T8 | Adopted (D-33); applied to `GLOSSARY.md` and UX_STANDARDS VRD-2 by their owner | — |
| T6 held back | Confirmed (D-11) | ADR 0019 |
| Vacuity guard, seeded built-ins, warning for hand-written predicates | Adopted (D-20) | ADR 0019, T8, W9 |
| T4 "never fills up" wording for g > 1 | Decided: the sentence names N (PL §17 OQ10) | T4 |

**E1 golden tests** (each with its PL vector): T3's Disproved at an overflow marking (T-08, T-16); the T4 resize statement in the exact wording of section 5; "never fills up" with a step that adds 2 tokens (T-06); T7 "its initial marking" equals the initial marking (T-10, T-11); T7 and T8 with a reachable overflow marking (NS V39, V46); one lowering test per template and per preset (PL §14.4).

**Glossary** (applied by the `GLOSSARY.md` owner from the merged block of `consistency-review.md`, D-28–D-33): Property template (a preset is a sub-term; avoid "pattern", "snippet", "macro", "rule"), Pool, Held by, Final predicate (UI label "done when"; one per behavior of a scope element; replaces this catalog's former "Done when" entry and the glossary's "final markings"), Step (avoid "frame", "move", "tick", "event"; "firing" is the formal act and is not avoided). Existing terms are used where they exist: initial marking (not "starting tokens" or "the start").

**Follow-ups owned elsewhere:** `docs/ux/flows.md` §4.13 (seven templates, a place picker per section 2.6, previews that say "reachable marking") and the T1, T2 and never-p panel copy (D-23, D-46); `v1-slices.md` slices 0, 1 and 4 (D-38).
