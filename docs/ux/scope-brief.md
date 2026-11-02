# UX scope brief: slices 0, 1 and 2

As of 2026-11-02. Owner: product manager. Revision 2, after review by the senior UX designer and the senior frontend developer. This brief lists the screens, panels and states that slices 0–2 need, and nothing else. Inputs: `docs/product/v1-slices.md`, `north-star.md`, `opportunity-solution-tree.md`, `experiments/e5-paper-test.md`, `docs/design/design.md`, `docs/research/stack.md`, ADRs 0001, 0005, 0006, 0007, 0010, 0012, 0013 and 0014, and `UX_STANDARDS.md`. Terms follow `GLOSSARY.md` exactly.

Tags:
- **[decided]** comes from a settled doc.
- **[proposal]** needs the product owner's yes.
- **[assumption]** is untested.
- **[route: X]** needs an answer from persona X before the slice starts.

If a surface or state is not listed here, it is out of scope. This brief does not change `UX_STANDARDS.md`. Where a slice needs a rule applied differently, §6 states it as a proposed amendment that the product owner approves and the senior UX designer writes into `UX_STANDARDS.md`.

## 1. What the UI is for

| Goal | Source | Served by (slice) |
|---|---|---|
| J1: know before committing whether a design can deadlock or overflow | north-star J1 | Canvas, port binding, Check, Verdict panel (0) |
| J2: re-check a changed design in seconds | J2 | Check, removing verdicts with the reason "Behavior changed since the last check." (0); stale marking, undo restores verdicts, cancel (1) |
| J3: reproduce an incident as a counterexample | J3 | Replay on the canvas, the step list and the behavior model pane (0) |
| J4: size a capacity | J4 | Capacity and when full in the relationship inspector (0) and in the net editor (1) |
| J6: read structure and behavior later in git | J6 | Explicit Save of readable TOML, with comments kept (0) |
| **E6**: model one real design of about 10 elements to its first decisive verdict in ≤ 45 min (I1) | OST E6 | New project, fast element and relationship creation, port binding in the inspector, a Reload loop without conflict traps, Copy file path / Open file (0); graphical net editor (1); state machine editor (2) |
| **E7**: name the root cause of a Disproved from the replay alone in ≤ 10 min, in ≥ 2 of 3 cases (I3) | OST E7 | One action from the verdict to the replay; steps written in architecture terms; a violation sentence on the last step; tokens on relationships and in the behavior model pane, which follows the step (0); active state (2) |
| Guardrail: never claim a verdict the tool did not get | ADR 0008, design §4 | The keysFresh rule on every surface that shows a verdict, a "files not loaded" banner, Unknown always with a reason and never colored like success, replay closes when the model changes (0) |

J5 (scan) belongs to slice 5 and is out of scope for this brief.

## 2. Layout and surface inventory

**Layout [proposal; route: senior-ux-designer to confirm]:**
- One window: the header, the canvas, and one right-hand column with the tabs **Inspector | Verdicts**.
- Below the canvas sits a resizable bottom pane with the tabs **Behavior model | Problems**. It has a minimum height. Below about 700 px of window height it collapses to a header strip, and that strip stays one action away.
- The right-hand column collapses with a visible button and a shortcut. The canvas always keeps at least half the window width (WIN-1). The bottom pane takes height, not width.
- While a region is collapsed, the header shows a summary, for example "Deadlock-free: Disproved" with its icon and "2 problems". Choosing the summary opens the region.
- During replay the step list replaces the Verdicts tab content. At 1024 × 640, the canvas, the step list and the behavior model pane are visible at the same time without scrolling the window.
- Port handles are visible without hover (WIN-3).

