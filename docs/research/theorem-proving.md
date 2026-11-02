# Code-level proofs and interactive theorem proving: feasibility research

> Terms and decisions made after this research (GLOSSARY.md, CLAUDE.md, docs/adr/) take precedence over this file. It is kept as the research record.

**As of 2026-11-02.** Scope: what it would take for the tool (Rust core, Tauri 2, React UI; C4 drill-down; behavior models checked push-button, see `verification.md`) to support (a) full functional-correctness proofs of Rust source code and (b) interactive theorem proving (Lean/Rocq). Every claim links to a primary source where one could be found. Claims marked **[unverified]** could not be confirmed against a primary source during this research.

---

## 0. TL;DR

- Rust verification is real and in production in 2026, but split into three classes. **Deductive auto-active verifiers** (Verus, Creusot, Prusti) prove full functional correctness with SMT and annotations written inline in Rust. **Extraction to a proof assistant** (Aeneas→Lean, hax→F*/Lean/Rocq, rocq-of-rust, RefinedRust) gives the strongest guarantees for the most effort. **Bounded/lightweight checkers** (Kani, ESBMC, Flux, Thrust) are cheap and push-button, but their guarantees are bounded or type-level.
- Proof effort is still expensive. Published ratios run from about **3:1 to 14:1** lines of proof+spec per line of verified code (Verus SOSP'24 ~5:1, SymCrypt/Aeneas ~14:1), and the work has needed experts. LLM agents now close a large share of *proofs* when a human supplies the *specs* (VeruSAGE: >80% of 849 real system proof tasks; SymCrypt: agents wrote the proofs). Specs remain the bottleneck and the trust anchor.
- **Recommendation:** v1 should use **Kani** for push-button, bounded "code honors the model" checks, which fits the existing Proved/Disproved/Unknown verdict model. v2 should use **Verus** for opt-in full functional correctness of selected Code-level elements, with Verus `state_machine!` as the bridge from Petri nets and state machines to code. Interactive Lean (via Aeneas) comes later and only as an "expert mode". Rocq, Isabelle and F* should not be embedded.

---

## 1. Landscape of Rust code verification (late 2026)

### 1.1 Overview table

| Tool | Class / guarantee | Backend | Rust subset (unsafe / traits / closures / async / interior mut.) | Automation | License | Activity (verified 2026-11-02) |
|---|---|---|---|---|---|---|
| **Verus** | Deductive, **full functional correctness**, incl. concurrency | Z3 (SMT) ([guide](https://verus-lang.github.io/verus/guide/)) | unsafe blocks yes; raw pointers partial; traits yes; closures partial (no mutable captures); **async no**; Cell/RefCell/atomics via `vstd` alternatives ([feature table](https://verus-lang.github.io/verus/guide/features.html)) | Auto-active: inline `requires/ensures`, proof fns, ghost/tracked code | MIT (GitHub API) | Rolling release tags daily, latest `release/0.2026.11.02.8bf68ee` ([repo](https://github.com/verus-lang/verus)); ~3.5k stars |
| **Creusot** | Deductive, **full functional correctness** (safe Rust) | Why3 / Coma, Why3find; solvers via Why3 ([repo](https://github.com/creusot-rs/creusot)) | Safe Rust. Unsafe is handled via Gillian-Rust in research ([PLDI'25](https://arxiv.org/abs/2403.15122)). Atomics/resource algebras/ghost code added in 0.13–0.14 ([CHANGELOG](https://raw.githubusercontent.com/creusot-rs/creusot/master/CHANGELOG.md)) | Auto-active (Pearlite specs, prophecies) | LGPL-2.1 | v0.14.0 on 2026-10-12, v0.13.0 on 2026-07-30 (GitHub API) |
| **Prusti** | Deductive (Viper), panics/overflow + contracts | Viper ([repo](https://github.com/viperproject/prusti-dev)) | Not documented on README | Auto-active; VS Code "Prusti Assistant" | MPL-2.0 | Self-described "prototype"; tags as recent as `v-2026-10-30` (git tags) |
| **Kani** | **Bounded** model checking; unbounded via contracts | CBMC ([paper](https://arxiv.org/abs/2607.01504)) | unsafe yes, with UB checks; trait objects/closure types partial; **concurrency out of scope, await no**; no stack unwinding ([feature support](https://model-checking.github.io/kani/rust-feature-support.html)) | Push-button harnesses; function/loop contracts experimental ([contracts](https://model-checking.github.io/kani/reference/experimental/contracts.html), [loop contracts](https://model-checking.github.io/kani/reference/experimental/loop-contracts.html)) | MIT OR Apache-2.0 ([README](https://github.com/model-checking/kani)) | v0.68.0 on 2026-09-16 (GitHub API); ASE 2026 paper |
| **Aeneas** (+ **Charon**) | **Functional translation to a proof assistant** → full functional correctness in Lean | Lean (main), HOL4, F*, Coq ([repo](https://github.com/AeneasVerif/aeneas)) | **Safe Rust only**; unsafe and concurrency excluded until separation-logic work lands; some loop/`&mut`-in-generic limitations ([README](https://github.com/AeneasVerif/aeneas)) | Interactive (Lean tactics + custom automation); AI agents in SymCrypt ([arXiv 2609.15648](https://arxiv.org/abs/2609.15648)) | Apache-2.0 | Nightly tags daily (Aeneas `nightly-2026.11.02`, Charon `nightly-2026.11.01`) |
| **hax** | Translation to F*/Lean/Rocq/ProVerif/SSProve | F* stable; Lean via Aeneas recommended; Rocq, ProVerif, SSProve experimental ([repo](https://github.com/cryspen/hax)) | "Large subset"; forbids `&mut` in return types and aliasing; **Windows not supported** | Interactive / SMT (F*) | Apache-2.0 | Tags `hax-rust-engine-v0.4.2` |
| **RefinedRust** | Refinement type system in Rocq/Iris, **safe + unsafe**, foundational proofs | Rocq + Iris ([PLDI'24](https://iris-project.org/pdfs/2024-pldi-refinedrust.pdf)) | Verified a `Vec` variant with unsafe pointer code; follow-up at OOPSLA'26 ([PDF](https://iris-project.org/pdfs/2026-oopsla-refinedrust2.pdf)) | Semi-automated (Lithium) | **[unverified]** | Research prototype ([project](https://plv.mpi-sws.org/refinedrust/)) |
| **Gillian-Rust** | Semi-automated separation-logic verification of **unsafe** Rust, linked to Creusot | Gillian ([PLDI'25](https://arxiv.org/abs/2403.15122)) | Targets unsafe std code | Semi-automated | **[unverified]** (Gillian core is BSD-3) | Proof of concept |
| **Flux** | **Refinement types** (liquid types) | Fixpoint/Z3 **[unverified for 2026]** ([PLDI'23](https://pldi23.sigplan.org/details/pldi-2023-pldi/64/Flux-Liquid-Types-for-Rust)) | Used to verify process isolation in Tock OS ([talk](https://events.ucsc.edu/event/cse-colloquium-flux-refinement-types-for-verified-rust-systems/)) | High: liquid inference synthesizes loop invariants | MIT | Active, ~930 stars, pushed 2026-11-02 |
| **Thrust** | Prophecy-based **refinement types**, CHC-solved | Z3 CHC default ([repo](https://github.com/coord-e/thrust), [PLDI'25](https://pldi25.sigplan.org/details/pldi-2025-papers/84/Thrust-A-Prophecy-Based-Refinement-Type-System-for-Rust)) | No external crates yet; pinned nightly-2025-09-08 | Very high (inference, few annotations) | MIT OR Apache-2.0 | Research, 26 stars |
| **rocq-of-rust** | Translation of THIR to Rocq, then interactive proof | Rocq ([repo](https://github.com/formal-land/rocq-of-rust)) | Traits handled via linking; unsafe not documented | Interactive, expert | Transpiler **AGPL-3.0**; Rocq libs MIT | "Work in progress"; Revm case study |
| **ESBMC (Rust)** | **Bounded** model checking + k-induction | ESBMC; translation reuses Kani ([Rust Foundation](https://rustfoundation.org/media/expanding-the-rust-formal-verification-ecosystem-welcoming-esbmc/)) | Safe and unsafe safety properties ([survey](https://arxiv.org/pdf/2605.26169)) | Push-button | Apache-2.0 **[unverified]** | v8.6 tag; accepted for verify-rust-std |

### 1.2 Classification by guarantee

| Guarantee | Tools | Notes |
|---|---|---|
| **Full functional correctness, unbounded, SMT-automated** | Verus, Creusot, Prusti | Specs and proofs live inline in Rust. The trusted base includes the SMT solver and the tool's encoding. |
| **Full functional correctness, foundational / proof-assistant** | Aeneas→Lean, hax→F*/Lean, RefinedRust (Rocq/Iris), rocq-of-rust | The strongest guarantees: Lean/Rocq kernels check the proofs. They are also the most expensive, and the generated models are hard for non-experts to read. |
| **Refinement types (lightweight, type-level)** | Flux, Thrust | Good for index safety, ranges, simple invariants. Not suited to rich behavioral specs. |
| **Bounded / bug-finding (unbounded only with contracts)** | Kani, ESBMC | Push-button, concrete counterexamples. Kani "extends bounded model checking beyond bug-finding" via contracts ([arXiv 2607.01504](https://arxiv.org/abs/2607.01504)). |

### 1.3 Notes that matter for this project

- **Verus is the most systems-oriented option.** It explicitly targets concurrent/systems code. AWS used it for primitives of the Nitro Isolation Engine, including unsafe blocks and custom locking schemes ([Amazon Science, 2026-08-31](https://amazon.science/blog/developing-provably-correct-rust-code-with-verus)). Verus feedback "typically arrives in under a second" per that article. Verus also states that it supports "a subset of Rust" and is "under active development. Features may be broken" ([repo](https://github.com/verus-lang/verus)). **No async** ([features](https://verus-lang.github.io/verus/guide/features.html)).
- **Verus has native state machines.** `state_machine!` and `tokenized_state_machine!` (VerusSync) describe transition systems with invariants and link them to executable code via ghost "tokens" ([state-machines guide](https://verus-lang.github.io/verus/state_machines/), [SOSP'24 paper](https://par.nsf.gov//servlets/purl/10630619), [OSDI'23 sharding](https://www.usenix.org/conference/osdi23/presentation/hance)). This is the most direct bridge from our Petri-net/state-machine models to code (see §3).
- **Kani has the best UX for counterexamples.** Concrete playback generates "a Rust unit test case that plays back a failing proof harness using a concrete counterexample" ([docs](https://model-checking.github.io/kani/reference/experimental/concrete-playback.html)). More than 16,000 harnesses run per code change in the std verification campaign ([arXiv 2607.01504](https://arxiv.org/abs/2607.01504)). Concurrency is out of scope.
- **Aeneas is production-proven.** Microsoft's SymCrypt has complete Lean proofs for Rust ML-KEM and SHA-3 used in Windows insider builds ([MSR blog](https://microsoft.com/en-us/research/blog/verifying-rust-cryptography-in-symcrypt-from-standards-to-code)). It covers safe Rust only. Users report toolchain drift across Lean versions and extraction gaps for generics and external crates ([arXiv 2605.30106 summary, unverified](https://www.emergentmind.com/papers/2605.30106)).
- **The ecosystem anchor is the Rust std verification contest.** It is tool-agnostic. Kani, ESBMC (GOTO Transcoder), VeriFast and Flux appear as accepted tools in a fork's README ([fork](https://github.com/flux-rs/verify-rust-std); [official repo](https://github.com/model-checking/verify-rust-std)). The official accepted-tools list was **[unverified]**.

---

## 2. Interactive provers and how a desktop app could drive them

### 2.1 Prover status

| Prover | Status 2026 | Machine interfaces | Goal display | Toolchain management |
|---|---|---|---|---|
| **Lean 4** | Stable v4.35.0, v4.36.0-rc2 (git tags). The Lean FRO now works in 6-month roadmaps (Y4-1: Sep 2026–Feb 2027): `grind`/`bv_decide` automation, a VC generator, `lake check` kernel re-checking, a code formatter, lean-beam as agent interface ([roadmap](https://lean-lang.org/fro/roadmap/y4-1)) | LSP via Lake server, with Lean-specific `$/lean/plainGoal`, `$/lean/plainTermGoal` and RPC `getInteractiveGoals` ([source](https://github.com/leanprover/lean4/tree/master/src/Lean/Server)); JSON REPL ([leanprover-community/repl](https://github.com/leanprover-community/repl)); Pantograph ([repo](https://github.com/lenianiva/Pantograph)); lean-beam (LSP extensions + MCP; "experimental beta", not an official FRO product) ([repo](https://github.com/ejgallego/lean-beam)) | `@leanprover/infoview`: a React app that can be hosted by any LSP-capable host with a webview, "not tested outside VS Code" ([lean4-infoview](https://github.com/leanprover/vscode-lean4/tree/master/lean4-infoview)) | `elan` + per-project `lean-toolchain` ([elan](https://github.com/leanprover/elan)); Lake build system ([Lake docs](https://lean-lang.org/doc/reference/latest/Build-Tools-and-Distribution/Lake/)) |
| **Rocq** (ex-Coq) | Renamed at 9.0 (2025-03-12) ([release](https://rocq-prover.org/releases/9.0.0)); latest tag V9.3.0 | rocq-lsp/coq-lsp (Flèche engine, incremental), **petanque** low-latency API for AI tools; it is "in the process of replacing" SerAPI ([repo](https://github.com/ejgallego/coq-lsp)) | Goals, shelved goals and bullets via LSP extensions | opam. README lists Rocq 9.0–9.2 (9.3 support **[unverified]**) |
| **Isabelle** | Mature (seL4) | TCP server with JSON messages ([system manual](https://isabelle.in.tum.de/library/Doc/System/Server.html)); Isabelle/VSCode LSP | Via PIDE/LSP | Self-contained distribution (large; JVM/Scala) |
| **F*** | Active (hax's stable backend) | `fstar.exe --ide` JSON-lines protocol. LSP only via the `fstar-vscode-assistant` TypeScript wrapper; the built-in LSP is "completely unmaintained" ([discussion](https://github.com/FStarLang/FStar/discussions/4194), [assistant](https://github.com/FStarLang/fstar-vscode-assistant)) | Through the wrapper | opam/binaries + Z3 pin |

### 2.2 Embedding options for a Tauri app

| Approach | How | Pros | Cons |
|---|---|---|---|
| **A. LSP client in the Rust core + infoview in the webview** (Lean) | Spawn `lake serve` per project, relay LSP JSON-RPC to the React side, host `@leanprover/infoview` (it needs only an LSP relay and `postMessage`) | Same goal UX as VS Code; incremental elaboration for free | Infoview is untested outside VS Code. We must implement a host-side "EditorApi" shim (**[unverified]** surface size) |
| **B. REPL/Pantograph for headless obligations** | JSON over stdin/stdout; env pickling to `.olean` | Best for batch re-checking, AI loops, "is this obligation closed?" | Must match the Lean toolchain exactly. Tactic mode is "experimental" ([repl](https://github.com/leanprover-community/repl)) |
| **C. "Open in VS Code"** | Generate a Lake/Cargo project and launch the user's editor with Lean/Verus/rust-analyzer extensions | Near-zero cost; uses mature tooling (verus-analyzer, vscode-lean4) | Leaves our canvas; status must be re-imported from CLI JSON |
| **D. rocq-lsp/petanque** | Same as A/B for Rocq | Good protocol design | A second ecosystem to maintain |

For Verus/Creusot/Kani **no prover UI is needed**: they are CLI tools with machine-readable output (Verus has `--output-json`, `--verify-module`, `--verify-function`, `--expand-errors` flags in [config.rs](https://raw.githubusercontent.com/verus-lang/verus/main/source/rust_verify/src/config.rs); Creusot runs `cargo creusot` and can open the Why3 IDE on failure ([guide](https://guide.creusot.rs/))). IDE support for Verus is `verus-analyzer`, a rust-analyzer fork marked "experimental" ([repo](https://github.com/verus-lang/verus-analyzer/)).

**Toolchain management reality.** Each tool pins a Rust nightly: Creusot 0.14 uses nightly 2026-08-03 ([CHANGELOG](https://raw.githubusercontent.com/creusot-rs/creusot/master/CHANGELOG.md)); Thrust uses nightly-2025-09-08 ([repo](https://github.com/coord-e/thrust)). Verus ships a pinned toolchain plus Z3 (**[unverified]** exact mechanism). Aeneas needs OCaml 5 plus Lean ([README](https://github.com/AeneasVerif/aeneas)). Lean projects pin through `lean-toolchain`. A toolchain manager therefore has to (1) install per-tool bundles side by side, (2) never touch the user's default `rustup` toolchain, and (3) record exact versions in each proof artifact for reproducibility. hax does not run on Windows ([repo](https://github.com/cryspen/hax)), which matters for a desktop app.

---

## 3. Specification languages and linking model ↔ code

### 3.1 How users write specs per tool

| Tool | Pre/post | Loop invariants / termination | Ghost / logic | Where it lives |
|---|---|---|---|---|
| Verus | `requires`, `ensures` (Rust syntax, macro-extended); `forall`/`exists`; `int`/`nat` math types ([guide](https://verus-lang.github.io/verus/guide/)) | `invariant`, `decreases` | `spec fn`, `proof fn`, `ghost`/`tracked` variables; state machines | Inline in `.rs` inside `verus!{}` |
| Creusot | `#[requires(x@ < i64::MAX@)]`, `#[ensures(result@ == x@ + 1)]` ([guide](https://guide.creusot.rs/)) | `#[invariant]`, `#[variant]` | Pearlite logic fns, `ghost!`, snapshots, prophecy `^` | Attributes in `.rs` |
| Prusti | `#[requires]`, `#[ensures]`, loop `body_invariant!` ([repo](https://github.com/viperproject/prusti-dev)) | yes | pure fns | Attributes in `.rs` |
| Kani | `#[kani::requires]`, `#[kani::ensures]`, `#[kani::proof]` harnesses with `kani::any()` ([contracts](https://model-checking.github.io/kani/reference/experimental/contracts.html)) | loop contracts (experimental) | stubs | Attributes + harness fns |
| Flux / Thrust | Refinement types on signatures; Thrust also `requires/ensures` ([Thrust](https://github.com/coord-e/thrust)) | inferred | – | Attributes |
| Aeneas | Lean theorems about the extracted functions, which return `Result` (functional translation, [ICFP'22](https://arxiv.org/abs/2206.07185)) | Lean proofs | Full Lean | Separate `.lean` files |
| hax | F*/Lean specs; Rust-side annotations via hax-lib **[unverified detail]** | prover-side | prover-side | Mixed |

**Implication for us.** The auto-active tools (Verus, Creusot, Kani, Prusti) keep specs **in the Rust source as attributes or macros**. That allows round-tripping: the Code-level canvas element shows the contract and the source file holds it. Our model stores the spec, generates the annotations, and parses them back on import. Aeneas/hax specs live in a separate prover language, so round-tripping means managing two artifacts.

### 3.2 Attaching specs to C4 levels

| C4 level | What a "spec" is | Suggested backing |
|---|---|---|
| System Context / Container | Temporal properties of interaction protocols (safety/liveness) | Petri nets / state machines + model checker (`verification.md`) |
| Component | Behavior model + interface contracts (allowed call sequences, invariants on component state) | State machine model, exported as Verus `state_machine!` spec or Kani harness oracle |
| Code | Function contracts, type invariants, loop invariants | Verus/Creusot/Kani annotations |

**Refinement link.** Each Code element gets a "refines" edge to a Component behavior model. The obligation reads: *the implementation's abstract state, under an abstraction function `α(code_state)`, only takes transitions allowed by the model*. That is exactly the shape of:

- **TLA+ refinement mappings.** Abadi & Lamport show that implementation correctness reduces to a state function mapping concrete to abstract states ([The Existence of Refinement Mappings](https://lamport.azurewebsites.net/pubs/abadi-existence.pdf)).
- **Event-B/Rodin.** Stepwise refinement with gluing invariants and proof obligations per refinement step. Rodin code generators target C/C++/Java/C#/Ada ([EB2ALL, Tasking Event-B](https://wiki.event-b.org/index.php/Code_Generation)). **No Rust target was found.**
- **IronFleet.** A layered high-level spec, then a distributed protocol, then a Dafny implementation, with TLA-style reasoning for liveness ([MSR](https://www.microsoft.com/en-us/research/publication/ironfleet-proving-practical-distributed-systems-correct/)).
- **Verdi.** Verified system transformers in Coq that lift a proof under an idealized network to a faulty one; first mechanized linearizability proof of Raft ([PLDI'15](https://homes.cs.washington.edu/~mernst/pubs/verify-distsystem-pldi2015-abstract.html)).
- **Anvil (OSDI'24, Verus).** A TLA-style state-machine model of a Kubernetes cluster plus a temporal-logic library on top of Verus. Three controllers are proved live (eventually stable reconciliation); a full run takes about 12 min ([USENIX](https://www.usenix.org/conference/osdi24/presentation/sun-xudong), [repo](https://github.com/vmware-research/verifiable-controllers)). This is the closest existing precedent for "state-machine model ↔ Rust code" proofs.

**Practical refinement path in this tool:**
1. **Model → spec generation.** Compile a (bounded, 1-safe or k-bounded) Petri net or state machine to a Verus `state_machine!` (states = markings/enum, transitions = actions with guards). Invariants the model checker already proved can be emitted as `#[invariant]` candidates. Their inductiveness is then re-proved in Verus, which is cheap when the model checker supplies a strengthening invariant.
2. **Code obligation.** The user annotates (or we scaffold) the abstraction function and per-method `ensures` clauses of the form "the abstract step is a model transition".
3. **Push-button cheap variant (v1).** Generate a **Kani harness** that drives the code with `kani::any()` sequences up to bound *k* and asserts that each step's `α`-image is an allowed model transition. The verdict is Unknown with the reason "no counterexample up to depth k", which is honest about the bound.

**Illustrative generated artifacts.** These are sketches of what our codegen would emit for a two-state "Door" model. The syntax follows the cited docs but was **not compiled** during this research.

```rust
// v1: Kani bounded refinement harness (Kani attributes per Kani docs)
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(9)]
fn door_refines_model() {
    let mut d = Door::new();
    for _ in 0..8 {
        let ev: Event = kani::any();          // nondeterministic environment
        let before = alpha(&d);               // abstraction function (user/scaffolded)
        d.handle(ev);
        assert!(model::step_allowed(before, ev, alpha(&d))); // generated from the model
    }
}
```

```rust
// v2: Verus state machine generated from the model (sketch; see Verus state-machine guide)
verus! {
state_machine!{ DoorModel {
    fields { pub open: bool, pub locked: bool }
    init!{ initialize() { init open = false; init locked = false; } }
    transition!{ open_door() { require !pre.locked; update open = true; } }
    transition!{ lock() { require !pre.open; update locked = true; } }
    #[invariant] pub fn safe(&self) -> bool { !(self.open && self.locked) }
    // inductiveness lemmas: emitted per transition, discharged by Z3
}}
}
```

The exec code then carries `ensures DoorModel::State::open_door(alpha(old(self)), alpha(self))`-style postconditions (exact form **[unverified]**). That is the per-method refinement obligation.

Caveat: unbounded Petri nets and liveness (fairness) properties do not map cleanly to SMT-based code proofs. Anvil had to build its own temporal-logic layer on top of Verus ([paper](https://par.nsf.gov//servlets/purl/10599460)).

---

## 4. AI-assisted proving: evidence vs hype

### 4.1 Math theorem proving (Lean)

| System | Result | Openness / locality | Source |
|---|---|---|---|
| AlphaProof (DeepMind) | IMO 2024: 3 of 5 non-geometry problems (silver-level with AlphaGeometry); Nature 2025. Bottlenecks on new definitions outside Mathlib | Not released | [Google Research](https://research.google/pubs/olympiad-level-formal-mathematical-reasoning-with-reinforcement-learning/), [DeepMind blog](https://deepmind.google/discover/blog/ai-achieves-silver-medal-standard-solving-international-mathematical-olympiad-problems/) |
| Harmonic Aristotle | IMO 2025: 5/6 with Lean-checked proofs (problems formalized first) | Commercial app/API; API terms **[unverified]** | [arXiv 2510.01346](https://alphaxiv.org/abs/2510.01346v1), [Harmonic](https://harmonic.fun/news/imo-ios) |
| Seed-Prover 1.5 (ByteDance) | PutnamBench 581/672 at ~10 H20 GPU-days per problem (~$168/problem est.) | Closed | [via arXiv 2602.24273](https://arxiv.org/pdf/2602.24273) |
| Aleph Prover (Logical Intelligence) | PutnamBench 668/672 reported at ~$68/problem; aggregators disagree | Closed | [via arXiv 2602.24273](https://arxiv.org/pdf/2602.24273) **[secondary]** |
| DeepSeek-Prover-V2 (671B) | miniF2F 88.9% (pass@8192); PutnamBench 49/658 | Open weights | [arXiv 2504.21801](https://arxiv.org/abs/2504.21801v1) |
| Goedel-Prover-V2 (8B/32B) | miniF2F 32B 88.1% / 90.4% w/ self-correction (pass@32); 8B 84.6%; PutnamBench 86 (pass@184) | Open; **8B is locally runnable** | [arXiv 2508.03613](https://arxiv.org/abs/2508.03613), [repo](https://github.com/Goedel-LM/Goedel-Prover-V2) |
| Kimina-Prover | 72B 80.7% miniF2F (pass@8192); open distills 1.5B/7B/8B (8B: 77.86% pass@32) | Distills open | [arXiv 2504.11354](https://arxiv.org/abs/2504.11354), [8B card](https://featherless.ai/models/AI-MO/Kimina-Prover-Distill-8B/readme) |
| Lean Copilot / LeanDojo | Automates 74.2% of proof steps on Mathematics in Lean vs 40.1% for aesop; runs local models | Open, local | [arXiv 2404.12534](https://arxiv.org/abs/2404.12534), [repo](https://github.com/lean-dojo/LeanCopilot) |
| Mistral Leanstral 1.5 | 119B MoE (6.5B active), Apache-2.0; **API deprecated 2026-09-29** | Weights availability **[unverified]** | [model card](https://docs.mistral.ai/models/model-cards/leanstral-1-5) |

**Reading:** miniF2F is saturated (~90% with open 32B models). PutnamBench is near-saturated only for closed systems spending tens to hundreds of dollars per problem. None of these models was trained for *program* verification, and the competition-math numbers do not transfer to proving Rust code correct.

### 4.2 Program verification (the relevant evidence)

| Work | Setting | Result | Source |
|---|---|---|---|
| AutoVerus (OOPSLA'25) | 150 Verus proof tasks (spec given) | >90% proved; over half in <30 s or 3 LLM calls | [arXiv 2409.13082](https://arxiv.org/abs/2409.13082) |
| Vericoding benchmark | Off-the-shelf LLMs synthesize code+proof from formal spec | **Dafny ~82%, Verus ~44%, Lean ~27%** (task sets not directly comparable) | [arXiv 2509.22908](https://arxiv.org/abs/2509.22908) |
| RagVerus / RVBench | Repository-level Verus proofs | Tripled pass rate on existing benchmarks under constrained budgets; +27% relative on RVBench | [arXiv 2502.05344](https://arxiv.org/abs/2502.05344v1), [2509.25197](https://arxiv.org/html/2509.25197v1) |
| **VeruSAGE-Bench** (NeurIPS'26) | 849 proof tasks from 8 real Verus-verified systems (OS, storage, allocators) | Best agent+model >80%; avg **15.3 min and $13.65 per task**; hardest >1 h | [NeurIPS](https://neurips.cc/virtual/2026/poster/139178), [code](https://github.com/microsoft/verus-proof-synthesis) |
| **SymCrypt / Aeneas** (2026) | Production crypto, 16.7 KLOC Rust | Agents "autonomously write formal proofs" checked by the Lean kernel; standards/intrinsics formalization "still requires expert design and review" | [arXiv 2609.15648](https://arxiv.org/abs/2609.15648) |

**Honest assessment.**
- **Evidence:** given a *correct human-written specification*, frontier LLM agents can now close most proof obligations for Verus and Lean-extracted Rust, at minutes and dollars per obligation. The prover kernel or SMT check makes this safe: a wrong proof is rejected.
- **Hype:** "AI proves your code correct" hides that **the spec is the claim**. LLM-written specs can be vacuous or wrong, and no benchmark above measures spec adequacy. Math-olympiad results say little about program verification.
- **Local vs API.** Open local models (Goedel-Prover-V2-8B, Kimina distills, Lean Copilot's models) are math/Lean-tuned. No open local model with published Verus results was found **[gap]**. Every system-level Verus result uses frontier API models. A local-first tool should therefore treat AI proving as **opt-in, BYO API key**, with the prover as the only arbiter, and send only the obligation (spec + code slice), never the whole repo.

---

## 5. Effort and cost

| Project | Code verified | Proof / spec | Effort | Ratio (proof+spec : code) | Source |
|---|---|---|---|---|---|
| seL4 (Isabelle) | ~8,700 C (+600 asm) | ~200k lines Isabelle | ~25 person-years total, ~12 py for kernel proof | ~20:1 | [SOSP'09](https://cgi.cse.unsw.edu.au/~kleing/papers/sosp09.html), [numbers](https://trustworthy.systems/projects/OLD/l4.verified/numbers) |
| CompCert back-end (Coq) | compiler back-end | ~37k lines Coq (incl. code) | ~2 py | n/a (code inside Coq) | [Leroy 2009](https://arxiv.org/abs/0902.2137) |
| IronFleet (Dafny) | IronRSL + IronKV | – | 3.7 py | ~5:1 safety, ~8:1 liveness **[course slides, not paper]** | [MSR](https://www.microsoft.com/en-us/research/publication/ironfleet-proving-practical-distributed-systems-correct/), [slides](https://courses.grainger.illinois.edu/cs525/sp2016/IronFleet.discussion.pdf) |
| Verdi Raft (Coq) | ~1k impl (slides) | ~5k (linearizability) + ~45k (state machine safety) lines | – | very high | [CPP'16](https://homes.cs.washington.edu/%7Emernst/pubs/raft-proof-cpp2016.pdf) |
| Verus case studies (SOSP'24) | 6.1K impl | 31K proof | 3–61× faster than prior tools | ~5:1 | [MSR](https://www.microsoft.com/en-us/research/publication/verus-a-practical-foundation-for-systems-verification/) |
| Cedar (Lean + DRT) | Rust 15,693 LOC prod | Lean model 1,673 + proofs 5,714 | Validator soundness: 4,686 lines, **18 person-days** | Lean model ~10× smaller than Rust; proofs:model ~3.4:1 | [arXiv 2407.01688](https://arxiv.org/html/2407.01688v1), [AWS blog](https://aws.amazon.com/blogs/opensource/lean-into-verified-software-development/) |
| AWS authorization engine (Dafny) | Whole engine, compiled to Java | – | **4 years**; deployed 2024, ~3× faster | n/a | [Amazon Science](https://www.amazon.science/publications/formally-verified-cloud-scale-authorization) |
| SymCrypt (Aeneas→Lean) | 16.7 KLOC Rust | 237 KLOC Lean (models+specs+proofs+libs) | Agent-assisted | ~14:1 | [arXiv 2609.15648](https://arxiv.org/abs/2609.15648) |
| Kani in std / s2n-quic / Firecracker | harness-level | Harnesses | CI per change; 16k+ harnesses on std | low | [arXiv 2607.01504](https://arxiv.org/abs/2607.01504), [s2n-quic guide](https://aws.github.io/s2n-quic/dev-guide/kani.html), [AWS blog](https://aws.amazon.com/blogs/opensource/how-open-source-projects-are-using-kani-to-write-better-software-in-rust/) |
| CreuSAT (Creusot) | Verified CDCL SAT solver | – | Master's thesis (2022) | – | [thesis](https://mn.uio.no/ifi/english/research/groups/psy/completedmasters/2022/skotaam) |

s2n-tls/SAW proof figures were not researched here **[gap]**.

**What this means for a typical user of our tool:**
- **Full functional correctness costs about 3–20 lines of proof+spec per line of code** and has historically needed verification experts. AI agents lower the *proof* share but not the *spec* share.
- Cedar shows the pragmatic pattern: a **small executable model** (Lean ~10× smaller than Rust), proved properties on the model, and **differential testing** model ↔ code. It found 21 bugs via testing and 4 via proofs ([arXiv 2407.01688](https://arxiv.org/html/2407.01688v1)). This maps directly onto our "model-first" tool: our behavior models play the role of Cedar's Lean model.
- Realistic user targets: (a) push-button bounded checks everywhere they are cheap (Kani); (b) full proofs only for a few small, critical kernels (a protocol state machine, a lock, a parser core).

---

## 6. Integration architecture

### 6.1 What we would build

| Component | Responsibility | Build vs reuse |
|---|---|---|
| **Spec model** | Typed specs on Code elements: contracts, invariants, abstraction function, `refines` edge to a Component model | **Build** (our domain). Serialize to Verus/Kani/Creusot attribute syntax |
| **Spec editor** | Monaco/CodeMirror panel with Verus/Kani-flavored Rust highlighting; templates generated from the model | Build UI. Reuse rust-analyzer/verus-analyzer for diagnostics where possible |
| **Codegen / round-trip** | Emit `verus!{}` modules, `#[kani::proof]` harnesses, `state_machine!` from Petri nets/state machines; re-import edited annotations | Build. Use `syn` for parsing **[design choice]** |
| **Proof obligation (PO) tracker** | One PO per (element, property, tool). State ∈ {Proved, Unknown (no counterexample up to k), Disproved(cex), Unknown(timeout/unsupported), Stale} | Build. Same verdict vocabulary as `verification.md` |
| **Runner / sandbox** | Spawn tool CLIs with per-PO targeting (`--verify-function`, Kani `--harness`), timeouts, JSON output parsing | Build a thin layer; tools exist |
| **Cache / incremental re-check** | Key = hash(spec, code slice + transitive deps, tool+toolchain version, solver seed/flags) → result. Invalidate on dependency change | Build. Verus/Kani have function/harness-level targeting; Lean has `.olean` and REPL pickling |
| **Counterexample mapping** | Kani concrete playback → unit test → replay trace → highlight on the canvas (which transition/state the code violated); Verus failing `ensures`/assertion spans → element highlight | Build the mapping; reuse Kani playback ([docs](https://model-checking.github.io/kani/reference/experimental/concrete-playback.html)) |
| **Goal view** (only for the interactive tier) | Show Lean goals for an open obligation | Reuse `@leanprover/infoview` + LSP relay; or REPL/Pantograph for headless |
| **Toolchain manager** | Download/pin Verus release, Kani, Z3, (later) elan/Lean, Aeneas/Charon; verify checksums; per-project lockfile | Build. Wrap `elan`/`rustup` with private homes |
| **AI assist (opt-in)** | Proof-repair loop: obligation → LLM → tool check → retry; budget caps; never auto-accept specs | Build the orchestration; models via user's API key, or local Lean models |

### 6.2 Risks

| Risk | Severity | Mitigation |
|---|---|---|
| **Tool churn**: Verus tags daily, Creusot/Aeneas track nightlies; syntax changes ("syntax might not be updated", [verus-analyzer](https://github.com/verus-lang/verus-analyzer/)) | High | Pin versions per project; adapter layer per tool; CI against pinned releases |
| **Subset gaps**: no async in Verus or Kani; Aeneas safe-only | High for real Rust import | Mark unsupported code as `Unknown (unsupported feature)`; verify isolated pure cores |
| **Spec adequacy**: proving the wrong thing | High | Show specs prominently; vacuity checks (Kani `cover!`, satisfiable preconditions) **[design]**; model-derived specs |
| **Proof instability** (SMT brittleness) | Medium | Store solver versions/seeds; AWS documents "proof brittleness" as a real issue ([paper](https://www.amazon.science/publications/formally-verified-cloud-scale-authorization)) |
| **Install size/platforms**: OCaml+Lean+Mathlib are GBs; hax lacks Windows | Medium | Lazy, optional install; core ships only Kani/Verus |
| **License**: Creusot LGPL, rocq-of-rust transpiler AGPL | Low–Medium | Invoke as external processes; don't link |
| **Liveness/unbounded models** don't map to code proofs | Medium | Keep those at model level; code obligations are safety/refinement only |
| **UX expectation**: "full proof" sounds like one click | High | Tiered verdicts; label effort upfront |

---

## Feasibility verdict

| Horizon | Realistic scope |
|---|---|
| **v1** | **No interactive theorem proving.** Push-button **Kani** obligations generated from the model: panic/overflow/UB freedom of Code elements, plus **bounded refinement checks** (code steps ⊆ model transitions up to depth k) with concrete-playback counterexamples mapped to the canvas. Verdicts: Proved / Disproved / Unknown, where a clean bounded run is Unknown (no counterexample up to k). Specs are generated and visible but edited mainly as model properties. |
| **v2** | **Verus** as opt-in "full proof" tier for selected Code elements. Model → `state_machine!` generation; contract editor; PO tracker with caching; failing-obligation views; optional AI proof-repair loop (BYO key). Expect users to verify small kernels, not whole crates. |
| **Later (v3+)** | **Aeneas→Lean** "expert mode" with embedded infoview and REPL-driven obligations, for teams that want kernel-checked proofs (crypto/parsers). Import from existing Rust code with automatic contract extraction. Possibly Creusot as an alternative SMT backend for safe-only code. |
| **Not recommended** | Embedding Rocq, Isabelle or F* UIs; promising "full functional correctness of source code" in general; supporting async Rust proofs. |

**Decision matrix** (1 = poor, 5 = excellent; the author's judgment, based on the evidence above):

| Criterion | Kani | Verus | Creusot | Aeneas→Lean |
|---|---|---|---|---|
| Push-button for non-experts | 5 | 2 | 2 | 1 |
| Strength of guarantee | 2 (bounded) / 3 (contracts) | 4 | 4 | 5 (kernel-checked) |
| Rust subset coverage (unsafe, concurrency) | 4 / 1 | 3 / 4 | 2 / 2 | 1 / 1 |
| Fit with state-machine/Petri models | 3 (harness oracle) | 5 (`state_machine!`) | 2 | 3 (Lean models) |
| Counterexamples for the canvas | 5 (concrete playback) | 2 (failing spans) | 2 | 1 |
| Bundling cost / platforms | 4 | 4 | 2 (OCaml/Why3) | 1 (OCaml + Lean) |
| License friendliness | 5 | 5 | 3 (LGPL) | 5 |
| Industrial evidence | 5 (std, AWS) | 4 (AWS Nitro) | 2 | 4 (SymCrypt) |
| AI-proof evidence on real code | 1 | 5 (VeruSAGE) | 1 | 4 (SymCrypt) |
| Release stability | 4 | 2 (rolling) | 3 | 2 (nightly) |

**Trusted computing base, to state honestly in the UI:**
- **Kani:** CBMC + the MIR translation + the unwind bound.
- **Verus/Creusot:** the SMT solver(s), the VC encoding and vstd/creusot-std axioms.
- **Aeneas→Lean:** the Lean kernel + the Charon/Aeneas translation (not verified end to end) + Lean models of the std library.

In every case the rustc/LLVM compiler is trusted. Verus explicitly does not aim to verify it ([guide](https://verus-lang.github.io/verus/guide/)).

**Why these bets:**
- **Kani first.** It is the only option that is push-button with no proof skills needed, has concrete counterexamples (unit tests) that fit our "map back to canvas" goal, a permissive license (MIT/Apache), AWS backing and a regular release cadence (0.68.0 on 2026-09-16). Its bounded verdicts slot into the honest "Unknown/bounded" story we already tell for models.
- **Verus over Creusot for v2.** Verus is systems- and concurrency-oriented, with **built-in state machines** that match our behavior models. It has fast SMT feedback, an industrial adopter (AWS Nitro), an MIT license, and the strongest AI-proof evidence on real systems (VeruSAGE). Creusot is solid, but its Why3/OCaml stack is heavier to bundle, it is LGPL, and it is safe-only. Verus's weakness is churn and its Rust-subset gaps.
- **Aeneas→Lean later.** It gives the strongest guarantees and has the most momentum (SymCrypt, Lean FRO investment in agents and VC generation), but it needs Lean expertise, OCaml+Lean toolchains, safe-only Rust and a separate spec artifact. Its value grows as AI agents mature, so it should be revisited around v3.

---

## Recommendations for this project (staged)

1. **Now (design):** Define the **PO data model and verdict lattice** shared with `verification.md` (Proved / Disproved(trace) / Unknown(reason), each possibly Stale). Add a `refines` relation and an **abstraction-function slot** on Code elements to the metamodel even before any prover exists.
2. **v1 spike (2–3 weeks):** Generate a Kani harness from a toy state machine plus a hand-written Rust impl. Run it via CLI, parse the result, map the concrete-playback trace onto the canvas. Measure run time and install size.
3. **v1:** Ship the Kani runner with pinned versions, a cache keyed by content hashes, and per-harness invalidation. No spec language of our own beyond model properties.
4. **v2 spike:** Petri net / state machine → Verus `state_machine!` codegen; prove an invariant found by the model checker inductive in Verus; link one `exec` fn to a transition. Evaluate verus-analyzer reuse for the spec editor.
5. **v2:** Verus tier with spec editor, round-trip of annotations, an AI proof-repair loop (opt-in, BYO API key, budget caps, specs never auto-accepted), and an "effort estimate" banner.
6. **v3+:** Evaluate Aeneas→Lean with an embedded `@leanprover/infoview` in the Tauri webview (spike: an LSP relay from the Rust core), plus lean-beam/REPL for headless checks. Re-evaluate hax if Windows support arrives.
7. **Continuously:** Track Rust std verification contest tool acceptance and Verus/Kani release notes; keep adapters thin.

---

## Open questions

- Is there an **official, current list** of accepted tools in the verify-rust-std contest (Flux, ESBMC, VeriFast, Kani…)? Only fork READMEs were seen.
- How stable is the **Verus syntax and vstd API** release to release? Is there a semver or stable channel beyond rolling tags?
- Can `@leanprover/infoview` be hosted in a Tauri webview with acceptable effort (EditorApi surface, import maps)? It has only been "tested in VS Code".
- What fraction of a **real imported Rust codebase** falls inside the Verus/Kani subsets (async, `dyn`, `Mutex`, I/O are excluded or partial)? A measurement on a sample crate is needed.
- Can Kani's **concurrency gap** be covered by model-level checking alone for concurrent components, with Verus only where locks are custom?
- **Spec adequacy:** what vacuity/sanity checks can we automate (satisfiable preconditions, `cover!` reachability, mutation testing of specs)?
- Local AI: are there open models with **published Verus or Rust-verification results** that run on consumer GPUs? None was found.
- Licensing review for bundling Creusot (LGPL), Z3 (MIT), CBMC, Why3 (LGPL) inside a distributed desktop app.

## Items not verified

RefinedRust and Gillian-Rust licenses; Flux's current solver backend; ESBMC license; Verus toolchain bundling mechanism; Rocq 9.3 support in rocq-lsp; hax spec syntax details; Harmonic Aristotle API terms; Leanstral weights availability; IronFleet ratios (lecture slides only); Aleph Prover/Seed-Prover PutnamBench figures (secondary leaderboards); the arXiv 2605.30106 friction report (aggregator summary only); s2n-tls/SAW effort figures.
