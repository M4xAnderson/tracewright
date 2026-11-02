# v1: walking skeleton and next slices

As of 2026-11-02; applies `docs/specs/decisions.md` (D-01 to D-70) and `docs/specs/net-semantics.md` revision 3. Each slice cuts through UI, Tauri commands, `engine`, `verify`, `recheck` and `format`, is demoable on its own and ends in a dogfooding session on a real design. Order follows `opportunity-solution-tree.md`; E5 (paper replay) and the four spikes E1–E4 run before slice 0 code. **[proposal]** marks a sequencing choice that needs the product owner's yes. Terms follow `GLOSSARY.md`.

## Slice 0: walking skeleton, "find a deadlock between components I drew"

**Outcome:** on Linux, the user draws a few elements and relationships in one view, gives them Petri nets with ports, checks the combined behavior and replays the counterexample on the architecture. This is the shortest path to the v1 gate.

**In:**
- One view at Container level inside one software system (no drill-down). Create, move, rename and delete elements and relationships on the canvas. The scope's children are the elements whose parent is the system (D-13). IDs are lowercase slugs that never change on rename (D-12).
- Behavior models are Petri nets **written in the element's TOML file** and shown read-only in the behavior model pane; a Reload command re-reads changed files. Name clashes, dangling references and places without when full are problems in the Problems list, never load failures (D-26).
- Ports; a relationship binds ports in pairs, each with its own direction, so a call and its reply sit on one relationship (D-07): a synchronous binding fuses transitions, an asynchronous binding adds a generated place with a capacity (bag; when full defaults to overflow). A transition port has one role, and at most one asynchronous binding (D-16).
- Built-in properties deadlock-free and no capacity exceeded on the scope element, seeded by New project as explicit entries in the files (D-20). Initial marking from the files. Done when is not checked yet: every deadlock-free check lowers "no final predicate" (D-05); a `[final]` table already in a file is kept verbatim with a warning (D-25).
- Verdict panel: one verdict per property, with scope and assumptions. Proved: technique and number of reachable markings, or the "no overflow place" certificate with the drop places named (D-03). Disproved: replay step by step with involved elements highlighted (NS §6.7 rule, D-34), plus the steps as text. Unknown, with its reason and a next step: model incomplete, property cannot be checked (user properties, no dead transitions or templates already in a file; D-54), internal disagreement, internal error, budget exhausted, budget exhausted while re-checking, capacity exceeded first (with its run replayable) (D-08). Nothing to check when no child has a behavior model (D-13). Pending re-evaluation after an edit.
- Explicit Save to the TOML directory with an unsaved-changes marker; open an existing project.
- Dev-only differential corpus (E1) in CI: block encoded as complement places, drop nets excluded; a corpus disagreement fails CI and never feeds a verdict (D-35).
- Dev-only measurement (E2, D-70, ADR 0024): project open time on a generated project of 2,000 elements with one element file and one layout file each (about 4,000 files) on an SSD, against the file-format §2.3 target of under 1 s. It is recorded with the slice; it is not a user-facing acceptance criterion.

**Acceptance criteria:**
- Given two sibling elements whose ports take two locks in opposite order, When the user checks deadlock-free, Then the verdict is Disproved and the replay ends in the marking where both wait, with both elements highlighted.
- Given a deadlock whose marking has no token in any pre set (a finished workload), When it is shown, Then the panel says "no transition is enabled and no final predicate is defined", says that done when comes with slice 1, and never advises a loop back to the initial marking (D-23).
- Given a producer bound to a consumer by an asynchronous relationship with capacity 3 and no back-pressure, When the user checks no capacity exceeded, Then the verdict is Disproved and the last step names the generated place and its capacity.
- Given a scope in which every place blocks or drops, When the user checks no capacity exceeded, Then the verdict is Proved by the "no overflow place" certificate, the panel names the drop places, and the independent checker accepted the certificate first.
- Given the worker-pool example from E1, When checked, Then the verdict is Proved and the panel names the property, the scope and the technique (exhaustive exploration, number of reachable markings).
- Given a scope with no deadlock in which an overflow place can exceed its capacity, When deadlock-free is checked, Then the verdict is Unknown "capacity exceeded first", names the place and suggests block, drop or a larger capacity.
- Given a scope in which one child has a behavior model and another has none, When checked, Then the verdict is Unknown "model incomplete" naming the element; Given no child has a behavior model, Then the panel shows nothing to check and no verdict.
- Given a check that exceeds its budget, When it ends, Then the verdict is Unknown with the reason "budget exhausted" (or "budget exhausted while re-checking") and never looks like success.
- Given a counterexample, When it is shown, Then it has first been replayed independently on the second explorer's own composition; if replay fails, the verdict is Unknown "internal error" (D-24).
- Given an edit that changes the behavior of a checked scope, When the edit lands, Then earlier verdicts for that scope are removed until the check runs again (stale marking comes in slice 1). Layout-only edits keep verdicts current (GLOSSARY: Current).
- Given unsaved changes, When the user quits, Then the app asks to save; after Save and reopen, elements, relationships, behavior models and layout are unchanged.

