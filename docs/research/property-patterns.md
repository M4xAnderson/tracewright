# Property patterns: what designers of concurrent and distributed systems need to state

> Terms and decisions made after this research (GLOSSARY.md, CLAUDE.md, docs/adr/) take precedence over this file. It is kept as the research record.

As of 2026-11-02. Question: which properties practitioners actually state about concurrent and distributed designs, which of them the v1 property language (ADR 0003) can express and how, and what the others would need.

Sources:
- Dwyer, Avrunin and Corbett's property specification patterns and their survey data.
- The Model Checking Contest (MCC) examinations.
- The TAPAAL (verifypn) and LoLA query languages.
- Properties sampled from published TLA+, P, Quint and Alloy specs, from industrial papers, the public TLA+ example corpus, the P tutorial, the Quint examples and the Alloy sample models.
- Our own E5 paper test.

Conventions:
- **[unverified]**: could not be confirmed from a primary source during this research.
- **[secondary]**: taken from a paper that cites the primary source.
- **[own derivation]**: argued here, not quoted from a source.
- Formulas use CTL notation:
  - `AG p`: p holds in every reachable marking.
  - `EF p`: some reachable marking satisfies p.
  - `AF p`: every run eventually reaches p.
  - `AG EF p`: from every reachable marking, a p-marking is reachable.
  - `GF p`: p holds infinitely often.
- Wording:
  - Dwyer et al. call the interval a pattern applies to its "scope". This file calls it a **pattern interval**, because *scope* is the element a property belongs to (GLOSSARY.md).
  - "Specification pattern" means Dwyer's notion. It never means the tool's pattern library.
  - "Liveness property" means a property of the form "something eventually happens". "L4-liveness of a transition" means the Petri net notion "the transition can always fire again".
  - Sources use "invariant". The tool's own wording says *property* (GLOSSARY.md lists invariant under Avoid). "P-invariant" stays the name of the certificate technique (ADR 0008).

---

## 0. Summary

**v1 covers the safety half of what practitioners write. For the most frequent pattern it offers only a stronger stand-in.**
- In Dwyer et al.'s survey of 555 property specifications, Response ("every P is eventually followed by S") was the most common pattern, followed by Universality and its dual Absence ([Dwyer et al., ICSE 1999](https://dl.acm.org/doi/10.1145/302405.302672)).
- Universality and Absence over the whole run map directly onto `never p`.
- Response and Existence are liveness properties and cannot be expressed in v1.
- The v1 stand-in is `always reachable` ("can always get back to …"). Under a fairness assumption on the edges of the reachability graph, it means "p infinitely often, or the run ends in p". That is *stronger* than "p eventually" (section 4).
- Strong fairness on transitions is **not** enough for this reading (counterexample in section 4).

