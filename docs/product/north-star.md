# North star, target user and non-goals

As of 2026-11-02. Settled decisions in `CLAUDE.md` are inputs here, not open questions. Tags: **[decided]** = user decision, **[assumption]** = not yet backed by evidence, to be tested in `opportunity-solution-tree.md`.

## Target user

**The solo designer of concurrent backends.** A senior developer or architect, working alone, who designs systems with services, queues, worker pools, locks and retries, keeps designs in git, reads Rust, and has been bitten by a stuck pipeline or a full queue before **[decided: primary user is the product owner; incident history is an assumption]**. Knows C4. Does not want to write TLA+ for every design, and would not keep a second artifact in sync.

Today's workaround: a sketch in draw.io or on a whiteboard, reasoning in the head about interleavings, then a load test or production finds the rest.

### Jobs to be done

| # | When (triggering situation) | I want to | So I can |
|---|---|---|---|
| J1 | I am about to commit to a design with queues, pools or locks between services | know whether it can deadlock or overflow | change the design before it costs code and incidents |
| J2 | I change an existing design (add a retry, a consumer, resize a pool) | re-check it in seconds | be sure the change did not bring a deadlock back |
| J3 | An incident happened (stuck consumer, full queue) | reproduce it as a counterexample in the model | know my fix closes it for every interleaving, not just the one I saw |
| J4 | I pick a capacity (queue length, pool size) | see whether a smaller value still never overflows | size resources from reasoning, not guesswork |
| J5 | I review a design without behavior models yet | see structural anti-patterns (cycles, missing timeouts) | catch cheap mistakes before modeling behavior |
| J6 | I come back to a design months later | read structure and behavior from one artifact in git | trust what I wrote then |

## North star metric

**Design questions answered per month.** A design question counts when all four hold:

1. It is about a **real design**: a system the user builds or runs, not a demo or test fixture.
2. It is a concrete question asked as a property ("can the queue between Order API and Billing exceed 50?").
3. The check ends **Proved**, or **Disproved** with a counterexample the user confirms as a defect of the design (not a modeling mistake).
4. The answer **confirmed or changed a design decision**, written down.

Why this metric: it counts value delivered (a decision informed by a decisive verdict), not activity. Unknown verdicts, toy models and modeling-mistake counterexamples do not count.

**Measured locally, no telemetry:** a dogfooding journal, one Markdown table row per question: date, project, question, verdict, minutes from opening the project to the verdict, decision changed (yes/no), notes. The journal is a private file outside the project repositories, because real designs may be confidential **[decided]**. Monthly sum = the metric.

| | Baseline | Target |
|---|---|---|
| v1 gate | 0 (tool does not exist) | First confirmed Disproved on a real design that changes a decision **[decided success criterion]** |
| Sustained | 0 | ≥ 4 per month (about one per week) for 3 consecutive months after slice 3 **[assumption: realistic usage rate]** |

## Input metrics

All from the journal plus a check log the app may write locally (no network). Targets are **[assumption]** until two weeks of dogfooding give a baseline.

| # | Input metric | Definition | Target |
|---|---|---|---|
| I1 | Time to first verdict | Minutes from creating a project for a real design to its first decisive verdict | ≤ 45 min for about 10 elements |
| I2 | Decisive rate | Share of checks on real designs that end Proved or Disproved, not Unknown | ≥ 80 %; every Unknown reason tallied |
| I3 | Counterexample understood | Share of Disproved verdicts where the user names the root cause from the replay alone, within 10 min, without reading the TOML | ≥ 80 % |
| I4 | Return use | Weeks with ≥ 2 sessions on a real design | ≥ 3 of 4 weeks |

## Guardrails

| Guardrail | Threshold | How we detect it |
|---|---|---|
| **Wrong Proved** | **0, ever. Stop-ship** | Differential test corpus checked against an independent engine in CI (see `opportunity-solution-tree.md` E1); any later contradiction (same check hash Proved, then a confirmed counterexample) is a P0 bug |
| Wrong Disproved | 0 | Every counterexample is replayed by the independent replayer before it is shown |
| Lost work | 0 incidents | Journal notes; crash-recovery journal (ADR 0001) |
| Check latency on real designs | Interactive checks answer within their budget (2 s target from the state-space spike), else Unknown with reason | Elapsed time in the verdict panel, noted in the journal |
| Scan noise | ≤ 20 % of findings suppressed with a reason meaning "not real" | Count of suppressions per scan |

## Non-goals

Not in v1, and not a reason to bend the design:

- Team use, sharing, collaboration, accounts, cloud backend, telemetry **[decided]**.
- Message order, request/reply matching, data in tokens, time, liveness and temporal logic beyond reachability (ADR 0002, 0003). Use TLA+, Quint or P for those.
- Overload, rates and metastability questions (E5, `experiments/e5-paper-test.md`). A Proved "always reachable" never means "will recover under load" **[decided]**.
- UML notations as something the user draws (class, component, deployment, activity, sequence) and database schema or ER modeling. Use PlantUML, Mermaid or dbdiagram.io. The only UML-like output we consider is a generated, read-only sequence view of a counterexample; database tables appear at most as an attribute of Database elements for the scan **[decided]**.
- Composition across levels; v1 combines siblings in one view only (ADR 0002).
- Code import, conformance and code proofs (later stage, `docs/research/theorem-proving.md`).
- Official AWS, Azure or GCP icons (ADR 0007).
- Being a general Petri net tool for researchers; competing with TAPAAL on engine power.
- Presentation polish for stakeholders; documentation sites or image export of views beyond what dogfooding needs.
- Portfolio polish before the v1 gate is met **[decided: second priority]**.

## Open questions

- Decided 2026-11-02: the north star, its measurement and the v1 gate are accepted by the product owner. The journal is a private file. From slice 1 the app can write a local check log (timestamps, verdict, hash, budget used) to make I1 and I2 automatic; local file only, off by default.