| Surface | Slice 0 | Slice 1 | Slice 2 |
|---|---|---|---|
| Start screen | New project **[proposal]**, Open project | unchanged | unchanged |
| Header | Project name, view title, "Level: Container", unsaved marker, "Saving…", Save, Reload, Check, command palette button, collapsed-region summary | Undo, Redo | unchanged |
| Canvas (one view, Container level) | Create, move, rename and delete elements and relationships; keyboard path; several relationships between the same two elements; the error state; replay highlights | Disk-change announcements | unchanged |
| Inspector | Element; relationship with port binding **[proposal]**; nothing selected shows the scope and its properties | unchanged | Behavior model kind: Petri net or state machine |
| Behavior model pane | Read-only Petri net, stable automatic layout, ports marked, tokens during replay, Pin | Petri net editor | State machine editor, active state during replay |
| Verdict panel | One row per built-in property with the states in §3.2; "files not loaded" banner | Stale, Cancel, elapsed time | unchanged |
| Replay | Step controls, step list, highlights, violation sentence, read-only mode | Frames that name deleted elements are dimmed | Steps written as state changes |
| Problems list | Parse errors, dangling relationships, unbound ports, places without when full | unchanged | State machine problems |
| Status region | `role=status`, polite: Save results, internal errors, final verdicts | Disk adoptions | unchanged |
| Dialogs | Quit with unsaved changes; delete confirmation; Reload conflicts; Save conflicts and failures | Restore offer; disk conflicts | unchanged |
| Command palette (cmdk) | Every command and its shortcut, including "Go to element…" and "Add relationship…" | Undo, Redo, "Turn on local check log" | New commands |

### 2.1 Slice 0 surfaces in detail

**Start screen**
- "New project" asks for an empty folder and creates the software system and its Container view.
  - The software system's name comes from the folder name. It is renamed in the inspector when nothing is selected.
  - Folder not empty: "Choose an empty folder. <path> already contains files."
- "Open project" opens a folder. It has these states:
  - Loading: shows the project name and a progress indicator.
  - Not a project: "<path> is not a project: arch.toml is missing. Choose a project folder or create a new project."
  - Newer format version: "<path> was saved by a newer version of this app (format <n>). Update the app to open it." The project does not open.
  - Loads with problems: the project opens and the Problems list opens (ADR 0001).
  - Any other failure: "Could not open <path>: <cause>." followed by a next step (TXT-2).
- Empty state: the two actions and nothing else.

**Header, Save, Reload and quit**
- The unsaved marker is visible whenever savedRev ≠ rev (CNV-5). It stays visible until the `saved` event arrives.
- While a Save runs, the header shows "Saving…".
- Save results go to the status region:
  - Success: "Saved 3 files."
  - Partial failure: "Saved 2 files. Could not save elements/billing.toml: <cause>." The unsaved marker stays.
  - A conflict opens the conflict dialog (§2.2).
- Reload with no conflicts adopts the files and reports "Reloaded 2 files." A Reload that hits a parse error keeps the in-memory model (design §4) and starts the "files not loaded" state (§3.2).
- Quit with unsaved changes offers "Save and quit", "Quit without saving" and "Cancel". If "Save and quit" hits a conflict or a failure, the app does not quit and shows the conflict dialog or the failure.

**Canvas**
- Element kinds are Container, Database and Queue. Each has its own generic icon plus a label, and none has kind-specific attributes **[proposal]**. There are no cloud icons (ADR 0007). Kinds never differ by color alone (CNV-3).
- An element without a behavior model shows a "No behavior model" badge with an icon and text. Under ADR 0010 this element makes every property in the scope Unknown "model incomplete".
- Several relationships between the same two elements, including both directions, are drawn with an offset. Each has its own label. E5 case 1 needs 8 synchronous bindings among 5 elements.
- In copy, a relationship is named "From → To (meaning)", for example "Order API → Billing (calls)".
- Long element names are truncated on the canvas and shown in full in the inspector. No text is available only on hover (WIN-3).
- **Keyboard path (A11Y-3):**
  - Navigate: "Go to element…" in the palette (a combobox over element names). Tab moves to the next element.
  - Select with Enter. Rename with F2 or in the inspector.
  - Connect: with the source selected, choose "Add relationship…" in the palette or the inspector, then pick the target in a combobox.
  - Move: the arrow keys move the selected element by one grid step.
  - Delete: the Delete key, with confirmation where it applies (see the last bullet of this list).
