# Verification research: proving behavioural properties of architecture models

> Terms and decisions made after this research (GLOSSARY.md, CLAUDE.md, docs/adr/) take precedence over this file. It is kept as the research record.

As of 2026-11-02. Scope: how the tool can **prove** behavioural properties push-button on Petri-net and state-machine models attached to C4 elements, and how it can return an honest `Unknown` when it cannot. The anti-pattern fallback scan and interactive theorem proving / code-level functional correctness are covered in separate documents (`theorem-proving.md` covers the latter). This document treats code-level proofs as a later, complementary tier (see Recommendations, section R5).

Conventions: "P/T net" means place/transition net (weighted arcs, unbounded places, no extensions). "1-safe" means every reachable marking puts at most one token on each place. Items marked **[unverified]** could not be confirmed from a primary source during this research.

---

## 1. Petri net theory that matters

### 1.1 Decidability and complexity for P/T nets

| Property (P/T nets, general) | Decidable? | Complexity | Source |
|---|---|---|---|
| Reachability (is marking M reachable?) | Yes (Mayr, Kosaraju, Lambert) | **Ackermann-complete**: upper bound plus matching lower bounds from 2021 | [Czerwiński & Orlikowski 2021](https://arxiv.org/abs/2104.13866), [Leroux 2021](https://arxiv.org/abs/2104.12695), [Esparza & Nielsen survey](https://arxiv.org/abs/2411.01592) |
| Coverability (is some M' ≥ M reachable?) | Yes | EXPSPACE: Rackoff upper bound 2^(c·n log n) space, Lipton lower bound 2^(c√n) | [Esparza & Nielsen survey, §3](https://arxiv.org/html/2411.01592v1) |
| Boundedness | Yes (Karp–Miller tree) | EXPSPACE (same Rackoff/Lipton bounds); k-boundedness PSPACE-complete | [survey §3](https://arxiv.org/html/2411.01592v1) |
| Structural boundedness (bounded for every initial marking) | Yes | **Polynomial** (linear programming) | [survey §3](https://arxiv.org/html/2411.01592v1) |
| Liveness (every transition can always fire again) | Yes | Recursively equivalent to reachability | [survey §3](https://arxiv.org/html/2411.01592v1), [Cheng, Esparza, Palsberg 1995](https://archive.model.in.tum.de/um/bibdb/esparza/tcs95.pdf) |
| Deadlock-freedom | Yes | Poly-time reducible to reachability | [survey §3](https://arxiv.org/html/2411.01592v1) |
| Home state (a marking reachable from every reachable marking) | Yes | At least as hard as reachability | [Best & Esparza 2016](https://archive.model.in.tum.de/um/bibdb/esparza/ipl16.pdf) |
| Workflow-net soundness (classical) | Yes | **EXPSPACE-complete**; structural soundness EXPSPACE-complete; generalised soundness PSPACE-complete | [Blondin, Mazowiecki, Offtermatt, LICS 2022](https://arxiv.org/abs/2201.05588) |
| LTL-style linear-time model checking | Fragments decidable (e.g. GF of state predicates reduces to reachability); with place predicates in general undecidable | — | [survey §5](https://arxiv.org/html/2411.01592v1) |
| CTL / CTL* / μ-calculus (branching time) | **Undecidable** for unbounded nets | — | [survey §5](https://arxiv.org/html/2411.01592v1) |
| Bisimulation / language equivalence of labelled nets | Undecidable in general | decidable for bounded nets | [survey §4](https://arxiv.org/html/2411.01592v1) |

Practical reading: on *unbounded* P/T nets everything interesting is decidable but with non-elementary worst cases; no tool implements the full reachability algorithm in practice. Tools instead combine semi-decision procedures (state equation, invariants, Karp–Miller, bounded exploration) that often terminate quickly.

### 1.2 Classes that make things cheap

| Class | Key results | Source |
|---|---|---|
| **1-safe / bounded nets** | Finite state space, so *every* temporal property (LTL, CTL, μ-calculus) is decidable by exploration. Reachability, liveness and deadlock are **PSPACE-complete** for 1-safe nets. Marking equivalence for 1-safe nets PSPACE-complete; bisimulation DEXPTIME-complete. | [Cheng, Esparza, Palsberg 1995](https://archive.model.in.tum.de/um/bibdb/esparza/tcs95.pdf), [survey §3–4](https://arxiv.org/html/2411.01592v1) |
| **Free-choice nets** (if two transitions share an input place, they have the same preset) | Liveness co-NP-complete; **liveness of bounded free-choice nets polynomial**; deadlock NP-complete for (1-safe) free-choice. Commoner's theorem: a free-choice net is live iff every siphon contains an initially marked trap. Home states of live and bounded FC nets polynomial (all traps marked). Reachability polynomial for live, bounded, cyclic FC nets. | [survey §3](https://arxiv.org/html/2411.01592v1), [Jančar, concise proof of Commoner's theorem](https://arxiv.org/abs/2401.12067), [Jančar, Best, Devillers, Ošťádal 2026](https://arxiv.org/abs/2602.02352), [Best/Desel/Esparza on traps and home states](https://archive.model.in.tum.de/um/bibdb/info/esparza.BDE92.shtml.html) |
| **Free-choice workflow nets** | Soundness decidable in **polynomial time** (a WF-net is sound iff its short-circuited net is live and bounded; for FC nets that is polynomial). Asymmetric-choice WF-nets: co-NP-hard even when 3-bounded. | [van der Aalst 1997, "Verification of workflow nets"](https://www.academia.edu/470709/Verification_of_workflow_nets), [PAT/NUS paper citing the AC hardness](https://pat.comp.nus.edu.sg/resources/publications/pdf/pn12.pdf) |
| **Marked graphs, conflict-free nets** | Reachability polynomial for marked graphs and bounded conflict-free nets; boundedness quadratic for conflict-free; liveness and deadlock polynomial for conflict-free. | [survey §3](https://arxiv.org/html/2411.01592v1) |
| **Acyclic nets** | Reachability NP-complete (the state equation is exact for acyclic nets, so ILP decides it). | [survey §3](https://arxiv.org/html/2411.01592v1) |

Consequence for the product: an architecture-level model of request flows, locks and bounded queues is naturally **bounded** (and with capacity complements, 1-safe or k-bounded). That is the class where push-button proofs are realistic. Free-choice structure (choices that do not depend on other components' state) gives polynomial structural proofs for liveness/soundness; models that violate free-choice (e.g. "take from queue A only if lock B is free") fall back to state-space methods.

### 1.3 What breaks with extensions

| Extension | Effect | Source |
|---|---|---|
| **Inhibitor arcs** (zero tests) | Two or more: Turing-complete, reachability undecidable. Exactly one: decidable (Reinhardt). Bounded nets with inhibitor arcs stay finite-state, so still decidable. | [survey §3](https://arxiv.org/html/2411.01592v1) |
| **Reset arcs** (empty a place: cancellation, timeouts that drop work) | Reachability undecidable; **boundedness undecidable** (Dufourd, Finkel, Schnoebelen, ICALP 1998); undecidable with three reset arcs, decidable with two resettable places (Dufourd, Jančar, Schnoebelen 1999). Coverability stays decidable (well-structured transition systems), but is Ackermann-hard. Generalised soundness undecidable for reset workflow nets. | [Dufourd et al. 1998 record](https://www.informatik.uni-hamburg.de/TGI/pnbib/d/dufourd_c1.html), [Blondin, Finkel, McKenzie, WBTS](https://lmcs.episciences.org/3928), [Soundness of reset workflow nets, arXiv 2503.04440](https://arxiv.org/abs/2503.04440) |
| **Coloured / data nets** | Finite colour sets unfold to P/T nets (same theory, exponential blow-up). Infinite data domains (integers, unbounded lists) give Turing power in general. Tools such as TAPAAL and ITS-Tools unfold coloured nets for verification. | [pnml.org: high-level and symmetric nets in ISO/IEC 15909](https://www.pnml.org/), [TAPAAL](https://www.tapaal.net/) |
| **Time** (Merlin time Petri nets) | Reachability and boundedness **undecidable** for time Petri nets. For bounded TPNs the state class graph (Berthomieu & Diaz) is finite and decides untimed reachability/LTL. | [Jones, Landweber, Lien 1977](https://minds.wisconsin.edu/handle/1793/57994), [Tina toolbox](https://projects.laas.fr/tina/index.php) |
| **Timed-arc Petri nets** (token ages) | Reachability undecidable, coverability decidable (Ruiz et al. 1999; Abdulla & Nylén 2001); TAPAAL verifies them via bounded/extrapolated engines. **[unverified: from the TAPAAL authors' survey, not read in full]** | [Jacobsen, Jacobsen, Møller, Srba, SOFSEM 2011](https://www.springerprofessional.de/en/verification-of-timed-arc-petri-nets/3593174) |

Design rule that follows: **keep extensions behind a boundedness guarantee.** Inhibitor/reset arcs, colours and time are all harmless for decidability as long as the net is bounded (finite state space), because everything reduces to finite-state model checking. They become dangerous only combined with unbounded places.

---

## 2. State-space explosion mitigation

| Technique | What it does | Answer quality | Key sources / tools |
|---|---|---|---|
| **Explicit exploration** (BFS/DFS, hashing) | Enumerate reachable markings | Definitive when it terminates (bounded nets). Bloom-filter/bit-state hashing variants are **under-approximate** (can miss states). | [LoLA](https://theo.informatik.uni-rostock.de/theo-forschung/tools/lola/) |
| **Stubborn sets / partial-order reduction** | Explore only a subset of enabled transitions per state; preserves deadlocks, and with extra conditions reachability, LTL-X, liveness | Definitive (exact w.r.t. the preserved property class) | [Valmari 1989, stubborn sets](https://ai.dmi.unibas.ch/research/reading_group/valmari-apn1989.pdf); LoLA; TAPAAL ([Bønneland et al. 2019](https://vbn.aau.dk/en/publications/stubborn-versus-structural-reductions-for-petri-nets)) |
| **Structural reductions** | Rewrite the net (fuse places/transitions, remove redundant parts) preserving a property class before exploration | Definitive if rules preserve the queried property | [Bønneland et al. 2019](https://vbn.aau.dk/en/publications/stubborn-versus-structural-reductions-for-petri-nets); ITS-Tools; [Amat, Dal Zilio et al., polyhedral reductions](https://fi.episciences.org/10141) |
| **Symmetry reduction** | Merge states equivalent under net automorphisms (replicated components) | Definitive | [LoLA](https://theo.informatik.uni-rostock.de/theo-forschung/tools/lola/) (automatic symmetry detection) |
| **Sweep-line** | Delete states behind a progress measure | Definitive for safety; memory only | [LoLA](https://theo.informatik.uni-rostock.de/theo-forschung/tools/lola/) |
| **Symbolic: BDD / MDD / SDD, saturation** | Represent state sets as decision diagrams; saturation exploits event locality of asynchronous systems | Definitive; can be orders of magnitude faster than BDD BFS, but sensitive to variable order | [Ciardo, Lüttgen, Siminiceanu, saturation](https://ntrs.nasa.gov/api/citations/20010022506/downloads/20010022506.pdf); ITS-Tools ([GitHub](https://github.com/lip6/ITSTools)); MCC StateSpace leader "tedd" ([MCC 2026](https://mcc.lip6.fr/2026/results.php)) |
| **Unfoldings / complete finite prefixes** | Partial-order (causality) representation; deadlock and reachability checked on the prefix (often via SAT) | Definitive for bounded nets; excellent on highly concurrent, low-conflict models | [Esparza, Römer, Vogler (ERV)](https://mediatum.ub.tum.de/doc/1094240/document.pdf); Punf/MPSat in [Workcraft](https://workcraft.org/help/backend/mpsat) |
| **Invariants (P- and T-semiflows)** | Linear algebra on incidence matrix; P-invariant y with yᵀC = 0 gives yᵀM = yᵀM₀ for all reachable M | **Proves** safety/boundedness (sufficient, not necessary); cannot disprove | [Tina `struct`](https://projects.laas.fr/tina/index.php), [Christensen & Petrucci, modular invariants](https://www.informatik.uni-hamburg.de/TGI/pnbib/c/christensen_s5.html) |
| **Siphons / traps** | Commoner-style deadlock/liveness arguments; trap constraints strengthen the state equation | Exact for free-choice liveness; sufficient otherwise | [Jančar 2024](https://arxiv.org/abs/2401.12067), [Esparza & Melzer 2000](https://archive.model.in.tum.de/um/bibdb/info/esparza.EM00.shtml.html) |
| **State equation via LP/ILP** | M = M₀ + C·x, x ≥ 0 integral. Infeasible ⇒ M unreachable. | **Proves unreachability** (over-approximation); a feasible solution is not a witness unless realised as a firing sequence. CEGAR refinement often terminates fast on negative instances. | [Wimmel & Wolf, CEGAR on the state equation](https://lmcs.episciences.org/1036); [Esparza & Melzer](https://archive.model.in.tum.de/um/bibdb/info/esparza.EM00.shtml.html) |
| **Bounded model checking (SAT/SMT)** | Unroll k steps (interleaving, step or process semantics) | **Semi-decision**: finds counterexamples up to k; proves nothing beyond k unless combined with a completeness threshold or induction | [Heljanko, CONCUR 2001](https://www.informatik.uni-hamburg.de/TGI/pnbib/h/heljanko_k2.html); [Apalache docs: "Bounded model checking is an incomplete technique"](https://apalache-mc.org/docs/apalache/running.html) |
| **k-induction / PDR (IC3) with SMT** | Find inductive invariants automatically | Definitive when it converges (yields an inductive invariant = checkable certificate); semi-decision otherwise | [Amat, Dal Zilio, Hujsa, PDR for generalized Petri nets, TACAS 2022](https://hal.laas.fr/hal-03545594); [SMPT](https://github.com/nicolasAmat/SMPT) |
| **Random / guided walks** | Simulate to falsify | Only finds counterexamples | [Tina `walk`](https://projects.laas.fr/tina/index.php); PetriSpot in [ITS-Tools](https://github.com/lip6/ITSTools) |
| **Karp–Miller coverability graph** | Finite abstraction with ω for unbounded places | Decides boundedness and coverability; does not decide reachability/liveness | [Tina `tina`](https://projects.laas.fr/tina/index.php), [survey §3](https://arxiv.org/html/2411.01592v1) |

The winning tools are **portfolios**. ITS-Tools combines structural reductions, SMT over the state equation, decision diagrams, partial-order explicit engines (LTSmin), LTL via Spot and random walks ([README](https://github.com/lip6/ITSTools)). Bønneland et al. report that stubborn sets and structural reductions combined give further nontrivial reductions ([2019](https://vbn.aau.dk/en/publications/stubborn-versus-structural-reductions-for-petri-nets)).

**Evidence each technique produces** (this is what drives the verdict model in R3):

- Proof certificates that can be checked independently: P-invariants / trap sets (linear check), an LP/ILP infeasibility certificate (Farkas dual), an inductive invariant from PDR/k-induction (one SMT query per check), or "complete exploration finished" (not independently checkable without redoing it).
- Counterexamples: firing sequences (explicit/BMC/walk), lassos (for liveness/LTL), partial-order runs (unfoldings).

---

## 3. Existing tools, formats and Rust crates

### 3.1 Petri net verification tools

| Tool | What it is | License | Embeddable from Rust? | Notes |
|---|---|---|---|---|
| **TAPAAL / verifypn** | GUI and engines for P/T, coloured and timed-arc nets; CTL, LTL, HyperLTL, k-bound; reduction via stubborn sets and structural rules | verifypn **GPL-3.0-or-later**; authors invite requests for an alternative license | Subprocess (C++ binary, reads PNML and XML queries). Linking would make our binary GPL. | Latest TAPAAL 4.0.4 (2026-06-03). [tapaal.net](https://www.tapaal.net/), [verifypn](https://github.com/TAPAAL/verifypn) |
| **ITS-Tools** | Portfolio checker (GAL + PNML), see §2 | **GPL-3.0** | Subprocess; needs Java 21+ (an experimental GraalVM native image exists) | [GitHub](https://github.com/lip6/ITSTools), [MCC wrapper](https://github.com/yanntm/ITS-Tools-MCC) |
| **LoLA 2** | Explicit-state checker for CTL* fragments; strong on deadlock/reachability; symmetry, POR, sweep-line, Bloom filters, coverability graph; can output a witness path | Official page states no license; the unofficial GitHub mirror is **AGPL-3.0** **[unverified upstream]** | Subprocess (C++); used as the backend of cargo-check-deadlock | [Rostock page](https://theo.informatik.uni-rostock.de/theo-forschung/tools/lola/), [mirror](https://github.com/hlisdero/lola) |
| **Tina toolbox** | tina/sift (state spaces, state classes for time PNs), selt (state/event LTL, counterexamples replayable in nd), muse (μ-calculus), struct (semiflows), plan, walk, play, nd | Not stated on the site **[unverified; historically binary-only freeware]** | Subprocess | Supports inhibitor/read arcs, time PNs with priorities/stopwatches, PNML via ndrio. [Tina](https://projects.laas.fr/tina/index.php) |
| **SMPT** | SMT-based reachability (BMC, k-induction, PDR, state equation, polyhedral reductions) | **GPL-3.0**, Python | Subprocess | Bronze in Reachability at MCC 2025 and 2026. [GitHub](https://github.com/nicolasAmat/SMPT) |
| **CPN Tools → CPN IDE** | Editing and simulation of coloured nets (Standard ML inscriptions); CPN IDE adds a JS editor on Access/CPN. CPN Tools development has stopped. | CPN IDE license not found **[unverified]** | No (Java/ML stack); interesting as a UX reference only | [cpnide.org](https://cpnide.org), [CPN IDE paper](https://research.tue.nl/en/publications/cpn-ide-an-extensible-replacement-for-cpn-tools-that-uses-accessc/), [cpntools.org](https://cpntools.org/documentation/) |
| **Workcraft** | Framework for Petri nets, STGs and more; verification via Punf (unfolding) + MPSat; failing property gives a trace that plays in the simulator | Workcraft **MIT**; Punf/MPSat license not found **[unverified]** | Subprocess (Java + native backends) | [GitHub](https://github.com/workcraft/workcraft), [Petri help](https://workcraft.org/help/petri/start) |
| **APT** | Analysis of Petri nets and labelled transition systems (Java) | GPL-2.0 | Subprocess | [GitHub](https://github.com/CvO-Theory/apt) |
| **petrivet** | **Pure-Rust** Petri net model checker: boundedness, deadlock-freedom, liveness, coverability, reachability; invariants, siphons/traps, Commoner–Hack; reachability graph and Karp–Miller; PNML import/export. Only ordinary nets so far (weights and capacities on roadmap). No releases, 2 stars, API unstable. | **LGPL-3.0** | Yes (library), but immature | Entered MCC 2026 (4th in StateSpace and GlobalProperties). [GitHub](https://github.com/MichaelOwenDyer/petrivet), [MCC submission](https://github.com/MichaelOwenDyer/petrivet-mcc) |

**Model Checking Contest** (ranked tools only):

| Category | MCC 2025 | MCC 2026 |
|---|---|---|
| StateSpace | 1. tedd, 2. GreatSPN+red | 1. tedd, 2. ITS-Tools, 3. TY, 4. petrivet |
| GlobalProperties (deadlock, quasi-liveness, stable marking, liveness, 1-safe) | 1. ITS-Tools, 2. TAPAAL | 1. ITS-Tools, 2. TAPAAL, 3. TY, 4. petrivet |
| UpperBounds | 1. ITS-Tools, 2. TAPAAL | 1. ITS-Tools, 2. TAPAAL, 3. TY |
| Reachability formulas | 1. TAPAAL, 2. ITS-Tools, 3. smpt | 1. TAPAAL, 2. ITS-Tools, 3. smpt, 4. TY |
| CTL | 1. TAPAAL, 2. ITS-Tools | 1. TAPAAL, 2. ITS-Tools, 3. TY |
| LTL | 1. TAPAAL, 2. ITS-Tools, 3. EnPac | 1. TAPAAL, 2. ITS-Tools, 3. TY |

Sources: [MCC 2025 results](https://mcc.lip6.fr/2025/results.php), [MCC 2026 results](https://mcc.lip6.fr/2026/results.php). LoLA and Tina did not take part in 2025/2026. I could not find descriptions of "tedd" or "TY" (Ferrite Technology) **[unverified]**. The GlobalProperties examinations in MCC are exactly the property set this project needs for v1.

### 3.2 Exchange format: PNML

- PNML is the XML transfer format of **ISO/IEC 15909-2** (2011, corrigendum 2012); Part 1 defines P/T nets, symmetric nets and high-level nets; **ISO/IEC 15909-3:2021** adds extensions and structuring mechanisms. pnml.org hosts RELAX NG grammars, EMF metamodels and PNTDs for reset/inhibitor nets; a 2026 grammar is listed as work in progress. ([pnml.org](https://www.pnml.org/))
- PNML is the MCC input format, so PNML export gives access to every MCC tool ([MCC](https://mcc.lip6.fr/2026/)). It does not standardise queries; MCC uses its own XML property language, and TAPAAL reads XML queries ([verifypn](https://github.com/TAPAAL/verifypn)).

### 3.3 Rust crates

Numbers are crates.io data fetched 2026-11-02 (total downloads, last update).

| Need | Crate | Version / activity | License | Maturity (honest) |
|---|---|---|---|---|
| SMT | [`z3`](https://crates.io/crates/z3) (+ `z3-sys`) | 0.22.0, 2.26 M dl, updated 2026-10-28; `bundled`/`vendored` features build Z3 from source | MIT (Z3 itself MIT, [repo](https://github.com/Z3Prover/z3)) | Mature, widely used; C++ build dependency is heavy. |
| SMT via subprocess | [`easy-smt`](https://crates.io/crates/easy-smt), [`smtlib`](https://crates.io/crates/smtlib), [`rsmt2`](https://crates.io/crates/rsmt2) | easy-smt 0.3.2 (1.57 M dl); smtlib 0.4.0 (2026-09); rsmt2 last 2022 | MIT/Apache | easy-smt is solid; avoids linking, any SMT-LIB solver works. |
| SAT (native Rust) | [`splr`](https://crates.io/crates/splr) | 0.19.0, updated 2026-08 | MPL-2.0 | Active, single author. |
| SAT (native Rust) | [`varisat`](https://crates.io/crates/varisat) | 0.2.2, **last release 2020-09** | MIT/Apache | Unmaintained; avoid for new work. |
| SAT (native Rust) | [`batsat`](https://crates.io/crates/batsat) | 0.6.0, 2025-01 | MIT | MiniSat port, stable but quiet. |
| SAT (bindings) | [`cadical`](https://crates.io/crates/cadical), [`rustsat`](https://crates.io/crates/rustsat) | cadical 0.1.16 (2025-04); rustsat 0.7.5 (2026-01), unified interface over several solvers | MIT | rustsat is the best-maintained umbrella. |
| BDD / DD | [`oxidd`](https://crates.io/crates/oxidd) | 0.13.0, 2026-09; BDD, BCDD, ZBDD, MTBDD, multi-threaded apply ([docs](https://docs.rs/oxidd/latest/oxidd/)) | MIT OR Apache-2.0 | Research-grade but active; most capable pure-Rust DD library. |
| BDD | [`biodivine-lib-bdd`](https://crates.io/crates/biodivine-lib-bdd) | 0.6.3, 2026-02 | MIT | Small, stable, used in BioDivine. |
| BDD bindings | [`cudd-sys`](https://crates.io/crates/cudd-sys), [`sylvan-sys`](https://crates.io/crates/sylvan-sys) | alpha / 2023 | — | Low-level FFI, little activity. |
| LP / ILP (state equation) | [`good_lp`](https://crates.io/crates/good_lp) (front-end), [`highs`](https://crates.io/crates/highs), [`microlp`](https://crates.io/crates/microlp) (pure Rust), [`russcip`](https://crates.io/crates/russcip) (SCIP) | good_lp 1.15.3 (4.37 M dl, 2026-08); microlp 0.7.0 (2026-10) | MIT / Apache-2.0 | Mature; HiGHS via good_lp is the pragmatic choice. |
| Petri nets | [`petrivet`](https://github.com/MichaelOwenDyer/petrivet) | unreleased | LGPL-3.0 | See 3.1. |
| Petri nets | [`netcrab`](https://crates.io/crates/netcrab) | 1.0.2, construction + export (PNML, LoLA, DOT) | MIT/Apache | Data structure only, no analysis. |
| Petri nets | [`pnets`](https://crates.io/crates/pnets), [`pns`](https://crates.io/crates/pns), [`petri-net-simulation`](https://crates.io/crates/petri-net-simulation) | pnets last 2021; others are simulators | various | Not verification tools. |
| Process mining (Petri nets, token replay) | [`process_mining`](https://crates.io/crates/process_mining) | 0.6.2, 2026-08 | MIT/Apache | Useful later for log-vs-model conformance. |
| Graphs | [`petgraph`](https://crates.io/crates/petgraph) | 0.8.3 | MIT/Apache | SCCs, for liveness on explored graphs. |
| Traces | [`itf`](https://docs.rs/itf) | 0.4.0, reads Apalache/Quint ITF traces | Apache-2.0 | Small and stable. |

No mature Rust-native Petri net *model checker* exists today. The building blocks (LP, SAT, SMT, DD, graphs) are mature enough that an in-house checker for bounded nets is realistic.

---

## 4. Alternatives and complements to Petri nets

| Formalism / tool | Strength | Proof capability | License / embedding | Best fit in this product |
|---|---|---|---|---|
| **Petri nets** | Concurrency, resources, token counting (pools, semaphores, queue occupancy), local causality; graphical. Rich structural theory (§1–2). | Full decision procedures for bounded nets; structural certificates | many tools (§3) | Default for flows, locks, bounded queues, worker pools, sagas. |
| **State machines / statecharts** | Per-component lifecycle (connection, circuit breaker, order status). Hierarchy and history are well known to developers. | Finite state machines compose into a finite product; translation to 1-safe nets is direct (one place per state, one token). | — | Component-level behaviour; compile to nets for checking. |
| **TLA+ (TLC) / Apalache / Quint** | Data-rich specs (sets, functions, records), distributed protocols. Apalache: SMT (Z3) BMC up to `--length` (default 10); unbounded proofs only via user-supplied **inductive invariants** (three `check` runs). Counterexamples as TLA+ or ITF JSON. | TLC: explicit, complete on finite instances; Apalache: bounded or inductive | TLA+ tools MIT ([repo](https://github.com/tlaplus/tlaplus)); Apalache Apache-2.0 ([repo](https://github.com/apalache-mc/apalache), [docs](https://apalache-mc.org/docs/apalache/running.html)); Quint Apache-2.0 ([repo](https://github.com/informalsystems/quint)); JVM subprocess | Export target for data-heavy protocols (consensus, idempotency keys, versioned state) that do not fit token games. |
| **Alloy 6** | Relational structure plus LTL with past; mutable `var` fields. Bounded by scope and `steps` (default 10); complete unbounded checking for `1..` steps needs NuSMV/nuXmv. | Bounded, or complete via external model checker on small models | [alloytools.org](https://alloytools.org/alloy6.html); repo license shows as NOASSERTION on GitHub **[unverified exact terms]** | Structural constraints (C4 topology rules, permission models), less for runtime behaviour. |
| **Timed automata (UPPAAL)** | Real-time clocks, deadlines, timeouts with quantitative bounds; `verifyta` CLI. | Complete for timed-automata reachability and its TCTL fragment | Free for academic use only; **commercial license required otherwise**, licence fetched online ([downloads](https://uppaal.org/downloads/)) | Only if timing *values* matter (SLA, "timeout < retry interval"). The licence makes bundling impractical. |
| **Process algebra (mCRL2)** | Processes with data, modal μ-calculus with data via PBES, bisimulation reduction, LTS comparison, counterexamples | Complete on finite state spaces; equivalence checking for refinement | Boost Software License ([repo](https://github.com/mcrl2org/mCRL2)); release 202607.0 ([site](https://www.mcrl2.org/web/index.html)) | Backend for compositional refinement checks (component model vs interface) via bisimulation minimisation. |
| **P language** | Communicating state machines (actor model) for distributed systems; P-Checker systematic testing, PEx exhaustive checking, PVerifier deductive proofs, **PObserve** checks production logs against P specs. Used across AWS (S3, DynamoDB, EC2, ...). | Testing by default; exhaustive/proof backends exist | MIT ([repo](https://github.com/p-org/P), [docs](https://p-org.github.io/P/whatisP/)) | Closest industrial analogue to "architecture behaviour model + conformance". Good benchmark for UX, and PObserve is a model for later log conformance. |
| **Session types** | Typed protocols, deadlock freedom by construction; Rumpsteak gives deadlock-free async multiparty protocols in Rust using k-MC and asynchronous subtyping | Proof by typing (no state exploration) | [Cutner, Yoshida, Vassor, PPoPP 2022](https://arxiv.org/abs/2112.12693) | Later: generate Rust skeletons from interaction models so that code conforms by construction. |

**When each is a better fit (architecture-level behaviour):**

- *Request flows, worker pools, bounded queues, locks/semaphores, fork/join of async tasks, saga orchestration*: Petri nets. Token counting and concurrency are native, and deadlock/soundness are the right questions.
- *Component lifecycles* (circuit breaker, connection states, order status): state machines, compiled to nets.
- *Retries with timing* ("3 retries with exponential backoff finish before the upstream 30 s timeout"): timed automata or time Petri nets. Without concrete numbers, model timeouts as nondeterministic choice in an untimed net. That over-approximates and is sound for safety.
- *Protocols with rich data* (idempotency keys, version vectors, leader election): TLA+/Apalache or mCRL2.
- *Pairwise message protocols between services*: session types / choreographies, which give deadlock freedom by construction.

---

## 5. Mapping software concepts to nets, and code-level tools

### 5.1 Pattern catalogue (net encodings)

| Concept | Standard encoding | Property questions | Notes / sources |
|---|---|---|---|
| Mutex | Place `lock` with 1 token; `acquire` consumes it, `release` returns it | Mutual exclusion = P-invariant `lock + Σ critical = 1`; deadlock from lock ordering | Used by [cargo-check-deadlock](https://github.com/hlisdero/cargo-check-deadlock) for `std::sync::Mutex` and `Condvar` |
| Semaphore / connection pool (n) | Place with n tokens | Boundedness, starvation (liveness) | — |
| Bounded channel / queue (capacity k) | Place `items` + complement place `free` with k tokens | k-boundedness by construction (P-invariant `items + free = k`); "full queue blocks producer" deadlocks | Complement places keep the net bounded, so all properties stay decidable (§1.3). |
| Unbounded queue | Unbounded place | Boundedness check flags it; reachability expensive | Prefer explicit capacities in the modelling language. |
| FIFO order | Not expressible with plain tokens (tokens are indistinguishable); needs coloured nets with list colours, or one place per slot | — | Keep FIFO out of v1. Order rarely matters for deadlock and soundness. |
| Async task spawn / join (fork/join) | Fork transition puts tokens into task places; join transition waits for all | Soundness (every spawned task is joined or detached), deadlock | Spawning in a loop needs a bound. cargo-check-deadlock also lists loops as unsupported. |
| Actor / mailbox | Place per mailbox (bounded); actor = state machine whose transitions consume messages | Unreceived messages at termination = improper completion | Go channel analysis via behavioural types: [Lange et al., POPL 2017](https://mrg.cs.ox.ac.uk/publications/fencing-off-go-liveness-and-safety-for-channel-based-programming/) |
| Request/response with timeout | Nondeterministic choice between `reply` and `timeout` transitions; late reply must be absorbed (else garbage tokens) | "Every request ends in exactly one outcome" = workflow soundness; the classic bug is that late replies are never consumed | Cancelling in-flight work needs reset arcs or cancellation regions. |
| Retry (max n) | Counter place with n tokens, or unrolled states | Termination (no infinite retry), bounded load | Unbounded retry plus unbounded queue makes the net unbounded. |
| Cancellation / cancel region | Reset arcs (YAWL cancellation regions) | Soundness of reset WF-nets: decidability is limited (§1.3); the YAWL work uses reachability analysis and reduction rules | [Wynn, van der Aalst, ter Hofstede, Edmond 2006](https://www.vdaalst.com/publications/p336.pdf). For bounded nets: encode reset as a finite set of transitions (one per possible count) or use complement places. |
| Saga (orchestrated, with compensations) | Workflow net: forward steps T1..Tn; failure of Ti routes to compensations C(i-1)..C1 in reverse | Soundness (always reaches `committed` or `compensated`, no leftover tokens); no dead transitions | No primary source found that verifies sagas as Petri nets specifically **[gap]**. Reversible-net view: [arXiv 2312.16714](https://arxiv.org/abs/2312.16714). |
| Circuit breaker, connection lifecycle | 1-safe state-machine subnet | Reachability of illegal state combinations with other components | — |

### 5.2 Deriving nets from Rust

- **cargo-check-deadlock** translates **MIR to a Petri net** and asks LoLA whether a deadlock is reachable. Supported: `Mutex`, `Condvar`, `thread::spawn`/`join`. Unsupported: async, channels, RwLock, Barrier, tokio/external sync, closures outside `spawn`, threads in loops; collections may cause false results. Dual MIT/Apache, needs nightly + `rustc-dev`. It builds on Tom Meyer's "A Petri-Net Semantics for Rust" (Granite). ([repo](https://github.com/hlisdero/cargo-check-deadlock), [Granite](https://github.com/Skasselbard/Granite), [netcrab](https://crates.io/crates/netcrab))
- **Zhang & Liu** (arXiv 2212.02754, revised 2025): program-to-net translation with ownership/lifetime semantics and pointer analysis, context-aware slicing, then reachability. They report fewer false positives/negatives than LockBud. ([arXiv](https://arxiv.org/abs/2212.02754))
- **async Rust**: I found **no** primary work translating `async`/`Future`/tokio programs to Petri nets **[gap]**. Behavioural-type extraction for Go channels ([Lange et al.](https://mrg.cs.ox.ac.uk/publications/fencing-off-go-liveness-and-safety-for-channel-based-programming/)) and Rumpsteak ([arXiv](https://arxiv.org/abs/2112.12693)) are the closest. Import from async Rust is therefore a research item, not an engineering item.

### 5.3 Code-level Rust verification tools (plug-in points later)

| Tool | Technique | Concurrency? | License | Role for conformance |
|---|---|---|---|---|
| [Kani](https://github.com/model-checking/kani) | Bit-precise bounded model checking (CBMC) | **No**: concurrent code compiled as sequential, `await` unsupported ([feature table](https://model-checking.github.io/kani/rust-feature-support.html)) | Apache-2.0 | Prove per-transition obligations (a handler maps inputs to the outcomes the model allows; no panic on any path). |
| [Prusti](https://github.com/viperproject/prusti-dev) (now `prusti/prusti`) | Deductive (Viper) | Limited | see repo | Contracts on functions that implement model transitions. |
| [Creusot](https://github.com/creusot-rs/creusot) | Deductive (Why3) | Limited | LGPL-2.1 | Same; see theorem-proving.md. |
| [Verus](https://github.com/verus-lang/verus) | SMT-based deductive, ghost state | Has concurrency support via ghost resources **[not verified in detail]** | MIT | Prove that an implementation refines a state-machine spec. |
| [Loom](https://docs.rs/loom/latest/loom/) | Exhaustive interleaving exploration under the C11 memory model with state reduction; optional preemption bound | Yes (threads, atomics) | MIT | Check that a concrete lock/queue implementation behaves like its net pattern. Exhaustive only within bounds. |
| [Shuttle](https://docs.rs/shuttle/latest/shuttle/) | Randomized (`check_random`), PCT (`check_pct`), DFS over all interleavings (`check_dfs`), `replay` of failing schedules; tokio wrappers | Yes | Apache-2.0 | Replay model counterexamples as schedules against real code. Randomized mode proves nothing ([README](https://github.com/awslabs/shuttle)). |

---

## 6. Counterexample presentation

How tools present witnesses:

- **Firing sequence replayed in a simulator.** Workcraft reports one or more traces leading to the violation and plays them in its simulator ([Petri help](https://workcraft.org/help/petri/start)). Tina's `selt` saves counterexamples for replay in `nd` ([Tina](https://projects.laas.fr/tina/index.php)). TAPAAL integrates editor, simulator and verifier ([tapaal.net](https://www.tapaal.net/)). LoLA can print a witness path ([LoLA](https://theo.informatik.uni-rostock.de/theo-forschung/tools/lola/)).
- **State sequences as data.** Apalache writes `counterexample.tla` ([docs](https://apalache-mc.org/docs/apalache/running.html)). ITF JSON has `vars`, `states` and an optional `loop` index for lasso traces ([ADR-015](https://apalache-mc.org/docs/adr/015adr-trace.html), Rust reader [`itf`](https://docs.rs/itf)).
- **Partial-order witnesses.** Unfolding and process-semantics BMC produce *runs* (causal partial orders) instead of interleavings ([ERV](https://mediatum.ub.tum.de/doc/1094240/document.pdf), [Heljanko](https://www.informatik.uni-hamburg.de/TGI/pnbib/h/heljanko_k2.html)). These are shorter and show what actually depends on what.
- **Schedules replayable against code.** Shuttle `replay` and Loom checkpoints ([Shuttle](https://docs.rs/shuttle/latest/shuttle/), [Loom](https://docs.rs/loom/latest/loom/)).

Ideas for the canvas (design proposals, not sourced):

1. **Trace as a timeline scrubber** under the canvas. Each step fires one transition and animates tokens along input arcs into the transition and out along output arcs. Keyboard step/back, play speed.
2. **Causal layering (step semantics).** Group independent transitions into one "step" so concurrent activity animates simultaneously. This matches partial-order witnesses and avoids the misleading total order of an interleaving.
3. **Cross-level highlighting.** Every net transition carries a provenance link to the C4 element/relationship it models. While the trace plays, the C4 diagram highlights the active relationships (a generated sequence-diagram view of the same trace).
4. **Explain the verdict at the final state.** Deadlock: list each relevant transition with the input place that lacks tokens (and which component holds the token, e.g. "lock B held by Worker 2"). Unboundedness: show the *pumping* subsequence from Karp–Miller that strictly increases the marking, rendered as a loop badge. Liveness violation: render a lasso (prefix + highlighted cycle) like ITF's `loop`.
5. **Proof display for `Proved`.** Show the certificate in domain terms. A P-invariant becomes "lock + in_critical_A + in_critical_B = 1, therefore mutual exclusion"; a trap becomes "this set of places can never be emptied". Users can inspect *why* something holds, not only that it does.
6. **Minimise first.** Shortest counterexample (BFS) by default; optionally slice the trace to transitions causally relevant to the violation.
7. **Export.** ITF-compatible JSON for traces, so traces can be replayed in tests or turned into Shuttle schedules later.

---

## 7. Composition across C4 levels

Relevant theory:

- **Modular nets, modular analysis.** Modules sharing places and/or transitions; invariants of the whole net can be built from module invariants, and there is modular state-space analysis for (coloured) nets. ([Christensen & Petrucci 2000](https://www.informatik.uni-hamburg.de/TGI/pnbib/c/christensen_s5.html), [CPN precursor](https://tidsskrift.dk/daimipb/article/view/6626))
- **Refinement and abstraction.** Stepwise refinement of transitions/places and abstraction of subnets into transitions, as a divide-and-conquer analysis approach ([Suzuki & Murata 1983](https://www.informatik.uni-hamburg.de/TGI/pnbib/s/suzuki_i7.html)). Which properties are preserved under which block conditions was **not verified** from the paper text.
- **Associative composition with interfaces.** Composition by merging labelled interface elements is not associative in general. Reisig's double-sided (left/right) interfaces make it associative; Heraklit applies this to modular, hierarchical Petri net models. ([Reisig 2019, Acta Informatica 56(3)](https://informatik.hu-berlin.de/de/forschung/gebiete/top/Publikationen2019), [Heraklit, arXiv 2202.01830](https://arxiv.org/abs/2202.01830), [composition calculus, arXiv 2408.15031](https://arxiv.org/abs/2408.15031))
- **Open nets, service partners, operating guidelines** (Wolf, Massuthe, Lohmann; tools Fiona/Wendy): interface places and the question "does this component have a partner such that the composition terminates properly?" I know this work but could not retrieve a primary source in this session **[unverified]**.
- **Bisimulation-based minimisation** (mCRL2) lets a verified component be replaced by its minimised behaviour over interface actions. Branching bisimulation preserves the properties expressible over the remaining actions ([mCRL2](https://www.mcrl2.org/web/index.html); preservation claim from general theory **[not re-verified here]**).
- **Hierarchical CPN substitution transitions** (CPN Tools/IDE) are syntactic hierarchy only. The flattened net is verified as a whole ([cpntools.org](https://cpntools.org/documentation/)).

Proposed composition scheme (design, builds on the above):

1. Each C4 element with a behaviour model exposes an **interface**: typed ports (input/output places, or shared transitions for synchronous calls). C4 relationships connect ports.
2. **Flattening** (default, always sound): compose all behaviour models along relationships into one net, then verify. Simple, but the state space grows with the number of components.
3. **Abstraction by contract** (scalable): verify each Component-level model against its **interface contract**, a small state machine or workflow net over port actions. The check is a refinement (trace inclusion or branching bisimulation after hiding internal transitions; decidable for bounded nets). At Container level, substitute contracts for the implementations. Safety/deadlock properties proven on the composition of contracts carry over if refinement preserves them. Trace inclusion is enough for safety; deadlock and liveness need a stronger relation (e.g. failures/branching bisimulation), so the tool must pick the relation per property.
4. **Workflow-module discipline**: if each component model is a sound workflow net with one entry and one exit, then replacing a transition by a sound WF-net preserves soundness (the classic refinement result for WF-nets) **[unverified in this session; check van der Aalst's work]**. That supports drill-down: a Container-level transition "handle request" refined by a Component-level WF-net.
5. Invariants compose: P-invariants of modules combine into invariants of the composed net ([Christensen & Petrucci](https://www.informatik.uni-hamburg.de/TGI/pnbib/c/christensen_s5.html)), so certificates can be cached per component and re-checked incrementally.

---

## Recommendations for this project

### R1. Net class and properties for v1

- **Net class:** P/T nets with arc weights and **mandatory explicit bounds**: place capacities, compiled to complement places, so every v1 model is bounded by construction. Allow inhibitor arcs and "reset" only on bounded places, compiled to plain transitions where possible. State machines compile to 1-safe subnets. No time and no colours in v1. Model timeouts as nondeterministic choice.
- **Properties (MCC GlobalProperties plus workflow soundness):**
  1. Deadlock-freedom, distinguishing *proper termination* (designated final markings) from *deadlock*.
  2. Safety: "marking predicate never holds" (mutual exclusion, "never two leaders", "never >k in flight").
  3. k-boundedness per place (queue overflow).
  4. Quasi-liveness: no dead transitions (unreachable handlers, dead code in the model).
  5. **Workflow soundness** for request/saga flows: option to complete, proper completion (no leftover tokens such as orphaned replies), no dead transitions.
  6. Liveness / home state ("system can always return to idle") on the explored graph via SCC analysis.
- Defer to v2: LTL with fairness, CTL, timed properties, coloured data.

### R2. Build vs. reuse

Build in Rust (core, embeddable, permissive licences):

| Component | Basis |
|---|---|
| Net IR, PNML import/export, compile-from-statecharts | own code; PNML grammar from [pnml.org](https://www.pnml.org/) |
| Structural analysis: incidence matrix, P/T-invariants, siphons/traps, free-choice detection, Commoner check, structural boundedness LP | own code + [`good_lp`](https://crates.io/crates/good_lp) with HiGHS |
| State equation + trap constraints (ILP) for unreachability proofs, with CEGAR refinement | [Wimmel & Wolf](https://lmcs.episciences.org/1036), [Esparza & Melzer](https://archive.model.in.tum.de/um/bibdb/info/esparza.EM00.shtml.html); `good_lp` |
| Explicit-state explorer with stubborn sets for deadlock/reachability, bounded by a time and memory budget; SCC analysis for liveness/soundness | [Valmari](https://ai.dmi.unibas.ch/research/reading_group/valmari-apn1989.pdf); [`petgraph`](https://crates.io/crates/petgraph) |
| BMC + k-induction (later PDR) via SMT for larger nets | [`z3`](https://crates.io/crates/z3) or [`easy-smt`](https://crates.io/crates/easy-smt); [Amat et al.](https://hal.laas.fr/hal-03545594) as algorithmic reference |
| Independent **witness replayer** and **certificate checker** (small, auditable) | own code |

Reuse (optional, out-of-process, behind a "backend" trait, user-installed):

- **TAPAAL verifypn** (GPL-3) and **ITS-Tools** (GPL-3, Java) as "deep check" backends via PNML export. They are MCC leaders across GlobalProperties, Reachability, CTL and LTL ([MCC 2026](https://mcc.lip6.fr/2026/results.php)). Calling them as separate processes keeps our core's licence independent. **Have a lawyer confirm** this when shipping GPL binaries with the installer; the safer default is that the user installs them.
- **petrivet** (LGPL-3, pure Rust): watch it. It may become a dependency or a source of collaboration. Too immature to depend on today.
- **Apalache/Quint** export for data-heavy protocol models (v2+), with ITF traces read back via [`itf`](https://docs.rs/itf).
- Avoid UPPAAL (licence) and CPN Tools (discontinued) as dependencies.

### R3. Verdict semantics

| Verdict | Produced when | Evidence shown and stored |
|---|---|---|
| **Proved** | (a) structural certificate: P-invariant, trap, siphon-trap (Commoner) for free-choice, or ILP infeasibility of state equation + traps; (b) inductive invariant from k-induction/PDR; (c) exhaustive exploration of the full (reduced) state space finished; (d) external backend says "holds" | (a)/(b): certificate re-checked by the independent checker and rendered in domain terms. (c): technique + state count + reduction used. (d): tool name, version, command, flagged as "trusted external" |
| **Disproved** | a concrete firing sequence (or lasso) reaches the violation | Trace, **always replayed** by the independent simulator before display; animated per §6 |
| **Unknown** | budget exhausted; net unbounded and no certificate found; BMC found nothing up to k; property outside the decidable fragment for the model's extensions | Reason category, what was tried, bound reached ("no counterexample up to 40 steps"), and the suggested fix (add capacity, split model, run deep backend). Then hand off to the anti-pattern scan |

Rules: never report `Proved` from BMC alone; never report `Disproved` without a replayed trace. Run the cheap structural and ILP methods first (milliseconds), then exploration with stubborn sets, then BMC/k-induction, then external backends (portfolio order, as ITS-Tools does).

### R4. Staging

1. **Stage 1 (MVP):** Net IR + PNML; structural analysis; explicit exploration with budget for 1-safe/bounded nets; deadlock, safety predicates, k-bound, dead transitions, workflow soundness; replayer; token animation of traces; verdict model R3.
2. **Stage 2:** stubborn sets; ILP state equation with traps (Proved for unreachability without exploration); Commoner/free-choice fast paths; pattern library (mutex, bounded queue, pool, request/timeout, retry, saga) as reusable subnets with pre-proven invariants.
3. **Stage 3:** composition across C4 levels (§7): ports, flattening, then contract refinement checks; cached per-component certificates; incremental re-verification on edit.
4. **Stage 4:** SMT-based BMC/k-induction; optional TAPAAL/ITS-Tools backends; LTL with fairness; time (bounded TPN or timed-arc) if users need deadlines.
5. **Stage 5 (code):** see R5.

### R5. Hand-off to code-level verification (later, complementary tier)

The model proves **protocol-level** properties (no deadlock, sagas always finish, bounded queues never overflow) *under assumptions* about each component. Every transition and port of a component model is an **assumption about the code**: "handler X always eventually produces exactly one of {ok, err}", "lock L is released on all paths", "this queue is bounded at 100". The hand-off point is the **component interface contract**:

- **Model side (this tool):** proves system properties assuming the contracts.
- **Code side:** discharges each contract per Rust function or module:
  - Kani harnesses for bounded, sequential per-transition obligations (Kani does not handle concurrency).
  - Verus/Creusot/Prusti for unbounded functional contracts (see theorem-proving.md).
  - Loom (exhaustive within bounds) or Shuttle DFS for concurrency primitives that implement a net pattern.
  - Model counterexample traces replayed as Shuttle schedules or integration tests.
- **Conformance without proofs:** MIR-to-net extraction in the style of cargo-check-deadlock for sync code, with async as research. Runtime trace conformance (token replay of logs against the net, cf. [`process_mining`](https://crates.io/crates/process_mining) and P's PObserve) yields "conforms on observed runs", never "Proved".

The UI should show a **proof chain**: system property → proved on model → assumes contracts C1..Cn → each contract `Proved (Verus)`, `Unknown (Kani: no counterexample up to k)`, `Tested (Shuttle)` or `Unverified`. The system property is only as strong as its weakest link, and the tool should say so.

---

## Open questions

1. **Licensing of external backends.** LoLA's upstream licence, Tina's licence, CPN IDE's and Punf/MPSat's terms could not be confirmed. Can GPL-3 verifypn/ITS-Tools be bundled in the installer as separate executables, or must users install them? Needs legal review.
2. **Composition relation per property.** Which refinement relation (trace inclusion, failures, branching bisimulation, WF-net refinement) to use per property class, and how to explain a failed refinement check to users. The open-net/operating-guideline literature needs a proper read.
3. **Async Rust import.** No primary work maps `async`/tokio to Petri nets. Should conformance for async code rely on runtime traces plus Shuttle rather than static extraction?
4. **FIFO and data.** How often do real architecture questions need message order or payload data? If often, a coloured-net or TLA+ export path moves earlier.
5. **Time.** Are quantitative timeouts (deadline arithmetic) a v1 user need? If yes: time Petri nets (state classes, bounded only) versus timed-arc nets (TAPAAL) versus timed automata (licence issue).
6. **Fairness.** Liveness of retries and polling usually needs weak/strong fairness assumptions. Which fairness notion to expose without confusing users?
7. **Budgets.** What time/memory budget per check keeps the UI interactive, and should deep checks run in the background with cached certificates?
8. **MCC newcomers.** "tedd" (StateSpace winner) and "TY" (Ferrite Technology) could not be identified. If either is Rust or permissively licensed, it changes the build-vs-reuse balance.

## Not verified in this session (summary)

- LoLA 2 upstream licence (mirror says AGPL-3.0); Tina, CPN IDE and Punf/MPSat licences; Alloy licence terms.
- Timed-arc Petri net (un)decidability details (taken from secondary records of the TAPAAL authors' work).
- Property-preservation conditions of Suzuki–Murata refinement; soundness preservation under WF-net refinement; open nets / operating guidelines.
- Verus concurrency support details; Prusti current licence after the move to `prusti/prusti`.
- Esparza's 1998 complexity introduction ([PDF](https://archive.model.in.tum.de/um/bibdb/esparza/course.pdf)) could not be text-extracted; the equivalent results were taken from the Esparza–Nielsen survey ([arXiv 2411.01592](https://arxiv.org/abs/2411.01592)).
