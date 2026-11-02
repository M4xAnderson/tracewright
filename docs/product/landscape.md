# Landscape: alternatives to petrinet

As of 2026-11-02. Primary sources only; prices are list prices from vendor pages fetched on that date. **[unverified]** marks what the source did not state or what could not be fetched. Terms follow `GLOSSARY.md`; "diagram" is used only for other tools' artifacts.

The real competitor for our target user is not a tool: it is **a whiteboard sketch plus intuition plus a load test**, and for a few people a separate TLA+ spec. Every alternative below is measured against that.

## 1. Architecture modeling

| Tool | For whom | What it does | Where it stops | Price / license |
|---|---|---|---|---|
| **Structurizr** ([docs](https://docs.structurizr.com/), [repo](https://github.com/structurizr/structurizr)) | Developers doing C4 "diagrams as code" | DSL model of people, systems, containers, components; many views from one model; dynamic views; built-in inspections such as disconnected elements or missing technology ([inspections](https://docs.structurizr.com/workspaces/inspections)) | Dynamic views are drawings of one scenario, nothing executes. Inspections are completeness checks, not behavior. Manual layout lives in machine JSON | Local and export commands free; `server` prebuilt binaries need a license (price page not reachable **[unverified]**); open core Apache-2.0 |
| **LikeC4** ([site](https://likec4.dev/), [repo](https://github.com/likec4/likec4)) | Developers wanting architecture-as-code in VS Code | DSL with custom element kinds, interactive drill-down views, diff, `validate` for syntax and layout drift ([CLI](https://likec4.dev/tooling/cli/)), draw.io round-trip | No behavior models, no semantic rules beyond validation | MIT, free |
| **IcePanel** ([pricing](https://icepanel.io/pricing)) | Teams communicating architecture to mixed audiences | Collaborative C4 modeling with drill-down, tags, overlays, **flows** (message sequences, alternate and parallel paths) | Flows are narrated paths, not checked. SaaS only (no desktop or on-prem stated) | Free: 1 editor, 100 model objects. Growth $40, Scale $80 per editor per month (annual) |
| **Ilograph** ([pricing](https://www.ilograph.com/pricing.html), [sequence perspectives](https://www.ilograph.com/docs/editing/perspectives/sequence-perspectives/)) | Architects presenting large systems interactively | YAML model, multiple perspectives, zoomable **sequence perspectives**; offline Desktop edition | Sequences are diagrams only; nothing simulated or proved | Free web tier; Pro/Team paid; Desktop paid (labels on the page unclear **[unverified]**) |
| **draw.io** ([site](https://www.drawio.com/)) | Everyone | Free-form diagrams, desktop app, huge shape library | No model: boxes carry no semantics, so nothing can be checked | Apache-2.0, "free forever" |
| **Archi** ([site](https://www.archimatetool.com/)) | Enterprise architects using ArchiMate 3.2 | Open-source ArchiMate modeling, viewpoints, validator for notation rules | Enterprise-level, notation-heavy; no behavior checks stated | Open source, free (license not named on page **[unverified]**) |
| **Sparx Enterprise Architect** ([shop](https://sparxsystems.com/products/ea/shop/)) | Enterprise and systems engineering teams (UML, SysML, ArchiMate) | Broad modeling suite; state machine and BPMN simulation in higher editions **[unverified: page returned 403]** | Heavy, Windows-first, simulation is not proof | Commercial, per-seat **[unverified]** |

**Pattern:** these tools capture structure well and stop at the box. Behavior is drawn (flows, dynamic views, sequences), never executed or proved. LikeC4 and Structurizr are the closest in spirit (model-first, text in git) and are why our format must offer import and export to them (`docs/research/stack.md` §4).

## 2. Design-level verification and specification

| Tool | For whom | What it does | Where it stops | Price / license |
|---|---|---|---|---|
| **TLA+ / TLC / TLAPS** ([site](https://lamport.azurewebsites.net/tla/tla.html), [repo](https://github.com/tlaplus/tlaplus)) | Engineers of concurrent and distributed systems willing to write math | Specs in set theory plus temporal logic; TLC explicit-state model checking of finite instances; TLAPS proofs; Toolbox and VS Code extension | A separate text artifact with no link to the architecture; real learning curve; counterexamples are state dumps | MIT, free |
| **PlusCal** (translator `pcal.trans` in the TLA+ repo) | Same, preferring pseudo-code | Algorithm language that translates to TLA+ | Same as TLA+ | MIT, free |
| **Apalache** ([repo](https://github.com/apalache-mc/apalache), [docs](https://apalache-mc.org/docs/apalache/running.html)) | TLA+ and Quint users with large data domains | SMT-based checking up to a length (default 10); unbounded only with a user-written inductive invariant | Its docs: "bounded model checking is an incomplete technique" | Apache-2.0, free |
| **Quint** ([site](https://quint.sh/), [repo](https://github.com/informalsystems/quint)) | Developers who want TLA+ semantics with programming-language syntax (blockchain, fintech, cloud infra) | Typed spec language, REPL, random simulator, model checking via Apalache | Text-only; architecture still lives elsewhere. "Quint Studio" on a waitlist | Apache-2.0, free |
| **P** ([docs](https://p-org.github.io/P/whatisP/), [repo](https://github.com/p-org/P)) | Distributed-systems teams (used across AWS) | Communicating state machines; P-Checker systematic exploration with replayable error traces; PEx exhaustive; PVerifier proofs; **PObserve** checks production logs against the spec | Default mode is testing, not proof; code-like language, no architecture view | MIT, free |
| **FizzBee** ([repo](https://github.com/fizzbee-io/fizzbee), [site](https://fizzbee.io/)) | Developers of microservices and cloud apps who find TLA+ too mathematical | Python-like spec language, model checker, outputs state graphs and counterexamples, performance guide; pattern library (2PC, saga, leader election) | Text spec separate from architecture; which properties it decides is in guides I could not fetch **[unverified]** | Apache-2.0, free; macOS and Linux binaries |
| **Alloy 6** ([site](https://alloytools.org/)) | Designers of structural and relational models | Relational logic with mutable state and temporal logic; visualizer; 6.2.0 (2025-01-09) | Bounded by scope and steps; strong on structure, weaker on resource counting | Open source (exact license **[unverified]**) |

**Pattern:** these tools prove or test behavior well, with mature engines. Each one needs a second artifact, a text spec, that the user writes and keeps in sync by hand, and results come back in spec terms, not in architecture terms. Evidence that the problem is real: AWS reported that TLA+ specs of real systems found design bugs other techniques missed ([Newcombe et al., CACM 2015](https://www.amazon.science/publications/how-amazon-web-services-uses-formal-methods): "prevent serious but subtle bugs from reaching production"). Adoption outside such teams is low **[assumption, no data found]**.

## 3. Petri net tools

| Tool | For whom | What it does | Where it stops | Price / license |
|---|---|---|---|---|
| **TAPAAL** ([site](https://www.tapaal.net/), [verifypn](https://github.com/TAPAAL/verifypn)) | Researchers, students, formal-methods engineers | Timed-arc, colored and stochastic nets; CTL, LTL, HyperLTL; k-boundedness; top of MCC 2026 ([results](https://mcc.lip6.fr/2026/results.php)); 4.0.4 (2026-06-03) | Nets only, no architecture; you model the whole system as one net by hand | Free; engine GPL-3.0 |
| **CPN IDE** ([site](https://cpnide.org/)) | Colored Petri net modelers (successor of CPN Tools) | Editing and simulation of colored nets on Access/CPN | Windows download only; state-space analysis not stated; linked docs from 2022 | Free (license not stated **[unverified]**) |
| **Workcraft** ([repo](https://github.com/workcraft/workcraft), [Petri help](https://workcraft.org/help/petri/start)) | Asynchronous circuit and concurrency researchers | Petri nets, STGs; deadlock and custom Reach properties via unfolding (Punf + MPSat); failing property plays in the simulator | Circuit-design focus, no software-architecture concepts | MIT (backends' terms **[unverified]**) |
| **WoPeD** ([site](https://woped.dhbw-karlsruhe.de/)) | Teaching and business-process modeling | Workflow nets: modeling, token game, analysis; web successor WoPeD.next | Single workflow net, no composition of components; soundness listed for the web version only | LGPL, free |
| **ITS-Tools, LoLA, Tina** (`docs/research/verification.md` §3.1) | Model-checking experts | Strongest engines (MCC leaders) | Command-line engines, no modeling UX | GPL-3.0 or unclear |

**Pattern:** the theory and engines are excellent and free. The UX targets people who think in places and transitions, and none of them knows what a service, a queue or a C4 level is. Academic prior art maps architecture models to nets (AADL to Petri nets for deadlock and buffer checks, [Renault and Kordon, ISORC 2009](https://lip6.fr/Fabrice.Kordon/pdf/2009-ISORC.pdf)); I found no maintained tool doing this for C4 **[search on 2026-11-02, not exhaustive]**.

## 4. Code-level tools (in passing)

- **Kani** ([repo](https://github.com/model-checking/kani)): bounded model checking of Rust code; no concurrency; MIT/Apache-2.0.
- **Loom** ([docs](https://docs.rs/loom/latest/loom/)) and **Shuttle** ([docs](https://docs.rs/shuttle/latest/shuttle/)): exhaustive or randomized interleaving tests of Rust concurrency primitives; MIT and Apache-2.0.
- **Verus** ([repo](https://github.com/verus-lang/verus)): deductive proofs of Rust incl. state machines; MIT.
- **ArchUnit, dependency-cruiser** (`docs/research/antipatterns.md` §4): structural rules on code.

They find bugs in code that exists. They cannot tell you before coding that the design's components deadlock when combined. They are the later code-proof tier (`docs/research/theorem-proving.md`), not competitors for v1.

## 5. The gap

Nobody combines the three parts in one local artifact: (1) a C4 model the user already draws, (2) behavior models attached to its elements and combined through ports, and (3) push-button checks whose counterexamples are replayed on the architecture, with honest Unknown. Architecture tools have (1). Spec tools have the engine behind (3) but not (1) or (2). Petri net tools have the engine and the notation but not (1). The gap is real but narrow: it matters only to someone who designs concurrent or asynchronous systems **and** will spend minutes per element on a behavior model.

## 6. Where petrinet does NOT beat alternatives

- **Data-rich protocols** (idempotency keys, versioned state, leader election, request/reply matching, message order): TLA+, Quint, P and FizzBee are strictly more expressive. v1 queues are bags (ADR 0002).
- **Liveness and temporal logic** ("eventually", fairness): TLA+, TAPAAL, P. v1 properties are reachability only (ADR 0003).
- **Time** (deadlines, timeout arithmetic): TAPAAL timed-arc nets, UPPAAL.
- **Raw checking power** on large nets: TAPAAL and ITS-Tools will beat an in-house checker for years.
- **Communicating architecture** to a team or stakeholders: IcePanel, Ilograph, Structurizr have collaboration, sharing and polish; v1 is single-user and local.
- **Breadth of modeling**: Sparx EA and Archi cover UML, SysML and ArchiMate; v1 covers C4 only.
- **Cost of entry**: draw.io is free and needs no behavior model; for structure-only questions it is enough.
- **Code truth**: Kani, Loom and Verus check the code; v1 proves only the model, which can be wrong about the code.