- **Placement:** an element created from the keyboard or the palette goes to the next free grid slot near the viewport center. An element that arrives through Reload without a layout entry is placed by ELK, and only unplaced elements are placed that way. Its position is saved at the next Save.
- **States:**
  - Empty: "Add the first container", with its shortcut.
  - Ideal.
  - Error: the canvas keeps showing the last model that loaded. An element whose file has a parse error carries an error badge. A relationship with a dangling end (`danglingEnd`) is drawn with an open, dashed end and an error badge, and it is listed in the Problems list.
  - `large-model`: 300 elements in this one view **[proposal, §6]**.
  - Replay.
- Slice 0 has no undo. Deleting an element that has a behavior model therefore asks for confirmation and names the file that Save will delete.

**Inspector**
- *Element:* name, element kind, description, the ports it offers, plus "Copy file path" and "Open file" (tauri-plugin-opener) **[proposal]**. When the element has no behavior model, the copy depends on the element's state:
  - Saved and clean: "No behavior model. Add it in elements/<id>.toml, then choose Reload."
  - Not saved yet: "Save, then add a behavior model in elements/<id>.toml and choose Reload."
  - Saved, with unsaved changes: "This element has unsaved changes. Save before you edit elements/<id>.toml, or Reload will report a conflict." Relationships are stored in their source element's file (stack.md §4), so a port binding and a hand-written net share one file.
- *Relationship:* meaning, from port, to port, binding (synchronous or asynchronous) and, for an asynchronous binding, capacity and when full. When full defaults to overflow (ADR 0014). This is where block or drop is set for generated places. States:
  - The element has no ports: "Billing has no behavior model yet, so it offers no ports."
  - Incompatible port kinds: "<port> and <port> cannot be bound: <reason>."
  - Unbound port: "Unbound port: every property in this scope is Unknown until it is bound."
- *Nothing selected:* the scope element (renamable) and its properties, deadlock-free and no capacity exceeded. They use the same keysFresh selector as the Verdict panel.

**Behavior model pane (read-only)**
- Shows the selected element's Petri net with automatic layout (elkjs). The same net always gets the same layout: ELK model-order options, the previous positions as seeds, and a cache per net hash, computed in a worker. While the layout is computed the pane shows "Laying out…", and the canvas never waits for it. No layout is saved, because BehaviorLayout comes in slice 1.
- Each place shows its capacity and when full. Ports are marked, and the initial marking is visible.
- During replay the pane follows the element of the current step. A "Pin" toggle keeps it on one element.
- The pane is not a view, and opening it is not a drill down.

**Replay**
- "Replay counterexample" on a Disproved row shows step 1. The behavior model pane opens if it is collapsed.
- Controls: first, previous, next, last, a scrubber, and the arrow keys. In replay, the arrow keys step through the replay and never move elements.
- Replay is read-only. Edits on the canvas and in the inspector are disabled with "Exit replay to edit". A Reload, or any other change to the model, closes the replay and the status region says "Replay closed: the model changed."
- Step list and `aria-live` text, written in architecture terms:
  - A step inside one element: "Step 3 of 7: Billing: take_lock"
  - A synchronous step: "Step 4 of 7: Order API → Billing (calls): reserve"
  - An asynchronous send: "Step 5 of 7: Order API sends to Billing: 2 of 3 messages waiting"
  - An asynchronous receive: "Step 6 of 7: Billing receives from Order API: 1 of 3 messages waiting"
  - The last step: the violation sentence (§3.3).
- Highlights for the involved elements and relationships use an outline plus a marker, never color alone, with at least 3:1 contrast (A11Y-1).
- Exiting the replay returns to the selection the user had before. There is no animation in slices 0–2.

**Problems list.** One line per cause, with the file and the element, plus "Copy file path" and "Open file". Choosing a line selects the element or relationship. This is the feedback loop for nets written by hand in TOML.

