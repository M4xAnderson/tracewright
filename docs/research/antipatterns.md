# Anti-pattern scanning for architecture models (and later Rust code)

> Terms and decisions made after this research (GLOSSARY.md, CLAUDE.md, docs/adr/) take precedence over this file. It is kept as the research record.

**As of 2026-11-02.** Research for the Petri-net-backed C4 modeling tool (Rust core, Tauri 2, React UI).
Scope: the *fallback* path. When a behavioral property cannot be proven, the tool scans the architecture graph for known anti-patterns. Formal verification of Petri nets is covered by a separate document and is out of scope here.

Legend: **[V]** = verified against the linked primary source during this research. **[S]** = taken from a secondary source or a search summary; check before relying on it. **[I]** = my own inference or design proposal, not a sourced fact.

---

## 0. Summary

- Literature gives a **stable core of graph-detectable smells**: cyclic dependency, hub-like dependency, unstable dependency, god component/megaservice (Arcan/Designite lineage), and microservice smells such as shared persistence, hardcoded endpoints, no API gateway, ESB misuse and wobbly interactions (Taibi et al., Neri/Soldani et al.). Most of them need only **typed nodes plus typed, attributed edges**.
- The closest prior art to this project is **μTOSCA / μFreshener** (Univ. of Pisa). It detects microservice smells on a *design-time topology graph* whose interaction edges carry exactly three booleans: `circuit_breaker`, `dynamic_discovery` and `timeout` [V]. Copy that idea, but make the attributes **tri-state** (yes / no / unknown) so that "unknown" maps to "not checkable" instead of "smell".
- Cloud frameworks (AWS, Azure, GCP) are prose. The **machine-checkable** parts already exist as resource-level rules: AWS Config conformance packs mapped to WA Reliability [V], Azure APRL with Resource Graph queries [V], GCP regional/zonal guidance [V]. On a diagram you can check the *topological* subset: single-AZ, no replica, no backup, public exposure, no autoscaling, no LB in front of replicas.
- Licensing: AWS docs are **CC BY-SA 4.0** [V], Google developer docs are **CC BY 4.0** [V], and Microsoft Learn defaults to **personal, non-commercial, no modification** unless a page carries an explicit license [V]. Write your own short rule texts and link to the sources. Do not copy them.
- Rule engine for a Rust core: **petgraph** computes graph facts (SCC, dominators, articulation points, page rank) [V]. **ascent** (Datalog in Rust, with stratified negation, aggregates and lattices; MIT, 0.8.1) [V] or plain Rust rule functions express the rules. Report results in a **SARIF-shaped** schema, which already distinguishes `fail` / `pass` / `open` (insufficient information) / `notApplicable` and supports suppressions with justification [V].

---

## 1. Catalogs: anti-patterns, definitions, graph detection rules

### 1.1 Source catalogs

