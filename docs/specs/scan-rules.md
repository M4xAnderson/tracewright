# Scan rules (v1)

As of 2026-11-02. Status: **Accepted** (applies `docs/specs/decisions.md`), revision 3, spec for slice 5 (`docs/product/v1-slices.md`). Revised after product, architecture and backend review; revision 3 applies the decisions owned by this spec (D-07, D-12, D-13, D-18, D-19, D-27, D-29, D-33, D-37, D-38, D-46) and aligns it with `docs/specs/net-semantics.md` revision 3 (**NS**), `docs/specs/state-machines.md` revision 3 (**SM**) and `docs/specs/file-format.md` (**FF**). It turns the research record (`docs/research/antipatterns.md` §1.2, §6, §7) into the rule set that the `scan` crate implements. Terms follow `GLOSSARY.md` (§10). Source tags: **[V]** checked against the linked primary source for this spec (2026-11-02), **[S]** secondary source, **[I]** our own design reasoning.

Every choice that revision 2 marked as a proposal is decided (D-18, ADR 0021); every question is closed (§12). ADR 0020 adds `suppressions.toml` to ADR 0001's file list, and FF §4.1, §6.2, §6.3 and §8 carry the format requirements of §2.7. §11 lists every departure from the research record.

## 1. Summary

The scan applies the rules to the whole model, at every level, and returns findings and Not checkable entries. It never returns verdicts. It never reads behavior models, port bindings or verdicts (ADR 0004: `scan` depends only on `model`). It never treats a value that is not stated as yes, no or a number. The order follows the product owner's priority.

| # | Rule ID | Name | Severity | Needs (beyond kinds and relationships) | Can be Not checkable | Slice |
|---|---|---|---|---|---|---|
| 1 | ARCH-CYCLE | Dependency cycle | Medium | nothing | never | 5 |
| 2 | RUN-SYNC-CYCLE | Synchronous call cycle | High | interaction | yes | 5 |
| 3 | REL-NO-TIMEOUT | Missing timeout on a remote synchronous call | High | interaction, timeout | yes | 5 |
| 4 | DATA-SHARED-DB | Database shared by several services | Medium | service (within one Software System) | yes | 5 |
| 5 | REL-SPOF | Single point of failure on a user flow | High (one replica) / Medium (one zone) | interaction, replicas, zones | yes | 5 if E8 passes, else 5b or later (D-18) |

The first four rules ship in slice 5 and are built in this order. **REL-SPOF ships in slice 5 only if the E8 dry run (hand-applying these rules to two real designs) finds at least one real single point of failure, and the user could state `replicas` and `zones` for most required units without research** (decided, D-18, ADR 0021). Otherwise it moves to slice 5b or later and does not hold back the other four. The scan runs on an explicit command in slice 5 (`run_scan`, §3.9); whether it later runs after every apply is decided after measuring (D-18). Reasons: `problems.md` §5 names cycles, synchronous cycles, timeouts and shared databases, not SPOF. REL-SPOF needs deployment data the target user may not draw, and it has the highest false-positive risk.

E8 uses this document and its test models. The E8 protocol and the journal record, per rule:
- findings;
- findings suppressed as *not real*;
- Not checkable entries, and how many of them the user resolved by stating the data;
- one journal row per finding that changed a design decision (`problems.md` §5: "at least one finding changes a design in the first month").

## 2. Element kinds and attributes

### 2.1 Element kind catalog (minimal)

Every element has exactly one kind, and every kind belongs to one level. A kind that does not match its level, or a parent of the wrong kind, is a `validate` problem (§3.10).

| Kind | Level | Parent | What it stands for | Used by rules as |
|---|---|---|---|---|
| Person | System Context | none | A human user | Entry point of a flow (REL-SPOF); left out of every other rule |
| Software System | System Context | none | A system, ours or external | Element of the System Context graph; a runtime unit when it has no children |
| Container | Container | Software System | A runtime boundary around running code: a service, web app, function, batch job | Runtime unit |
| Database | Container | Software System | One logical database or schema | Runtime unit; a data store |
| Queue | Container | Software System | One queue, topic or stream | Runtime unit; a data store |
| Component | Component | Container | A part of a container's code | Runs inside its container |
| Code element | Code | Component | A type, module or function | Runs inside its container |