**Out:** graphical Petri net editing, undo and redo, recovery journal, stale marking, cancel, drill-down, other levels, done when (final predicates), user properties and templates, state machines, scan, Windows and macOS builds, import and export.

## Slice 1: draw and change behavior on the canvas

**Outcome:** the user edits an element's Petri net graphically, marks where a run is meant to end, and iterates on a design without losing work or trusting an outdated verdict.

**In:** Petri net editor in the behavior model pane (places with capacity and when full, transitions, arcs, ports, initial marking); done when on the scope element, minimal form: the editor offers a conjunction of `place = n`, hand-written `[final]` text may be any predicate, and the one predicate parser ships now, used only by done when (D-05, D-25); undo and redo; recovery journal (ADR 0001); stale verdicts kept and marked, restored on undo (ADR 0005); cancel a running check (Unknown "cancelled"); elapsed time; Windows build; optional local check log for I1/I2.

**In: the template "never holds the whole pool" (T5), alone and unconditionally (D-60, ADR 0023).** Exactly what T5 needs moves here from slice 4 (`docs/specs/property-language.md` §8.3):
- parsing and resolving the `pool-share` parameters only (pool, held by, holder label, optional K), with the place picker that offers place ports and generated places and "make it a port" inline (D-01);
- its expansion `never Σ held by ≥ K` (default K = the initial tokens of the pool and the held-by places) and its preconditions E304, E305 and E308, checked at lowering and in the picker;
- one picker entry, T5; no other template is offered before slice 4;
- `never p` in both explorers, evaluated on overflow markings too (D-02), with its counterexample, independent replay and CheckKey line;
- a T5 entry is no longer "property cannot be checked" from slice 1 (in slice 0 it is, with "never holds the whole pool is checked from slice 1");
- the T5 panel: headline, "does not mean" line, Covers line with the held-by places, the Checks line, and the value of K the verdict was computed for.
Still slice 4: T3, T4 (including "never fills up"), T7, T8, custom predicates, the vacuity guard and the T5 hint about steps that take from the pool.

**Acceptance criteria:** Given E5 `case1_ebs_fixed` with done when `api_b/done = 2`, When deadlock-free is checked, Then it is Proved (99 markings), where slice 0 gave Disproved at the finished run (NS V49, V55). · Given E5 `case1_ebs_timeout` with done when `api_b/done = 2`, When deadlock-free and T5 (pool `control_plane/threads_free`, held by `in_a`, `wait_a`, `ret_a`, default K) are checked, Then deadlock-free is Proved and T5 is Disproved with a 3-step replay ending where the held-by places hold all 3 threads (PL V-42, T-01). · Given a T5 whose pool starts empty and no K, When it is checked, Then the verdict is Unknown "property cannot be checked: the pool starts with no tokens" (PL T-20). · Given Proved verdicts for deadlock-free and no capacity exceeded, When the user edits done when, Then only deadlock-free becomes stale (ADR 0018; from slice 4 also a "can always get back to: done" template, ADR 0022). · Given a Proved verdict, When the user changes a capacity, Then the verdict is marked stale, and undo brings it back as current. · Given the app is killed after three gestures, When it restarts, Then it offers to restore them. · Given a running check, When the user cancels, Then the verdict is Unknown "cancelled" within 1 s.

**Out:** pattern library, copy and paste of subnets, macOS signing, user properties other than T5, every other template, no dead transitions and the vacuity guard.

## Slice 2: state machines

**Outcome:** the user describes a component lifecycle (circuit breaker, connection) as a flat state machine and checks it combined with Petri net siblings **[decided: state machines ship in the slice after Petri nets]**.

**In:** flat state machine editor; input and output events as transition ports, states as place ports (D-01); compilation to a 1-safe Petri net whose state places overflow (ADR 0002, D-14); a message that arrives while no transition on its event is possible waits in the generated place (D-15); final states give the own-scope done when `Σ final states = 1` unless one is stated (D-42); every counterexample is also checked in state machine terms, and a failure is Unknown "internal error" (D-24); replay shows the active state.

**Acceptance criteria:** Given a circuit breaker state machine bound to a caller Petri net, When a deadlock exists, Then the replay shows each step as a state change of the state machine, not as places. · Given a state machine with final states and no stated done when, When its own deadlock-free is checked, Then stopping in a final state is not a deadlock. · Given a state machine, When it is saved, Then its TOML holds states and events, not the compiled net.

**Out:** nested states, history, data variables, guards on data.

## Slice 3: drill down across levels