**Status region.** A polite `role=status` area that never covers a focus stop (A11Y-2). It announces Save results, `internalError` (with "Copy details"), replay closing, and final verdicts ("Deadlock-free: Disproved"). It never announces pending re-evaluation.

**Theme.** Light and dark follow the OS setting **[assumption]**. prefers-color-scheme in WebKitGTK under WSLg is unverified (stack.md §1.10). If E3 shows that it does not follow the OS, add the palette commands "Theme: System / Light / Dark". There is no settings screen.

### 2.2 Conflict dialog (slice 0)
Save and Reload both report conflicts in one dialog, "Files changed on disk", with one row per file and a choice per file:
- **Keep my changes:** the in-app version is kept, and the next Save writes it, which replaces the version on disk. The row says: "Your edits in the file will be replaced at the next Save."
- **Use the file on disk:** the in-app changes to that element are discarded and the version on disk is loaded.
- "Cancel" leaves everything unchanged and nothing is written.

Nothing is overwritten without a choice in this dialog (CNV-5, design §4). Slice 1 uses the same dialog for disk changes that the watcher detects.

## 3. Verdict states and copy

### 3.1 Words
Use: Check, Verdict panel, Proved, Disproved, Unknown, Replay counterexample, Save, Reload, element, relationship, view, level, capacity, when full, marking.

Never use: run verification, job, passed, failed, verified, inconclusive, trace, simulation, animation, node, edge, diagram, workspace, layer, fires, gestures, keep mine, take disk.

### 3.2 States of a Verdict panel row
Each row names the property, its scope and its level (VRD-4). Every surface that shows a verdict, including the nothing-selected inspector, uses the same keysFresh selector.

| State (capture name) | When | Shows | Slice |
|---|---|---|---|
| Nothing to check yet (`no-behavior`) | The scope has no element with a behavior model, which includes a new, empty project **[route: senior-backend-dev]** | "Nothing to check yet: no element in this view has a behavior model." No verdict icon, and no check runs | 0 |
| No verdict yet (`no-verdict`) | No check has run for the current key | The property and a Check action. After a behavior edit removed a verdict: "Behavior changed since the last check." No verdict icon | 0 |
| Pending re-evaluation (`pending`) | Keys are not fresh for the displayed revision (design §4) | Fixed row height and position. The property and scope stay, and a neutral placeholder replaces only the icon and the verdict text. Nothing goes to `aria-live`. After about 1 s: "Updating checks…". It never shows the old verdict and is never colored like success | 0 |
| Checking (`running`) | Running | A progress indicator; elapsed time and Cancel from slice 1 | 0 / 1 |
| Proved (`proved`) | Current key, Proved | Icon plus "Proved"; "Checked all 1,970 reachable markings; re-checked by a second explorer"; assumptions, for example relationships that bind no ports; "Holds for every order of steps with the capacities and initial tokens in this model. Time and rates are not modeled." (E5 implications 5 and (b)) | 0 |
| Disproved (`disproved`) | Current key, Disproved | Icon plus "Disproved", the violation sentence and "Replay counterexample". On deadlock-free, when no final predicate is defined and the run ended with no token in any pre set (NS §9.6), slice 0 adds: "No transition is enabled and no final predicate (done when) is defined, so this run stops here. If this is where the run is meant to end, done when will let you say so; it comes with slice 1. Until then, a run that finishes counts as a deadlock." From slice 1 the last sentence becomes "…define done when." with a "Define done when" action. It never advises looping back to the initial marking (D-23, D-56) | 0 / 1 |
| Unknown (`unknown`) | See §3.4 | Icon plus "Unknown", the reason and a next step. Never green and never a check mark (VRD-1, VRD-3) | 0 |
| Stale (`stale`) | Keys are fresh but the verdict's key no longer matches | The previous verdict marked "Stale" and visually subordinate. Undo makes it current again | 1 |

