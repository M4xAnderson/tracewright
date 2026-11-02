# Problems we want to solve, ranked

As of 2026-11-02. Ranked by contribution to the north star (`north-star.md`) and the v1 success criterion: find a real deadlock or overflow in a design the user drew, with zero wrong Proved. Tags: **[evidence]** links a source; **[assumption]** names the test that would confirm it (`opportunity-solution-tree.md`).

Format: problem statement · evidence · how we'd know it is solved.

## 1. Deadlocks and overflows between components are found too late

The solo designer needs a way to know, before writing code, whether components that each look correct can deadlock or overflow a queue or pool when combined, because these defects only show up under rare interleavings, which today costs incidents, load-test time and redesign after the code exists.

- **[evidence]** AWS engineers found subtle design bugs with TLA+ that other techniques missed ([Newcombe et al., CACM 2015](https://www.amazon.science/publications/how-amazon-web-services-uses-formal-methods)). The P language exists because of the same class of bugs ([P docs](https://p-org.github.io/P/whatisP/)).
- **[assumption A1, value]** The product owner's own designs contain such defects, and they are expressible as Petri nets with capacities and bag queues, without time or message order. Test: E5 (paper replay of past incidents).
- **Solved when:** the first confirmed Disproved on a real design changes a decision (v1 gate), and the north star reaches ≥ 4 design questions answered per month.

## 2. Checking behavior needs a second artifact nobody keeps in sync

The solo designer needs behavior checking attached to the architecture they already draw, because TLA+, Quint, P, FizzBee and Petri net tools all need a separate text spec or a hand-built whole-system net, which today means checking is skipped entirely.

- **[evidence]** None of the spec or Petri net tools in `landscape.md` §2–3 reads an architecture model; none of the architecture tools in §1 executes behavior.
- **[assumption A2, usability]** Modeling one element's behavior with ports takes minutes, not hours, and composing siblings through relationships matches how the user thinks. Test: E6 (time to model one real design).
- **Solved when:** I1 time to first verdict ≤ 45 min for about 10 elements, and the user does not fall back to a separate spec for v1-expressible questions (journal notes).

## 3. Results are untrustworthy or unreadable

The solo designer needs every verdict to say exactly what it covers and every counterexample to read in architecture terms, because bounded results shown as success and state dumps in net terms either mislead or get ignored.

- **[evidence]** Apalache: "bounded model checking is an incomplete technique" ([docs](https://apalache-mc.org/docs/apalache/running.html)); P's default checker is systematic testing, not proof ([P docs](https://p-org.github.io/P/whatisP/)); Petri tools report firing sequences of places and transitions (`docs/research/verification.md` §6).
- **[decided]** A wrong Proved must never happen; Unknown always carries a reason.
- **[assumption A3, usability]** Replay on the canvas with highlighted elements is enough to find the root cause without reading the net. Test: E7.
- **[assumption A4, feasibility]** An in-house checker can be correct enough for zero wrong Proved. Test: E1 differential corpus.
- **Solved when:** zero wrong Proved and zero wrong Disproved in the corpus and in use; I3 ≥ 80 % of counterexamples understood from the replay alone.

## 4. Checks are too slow or too often Unknown to use while designing

The solo designer needs answers in seconds while editing, because a check that takes minutes or ends Unknown breaks the design loop and the tool stops being used.

- **[evidence]** State-space explosion is the main limit of explicit exploration (`docs/research/verification.md` §2); budgets end checks as Unknown by design (`GLOSSARY.md`).
- **[assumption A5, feasibility]** Real v1 designs (siblings in one view, capacities everywhere) stay within a 2 s budget. Test: E1 state-space spike, then I2 on real designs.
- **Solved when:** I2 decisive rate ≥ 80 % on real designs, interactive checks within budget.

## 5. Structural anti-patterns slip through when there is no behavior model yet

The solo designer needs a quick scan for cycles, synchronous call cycles, missing timeouts and shared databases, because most elements will never get a behavior model and review alone misses these.

- **[evidence]** Graph-detectable anti-patterns are well catalogued; LikeC4 validates syntax only and Structurizr inspections check completeness (`docs/research/antipatterns.md` §0, §4).
- **[assumption A6, value]** Findings on the user's designs are mostly real. Test: E8.
- **Solved when:** ≤ 20 % of findings are suppressed as "not real", and at least one finding changes a design in the first month of scanning.

## 6. Design files are locked in or unreadable in git (supporting)

The solo designer needs models as readable, mergeable text in their own repo, because SaaS tools and machine JSON layouts make history and branches useless.

- **[evidence]** IcePanel is SaaS only ([pricing](https://icepanel.io/pricing)); Structurizr's layout JSON is "not designed to be edited by hand" (`docs/research/stack.md` §4.1).
- **[assumption A7, feasibility]** The TOML directory format diffs and merges cleanly. Test: E2 TOML/merge spike.
- **Solved when:** a three-way merge of two branches that touch different elements has no conflict, and a layout change touches only layout files.

## Parked (not v1)

- **Design and code drift apart** (conformance). Real, but needs code import first; later stage per `CLAUDE.md`.
- **Data-rich protocols, message order, time, liveness.** Real for some designs; covered by TLA+, Quint, P. Revisit if E5 shows the user's real defects need them (A1 fails).
