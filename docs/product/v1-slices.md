# v1: walking skeleton and next slices

As of 2026-11-02. Each slice cuts through UI, Tauri commands, `engine`, `verify` and `format`, is demoable on its own and ends in a dogfooding session on a real design. Order follows `opportunity-solution-tree.md`; E5 (paper replay) and the four spikes E1–E4 run before slice 0 code. **[proposal]** marks a sequencing choice that needs the product owner's yes.

## Slice 0: walking skeleton, "find a deadlock between components I drew"

**Outcome:** on Linux, the user draws a few elements and relationships in one view, gives them Petri nets with ports, checks the combined behavior and replays the counterexample on the architecture. This is the shortest path to the v1 gate.

**In:**
- One view at Container level inside one software system (no drill-down). Create, move, rename and delete elements and relationships on the canvas.
- Behavior models are Petri nets **written in the element's TOML file** and shown read-only on the canvas; a Reload command re-reads changed files **[proposal: graphical net editing waits for slice 1]**.
- Ports; a relationship binds ports: synchronous fuses transitions, asynchronous adds a generated place with a capacity (bag).
- Built-in properties on the scope element (the system whose view it is): deadlock-free, no capacity exceeded. Initial marking from the files.
- Verdict panel: one verdict per property, with scope. Disproved: replay step by step with involved elements highlighted, plus the steps as text. Unknown: reason (budget exhausted, internal error).
- Explicit Save to the TOML directory with an unsaved-changes marker; open an existing project.
- Dev-only differential corpus (E1) in CI.

**Acceptance criteria:**
- Given two sibling elements whose ports take two locks in opposite order, When the user checks deadlock-free, Then the verdict is Disproved and the replay ends in the marking where both wait, with both elements highlighted.
- Given a producer bound to a consumer by an asynchronous relationship with capacity 3 and no back-pressure, When the user checks no capacity exceeded, Then the verdict is Disproved and the last step names the generated place and its capacity.
- Given the worker-pool example from E1, When checked, Then the verdict is Proved and the panel names the property, the scope and the technique (exhaustive exploration, number of markings).
- Given a check that exceeds its budget, When it ends, Then the verdict is Unknown with the reason "budget exhausted" and never looks like success.
- Given a counterexample, When it is shown, Then the independent replayer has replayed it first; if replay fails, the verdict is Unknown "internal error".
- Given an edit that changes the behavior of a checked scope, When the edit lands, Then earlier verdicts for that scope are removed until the check runs again (stale marking comes in slice 1). Layout-only edits keep verdicts current (GLOSSARY: Current).
- Given unsaved changes, When the user quits, Then the app asks to save; after Save and reopen, elements, relationships, behavior models and layout are unchanged.

**Out:** graphical Petri net editing, undo and redo, crash-recovery journal, stale marking, cancel, drill-down, other levels, user properties, final markings, state machines, scan, Windows and macOS builds, import and export.

## Slice 1: draw and change behavior on the canvas

**Outcome:** the user edits an element's Petri net graphically and iterates on a design without losing work or trusting an outdated verdict.

**In:** Petri net editor inside an element (places with capacity, transitions, arcs, ports, initial marking); undo and redo; crash-recovery journal (ADR 0001); stale verdicts kept and marked, restored on undo (ADR 0005); cancel a running check (Unknown "cancelled"); elapsed time; Windows build; optional local check log for I1/I2.

**Acceptance criteria:** Given a Proved verdict, When the user changes a capacity, Then the verdict is marked stale, and undo brings it back as current. · Given the app is killed after three gestures, When it restarts, Then it offers to restore them. · Given a running check, When the user cancels, Then the verdict is Unknown "cancelled" within 1 s.

**Out:** pattern library, copy and paste of subnets, macOS signing.

## Slice 2: state machines

**Outcome:** the user describes a component lifecycle (circuit breaker, connection) as a flat state machine and checks it combined with Petri net siblings **[decided: state machines ship in the slice after Petri nets]**.

**In:** flat state machine editor; events as ports; compilation to a 1-safe Petri net (ADR 0002); replay shows the active state.

**Acceptance criteria:** Given a circuit breaker state machine bound to a caller Petri net, When a deadlock exists, Then the replay shows each step as a state change of the state machine, not as places. · Given a state machine, When it is saved, Then its TOML holds states and events, not the compiled net.

**Out:** nested states, history, data variables, guards on data.

## Slice 3: drill down across levels

**Outcome:** the user models System Context to Component and checks the combined behavior inside any element whose children have behavior models.

**In:** drill down and go up, breadcrumb (CNV-1, CNV-2); a scope per view; checks for the children of any element.

**Acceptance criteria:** Given a container with three components with behavior models, When the user drills down and checks deadlock-free, Then the verdict names the container as scope. · Given a verdict at Component level, When the user goes up, Then the container shows it in its inspector.

**Out:** composition across levels, refinement checks (ADR 0002).

## Slice 4: ask my own questions

**Outcome:** the user states design questions as properties: "never more than 2 orders in flight", "idle is always reachable".

**In:** user properties as linear marking predicates with "never" and "always reachable" (ADR 0003); final markings, so proper termination is not reported as deadlock; no dead transitions.

**Acceptance criteria:** Given a saga with a final marking, When it ends there, Then deadlock-free is Proved. · Given "never Billing/inflight + Shipping/inflight > 2", When it can be violated, Then Disproved with a replay ending where the sum is 3.

**Out:** liveness, temporal logic, data.

## Slice 5: anti-pattern scan

**Outcome:** the user sees structural anti-patterns on designs without behavior models.

**In:** scan with ARCH-CYCLE, RUN-SYNC-CYCLE, REL-NO-TIMEOUT, DATA-SHARED-DB, REL-SPOF (`docs/research/antipatterns.md` §7.2); attributes yes, no or unknown; findings panel; suppression with a reason; not checkable.

**Acceptance criteria:** Given a relationship with an unknown timeout, When the scan runs, Then REL-NO-TIMEOUT reports not checkable for it, never a finding. · Given a suppressed finding, When the scan runs again, Then it stays listed as suppressed with its reason.

**Out:** user-written rules, cloud well-architected rule packs, threshold tuning UI.

## Later, pulled forward on evidence

Faster and more decisive checks (P-invariant certificates, ILP state equation, stubborn sets) move before slice 3 if E9 shows I2 < 80 %. Pattern library moves before slice 2 if E6 fails. PNML, Structurizr and LikeC4 import and export after slice 5.

## Open questions

- Decided 2026-11-02: re-checked evidence for an exhaustive Proved is a second, deliberately simple explorer (ADR 0008).
- Decided 2026-11-02: slice 0 authors nets in TOML; the graphical net editor comes in slice 1.