**Files not loaded (`files-not-loaded`):** while a parse error is outstanding, a banner above the rows reads: "Verdicts are for the last version that loaded. elements/billing.toml has an error (line 12: <cause>). Fix it and choose Reload." The banner links to the Problems line. Slice 0.

In slice 0 a stale verdict is removed rather than marked (`v1-slices.md:24`). A layout-only edit keeps the verdict current, after a short pending state. A deadlock with 10 or more waiting lines scrolls inside its row.

### 3.3 Violation sentences (on the last replay step and on the Disproved row)
- **Deadlock:** "Deadlock: no step is possible." Then one line per waiting step: "Billing waits at take_lock for a token in lock_b." The marked places are shown in the behavior model pane. "Held by <element>" is not shown in slices 0–2, because `Violation::Deadlock` does not carry it (design §3).
- **Capacity exceeded:**
  - On a generated place: "Order API → Billing (publishes to) holds 4 messages; capacity 3."
  - On a place inside a behavior model: "Billing: queue holds 4 tokens; capacity 3."
  - Both name the place and its capacity (`v1-slices.md:20`).
- Proved and Disproved rows carry no overflow caveat. Under ADR 0014, a Disproved is reached before any overflow, and a Proved means that no overflow marking is reachable. The caveat belongs only to Unknown "capacity exceeded first" (§3.4).

### 3.4 Unknown reasons in slice 0

| Reason | Text must say | Next step |
|---|---|---|
| budget exhausted | How many markings were explored and how long it took, and whether the main explorer or the re-check ran out | "Lower capacities or initial tokens, or remove elements from this view." |
| capacity exceeded first | "Deadlock-free was decided only up to the first time <relationship or place> exceeds capacity 3." | For a generated place: set when full to block or drop in the relationship inspector, or raise the capacity. For a place inside a behavior model: the same in elements/<id>.toml |
| model incomplete | One line per cause, each of which selects its element or relationship | Unbound port: bind it in the relationship inspector. Dangling relationship: fix it on the canvas. Missing behavior model or missing when full: edit elements/<id>.toml, then Reload |
| property cannot be checked | "Property cannot be checked:" and the detail lines; in slice 0, for kinds other than deadlock-free and no capacity exceeded: "This kind of property is checked from slice 4." (D-25, D-54) | None in slice 0 beyond the text |
| internal disagreement | "The two explorers disagreed about this model, so the tool gives no verdict. This is a bug in the tool, not in your design." | "Copy details", plus the log folder |
| internal error | "This is a bug in the tool, not in your design." | The path of the local crash log (ADR 0006/0012), plus "Copy details" |

"cancelled" moves to slice 1 (§4). A cancellation caused by a key change belongs to an old key and is never shown (design §4).

### 3.5 When checks run **[proposal]**
- In slice 0 a check runs automatically in only one case: opening a project queues every property (ADR 0005). Every other check starts when the user chooses Check.
- From slice 1, a key that undo restores and that has no cached verdict is resubmitted automatically (ADR 0013, design §4).
- After slice 1, decide from the journal whether checks should run automatically.

## 4. Slice 1 additions (draw and change behavior)
- **Done when** (D-05): "Define done when" on a deadlock-free row, in the inspector of a scope element and in the palette. The form offers a conjunction of `place = n`; hand-written `[final]` text may be any predicate. Help: "Done when says which markings count as proper termination; a run that stops there is not a deadlock." Editing it stales only deadlock-free of that behavior. In slice 0 a `[final]` table is a warning and checks run without it (D-25).
- **Petri net editor** in the behavior model pane, which can grow to a large split. It edits places with capacity, transitions, arcs, ports and the initial marking, and it saves layout as BehaviorLayout. A place cannot exist without a when full choice (ADR 0014). Replay highlights also appear in the editor.
- **Undo and redo:** header buttons, shortcuts and palette entries. Ctrl+Z is ignored while focus is in a text input. Delete no longer asks for confirmation.
- **Restore offer:** "Restore 3 unsaved changes" or "Open without restoring". After a crash during restore, the offer is only "Open without restoring" (design §4). A journal that is set aside gets its own message.
- **Stale** (§3.2), **Cancel** (Unknown "cancelled": "You cancelled this check." with the next step "Check again", within 1 s), and **elapsed time**.
- **Disk changes** are announced within about 1 s. A clean element is adopted and the status region says so. A dirty element opens the §2.2 dialog.
- **Windows build** with the same surfaces. **Local check log:** one palette toggle, off by default.

