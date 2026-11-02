# Tracewright – architecture modeling and verification

The language of Tracewright, a local tool in which one developer models a software system across drill-down levels, attaches behavior to its parts, proves properties of that behavior, and scans the architecture for anti-patterns where a proof is not possible. US English throughout. As of 2026-11-02 (applies `docs/specs/decisions.md` D-01 to D-59). Only words a user or a domain expert uses belong here; code names stay in the specs.

## Modeling

**Project**:
A folder, usually inside a git repository, that holds one model and everything the tool stores about it.
_Avoid_: Workspace, repo, model directory

**Model**:
The complete description of one software system: its elements, relationships, behavior models and properties.
_Avoid_: Diagram, model file, workspace

**Element**:
One thing in the model, such as a person, software system, container, component, code element or infrastructure part. Every element sits at exactly one level.
_Avoid_: Node, shape, box, cell, object

**Element kind**:
What sort of element something is, for example Container, Queue, Database or AWS Lambda. The kind decides the icon and which attributes the element has.
_Avoid_: Type, palette type, cloud product

**Child**:
An element whose parent is a given element; the view of an element shows its children. In a valid project Persons and Software Systems have no parent, so they are never children; the tool goes by the stated parent, never by the kind.
_Avoid_: Sub-element, member, nested element

**ID**:
The stable name of an element, relationship, view or property in the files and in predicates: lowercase, starts with a letter, at most 48 characters, for example `order-api`. It is created once and never changes when the display name changes. Names inside a behavior model keep their case.
_Avoid_: Key, handle, name (that is the display name)

**Relationship**:
A directed connection from one element to another with a meaning, for example "calls synchronously" or "publishes to". It can hold bindings and carries attributes such as interaction and timeout.
_Avoid_: Edge, relation, link, arrow, dependency

**Level**:
One of the four C4 abstraction levels: System Context, Container, Component, Code.
_Avoid_: Layer, zoom level, tier

**View**:
What the canvas shows at one time: the inside of one element, or the system landscape at the top. Every view belongs to exactly one level.
_Avoid_: Diagram, page

**Drill down**:
Opening the view of an element's inside, one level lower. The reverse is **go up**.
_Avoid_: Zoom in, expand, enter

**Layout**:
Where elements and relationships are drawn in a view. Layout never changes the meaning of the model.
_Avoid_: Diagram, arrangement

## Behavior

**Behavior model**:
A Petri net or state machine attached to an element that describes how the element acts over time. An element has at most one.
_Avoid_: Behaviour, process model, workflow, dynamic model

**Petri net**:
A behavior model made of places that hold tokens and transitions that move tokens. In this tool every place has a capacity.
_Avoid_: Net graph, flow

**Capacity**:
The largest number of tokens a place may hold, for example the length of a queue or the size of a connection pool. Every place also says what happens when it is full.
_Avoid_: Bound, limit, size

**When full**:
What a place does at its capacity: **block** (the sender waits; back-pressure), **drop** (the token is discarded; load shedding) or **overflow** (exceeding the capacity is a violation). Generated places overflow unless the user says otherwise; places inside a Petri net always state it; the places of a state machine's states overflow.
_Avoid_: Policy, overflow mode, backpressure flag

**Marking**:
How many tokens each place holds at one moment; the state of a Petri net. Every Petri net has an **initial marking**. A marking that satisfies the final predicate is **final**.
_Avoid_: State (when talking about Petri nets), configuration, final markings (as a list)

**Reachable marking**:
A marking the net can reach from its initial marking by firing transitions without passing through an overflow marking. The count of reachable markings is what a Proved by exhaustive exploration reports.
_Avoid_: State, state space, state count

**Overflow marking**:
A marking in which a place holds more tokens than its capacity. It is reached by exactly one firing into an overflow place from an in-bounds marking, violates "no capacity exceeded", and nothing happens after it: it is neither a deadlock nor final. Only "no capacity exceeded" and "never p" look at its counts.
_Avoid_: Error state, bad marking, terminal state