**The v1 property kinds are the "fast classes" of the Petri net tools.**
- LoLA reduces queries where it can to deadlock checks, plain reachability (`EF φ`, `AG φ`) and `AGEF φ`, and calls reachability "the particular strength of LoLA" ([LoLA 2.0 manual](https://github.com/hlisdero/lola/blob/HEAD/lola/doc/lola.texi)).
- These are exactly our built-ins, `never p` and `always reachable p`.
- LoLA's atoms (linear terms, `FIREABLE`, `INITIAL`, `DEADLOCK`) are a near match for our predicate language ([LoLA formula grammar](https://github.com/hlisdero/lola/blob/HEAD/lola/doc/grammars/formula.ebnf)).

**Published models are mostly safety properties, with a substantial minority of liveness properties.**
- In the public TLA+ example corpus, 213 of 245 TLC configurations check invariants and 99 check temporal properties. 45 of those 99 (18 % of all configs) have liveness-sounding property names (method and commit in 2.2).
- The Azure Cosmos DB model has 14 safety and 10 liveness properties ([Hackett, Rowe, Kuppe 2022](https://arxiv.org/html/2210.13661)).
- MongoDB's reconfiguration paper checks safety only ([Schultz et al., OPODIS 2021](https://arxiv.org/html/2102.11960v3)).
- The P tutorial pairs each safety monitor with a liveness monitor ([P tutorial](https://p-org.github.io/P/tutorial/twophasecommit/)).

**The properties that matter most in industrial work need data, identity or order.**
- Examples: linearizability, read-your-writes, leader completeness, "at most one leader *per term*" and "no committed data lost" ([AWS 2014](https://lamport.azurewebsites.net/tla/formal-methods-amazon.pdf), [Cosmos DB](https://arxiv.org/html/2210.13661), [MongoDB](https://arxiv.org/html/2102.11960v3), [P Raft spec](https://github.com/p-org/P/blob/master/Tutorial/6_Raft/PSpec/SafetyOneLeader.p)).
- ADR 0002 already puts these outside v1.
- Some have exact counting versions: "never more than k uncommitted writes" and "never one participant aborted and another committed". These fit v1.
- Others have only a *stronger* counting variant that correct protocols violate. "At most one primary at all" fails on a correct protocol that keeps a stale primary in an older term (3.3).

**MCC categories are not evidence of need.** MCC formulas are generated randomly every year ([Amat et al., TOOLympics 2023](https://homes.cs.aau.dk/~srba/files/MCC:24.pdf)). They show what engines can answer, not what designers ask.

**Practitioners check that the good thing can happen at all.**
- Quint's `--witnesses` and the Bakery example's `no_critical_witness` invariant serve this purpose, and so does Alloy's `run SomeLeader … expect 1` (2.3).
- In v1, the cheapest guard against such vacuous Proveds is to run no dead transitions next to user properties and to show what was explored (F4).

**Four findings touch settled decisions** (section 5):
- (F1) Properties that name internal places, generated places or states conflict with ADR 0003's `element/port` rule. This is a proposed amendment, and slice 4's acceptance example depends on it.
- (F2) Negative weights are needed for comparisons between two sums.
- (F3) Final markings per property. This is a proposed amendment that moves finals from `Net` into `DeadlockFree`.
- (F4) "Can reach p" (`EF p`) would change the glossary meaning of Proved, so cheaper alternatives come first.

**Recommendation in brief:**
- v1 templates, all expanding into existing v1 kinds:
  1. "At most k at once", with the variants mutual exclusion and "never holds the whole pool".
  2. "Can always get back to / can always finish".
  3. "Can always finish cleanly" (k-case workflow soundness).
- Plus a vacuity guard (no dead transitions next to user properties, and exploration counts in the panel).
- After v1: "can reach p", "smallest safe capacity", an `enabled(t)` atom, "nothing leaks", observers, and eventually/leads-to under explicit fairness.
- Explicitly out:
  - free-form CTL/LTL;
  - time, probabilities, rates and metastability;
  - data, identity and order properties;
  - hyperproperties;
  - net-level sanity properties (StableMarking, OneSafe, L4-liveness of every transition) as user-facing properties.

---

## 1. Baseline: the v1 property language

From ADR 0003, ADR 0010, ADR 0014 and `docs/design/design.md` §3 and §6:

| v1 kind | Meaning | CTL | Disproved evidence (ADR 0010) | When an overflow marking is reachable (ADR 0014) |
|---|---|---|---|---|
| deadlock-free | no reachable marking without an enabled transition, unless it is a final marking | `AG (dead → final)` | counterexample | violation found before any overflow: Disproved; otherwise Unknown "capacity exceeded first" |
| no capacity exceeded | no reachable overflow marking | `AG ∧ₚ m(p) ≤ cap(p)` for `overflow` places | counterexample | the overflow marking *is* the Disproved |
| no dead transitions | every transition is enabled in some reachable marking (quasi-liveness) | `∧ₜ EF enabled(t)` | exhaustive | Proved impossible, so Unknown; whether a dead set found then counts as Disproved is **open** (see below) |
| never p | p holds in no reachable marking | `AG ¬p` | counterexample | as deadlock-free |
| always reachable p | from every reachable marking a p-marking is reachable | `AG EF p` | counterexample, then exhaustive | as deadlock-free |

`p` is a linear marking predicate: a weighted sum of places compared with a constant, combined with and, or and not (ADR 0003). Time, data, message order and temporal operators beyond these are excluded (ADR 0002, ADR 0003, `north-star.md` non-goals).

**Consequence for templates [own derivation].** Generated places overflow by default (ADR 0014). On a model that still has default generated places, many properties in section 6 end Unknown "capacity exceeded first" until the user picks `block` or `drop`, or proves no capacity exceeded with a larger capacity. The template UI should say so up front.

**Open semantic question for the slice 4 spec.** No dead transitions has no violating marking. ADR 0014 says "nothing happens after" an overflow marking, so the explored graph is the whole behavior, and a dead set found there is arguably decisive. But ADR 0014 also phrases Disproved as "a violation reached without passing through an overflow marking", which does not fit a set of transitions. Proposal: a transition enabled only *in* an overflow marking does not count as enabled, because overflow markings are not expanded. A dead set found while an overflow marking is reachable is Unknown "capacity exceeded first", because a fix of the capacity may change it. This needs a decision, not research.

What follows mechanically, without an ADR change [own derivation]:
- `=`, `≠`, `<`, `≤`, `>`, `≥` against a constant reduce to each other over integers (`x ≠ c` is `x < c or x > c`).
- `enabled(t)` for an unbound transition is a conjunction of linear constraints (firing rule in ADR 0010 and ADR 0014):
  - each pre place needs `m(p) ≥ w(p,t)`;
  - each `block` post place needs `m(p) − w(p,t) + w(t,p) ≤ cap(p)`.

  A user can therefore write it by hand. For fused transitions this gets harder (F5).
- A comparison between two sums (`a ≤ b`) is `a − b ≤ 0`. It needs negative weights (F2).
- If `always reachable F` is Proved, every reachable dead marking satisfies F, because a dead marking can reach only itself. So deadlock-free with final F adds nothing to `always reachable F`, and the templates do not pair them.

Expressibility levels used in the tables:

| Level | Meaning |
|---|---|
| **Yes** | Stated directly as one v1 kind. |
| **Template** | Expands into one or more v1 properties. No ADR change, only UI sugar (but see F1 for which places it may name). |
| **Model edit** | Expressible only if the modeler adds places to the behavior model, for example a history place with `drop`. |
| **Stronger only** | v1 can state a counting variant that is strictly stronger than the published property. Correct designs may violate it, so it is not offered as that property. |
| **ADR change** | Needs a new property kind or atom, cheap within the current engine. |
| **No** | Needs temporal logic beyond reachability, fairness, time, probabilities or data. A stand-in may be named. |
| **N/A** | Does not apply to P/T nets with typed places, or belongs to the scan rather than to checks. |

---

## 2. Sources and what they say about need

### 2.1 Dwyer, Avrunin and Corbett: the specification pattern system and its survey

**Pattern system.**
- The patterns are qualitative and either state-based or event-based.
- Occurrence patterns: Absence, Universality, Existence, Bounded Existence. Order patterns: Precedence, Response, and their chain variants.
- Each pattern comes in five pattern intervals: Global, Before R, After Q, Between Q and R, After Q until R.
- Patterns and intervals combine into 55 pattern types ([Dwyer et al., FMSP 1998](https://dl.acm.org/doi/pdf/10.1145/298595.298598); [ICSE 1999](https://dl.acm.org/doi/10.1145/302405.302672)).
- Bounded Existence means P occurs at most k times *along a run*. It does not mean "at most k at once".
- I took the pattern and interval list from search-surfaced text of the ICSE paper ([copy](https://www.cs.colostate.edu/~france/CS614/Readings/Readings2011/PropPatterns2p411-dwyer.pdf)). The full PDF is a scan I could not extract.

**Survey.**
- "In all, we collected 555 specifications from at least 35 sources"; "511 (92%) matched one of our patterns".
- The most common pattern is Response, the next most common Universality and its dual Absence (same copy, quoted from search-surfaced OCR text).

**Counts.**
- Table 1 lists Response at 245 (≈44 % of 555) and Universality at 119 (≈21 %). These come from OCR text with garbled layout **[unverified against the original table]**.
- Per-pattern counts for Absence, Existence and Precedence, and the counts for non-global intervals: **[unverified]**.
- The global interval covers 438 of 511 matched instances (85.7 %) **[secondary]**, per [How Understandable Are Pattern-based Behavioral Constraints for Novice Software Designers?, TOSEM 2019](https://dl.acm.org/doi/fullHtml/10.1145/3306608).

**Threats to validity for us.**
- The corpus dates from 1999 and comes from finite-state verification users (protocols, hardware, embedded control), not from architecture designers.
- A case study on industrial service-based applications reports that "some needs of the industry … are not fully met" by the research pattern catalogs (Bianculli, Ghezzi, Pautasso, Senti, ICSE 2012 SEIP; [abstract](https://design.inf.usi.ch/node/39)) **[abstract only, body not read]**.

**Later catalogs add time and probability.**
- Real-time patterns: [Konrad & Cheng, ICSE 2005](https://dl.acm.org/doi/10.1145/1062455.1062526).
- Probabilistic patterns: [Grunske, ICSE 2008](https://www.researchgate.net/publication/221554456_Specification_patterns_for_probabilistic_quality_properties).
- A unified catalog of "all known plus 40 newly identified or extended patterns": [Autili et al., TSE 2015](https://doi.org/10.1109/TSE.2015.2398877).
- All of them sit outside v1 (no time, no rates).

### 2.2 TLA+ in industry and in the public example corpus

**AWS (2014)** ([Newcombe et al., *Use of Formal Methods at Amazon Web Services*, 2014](https://lamport.azurewebsites.net/tla/formal-methods-amazon.pdf); published as CACM 58(4), 2015):
- AWS teams state "Safety properties: what the system is allowed to do" and "Liveness properties: what the system must eventually do".
- The example for the second is "whenever the system receives a request, it must eventually respond to that request".
- On the lock-free data structure, AWS "failed to find a liveness bug as we did not check liveness".
- A DynamoDB bug needed a 35-step error trace.
- The paper also describes sustained slowdowns, where "the system will eventually make progress; it is not stuck in a logical deadlock, livelock", as something they do not "yet know of a feasible way to model".
- This is the same boundary that E5 found for metastability.

**AWS (2025).** AWS now also uses P, a language of communicating state machines, for example in moving S3 to strong read-after-write consistency ([Brooker & Desai, ACM Queue 2025](https://queue.acm.org/detail.cfm?id=3712057)) **[read from search abstracts only; the article returned 403]**.

**Azure Cosmos DB** ([Hackett, Rowe, Kuppe, arXiv 2210.13661](https://arxiv.org/html/2210.13661); venue beyond arXiv **[unverified]**):
- Consistency levels are mostly modeled as read operators over a log. Strong consistency is checked by refinement against a linearizability specification.
- One stated property is "Bounded staleness consistency should never accept more than StalenessBound uncommitted writes at once". That is a counting bound.
- Another is "All writes eventually complete, either with success or failure", a liveness property.
- The model has 14 safety and 10 liveness properties, checked with TLC.

**MongoDB** ([Schultz, Zhou, Dardik, Tripakis, OPODIS 2021](https://arxiv.org/html/2102.11960v3)):
- The logless reconfiguration protocol checks Election Safety ("it is not the case that both s and t are primary and have the same term") and Leader Completeness with TLC on finite instances.
- The paper says: "We do not address liveness properties in this work".
- Election Safety is per term. Two primaries in *different* terms are allowed, and correct protocols reach such states while a stale primary has not yet stepped down. The P Raft tutorial states the same property per term (2.3).

**tlaplus/Examples corpus** ([repo](https://github.com/tlaplus/Examples)).

Method:
- Shallow clone at commit `0e32a039a4286cc3e1b0f70c00eaff3178d49f85` (committed 2026-10-30), all 245 `.cfg` files.
- Comments `(* … *)` and `\* …` are stripped. Tokens after `INVARIANT`/`INVARIANTS` or `PROPERTY`/`PROPERTIES` count until the next TLC keyword.
- Liveness names match the case-insensitive regex `live|termin|eventual|progress|starv|respon|converg|leadsto|fair|success|stabil|finish`.
- Refinement names match `refine|spec|implement|!`.
- The two regexes overlap: a name can match both.

| Measure | Count |
|---|---|
| configs with an invariant | 213 / 245 (87 %) |
| configs with a temporal property | 99 / 245 (40 %) |
| … of which a property name matches the liveness regex | 45 / 99 (18 % of all configs) |
| … of which a property name matches the refinement regex | 24 / 99 |
| configs that set `CHECK_DEADLOCK FALSE` | 58 / 245 (24 %) |
| most frequent invariant names | `TypeOK` 112, `TypeInvariant` 34, `Inv` 27, `Safety` 10, `SafetyInvariant` 10 |

An independent rerun by a reviewer with its own regexes gave 213, 99, 58, 112, 34, 43 (liveness) and 24 (refinement). The liveness count depends on the regex.

Caveats:
- Name-based classification is a heuristic.
- The corpus is teaching- and research-heavy, not industrial.

Interpretation:
- Type invariants dominate. In a P/T net, places hold token counts only, and capacities cover ranges. So a type invariant has no counterpart beyond "no capacity exceeded" **[own derivation]**.
- TLC checks for deadlock by default, and `CHECK_DEADLOCK FALSE` or `-deadlock` turns that off ([TLC config](https://learning.tlapl.us/intro/tlc-config/), [TLC docs](https://docs.tlapl.us/using:tlc:start)).
- A quarter of the configs turn it off. My reading **[own derivation, not checked per spec]**: either the spec terminates on purpose, or it is an open spec whose terminal states are intended (for example a fragment without an environment). Both cases support final markings (ADR 0003).

**Typical safety property shapes.**
- In the two-phase-commit example, `TCConsistent` is the state predicate "two RMs have not arrived at conflicting decisions": no RM is aborted while another is committed ([TCommit.tla](https://github.com/tlaplus/Examples/blob/master/specifications/transaction_commit/TCommit.tla)).
- With the RMs folded into counts, this becomes the linear predicate `aborted ≥ 1 and committed ≥ 1`, which must never hold.
- The Consensus example states liveness as `Success == <>(chosen # {})` and needs `WF_chosen(Next)`, a weak fairness assumption, to prove it ([Consensus.tla](https://github.com/tlaplus/Examples/blob/master/specifications/Paxos/Consensus.tla)).

### 2.3 P, Quint and Alloy: mechanisms and sampled properties

**P.**
- Specifications are "observer machines (imagine runtime monitors)". They observe events, assert safety, and express liveness with `hot` states.
- An execution that never leaves a hot state is "flagged as a potential liveness violation" ([P manual, monitors](https://p-org.github.io/P/manual/monitors/)).
- Monitors cannot send or create machines, so they cannot change system behavior.
- This is the observer approach that section 3.1 needs for Precedence and for patterns over a pattern interval.

**Quint.**
- `quint verify` takes `--invariant`, `--inductive-invariant` (Apalache only) and `--temporal`.
- `quint run` takes `--witnesses`, which reports "for how many traces the witness is true" ([Quint CLI](https://quint.sh/docs/quint)).

**Alloy 6.**
- Adds mutable `var` state and LTL with past operators.
- Analysis is bounded by scope and by `steps` (default `for 10 steps`). Unbounded checking with `1..` steps needs NuSMV or nuXmv ([Alloy 6](https://alloytools.org/alloy6.html)).
- Classic Alloy is used "to check properties of the model by generating counterexamples" ([about](https://alloytools.org/about.html)), largely for structural constraints. In this tool, structural constraints belong to the scan (rules), not to checks.

**Sample of published properties.** The sample covers the P tutorial, the distributed examples in the Quint repository, and the Alloy sample models. It is small and teaching-oriented. It shows shapes, not frequencies. Each item is classified in 3.3.

| Spec | Property (quoted or paraphrased from the source) | Kind |
|---|---|---|
| P tutorial, 2PC: `AtomicityInvariant` ([Atomicity.p](https://github.com/p-org/P/blob/master/Tutorial/2_TwoPhaseCommit/PSpec/Atomicity.p)) | "if a transaction is committed by the coordinator then it was agreed on by all participants" | safety, per transaction |
| P tutorial, 2PC: `Progress` (same file) | "Every received transaction from a client must be eventually responded back". The monitor keeps an integer `pendingTransactions` and is `hot` while it is non-zero. | liveness over an anonymous count |
| P tutorial, client/server: `BankBalanceIsAlwaysCorrect`, `GuaranteedWithDrawProgress` ([BankBalanceCorrect.p](https://github.com/p-org/P/blob/master/Tutorial/1_ClientServer/PSpec/BankBalanceCorrect.p)) | balances stay correct and never below 10; "all withdraw requests submitted by the client are eventually responded" | safety over data; liveness |
| P tutorial, failure detector: `ReliableFailureDetector` ([ReliableFailureDetector.p](https://github.com/p-org/P/blob/master/Tutorial/4_FailureDetector/PSpec/ReliableFailureDetector.p)) | "all nodes that have been shutdown … will eventually be detected" | liveness, per node |
| P tutorial, Raft: `SafetyOneLeader`, `SafetyLogMatching`, `SafetyLeaderCompleteness`, `LivenessProgress`, … ([SafetyOneLeader.p](https://github.com/p-org/P/blob/master/Tutorial/6_Raft/PSpec/SafetyOneLeader.p); [PSpec folder](https://github.com/p-org/P/tree/master/Tutorial/6_Raft/PSpec)) | "At term {0} there are multiple leaders": at most one leader **per term** | safety over data and identity |
| Quint, ReadersWriters ([ReadersWriters.qnt](https://github.com/informalsystems/quint/blob/main/examples/classic/distributed/ReadersWriters/ReadersWriters.qnt)) | `Safety`: not (readers ≠ ∅ and writers ≠ ∅), and size(writers) ≤ 1. `Liveness`: every actor is always eventually a reader and a writer, under weak fairness | safety; liveness per actor |
| Quint, Bakery ([bakery.qnt](https://github.com/informalsystems/quint/blob/main/examples/classic/distributed/Bakery/bakery.qnt)) | mutual exclusion (`correctness`), proved with an inductive invariant. Also `no_critical_witness`, run "to see that even with this parameters a node can actually enter the critical section" | safety; reachability sanity check |
| Quint, LamportMutex ([LamportMutex.qnt](https://github.com/informalsystems/quint/blob/main/examples/classic/distributed/LamportMutex/LamportMutex.qnt)) | `Mutex`: at most one process in the critical section | safety |
| Quint, two-phase commit ([two_phase_commit.qnt](https://github.com/informalsystems/quint/blob/main/examples/classic/distributed/TwoPhaseCommit/two_phase_commit.qnt)) | `consistencyInv`: no RM Committed while another is Aborted | safety |
| Quint, DiningPhilosophers5 ([DiningPhilosophers5.qnt](https://github.com/informalsystems/quint/blob/main/examples/classic/distributed/DiningPhilosophers/DiningPhilosophers5.qnt)) | `Safety`: no two adjacent philosophers eat at once. `DeadlockFreedom = weakFair(step) implies always(eventually(not(DeadlockCondition)))`. `LivelockFreedom`: under weak fairness, nobody stays hungry forever | safety; deadlock; liveness per philosopher |
| Quint, ewd840 ([ewd840.qnt](https://github.com/informalsystems/quint/blob/main/examples/classic/distributed/ewd840/ewd840.qnt)) | safety: termination is detected only when all nodes are inactive. Liveness: "Once all nodes are inactive, termination will eventually be detected", under `weakFair`. Two further properties are documented as holding or failing depending on *which* action is weakly fair | safety; liveness sensitive to the fairness choice |
| Alloy book, ring election ([ringElection1.als](https://github.com/AlloyTools/org.alloytools.alloy/blob/master/org.alloytools.alloy.extra/extra/models/book/chapter6/ringElection1.als)) | `AtMostOneElected`: "This should not find any counterexample". `AtLeastOneElected` "generates a counterexample in which nothing happens" | safety; liveness refuted by the lack of a progress assumption |
| Alloy sample, ring leader ([ringlead.als](https://github.com/AlloyTools/org.alloytools.alloy/blob/master/org.alloytools.alloy.extra/extra/models/examples/algorithms/ringlead.als)) | `OneLeader` (at most one leader at each tick), `LeaderHighest`, `Liveness` (with no lost messages, a leader is found), and `run SomeLeader … expect 1` | safety; conditional liveness; reachability sanity check |
| Alloy sample, Dijkstra ([dijkstra.als](https://github.com/AlloyTools/org.alloytools.alloy/blob/master/org.alloytools.alloy.extra/extra/models/examples/algorithms/dijkstra.als)) | `DijkstraPreventsDeadlocks`: ordered mutex acquisition implies no deadlock | deadlock freedom |
| Alloy, Chord ([Zave, TSE 2017, arXiv 1610.01140](https://arxiv.org/abs/1610.01140)) | ring maintenance specified with an inductive invariant. Correctness is an eventual property of the ring, proved in Alloy "for networks of bounded size" | safety (inductive) plus liveness over identities **[abstract only]** |

Observations from the sample:
- (a) Every sample pairs safety with liveness, and liveness is almost always "every request/job/node eventually gets an answer or detection". P's `Progress` monitor keeps exactly the anonymous counter that section 4 uses.
- (b) Liveness claims always come with an explicit fairness or progress assumption: Quint's `weakFair`, Alloy's `NoLostMessages`, P's "in the absence of node failures". ewd840 shows that the *choice* of fairness changes the verdict.
- (c) Mutual exclusion and "no conflicting decisions" recur and are linear over folded state places.
- (d) Practitioners routinely check that the good state is reachable at all (Bakery's witness, Alloy's `run … expect 1`, Quint `--witnesses`).
- (e) Leader and term properties are per identity.

### 2.4 Petri net tools: MCC, TAPAAL, LoLA

**MCC 2026 examinations** ([rules](https://mcc.lip6.fr/2026/pdf/rules.pdf), E-1.1 to E-1.7):
- Six categories: StateSpace, GlobalProperties, UpperBounds, Reachability, CTL and LTL formulas.
- GlobalProperties covers ReachabilityDeadlock, QuasiLiveness, StableMarking, Liveness and OneSafe.
- UpperBounds asks for "its exact upper bound" of designated places, 16 formulas per model.
- Reachability, CTL and LTL each come as Cardinality (comparisons of token counts) and Fireability (fireability of transitions), 16 formulas each.

**How the MCC defines its properties** ([Amat et al., *Behind the Scene of the Model Checking Contest*, TOOLympics 2023, LNCS](https://homes.cs.aau.dk/~srba/files/MCC:24.pdf); [venue](https://homepages.laas.fr/namat/publication/toolympics)):
- The global properties are defined as the existence of a counterexample: "a place whose marking can exceed 1", "a deadlocked state", "a place whose marking never varies", "a transition that can never be fired", "a transition that is not live".
- Before 2020 only deadlocks were computed.
- Reachability formulas are `AG p` or `EF p`.

**MCC formulas are random.** "Formulas are randomly generated and changed for each edition", by the tool Citili since 2020 (same source). So they are benchmark material, not a sample of user questions.

**TAPAAL / verifypn query language** ([grammar](https://github.com/TAPAAL/verifypn/blob/main/src/PetriEngine/PQL/PQLQueryParser.y), [XML reader](https://github.com/TAPAAL/verifypn/blob/main/src/PetriParse/QueryXMLParser.cpp), [options](https://github.com/TAPAAL/verifypn/blob/main/src/PetriEngine/options.cpp)):
- Quantifiers: `EF`, `EG`, `AF`, `AG`, `EX`, `AX`, and `A`/`E` with `X`, `F`, `G`, `U`.
- Atoms: `true`, `false`, `deadlock`, and comparisons of integer expressions built from token counts, constants, `+`, `-` and `*`. The `*` is between terms, so it allows non-linear products.
- Fireability: `t?` / `fireable(t1, …)`. A list means a disjunction.
- XML elements: `tokens-count`, `is-fireable`, `integer-le`, `integer-sum`, `integer-product`, `integer-difference`, `deadlock`, `place-bound`.
- Options: `-k, --k-bound` (a token bound for unbounded nets), `--trace`, and `--trace-replay`, which verifies a trace "against the provided model and query".
- TAPAAL also answers HyperLTL queries ([features](https://tapaal.net/features/)).

**LoLA 2.0** (manual dated April 2016, read from the unofficial mirror; upstream licence open per `verification.md`).

Grammar ([formula.ebnf](https://github.com/hlisdero/lola/blob/HEAD/lola/doc/grammars/formula.ebnf)):
- Terms are linear (`NUMBER * term`, `+`, `-`, `oo`), compared with `= != > >= < <=`.
- Atoms are `FIREABLE(t)`, `INITIAL` and `DEADLOCK`.
- Operators are CTL*, with the shorthands `REACHABLE`, `INVARIANT` and `IMPOSSIBLE`.

The manual's list of "implicitly supported properties" ([lola.texi](https://github.com/hlisdero/lola/blob/HEAD/lola/doc/lola.texi)):
- Reachability `EF P`, invariance `AG P`, deadlocks, and k-boundedness of a place `AG p <= k`.
- Dead transition `AG NOT FIREABLE(t)`.
- Quasi-liveness, checked "for each transition individually", because "State spaces that preserve single dead transitions are orders of magnitude smaller than state spaces that preserve quasi-liveness".
- Liveness of a transition `AGEF FIREABLE(t)` and liveness of a state predicate `AGEF P`.
- Reversibility `AGEF INITIAL`.
- Causal precedence. "Transition t causally precedes a state predicate P if every path from the initial marking to a marking satisfying P contains an occurrence of t". The manual's recipe is to add "a new place q with a single initial token … a pre-place of t" and check reachability of `P AND (q = 1)`. This is exact for that single question, because only prefixes before t's first occurrence matter. But it disables t after one firing, so it changes the net's other behavior (see F6).
- Relaxed soundness and soundness of a workflow net.
- "Home states … cannot be answered with this release".

LoLA 1.x had named modes: `DEADTRANSITION`, `LIVEPROP`, `REVERSIBILITY`, `EVENTUALLYPROP` (`F φ`), `FAIRPROP` (`GF φ`) and `STABLEPROP` (`FG φ`). They show which property *shapes* users asked a Petri net tool for (same manual, compatibility table).

**Workflow soundness.**
- Van der Aalst et al. classify soundness notions for workflow nets, including k-soundness (k cases in the initial place), and discuss their decidability ([Formal Aspects of Computing 2011](https://link.springer.com/article/10.1007/s00165-010-0161-4)).
- LoLA's manual defines (classical) soundness as: "the final marking is reachable from all reachable markings, and the net has no dead transitions".

### 2.5 Our own evidence (E5)

`docs/product/experiments/e5-paper-test.md` reproduced three AWS incidents. It used:
- `deadlock_free` with a final predicate;
- `no_capacity_exceeded`;
- `never` with a share-of-pool sum;
- `always_reachable`.

Lessons:
- **Linear predicates over internal places beat deadlock-freedom.**
  - The share-of-pool predicate `control_plane/in_a + wait_a + ret_a ≥ 3` still caught case 1 when a timeout made deadlock-free and always reachable come out Proved.
  - In `case1_ebs_bad.toml`, those places are *not* ports. Only transitions are marked `port = true`.
- **Always reachable must not be read as "will recover"** (E5 implication 5).
- **Final predicates differ per property.** Case 1 needed `final = "api_b/done = 2"` on deadlock-free, while AZ-A may stay stuck (E5 implication 2).

---

## 3. Pattern catalog and expressibility

### 3.1 Dwyer specification patterns (global pattern interval unless stated)

| Pattern | Formula | Evidence of need | v1 | How in v1 | What it needs otherwise |
|---|---|---|---|---|---|
| Absence (state) | `AG ¬P` | among the three most common Dwyer patterns ([ICSE 1999](https://dl.acm.org/doi/10.1145/302405.302672)) | **Yes** | `never P` | — |
| Absence (event: t never fires) | `AG ¬enabled(t)` | same; LoLA "dead transition" | **Template** (unbound t); **ADR change** for fused t | `never enabled(t)` with the enabling condition written out (§1) | `enabled(t)` atom (F5); in a combined scope also F1 |
| Universality | `AG P` | second most common, 119 of 555 **[OCR]**; also Cosmos "never … more than StalenessBound uncommitted writes at once" ([Hackett et al.](https://arxiv.org/html/2210.13661)) and slice 4's "never more than 2 orders in flight" (Universality over a counting predicate) | **Yes** | `never not P`; for counts, `never Σ > k` | — |
| Existence | `AF P` | P liveness monitors, AWS "must eventually" | **No** | Stand-in: `always reachable P`. It is a sufficient but **strictly stronger** stand-in (§4). If P holds once and is then unreachable (for example "eventually initialized"), the stand-in is Disproved although "P eventually happens" holds, and that Disproved does not answer the user's question. | AF with an explicit fairness notion and lasso counterexamples (new evidence kind) |
| Bounded Existence (P occurs at most k times along a run) | counter over history | Dwyer pattern | **Model edit** | the modeler adds a counter place that the observed transition increments, with capacity k+1 and `when_full = drop`, then `never count ≥ k+1` [own derivation] | tool-generated observer place per property (F6) |
| Precedence (S before P) | `A[¬P W S]` | Dwyer pattern; LoLA "causal precedence" | **Model edit** | a history place `s_done` with capacity 1 and `when_full = drop`; S also puts a token into it; then `never (P and s_done = 0)`. It never blocks S [own derivation]. LoLA's one-token pre place is background only: it disables S after one firing (F6). | observer place generated per property (F6) |
| Response (P leads to S) | `AG (P → AF S)` | **most common** Dwyer pattern, 245 of 555 **[OCR]**; AWS's liveness example ([2014](https://lamport.azurewebsites.net/tla/formal-methods-amazon.pdf)); Cosmos "all writes eventually complete"; P `Progress`, `GuaranteedWithDrawProgress` (2.3) | **No** (stand-in: **Template**) | For anonymous requests in a place `pending`: `always reachable pending = 0` gives Response under edge-fairness (§4). **Precondition:** every transition that removes tokens from `pending` means "answered", and `pending` is not `drop`. Otherwise a design that sheds or times out every request is Proved. | LTL/CTL with fairness and lasso counterexamples; or export to TLA+/Quint/P |
| Chain precedence / chain response | sequences of events | Dwyer chain variants | **No** | — | observer automata (F6), later |
| Patterns over the pattern intervals Before R, After Q, Between, After-until | the above, restricted to an interval | ≈14 % of matched instances are not global **[secondary]** | **No** (safety cases: **Model edit**) | with phase places in the model, a safety pattern over an interval becomes `never (phase = 1 and P)` | observer per interval (F6) |

### 3.2 Standard Petri net properties (MCC, TAPAAL, LoLA, workflow nets)

| Property | Source | v1 | How in v1 | Notes |
|---|---|---|---|---|
| Deadlock | MCC ReachabilityDeadlock; LoLA `EF DEADLOCK`; verifypn `deadlock` atom; TLC default | **Yes** | built-in deadlock-free | Ours excludes final markings. The MCC's does not, and 24 % of the TLA+ example configs turn deadlock checking off (2.2). |
| Deadlock inside a predicate (`EF (deadlock and P)`) | verifypn, LoLA | **Yes** (as finals) | deadlock-free with final predicate `¬P` = `AG (dead → ¬P)` | Needs a final predicate per property when one scope asks several deadlock questions (F3). |
| Quasi-liveness (no dead transitions) | MCC QuasiLiveness; LoLA per transition | **Yes** | built-in | LoLA checks each transition separately to save memory ([lola.texi](https://github.com/hlisdero/lola/blob/HEAD/lola/doc/lola.texi), "Quasi-liveness"). Our verdict lists the dead set. Overflow interaction is open (§1). |
| L4-liveness of a transition (t can always fire again) | LoLA `AGEF FIREABLE(t)`; MCC Liveness | **Template** (unbound t); **ADR change** for fused t | `always reachable enabled(t)`, enabling condition written out | Detects a *partial* deadlock (one element stuck forever while others proceed), which deadlock-free misses. Readable only with F5. |
| L4-liveness of every transition | MCC Liveness | **Template** (one per transition) | as above, for all t | Do not offer as a built-in. One-shot transitions (startup, terminal steps) make real designs fail it [own derivation]. |
| Recurrence of a predicate (`AGEF P`) | LoLA | **Yes** | `always reachable P` | — |
| Reversibility | LoLA `AGEF INITIAL` | **Template** | `always reachable` (each chosen place = its initial count) | "can always get back to idle" (E5 wording) |
| Home state existence | LoLA: unsupported | **No** | — | Not a design question; skip. |
| k-boundedness of a place | LoLA `AG p <= k`; verifypn `place-bound` | **Yes** | `never p > k`, or capacity k with `overflow` + no capacity exceeded | verifypn's `--k-bound` is for unbounded nets. In our tool every place has a capacity (glossary). |
| OneSafe | MCC | **Yes** | capacity 1 with `overflow`, or `never p ≥ 2` | A net property, not a design question. |
| StableMarking (some place never changes) | MCC | **Template** (per place) | `never p ≠ m0(p)` | A modeling sanity check, not a design question. |
| UpperBounds (exact maximum count of a set of places) | MCC UpperBounds | **Template** (two checks) | maximum k: `never Σ ≥ k` Disproved (k is reached) and `never Σ > k` Proved | Basis of "smallest safe capacity" (§6.2, L2), with the limits stated there. |
| Reachability Cardinality (`EF`/`AG` over token comparisons) | MCC; verifypn; LoLA | `AG`: **Yes**; `EF`: **ADR change** (or the F4 alternatives) | `AG p` = `never not p`; `EF p` today only as a Disproved `never p` | Comparisons between two sums need negative weights (F2). |
| Reachability Fireability | MCC; verifypn `t?`; LoLA `FIREABLE` | **Template** (unbound t); **ADR change** for fused t | expand the enabling condition | atom recommended (F5) |
| Products of token counts | verifypn `*`, `integer-product` | **No** | — | Breaks the linear/ILP certificate path that ADR 0003 relies on. LoLA's grammar stays linear. |
| CTL / LTL formulas in general | MCC, TAPAAL, LoLA | **No** (except `AG`, `AG EF`) | — | MCC formulas are random (2.4), so there is no evidence that designers need arbitrary nesting. |
| HyperLTL | TAPAAL ([features](https://tapaal.net/features/)) | **No** | — | Information-flow questions; out of scope. |
| Workflow soundness, k cases: option to complete | van der Aalst et al.; LoLA | **Template** | `always reachable F`, where F is the full final condition (§6.1, T3) | — |
| Workflow soundness, k cases: proper completion | same | **Template** | `never (done = N and not F)`, with N cases from the initial marking | `done ≥ 1` is the single-case form. With N folded cases it gives false Disproveds while other cases still run. Resource places count against their initial counts, not against 0. Catches orphaned replies (`verification.md` §5.1). |
| Workflow soundness: no dead transitions | same | **Yes** | built-in | — |

### 3.3 Properties from published distributed-system models

| Property as published | Source | v1 | How in v1, or why not |
|---|---|---|---|
| Type invariant (`TypeOK`) | most frequent invariant in the TLA+ corpus (2.2); Quint examples | **N/A** | Places hold counts. Capacities and "no capacity exceeded" cover ranges [own derivation]. |
| Mutual exclusion / at most one holder (`Mutex`, `MutualExclusion`, `correctness`) | TLA+ corpus; Quint Bakery, LamportMutex; Alloy `AtMostOneElected`, `OneLeader` | **Yes** | `never Σ critical ≥ 2` |
| Readers/writers exclusion | Quint ReadersWriters | **Yes** | `never (readers ≥ 1 and writers ≥ 1)` and `never writers ≥ 2` |
| No two adjacent philosophers eat | Quint DiningPhilosophers5 | **Yes** (philosophers as separate state places, not folded) | `never eating_i + eating_{i+1} ≥ 2` for each pair i |
| Election safety: at most one primary/leader **per term** | MongoDB ([OPODIS 2021](https://arxiv.org/html/2102.11960v3)); P Raft `SafetyOneLeader` | **No** (counting variant: **Stronger only**) | `never primaries ≥ 2` is strictly stronger. Correct protocols reach a state with a stale primary in an older term, so it would be Disproved on a correct design. Per term needs data (colored nets, ADR 0002). |
| Two-phase commit consistency (no RM aborted while another committed) | [TCommit.tla](https://github.com/tlaplus/Examples/blob/master/specifications/transaction_commit/TCommit.tla); Quint `consistencyInv` | **Yes** | folded RMs: `never (aborted ≥ 1 and committed ≥ 1)` |
| Atomicity: committed only if all participants agreed | P `AtomicityInvariant` | **Yes** for one transaction at a time; **No** per transaction id | single transaction: `never (committed ≥ 1 and agreed < N)`; several concurrent transactions need identity |
| Bounded staleness / bounded in-flight work | Cosmos ([Hackett et al.](https://arxiv.org/html/2210.13661)); slice 4 example | **Yes** | `never Σ uncommitted > k` |
| Share of a resource (bulkhead) | E5 case 1 | **Yes** (needs F1 for internal places) | `never Σ held_by_X ≥ c` |
| Resource conservation / no leak (free + used = N) | `verification.md` §5.1 (mutex and pool P-invariants) | **Yes** | `never (free + Σ used ≠ N)`; later provable by a P-invariant certificate (ADR 0008) |
| Termination detected only when all are inactive | Quint ewd840 | **Yes** (with 1-safe state places) | `never (detected ≥ 1 and active ≥ 1)` |
| No load shed (a `drop` place never discards) | ADR 0014 semantics | **Template** for unbound t, written by hand; readable only with F5 (§6.2, L3) | `never (enabled(t) and m(p) > cap(p) − w(t,p) + w(p,t))` for each t with `drop` post place p [own derivation]. In a combined scope t's pre places are internal (F1). |
| Every request eventually answered / all writes eventually complete | AWS 2014; Cosmos; P `Progress`, `GuaranteedWithDrawProgress` | **No** (stand-in: **Template**) | `always reachable pending = 0` with the precondition of §3.1. It says "can always clear", not "will". |
| Termination (`Termination`, `EventuallyTerminates`) | TLA+ corpus | **No** (stand-in: **Template**) | `always reachable F` = "p infinitely often or ends in F" under edge-fairness (§4) |
| Deadlock freedom under fairness (`DeadlockFreedom` as `WF ⇒ GF ¬deadlock`) | Quint DiningPhilosophers5; Alloy `DijkstraPreventsDeadlocks` | **Yes** | built-in deadlock-free (stronger: no reachable deadlock at all) |
| Starvation / livelock freedom per process | TLA+ corpus; Quint DiningPhilosophers5, ReadersWriters `Liveness` | **No** | per-process identity and fairness |
| Eventual failure detection per node | P `ReliableFailureDetector` | **No** | identity and liveness |
| A value is eventually chosen (`<>(chosen # {})` under WF) | [Consensus.tla](https://github.com/tlaplus/Examples/blob/master/specifications/Paxos/Consensus.tla); Alloy `AtLeastOneElected`, `Liveness` | **No** (stand-in: `always reachable chosen ≥ 1`) | needs fairness. Alloy's own comment shows the failure without it: a counterexample "in which nothing happens". |
| Linearizability, read-your-writes, monotonic reads, consistent prefix | Cosmos | **No** | values, order and identity of operations (ADR 0002 consequences) |
| Leader completeness, log matching, "committed data never lost" | MongoDB; AWS 2014; P Raft | **No** | data and identity |
| Refinement of an abstract spec | TLA+ corpus (24 of 99 property configs); Cosmos | **No** | ADR 0002 excludes refinement in v1 |
| Sustained slowdown / metastability ("not stuck … but effectively unavailable") | AWS 2014; E5 case 3 | **No** | rates and time; north-star non-goal |
| Chord ring correctness (eventual, over identifiers) | Zave TSE 2017 | **No** | identity, data and liveness |
| Structural assertions (no cycles, every X has a Y) | classic Alloy | **N/A** | belong to the scan (ARCH-CYCLE etc., `antipatterns.md`) |

### 3.4 Atoms and predicate forms

| Form | TAPAAL | LoLA | MCC | v1 |
|---|---|---|---|---|
| weighted sum of places vs constant | yes | yes (linear) | sums vs constant | **yes** (ADR 0003) |
| sum vs sum | yes | yes (via `-`) | yes (`integer-le` of two counts) | **needs negative weights (F2)** |
| products of counts | yes | no | no | **no** |
| `enabled(t)` / fireability | `t?` | `FIREABLE(t)` | Fireability categories | **template for unbound t now; atom later (F5)** |
| `deadlock` as an atom | yes | `DEADLOCK` | Deadlock examination | **through finals (F3)** |
| `initial` | no | `INITIAL` | no | **template (reversibility)** |

---

## 4. "Always reachable" as the stand-in for liveness

**Why transition fairness is not enough [own derivation].**
- Take a token that moves between places L and R through transitions `l2r` and `r2l`, and a flag place toggled by `set` and `unset`. Let p = `L = 1 and flag = 1`.
- The reachability graph is strongly connected, so `AG EF p` holds.
- Yet the cycle (L,0) → `l2r` → (R,0) → `set` → (R,1) → `unset` → (R,0) → `r2l` → (L,0) fires every transition infinitely often and never reaches p.
- So strong fairness on *transitions* does not turn `always reachable` into "eventually".

**Claim [own derivation; standard bottom-SCC argument].**
- Assume a finite reachability graph. Call a run *edge-fair* if, from every marking it visits infinitely often, it takes every outgoing edge of that marking infinitely often.
- Then `always reachable p` (`AG EF p`) holds iff every maximal edge-fair run either ends in a dead marking that satisfies p, or visits p-markings infinitely often.
- (⇒) The markings an infinite edge-fair run visits infinitely often form a bottom strongly connected component. `AG EF p` puts a p-marking in it, and edge-fairness visits all of it.
- (⇐) Suppose a reachable marking has no p-marking reachable. Then any edge-fair continuation from it never meets p.

**What this means for Existence.**
- The property is recurrence (`GF p`, or "ends in p"). It implies `AF p`, but `AF p` does not imply it.
- So `always reachable` is a sufficient, strictly stronger stand-in for Existence and for Response, never an equivalent.

**Probability 1 [own derivation].** The same argument gives "p infinitely often or ends in p with probability 1" when the scheduler is a finite Markov chain, that is, a fixed positive probability for each (marking, enabled transition) pair. A history-dependent scheduler whose probabilities decay towards zero breaks this. The notion of "fairly correct" systems, and its coincidence with probabilistic and topological largeness for finite-state systems, is due to [Varacca & Völzer, LICS 2006](https://lics.siglog.org/archive/2006/VaraccaVolzer-Temporallogicsandmo.html).

**Response on anonymous tokens [own derivation].**
- Suppose requests are tokens in a place `pending`, and every transition that removes a token from `pending` means "answered".
- Then "`pending = 0` infinitely often or at the end" implies that every request that entered was answered, because no token can stay in an empty place.
- So `always reachable pending = 0` is Response under edge-fairness. No deadlock-free pairing is needed, because a reachable dead marking with `pending > 0` already disproves it (§1).
- The precondition is not something the tool can infer:
  - a `drop` on `pending` discards requests;
  - a timeout or discard transition empties `pending` without answering.
  - Either makes the property Proved for a design that sheds every request. That would be a wrong Proved in the user's mind.
- The template therefore asks for the answer transitions. It refuses, structurally, if any other transition consumes from `pending` or if `pending` is `drop`.
- This is E5's "slow = never responds" abstraction (E5 recommendation (a)). A timeout that answers with an error is an answer only if the user says so, and E5 case 1 shows that it can hide the defect.
- P's `Progress` monitor uses the same anonymous counter (2.3).

**Why the wording matters.**
- Edge-fairness over the *global* reachability graph is far stronger than what schedulers, networks or overloaded services provide. It assumes that a recovery step that is possible in a recurring situation is eventually taken.
- E5 case 3 is exactly the case where recovery is possible but not taken under load. AWS describes sustained slowdowns that are not deadlocks or livelocks and that it cannot model ([2014](https://lamport.azurewebsites.net/tla/formal-methods-amazon.pdf)).
- So the template reads "can always get back to …" or "can always finish", never "will" (E5 implication 5, `north-star.md` non-goals).
- The verdict panel leads with the literal claim and then the limit: "From every reachable state, idle can be reached. This does not mean it will be reached, especially under load." A second line may name the assumption under which it would mean "eventually": "only if, in every situation that keeps recurring, every step possible there is eventually taken".

**What full liveness would need.**
- An explicit fairness notion per element or transition (weak or strong) that the user chooses. ewd840 in the Quint examples shows that the choice changes the verdict (2.3). Without fairness, any independent loop (for example `network/blip` in E5 case 3) refutes every `AF` [own derivation].
- A lasso counterexample (prefix plus cycle) as a new evidence kind, with its own replay rule in `recheck`.
- SCC analysis on the explored graph, which `verification.md` R1 item 6 already plans.
- Real tools need fairness too: the TLA+ Consensus liveness needs `WF_chosen(Next)` (2.2), Quint's examples state `weakFair`, P assumes no node failures, and LoLA 1.x had separate `FAIRPROP` and `STABLEPROP` modes (2.4).

---

## 5. Findings that touch settled decisions

### F1. Properties over internal places, generated places and states. Proposed amendment of ADR 0003.

**The conflict.**
- ADR 0003 says a property "refers to places through qualified `element/port` IDs". ADR 0010 says that in a combined scope "every port of every child must be bound … otherwise lowering fails with 'model incomplete'".
- Read together: in a combined scope a property can name only ports, and a place exposed as a port just for observation makes the model incomplete.
- ADR 0002 rejected references into another element's net because renames would break other files and would "rule out later contract abstraction".

**Who needs more than ports.**
- E5 case 1's decisive predicate (`control_plane/in_a + wait_a + ret_a ≥ 3`) names places that are not ports: `case1_ebs_bad.toml` marks only transitions as ports.
- Slice 4's acceptance example `never Billing/inflight + Shipping/inflight > 2` sums places of two children. It is not in the E5 files, but it needs the same answer.
- The north-star question "can the queue between Order API and Billing exceed 50?" names a **generated place**, whose qualified ID is the relationship (ADR 0010 Semantics), not `element/port`.
- "The breaker is open" names a **state machine state**, which slice 2 compiles to a place.

**Options.**
- **(a) Name any place.** Properties may name any place of the scope's own behavior model, any place of a child (`element/place`), any generated place (`relationship`) and any state machine state (`element/state`).
  - Uses the existing qualified IDs of ADR 0010.
  - Costs:
    - The parent depends on the child's internals, which brings back both costs ADR 0002 avoided for bindings. A parent property that names `child/internal_place` stops working when the child's net is replaced by a contract.
    - A renamed internal place shows up as a dangling reference ("model incomplete", Unknown).
    - Silent retargeting: if a place is renamed and another place later takes the old name, the property quietly refers to the wrong place, and a Proved about it is wrong in the user's mind. Mitigation: the rename command updates property references in the same gesture (ADR 0001 journal covers multi-file gestures).
- **(b) Observable place port.** A place port that is exempt from ADR 0010's must-be-bound rule.
  - Keeps encapsulation and contract abstraction.
  - Costs one more port kind and an explicit marking step for the modeler.
  - Generated places would still need a rule, because they belong to the relationship and are visible in the parent's view anyway.
- **(c) Keep the property in the child's own scope** [own derivation].
  - In the own scope, ports are closed: a transition port fires alone (ADR 0010).
  - For a child whose ports are all transitions, every combined run projects onto a run of the child alone. A fused transition adds the partner's pre and post places, and an async link adds a generated place. Both can only restrict when the child's transition fires, and an overflow marking ends the run.
  - So a `never p` over the child's places that is Proved in its own scope also holds in every combined scope.
  - Limits:
    - It is not true for children with place ports, because partners add or remove tokens there.
    - It does not preserve `always reachable`.
    - It cannot express sums across two children (the slice 4 example).
    - A Disproved in the own scope is correct for that scope but may not occur when combined.
  - It covers E5 case 1's share-of-pool predicate without breaking encapsulation: in `control_plane`'s own scope, `accept_a` fires three times, which is the missing bulkhead.

**Recommendation.** (a) for the scope's own places, generated places in the scope's view and state machine states, with rename-updates-references. Then (b) or (a) for children's internal places, as the user's decision. Offer (c) as a hint in the UI regardless. Slice 4 cannot be specified until this is decided. **Escalated** (open question 1).

### F2. Negative integer weights (ADR 0003 clarification)

- "A weighted sum of places compared with a constant" does not say whether weights may be negative.
- Comparisons between two sums need `a − b ≤ 0`. They appear in MCC Cardinality formulas, verifypn and LoLA (3.4), and in "never more in flight than workers".
- Recommendation: state integer weights, negative allowed. The linear and ILP certificate path is unaffected, because ILP constraints are signed anyway [own derivation].

### F3. Final markings per property. Proposed amendment of ADR 0003 and a design.md §3 type change.

**The problem.**
- ADR 0003 makes final markings model data, and `Net { …, finals: Option<FinalPredicate> }` puts one predicate on the scope's net.
- E5 put `final = …` on each deadlock-free property, and two deadlock questions in one scope need different finals. Examples: "AZ-B always finishes" while AZ-A may stay stuck (E5 case 1), and "no deadlock while orders are pending".

**Structural consequence today.**
- `finals` is part of the flattened net, so it is part of the canonical encoding.
- Changing the final predicate therefore changes the CheckKey of *every* property in the scope, including `never p` and no capacity exceeded, whose meaning does not depend on finals.
- Those verdicts become stale spuriously. That goes against the intent of ADR 0013's invariance tests ("staleness is never spurious").

**Recommendation.**
- Move finals into the property: `NetProperty::DeadlockFree(Option<FinalPredicate>)`, and remove `finals` from `Net`.
- The model may keep a scope-level default final predicate that lowering copies into each deadlock-free property unless the property overrides it.
- ADR 0010's deadlock rule reads the final predicate from the property.
- The CheckKey still covers it through the canonical property (ADR 0013).
- After §1's observation, the templates in section 6 no longer depend on F3. Only deadlock-free with several final predicates in one scope does.
- **Escalated** (open question 2).

### F4. Guarding against vacuous Proveds, and a possible "can reach p" kind

**The risk.**
- A Proved `never p`, deadlock-free or no capacity exceeded on a model that barely moves is vacuous.
- Real cases: an initial marking that is already dead; a branch whose guard can never be met, so the scenario the property was about never happens; a port bound so that the fused transition can never fire.
- E5 case 1 with a timeout is *not* such a case. There the model moves freely, and the Proveds come from abstracting time away.
- Practitioners guard against this explicitly: Quint `--witnesses`, Bakery's `no_critical_witness`, Alloy `run … expect 1` (2.3).

**Three candidates, cheapest first:**

| Option | What | Changes to semantics |
|---|---|---|
| (i) Vacuity guard | Run no dead transitions next to the user's properties in each scope. The Proved panel shows the number of reachable markings and lists every transition enabled in no reachable marking. | None. It reuses an existing built-in and existing statistics. |
| (ii) "Expected: reachable" on `never p` | A display flag on `never p`. A Disproved is shown as the expected outcome ("reachable, as expected: replay"), and a Proved is shown as a warning. | None. The verdict is still Disproved or Proved of `never p`. A counterexample may not pass through an overflow marking (ADR 0010 replay rule), and a Proved with an overflow marking reachable is Unknown (ADR 0014), exactly as for `never p`. |
| (iii) New kind "can reach p" (`EF p`) | Proved by a replayed sequence of steps to a p-marking; Disproved by exhaustive agreement | Large: see below |

**What (iii) changes.**
- GLOSSARY.md **Proved** ("holds for every possible behavior") does not fit an existential claim. A user who sees "Proved: can reach done" may read "always reaches done", which is a wrong Proved in the user's mind.
- **Disproved** ("has a counterexample") does not fit an exhaustive refutation.
- **Evidence** would gain a third kind, a replayed positive sequence. **Counterexample** ("ends in a violation"), **Replay** ("stepping through a counterexample") and the Relationships line "Only a Disproved verdict has a counterexample" all need changes.
- ADR 0008 allows only a certificate or a second explorer's agreement for Proved, so it needs an amendment, and so does ADR 0010's evidence-by-kind list.
- ADR 0014 interaction: the positive sequence must not pass through an overflow marking. An exhaustive "not reachable" while an overflow marking is reachable is Unknown "capacity exceeded first".
- The positive sequence needs its own glossary term. *Witness*, *trace* and *scenario* are all under Avoid, so the proposal is **Example** (see glossary changes).

**Recommendation.** (i) in slice 4, (ii) as a candidate for slice 4 if the user wants a "can reach" question, (iii) only after v1 and only if (ii) proves confusing. **Escalated** (open question 3).

### F5. An `enabled(t)` atom

Writing enabling conditions by hand is error-prone:
- `block` post places add capacity terms.
- In a combined scope, a bound port transition never fires alone (ADR 0010).
- `Enabled(element/transition)` names no transition of the composed net, because fused transitions have IDs `relationship/caller+callee`.

Definition: `Enabled(element/t)` holds in a marking iff some transition of the composed net whose constituents include `element/t` is enabled. That is the unbound transition itself, or each fused transition that contains it.

| Option | Evaluated by | Effect |
|---|---|---|
| (a) desugar in `engine` into linear constraints | shared lowering code | covered by golden tests only |
| (b) `Enabled(QualifiedId)` as an atom of the predicate in `NetProperty` | `verify` and `recheck`, each resolving the constituents from its own composition with its own firing rule (ADR 0009, ADR 0010) | cross-checked |

Recommendation: (b), as an amendment of ADR 0003:
- `net` stores the atom as data only, with no evaluation (ADR 0009 and design.md §3: no `enabled` in `net`).
- The canonical encoding uses the qualified ID, so the CheckKey covers it (ADR 0013).
- The atom is a disjunction of linear constraints, so ADR 0003's decidability argument holds. For certificates (ADR 0008), a disjunction needs one ILP per disjunct [own derivation].

In a combined scope, the "Template" rows that use `enabled(t)` also need F1, because the pre places of a port transition are internal.

### F6. Observer places for history (after v1)

- Precedence, event counts and patterns over pattern intervals need a monitor added to the checked net per property. Precedents: LoLA's causal-precedence place and P's spec machines (2.3, 2.4).
- To keep ADR 0010's guarantee, the observer must be part of the `OpenNets` that `recheck` composes, not something added to the flattened net.
- **An observer must never change behavior** [own derivation]:
  - It may not be a pre place of the observed transition. LoLA's one-token pre place disables t after one firing. Under ADR 0014's global overflow rule, that can turn an Unknown "capacity exceeded first" into a Proved for another property.
  - Its `when_full` is `drop`, never `overflow`. An `overflow` observer would make every other property Unknown "capacity exceeded first", and would turn the user's no capacity exceeded into a Disproved on a phantom place.
- For "at most k times", use capacity k+1 with `drop`. The count saturates at k+1, the observed transition is never blocked, and the property is `never count ≥ k+1`.
- No capacity exceeded must ignore observer places, and observer places must be excluded from no dead transitions and from final predicates.
- Until F6 exists, the Model edit rows of 3.1 tell the modeler to use the same encoding (capacity, `drop`, a post arc only), because ADR 0014 requires every place in a behavior model to choose its `when_full`.

---

## 6. Recommendation

**How the ranking was made.**
1. Evidence from this tool's own setting (E5, three AWS incidents modeled by the target user) weighs most.
2. Then published distributed-system models: AWS, Cosmos, MongoDB, and the P, Quint and Alloy samples.
3. Then Dwyer's 1999 frequencies, which come from finite-state verification users, not from solo backend designers (`north-star.md` target user).

Within that, a template ranks higher if it:
- is exact rather than a stand-in, because a stand-in risks a wrong Proved in the user's mind;
- serves a north-star job (J1, J4);
- is cheap to build.

### 6.1 Templates for v1 (slice 4), ranked

**What every template must do.**
- Each template expands into existing v1 kinds. Each expanded property is checked separately and keeps its own verdict (GLOSSARY.md: one check per property, one verdict per check).
- The template group in the panel shows a **summary** derived from current verdicts:
  - Disproved if any part is Disproved;
  - otherwise Unknown if any part is Unknown, stale or not yet checked;
  - Proved only if every part is Proved and current.
- The summary is a display rule, not a fourth verdict and not a check. It is never cached.
- Storage is a file-format decision (ADR 0001), **escalated** in open question 4. Recommended: store the expanded properties, each with a `template` tag, a group ID and the template's parameters. The checker, the cache and old files stay unchanged. Editing a parameter rewrites all parts in one gesture.

**What the templates depend on.**
- F1 for any template that names internal places, generated places or states, and F2 for comparisons between two sums.
- None depends on F3, because no template pairs deadlock-free with `always reachable` (§1).
- On models with default generated places (`overflow`), expect Unknown "capacity exceeded first" until capacities and `when_full` are chosen (§1).

| Rank | Template (user wording) | Expands to | Why this rank |
|---|---|---|---|
| T1 | **At most k at once** ("never more than 2 orders in flight"). Variants: **Mutual exclusion** (k = 1: "at most one holder of the lock"); **Never holds the whole pool** (`Σ held_by_X` against the pool's initial count c: "AZ-A never holds all c threads", E5's share of a resource) | `never Σ w·m(p) > k`; mutual exclusion `never Σ critical ≥ 2`; whole pool `never Σ held_by_X ≥ c` | E5 case 1: the only property still decisive under a timeout that fooled deadlock-free and always reachable. Slice 4 acceptance example; Cosmos bounded staleness; mutual exclusion recurs in TLA+, Quint and Alloy (2.3, 3.3); Universality is among the most common Dwyer patterns; J1 and J4. Exact meaning, no fairness caveat. |
| T2 | **Can always get back to idle / can always finish / can always clear pending** | idle: `always reachable` (each chosen place = its initial count). Finish: `always reachable F`. Clear: `always reachable pending = 0`, with the answer transitions named and the structural precondition of §4 enforced | The only v1 stand-in for Response (most common Dwyer pattern; P `Progress`; AWS "must eventually respond") and for termination. LoLA reversibility; E5 P2/P3. Carries the §4 wording and links E5's "slow = never responds" lesson. Ranked below T1 because it is a stronger stand-in, not the property itself. |
| T3 | **Can always finish cleanly** (k-case workflow soundness) | Precondition: N cases in the initial marking and no transition creates cases (shown as an assumption). The user marks the place `done`, the per-case places and the resource places. F := `done = N` and every per-case place (including generated places of the flow) = 0 and every resource place = its initial count. Expands to `always reachable F` + `never (done = N and not F)` + no dead transitions | van der Aalst k-soundness and LoLA soundness; sagas and request flows (`verification.md` R1 item 5); catches orphaned replies and leaked resources. Third because it needs the most user input and E5 did not need it. |

**Also in slice 4, not a template:** the vacuity guard of F4 (i). It runs no dead transitions next to user properties and adds the exploration counts and never-enabled transitions to the Proved panel.

The built-ins stay as they are: deadlock-free (with finals, F3), no capacity exceeded, no dead transitions.

**Keep-or-cut rule for dogfooding** (journal, `north-star.md`):
- Count how many journal questions were stated through each template versus a raw predicate.
- Keep a template that carried at least one journal question and produced no verdict the user misread.
- Rename or cut one that was misread.
- If templates do not shorten I1 (time to first verdict) compared with raw predicates, do not add more before v1 ends.

### 6.2 After v1, ranked

| Rank | Addition | Needs | Value |
|---|---|---|---|
| L1 | **Can reach p** | F4 (ii) first, (iii) only if needed | Practitioners' sanity check (Quint witnesses, Alloy `run`, Bakery witness); MCC/LoLA `EF`. Low cost for (ii). A candidate to pull into slice 4. |
| L2 | **Smallest safe capacity** ("how small can this queue be?") | UI orchestration; no new verdict | J4 ("see whether a smaller value still never overflows"); MCC UpperBounds. For an `overflow` place, behavior below the capacity does not depend on it, so a binary search over k with `never Σ > k` on one net finds the largest count that occurs. That is the smallest capacity with identical behavior, but only if it lies below the current capacity; otherwise the check reports the overflow. For `block` and `drop` places the count never exceeds the capacity by construction, and a smaller capacity changes behavior (more blocking, more shedding). There J4 means re-checking the user's properties (deadlock-free, no load shed, T1–T3) at each candidate capacity: different nets, different CheckKeys. The number is shown only if every check behind it was decisive. If any is Unknown, the result is Unknown, never a number. Uses "capacity", not "bound" (GLOSSARY.md). |
| L3 | **`enabled(t)` atom** → "t never fires", "never sheds load", "t can always fire again" | F5 (and F1 in combined scopes) | Fireability categories of MCC, TAPAAL and LoLA; partial-deadlock detection through L4-liveness per transition. |
| L4 | **Nothing leaks** (conservation) | none beyond F1; later a P-invariant certificate (ADR 0008) | Pool and lock P-invariants (`verification.md` §5.1). Pulled forward only if the journal shows leak questions; E5 did not need it. |
| L5 | **Observer places**: "S always before P", "at most k retries in total", patterns over pattern intervals | F6 | Dwyer Precedence, Bounded Existence and pattern intervals; LoLA causal precedence; P monitors. |
| L6 | **Eventually / leads-to under explicit fairness**, with lasso counterexamples | fairness design, new evidence kind and replay rule, SCC engine | Response and Existence proper; Cosmos, AWS and P liveness. Otherwise export to TLA+, Quint or P (`verification.md` R2). |

### 6.3 Explicitly not supported (and what to tell the user)

- **Free-form CTL/LTL entry and nested temporal operators.** MCC formulas are random, so there is no evidence that designers need arbitrary nesting (2.4). Pattern systems exist because raw temporal logic is hard to write ([Dwyer et al.](https://dl.acm.org/doi/10.1145/302405.302672)). Templates replace it.
- **Time, deadlines, bounded response "within d", rates, overload, metastability, probabilities.** Real-time and probabilistic pattern catalogs exist (2.1). AWS could not model sustained slowdowns (2.2). See E5 and the `north-star.md` non-goals. Point to timed or stochastic tools.
- **Data, identity and order.** This covers linearizability, session guarantees, leader completeness, log matching, durability of specific values, election safety per term, FIFO, request/reply matching, and starvation freedom per process. See the ADR 0002 consequences; use TLA+, Quint or P. Do not offer stronger counting variants (such as "at most one primary at all") under these names (3.3).
- **Hyperproperties** (HyperLTL, information flow).
- **Refinement and conformance between levels.** ADR 0002; later contract work.
- **Net-level sanity properties as user-facing properties:** StableMarking, OneSafe, and L4-liveness of *every* transition. These are MCC benchmark properties, not design questions, and whole-net liveness is noisy for one-shot transitions. They may surface as scan-like hints later.
- **Type invariants.** Places hold counts; capacities cover ranges.
- **Non-linear arithmetic** (products of token counts). Breaks the certificate path (ADR 0003).
- **Structural assertions about the architecture graph as properties.** They belong to the scan (rules), not to checks.

---

## 7. Proposed glossary changes

- **Property template** (new, slice 4). Proposed definition: "A named, parameterized way to state one or more properties, for example 'At most k at once'. Each property it expands into is checked separately and has its own verdict; the template only shows a summary of them." _Avoid_: pattern, query, formula, macro.
- **Observer place** (new, after v1, F6). Proposed definition: "A place the tool adds to a checked net to record history for one property, for example how often a transition fired. It never blocks a transition and discards tokens when full." _Avoid_: monitor, ghost place, history variable, spec machine.
- **Example** (new, only if F4 (iii) is chosen). Proposed definition: "A sequence of steps from the start that ends in a marking a 'can reach' property asks for." _Avoid_: witness, trace, scenario, run. Option (iii) also needs these changes:
  - **Proved** gets a sentence for existential properties.
  - **Disproved** allows exhaustive evidence.
  - **Evidence** adds "a replayed example".
  - **Replay** covers examples.
  - The Relationships line "Only a Disproved verdict has a counterexample" gains "a Proved 'can reach' verdict has an example".
- No change to **Property** (Avoid already lists *invariant*). In this file and in specs, "liveness property" and "L4-liveness of a transition" are kept apart, and "pattern interval" replaces Dwyer's "scope". Neither needs a glossary entry while it appears only in research.

## 8. Open questions

1. **F1:** may properties name a child's internal places (`element/place`), generated places (`relationship`) and states? Or only an "observable" place port? Or should such properties stay in the child's own scope? Recommended: own places, generated places and states yes, with rename-updates-references. For children's internal places this is the user's decision. Slice 4's acceptance example depends on the answer.
2. **F3:** may a property override the scope's final predicate, with finals moved from `Net` into `DeadlockFree`? Recommended: yes, with a scope-level default.
3. **F4:** which guard against vacuous Proveds ships in slice 4: (i) the no-dead-transitions guard with exploration counts, (ii) "expected: reachable" on `never p`, or (iii) a "can reach" kind? Recommended: (i) now, (ii) if a "can reach" question is wanted, (iii) after v1.
4. **Template storage:** store templates as tagged expanded properties, or as their own entity in the project files? Recommended: tagged expanded properties. This is a file-format decision (ADR 0001), and an ADR should record it.
5. **No dead transitions with a reachable overflow marking** (§1): is a dead set Disproved or Unknown "capacity exceeded first"? Recommended: Unknown. This belongs in the slice 4 spec or an ADR 0014 note.
6. **F5:** `enabled(t)` as an atom evaluated by both explorers, or as engine sugar? Recommended: atom.
7. The exact Dwyer Table 1 counts (Absence, Existence, Precedence, the pattern intervals) need a readable copy of the ICSE 1999 paper (ACM DL access).

## 9. Unverified items and gaps

- Dwyer et al.: the per-pattern counts beyond Response (245) and Universality (119), which come from OCR text. The distribution over pattern intervals (438/511 global) is secondary.
- Content of Brooker & Desai 2025 beyond search abstracts. The Cosmos DB paper's venue beyond arXiv. Bianculli et al. 2012 beyond the abstract. Zave 2017 beyond the abstract.
- The LoLA manual was read from the unofficial GitHub mirror (version 2.0, April 2016), not from the upstream download.
- The TLA+ corpus classification into liveness and refinement is by property name only. The reason why configs disable deadlock checking was not checked per spec.
- The P, Quint and Alloy sample (2.3) is small and teaching-oriented: the P tutorial, the Quint distributed examples, and the Alloy book and sample models. It shows property shapes, not frequencies. Published industrial P specs (for example from the S3 work) were not available to read.
- The fairness counterexample, the edge-fairness equivalence, the probability-1 condition, the Response-on-anonymous-tokens argument, the projection argument of F1 (c) and the observer encoding are my derivations, not quoted results.