## 5. Slice 2 additions (state machines)
- In the inspector, the behavior model kind is Petri net or state machine. The state machine editor lives in the same pane and edits flat states, transitions labeled with events, and the initial state. Events are its ports.
- Replay shows the active state on the element and in the pane. Steps read as state changes ("Breaker: Closed → Open on failure"), never as place names (`v1-slices.md:45`).
- The Problems list adds state machine causes, such as an unreachable state or an event that is not bound.

## 6. How UX_STANDARDS applies per slice **[proposal: amendment to UX_STANDARDS.md; approver: product owner; editor: senior-ux-designer]**

| Rule or state | Slices 0–2 | From |
|---|---|---|
| Capture `large-model` | 300 elements in the one view (the per-view target) | ≥ 1,000 elements in the project from slice 3 |
| CNV-1 | The level part ("Level: Container") applies from slice 0 | Breadcrumb and go up from slice 3 |
| CNV-2 | Not applicable (no drill down) | Slice 3 |
| CNV-5 | Unsaved marker: full. Disk changes are announced at Save and at Reload and never overwritten, because Save re-hashes every file (design §4) | Watcher announcement within about 1 s from slice 1 |
| A11Y-3 | Create, select, connect, rename, delete, move; undo from slice 1 | Drill down and go up from slice 3 |
| VRD-4 stale marking | Slice 0 removes the verdict, so no stale verdict is ever shown | Slice 1 |
| VRD-5 | Not applicable | Slice 5 |
| VRD-6 | Not applicable | Slice 1 |

Until the amendment is approved, every slice 0 review gets this table with its brief.

## 7. UX success criteria

| # | Criterion | How it is measured | Slice |
|---|---|---|---|
| U1a | Every state and every piece of copy in this brief for the slice renders correctly | Playwright per state on Rust-written fixtures (capture matrix below) | 0, 1, 2 |
| U1b | Every acceptance criterion in `v1-slices.md` for the slice can be shown in the real app on the seed projects | One tauri-driver/WebdriverIO scenario per criterion, on Linux | 0, 1, 2 |
| U2 | 0 verdicts shown as current while keys are not fresh or do not match, on every surface that shows a verdict | Vitest keysFresh test with Rust-written fixtures | 0 |
| U3 | 100 % of Unknown rows in the seed set show a reason and a next step, and none uses a success color or icon. There is one seed per reason, plus an empty project | UX review against VRD-1 and VRD-3 | 0 |
| U4 | One action from a Disproved row to the first replay frame; every step can be reached by keyboard; replay closes when the model changes | Scenario test | 0 |
| U5 | **E7:** in ≥ 2 of 3 Disproved verdicts on real designs, the user names the root cause from the replay within 10 min without opening TOML | Dogfooding journal | 0 (re-measure in 2) |
| U6 | **E6 baseline:** I1 for one real design of about 10 elements, with nets written in TOML; the number of Reload conflicts is logged | Journal, minutes | 0 |
| U7 | **E6 decision:** I1 ≤ 45 min | Journal; if it fails, the pattern library moves before slice 2 (D-57) | 1 **[decided]** |
| U8 | After a TOML edit, Reload shows the new net or a problem naming the file and its cause within 1 s for a 10-element project **[assumption]** | Scenario test | 0 |
| U9 | 0 lost-work incidents: quitting with unsaved changes always asks, "Save and quit" never quits on a conflict, and Save never writes over a file that changed on disk without a "Keep my changes" choice | Crash and conflict tests, journal | 0, 1 |
| U10 | Gesture to painted frame p95 < 50 ms; frame time p95 < 33 ms during a scripted pan and zoom over the 300-element seed | E4 harness on WebKitGTK | 0 |
| U11 | 0 open must findings at 1024 × 640 and 1440 × 900, light and dark, for the surfaces in the slice | UX review on the capture matrix | 0, 1, 2 |
| U12 | Cancel ends as Unknown "cancelled" within 1 s; undo after a capacity change shows the earlier verdict as current again | Scenario test | 1 |
| U13 | A layout-only edit is pending for < 100 ms p95 at 300 elements **[assumption]** | E4 harness | 0 |