**Deadlock**:
A reachable marking in which no transition is enabled and that is not final. Deadlock is global: the whole scope stops. Parts that are stuck while others go on are what "always reachable" catches.
_Avoid_: Hang, freeze, lockup

**Run**:
A sequence of firings from one marking to another; the formal object inside a counterexample. Not every run is a counterexample.
_Avoid_: Firing sequence, trace, path

**Step**:
One fired transition together with the marking after it; a counterexample is a list of steps from the initial marking. Replay shows a step of a state machine as a state change.
_Avoid_: Frame (reserved for the rendered replay), move, tick, event

**Dead transition**:
A transition that is enabled in no reachable marking.
_Avoid_: Unreachable transition, unused transition

**Port**:
A place or transition that a behavior model offers to the outside. A **transition port** is what relationships bind. A **place port** is never bound in v1; it exists so that a property or final predicate of the parent scope can name it. For a state machine, an input or output event is a transition port standing for every transition on it, and every state is a place port; internal events are not ports. Relationships connect to ports, never to the inside of another element's behavior model.
_Avoid_: Interface, pin, endpoint, hook, input/output port, event port

**Binding**:
One pair of ports that a relationship connects, with its own direction. Its mode is **synchronous** (the two transitions fire as one fused transition; the caller side is one end, the callee side the other) or **asynchronous** (messages wait in the relationship's generated place; the sender side is one end, the receiver side the other). A relationship holds zero or more bindings, at most one of them asynchronous; a call and its reply are two bindings of one relationship. A transition port has one role: caller, callee or member of one asynchronous binding. The mode is not the relationship's interaction.
_Avoid_: Link, wire, connection, port mapping

**Fused transition**:
The one transition a synchronous binding makes from a caller transition and a callee transition; the two bound ports no longer fire on their own.
_Avoid_: Merged transition, joint step, synchronized transition

**Generated place**:
The place that an asynchronous binding adds between the ports of two elements; it stands for the messages in transit and holds them as a bag, without order. It belongs to the relationship.
_Avoid_: Channel, buffer, queue place

**Qualified ID**:
The string that names one place or transition in a check: `element/name` for an element's own place, transition or state, `relationship/caller+callee` (bare names) for a fused transition, and `relationship` for a generated place. Places, transitions, states and events of one element share one namespace.
_Avoid_: Path, key, address

**Linear marking predicate**:
A statement about one marking: sums of token counts with whole-number weights (negative allowed), compared with whole numbers and combined with and, or and not, for example `billing/inflight + shipping/inflight > 2`. Short form: **predicate**. User properties and final predicates are written as predicates.
_Avoid_: Condition, formula, query, expression, constraint, guard

**Final predicate**:
The predicate of a scope element that says which markings count as proper termination, one for its own behavior and one for its combined behavior. It is checked as part of deadlock-free: a marking without enabled transitions that satisfies it is not a deadlock. The UI label is "done when". A state machine's final states contribute to it. Overflow markings are never final. From slice 1.
_Avoid_: Final markings (as a list), final condition, end state, goal, accept condition, exit condition, done condition, final state (a state machine term)

**State machine**:
A behavior model made of states and transitions between them, triggered by events. The tool turns it into a Petri net before checking it.
_Avoid_: Statechart, FSM, state-machine, statemachine

**Event**:
Something that makes a state machine take a transition. Every event has a direction: input, output or internal.
_Avoid_: Signal, message (for the event itself), trigger

**Input event**:
An event that something outside the element causes: a call the element accepts or a message it receives. A transition port. A message that arrives while no transition on its event is possible waits in the generated place.
_Avoid_: Inbound port, receive event

**Output event**:
An event by which the element calls something or sends a message. A transition port.
_Avoid_: Outbound port, send event

**Internal event**:
An event that happens inside the element without a partner, for example a timer expiring or a failure. Not a port.
_Avoid_: Private event, spontaneous transition

**Initial state**:
The one state a state machine is in before anything happens.
_Avoid_: Start state, default state

**Final state**:
A state in which stopping counts as proper termination; it contributes to the scope element's final predicate.
_Avoid_: End state, accepting state

**Active state**:
The state a state machine is in at one moment; replay shows it per step.
_Avoid_: Current state, mode (a binding's mode)

**State machine transition**:
A step from one state to another on one event. It becomes one or more Petri net transitions when the state machine is compiled (one per partner in a synchronous binding).
_Avoid_: Edge, arc

## Checking

**Property**:
A statement that the tool tries to prove about the behavior inside one element, the property's **scope**. The kinds are deadlock-free, no capacity exceeded and no dead transitions (built in), and never p and always reachable p (user properties, written as predicates or through property templates). New project stores deadlock-free and no capacity exceeded on the system element.
_Avoid_: Requirement, spec, assertion, invariant

**Own scope / combined scope**:
The two kinds of behavior a property of a scope element can be about. Own scope: the element's own behavior model, with its ports firing freely. Combined scope: the behavior of its children, combined through their bindings; a predicate there may name only place ports of the children and generated places.
_Avoid_: Scope kind, local scope, system scope, level scope

**Property template**:
A named sentence with blanks that the user fills in to state a property without writing a predicate, for example "Never holds the whole pool". Filled in, it expands to exactly one property and adds no meaning of its own; the tool expands it again from the current model at every check, so values such as a capacity follow edits. A **preset** is one fixed choice inside a template, for example "its initial marking". A **precondition** is what a template needs from the model; when it fails, the verdict is Unknown "property cannot be checked".
_Avoid_: Pattern, query, formula, macro, snippet, rule

**Pool**:
A place whose tokens stand for interchangeable resources that steps take and give back, such as threads, connections or permits. It usually starts full.
_Avoid_: Semaphore, resource place, free list

**Held by**:
The places that count as one holder's share of a pool in a "never holds the whole pool" property. Tokens in other places are not counted.
_Avoid_: Owned by, allocated to, in use by

**Vacuity guard**:
From slice 4, "no dead transitions" runs next to user properties and every Proved shows how many reachable markings it covered, so a Proved that holds only because almost nothing can happen is noticed.
_Avoid_: Sanity check, coverage

**Check**:
One attempt to prove one property of its scope. Every check ends with exactly one verdict.
_Avoid_: Verification run, analysis, job, test

**Verdict**:
The outcome of a check: Proved, Disproved or Unknown. There are no other verdicts.
_Avoid_: Result, status, pass/fail

**Proved**:
The property holds for every possible behavior of the model, and the tool has re-checked the evidence for it.
_Avoid_: Passed, verified, green, Proved-bounded

**Disproved**:
The property does not hold. The tool has either a counterexample that shows it or, for no dead transitions, an exhaustive exploration that the second explorer confirmed.
_Avoid_: Refuted, failed, violated

**Unknown**:
The tool could not decide the property. An Unknown verdict always carries one reason from a closed list of seven: model incomplete, property cannot be checked, internal disagreement, internal error, cancelled, budget exhausted (also "budget exhausted while re-checking") and capacity exceeded first.
_Avoid_: Inconclusive, open, partial, bounded-proved, not checkable (reserved for scan rules)

**Nothing to check**:
Said of a scope in which no child has a behavior model (or, for an own scope, whose element has none). Its properties have no verdict at all, not even Unknown.
_Avoid_: Empty verdict, n/a, skipped

**Technique**:
How a Proved was obtained, named on every Proved: **exhaustive exploration** (every reachable marking was visited and a second explorer confirmed it; the panel shows how many) or a certificate.
_Avoid_: Method, strategy, mode

**Evidence**:
What the tool re-checked before it gives a Proved or Disproved: a certificate that the independent checker accepted, an exhaustive exploration that a second, independent explorer confirmed, or a replayed counterexample. For always reachable, a replayed counterexample plus a confirmed exhaustive exploration from its last marking.
_Avoid_: Proof, witness, trace

**Certificate**:
A small object that proves a property without exploring and that the independent checker checks on its own composition of the behavior models. In v1 only "no overflow place", which proves no capacity exceeded and names the drop places; P-invariant bounds come later.
_Avoid_: Proof object, witness

**Counterexample**:
A run from the initial marking, shown as steps, that ends in a violation of a property. It has zero steps when the initial marking already violates the property.
_Avoid_: Trace, witness, firing sequence, scenario

**Replay**:
Stepping through a counterexample on the canvas, with the involved elements highlighted.
_Avoid_: Animation, simulation, playback

**Assumption**:
A relationship of a scope that takes no part in a check because it binds no ports or leads outside the scope; every verdict of the scope lists them.
_Avoid_: Ignored relationship, exclusion, precondition (a template term)

**Closed-port note**:
The line on every own-scope verdict that says every port of the element can fire at any time, because in its own scope no partner holds a port back. It is not an assumption.
_Avoid_: Assumption, open-world note

**Budget**:
The time and memory a check may use. A check that exhausts its budget ends as Unknown.
_Avoid_: Timeout (a relationship attribute), bound, limit

**Current**:
Said of a verdict or finding that was computed for exactly the behavior the model now describes. Layout changes never make a verdict stale.
_Avoid_: Valid, fresh, up to date

**Pending re-evaluation**:
Said of a verdict while the tool works out, after an edit, whether it is still current. It is shown neither as current nor as stale and never as success.
_Avoid_: Loading, refreshing, checking (a check is running)

**Stale**:
Said of a verdict or finding that was computed for an earlier version of the model whose behavior differs from the current one. Editing the final predicate makes only deadlock-free verdicts and "can always get back to: done" verdicts of that behavior stale, because both read it.
_Avoid_: Outdated, old, invalid

## Scanning

**Anti-pattern**:
A known harmful structure in an architecture, such as a dependency cycle or a single point of failure.
_Avoid_: Antipattern, smell, bad practice

**Rule**:
A named description of one anti-pattern that the tool can look for in a model, for example RUN-SYNC-CYCLE.
_Avoid_: Check, inspection, smell, lint

**Scan**:
Applying all rules to the model. A scan produces findings, never verdicts.
_Avoid_: Analysis, lint, inspection, check

**Finding**:
One place where a rule matched: the rule, a severity, the affected elements and the evidence, for example an example cycle.
_Avoid_: Issue, violation, warning, smell, problem (a model error)

**Severity**:
How serious a finding is: High or Medium.
_Avoid_: Level (a C4 level), priority, error/warning (those grade problems)

**Example cycle**:
The one shortest cycle that a cycle finding shows, step by step, with the relationship behind each step.
_Avoid_: Witness, trace, path

**Not checkable**:
Said of a rule that could not be applied to a subject (an element, a relationship, a set of elements or the whole model) because the model lacks the data it needs, for example a timeout that is not stated. Each case is a Not checkable entry that names the missing data. Not the Unknown reason "property cannot be checked".
_Avoid_: Unknown (reserved for verdicts), n/a, skipped, open

**Not stated**:
The value of an attribute that the model does not give. A rule never treats it as yes, no or a number; it leads to Not checkable when stating it could change the outcome.
_Avoid_: Unknown, null, default, n/a

**Suppression**:
The user's decision about a finding, with a written reason, a date and a disposition: **accepted** (the finding is real and the user accepts it) or **not real** (the rule is wrong here). A suppressed finding stays visible as suppressed.
_Avoid_: Ignore, waiver, exception, baseline

**Interaction**:
Whether the dependent end of a relationship waits for the other end's answer before it goes on: synchronous or asynchronous. Stated by the user; bindings never set it, and it is not a binding's mode.
_Avoid_: Sync flag, call type, mode (a binding's mode)

**Timeout**:
A relationship attribute: the dependent end gives up waiting after a set time, on both connection and request. Word sense: "timeout" means only this attribute, as in the rule REL-NO-TIMEOUT and its findings. It never means a check that ran out of budget (that is Budget), and a state machine event the user names "timeout" is an internal event, not this term.
_Avoid_: Deadline, budget

**Runtime unit**:
An element that runs or stores data on its own: a Container, Database, Queue, or a Software System without children. Components and code elements run inside their container.
_Avoid_: Process, deployable, node, service

**Service (attribute)**:
The name of the service a Container belongs to; containers of one Software System with the same service count as one service.
_Avoid_: Owner, team

## Screen

**Verdict panel**:
The panel (the Verdicts tab) that lists the properties of the scope in view, each with its verdict, or why it has none (nothing to check, no verdict yet, pending re-evaluation, checking), and from which a counterexample is replayed.
_Avoid_: Verify panel, results panel, checks panel, test panel

**Problems list**:
The Problems tab of the bottom pane: all current problems, one line per cause with its file and element; choosing a line selects what it names.
_Avoid_: Error list, issues panel, console, log

**Problem**:
Something in the files or the model that stops a file from loading or a scope from being checked, for example a parse error, a dangling relationship, an unbound port, a place without when full or two names that clash. A problem is **fatal** (the project does not open, for example a newer format version), an **error** (it blocks the file, scope or property it touches), or a **warning** or **info** (shown, nothing blocked). A problem is never a scan finding and never a verdict.
_Avoid_: Issue, finding, violation, diagnostic

**Dangling reference**:
A reference to an element, port, place, state or relationship that does not exist, for example after a delete or a typo in a file. In a relationship or binding it is a problem that blocks the scope (model incomplete); in a property or final predicate it makes only that property Unknown "property cannot be checked".
_Avoid_: Broken link, orphan, missing target

**Behavior model pane**:
The Behavior model tab of the bottom pane below the canvas. It shows the selected element's behavior model with its ports, capacities and the tokens of the current replay step, and follows the steps of a replay unless the user turns "Follow steps" off. It is read-only in slice 0 and holds the editor from slice 1. It is not a view, and opening it is not a drill down.
_Avoid_: Behavior view, net view, net panel, inspector

## Files

**Format version**:
The integer `format` at the top of every project file. Older files are migrated when they are read and upgraded when they are saved; a file of a newer format version is refused.
_Avoid_: Schema version, file version, project version

**Recovery journal**:
The app's own record of unsaved changes, kept in the app data folder and never in the project, from which those changes can be restored after a crash.
_Avoid_: Autosave, backup, undo log, history

## Code

**Code import**:
Deriving elements and relationships of the model from source code.
_Avoid_: Reverse engineering, extraction, sync

**Conformance**:
Whether the code still matches the model: the same elements and relationships, and only behavior the model allows.
_Avoid_: Sync, consistency, drift (drift is the lack of conformance)

**Code proof**:
A proof about the source code itself rather than about a behavior model, either up to a bound or in full.
_Avoid_: Code verification, formal verification (too broad)

## Relationships

- A **project** holds one **model**. A model holds **elements**, **relationships**, **behavior models** and **properties**.
- An **element** sits at one **level**; drilling down into it opens a **view** one level lower that shows its **children**.
- An **element** has at most one **behavior model**, which offers **ports**. A **relationship** between two elements holds **bindings** of their ports, which combines their behavior; its **interaction** and **timeout** are separate attributes for the scan.
- A **property** has one **scope** element and is about its own scope or its combined scope. A scope element has one **final predicate** per behavior, which deadlock-free uses.
- A **check** proves one **property** and ends with one **verdict**. A Disproved verdict has a **counterexample**, which can be **replayed**, except for no dead transitions, whose evidence is a confirmed exhaustive exploration.
- A **problem** blocks a file or a scope; a **finding** comes from a **scan**. Neither is a verdict.
- A **scan** runs **rules** over the whole model and yields **findings** and Not checkable entries; a finding can be **suppressed**. Scans run independently of checks.
- A verdict or finding becomes **stale** when the model changes after it was computed; until the tool knows, a verdict is **pending re-evaluation**.