Sources:
- C4 defines a container as "a runtime boundary around some code that is being executed or some data that is being stored", and lists databases as containers [V] ([C4: Container](https://c4model.com/abstractions/container)).
- C4 recommends modeling each queue or topic explicitly as a container, treated as a data store [V] ([C4: Queues and topics](https://c4model.com/abstractions/queues-and-topics)).
- C4 models a microservice as "a group of one or more containers" (for example an API container plus a database schema container). When a separate team owns the service, it is "'promoted' from a group of containers to a software system" [V] ([C4: Microservices](https://c4model.com/abstractions/microservices), re-checked 2026-11-02).

There is no Service kind. A service is one or more Containers, and the `service` attribute (§2.3) says which containers form one service. Cloud product kinds (AWS Lambda, DynamoDB, SQS, …) come later. Each will map to exactly one base kind above, and rules only ever see the base kind. Load balancer, gateway and cache kinds are not needed by these five rules and are left out of v1.

**Format (FF §6.2):** the element `kind` field, with all seven kinds above, is in format 1 from slice 0 (the canvas needs it for the icon), so slice 5 needs no FMT-1 version bump for kinds. A Person or Software System has no parent, so it is never a child of a combined scope (NS 6.6, D-13); E5's Person is modeled as a container stand-in for checks.

### 2.2 Attribute values

- **Yes/no attributes** hold `yes`, `no` or not stated.
- **Count attributes** hold a whole number from 1 to 1,000, or not stated.
- **Name attributes** hold a name of 1 to 64 characters after trimming, or not stated. Names are compared exactly after trimming, case-sensitive.
- **Not stated means the model does not say.** It is the value of every attribute until the user sets it.
- **There are no kind defaults in v1.** Nothing is ever assumed, so a quickly drawn design produces Not checkable entries, not findings. When cloud product kinds arrive, their defaults form a separate "assumed" tier that rules and the UI tell apart from stated values, never plain stated values (D-18, ADR 0021). This is the deliberate opposite of μTOSCA, whose interaction properties `timeout`, `circuit_breaker` and `dynamic_discovery` all default to `false` [V] ([microTOSCA.yml](https://raw.githubusercontent.com/di-unipi-socc/microTOSCA/master/1.3/microTOSCA.yml)). This answers research open question 1.
- **Required attributes.** When an attribute that a rule requires is not stated, and stating it could change the outcome, the rule gives Not checkable for that subject and names the missing attributes. When no possible value could change the outcome, the rule gives its outcome and does not ask. Every rule follows this, with no exceptions.
- **Words.** The UI, the files and this spec all say "not stated". "Unknown" is reserved for verdicts (GLOSSARY). This settles the draft's open question 1.

### 2.3 Element attributes

| Attribute | Kinds | Values | Meaning | Used by |
|---|---|---|---|---|
| `replicas` | Container, Database, Queue | count or not stated | Instances that can take over this element's role, including failover standbys. | REL-SPOF |
| `zones` | Container, Database, Queue | count or not stated | Distinct failure zones (for example availability zones) the replicas run in. `validate`: `zones ≤ replicas` when both are stated. | REL-SPOF |
| `service` | Container, Database | name or not stated | The service this element belongs to (not a team name). Containers of one Software System with the same `service` count as one service. On a Database it is shown as evidence only. | DATA-SHARED-DB |

Research §7.3 called this attribute `owner (team/service)`. It is renamed `service` because a team name would merge two services into one and silently clear a shared-database finding.

Database `tables` are not in v1 (§7, §11).

### 2.4 Relationship attributes

Both attributes are defined in **dependency direction** (§3.1): from the dependent end u to the end v it depends on, whichever way the relationship is drawn.

| Attribute | Values | Meaning | Used by |
|---|---|---|---|
| `interaction` | `synchronous`, `asynchronous` or not stated | Synchronous: u waits for v's answer before it goes on. Asynchronous: u does not wait. | RUN-SYNC-CYCLE, REL-NO-TIMEOUT, REL-SPOF |
| `timeout` | yes, no or not stated | u gives up waiting for v after a set time, on both connection and request. It only matters for synchronous interaction. It is not the Budget of a check and not a state machine's internal event that may be named `timeout` (D-33). | REL-NO-TIMEOUT |

**Interaction is not the binding mode (D-29).** A binding's `mode` (`sync` or `async`, FF §6.7) says how two ports are composed in a check (a fused transition or a generated place, ADR 0002). A relationship's `interaction` (`synchronous` or `asynchronous`) says whether the dependent end waits. They are different data with different words, and neither sets the other (§2.5).

### 2.5 Interaction and port bindings (decided, D-07, D-18; ADR 0016, ADR 0021)

- **Interaction is always stated by the user. Port bindings never set or override it.** A sync binding is a rendezvous (fused transitions), and an async binding is a generated place (ADR 0002). Neither says whether the dependent end waits for an answer:
  - a reply is its own sync binding pointing back, on the call's relationship (E5 case 1, NS 13.1: `control_plane/return_a → api_a/reply` on `r1`, `cluster_a/respond → control_plane/done_a` on `r3`);
  - a request and reply over two async bindings is still a caller that waits (E5 case 3, NS 13.3: the user states `interaction = "synchronous"` on the request relationship).
  Deriving interaction from bindings would make every request/reply pair look like a call cycle, or would lock a waiting caller to "asynchronous".
- **One relationship carries a call and its reply.** Each binding has its own direction (ADR 0016, NS L11, D-07): one port of each of the relationship's two elements, either way around. The relationship is drawn from the caller to the callee and holds the call binding (`from` in the source) and the reply binding (`from` in the target, `to` in the source). A reply drawn as its own relationship means "the target depends on the source", and the scan reads it that way (test RS9b).
- **Asynchronous replies need their own relationship.** NS L8 allows at most one asynchronous binding per relationship, so an asynchronous request and its asynchronous reply sit on two relationships (E5 case 3). The reply relationship reads as a dependency, and ARCH-CYCLE reports the pair (AC12); RUN-SYNC-CYCLE does not when the reply is stated `asynchronous`. Suppress as *accepted* with the reason "asynchronous reply channel". E8 counts these suppressions.
- **`validate` warnings** (FF `interactionVsBinding`). These never block anything. The stated value is still the one the scan uses.
  - Interaction is stated `asynchronous`, but a sync binding of the relationship has its `from` port in the drawn source: "the bindings make the source wait for the target".
  - A hand-edited file with both a stored interaction and bindings keeps both. Nothing is dropped (FMT-1).
- **Inspector suggestion.** When interaction is not stated and a sync binding has its `from` port in the source, the inspector offers "Set interaction: synchronous". This is an ordinary undoable Command and is never applied on its own; the stored attribute wins.
- Bindings are therefore not scan inputs. A timeout modeled inside a bound behavior model (E5 `case1_ebs_timeout.toml`) is not linked to the `timeout` attribute either (research open question 3). Both are known sources of Not checkable entries that E8 counts.

### 2.6 File representation (FF §6.3, §6.7)

A not stated attribute is an absent key. Known values are written like this:
- `interaction = "synchronous"` or `"asynchronous"`
- `timeout = "yes"` or `"no"`
- `replicas = 3`, `zones = 2`
- `service = "orders"`

The keys are in format 1 from slice 0 (FF §6.3), so projects from earlier slices load unchanged and slice 5 needs no FMT-1 version bump for them. Out-of-range values (a count of 0 or above 1,000, `zones > replicas`, an empty name, an attribute on a kind that does not allow it) are the FF warning `badAttribute`: the value is kept and the scan treats it as not stated (§3.10). A wrong TOML type, or a `service` longer than 64 characters, is the FF problem `schema` and is refused at load (LIM-2). Counts are a `Count(u16)` newtype (LIM-4).

### 2.7 Suppressions file (FF §8; ADR 0020)

Suppressions are stored in one file, `suppressions.toml`, at the project root (ADR 0020 adds it to ADR 0001's file list). It holds a `[[suppression]]` array sorted by key, with the fields `key`, `disposition` (`"accepted"` or `"not-real"`), `reason` and `date` (FF §8). A finding can span many elements, so its suppression belongs in no element's file.

- The file is in format 1 from the start and is created on the first suppression; an absent file means no suppressions, and a file that does not load shows every finding unsuppressed (FF §3.3: failing open shows more, never less).
- LIM-2 limits: at most 10,000 suppressions, file size at most 4 MiB, checked before parsing (FF §3.5).
- `validate` rules, each the FF warning `badSuppression` (the entry suppresses nothing and is kept verbatim): empty or too-long reason, unknown rule ID, malformed key, duplicate key, a `disposition` outside the two values.
- Finding keys need no escaping: the ID alphabet (FF §4.1, D-12, ADR 0020) is lowercase letters, digits, `-` and `_`, so it excludes `|`, `=` and `,`.

There is no per-project rule disabling in v1 (§11).

## 3. Shared definitions

### 3.1 Which relationships count, and their dependency direction

**Excluded from every rule.** The scan summary counts each group and points to the problems panel.
- Relationships with a dangling end.
- Relationships that touch an element excluded by §3.10.
- Relationships between an element and its own ancestor or descendant.
- Relationships where both ends are a Database or Queue (replication or forwarding inside the data layer).
- Relationships with a Person at either end, except as entry points in REL-SPOF.

**Dependency direction.** A relationship drawn from s to t means "s depends on t", with one exception: when s is a Database or Queue and t is not, the dependency runs from t to s.
- C4 lets a relationship show either a dependency or a data flow, as long as the label matches [V] ([C4 notation](https://c4model.com/diagrams/notation)). Its queue examples draw messages flowing from the queue to the consumer [V] ([C4: Queues and topics](https://c4model.com/abstractions/queues-and-topics)).
- A store never depends on its consumer. After this rule, every Database and Queue is a sink and never sits on a cycle.
- Evidence always shows the relationship as drawn ("drawn d → a, read as a depends on d").

Below, `dep(r) = (u, v)` is relationship r in dependency direction, and `int(r)` is its stated interaction.

### 3.2 Levels and lifted relationships

Levels are ordered System Context > Container > Component > Code. For an element e and a level L:
- `up(e, L)` is e if e is at L;
- it is e's ancestor at L if e is below L;
- it is undefined if e is above L.

The **level graph** `G_L` has the non-Person elements at level L. Take each counted relationship r with `dep(r) = (u, v)`. If `up(u, L)` and `up(v, L)` are both defined and different, r adds the **lifted relationship** `up(u, L) → up(v, L)`, and r is a **carrier** of it.
- Several relationships can carry the same lifted relationship. It keeps the sorted set of its carriers.
- Relationships whose ends map to the same element at L are dropped at L.

This matches C4 practice: Structurizr's DSL creates "implied relationships" from a relationship up to the parents of its ends by default [V] ([Structurizr implied relationships](https://docs.structurizr.com/dsl/implied-relationships)).

### 3.3 Runtime units, the runtime graph and remote relationships

**Runtime unit** `ru(e)` of an element e:

| e | `ru(e)` |
|---|---|
| Container, Database or Queue | itself |
| Component or Code element | its Container-level ancestor |
| Software System with no children | itself |
| Software System with children | undefined ("unresolved") |
| Person | undefined |

**Runtime graph** `G_run` has the runtime units. Take each counted relationship r with `dep(r) = (u, v)` and both `ru` defined.
- If `ru(u) ≠ ru(v)`, r adds `ru(u) → ru(v)`, carried by r with `int(r)`.
- If `u = v = ru(u)`, r adds a **self-relationship**: a runtime unit that calls its own endpoint.
- `G_sync` keeps the lifted relationships with at least one synchronous carrier. `G_su` keeps those with at least one carrier that is synchronous or has no stated interaction.

**Remote.** A relationship is remote when either:
- its ends have different runtime units;
- it is a self-relationship of a runtime unit (it goes over the network to the unit's own endpoint); or
- either end is a Software System with children (ancestor relationships are excluded, so it crosses a system boundary).

Any other relationship stays inside one runtime unit and is an **in-process call**. Example: a Component calling another Component, or itself, in the same Container. RUN-SYNC-CYCLE and REL-NO-TIMEOUT use this one definition.

### 3.4 Cycles, ID order and example cycles

- **ID order** is byte-wise lexicographic order of the slug strings (`order-api-10` sorts before `order-api-2`). Every "smallest", "sorted" and "in ID order" in this spec means this order.
- **Strongly connected sets** are computed with petgraph's `kosaraju_scc`. It is documented as "iterative and does two passes over the nodes" [V] ([docs.rs, petgraph 0.8.3](https://docs.rs/petgraph/latest/petgraph/algo/scc/kosaraju_scc/fn.kosaraju_scc.html)), so it meets LIM-3 (no recursion; cycle detection in `scan` is named there). `tarjan_scc` is documented as "recursive" [V] ([docs.rs](https://docs.rs/petgraph/latest/petgraph/algo/scc/tarjan_scc/fn.tarjan_scc.html)) and is not used. The order of the returned sets and of their members is not relied on: `scan` sorts both by ID.
- Every loop is bounded by the element and relationship counts, which pass the format spec's LIM-2 limits on load (with headroom above the 2,000-element target). Rules poll the job's cancel flag between levels and between sets (LIM-1, §3.9).
- **Example cycle:** the shortest cycle through the member with the smallest ID. It is found by breadth-first search inside the set, visiting neighbors in ID order. Each step names the drawn relationship that carries it (the smallest ID among eligible carriers; RUN-SYNC-CYCLE allows only synchronous carriers). The result is deterministic.
- `R(C)` is the set of drawn relationships that carry a lifted relationship with both ends in C. For RUN-SYNC-CYCLE, `R_sync(D)` is the set of synchronous carriers of the lifted relationships of `G_sync` with both ends in D.

### 3.5 Outcomes

For each subject (an element, a relationship, a set of elements, or the whole model), a rule gives exactly one of:
- a **finding**;
- a **Not checkable entry** that names the missing attributes or the missing model detail;
- nothing.

A rule never gives Unknown (VER-4 reserves it for verdicts) and never a verdict of any kind. If a rule panics, its join gives one entry "internal error" for that rule, and the other rules' results still show (ADR 0012, ADR 0021, §3.9).

### 3.6 Severity

- **High:** one failure or one slow call can stop a user flow or make it wait without limit.
- **Medium:** coupling that makes change harder or lets a failure spread.

These two words are the only severities (D-18, ADR 0021); the research record's error/warning/note and any v1-slices wording that uses them are superseded (§11 item 6). v1 has no lower level and no threshold tuning (out of scope for slice 5). For a later SARIF export:

| Here | SARIF |
|---|---|
| High | level `error` |
| Medium | level `warning` |
| Not checkable | result kind `open` ("insufficient information") |
| suppression | `suppressions[].justification` |

(Research §6.1 [V].)

### 3.7 Finding contents, level, key and order

A finding holds:
- the rule ID and severity;
- a finding key;
- the level it is shown at;
- the affected elements and relationships, selectable on the canvas (UX VRD-5);
- the evidence listed per rule;
- a one-sentence message in plain English;
- a short remediation with source links.

Messages are rendered from the current display names when they are shown, not stored. A rename therefore never makes a finding stale.

**Level per rule:**

| Rule | Level |
|---|---|
| ARCH-CYCLE | the level of its set |
| RUN-SYNC-CYCLE | Container when every member is at Container level, otherwise System Context |
| REL-NO-TIMEOUT | the lower level of the relationship's two drawn ends |
| DATA-SHARED-DB | Container |
| REL-SPOF | the level of the judged element (Container) |

**Finding key.** Keys are saved in `suppressions.toml`, so they are format data (FMT-1). The canonical encoding:

```
key      = rule-id 1*("|" field)
field    = name "=" value
value    = id-list / level / defect
id-list  = id *("," id)        ; sorted in ID order, no duplicates
level    = "system-context" / "container" / "component" / "code"
defect   = "one-replica" / "one-zone"
```

Keys contain element and relationship IDs only, never names. The ID alphabet (FF §4.1) is lowercase and excludes `|`, `=` and `,` (§2.7), so no escaping is needed. Two implementations must produce identical bytes.

| Rule | Key |
|---|---|
| ARCH-CYCLE | `ARCH-CYCLE|level=container|members=a,b` |
| RUN-SYNC-CYCLE | `RUN-SYNC-CYCLE|members=a,b` |
| REL-NO-TIMEOUT | `REL-NO-TIMEOUT|relationship=r1` |
| DATA-SHARED-DB | `DATA-SHARED-DB|database=d|accessors=a,b` |
| REL-SPOF | `REL-SPOF|element=d|defect=one-replica` |

New information shows a suppressed finding again: a cycle that gains a member, a database that gains an accessor, or an element that moves from "one zone" to "one replica" gets a new key.

**Order.** The panel orders by severity, then rule priority (§1), then key. Not checkable entries are grouped per rule, with a count and one action per missing attribute ("Set timeout on 7 relationships").

### 3.8 Suppression

**What it stores (decided, D-18, ADR 0021).** A suppression is the user's decision about a finding. It is a Command (undoable, journaled; `design.md` §3, §4): `Suppress { key, disposition: Accepted | NotReal, reason, date }`. It stores, under a finding key:
- a **reason**: required, 1 to 500 characters after trimming;
- a **disposition**: either *accepted* (the finding is real and the user accepts it) or *not real* (the rule is wrong here); TOML `"accepted"` or `"not-real"` (FF §8);
- the **date**, a local calendar date (TOML local date, FF §8) taken from the Clock port by the engine and passed into the Command, because `model::apply` is pure.

Editing a suppression issues the same Command again. `DeleteSuppression { key }` removes it. Suppressions are model data and are saved with the project. Findings are never saved; they are recomputed (in the spirit of ADR 0005).

**Display.**
- After a scan, a finding whose key has a suppression is shown as suppressed, with its reason and disposition. It stays listed and is counted separately ("4 findings, 1 suppressed").
- The suppression state is joined at display time, so suppressing never needs a new scan.
- A suppression whose key matches no current finding is listed under "Suppressions without a finding", with a Delete action. It is never deleted automatically, so a scan never makes the model dirty.
- Not checkable entries cannot be suppressed. The way out is to state the attribute.
- There is no subtree-wide or rule-wide suppression in v1.

**Noise guardrail** (north star: ≤ 20 % of findings suppressed as "not real"). It is shown in the panel footer.
- Numerator: the current shown findings that carry a *not real* suppression.
- Denominator: all current shown findings, suppressed ones included.
- Not counted anywhere: Not checkable entries, internal errors, covered findings that are hidden (§4), and suppressions without a finding.
- The footer also shows the number of Not checkable entries per rule. With no kind defaults, a flood of Not checkable entries is the likelier failure mode.

### 3.9 Running, isolation, cancellation and staleness

**Running (D-27, ADR 0021, design §4).** `run_scan` is an explicit command in slice 5. It starts an engine job and returns at once (CODING_STANDARDS: scans run as cancellable engine jobs). The job clones the current `Arc<Model>` and spawns one supervised thread per rule, `scan-<n>-<rule-id>`; a monitor thread of the scan joins each one and emits one `findings` event. A panicked join gives "internal error" for that rule only.
- These threads are within the `scan-*` prefix of ADR 0012, as ADR 0021 records.
- ADR 0006 bans `catch_unwind`, so a thread per rule is the only way to keep the per-rule promise.

**Cancellation.** `cancel_scan` sets the job's cancel flag, which every rule polls (§3.4). A cancelled scan shows no partial results: its `findings` event carries `cancelled: true` and no findings (design §3). The panel keeps the previous results under their own hash (shown as stale if the model changed since) and says "Scan cancelled".

**Staleness.** Results carry the **scan input hash**: a canonical encoding of exactly what the rules read.
- Included: element ID, kind and parent; relationship ID and ends; the stated interaction, timeout, replicas, zones and service; the excluded set (§3.10); and the list of rules run.
- Not included: layout, display names, descriptions, behavior models, port bindings (not scan inputs, §2.5) and suppressions (joined at display).
- Results carry it as `scanHash` (design §3 `Sync.scanHash` and the `findings` event; never the model hash, D-27). When the current hash differs, the results show as stale (GLOSSARY: Stale).
- Fitness tests in the style of ADR 0013: every mutation of a scan input changes the hash; a layout change, a rename, a binding change or a different insertion order does not.

**Cost.** Every rule except ARCH-CYCLE's fold step is linear in elements plus relationships per level. The fold step is one linear pass per strongly connected set. At 2,000 elements the scan needs no budget beyond the cancel flag.

### 3.10 Models with `validate` problems

The loader admits models with problems (ADR 0001), so the scan states its precondition.
- **Element excluded** (FF `kindLevel`). An element whose kind does not match its level, whose parent has the wrong kind or is missing, or that sits below such an element is excluded from every rule. All relationships touching it are excluded too. The summary counts them: "Excluded: 2 elements, 3 relationships with problems". An element file that did not load (FF §3.3) is not in the model: relationships to its element count as dangling (§3.1), and the summary names the files not loaded, because a missing element can hide a finding.
- **Attribute treated as not stated** (FF `badAttribute`). An attribute value that fails `validate` (out of range, or `zones > replicas`) is treated as not stated. The Not checkable entry says "replicas and zones disagree; see the problems panel".
- **Interaction disagrees with bindings** (FF `interactionVsBinding`). The stated interaction is used, and the warning stays in the problems panel (§2.5).

### 3.11 What "no findings" means

The empty state never reads as "clean". It shows:
- the rules that ran and the number of elements;
- the number of Not checkable entries and excluded elements, and the element files not loaded;
- a link to the blind spots below.

For example: "No findings. 5 rules ran on 42 elements. 12 Not checkable entries. What the scan does not see". The three facts above and the blind-spot list are normative; the wording, and "not stated" and the rule IDs in `flows.md` §4.14, belong to the senior UX designer, whose copy may not contradict them (D-46).

Blind spots:
- flows not started by a Person (Persons are the only entry points in v1, D-18);
- external systems, which REL-SPOF never judges;
- waiting over asynchronous paths (a full producer buffer);
- correlated failures;
- pool exhaustion without a cycle;
- anything that only behavior models and checks can show.

## 4. ARCH-CYCLE: Dependency cycle

**Rationale.**
- "The dependency graph of packages or components should have no cycles" (Robert C. Martin, Acyclic Dependencies Principle) [S] ([Wikipedia](https://en.wikipedia.org/wiki/Acyclic_dependencies_principle); Martin's original paper not fetched).
- Arcan: a cyclic dependency "arises when two or more architectural components depend on each other directly or indirectly" [V] ([Arcan wiki](https://essere.disco.unimib.it/wiki/arcan/)).
- Taibi, Lenarduzzi and Pahl list Cyclic Dependency, "a cyclic chain of calls between microservices". Its problem is that the services "can be hard to maintain or reuse in isolation". It has perceived harmfulness 7 of 10, third after Hardcoded Endpoints and Wrong Cuts [V] ([arXiv 1908.04101](https://arxiv.org/abs/1908.04101), Tables 1.1 and 1.3).

This rule takes the structural view (any interaction). RUN-SYNC-CYCLE takes the runtime view.

**Detection.**
1. **Sets.** For each level L, build `G_L` (§3.2) from all counted relationships, whatever their interaction. `S_L` is the strongly connected sets with at least 2 elements. Self-relationships are not cycles for this rule.
2. **Fold (one cause, one finding).** Go through the levels from Code up to Container. Take a set C in `S_L` whose members map (`up(·, L+1)`) to at least two elements. Those elements lie in exactly one set C′ of `S_{L+1}`, because contracting a strongly connected set keeps it strongly connected.
   - Remove from `G_{L+1}` every lifted relationship whose carriers all lie in `R(C)`. If C′ is then no longer strongly connected, C is necessary for C′, and **C folds into C′**.
   - A set whose members all map to one element never folds: its relationships are dropped one level up.
   - Folding is transitive.
3. **In-parent cycles of a folded set.** For each C that folded, take the relationships of `R(C)` whose two ends map to the same element at L+1. Compute their strongly connected sets with at least 2 elements. Each one is its own set at L, because it is an independent cause that survives when the cross-parent relationships are removed. Such sets take part in no further folding.
4. **Findings.** Every set of `S_L` that did not fold, and every set from step 3, is one finding at its level. Its evidence lists:
   - the sets folded into it, transitively ("also visible at Component level");
   - the lower sets that map into it but did not fold ("see also: 2 Component-level cycles, reported separately").
5. **Covered by RUN-SYNC-CYCLE.** This rule computes the strongly connected sets of `G_sync` itself, with the shared function that RUN-SYNC-CYCLE also uses. It does not read the other rule's output.
   - A finding is **covered** when its set, or a set folded into it, has a runtime-unit image `{ru(m)}` (all defined) equal to such a `G_sync` set. The covered finding carries the key of that RUN-SYNC-CYCLE finding.
   - The panel hides a covered finding while RUN-SYNC-CYCLE ran successfully in the same scan and its finding with that key is not suppressed as *not real*.
   - Otherwise the covered finding is shown. That happens when RUN-SYNC-CYCLE was not in the rule set, ended in "internal error", or its finding is suppressed as *not real*.
   - This way, a real cycle never disappears because of another rule's state, and a synchronous cycle is not reported twice.

**Attributes.** None beyond kind, parent and relationships. **Not checkable:** never. Excluded relationships are counted in the summary.

**Severity:** Medium.

**Evidence:**
- the level and the members;
- the example cycle, with the drawn relationship behind each step. A lifted step reads "Billing → Ledger, through billing-worker → ledger-api";
- the number of relationships inside the set;
- the folded and see-also sets.

**Message:** "Order API, Billing and Shipping depend on each other in a cycle." **Remediation:** remove or invert one relationship (an event through a Queue in place of a call back), or merge the elements if they always change together. Taibi et al. report "refinement of the cycles" and an API gateway as adopted solutions [V].

**False positives.**
- Callbacks and webhooks (we call a payment provider and it calls us back) are a deliberate two-way coupling. Suppress as *accepted*.
- An asynchronous request and its asynchronous reply need two relationships (NS L8), so they read as a cycle (AC12, §2.5). Suppress as *accepted*.
- Lifting turns every two-way pair of child relationships into a parent cycle. That is correct by definition, but it may surprise.
- A parent cycle with two independent lower causes is reported once per cause plus once at the parent level (AC8). Fixing one cause leaves the parent cycle.

**False negatives:**
- cycles through a Queue or Database are not cycles by design (§3.1);
- self-relationships are ignored (research A1 counts self-loops; §11).

**Suppression key:** `ARCH-CYCLE|level=…|members=…`.

**Test models** (notation and harness in §9; Rules column: AC = ARCH-CYCLE, RS = RUN-SYNC-CYCLE):

| # | Rules | Model | Expected (complete) |
|---|---|---|---|
| AC1 | AC, RS | `system s { container a; container b }`; `r1: a -> b async`; `r2: b -> a sync` | F ARCH-CYCLE Medium, Container, {a, b}, example a →r1 b →r2 a. RS: nothing. |
| AC2 | AC, RS | `system s { container a; container b; queue q }`; `r1: a -> q async` ("publishes to"); `r2: q -> b async` ("delivers to"); `r3: b -> a sync` | Nothing. r2 reads as b depends on q, so q is a sink. |
| AC3 | AC, RS | `system s { container a { component a1; component a2 }; container b { component b1 } }`; `r1: a1 -> b1 async`; `r2: b1 -> a2 async`; `r3: a1 -> a2`; `r4: a2 -> a1` | Two findings. (1) F ARCH-CYCLE Medium, Container, {a, b}, example a →r1 (a1 → b1) b →r2 (b1 → a2) a, also visible at Component level {a1, a2, b1} (removing r1, r2 breaks {a, b}, so it folds). (2) F ARCH-CYCLE Medium, Component, {a1, a2}, example a1 →r3 a2 →r4 a1 (step 3: in-parent cycle of the folded set). RS: nothing. |
| AC4 | AC, RS | `system s { container a { component a1; component a2 } }`; `r1: a1 -> a2`; `r2: a2 -> a1` | F ARCH-CYCLE Medium, Component, {a1, a2}. RS: nothing (in-process). |
| AC5 | AC, RS | `system s1 { container a1 }; system s2 { container b1 }`; `r1: a1 -> b1 async`; `r2: b1 -> a1 async` | F ARCH-CYCLE Medium, System Context, {s1, s2}, also visible at Container level {a1, b1}. RS: nothing. |
| AC6 | AC, RS | `person p`; `system s { container a; container b }`; `r1: p -> a`; `r2: a -> p` ("emails"); `r3: a -> b sync`; `r4: b -> a sync` | F RUN-SYNC-CYCLE High {a, b}, example a →r3 b →r4 a. C ARCH-CYCLE Container {a, b}, covered by `RUN-SYNC-CYCLE|members=a,b` (hidden). r1, r2 excluded (Person). |
| AC6b | AC | AC6 | F ARCH-CYCLE Medium, Container, {a, b}, covered-by key recorded, **shown** (RUN-SYNC-CYCLE not run). |
| AC6c | AC, RS | AC6, with `RUN-SYNC-CYCLE|members=a,b` suppressed as *not real* | Panel: RUN-SYNC-CYCLE finding suppressed; ARCH-CYCLE {a, b} **shown**. |
| AC6d | AC, RS | AC6, RUN-SYNC-CYCLE made to panic (fault injection) | RS: "internal error". F ARCH-CYCLE Medium {a, b} **shown**. |
| AC7 | AC, RS | `system s { container a { component a1; component a3 }; container b { component b1 } }`; `r1: a1 -> b1 async`; `r2: b1 -> a1 async`; `r5: a3 -> b1 async` | F ARCH-CYCLE Medium, Container, {a, b}, also visible at Component level {a1, b1}. One finding: r5 does not block folding, because removing r1 and r2 still breaks b → a. RS: nothing. |
| AC8 | AC | `system s { container a { component a1; component a2 }; container b { component b1; component b2 } }`; `r1: a1 -> b1 async`; `r2: b1 -> a1 async`; `r3: a2 -> b2 async`; `r4: b2 -> a2 async` | Three findings: Container {a, b} (see also both Component sets); Component {a1, b1}; Component {a2, b2}. Neither lower set is necessary on its own. |
| AC9 | AC | `system s { container a { component a1 { code x }; component a2 { code y } } }`; `r1: x -> y`; `r2: y -> x` | F ARCH-CYCLE Medium, Component, {a1, a2}, also visible at Code level {x, y}. |
| AC10 | AC | `system s { container a; container b }`; `r1: a -> b async`; `r2: b -> ghost` (dangling) | Nothing. Summary: 1 relationship excluded (dangling end). |
| AC11 | AC | AC1 with `ARCH-CYCLE|level=container|members=a,b` suppressed as *accepted*, then `container c`, `r3: b -> c async`, `r4: c -> a async` added | F ARCH-CYCLE Medium {a, b, c}, new key, not suppressed. The old suppression is listed under "Suppressions without a finding". |
| AC12 | AC, RS | E5 case 3 request and reply (NS 13.3): `system s { container metadata; container storage }`; `r2: storage -> metadata sync` binds `async storage/request -> metadata/accept`; `r3: metadata -> storage async` binds `async metadata/respond -> storage/inbox` (two relationships, because NS L8 allows one asynchronous binding per relationship) | F ARCH-CYCLE Medium, Container, {metadata, storage}, example metadata →r3 storage →r2 metadata, shown (no `G_sync` set covers it). RS: nothing. Known false positive of §2.5: suppress as *accepted*, "asynchronous reply channel". |

## 5. RUN-SYNC-CYCLE: Synchronous call cycle

**Rationale.** When runtime units call each other synchronously in a loop, each one holds a request thread (or connection) while it waits for the next. With bounded pools, they can all end up waiting for each other.
- ZeroC's Ice manual describes exactly this: "the only thread in the server thread pool is busy waiting for its invocation of `opB` to complete", so the callback cannot run, and the callers "remain blocked indefinitely unless timeouts are used". Its fixes are a larger pool, a separate pool for callbacks, or asynchronous dispatch [V] ([Ice 3.5: Nested invocations](https://archive.zeroc.com/x/xQDg)).
- E5 case 1 reproduced the related thread-starvation pattern from the AWS EBS 2011 post-mortem. It also showed that **adding a timeout hides the starvation from deadlock-free and always-reachable, but not from a share-of-pool "never" predicate** (`docs/product/experiments/e5-paper-test.md`, case 1 "Lost", Implication 1). A timeout turns waiting into failed calls; it does not remove the cycle.
- A cycle is one common cause, not the only one. Pool exhaustion can also happen without a cycle [S] (an IBM patent application seen in search results, not fetched). That case is what behavior models and checks are for.

**Detection.**
1. Build `G_run`, `G_sync` and `G_su` (§3.3). Only runtime units count: components share their container's process, so synchronous calls between components of one container are in-process calls (ARCH-CYCLE reports cycles among them).
2. **Findings:** every strongly connected set of `G_sync` with at least 2 units, and every unit with a synchronous self-relationship.
3. **Not checkable (interaction not stated).** Take every strongly connected set D of `G_su` with at least 2 units, or a unit whose self-relationship is synchronous or has no stated interaction. If its members are not exactly the members of a finding from step 2, D is Not checkable.
   - The missing data is interaction on those relationships with no stated interaction that carry a lifted relationship inside D **that has no synchronous carrier**.
   - Stating the others cannot change the outcome.
   - If D equals a finding, nothing is asked.
4. **Not checkable (unresolved end).** Take the counted relationships with synchronous or not stated interaction that have an end at a Software System with children.
   - Build `G′` from `G_su` by merging each such system with all its containers into one vertex and adding those relationships.
   - A relationship whose two ends land in one strongly connected set of `G′` is Not checkable: "draw it to the container of Billing System that receives the call".
5. The finding's level follows §3.7.

**Attributes.** interaction is required.
- `synchronous`: carried into `G_sync`.
- `asynchronous`: never part of a call cycle.
- Not stated: only in `G_su`, so it can give Not checkable and never a finding.

`timeout` is shown in the evidence and chooses the message, but it does not change the outcome.

**Not checkable:** steps 3 and 4.

**Severity:** High.

**Evidence:**
- the members;
- the example cycle, built from synchronous carriers only, with each drawn relationship;
- the `timeout` value of each step;
- for lifted steps, the components at each end.

**Hand-off to checks (M4).** If the members' parent has behavior models, the panel suggests two checks:
- deadlock-free;
- the share-of-pool predicate template from slice 4 ("never does one caller hold all of pool P").

The panel states: "A Proved deadlock-free does not clear this finding. With timeouts, calls fail instead of waiting, and a pool can still be used up (E5 case 1)." The scan does not read verdicts.

**Message** (chosen by the timeouts of the example cycle):
- Every step has `timeout = yes`: "Order API and Billing call each other synchronously; under load they can use up each other's request threads, and calls then fail by timeout."
- Otherwise: "Order API and Billing call each other synchronously; under load their request threads can wait on each other, without limit where a call has no timeout."

**Remediation:** make one direction asynchronous, give the calls back their own pool (bulkhead), or remove the call back.

**False positives.**
- Lifting to containers reports a cycle even when no single request goes around it (a1 → b1 and b2 → a2). That is still the cross-waiting risk when each container has one request pool, and it is why containers, not components, are the unit. If the two directions use separate pools, suppress as *accepted* with that reason.
- A reply drawn as its own relationship reads as a call back (RS9b). Put the reply binding on the call's relationship instead (§2.5, ADR 0016).
- A call that is stated synchronous but is fire-and-forget is a modeling error. Fix the attribute.

**Suppression key:** `RUN-SYNC-CYCLE|members=…`.

**Test models:**

| # | Rules | Model | Expected (complete) |
|---|---|---|---|
| RS1 | AC, RS | `system s { container a; container b; container c }`; `r1: a -> b sync`; `r2: b -> c sync`; `r3: c -> a sync` | F RUN-SYNC-CYCLE High {a, b, c}, example a →r1 b →r2 c →r3 a, "wait … without limit" message. C ARCH-CYCLE {a, b, c} covered (hidden). |
| RS2 | AC, RS | `system s { container a; container b }`; `r1: a -> b sync`; `r2: b -> a` | NC RUN-SYNC-CYCLE {a, b}, missing interaction on r2. F ARCH-CYCLE Medium, Container, {a, b} (no `G_sync` set covers it). |
| RS3 | AC, RS | `system s { container a { component a1; component a2 }; container b { component b1; component b2 } }`; `r1: a1 -> b1 sync`; `r2: b2 -> a2 sync` | F RUN-SYNC-CYCLE High {a, b}, example a →r1 (a1 → b1) b →r2 (b2 → a2) a. C ARCH-CYCLE Container {a, b} covered (hidden). |
| RS4 | AC, RS | `system s { container a { component a1; component a2 } }`; `r1: a1 -> a2 sync`; `r2: a2 -> a1 sync` | RS: nothing (one runtime unit). F ARCH-CYCLE Medium, Component, {a1, a2}. |
| RS5 | AC, RS | `system s { container a; container b }; system x`; `r1: a -> x sync`; `r2: x -> s sync` | NC RUN-SYNC-CYCLE r2, unresolved end s ("draw it to the container of s that receives the call"). F ARCH-CYCLE Medium, System Context, {s, x}. |
| RS6 | AC, RS | `system s { container a }`; `r1: a -> a sync` | F RUN-SYNC-CYCLE High {a}, example a →r1 a. AC: nothing (self-relationships ignored). |
| RS7 | AC, RS | `system s { container a; container b }`; `r1: a -> b sync`; `r2: b -> a sync`; `r3: a -> b async` | F RUN-SYNC-CYCLE High {a, b}, example a →r1 b →r2 a (never r3). C ARCH-CYCLE {a, b} covered (hidden): sets are compared by members, not by `R`. |
| RS8 | AC, RS | `system s { container a; container b; container c }`; `r1: a -> b sync`; `r2: b -> a sync`; `r3: b -> c`; `r4: c -> a` | F RUN-SYNC-CYCLE High {a, b}. NC RUN-SYNC-CYCLE {a, b, c}, missing interaction on r3, r4. F ARCH-CYCLE Medium {a, b, c} (image differs from {a, b}). |
| RS9 | AC, RS, NT | E5 case 1 at Container level: `system ebs { container api_a; container api_b; container control_plane; container cluster_a; container cluster_b }`; `r1: api_a -> control_plane sync timeout=no` binds `sync api_a/call -> control_plane/accept_a` and `sync control_plane/return_a -> api_a/reply`; `r2: api_b -> control_plane sync timeout=no` (same for b); `r3: control_plane -> cluster_a sync timeout=no` binds `sync control_plane/fwd_a -> cluster_a/create` and `sync cluster_a/respond -> control_plane/done_a`; `r4: control_plane -> cluster_b sync timeout=no` (same for b). This is the four-relationship mapping of NS 13.1 (D-07) | RS: nothing. AC: nothing. The reply bindings add no relationship and no dependency. F REL-NO-TIMEOUT High on r1, r2, r3, r4. |
| RS9b | RS | RS9 plus `r5: cluster_a -> control_plane sync` ("responds to") carrying the binding `cluster_a/respond -> control_plane/done_a` moved from r3 (legal for checks under NS L11; the net is the same up to the fused ID `r5/respond+done_a`) | F RUN-SYNC-CYCLE High {cluster_a, control_plane}: the drawing says each waits on the other. Remedy: move the binding back to r3 and delete r5 (§2.5). |

## 6. REL-NO-TIMEOUT: Missing timeout on a remote synchronous call

**Rationale.**
- AWS Well-Architected REL05-BP05 "Set client timeouts": "Set both a connection timeout and a request timeout on any service dependency call and generally on any call across processes". Some frameworks "have default values that are infinite". Level of risk if not established: **High** [V] ([REL05-BP05](https://docs.aws.amazon.com/wellarchitected/latest/reliability-pillar/rel_mitigate_interaction_failure_client_timeouts.html)).
- Neri, Soldani, Zimmermann and Brogi define Wobbly Service Interactions: an interaction "is 'wobbly' when a failure in mf can result in triggering a failure also in mi". Timeouts are the second most discussed fix after circuit breakers, and the timeout "can be also set in the invoker" [V] ([arXiv 1906.01553](https://arxiv.org/abs/1906.01553), §4.3).
- μTOSCA records `timeout` per interaction [V] ([microTOSCA.yml](https://raw.githubusercontent.com/di-unipi-socc/microTOSCA/master/1.3/microTOSCA.yml)).

Severity follows AWS's High risk level. This rule is about waiting without limit on a hung dependency. As E5 case 1 showed, a timeout bounds each wait but does not by itself prevent pool starvation under load. That is a question for checks.

**Detection.** Judge each counted relationship r (§3.1) in dependency direction, `dep(r) = (u, v)`. Only relationships where u is a Container, Component, Code element or Software System, and r is remote (§3.3), are judged. Persons are excluded, and after §3.1 u is never a Database or Queue.

| interaction | timeout | Outcome |
|---|---|---|
| synchronous | no | Finding |
| synchronous | yes | nothing |
| synchronous | not stated | Not checkable: timeout |
| asynchronous | any | nothing |
| not stated | yes | nothing (u does not wait without limit, whatever the interaction) |
| not stated | no | Not checkable: interaction |
| not stated | not stated | Not checkable: interaction, timeout |

In-process calls give nothing. Each drawn relationship is judged on its own; nothing is lifted.

**Not checkable:** see the table. This meets the slice 5 acceptance criterion: a timeout that is not stated gives Not checkable, never a finding.

**Severity:** High.

**Evidence:**
- the relationship as drawn, and in dependency direction when they differ;
- u and v with their runtime units;
- interaction;
- `timeout = no`.

**Message:** "Checkout waits for Payments with no timeout; if Payments hangs, Checkout's threads hang too." **Remediation:** set a connection and a request timeout below the caller's own deadline. Consider a circuit breaker and bounded retries (AWS REL05-BP03, REL05-BP05).

**False positives.**
- A call drawn at both Component and Container level gives two findings. They are reported each in v1 and counted in E8; folding comes only if the noise shows (D-18). State the attribute on both, or suppress the container-level one as *not real*.
- A sidecar drawn as its own Container makes a local call look remote.

**False negatives:**
- An asynchronous send can still block when a producer buffer is full. Model that with `when_full = block` in a behavior model (ADR 0014).
- A timeout modeled only inside a behavior model is not seen (§2.5).

**Suppression key:** `REL-NO-TIMEOUT|relationship=…`.

**Test models** (Rules: NT only):

| # | Model | Expected (complete) |
|---|---|---|
| NT1 | `system s { container a; container b }`; `r1: a -> b sync timeout=no` | F REL-NO-TIMEOUT High on r1. |
| NT2 | same, `r1: a -> b sync` | NC r1, missing timeout. |
| NT3 | same, `r1: a -> b async` | Nothing. |
| NT4 | same, `r1: a -> b timeout=no`; `r2: b -> a timeout=yes` | NC r1, missing interaction. Nothing for r2. |
| NT5 | `system s { container a { component a1; component a2 }; container b }`; `r1: a1 -> a2 sync timeout=no`; `r2: a1 -> b sync timeout=no` | Nothing for r1 (in-process). F High on r2. |
| NT6 | `person p`; `system s { container a; database d }`; `r1: p -> a sync timeout=no`; `r2: a -> d sync timeout=no` | Nothing for r1 (Person). F High on r2 (database calls need timeouts too). |
| NT7 | `system s { container a; database d }`; `r1: d -> a sync timeout=no` ("returns rows to") | F High on r1; evidence: drawn d → a, read as a waits on d. |
| NT8 | `system s { container a { component a1 } }`; `r1: a -> a sync timeout=no`; `r2: a1 -> a1 sync timeout=no` | F High on r1 (a runtime unit calling its own endpoint is remote). Nothing for r2 (in-process). |
| NT9 | `system s { container a; container b }`; `r1: a -> b timeout=no` binds `sync a/call -> b/accept` | NC r1, missing interaction (bindings do not state it). The inspector offers "Set interaction: synchronous". |
| NT10 | `system s { container a }; system x { container x1 }`; `r1: a -> x sync timeout=no` | F High on r1 (crosses a system boundary). |

## 7. DATA-SHARED-DB: Database shared by several services

**Rationale.**
- Neri et al.: "The Shared Persistence smell occurs whenever two microservices access and manage the same database, possibly violating the decentralisation design principle". The fixes are to split the database, add a data manager service, or merge the services [V] ([arXiv 1906.01553](https://arxiv.org/abs/1906.01553), §4.4).
- Taibi et al. rate Shared Persistence 6.05 of 10 [V; the value may be a text-extraction artifact for 6.5, as the research record notes] ([arXiv 1908.04101](https://arxiv.org/abs/1908.04101), Table 1.1).
- Richardson names the drawbacks "Development time coupling" and "Runtime coupling - because all services access the same database they can potentially interfere with one another" [V] ([Shared database](https://microservices.io/patterns/data/shared-database.html)).
- Sharing a server is fine when data stays private: "Schema-per-service" and "Private-tables-per-service" [V] ([Database per service](https://microservices.io/patterns/data/database-per-service.html)). A Database element stands for one logical database or schema (§2.1), so schema-per-service is drawn as separate Database elements.
- C4 models one service as one or more containers, and promotes a service owned by a separate team to its own Software System [V] ([C4: Microservices](https://c4model.com/abstractions/microservices)). So containers of different Software Systems are different services, and within one system the `service` attribute decides.

**Detection.** For each Database d:
1. **Accessors.** Every counted relationship r with `dep(r) = (u, d)` where u is not a Person, Database or Queue. A relationship drawn from d to a consumer counts, by §3.1.
2. **Accessor unit.** `ru(u)`, or u itself when u is a Software System with children (an outside system reading our data). A Software System accessor merges with any accessor containers of its own.
3. **Two accessor units x, y** are:
   - *certainly different services* when they lie in different Software Systems, or both have a stated `service` and the names differ;
   - *certainly one service* when they are the same unit, or lie in one Software System with the same stated `service`;
   - *possibly different* otherwise (one Software System, at least one `service` not stated).
4. **Finding** if at least one pair is certainly different.
5. Otherwise, **Not checkable** if at least one pair is possibly different. The missing data is `service` on those accessor units that have none, for example "service not stated for: api, worker".
6. Otherwise nothing.

**Attributes.** `service` is required within one Software System. Across systems no attribute is needed: the premise of the source definition, two services reaching one database, is drawn data. Database `service` is evidence only.

**Not checkable:** step 5.

**Severity:** Medium.

**Evidence:**
- the database, and its `service` if stated;
- each service, with its accessor units and relationships, grouped by stated `service` or by Software System.

**Message:** "Orders DB is used by Order API and Billing; a schema change in one can break the other." **Remediation:**
- give each service its own database or schema (draw one Database per schema), or let one service own the data and offer it through an API;
- if they are one service, give them the same `service`.

**False positives.**
- Services that use separate tables inside one schema: suppress as *accepted*, with that reason. v1 does not model tables (§11).
- Read-only reporting replicas fed from the database: suppress as *accepted*, or draw the replica as its own Database.
- Change-data-capture consumers drawn from d count as accessors by design.

**False negatives:** one service reading another service's private database through a single relationship (Taibi's Inappropriate Service Intimacy) is not in v1.

**Suppression key:** `DATA-SHARED-DB|database=…|accessors=…` (accessor unit IDs). A new accessor shows the finding again.

**Test models** (Rules: SD only):

| # | Model | Expected (complete) |
|---|---|---|
| SD1 | `system s { container a; container b; database d }`; `r1: a -> d sync`; `r2: b -> d sync` | NC DATA-SHARED-DB d, service not stated for a, b. |
| SD2 | `system s { container api [service=orders]; container worker [service=orders]; database d [service=orders] }`; `r1: api -> d`; `r2: worker -> d` | Nothing (one service). |
| SD3 | `system s { container a [service=orders]; container b [service=billing]; database d }`; `r1: a -> d`; `r2: b -> d` | F DATA-SHARED-DB Medium on d, services orders {a}, billing {b}; key `DATA-SHARED-DB|database=d|accessors=a,b`. |
| SD4 | `system s { container a { component a1; component a2 }; database d }`; `r1: a1 -> d`; `r2: a2 -> d` | Nothing (one accessor unit). |
| SD5 | `system s1 { container a; database d }; system s2 { container b }`; `r1: a -> d`; `r2: b -> d` | F Medium on d (different Software Systems; no service needed). |
| SD6 | `system s { container a [service=orders]; container b [service=billing]; database d }`; `r1: a -> d`; `r2: d -> b` ("streams changes to") | F Medium on d, accessors a, b (r2 reads as b depends on d). |
| SD7 | `system s { container a [service=orders]; database d }; system x { container x1 }`; `r1: a -> d`; `r2: x -> d` | F Medium on d, accessors a and x (outside system as one accessor). |
| SD8 | `system s { container a [service=orders]; container b; container c [service=orders]; database d }`; `r1: a -> d`; `r2: b -> d`; `r3: c -> d` | NC d, service not stated for b. |
| SD9 | SD3 with its finding suppressed as *accepted*, then `container c [service=shipping]`, `r3: c -> d` added | F Medium with new key `…|accessors=a,b,c`, not suppressed. The old suppression is listed without a finding. |

## 8. REL-SPOF: Single point of failure on a user flow

**Rationale.**
- Google Cloud: "you must design your systems to avoid any single points of failure" and "distribute and replicate your services and applications across multiple zones and regions" [V] ([Build highly available systems](https://docs.cloud.google.com/architecture/framework/reliability/build-highly-available-systems)).
- AWS REL10-BP01: "A fundamental principle for service design in AWS is to avoid single points of failure". Its anti-pattern: "Your production workload exists only in a single Availability Zone". Its guidance: "Deploy and operate all production workloads in at least two Availability Zones (AZs) in a Region". Level of risk: High [V] ([REL10-BP01](https://docs.aws.amazon.com/wellarchitected/latest/reliability-pillar/rel_fault_isolation_multiaz_region_system.html), re-checked 2026-11-02).
- With hard dependencies, "the invoking system's availability is the product of the dependent systems' availabilities", while a soft dependency's failure "is compensated for in the application" [V] ([AWS: Availability](https://docs.aws.amazon.com/wellarchitected/latest/reliability-pillar/availability.html)).

**Why reachability and not dominators** (a change from research §1.2 A15 [I]). A relationship means "uses", and a request needs all of its hard dependencies (AWS's product rule), not one of several paths. Dominator analysis assumes alternative paths, which a C4 diagram does not express. Redundancy is stated with `replicas` and `zones`. So every single-instance unit that a user flow reaches synchronously is a single point of failure by itself.

**Detection.**
1. **Entry points.** Persons are the only entry points in v1 (decided, D-18; ADR 0021). For each relationship r from a Person p to an element t, the entry unit is `ru(t)`.
   - If t is a Software System with children, r is Not checkable: "Customer uses Shop, but no container of Shop".
   - If no Person relationship exists at all, the result is one model-level Not checkable entry: "No Person uses an element, so REL-SPOF has no flow to follow".
2. **Required units.** Breadth-first search over `G_run` from the entry units in ID order, visiting neighbors in ID order.
   - From a reached unit, a lifted relationship is followed when a carrier is synchronous.
   - When the carrier is asynchronous and the target is a Queue, the Queue is marked required but not followed: the Queue must accept the message, but its consumers are not on the request's path.
   - Other asynchronous carriers are not followed.
   - Childless Software Systems are traversed but not judged.
   - The **path** of a unit is its breadth-first predecessor path (ties broken by ID order).
3. **Possibly required units.** The same search, also following carriers with no stated interaction, minus the required units.
4. **Unresolved ends.** A relationship from a required or possibly required unit to a Software System with children, with synchronous or not stated interaction, is Not checkable: "draw it to the container used".
5. **Judge** each required or possibly required Container, Database and Queue. Because `validate` requires `zones ≤ replicas`, `zones ≥ 2` implies `replicas ≥ 2`.

| replicas | zones | Required unit | Possibly required unit |
|---|---|---|---|
| 1 | any | Finding, High, "one replica" | Not checkable: interaction (the relationships with no stated interaction on its path) |
| ≥ 2 or not stated | 1 | Finding, Medium, "one zone" (evidence notes it if replicas are not stated) | Not checkable: interaction |
| any | ≥ 2 | nothing | nothing |
| ≥ 2 | not stated | Not checkable: zones | Not checkable: zones, interaction |
| not stated | not stated | Not checkable: replicas, zones | Not checkable: replicas, zones, interaction |

**Attributes.** interaction is required for the path; `replicas` and `zones` are required for the subject. Values that fail `validate` are treated as not stated (§3.10).

**Not checkable:** steps 1 and 4, and the table.

**Severity:** High for one replica, Medium for one zone.

**Evidence:**
- the element, its `replicas` and `zones`, and the defect;
- the path from a Person, with each drawn relationship ("Customer → Web App → Orders DB").

**Message:** "Orders DB runs as one replica, and a flow from Customer reaches it synchronously: Customer → Web App → Orders DB." **Remediation:**
- run at least two replicas in at least two zones; or
- make the call asynchronous; or
- if the caller has a fallback, suppress as *accepted* and name the fallback. A hard/soft attribute may come later (research §7.3).

**False positives.**
- Client apps that run on each user's device (a browser or mobile app), reached directly from a Person: suppress as *not real*. A client-device kind may come later.
- Lifting over-approximates: once a container is reached, all of its synchronous calls count as on the flow, even ones that only an admin feature uses.
- Managed services that replicate across zones internally need `replicas` and `zones` stated by the user, because v1 has no kind defaults. AWS lists S3, EFS, Aurora, DynamoDB, SQS and Kinesis Data Streams as multi-AZ by default [V] (REL10-BP01).

**False negatives:**
- flows started by external systems or schedules are not followed unless a Person is drawn;
- a single external system (for example a payment provider) is never judged;
- correlated failures (replicas sharing one database) are out of scope.

**Suppression key:** `REL-SPOF|element=…|defect=…`.

**Test models** (Rules: SP only):

| # | Model | Expected (complete) |
|---|---|---|
| SP1 | `person p`; `system s { container a [replicas=3, zones=2]; database d [replicas=1] }`; `r1: p -> a`; `r2: a -> d sync` | F REL-SPOF High on d, "one replica", path p →r1 a →r2 d. Nothing for a. |
| SP2 | SP1 with `a [replicas=2, zones=1]`, `d [replicas=2, zones=2]` | F REL-SPOF Medium on a, "one zone". |
| SP3 | SP1 with `a` (nothing stated), `d [replicas=1]` | NC a, missing replicas, zones. F High on d. |
| SP4 | `person p`; `system s { container a [replicas=2, zones=2]; queue q [replicas=3, zones=3]; container w [replicas=1] }`; `r1: p -> a`; `r2: a -> q async`; `r3: w -> q async` ("consumes from") | Nothing. q is required and fine; w is behind the queue. |
| SP5 | `person p`; `system s { container a [replicas=2, zones=2]; container b [replicas=1] }`; `r1: p -> a`; `r2: a -> b` | NC b, missing interaction on r2. |
| SP6 | `system s { container a [replicas=1] }` (no Person) | One model-level NC: no Person uses an element. |
| SP7 | `person p`; `system s { container a [replicas=1] }`; `r1: p -> s` | NC r1: p uses s, but no container of s. |
| SP8 | `person p`; `system s { container a [zones=1] }`; `r1: p -> a` | F Medium on a, "one zone"; evidence: replicas not stated. |
| SP9 | `person p`; `system s { container a [zones=2] }`; `r1: p -> a` | Nothing (zones ≥ 2 implies replicas ≥ 2). |
| SP10 | `person p`; `system s { container a [replicas=1, zones=2] }`; `r1: p -> a` | NC a: replicas and zones disagree (see the problems panel). |
| SP11 | SP2 with its finding suppressed as *accepted*, then a changed to `[replicas=1, zones=1]` | F High on a, key `…|defect=one-replica`, not suppressed. The old suppression is listed without a finding. |

## 9. Test model notation and harness

**Notation.**
- `person p`, `system s { … }`, `container a { … }`, `database d`, `queue q`, `component a1`, `code x`: an element of that kind with ID p, s, …. Braces hold children.
- Square brackets right after the ID hold attributes: `container a [replicas=2, zones=2, service=orders] { component a1 }`. An attribute that is not written is not stated.
- `r1: a -> b sync timeout=no`: relationship r1 drawn from a to b. `sync` and `async` state the interaction; absent means not stated. `binds sync a/call -> b/accept` lists a binding with its mode and its `from` and `to` port as qualified IDs (FF §6.7; ADR 0016: either end may be the `from`). Bindings never state interaction.
- Results:
  - `F RULE Severity …`: a finding.
  - `C RULE …`: a covered ARCH-CYCLE finding (§4 step 5), hidden or shown as stated.
  - `NC RULE subject, missing …`: a Not checkable entry.
  - "Nothing": no result.

**Harness.**
- Each test runs `scan::run(&model, &rules)` with exactly the rules in its Rules column. The expected column is the **complete** output of those rules: findings, covered findings, Not checkable entries and excluded counts. Rows marked "Panel" also join suppression state.
- Every test is a golden insta snapshot of results, keys and evidence.
- Proptests:
  - a model built in another insertion order gives the same results and byte-identical keys;
  - every scan-input mutation changes the scan input hash, and layout, rename and binding changes do not (§3.9).
- A fault-injection rule checks AC6d.
- The tests can be written in project TOML once the format spec exists. The notation above is for this document and for E8.

## 10. Terms

The glossary owner applies these entries from the merged block of `consistency-review.md` with D-18, D-29 and D-33; this spec uses them exactly.

| Term | Definition | Avoid |
|---|---|---|
| Not checkable | Said of a rule that could not be applied to a subject (an element, a relationship, a set of elements or the whole model) because the model lacks the data it needs, for example a timeout that is not stated. Each case is a **Not checkable entry** that names the missing data. | Unknown (reserved for verdicts), n/a, skipped, open |
| Suppression | The user's decision about a finding, with a written reason and a disposition: **accepted** (the finding is real and the user accepts it) or **not real** (the rule is wrong here). A suppressed finding stays visible as suppressed. | Ignore, waiver, exception, baseline |
| Not stated | The value of an attribute that the model does not give. A rule never treats it as yes, no or a number; it leads to Not checkable when stating it could change the outcome. | Unknown, null, default, n/a |
| Interaction | Whether the dependent end of a relationship waits for the other end's answer before it goes on: synchronous or asynchronous. Stated by the user; bindings never set it. Not a binding's mode (`sync`/`async`), which says how two ports are composed in a check. | Sync flag, call type, mode |
| Timeout | A relationship attribute: the dependent end gives up waiting after a set time, on both connection and request. It is not the Budget of a check and not a state machine's internal event named `timeout`. | Deadline, Budget |
| Runtime unit | An element that runs or stores data on its own: a Container, Database, Queue, or a Software System without children. Components and code elements run inside their container. | Process, deployable, node, service |
| Example cycle | The one shortest cycle that a cycle finding shows, step by step, with the relationship behind each step. | Witness, trace, path |
| Severity | How serious a finding is: High or Medium. | Level (a C4 level), priority, error/warning |
| Service (attribute) | The name of the service a Container belongs to; containers of one Software System with the same service count as one service. | Owner, team |

**Spec-internal terms.** These are never shown in the UI, or are shown only as plain words: level graph, lifted relationship, carrier, dependency direction, remote, in-process call, entry point, required and possibly required unit, accessor unit, finding key, covered finding, scan input hash.

## 11. Decisions and deviations

**Kept from the ADRs.**
- `scan` reads only `model` (ADR 0004).
- Findings are recomputed and never saved (ADR 0005 spirit); only suppressions are saved.
- A rule panic becomes an "internal error" entry on its own `scan-<n>-<rule-id>` thread (ADR 0012, ADR 0021).
- No bundled icons are involved (ADR 0007).

**Changes outside this spec (decided; owners per `decisions.md`).**
- ADR 0001 file list: `suppressions.toml` added by ADR 0020 (done).
- Format: the `kind` field with all seven kinds in format 1, the lowercase ID alphabet without `|`, `=` and `,`, the attribute keys of §2.6 and the suppressions file are in FF §4.1, §6.2, §6.3 and §8 (D-12, D-19).
- `design.md` §3, §4: one supervised thread per rule, `cancel_scan`, `Suppress { key, disposition, reason, date }`, and `scanHash` = the scan input hash (D-27, D-37; done).
- Glossary: §10 (GSS, D-18, D-29, D-33).
- `v1-slices.md` slice 5: "not stated" instead of "unknown", severities High/Medium (product manager, D-38).
- UX `flows.md` §4.14: rule IDs RUN-SYNC-CYCLE and DATA-SHARED-DB (mockups' REL-SYNC-CYCLE, REL-SHARED-DB and REL-CALLBACK are not rules), severity words, "not stated", the empty state of §3.11 (UX, D-18, D-46).

**Decided product items respected.**
- Database tables are not modeled. north-star.md's decided non-goal allows tables "at most as an attribute of Database elements for the scan". Relationship-level `tables` would go beyond that, so table-level clearing is cut from v1. Schema-per-service is drawn as separate Database elements, and private tables inside one schema are suppressed as *accepted*.
- Threshold tuning stays out. There is also no per-project rule disabling in v1: suppression with a reason is enough, and disabling would hide noise from the guardrail. `scan::run` still takes a rule list, for tests.

**Deviations from `docs/research/antipatterns.md`.**
1. REL-SPOF uses reachability over synchronous relationships, not dominators (§8).
2. There are no kind defaults (answers research open question 1).
3. A Database or Queue as the drawn source is read in reverse dependency direction (§3.1).
4. RUN-SYNC-CYCLE works on runtime units only.
5. Strongly connected sets use petgraph's iterative `kosaraju_scc`, not the recursive `tarjan_scc` the research named.
6. Severities are High/Medium, not error/warning/note. REL-NO-TIMEOUT moves from warning (research §6.2) to High, following AWS's High risk level.
7. ARCH-CYCLE ignores self-relationships (research A1 counts self-loops).
8. `owner`/`shared_ok` become `service`, and DATA-SHARED-DB needs `service` within one Software System. There is no `logical_db` or `tables`: one Database element is one logical database or schema.
9. Interaction and timeout are not derived from behavior models or bindings (research open question 3 stays open as a research item; §2.5).
10. Threat Dragon's many dispositions are reduced to *accepted* and *not real*, which is what the noise guardrail needs.

**Deviations from the draft.** These are all from the reviews:
- interaction is stated, not derived from bindings;
- `tables` is removed;
- DATA-SHARED-DB gives Not checkable for unstated services;
- folding is based on necessity and reports in-parent cycles;
- covered findings are joined at display;
- REL-NO-TIMEOUT judges in dependency direction;
- the self-relationship of a runtime unit is remote;
- the REL-SPOF table follows the §2.2 rule.

## 12. Closed questions

| # | question (revision 2) | decision |
|---|---|---|
| 1 | Suppression disposition *not real*, with the glossary amendment | D-18: accepted (§3.8, §10); ADR 0021 |
| 2 | Interaction always stated, never derived from bindings; one relationship carries a call and its reply | D-18, D-07: accepted (§2.5); ADR 0016, ADR 0021. RS9b stays as the test for a reply drawn as its own relationship |
| 3 | REL-SPOF ship rule tied to E8 | D-18: accepted (§1) |
| 4 | Entry points beyond Persons | D-18: Persons only in v1 (§8); revisit after E8 |
| 5 | Kind defaults for cloud product kinds | D-18: later, as a separate "assumed" tier, never plain stated values (§2.2) |
| 6 | Format items and the ADR 0001 file-list amendment | D-12, D-19, ADR 0020: in FF §4.1, §6.2, §6.3, §8 |
| 7 | Duplicate REL-NO-TIMEOUT findings for one call drawn at two levels | D-18: reported each in v1 and counted in E8; folding only if the noise shows (§6) |
| 8 | Scan trigger | D-18: explicit `run_scan` in slice 5; measure, then decide on scanning after every apply (§1, §3.9) |

## 13. Sources

Checked on 2026-11-02 [V]:
- AWS: [REL05-BP05](https://docs.aws.amazon.com/wellarchitected/latest/reliability-pillar/rel_mitigate_interaction_failure_client_timeouts.html) · [REL10-BP01](https://docs.aws.amazon.com/wellarchitected/latest/reliability-pillar/rel_fault_isolation_multiaz_region_system.html) (re-checked for this revision) · [Availability](https://docs.aws.amazon.com/wellarchitected/latest/reliability-pillar/availability.html)
- [Google Cloud: Build highly available systems](https://docs.cloud.google.com/architecture/framework/reliability/build-highly-available-systems)
- Papers: [Neri et al., arXiv 1906.01553](https://arxiv.org/abs/1906.01553) (PDF text, §4.3, §4.4) · [Taibi et al., arXiv 1908.04101](https://arxiv.org/abs/1908.04101) (PDF text, Tables 1.1, 1.3)
- [microTOSCA.yml](https://raw.githubusercontent.com/di-unipi-socc/microTOSCA/master/1.3/microTOSCA.yml)
- microservices.io: [Shared database](https://microservices.io/patterns/data/shared-database.html), [Database per service](https://microservices.io/patterns/data/database-per-service.html)
- C4: [Container](https://c4model.com/abstractions/container), [Queues and topics](https://c4model.com/abstractions/queues-and-topics), [Microservices](https://c4model.com/abstractions/microservices) (re-checked for this revision), [Notation](https://c4model.com/diagrams/notation)
- [Structurizr implied relationships](https://docs.structurizr.com/dsl/implied-relationships)
- [Arcan wiki](https://essere.disco.unimib.it/wiki/arcan/)
- [ZeroC Ice 3.5 Nested invocations](https://archive.zeroc.com/x/xQDg)
- petgraph 0.8.3: [kosaraju_scc](https://docs.rs/petgraph/latest/petgraph/algo/scc/kosaraju_scc/fn.kosaraju_scc.html) ("iterative and does two passes over the nodes"; returned in postorder, which `scan` re-sorts), [tarjan_scc](https://docs.rs/petgraph/latest/petgraph/algo/scc/tarjan_scc/fn.tarjan_scc.html) ("recursive")

Not verified:
- Martin's original ADP paper (cited through Wikipedia [S]).
- The claim that pool deadlocks also occur without cycles (a patent abstract seen only in search results [S]).
- Whether Taibi's "6.05" is 6.5.
- No license footer was visible on the Google Cloud page, so the rule texts here are our own words with links, not copies.