| Catalog | What it contributes | Link |
|---|---|---|
| Garcia, Popescu, Edwards, Medvidovic, *Toward a Catalogue of Architectural Bad Smells* (QoSA 2009, LNCS) | Defines an "architectural bad smell" and four smells: Connector Envy, Scattered (Parasitic) Functionality, Ambiguous Interfaces, Extraneous Adjacent Connector [S] | [Springer](https://doi.org/10.1007/978-3-642-02351-4_10), [Garcia slides](https://gsaw.org/wp-content/uploads/2013/07/2011s12b_garcia.pdf) |
| Arcelli Fontana et al., Arcan (ICSME 2016, ICSA 2017) | Cyclic Dependency, Hub-Like Dependency, Unstable Dependency; later God Component and more. Graph-database based [S] | [Arcan wiki (UniMiB)](https://essere.disco.unimib.it/wiki/arcan/), [Arcan 2017 paper PDF](https://boa.unimib.it/bitstream/10281/155470/1/PID4705339.pdf) |
| Sharma, Designite / DPy | God Component, Feature Concentration, Scattered Functionality, Dense Structure, Unstable Dependency, Cyclic Dependency [S] | [Sharma blog](https://tusharma.medium.com/analyze-python-code-quality-beyond-syntactic-issues-273d83391341) |
| Taibi & Lenarduzzi, *On the Definition of Microservice Bad Smells* (IEEE Software 35(3), 2018) | 11 microservice smells from 72 practitioner interviews [S] | [IEEE CS summary](https://publications.computer.org/software-magazine/2018/08/03/definition-microservice-bad-smells-poor-design/) |
| Taibi, Lenarduzzi, Pahl, *Microservices Anti-Patterns: A Taxonomy* (Springer 2019) | 20 anti-patterns with perceived harmfulness (0–10). Top: Hardcoded Endpoints 8, Wrong Cuts 8, Cyclic Dependency 7, then API Versioning, Shared Persistence, ESB Usage, Megaservice, Inappropriate Service Intimacy (~6), Lack of Monitoring, No API-Gateway, Shared Libraries (5) [V, from PDF Table 1.1] | [arXiv 1908.04101](https://arxiv.org/abs/1908.04101) |
| Neri, Soldani, Zimmermann, Brogi, *Design principles, architectural smells and refactorings for microservices: a multivocal review* (SICS 2019) | 7 smells organised by principle: Multiple Services in One Container, Endpoint-based Service Interactions, No API Gateway, Wobbly Service Interactions, ESB Misuse, Shared Persistence, Single-layer Teams, plus 16 refactorings [V] | [arXiv 1906.01553](https://arxiv.org/abs/1906.01553) |
| μTOSCA / μFreshener (Pisa) | Topology model plus smell detector and refactoring UI (MIT) [V] | [microTOSCA.yml](https://raw.githubusercontent.com/di-unipi-socc/microTOSCA/master/1.3/microTOSCA.yml), [microFreshener](https://github.com/di-unipi-socc/microFreshener) |
| Bogner et al., *Collaborative repository for service-based antipatterns* (ICSA-C 2019) | Structured (JSON) antipattern documentation [S] | [Uni Stuttgart record](https://puma.ub.uni-stuttgart.de/bibtex/2dde619cf598256f27c3673e6564aba12/wagnerst) |
| Cerny et al., *Catalog and detection techniques of microservice anti-patterns and bad smells: a tertiary study* (JSS 206, 2023) | 58 consolidated anti-patterns in 5 categories, from 203; notes inconsistent terminology across studies [S] | [Oulu repo](https://oulurepo.oulu.fi/handle/10024/44035), [NSF PAR full text](https://par.nsf.gov/servlets/purl/10493702) |
| Azure Architecture Center, performance antipatterns | Busy Database, Busy Front End, Chatty I/O, Extraneous Fetching, Improper Instantiation, Monolithic Persistence, No Caching, Noisy Neighbor, Retry Storm, Synchronous I/O [V] | [learn.microsoft.com antipatterns](https://learn.microsoft.com/en-us/azure/architecture/antipatterns/) |
| Azure Circuit Breaker pattern | Timeouts, retries and circuit breakers, and when *not* to use them (for example message-driven systems with DLQs) [V] | [Circuit Breaker](https://learn.microsoft.com/en-us/azure/architecture/patterns/circuit-breaker) |
| Bakhtin, Esposito, Lenarduzzi, Taibi, *Hub-like microservice detection* (arXiv 2025) | Hub detectors largely **disagree**. Arcan-style thresholds are weak; normalized degree centrality or ER-encoding works better [V] | [arXiv 2506.07683](https://arxiv.org/abs/2506.07683) |

### 1.2 Rules: definition → graph detection → model data → false-positive risk

Notation [I]: `G = (V, E)`. `kind(v)` ∈ {person, system, container, component, code, db, cache, queue, topic, lb, gateway, esb, …}. Edge attributes: `sync` (request/response vs async), `hard` (hard vs soft dependency, per AWS wording), `timeout`, `retry`, `cb` (circuit breaker), `discovery`, `owner`. `Gs` = the subgraph of sync edges between runtime elements.

| # | Anti-pattern (source) | Definition | Detection rule on the model graph | Model data needed | FP risk |
|---|---|---|---|---|---|
| A1 | **Cyclic dependency** (Arcan; Taibi: Cyclic Dependency) | Two or more components depend on each other directly or indirectly ([Arcan wiki](https://essere.disco.unimib.it/wiki/arcan/)) [S] | SCCs of size >1 (or self-loops) in the dependency graph at one C4 level; report each SCC plus one witness cycle. `petgraph::algo::tarjan_scc` [V] | Dependency edges per level | Low for build-time deps; medium for runtime (callbacks or events through a broker are not true cycles, so exclude async edges via queues) |
| A2 | **Cyclic sync service calls** [I, specialization of A1] | Services call each other synchronously in a loop, which risks deadlock and retry amplification | SCC in `Gs` restricted to runtime nodes | `sync` on edges | Low if `sync` is accurate. If a Petri net exists, hand off to the prover (possible deadlock) |
| A3 | **Hub-like dependency** (Arcan, Designite) | An element with many incoming *and* outgoing dependencies ([Arcan wiki](https://essere.disco.unimib.it/wiki/arcan/)) [S] | `fan_in(v) > q(fan_in) ∧ fan_out(v) > q(fan_out)`. Arcan-style uses a high quantile (a 0.75 quantile is reported by [Bakhtin et al.](https://arxiv.org/abs/2506.07683)) [S]. Better: normalized degree centrality [V same paper] | Edges only | **High** on small models (quantiles are meaningless below ~15 nodes). Gateways, LBs and brokers are hubs by design, so whitelist infra kinds |
| A4 | **Unstable dependency** (Arcan, Designite, Martin's SDP) | A component depends on components less stable than itself ([search summary of ICSME'16](https://boa.unimib.it/handle/10281/130605)) [S] | Edge `u→v` with `I(v) > I(u)`. Flag `u` if the share of such edges exceeds a threshold (a 30% default for Arcan is reported second-hand [S]). dependency-cruiser's `moreUnstable` is the same per-edge test [V] ([rules ref](https://github.com/sverweij/dependency-cruiser/blob/main/doc/rules-reference.md)) | Edges (to compute `I`) | Medium. Instability on a hand-drawn model reflects drawing completeness, not code |
| A5 | **God component / Megaservice** (Arcan, Designite; Taibi: Megaservice) | Oversized component. Megaservice = "a service that does a lot of things", with several business processes in one service ([Taibi et al.](https://arxiv.org/abs/1908.04101)) [V] | Number of children (components inside a container), number of responsibilities/capabilities tagged, or LOC after code import, above a percentile of siblings | Children, or `responsibilities[]`, or LOC | Medium–high: size alone is a weak signal. Report as `note`/`review` |
| A6 | **Shared persistence** (Taibi; Neri et al.) | Two or more services access the same database ([Neri et al.](https://arxiv.org/abs/1906.01553)) [V] | `kind(d)=db ∧ |{s : s→d, kind(s)=service}| ≥ 2`, unless `d.shared_ok` or the services share an `owner` / bounded context | DB nodes, data-access edges, optional owner | Medium: one DB *server* with separate schemas per service is fine. Needs `schema`/`logical_db` attribute to disambiguate |
| A7 | **Inappropriate service intimacy** (Taibi) | A service connects to another service's private data instead of its API ([Taibi et al.](https://arxiv.org/abs/1908.04101)) [V] | Edge `s→d` where `d.owner ≠ s` and `d.owner` is another service | `owner` on data stores | Low if ownership is modeled |
| A8 | **Hardcoded / endpoint-based interactions** (Taibi: Hardcoded Endpoints; Neri: Endpoint-based Service Interactions) | Services address a specific instance or IP:port, so new replicas are unreachable ([Neri et al.](https://arxiv.org/abs/1906.01553)) [V] | Sync edge to a target with `replicas>1` where `discovery = no` and no LB/gateway/mesh on the path | `discovery` (tri-state), `replicas` | High if `discovery` is unknown → report **open/not checkable**, not fail |
| A9 | **No API gateway** (Taibi; Neri) | External clients invoke microservices directly ([Neri et al.](https://arxiv.org/abs/1906.01553)) [V] | Edge `person/external → service` with ≥2 such service targets and no `gateway`/`lb` node in between | Node kinds, external/internal boundary | Medium: single-service systems or BFF designs are legitimate. Threshold ≥2 entry services |
| A10 | **ESB misuse** (Taibi: ESB Usage; Neri: ESB Misuse) | An ESB becomes a central hub and bottleneck holding business logic ([Neri et al.](https://arxiv.org/abs/1906.01553)) [V] | `kind(v)=esb` with a hub score (A3) above threshold, or `v.has_business_logic` | Kind `esb`, optional flag | Medium. Simply having an ESB is not a smell; centralization is |
| A11 | **Wobbly service interaction** (Neri) / **missing timeout, circuit breaker** | A failure in one service cascades to its consumers ([Neri et al.](https://arxiv.org/abs/1906.01553)); mitigations are circuit breakers, timeouts and bulkheads [V]. AWS REL05-BP05 "Set client timeouts" [V] ([REL05](https://docs.aws.amazon.com/wellarchitected/latest/reliability-pillar/design-interactions-in-a-distributed-system-to-mitigate-or-withstand-failures.html)) | Sync edge to a remote element (different container/deployment node) with `timeout = no` (fail), or `cb = no` *and* the edge is `hard` (warn) | `sync`, `timeout`, `cb`, `hard` (tri-state) | Low when explicit; **open** when unknown. Azure: CB is not needed for message-driven flows with DLQ [V] |
| A12 | **Retry storm / unbounded retries** (Azure Retry Storm; AWS REL05-BP03) | Retrying failed requests too often ([Azure](https://learn.microsoft.com/en-us/azure/architecture/antipatterns/)) [V] | Path in `Gs` where ≥2 consecutive edges have `retry = yes` without `retry_budget`/backoff (multiplicative amplification `Π (1+retries_i)`) [I] | `retry.count`, `retry.backoff` | Medium. Amplification is a real effect, but limits are often configured outside the diagram |
| A13 | **Long synchronous call chain** [I; AWS REL04-BP02 "Implement loosely coupled dependencies" [V]](https://docs.aws.amazon.com/wellarchitected/latest/reliability-pillar/design-interactions-in-a-distributed-system-to-prevent-failures.html) | Latency and availability multiply along hard sync chains | Longest path in the condensation of `Gs` (DAG after SCC collapse) from an entry point > N (default 3–4). Also report the composed availability (§2.4) | `sync`, `hard`, optional `availability`, `latency` | Medium: depth threshold is opinion. Show the value and let the user set the limit |
| A14 | **Chatty interaction** (Azure Chatty I/O) | Many small network requests ([Azure](https://learn.microsoft.com/en-us/azure/architecture/antipatterns/)) [V] | Edge with `calls_per_request > k`, or ≥k parallel edges between the same pair | Call multiplicity on edges (rarely drawn) | High. Usually **not checkable** at the model level; better from Petri-net firing counts or traces |
| A15 | **Single point of failure** (GCP reliability: "avoid any single points of failure" [V]) ([GCP](https://docs.cloud.google.com/architecture/framework/reliability/build-highly-available-systems)) | A component whose failure takes down the flow | In the request graph from entry `e`, any node `v` that **dominates** all sinks of a critical flow (`petgraph::algo::dominators` [V]) with `replicas ≤ 1` or `zones ≤ 1` | Entry points, `replicas`, `zones`, flow criticality | Medium: managed services (S3, DynamoDB, Cloud SQL HA) are multi-AZ internally. Default by kind + override |
| A16 | **Multiple services in one container** (Neri) | Services cannot be deployed or scaled independently [V] | Deployment node hosting ≥2 `service` elements of different owners | Deployment view | Low |
| A17 | **Distributed monolith** [composite, no single canonical definition verified] | Services that must be deployed and changed together | Conjunction: A1/A2 SCC among services ∧ A6 shared DB ∧ high sync coupling (A13). Report as a *meta-finding* linking the evidence findings [I] | As for the parts | High as a label. Show it as an aggregation, not as a separate fail |
| A18 | **Connector envy / ambiguous interface / scattered functionality** (Garcia) | Interaction logic inside components; one generic entry point dispatching many services; one concern spread over many components [S] | Need interface operations and concern tags: `|ops(i)|=1 ∧ dispatch_targets>1`; concern tag on >k components | Interfaces with operations, `concerns[]` | High at diagram level. Better after code import (v2) |
| A19 | **Lack of monitoring** (Taibi, NEW) | No observability [V table] | Runtime element without an edge to an `observability` node or a `monitored` flag | Flag or edge | Medium |

**Design note [I].** Every rule should declare `requires: [attribute paths]`. If any required attribute is `unknown` on the matched elements, the result is **open** ("not checkable with the data in the model") and lists the missing attributes. μTOSCA defaults `timeout`/`circuit_breaker`/`dynamic_discovery` to `false` ([microTOSCA.yml](https://raw.githubusercontent.com/di-unipi-socc/microTOSCA/master/1.3/microTOSCA.yml)) [V]. That would make every quickly drawn diagram light up red, so do not copy those defaults.

---

## 2. Metrics

### 2.1 Martin package metrics [V, [Wikipedia: Software package metrics](https://en.wikipedia.org/wiki/Software_package_metrics), citing Martin, *Agile Software Development*, 2002]

| Metric | Formula | Range / reading |
|---|---|---|
| Ca (afferent coupling) | # classes outside the package that depend on classes inside | responsibility |
| Ce (efferent coupling) | # classes outside that the package depends on | dependence |
| I (instability) | `Ce / (Ca + Ce)` | 0 = maximally stable, 1 = maximally unstable |
| A (abstractness) | abstract classes + interfaces / total classes | 0 concrete … 1 abstract |
| D (distance from main sequence) | `|A + I − 1|` | 0 on the main sequence, 1 farthest away |

Mapping to a C4 model [I]: at the component level, count *elements* instead of classes. `A` needs a notion of abstraction, for example elements of kind `interface`/`contract`/`API spec`, or a boolean `abstract`. Without that, compute only I and SDP violations (A4). After code import (§5), compute A from traits vs. concrete types per module. Wikipedia notes that some tutorials get I wrong (it is `Ce/(Ca+Ce)`, not the reverse) [V].

### 2.2 Coupling, cohesion, fan-in/out

| Metric | Definition on the model graph [I] | Use |
|---|---|---|
| fan-in / fan-out | in/out-degree on dependency edges at one level (Arcan computes fan-in, fan-out, instability [S]) | A3, A4 |
| Normalized degree centrality | `deg(v)/(n−1)` | Hub detection recommended over raw quantiles by [Bakhtin et al.](https://arxiv.org/abs/2506.07683) [V] |
| PageRank | `petgraph::algo::page_rank` [V] | Ranking "most depended-upon" elements |
| Coupling between containers | # edges crossing container boundaries / # edges | Distributed-monolith evidence |
| Relational cohesion (component) | internal edges / children | God component / feature concentration |
| Dependency density | average degree of the package graph (DPy's "Dense Structure" [S]) | Whole-model smell |

### 2.3 Depth of call chains

`depth(e)` = longest path, counted in hard sync edges, in the condensation DAG of `Gs` reachable from entry `e`. petgraph provides `condensation` and `toposort` but **no longest-path function** [V, [algo index](https://docs.rs/petgraph/latest/petgraph/algo/index.html)]. Implement a DP over the topological order (O(V+E)) [I]. Report the path as evidence.

### 2.4 Availability math [V, [AWS WA Reliability – Availability](https://docs.aws.amazon.com/wellarchitected/latest/reliability-pillar/availability.html)]

| Case | Formula | Example from AWS |
|---|---|---|
| Hard (serial) dependencies | `A = A_invoker × Π A_dep_i` | 99.99% × 99.99% × 99.99% ≈ 99.97% |
| Independent redundant components | `A = 1 − Π (1 − A_i)` (AWS writes it for two equal components) | two × 99.9% → 99.9999% |
| Estimate from MTBF/MTTR | `A = MTBF / (MTBF + MTTR)` | 150 d MTBF, 1 h MTTR → 99.97% |
| Soft dependency | excluded from the product, because the application compensates | AWS hard vs. soft distinction |
| k-of-n (quorum) [I, standard reliability math, not from AWS page] | `Σ_{j=k..n} C(n,j) a^j (1−a)^{n−j}` | 2-of-3 replicas |

Caveats: the formulas assume **independence**. Replicas in one AZ or sharing a DB are not independent (AWS uses separate AZs as the example) [V]. Implement as a lattice computation over the flow DAG: serial = product, parallel = 1−Π(1−a) [I]. Show the computed value next to the user's `availability_target`. When it falls short, flag as "target not achievable by topology", which counts as a proof-style result rather than a heuristic.

---

## 3. Cloud well-architected rule sources

### 3.1 Frameworks

| Provider | Pillars | Machine-readable assets | Link |
|---|---|---|---|
| AWS Well-Architected | Operational excellence, security, reliability, performance, cost optimization, sustainability (pillar IDs in the custom-lens spec) [V] | Best-practice IDs (e.g. REL04-BP02, REL05-BP03/05, REL09-BP01..04, REL10-BP01..03) [V]; **AWS Config conformance pack mapped to the Reliability pillar** [V]; custom-lens JSON with `riskRules` (`HIGH_RISK`/`MEDIUM_RISK`/`NO_RISK`, boolean `condition`s over choices) [V] | [Reliability pillar](https://docs.aws.amazon.com/wellarchitected/latest/reliability-pillar/availability.html), [Config ↔ WA Reliability](https://docs.aws.amazon.com/config/latest/developerguide/operational-best-practices-for-wa-Reliability-Pillar.html), [Lens format](https://docs.aws.amazon.com/wellarchitected/latest/userguide/lenses-format-specification.html) |
| Azure Well-Architected | Reliability, security, cost optimization, operational excellence, performance efficiency; reliability checklist RE:01–RE:10 (e.g. RE:05 redundancy, RE:06 scaling, RE:07 self-preservation, RE:09 DR) [V] | **APRL v2**: recommendations with Azure Resource Graph queries that return only non-compliant resources; MIT per repo sidebar [V]; azqr bundles the APRL queries [S] | [Reliability checklist](https://learn.microsoft.com/en-us/azure/well-architected/reliability/checklist), [APRL v2](https://azure.github.io/Azure-Proactive-Resiliency-Library-v2/welcome), [APRL repo](https://github.com/Azure/Azure-Proactive-Resiliency-Library-v2) |
| Google Cloud Well-Architected | Operational excellence; security, privacy & compliance; reliability; cost; performance; sustainability [V] | Reliability principle "build highly available systems through resource redundancy": identify failure domains, replicate across zones/regions, use health-checked LBs [V]; zonal vs regional resources [S] | [Framework](https://docs.cloud.google.com/architecture/framework), [HA via redundancy](https://docs.cloud.google.com/architecture/framework/reliability/build-highly-available-systems), [DR guide](https://docs.cloud.google.com/architecture/disaster-recovery) |

### 3.2 What is checkable on a diagram-level model

Rule names in the AWS column are verified Config managed rule identifiers from the conformance pack page above, unless marked otherwise.

| Check (diagram level) | Minimum attributes | AWS reference | Azure reference | GCP reference |
|---|---|---|---|---|
| DB not multi-AZ / no standby replica | `kind=db`, `zones`, `replicas` | `rds-multi-az-support`, `rds-cluster-multi-az-enabled`; REL10-BP01 | RE:05 redundancy | Regional over zonal resources ([DR guide](https://docs.cloud.google.com/architecture/disaster-recovery)) [S] |
| Compute in one AZ / one instance | `replicas`, `zones` | `elbv2-multiple-az`, `clb-multiple-az`, `lambda-vpc-multi-az-check` | RE:05 | HA via redundancy |
| No backup / no PITR for stateful store | `backup` (tri-state), `pitr` | `db-instance-backup-enabled`, `dynamodb-pitr-enabled`, `rds-in-backup-plan`, `elasticache-redis-cluster-automatic-backup-check`; REL09-BP01/03 | RE:09 DR | DR guide |
| Public object storage | `kind=bucket`, `public_read` | `s3-bucket-public-read-prohibited` ([rule](https://docs.aws.amazon.com/config/latest/developerguide/s3-bucket-public-read-prohibited.html)) [V] | (security pillar, not researched in depth) | (security pillar, not researched in depth) |
| No autoscaling on scalable tier | `autoscaling` | `dynamodb-autoscaling-enabled`, `autoscaling-group-elb-healthcheck-required` | RE:06 scaling | "use elasticity" (perf pillar) |
| Replicas without LB / health checks | `replicas>1`, upstream `lb`, `health_check` | `autoscaling-group-elb-healthcheck-required`, `elb-cross-zone-load-balancing-enabled` | RE:07 | LB with health checks [V] |
| Async consumer without DLQ | `kind=queue/function`, `dlq` | `lambda-dlq-check` | Circuit-breaker page: DLQ handling [V] | – |
| Data store not encrypted at rest | `encrypted` | `rds-storage-encrypted`, `s3-bucket-server-side-encryption-enabled`, `efs-encrypted-check` | security pillar | security pillar |
| No observability / logging | `monitored` or edge to observability node | `rds-enhanced-monitoring-enabled`, `elb-logging-enabled`, `api-gw-xray-enabled` | RE:10 | Ops excellence |
| Missing client timeouts / retry limits | edge `timeout`, `retry` | REL05-BP05, REL05-BP03 | RE:07, Retry/CB patterns | Reliability: graceful degradation |

Not checkable at diagram level without extra data: cost rightsizing, patch compliance, IAM least privilege, quotas and DR test frequency. Represent those as `notApplicable` or as checklist questions, not as findings [I].

### 3.3 Licensing and terms for reusing rule texts (not legal advice)

| Source | Terms found | Consequence [I] |
|---|---|---|
| AWS docs (docs.aws.amazon.com, including WA whitepapers) | AWS Site Terms: documentation "is licensed under CC-BY-SA-4.0", code under MIT-0 [V] ([AWS Site Terms](https://aws.amazon.com/terms/)). The rest of the AWS site is personal-use only | Short adapted descriptions allowed with attribution. **ShareAlike**: keep AWS-derived texts in a separate rule-pack data file licensed CC BY-SA 4.0. Rule IDs such as `REL10-BP01` are facts and safe to reference |
| Microsoft Learn (Azure WAF, Architecture Center) | Default ToU: personal and **non-commercial** use, **no modifications**, unless a document has explicit license terms [V] ([MS Learn ToU](https://learn.microsoft.com/en-us/legal/termsofuse)). Page metadata shows the WAF source lives in `MicrosoftDocs/well-architected-pr`; the public `MicrosoftDocs/well-architected` URL returned 404 on 2026-11-02 [V]. Other MicrosoftDocs repos use CC BY 4.0 + MIT [S] | **Do not copy text.** Paraphrase in your own words and link to RE:xx pages. APRL v2 (MIT) [V] and the TMT templates (MIT) [V] are reusable with notice |
| Google Cloud docs | Google Developers Site Policies: most content CC BY 4.0, code samples Apache 2.0; trademarks and media excluded; attribution with a link back required [V] ([Site Policies](https://developers.google.com/terms/site-policies)). Footer not confirmed on the specific architecture-framework pages [S] | Reuse with attribution. Check the footer of each page you quote |
| Academic catalogs | Copyright of the publishers. Cerny et al. is CC BY [S] | Cite and paraphrase. Smell *names* are not protected text |

---

## 4. Existing tools and how they express rules

| Tool | Analyzes | Rule expression | Severity / suppression / baseline | Lesson for us |
|---|---|---|---|---|
| **Arcan** (UniMiB; Java, later C/C++) | Code → dependency graph in a graph DB | Built-in detectors (CD, HL, UD, GC, …) [S] | Thresholds; ML-learned thresholds planned [S] | Fixed detector set is enough for v1; thresholds need tuning |
| **Designite / DPy** (Sharma) | C#, Java, Python | Built-in detectors [S] | – | Same smell family; size thresholds |
| **ArchUnit** (Java) | Bytecode | Fluent Java DSL `classes().that()…should()…`, `slices()…beFreeOfCycles()`, `layeredArchitecture()`, `adhereToPlantUmlDiagram(..)` [V] ([user guide](https://www.archunit.org/userguide/html/000_Index.html)) | `FreezingArchRule` = violation store/baseline; `archunit_ignore_patterns.txt` regex suppressions [V] | **Diagram-as-rule** (PlantUML) is close to model-first: the model defines allowed edges and code import checks conformance |
| **jQAssistant** | Code/artifacts → Neo4j | **Cypher** in XML rule files: *concepts* (enrich graph), *constraints* (violation if rows returned), *groups*; `requiresConcept`/`providesConcept` ordering [V] ([manual](https://jqassistant.github.io/jqassistant/current/)) | info/minor/major/critical/blocker; warn/fail thresholds; `jqassistant-baseline.xml` [V] | Concept/constraint split = derived facts + rules. Maps directly to Datalog |
| **Sonargraph** (hello2morrow) | Java, C#, C++, Python… | Architecture **DSL**: artifacts (include/exclude filters), interfaces (incoming) and connectors (outgoing); access needs explicit `connect`; multiple aspects checked in parallel; Groovy script API [S] ([manual](https://eclipse.hello2morrow.com/doc/standalone/content/index.html)) | Build fails on violations [S] | Port/connector-based allowed-dependency model |
| **Structure101** | Code structure | Layer/structure rules; acquired by Sonar in Oct 2024 and being folded into SonarQube [V] ([press release](https://sonarsource.com/company/press-releases/sonar-acquires-structure101-to-strengthen-code-quality-offering)) | – | Commercial; no open rule format to reuse |
| **dependency-cruiser** (JS/TS) | Module graph | JSON `forbidden` / `allowed` / `required` rules with `from`/`to` regex, `circular`, `orphan`, `reachable`, `moreUnstable`, `numberOfDependents…` [V] ([rules reference](https://github.com/sverweij/dependency-cruiser/blob/main/doc/rules-reference.md)) | error/warn/info/ignore; `comment` per rule [V] | **Best simple declarative format** to imitate for user rules |
| **Structurizr** (C4) | Workspace model | Built-in *inspections*, Checkstyle-like (e.g. `model.element.disconnected`, `model.container.technology`, `model.relationship.description`) [V] ([inspections](https://docs.structurizr.com/workspaces/inspections)) | error (default)/warning/info/ignore; set via properties on workspace, element, relationship or view, inherited by children, with wildcards [V] | **Suppression as element property inherited down the C4 tree** fits our drill-down levels |
| **LikeC4** | `.c4` DSL | `likec4 validate` checks syntax errors and layout drift only; no lint rules documented [V] ([CLI](https://likec4.dev/tooling/cli/)) | exit code | No semantic rule engine, which is a gap we can fill |
| **Archi** (ArchiMate) | Model | Built-in Validator (e.g. invalid nesting, junction rules per spec) [S] ([issue #577](https://github.com/archimatetool/archi/issues/577)) | – | Metamodel well-formedness is a separate rule class |
| **μFreshener** | μTOSCA topology | Built-in smell detectors + refactoring suggestions; team (organisational) smells [V/S] | – | Closest prior art for model-level microservice smells |
| **OWASP Threat Dragon** | DFD diagrams | Threats per element type (STRIDE/LINDDUN/CIA/DIE/PLOT4ai); "threat by context" uses element properties (flow: protocol, encrypted, public network; store: encrypted, stores credentials, …) [V] ([threats doc](https://www.threatdragon.com/docs/usage/threats.html)) | Severity TBD/Low/Medium/High/Critical; status N/A, Open, Mitigated, Accepted, Transferred, Avoided, Eliminated; out-of-scope elements excluded [V] | **Status vocabulary for findings** (Accepted/Transferred) is better than plain suppress |
| **Microsoft Threat Modeling Tool** | DFD stencils + properties | Template `.tb7` (XML, **MIT**) with per-threat `<Include>`/`<Exclude>` boolean expressions, e.g. `source is 'GE.DS' and (target is 'GE.P' or target is 'GE.EI')`, `(flow crosses 'GE.TB.L' or flow crosses 'GE.TB.B')`, exclude `flow.authenticatesSource is 'Yes' or source.implementsAuthenticationScheme is 'Yes'` [V] ([templates repo](https://github.com/microsoft/threat-modeling-templates)) | Out-of-scope elements removed from the generation matrix [S] | **Proven rule-on-diagram engine**: a tiny boolean DSL over `source`/`target`/`flow` + attributes + boundary crossing is enough for many rules |
| AWS WA Tool custom lens | Questionnaire | JSON: pillars → questions → choices; `riskRules` with boolean conditions [V] | HIGH/MEDIUM/NO_RISK | Rule-pack metadata format idea (pillar mapping, improvement plan URL) |

### 4.1 Rule language options for a Rust core

| Option | Fit | Notes |
|---|---|---|
| **Plain Rust rule functions** over a typed model plus petgraph | ★★★ v1 | Fastest to build and debug; full access to graph algorithms. Rules are not user-editable |
| **ascent** (Datalog macros) | ★★★ for recursive or relational rules | Stratified negation, aggregates (`sum/min/max/count/mean`), lattices, rayon-parallel `ascent_par!`, BYODS [V] ([repo](https://github.com/s-arash/ascent)); crates.io 0.8.1, MIT [V]. **Compile-time** macro: rules are fixed at build time [I] |
| **crepe** | ★★ | Datalog proc macro, semi-naive, stratified negation; 0.2.0, MIT/Apache [V] ([repo](https://github.com/ekzhang/crepe)). Fewer features than ascent (no lattices documented) |
| **datafrog** | ★ | Minimal embedded Datalog *engine* without a runtime; you write the fixpoint loop by hand (`Iteration`, `Variable`, `from_join`) [V] ([repo](https://github.com/rust-lang/datafrog)); 2.0.1 |
| **Trustfall** | ★★ for user-written queries | GraphQL-like queries with `@filter`, `@recurse`, `@output` over any data source via an adapter; used by cargo-semver-checks; Apache-2.0, 0.8.2 [V] ([repo](https://github.com/obi1kenobi/trustfall)). **Runtime** queries suit user rules |
| **Cozo** (CozoScript Datalog DB) | ✗ | Built-in graph algorithms, but crates.io `cozo` 0.7.6 was last updated 2023-12 (MPL-2.0) [V] → stale |
| **Cypher** (jQAssistant style) | ✗ for embedded | Needs Neo4j or an embedded Cypher engine; no mature embedded Rust Cypher engine verified [I/unverified] |
| **TMT-style boolean filters** / dependency-cruiser-style JSON | ★★★ for user rules v2 | Small, safe, serializable; enough for local (edge/element) rules. Global rules (cycles, dominators) are exposed as built-in predicates (`in_cycle`, `is_spof`, `depth>3`) [I] |
| **differential-dataflow** | ★ | Incremental (good for live canvas updates) but heavy; 0.25.1, MIT [V crates.io] |

Graph algorithms: **petgraph 0.8.3** provides `tarjan_scc`, `condensation`, `toposort`, `dominators`, `articulation_points`, `bridges`, `page_rank`, `all_simple_paths`, `has_path_connecting`, `greedy_feedback_arc_set`, `k_shortest_path` [V] ([docs](https://docs.rs/petgraph/latest/petgraph/algo/index.html)). It has no longest path and no betweenness centrality, so implement those yourself [V absence].

---

## 5. Later: Rust code import

| Source | Gives | Stability / cost | Link |
|---|---|---|---|
| `cargo metadata --format-version 1` | Packages, targets (lib/bin/proc-macro), dependency kinds (normal/dev/build), resolved graph with `dep_kinds` and features, workspace members → **Container/Component level** for workspaces | Stable JSON v1; additive changes allowed [V]; `cargo_metadata` crate 0.23.1 [V] | [docs](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html) |
| `syn` | Parses Rust source into a syntax tree (fast, no compiler) → `mod`, `use`, `impl`, `async fn`, attribute and macro *syntax* | No name resolution or macro expansion [I], so `use` paths are textual. syn 3.0.9 [V crates.io] | [docs.rs/syn](https://docs.rs/syn) |
| `ra_ap_*` (rust-analyzer as a library: `ra_ap_hir`, `ra_ap_ide`, …) | Resolved module tree, name resolution, types, call targets, trait impls → accurate module deps, method-call resolution (`tokio::sync::Mutex::lock`) | Published continuously as **0.0.x** (0.0.358 on 2026-10-12) [V]; breaking changes on every bump under Cargo semver [I]. Pin exact versions | [crates.io ra_ap_hir](https://crates.io/crates/ra_ap_hir) |
| `rust-analyzer scip <path> [--output index.scip]` (also `lsif`, `analysis-stats`, `diagnostics`) | SCIP index: symbols, definitions, references (protobuf) → language-neutral dependency and usage graph | CLI subcommands "do not provide any stability guarantees" (rust-analyzer source) [V]; SCIP schema Apache-2.0, rust-analyzer is a listed indexer [V]; `scip` crate 0.10.0 [V] | [flags.rs](https://github.com/rust-lang/rust-analyzer/blob/master/crates/rust-analyzer/src/cli/flags.rs), [SCIP](https://github.com/sourcegraph/scip) |
| `cargo-modules` | `structure`, `dependencies` (DOT; `--acyclic` errors on cycles; filters `--no-uses`, `--no-externs`…), `orphans` [V] | MPL-2.0, 0.28.0 [V]. Useful as reference or oracle | [repo](https://github.com/regexident/cargo-modules) |
| `rustc_public` (formerly Stable MIR) | MIR bodies: precise call graph after monomorphization, `async` lowered to coroutines, drop and lock-guard scopes | Nightly + `rustc-dev`; "still completely unstable"; crates.io release planned, not confirmed [S] | [book](https://rust-lang.github.io/rustc_public/), [rustdoc](https://doc.rust-lang.org/stable/nightly-rustc/rustc_public/index.html) |
| Charon (AeneasVerif) | Extracts a whole crate's MIR into ULLBC/LLBC **JSON**; rustc driver on pinned nightly [S] | Research tool, used by Aeneas/Eurydice | [paper arXiv 2410.18042](https://arxiv.org/html/2410.18042v3) |
| lockbud | MIR-level detectors: double-lock, conflicting lock order, condvar misuse, atomicity violation [V] | Pinned nightly (nightly-2026-02-07), BSD-3 [V] | [repo](https://github.com/BurtonQin/lockbud) |

**Mapping code → model [I]:**

| Model concept | Extraction heuristic | Tool tier |
|---|---|---|
| Container | workspace member with `bin` target, or a `[[bin]]` | cargo metadata |
| Component | top-level module or library crate | cargo metadata + syn/ra_ap |
| Dependency edge | `use`/path references between modules, weighted by count | syn (approx.) → ra_ap/SCIP (exact) |
| Abstractness `A` | traits / (traits + structs + enums) per module | syn |
| Queue / channel (Petri place) | calls to `tokio::sync::mpsc::channel`, `std::sync::mpsc::channel`, `crossbeam_channel::*`, `flume::*`; sender/receiver flowing to different tasks | ra_ap or SCIP symbol references |
| Active process (Petri transition source) | `tokio::spawn`, `std::thread::spawn`, `#[tokio::main]` | ra_ap / SCIP |
| Shared resource / lock | `Mutex`, `RwLock` (std / tokio / parking_lot), `Arc<…Mutex…>` | ra_ap (types) |
| Remote sync call | `.await` on `reqwest`, `tonic` client, `sqlx` → edge to external element; `timeout` attr = wrapped in `tokio::time::timeout` or client `.timeout(..)` | ra_ap; MIR for exact scoping |
| Retry / circuit breaker | use of `tower::retry`, `tower::timeout`, `backoff`, `tokio-retry` … | SCIP symbol match |

Recommended path: phase 1 `cargo metadata` + `syn` (no nightly, fast, good enough for module graph and A/I). Phase 2 `rust-analyzer scip` as a subprocess, which avoids linking the unstable `ra_ap_*` API. Phase 3 MIR (rustc_public/Charon) only if the behavior-model extraction needs it [I].

---

## 6. Reporting findings

### 6.1 Result status: use SARIF semantics [V, [SARIF 2.1.0 errata01](https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/sarif-v2.1.0-errata01-os-complete.html) §3.27.9–10, §3.35, §3.27.24]

| Our UI state | SARIF `kind` | Meaning per spec |
|---|---|---|
| **Anti-pattern found** | `fail` (+ `level` error / warning / note) | Problem found; severity via `level` |
| **Clean** | `pass` | Rule evaluated, no problem found |
| **Not checkable with model data** | `open` | "insufficient information to decide whether a problem exists" |
| Rule irrelevant (e.g. no DB in model) | `notApplicable` | Rule does not apply to the target |
| Heuristic, human must judge (hub, god component) | `review` | Requires human review |
| Metric report | `informational` | Not a problem |
| Proven by formal engine | `pass`/`fail` + property `provenBy: "petri"` [I] | Lets the UI show "proved" vs "scanned" |

Suppressions: `kind` inSource/external; `status` accepted/underReview/rejected; `justification` string [V]. Baseline: `baselineState` new/unchanged/updated/absent [V]. Rule metadata: `reportingDescriptor` with `helpUri` and `defaultConfiguration` [V, §3.49].

### 6.2 Finding schema [I]

```text
Finding {
  rule_id: "ARCH-CYCLE-SYNC", rule_version, pack: "core" | "aws" | ...
  outcome: Fail{severity} | Pass | Open{missing: [AttrRef]} | NotApplicable | Review
  primary: ElementRef            // stable model UUID + C4 level + view ids where visible
  related: [ElementRef|EdgeRef]  // e.g. the whole cycle, the dominated sinks
  evidence: { witness_path?, metrics: {I:0.83, fan_in:12}, attrs_used: {edge#42.timeout: No} }
  message: templated text with {placeholders}
  remediation: short text + links (e.g. AWS REL05-BP05, Azure Circuit Breaker)
  fingerprint: hash(rule_id, sorted element UUIDs)   // stable across layout moves
  suppression?: { reason (required), author, created, expires?, status: Accepted|UnderReview|Transferred, scope: element|subtree|model }
  provenance: Scanned | Proved | Disproved(counterexample_ref)
}
```

UI rules [I]:
- **Link back:** clicking a finding selects `primary` and highlights `related`. If the element is at another C4 level, navigate down or up. Paths are drawn as an overlay. On a level where the element is collapsed, the badge goes on the nearest visible ancestor (aggregate count).
- **Open findings are actionable:** "Set `timeout` on 3 edges to evaluate ARCH-TIMEOUT". One click filters the canvas to those edges. Show **coverage**: per rule, the % of matched elements with complete data.
- **Suppressions** live in the model file as Structurizr-style properties: element- or relationship-level, inherited by children ([Structurizr inspections](https://docs.structurizr.com/workspaces/inspections)) [V]. A reason is mandatory, with optional expiry, and it is keyed by `fingerprint` so it survives renames. Use Threat Dragon-style dispositions (Accepted / Transferred / Mitigated) [V] instead of plain on/off.
- **Baseline:** like ArchUnit `FreezingArchRule` or jQAssistant baseline [V]. Show only *new* findings after import of a legacy system.
- **Severity defaults:** error = availability/data-loss risk on a critical flow (SPOF, no backup, sync cycle); warning = coupling/resilience (shared DB, no timeout); note/review = size and hub heuristics.
- Export SARIF so findings can go into CI and GitHub code scanning later.

---

## 7. Recommendations for this project

### 7.1 Rule engine

1. **Model → fact extraction layer** (Rust): typed element and edge tables with **tri-state attributes** (`Known(v) | Unknown`), and kind defaults from the palette (e.g. AWS S3 ⇒ multi-AZ = Known(true)) [I].
2. **Graph facts via petgraph** per C4 level and per view: SCCs, condensation depth, dominators from entry points, articulation points, degree centrality, page rank, instability.
3. **Rules as Rust functions** behind a `Rule` trait (`id`, `requires()`, `evaluate(&Facts) -> Vec<Finding>`). Use **ascent** inside rules that are naturally recursive or relational (transitive reachability across levels, ownership propagation, composite "distributed monolith"). Its lattices also suit availability composition (min/product over paths) [I].
4. **v2 user rules:** a small serializable DSL modeled on MS TMT `Include`/`Exclude` expressions and dependency-cruiser `from`/`to`. Evaluate it at runtime and expose global facts as predicates (`in_sync_cycle`, `is_spof`, `chain_depth`). Consider Trustfall if users want ad-hoc queries [I].
5. **Interplay with the prover** [I]: run the scanner always (cheap). When the Petri-net engine proves or refutes a property tied to a rule (e.g. deadlock-freedom for A2), upgrade the finding's `provenance`. Scanning is the fallback, but its findings remain useful as explanations.

### 7.2 v1 rule set (ranked by value × feasibility)

| Rank | Rule ID | Based on | Value | Feasibility | Minimum attributes |
|---|---|---|---|---|---|
| 1 | ARCH-CYCLE | A1 cyclic dependency (per level) | High | Trivial (`tarjan_scc`) | dependency edges |
| 2 | RUN-SYNC-CYCLE | A2 sync cycle among runtime elements | High | Trivial | edge `sync` |
| 3 | REL-SPOF | A15 dominator with `replicas≤1 ∨ zones≤1` on a critical flow | High | Medium | entry points, `replicas`, `zones`, kind defaults |
| 4 | REL-NO-TIMEOUT | A11 remote sync edge without timeout | High | Trivial | edge `sync`, `timeout`, remote vs local (container boundary) |
| 5 | DATA-SHARED-DB | A6 + A7 shared persistence / foreign data access | High | Trivial | `kind=db`, data edges, `owner` (optional `logical_db`) |
| 6 | REL-AVAIL-BUDGET | §2.4 composed availability vs target along hard paths | High | Medium | `availability` per element (kind defaults), edge `hard`, `availability_target` on flow |
| 7 | REL-NO-BACKUP | stateful store without backup/PITR | High | Trivial | `kind∈{db,bucket,volume}`, `backup` |
| 8 | REL-SINGLE-AZ-DB | DB not multi-AZ / no replica | High | Trivial | `zones`, `replicas` on db |
| 9 | RUN-CHAIN-DEPTH | A13 sync chain depth > N | Med-High | Easy (DP over condensation) | edge `sync`, entry points |
| 10 | REL-NO-CB | A11 hard sync edge to a remote service without CB/fallback | Medium | Trivial | `cb`, `hard` |
| 11 | SEC-PUBLIC-STORE | public bucket/DB, or DB reachable from internet without gateway | Medium | Trivial | `public`, boundary/zone (internet) |
| 12 | MS-NO-GATEWAY | A9 external actor calls ≥2 services directly | Medium | Trivial | kinds, external boundary |
| 13 | ARCH-UNSTABLE-DEP | A4 SDP violation (`I(v) > I(u)`) | Medium | Trivial | edges (I computed) |
| 14 | REL-RETRY-AMPLIFY | A12 consecutive retrying edges | Medium | Easy | `retry.count` |
| 15 | ARCH-HUB / ARCH-GOD (review) | A3 / A5, reported as `review` | Low-Med | Easy | edges, children count |

Each rule returns `open` with the list of missing attributes whenever a required attribute is `Unknown`.

### 7.3 Minimum model schema implied by v1 [I]

- **Element:** `id (UUID)`, `kind` (palette type, including cloud product), `level` (C4), `parent`, `owner` (team/service), `replicas`, `zones`, `regions`, `availability` (default per kind), `stateful`, `backup`, `pitr`, `public`, `autoscaling`, `encrypted`, `monitored`, `entry_point` / `critical_flow` tags.
- **Edge:** `kind` (uses / reads / writes / publishes / subscribes / deploys), `sync`, `hard`, `timeout`, `retry {count, backoff}`, `cb`, `discovery`, `protocol`, `crosses_boundary` (derived).
- **Flow (optional):** named path from an entry point with `availability_target` and latency budget.
- All boolean-like attributes tri-state. Palette kinds carry defaults with a citation (e.g. "regional resource" per GCP DR guide).

---

## 8. Open questions

1. **Tri-state vs. defaults:** should palette defaults (e.g. "managed DB ⇒ multi-AZ") count as `Known` or as `Assumed`? `Assumed` lets results show "pass (assumed)" and makes assumptions auditable.
2. **Level semantics:** do findings at the Component level propagate up (aggregate badge) or are rules evaluated only on the level being viewed? Cycles that appear only after collapsing (container-level cycle formed by component edges) need a decision.
3. **Petri-net coupling:** which attributes can be *derived* from attached behavior models (timeouts as timed transitions, retries as loops, channels as places) instead of being entered by hand? Coordinate with the formal-verification document.
4. **Thresholds:** hub/god/chain-depth thresholds on small hand-drawn models. Fixed defaults, per-project settings, or relative to model size?
5. **User-defined rules:** is a runtime DSL needed in v1, or are compiled rule packs (core, AWS, Azure, GCP) enough until code import lands?
6. **Licensing of rule packs:** do you accept a CC BY-SA 4.0 data file for AWS-derived texts, or write all texts from scratch and only link out (simpler)?
7. **Code import target:** SCIP subprocess (stable-ish format, unstable CLI) vs. linking `ra_ap_*` (rich semantics, churn every week). Nightly-only MIR tools conflict with shipping a desktop app.

## 9. Not verified / caveats

- Arcan's exact thresholds (Unstable Dependency 30%, Hub-Like quantiles) come from secondary summaries. The Arcan PDF returned HTTP 403.
- Garcia et al.'s full smell definitions: the PDF could not be parsed. Definitions come from author slides and search summaries; "Extraneous Adjacent Connector" is defined only via a related source.
- Cerny et al. 2023 contents (whether it names "distributed monolith" or "chatty service"): full text unreachable from the sandbox. "Distributed monolith" is presented here as a composite, not a cited definition.
- Taibi taxonomy scores: values printed as "6.05" in the extracted PDF text may be "6.5" (text-extraction artifact).
- Azure WAF GitHub license: the public repo URL returned 404, so CC BY 4.0 for WAF content is **not** confirmed. Treat MS Learn ToU as binding.
- The Google CC BY footer was not seen on the specific architecture-framework pages fetched.
- Designite's official documentation page was not readable. Its smell definitions come from Sharma's blog and secondary papers.
- Sonargraph DSL details come from hello2morrow blog posts (2015–2017) and manual tables of contents, not the full language spec.
- No embedded Cypher engine for Rust was evaluated.
- `rustc_public` crates.io release status is unconfirmed.
- crates.io "updated" dates reflect the last crate metadata update, not necessarily the last release.
