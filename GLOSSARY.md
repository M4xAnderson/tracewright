# petrinet – architecture modeling and verification

The language of a local tool in which one developer models a software system across drill-down levels, attaches behavior to its parts, proves properties of that behavior, and scans the architecture for anti-patterns where a proof is not possible. US English throughout.

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

**Relationship**:
A directed connection from one element to another with a meaning, for example "calls synchronously" or "publishes to".
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
A Petri net or state machine attached to an element that describes how the element acts over time.
_Avoid_: Behaviour, process model, workflow, dynamic model

**Petri net**:
A behavior model made of places that hold tokens and transitions that move tokens. In this tool every place has a capacity.
_Avoid_: Net graph, flow

**Capacity**:
The largest number of tokens a place may hold, for example the length of a queue or the size of a connection pool. Every place also says what happens when it is full.
_Avoid_: Bound, limit, size

**When full**:
What a place does at its capacity: **block** (the sender waits; back-pressure), **drop** (the token is discarded; load shedding) or **overflow** (exceeding the capacity is a violation). Generated places overflow unless the user says otherwise; places inside a behavior model always state it.
_Avoid_: Policy, overflow mode, backpressure flag

**Overflow marking**:
A marking in which a place holds more tokens than its capacity. It violates "no capacity exceeded" and nothing happens after it.
_Avoid_: Error state, bad marking

**Marking**:
How many tokens each place holds at one moment; the state of a Petri net. Every Petri net has an **initial marking**, and may have **final markings** that count as proper termination.
_Avoid_: State (when talking about Petri nets), configuration

**Generated place**:
The place that an asynchronous relationship adds between the ports of two elements; it stands for the messages in transit. It belongs to the relationship.
_Avoid_: Channel, buffer, queue place

**Port**:
A place or transition that a behavior model offers to the outside. Relationships connect to ports, never to the inside of another element's behavior model.
_Avoid_: Interface, pin, endpoint, hook

**State machine**:
A behavior model made of states and transitions between them, triggered by events. The tool turns it into a Petri net before checking it.
_Avoid_: Statechart, FSM, state-machine, statemachine

## Checking

**Property**:
A statement that the tool tries to prove about the behavior inside one element, for example "never deadlocks" or "the queue never overflows". That element is the property's **scope**: its own behavior model, or the combined behavior of the children shown in its view.
_Avoid_: Requirement, spec, assertion, invariant

**Check**:
One attempt to prove one property of one behavior model. Every check ends with exactly one verdict.
_Avoid_: Verification run, analysis, job, test

**Verdict**:
The outcome of a check: Proved, Disproved or Unknown. There are no other verdicts.
_Avoid_: Result, status, pass/fail

**Proved**:
The property holds for every possible behavior of the model, and the tool has re-checked the evidence for it.
_Avoid_: Passed, verified, green, Proved-bounded

**Disproved**:
The property does not hold, and the tool has a counterexample that shows it.
_Avoid_: Refuted, failed, violated

**Unknown**:
The tool could not decide the property. An Unknown verdict always carries a reason, for example "budget exhausted", "no counterexample up to depth 40" or "cancelled".
_Avoid_: Inconclusive, open, partial, bounded-proved

**Evidence**:
What the tool re-checked before it gives a Proved or Disproved: a certificate, an exhaustive exploration that a second, independent explorer confirmed, or a replayed counterexample.
_Avoid_: Proof, witness, trace

**Counterexample**:
A sequence of steps from the start that ends in a violation of a property.
_Avoid_: Trace, witness, firing sequence, scenario

**Replay**:
Stepping through a counterexample on the canvas, with the involved elements highlighted.
_Avoid_: Animation, simulation, playback

**Budget**:
The time and memory a check may use. A check that exhausts its budget ends as Unknown.
_Avoid_: Timeout, bound, limit

**Current**:
Said of a verdict or finding that was computed for exactly the behavior the model now describes. Layout changes never make a verdict stale.
_Avoid_: Valid, fresh, up to date

**Stale**:
Said of a verdict or finding that was computed for an earlier version of the model whose behavior differs from the current one.
_Avoid_: Outdated, old, invalid

## Scanning

**Anti-pattern**:
A known harmful structure in an architecture, such as a dependency cycle or a single point of failure.
_Avoid_: Antipattern, smell, bad practice

**Rule**:
A named description of one anti-pattern that the tool can look for in a model.
_Avoid_: Check, inspection, smell, lint

**Scan**:
Applying all enabled rules to the model. A scan produces findings, never verdicts.
_Avoid_: Analysis, lint, inspection, check

**Finding**:
One place where a rule matched: the rule, a severity, the affected elements and the evidence.
_Avoid_: Issue, violation, warning, smell

**Not checkable**:
Said of a rule that could not be applied to an element because the model lacks the data it needs, for example an unknown timeout.
_Avoid_: Unknown (reserved for verdicts), n/a, skipped

**Suppression**:
The user's decision to accept a finding, with a written reason. A suppressed finding stays visible as suppressed.
_Avoid_: Ignore, waiver, exception, baseline

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
- An **element** sits at one **level**; drilling down into it opens a **view** one level lower that shows its children.
- An **element** has at most one **behavior model**, which offers **ports**. A **relationship** between two elements can bind their ports, which combines their behavior.
- A **property** has one **scope** element and talks about the behavior inside it.
- A **check** proves one **property** and ends with one **verdict**. Only a Disproved verdict has a **counterexample**, which can be **replayed**.
- A **scan** runs **rules** over the whole model and yields **findings**; a finding can be **suppressed**. Scans run independently of checks.
- A verdict or finding becomes **stale** when the model changes after it was computed.