**Capture matrix (slice 0):**
- Canvas: `empty`, `ideal`, `error`, `large-model`, `replay`.
- Verdict panel: `no-behavior`, `no-verdict`, `pending`, `running`, `proved`, `disproved`, `unknown` (one capture per reason), `files-not-loaded`.
- Dialogs: `conflict`, `quit-unsaved`, `delete-confirm`.
- Start screen: `empty`, `error`.

Slice 1 adds `stale`, `restore` and `cancelled`. Slice 2 adds `replay` with a state machine. Each is captured at 1024 × 640 and 1440 × 900, in light and dark.

## 8. UI non-goals (slices 0–2)
- Drill down, breadcrumb, other levels, and external persons or systems in the view (slice 3). Property editor, user properties, and no dead transitions (slice 4). Done when arrives in slice 1 (§4). Scan, findings and suppressions (slice 5).
- Token animation, play speed, and a generated sequence view of a counterexample. A per-step explanation is pulled forward only if E7 fails.
- "Held by <element>" in deadlock sentences, unless the backend adds it as checked evidence.
- Budget settings, a settings screen, custom themes, onboarding tours, and sample-project galleries.
- A command that lays out the whole view automatically; a minimap **[proposal]**; a search or filter panel beyond "Go to element…" **[proposal]**; copy and paste; a pattern library (it moves before slice 2 if the E6 decision at slice 1 fails; D-57).
- Native menu bar, multiple windows or tabs, a recent-projects list **[proposal]**, and reopening the last project on launch **[proposal]**.
- Image export; PNML, Structurizr and LikeC4 import and export; polish for presenting to stakeholders; cloud provider icons (ADR 0007).
- macOS-specific UI, and anything networked: no accounts, no telemetry, no update checks.

## 9. Open questions
1. **User:** approve the §6 amendment to UX_STANDARDS.md (`large-model` = 300 per view until slice 3, the per-slice table, and the CNV-5 exception for slice 0)? Recommendation: yes.
2. **User:** CNV-5 in slice 0: is "announced at Save and Reload" acceptable? Recommendation: yes; re-open only if the journal shows a missed hand edit.
3. **User:** New project, port binding in the inspector, and the Container, Database and Queue kinds in slice 0? Recommendation: yes to all three.
4. **User:** checks run on Check in slice 0, apart from the open-project queue (§3.5)? Recommendation: yes.
5. **Closed (D-57, 2026-11-02):** the E6 decision rule is applied at slice 1, and slice 0 gives the baseline (U6, U7). If E6 fails, the pattern library moves before slice 2; `opportunity-solution-tree.md` E6 matches.
6. **senior-backend-dev:** lowering a scope with no behavior model returns a typed reason (`no-behavior`) instead of an empty net, which is Disproved deadlock-free today. Can a conflict be detected per entity table instead of per file, so that a port binding and a hand-written net in the same file do not conflict?
7. **software-architect:** keep ADR 0010 (an element without a behavior model makes the scope "model incomplete")? Recommendation: keep it for slice 0 and decide from the E6 journal.
8. **senior-ux-designer:** confirm the one-column layout with tabs and the bottom pane (§2).
9. **User / glossary:** add "Verdict panel", "Problems list", "problem", "pending re-evaluation" and "behavior model pane" to GLOSSARY.md.