**Outcome:** the user models System Context to Component and checks the combined behavior inside any element whose children have behavior models.

**In:** drill down and go up, breadcrumb (CNV-1, CNV-2); a scope per view; checks for the children of any element.

**Acceptance criteria:** Given a container with three components with behavior models, When the user drills down and checks deadlock-free, Then the verdict names the container as scope. · Given a verdict at Component level, When the user goes up, Then the container shows it in its inspector.

**Out:** composition across levels, refinement checks (ADR 0002).

## Slice 4: ask my own questions

**Outcome:** the user states design questions as properties: "never more than 2 orders in flight", "idle is always reachable".

**In:**
- User properties as linear marking predicates with "never" and "always reachable" (ADR 0003). In a combined scope a predicate names only place ports of the children (states included) and generated places; a template expansion computed from the model may name any place (D-01). Never p also looks at overflow markings; such a Disproved says that the run also exceeds a capacity (D-02).
- Property templates, stored by kind, version and parameters, each expanding to exactly one property (ADR 0019, D-11): T3 never overflows, T4 never more than (including "never fills up"), T7 can always get back to, T8 every transition can fire; T5 never holds the whole pool is already in from slice 1 (D-60, no evidence gate). T6 "never at the same time" is held back. A failed precondition is Unknown "property cannot be checked".
- Built-in no dead transitions, and the vacuity guard: it runs next to user properties (T5 included) and next to deadlock-free whenever the scope has done when, and every Proved shows its number of reachable markings (D-20, D-61). A hand-written predicate that when full alone decides gets warning W503, not Unknown.

**Acceptance criteria:** Given `never billing/inflight + shipping/inflight > 2` with both places declared as place ports, When it can be violated, Then Disproved with a replay ending where the sum is 3. · Given the same predicate naming a place that is not a port, When checked in the combined scope, Then the verdict is Unknown and names the place and the fix "make it a port". · Given a never p whose violating marking is an overflow marking, When checked, Then Disproved, and the panel says the run also exceeds the capacity of the named place. · Given a scope with user properties, When they are checked, Then no dead transitions is checked next to them. · Given E5 `case1_ebs_timeout` with done when and no user property, When deadlock-free is checked, Then no dead transitions is checked next to it and lists the AZ-A reply steps as dead (PL V-46, D-61).

**Out:** liveness, temporal logic, data, "can reach p" (D-40), `enabled(t)` atoms (D-45), a per-property done when (D-41).

## Slice 5: anti-pattern scan

**Outcome:** the user sees structural anti-patterns on designs without behavior models.

**In:** an explicit scan with ARCH-CYCLE, RUN-SYNC-CYCLE, REL-NO-TIMEOUT, DATA-SHARED-DB and REL-SPOF (`docs/specs/scan-rules.md`; REL-SPOF only if the E8 dry run finds a real single point of failure with replicas and zones statable); cancel a scan; attributes yes, no or not stated; interaction stated by the user, never set by bindings; Persons are the only entry points; severities High and Medium; findings panel; suppression with a disposition (accepted or not real), a reason and a date; Not checkable entries; duplicate REL-NO-TIMEOUT findings reported each (D-18, D-27).

**Acceptance criteria:** Given a relationship whose timeout is not stated, When the scan runs, Then REL-NO-TIMEOUT reports Not checkable for it, never a finding. · Given a suppressed finding, When the scan runs again, Then it stays listed as suppressed with its disposition and reason. · Given findings, When a binding or attribute the rules read changes, Then the findings are marked stale until the next scan.

**Out:** user-written rules, cloud well-architected rule packs, assumed cloud-kind defaults, threshold tuning UI, per-project rule disabling.

## Later, pulled forward on evidence

Faster and more decisive checks (P-invariant certificates, ILP state equation, stubborn sets) move before slice 3 if E9 shows I2 < 80 %. Pattern library moves before slice 2 if E6 fails; the E6 rule is applied at slice 1, slice 0 gives the baseline (D-57). PNML, Structurizr and LikeC4 import and export after slice 5.

## Open questions

- Decided 2026-11-02: re-checked evidence for an exhaustive Proved is a second, deliberately simple explorer (ADR 0008).
- Decided 2026-11-02: slice 0 authors nets in TOML; the graphical net editor comes in slice 1.
- Decided 2026-11-02 (delegated, `decisions.md`): done when moves from slice 4 to slice 1, minimal form (D-05); "no overflow place" is a certificate (D-03); Unknown has seven reasons (D-08); templates per ADR 0019 (D-11, D-20).
- Decided 2026-11-02 (product owner): T5 "never holds the whole pool" ships in slice 1 alone and unconditionally, T4 stays in slice 4, and the evidence gate is removed (D-60, ADR 0023); the vacuity guard also runs next to deadlock-free whenever done when is set, from slice 4 (D-61).
