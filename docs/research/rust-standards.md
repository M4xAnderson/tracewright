# Rust coding standards: sources and rationale

> Terms and decisions made after this research (GLOSSARY.md, CLAUDE.md, docs/adr/) take precedence over this file. It is kept as the research record.

As of 2026-11-02. Research input for `CODING_STANDARDS.md`. This file is not the rulebook. It inventories the sources, translates them into Rust idioms, resolves conflicts and ends with a proposed rule outline and open questions. Every claim links to its primary source. "Not verified" marks what I could not confirm.

**Settled resolution order** (user decision, CLAUDE.md): **TigerStyle > Google Rust Style Guide > other Rust guidelines (Rust API Guidelines, Microsoft Pragmatic Rust, Fuchsia, Clippy defaults) > Clean Code.** I found three places where a lower source should win anyway, because the higher one is Zig-specific or fights a compiler or Clippy convention. They are listed in section 4 as "exception".

Project context: a local, single-user desktop app. Rust core (model, Petri-net verification, anti-pattern scan), Tauri 2 shell, React/TS UI with a heavy canvas editor. No server, no accounts, no telemetry.

## 1. Source inventory

### 1.1 Google: yes, there is now a public Rust style guide

- **Google Rust Style Guide**: <https://google.github.io/styleguide/rustguide.html>. Source file `rustguide.md` in `google/styleguide`. The GitHub API shows 5 commits, the first on **2026-10-21** and the latest on 2026-10-24, all "Automated export from internal guides" ([commit history](https://github.com/google/styleguide/commits/gh-pages/rustguide.md)). The guide is about two weeks old. It has no version number. Scope: "Android, Chromium, and other codebases … intend to start applying it" ([Scope](https://google.github.io/styleguide/rustguide.html#scope)).
- It **builds on** the official [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/) and the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) "to the extent they don't conflict" ([Follow public Rust style rules](https://google.github.io/styleguide/rustguide.html#public-rules)). So the API Guidelines are part of the Google base by reference.
- Goals, in its words: readability over writeability, better automation (large-scale changes), consistency with the ecosystem, "rules should pull their weight", and avoiding minor disputes ([Goals](https://google.github.io/styleguide/rustguide.html#goals-of-the-style-guide)).
- **Library vs application code**: API design rules bind library code. For application code they are "a strong recommendation … favor simplicity and directness over broad reusability. Avoid premature generalization" ([api-design-context](https://google.github.io/styleguide/rustguide.html#api-design-context)). This matches raubritter's API-1..3 split and fits us: core crates are library code, `src-tauri` is application code.
- Tags `[chrome]` and `[Fuchsia]` mark rules that apply only to those codebases. We drop them (flat `rust_library` layout, `chromium::import!`, no async, no std threads, `fuchsia_sync::Mutex`, gtest interop).

**Full section inventory of the Google guide (non-tagged rules):**

| Section | Rules (short) |
|---|---|
| Code organization | name files and leaf modules after the primary item; no APIs in non-leaf modules (only `mod` and re-exports); **no glob imports** (exceptions: `use super::*` in inline test modules, `use Enum::*` locally, `pub use sub::*` in non-leaf modules), no preludes; one module per `use` statement (= rustfmt `imports_granularity = Module`); trait imports locally scoped; qualify free-function calls with their module (`mem::swap`); types imported and used unqualified (`Regex`, but `io::Error`) |
| Formatting | rustfmt with `max_width = 100`, `hex_literal_case = Lower`, `format_macro_matchers = true` (may be skipped on stable); escapes for invisible characters; `#[rustfmt::skip]` only for manual alignment or generated code, "not … for personal style choices" |
| Ownership | avoid `Rc`/`Arc` unless alternatives are "unreasonably complex"; prefer borrows over `.clone()` deep copies |
| Inputs/outputs | accept `&str`, `&[T]`, `Option<&T>`; libraries return iterators, apps may return `Vec`; `impl Into<T>` "almost never" in app code; **enum over bool** for non-obvious parameters |
| Reporting errors | `Option` for obvious failures; `Result` for recoverable, externally caused failures; **`panic!` for bugs and violated invariants**, which "must be fixed in the code, not handled at runtime"; codebases may forbid panics ([reporting-errors](https://google.github.io/styleguide/rustguide.html#reporting-errors)) |
| Types | newtypes for semantic differences and type safety; named fields over tuples |
| Traits | add impls when needed; implement std traits (`FromStr`) instead of custom methods; `#[derive]` where possible; libraries prefer generics over `dyn`; **"don't use traits merely to organize code"**; extension traits `~Ext`; `Default` for options structs; pair `From` with explicit constructors |
| Concurrency | never reach into a poisoned mutex |
| Macros | order of preference: plain Rust > `macro_rules!` > codegen > proc macros |
| Naming | `as_`/`to_`/`into_`; descriptive lifetimes; `new`/`try_new`/`from_foo`; **avoid abbreviations** |
| Documentation | doc comments on public APIs (`missing_docs`); document panics on malformed input; document unsafe invariants; no trivial comments; `///` items, `//!` modules, `//` implementation notes and TODOs |
| Other features | **unsafe only if no safe way**, isolated in a small crate, `SAFETY:` comment plus a written justification (benchmarks if for performance); no unstable features; getter/setter names (`foo`, `foo_mut`, `set_foo`, `take_foo`); `len` with `is_empty`; `#[non_exhaustive]` on cross-team public enums and structs; `#[must_use]` on functions whose result must not be ignored and on error types; no code before `main` |
| Implementation | don't overuse type inference (annotate `.into()`, `.parse()`, `collect`); pattern matching over check-then-unwrap; **`expect` over `unwrap`**; list all variants when matching `non_exhaustive` enums; **context strings in `assert!`**; avoid long iterator chains and `for_each`; **`panic::catch_unwind` is prohibited**; `format!` over `+` |
| Testing | per-codebase framework; tests may panic |

Not covered by Google: function length, Clippy configuration (it only names individual lints), assertions as a design tool, resource limits, dependencies.

### 1.2 Other Google material

| Source | What it is | Use for us |
|---|---|---|
| [Chromium Rust style](https://chromium.googlesource.com/chromium/src/+/main/styleguide/rust/rust.md) | Points to the Google guide, the official Rust Style Guide and the API Guidelines; formatting through `git cl format` | Confirms the Google guide is the base; nothing new |
| [Android Rust overview](https://source.android.com/docs/setup/build/rust/building-rust-modules/overview), [Android Rust patterns](https://source.android.com/docs/setup/build/rust/building-rust-modules/android-rust-patterns) | Build-system and interop how-tos (logging, AIDL, FFI, CXX) | No style, lint or unsafe policy found on these pages; nothing to carry over |
| [Fuchsia Rust API rubric](https://fuchsia.dev/fuchsia-src/development/api/rust) | Rust API Guidelines adapted for Fuchsia; three extra unsafe rules (`// SAFETY:` that justifies soundness, "Trust me" is not enough) | Supplement to the unsafe rules; examples use `?`, never `unwrap` |
| [Comprehensive Rust](https://google.github.io/comprehensive-rust/) | Course by the Android team; not a style guide | Onboarding only; the Google guide cites it for naming |

### 1.3 Microsoft Pragmatic Rust Guidelines

[microsoft.github.io/rust-guidelines](https://microsoft.github.io/rust-guidelines/), version 2026.6, last generated 2026-10-26, MIT ([repo](https://github.com/microsoft/rust-guidelines)). "Guidelines declared _must_ are supposed to always hold." It builds on the API Guidelines and Clippy. The [checklist](https://microsoft.github.io/rust-guidelines/guidelines/checklist/index.html) has **91 IDs** in the form `M-UPPER-KEBAB` in 12 chapters: Universal, Library (Interop, UX, Resilience, Building), Macros, Applications, FFI, Correctness, Performance, Project, Documentation, AI.

Relevant for us:
- [M-STATIC-VERIFICATION](https://microsoft.github.io/rust-guidelines/guidelines/universal/index.html): a concrete `[lints]` baseline (all default groups plus `pedantic` at warn, plus about 20 restriction lints such as `allow_attributes_without_reason`, `undocumented_unsafe_blocks`, `map_err_ignore`, `clone_on_ref_ptr`). Tools: rustfmt, cargo-audit, cargo-hack, cargo-udeps, miri. We reuse it in section 5.
- **M-LINT-OVERRIDE-EXPECT**: use `#[expect(lint, reason = "…")]` instead of `#[allow]`, because an expectation warns once it is stale.
- Correctness ([page](https://microsoft.github.io/rust-guidelines/guidelines/correctness/index.html)): **M-PANIC-IS-STOP** ("Panics are not exceptions"), **M-PANIC-ON-BUG** (detected bugs panic, input errors return `Result`), **M-PANIC-MESSAGE**, **M-UNSAFE** (valid reason, documented, Miri-tested), **M-UNSOUND** ("unsound code is never acceptable").
- **M-DOCUMENTED-MAGIC** (named constants that say why), **M-WEASEL-WORDS** (no `Manager`/`Service`/`Factory`), M-APP-ERROR (apps may use anyhow), M-LATEST-EDITION, M-CARGO-WORKSPACE.
- **AI chapter** ([page](https://microsoft.github.io/rust-guidelines/guidelines/ai/index.html)): M-DESIGN-FOR-AI, M-SINGLE-ITEM-PATH (one public path per item), M-NO-META-DESIGN-DOCUMENTATION (document the end state, not the design journey), **M-TAUTOLOGICAL-TESTS** (tests must not restate constants or mirror the implementation), M-RUST-SHAPED (no 1:1 ports of idioms from other languages). The [Agents & LLMs page](https://microsoft.github.io/rust-guidelines/agents/index.html) offers the whole guide as one file for agent sessions: [`all.txt`](https://microsoft.github.io/rust-guidelines/agents/all.txt), about 34k tokens.
- Not verified: the AI page did not show must/should levels for the five AI rules.

### 1.4 Official Rust sources

- **Rust API Guidelines** ([checklist](https://rust-lang.github.io/api-guidelines/checklist.html)): `C-*` IDs in 11 areas. Most relevant: C-CASE, C-CONV, C-GETTER, C-COMMON-TRAITS, C-GOOD-ERR, C-FAILURE (document errors, panics, safety), C-NEWTYPE, C-CUSTOM-TYPE (no `bool`/`Option` for meaning), C-VALIDATE, C-NO-OUT (no out-parameters), C-DEBUG.
- **Rust Style Guide** ([doc](https://doc.rust-lang.org/nightly/style-guide/)): what rustfmt implements. rustfmt options and their stability are in [Configurations.md](https://github.com/rust-lang/rustfmt/blob/main/Configurations.md).
- **Clippy groups** ([usage](https://doc.rust-lang.org/clippy/usage.html), [index](https://clippy.rust-lang.org/master/index.html), 852 lints on 2026-11-02): `correctness` deny; `suspicious`, `style`, `complexity`, `perf` warn; `pedantic`, `restriction`, `nursery`, `cargo` allow. On `pedantic`, the Clippy docs say it is "ready to be used in production, [but] you can expect to sprinkle multiple `#[allow(..)]`s". On `restriction`: "You shouldn't enable the whole lint group, but cherry-pick lints." Groups of the lints we use, checked in the index: `too_many_lines` pedantic (default threshold 100); `indexing_slicing`, `arithmetic_side_effects`, `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `dbg_macro`, `print_stdout`, `missing_assert_message`, `allow_attributes_without_reason`, `undocumented_unsafe_blocks`, `as_conversions`, `cognitive_complexity` restriction; `cast_possible_truncation`, `missing_panics_doc`, `missing_errors_doc`, `collapsible_else_if`, `fn_params_excessive_bools` pedantic; `too_many_arguments`, `excessive_nesting` complexity; `collapsible_if`, `upper_case_acronyms` style. **No Clippy lint bans recursion**: only `unconditional_recursion`, `only_used_in_recursion`, `self_only_used_in_recursion`, `main_recursion` and `recursive_format_impl` exist.
- **Unsafe**: the [Unsafe Code Guidelines](https://rust-lang.github.io/unsafe-code-guidelines/) say of themselves that they are "largely abandoned right now" and "mostly a 'recommendation'". They are not normative, so we cite them only as background, like the [Rustonomicon](https://doc.rust-lang.org/nomicon/). [Miri](https://github.com/rust-lang/miri) detects UB (out-of-bounds access, use-after-free, uninitialized data, data races, aliasing violations). It is nightly-only, has no FFI and no network, and "does not catch every violation".
- **Assertions in std**: `debug_assert!` runs only in non-optimized builds. "The performance cost of assertions, however, is not measurable in general. Replacing `assert!` with `debug_assert!` is thus only encouraged after thorough profiling, and more importantly, only in safe code!" ([std docs](https://doc.rust-lang.org/std/macro.debug_assert.html)). In Cargo profiles, `overflow-checks` and `debug-assertions` default to `true` in dev and `false` in release ([profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)).

## 2. TigerStyle → Rust

Source: [TIGER_STYLE.md](https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/TIGER_STYLE.md) (Zig; design goals in order: safety, performance, developer experience). Fit: **K** keep, **A** adapt, **D** drop for a GUI desktop app.

| TigerStyle rule (quoted or condensed) | Rust translation | Fit |
|---|---|---|
| "Zero technical debt … do it right the first time" | no `todo!`/`unimplemented!`/`dbg!` (Clippy deny); no `TODO` comments in merged code (review/grep); unfinished work becomes an issue | K |
| "Only very simple, explicit control flow" | no `catch_unwind` (also Google); no panics for control flow (M-PANIC-IS-STOP); macros as a last resort (Google) | K |
| "Do not use recursion" | graph traversals (reachability, C4 drill-down, cycle detection) use an explicit `Vec` stack or queue. No lint exists, so review enforces it. A parser that must recurse has a depth limit asserted at entry. Rust guarantees no tail calls, so deep recursion means stack overflow | K |
| "Minimum of excellent abstractions"; abstractions "never zero cost" | Google: no traits just to organize code; enums with `match` before trait objects for closed sets | K |
| "Put a limit on everything"; all loops and queues bounded; assert non-terminating loops | **central to a model checker**: reachability exploration has `max_states`, `max_depth` and a cancel flag, and returns `Unknown` when it hits a bound (never "proved"). Channels are bounded (`sync_channel(n)`, `tokio::sync::mpsc::channel(n)`); model files have size and count limits checked before allocation; an endless event loop is commented and `loop {}` asserted | **A** (central) |
| "Explicitly-sized types like `u32` … avoid `usize`" | IDs and counts in the domain and file format are newtypes over `u32`/`u64` (`PlaceId(u32)`); `usize` only for in-memory indexing; conversions via `try_from` (pedantic `cast_possible_truncation`) | A |
| Assertions: "assertion density … minimum of two assertions per function"; assert arguments, returns, invariants; "the only correct way to handle corrupt code is to crash" | in **core crates**: `assert!` for preconditions, postconditions and invariants (on in release; std says cost is generally not measurable); `debug_assert!` only for O(n) or expensive checks, after profiling; every assertion carries a message (`missing_assert_message`, Google, M-PANIC-MESSAGE). Density is not linted; review asks for it on core functions only, not on UI glue or Tauri commands | A |
| Pair assertions (before write, after read) | file format: validate the model before serializing **and** after deserializing; Petri-net invariants (token conservation under P-invariants) asserted after every firing in debug, checked by proptest | K |
| Split compound assertions; `if (a) assert(b)` | `assert!(a, "…"); assert!(b, "…");`; `if a { assert!(b, "…") }` | K |
| Assert compile-time constant relationships | `const _: () = assert!(MAX_STATES <= u32::MAX as usize);` (const panics stable since Rust 1.57; not re-checked in this session) | K |
| Assert positive and negative space; tests exhaustive with invalid data | property tests with valid **and** invalid models (proptest); fuzz the file parser (cargo-fuzz) | K |
| "Statically allocate all memory at startup … no dynamic allocation after initialization" | **does not fit**: models grow while the user edits. Keep the intent: `Vec::with_capacity` from known bounds, reuse buffers in hot loops (M-MEM-REUSE), no unbounded growth | D (keep intent) |
| Smallest scope, few variables in scope | shadowing for staged values; compute close to use; `let … else` | K |
| "Hard limit of 70 lines per function" | `clippy::too_many_lines` with `too-many-lines-threshold = 70` (Clippy default is 100). Not verified: whether Clippy counts blank and comment lines the way TigerStyle counts screen lines | K [auto] |
| Hourglass: few params, simple return; "push `if`s up and `for`s down"; parent owns state, leaves are pure | free functions over `&self` methods for pure leaves (M-REGULAR-FN); state changes only in the owning `struct`'s methods | K |
| "All compiler warnings at the strictest setting from day one" | `RUSTFLAGS=-D warnings` in CI; workspace lints (section 5) | K [auto] |
| "Don't act directly in reaction to external events … batch" | UI events become commands into the core; analysis runs debounced or batched, not on every mouse move | A |
| Split compound conditions into nested `if/else`; consider the `else`; state invariants positively | conflicts with Clippy `collapsible_if` (style) and `collapsible_else_if` (pedantic); TigerStyle wins, so both are `allow` | K |
| "All errors must be handled" (92 % of catastrophic failures, per the cited OSDI paper) | `Result` everywhere for operating errors; no `let _ =` on `Result` (`let_underscore_must_use`); no `.ok()` that discards an error (`unused_result_ok`, M baseline); no `map_err(\|_\| …)` (`map_err_ignore`) | K |
| "Always motivate, always say why" | comments and doc comments say why; magic numbers are named constants with a reason (M-DOCUMENTED-MAGIC) | K |
| Pass options to library calls explicitly, don't rely on defaults | explicit serde attributes on the file format, explicit `Ordering`, explicit capacities and timeouts | K |
| Performance: back-of-envelope sketches, batching, control vs data plane | the verification engine (state-space size is exponential) gets a sketch in its design doc; the UI does not | A |
| Naming: `snake_case`, no abbreviations, units last (`latency_ms_max`), nouns, helper prefix, callbacks last | Rust casing ([C-CASE](https://rust-lang.github.io/api-guidelines/naming.html)) is already snake/Camel; units and qualifiers last (`timeout_ms_max`, `states_max`); no abbreviations (also Google) | K |
| Acronym capitalization `VSRState` | **exception**: Rust uses `VsrState` (C-CASE, Clippy `upper_case_acronyms`) | D |
| Options struct when arguments can be mixed up ("a function taking two `u64` must use an options struct") | options struct with `Default` (Google), or newtypes so mix-ups don't compile | K |
| "Pass arguments over 16 bytes as `*const`"; in-place init via out pointer | **exception**: Zig-specific. Rust borrows `&T` anyway, and C-NO-OUT forbids out-parameters | D |
| Simpler return types: `void` > `bool` > `u64` > `?u64` > `!u64` | `()` > `bool` > value > `Option` > `Result`; don't return `Result` from infallible functions (`unnecessary_wraps`, pedantic) | K |
| Index, count and size as distinct; explicit division (`div_exact`, `div_ceil`) | `usize::div_ceil`, `checked_*`; `overflow-checks = true` in release | K |
| Descriptive commit messages | commit and PR template | K |
| `zig fmt`, 4 spaces, **100 columns hard** | rustfmt `max_width = 100` (default, also Google) | K [auto] |
| "Zero dependencies" | does not fit Tauri. Adapt: every new crate justified in the PR, cargo-deny bans and licenses, `default-features = false` where possible | A |
| Scripts in the primary language | `cargo xtask` instead of shell scripts (optional) | A |

## 3. Clean Code: what holds in Rust

Sources: Robert C. Martin, *Clean Code* (1st ed. 2008; [2nd ed. 2025](https://www.pearson.com/en-us/subject-catalog/p/clean-code-a-handbook-of-agile-software-craftsmanship-2nd-edition/P200000013239), a "comprehensive rewrite"). Not verified: I could not read the 2nd edition, so its rules are cited through the debate below, not quoted from the book.

**Still holds** (and is already covered by a higher source):
- Meaningful, intention-revealing names, no abbreviations (Google "avoid abbreviations", TigerStyle "get the nouns and verbs just right").
- Functions do one thing at one level of abstraction (TigerStyle hourglass, "push ifs up").
- DRY. Even Muratori calls it "reasonable advice" ([article](https://www.computerenhance.com/p/clean-code-horrible-performance)).
- Few arguments (TigerStyle options struct; Clippy `too_many_arguments`).
- "Don't return null" maps to `Option<T>`; "don't pass null" maps to non-`Option` parameters.
- Boundaries: wrap third-party APIs (M-DONT-LEAK-TYPES; Tauri types stay in `src-tauri`).
- Tests are first-class code (TigerStyle exhaustive tests; M-TAUTOLOGICAL-TESTS).

**Contested or obsolete:**
- **Tiny functions.** Ousterhout says Clean Code "encourages programmers to create teeny-tiny methods". Martin answers that he would rather "err on the side of decomposition" ([APOSD vs Clean Code, Sept 2024–Feb 2025](https://github.com/johnousterhout/aposd-vs-clean-code)). Ousterhout argues for *deep modules*: a simple interface over substantial logic ([APOSD 2nd ed. 2021](https://web.stanford.edu/~ouster/cgi-bin/aposd.php)). That matches TigerStyle's hourglass. **Decision**: 70 lines is an upper bound, not a target. No rule splits a function only to make it shorter.
- **Comments.** Clean Code's "Comments are always failures" (quoted in the debate) is contradicted by TigerStyle ("always say why") and Google (document public APIs, panics, unsafe invariants). Ousterhout estimates that missing comments cost "easily 10-100x" more than wrong ones. **Decision**: why-comments are required, what-comments are a finding.
- **Exceptions over return codes.** Obsolete in Rust: there are no exceptions. Use `Result` and `?` for operating errors, panic for bugs (Google, M-PANIC-ON-BUG). `catch_unwind` is banned (Google).
- **"Replace switch with polymorphism".** Muratori measured the polymorphic version about 1.5x slower than a `switch` and about 10x slower than table-driven code ([article](https://www.computerenhance.com/p/clean-code-horrible-performance)). In Rust, closed sets are `enum` plus an exhaustive `match`, which the compiler checks. Google: no traits merely to organize code. **Decision**: closed sets are enums; traits only for real extension points.
- **TDD as mandatory.** Ousterhout is "not a fan" ([debate](https://github.com/johnousterhout/aposd-vs-clean-code)). **Decision**: tests are required (TEST rules), test-first is not.
- **Small classes, SRP.** Holds at module and crate level (M-SMALLER-CRATES, M-BALANCED-MODULES), but Google prefers inherent impls over trait ceremony.

## 4. Conflicts and resolution

| # | Conflict | Winner | Resolution |
|---|---|---|---|
| C1 | Function size: Clean Code very small vs TigerStyle 70 lines hard vs Clippy default 100 | TigerStyle | `too-many-lines-threshold = 70`; no lower bound (Ousterhout supports this) |
| C2 | Comments: Clean Code "comments are failures" vs TigerStyle "say why" vs Google "omit trivial comments" | TigerStyle + Google | why-comments required, trivial or what-comments are findings |
| C3 | TODOs: Google allows `// TODO: bug-link` vs TigerStyle zero technical debt | TigerStyle | no `TODO`/`FIXME` in merged code; an issue link in the PR instead (raubritter G-4) |
| C4 | Assertions: TigerStyle crash on corruption, 2 per function vs Google "panic for bugs" plus the option to avoid panics vs a GUI that must not lose user work | TigerStyle, adapted | `assert!` (on in release) in core crates; a crash is acceptable **only because** files are saved atomically and autosaved (FMT-2). Open question Q3 |
| C5 | `debug_assert!` vs `assert!` | std docs | `assert!` by default; `debug_assert!` only for expensive checks with a comment |
| C6 | `unwrap`: Clean Code silent vs Google "`expect` over `unwrap`", pattern matching over check-then-unwrap | Google | `unwrap_used` deny (allowed in tests); `expect("invariant: …")` allowed |
| C7 | Nested `if` (TigerStyle) vs Clippy `collapsible_if`/`collapsible_else_if` | TigerStyle | allow both lints workspace-wide |
| C8 | Acronyms `VSRState` (TigerStyle) vs C-CASE `VsrState` | **exception: Rust** | compiler and Clippy convention; fighting it costs more than it gains |
| C9 | Out pointers and `*const` for >16 bytes (TigerStyle) vs C-NO-OUT and borrowing | **exception: Rust** | Zig-specific; drop |
| C10 | Static allocation (TigerStyle) vs an interactive editor | **exception: domain** | keep "limit everything" (LIM rules), drop static allocation |
| C11 | `u32` everywhere (TigerStyle) vs `usize` indexing in std | TigerStyle, adapted | domain and file IDs/counts `u32`/`u64` newtypes; `usize` only as an in-memory index |
| C12 | Zero dependencies (TigerStyle) vs Tauri, serde, React | adapted | justified dependencies plus cargo-deny (DEP rules) |
| C13 | Polymorphism (Clean Code) vs enums (Google, Muratori) | Google | enums plus exhaustive `match` |
| C14 | Google "no glob imports" vs `use super::*` in tests | Google already allows it | allowed in `#[cfg(test)] mod tests` only |
| C15 | Google `imports_granularity = Module` vs stable rustfmt | Google allows skipping | the option is **unstable** ([Configurations.md](https://github.com/rust-lang/rustfmt/blob/main/Configurations.md)), so review enforces it, or nightly rustfmt (Q6) |
| C16 | `#[allow]` (common) vs `#[expect(…, reason)]` (Microsoft) | Microsoft (no conflict with higher sources) | `allow_attributes_without_reason` deny plus `allow_attributes` warn |

## 5. Tooling

### 5.1 Versions (2026-11-02)

Rust stable **1.99.1** (2026-10-15; 1.99.0 on 2026-10-01) ([releases](https://blog.rust-lang.org/releases/)). By the six-week cadence the next release would be about 2026-11-12 (my inference, not verified). Edition 2024 was stabilized in Rust **1.85.0** ([edition guide](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)).

| Tool | Version | Date | MSRV (`rust_version` on crates.io) | Note |
|---|---|---|---|---|
| [tauri](https://crates.io/crates/tauri) | 2.12.5 | 2026-10-21 | 1.90 | sets our floor |
| [cargo-nextest](https://crates.io/crates/cargo-nextest) | 0.9.150 | 2026-10-26 | 1.91 | test runner |
| [cargo-deny](https://crates.io/crates/cargo-deny) | 0.20.2 | 2026-07-09 | 1.88 | [checks](https://embarkstudios.github.io/cargo-deny/checks/index.html): advisories, bans (incl. duplicates), licenses, sources |
| [cargo-machete](https://crates.io/crates/cargo-machete) | 0.9.2 | 2026-04-15 | n/a | unused deps, stable toolchain |
| [cargo-udeps](https://crates.io/crates/cargo-udeps) | 0.1.61 | 2026-04-29 | n/a | needs nightly (not verified in this session) |
| [cargo-shear](https://crates.io/crates/cargo-shear) | 1.14.0 | 2026-09-22 | 1.95 | alternative to machete (docs not read) |
| [proptest](https://crates.io/crates/proptest) | 1.12.0 | 2026-10-10 | 1.88 | property tests |
| [quickcheck](https://crates.io/crates/quickcheck) | 1.1.0 | 2026-02-10 | 1.85 | released less often |
| [cargo-mutants](https://crates.io/crates/cargo-mutants) | 27.1.0 | 2026-06-02 | 1.88 | [mutation testing](https://mutants.rs/) |
| [cargo-fuzz](https://crates.io/crates/cargo-fuzz) | 0.13.2 | 2026-06-09 | n/a | needs **nightly** ([setup](https://rust-fuzz.github.io/book/cargo-fuzz/setup.html)) |
| [cargo-llvm-cov](https://crates.io/crates/cargo-llvm-cov) | 0.9.1 | 2026-09-06 | 1.87 | coverage report |
| [thiserror](https://crates.io/crates/thiserror) / [anyhow](https://crates.io/crates/anyhow) | 2.0.21 / 1.0.104 | 2026-09 / 07 | 1.77 / 1.68 | typed errors in libs / apps (M-APP-ERROR) |
| [insta](https://crates.io/crates/insta) | 1.49.0 | 2026-10-03 | 1.66 | snapshot tests (file-format golden files) |
| Miri | nightly component | n/a | n/a | only if `unsafe` ever appears |

**Edition 2024 notes:** `edition = "2024"` implies `resolver = "3"`, which is **MSRV-aware**: it prefers dependency versions compatible with `rust-version` ([cargo-resolver](https://doc.rust-lang.org/edition-guide/rust-2024/cargo-resolver.html)). In a virtual workspace, `resolver` must be set explicitly in `[workspace]`. `unsafe_op_in_unsafe_fn` warns by default ([chapter](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html)). The edition also has chapters on unsafe attributes and `static mut` references; I did not read the remaining 2024 chapters (temporary scopes in `if let` and tail expressions, RPIT capture, never-type fallback) for this file.

**MSRV policy (proposal):** the app is not a published library, so MSRV means the pinned toolchain. `rust-toolchain.toml` pins `1.99.1`; `[workspace.package] rust-version = "1.99"`; upgrades come in their own PR (M-MSRV: "conservatively updated").

### 5.2 Configuration sketch

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "3"
members = ["crates/*", "src-tauri"]

[workspace.package]
edition = "2024"
rust-version = "1.99"

[profile.release]
overflow-checks = true          # TigerStyle: crash on corruption instead of wrapping (C4)

[workspace.lints.rust]
unsafe_code = "forbid"          # Google: unsafe only if no safe way; we have none
missing_debug_implementations = "warn"   # M-STATIC-VERIFICATION baseline
unused_lifetimes = "warn"
redundant_lifetimes = "warn"
trivial_numeric_casts = "warn"

[workspace.lints.clippy]
correctness = { level = "deny", priority = -1 }
suspicious  = { level = "warn", priority = -1 }
style       = { level = "warn", priority = -1 }
complexity  = { level = "warn", priority = -1 }
perf        = { level = "warn", priority = -1 }
pedantic    = { level = "warn", priority = -1 }   # Microsoft baseline; Clippy itself uses it
# TigerStyle wins over these (C7)
collapsible_if = "allow"
collapsible_else_if = "allow"
# Panics are for bugs only, with a message (Google, M-PANIC-*)
unwrap_used = "deny"
missing_assert_message = "warn"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
print_stdout = "deny"
print_stderr = "deny"
# Errors are handled, never discarded (TigerStyle)
let_underscore_must_use = "deny"
map_err_ignore = "warn"
unused_result_ok = "warn"
# Lint overrides use #[expect(.., reason = "..")] (M-LINT-OVERRIDE-EXPECT)
allow_attributes = "warn"
allow_attributes_without_reason = "deny"
undocumented_unsafe_blocks = "deny"   # only bites if unsafe_code is ever lifted for one crate
```

Every member crate sets `[lints] workspace = true` ([workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html); `[lints]` is respected since Cargo 1.74, [manifest](https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section)). Lower `priority` numbers are overridden by higher ones, so groups get `-1`. `arithmetic_side_effects` and `indexing_slicing` are **not** enabled workspace-wide: an out-of-bounds index panics, which is TigerStyle's "crash on bug", and `overflow-checks = true` covers overflow. Whether to enable them in the verification engine crate only is Q4.

```toml
# clippy.toml
too-many-lines-threshold = 70     # TigerStyle
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
max-fn-params-bools = 1           # Google: enum over bool
# disallowed-methods = [ { path = "std::process::Command::new", reason = "IPC-4" } ]
```

```toml
# rustfmt.toml (stable options only)
style_edition = "2024"
max_width = 100                   # TigerStyle + Google (also the default)
hex_literal_case = "Lower"        # Google
use_field_init_shorthand = true
use_try_shorthand = true
# imports_granularity = "Module" and format_macro_matchers = true: Google wants them, but both are unstable (C15)
```

**One `check` command** (local and CI, no deploy): `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo nextest run --locked`, `cargo test --doc`, `cargo deny check`, `cargo machete`, plus the TS checks (6.3). Nightly-only jobs, scheduled and not merge gates: `cargo fuzz` on the file parser, `cargo mutants --in-diff` on core crates (not verified: the `--in-diff` flag is not on the mutants.rs front page). Coverage (`cargo llvm-cov`) is a report, not a gate (raubritter TEST-11).

## 6. Local desktop specifics

### 6.1 Model files outlive releases (from raubritter API-4..8)

The model file is the only data that outlives a release. The IPC contract does not need compatibility, because frontend and core ship in one binary. Proposed rules (project decisions; the sources are the raubritter rules plus TigerStyle limits and pair assertions):
- Explicit `format_version`. The reader opens every released version through pure migration functions `vN → vN+1`. Each released version has a golden fixture test (insta).
- Element IDs are stable, never reused, and survive renames. Serialization is deterministic (`BTreeMap`, sorted) so models diff well in git.
- An unknown newer version or unknown element kind means: refuse to open, or open read-only with a message. Never drop fields silently, never overwrite.
- Saving is atomic (temp file in the same directory, flush, rename) with autosave. This is what makes TigerStyle's "crash on bug" acceptable (C4).
- File content is untrusted input: size and count limits before allocation (LIM), typed serde decoding, no panic on malformed input (Google: document panics; malformed input is a `Result`), parser fuzzed.

### 6.2 Tauri boundary and security

- Tauri treats the core (full system access) and the WebView (only what IPC exposes) as separate trust groups. "Inspecting and strongly defining all data crossing these boundaries" is "very important" ([security](https://v2.tauri.app/security/)). Capabilities in `src-tauri/capabilities/*.json` grant permissions per window ([capabilities](https://v2.tauri.app/security/capabilities/)). Registered commands are allowed for all windows by default unless restricted.
- fs plugin: a permission "by itself does **not** allow access to any path"; `deny` beats `allow`; parent-directory traversal is blocked ([fs plugin](https://v2.tauri.app/plugin/file-system/)). Proposal: file access goes through Rust commands plus native dialogs, not through the fs plugin from TS.
- Command return values and errors must implement `serde::Serialize`. The docs show a `thiserror` enum serialized as a tagged `{ kind, message }` that TS can match ([calling Rust](https://v2.tauri.app/develop/calling-rust/)). Heavy work goes into async commands so the UI does not freeze. Streams such as verification progress use **Channels**.
- Not verified: how a panic inside a `#[tauri::command]` surfaces (crash vs rejected promise). See Q3.

### 6.3 React/TypeScript rules carried over from raubritter

Source: `/home/nils/.t3/projects/raubritter/CODING_STANDARDS.md` (as of 2026-11-01). Differences here: no server, Supabase, offline store or PWA; data comes from Rust over Tauri IPC (commands and channels); the UI is a canvas editor; everything is English. Effect 4 and TanStack Query are open (Q1, Q2). Current versions: TypeScript 7.0.2, React 19.3.1, Oxlint 1.90.0, Oxfmt 0.75.0, Biome 2.6.4, Vitest 5.0.5, knip 6.44.1, `@tauri-apps/api` 2.12.4 ([npm registry](https://www.npmjs.com/package/typescript), queried 2026-11-02). **Lint/format**: keep Oxlint plus Oxfmt (same as raubritter, type-aware rules with TS 7). Biome is the alternative, but switching only costs consistency between the two projects.

| raubritter ID | Verdict | Reason |
|---|---|---|
| G-1 | adapt | over-engineering ban; one user, one maintainer; the "not without an ADR" list is rewritten for this stack |
| G-2, G-3 | keep | one concern per PR, dependency justification (now also crates); why-comments (= TigerStyle) |
| G-4 | keep | no TODO, no commented-out code (= TigerStyle zero debt, C3) |
| STR-1 | adapt | dependency direction: `ui` → IPC client → (Rust) `src-tauri` → `core` crates; core never knows Tauri |
| STR-3, STR-4, STR-6, STR-9 | keep | knip, feature folders, no barrels, no default exports apply unchanged |
| STR-5 | open | depends on whether a router is used at all (Q2) |
| STR-7 | adapt | one `ipc/` module wraps `invoke`/`Channel`; components never call `invoke` directly |
| STR-2, STR-8 | drop | single UI package; no env config (settings live in Rust) |
| NAM-1, NAM-5, NAM-6 | keep | kebab-case files, React naming, no `I` prefix |
| NAM-2 | adapt | English `GLOSSARY.md` terms (Place, Transition, Container…) used exactly, in Rust and TS alike |
| NAM-3, NAM-4 | drop | German-only |
| TS-1 | adapt | TS 7 `strict`, `noUncheckedIndexedAccess`, `verbatimModuleSyntax`, `erasableSyntaxOnly`; Effect language service only if Q1 = yes |
| TS-2, TS-4, TS-5, TS-7…TS-13, TS-15, TS-16 | keep | language hygiene independent of backend; TS-6/TS-11 also cover verdicts (`Proved`/`Disproved`/`Unknown`) and IPC error `kind`s |
| TS-3 | adapt | unknown data = clipboard, drag-and-drop, `localStorage`, `postMessage`; IPC payloads are typed by generated bindings (Q5) |
| TS-14 | adapt | serde writes Rust `None` as `null` unless `skip_serializing_if` is used ([serde](https://serde.rs/attr-skip-serializing.html), not re-read). Pick one convention at the IPC boundary |
| NUM-1, NUM-2 | drop | no money |
| NUM-3 | keep | numeric inputs (token counts, bounds) parsed with checks |
| VAL-1 | adapt | boundaries on the TS side are forms, clipboard, drag-and-drop and persisted UI prefs; files are parsed in Rust (6.1) |
| VAL-2 | adapt | domain types come from Rust (generated), not hand-written TS interfaces |
| VAL-4 | keep | no re-validation between internal functions |
| VAL-5 | adapt | English messages |
| VAL-3 | drop | Effect-JIT/CSP detail |
| API-1…API-3 | adapt | bind Rust core crates via Google's library-code rule; TS keeps API-2 (no boolean mode params = Google enum-over-bool) |
| API-4 | keep | update all callers, no `V2` |
| API-5, API-8 | adapt | become the file-format rules (6.1) |
| API-6, API-7 | drop | Dexie and RPC |
| EFF-1…EFF-17 | open | only if Effect is adopted (Q1) |
| LOG-1 | adapt | no Sentry or telemetry; errors shown in UI and written to a local log, once |
| LOG-2 | keep | no `console.*`; log levels fixed |
| R-1 | adapt | new state table: model state lives in the Rust core; the UI holds a projection, canvas viewport and selection as UI state |
| R-2, R-3, R-6, R-8, R-11, R-12, R-16, R-19, R-20, R-23, R-24, R-25 | keep | pure React discipline, independent of data source |
| R-18 | keep | `useSyncExternalStore` for Tauri event and channel subscriptions and the core-projection store |
| R-9 | adapt | React Compiler on; exception for referential stability required by the canvas library, with a comment |
| R-10 | keep | keys from stable element IDs (6.1) |
| R-13 | adapt | mutations are IPC commands; undo/redo lives in the core, not in React state |
| R-14 | adapt | from the outbox rule keep only: long verifications never block input; async command plus progress Channel plus cancel |
| R-22 | keep | loading, error and empty states, plus "verification running / Unknown" |
| R-4, R-5, R-17 | open | server-data patterns; only relevant if TanStack Query wraps IPC (Q2) |
| R-7, R-15, R-21 | drop | Dexie, URL search params, `useLiveQuery` |
| UI-1, UI-3 | keep | native buttons/links; shadcn/Base UI primitives (confirm in stack research) |
| UI-2 | adapt | `lang="en"`; every canvas action has a keyboard path |
| UI-4 | adapt | verdicts and counterexample highlights are not color-only |
| UI-5 | adapt | container queries keep; phone breakpoints and safe-area rules drop |
| PERF-5 | keep | tuning (canvas virtualization, workers) only with a before/after measurement |
| PERF-2 | adapt | no sequential IPC calls without a data dependency |
| PERF-4 | adapt | fonts bundled locally, no CDN |
| PERF-1, PERF-3 | drop | bundle download and service worker do not matter for a local app |
| TEST-1, TEST-3, TEST-4, TEST-5, TEST-6, TEST-9 | keep | logic tests, time as parameter, no sleeps, role queries, independent tests, builders |
| TEST-7 | adapt | mock only the IPC boundary; `@tauri-apps/api` ships IPC mocks (not verified in this session) |
| TEST-8, TEST-10 | adapt | Vitest browser mode for canvas components; English test names |
| TEST-11 | adapt | few E2E flows; Tauri E2E tooling is a stack-research question |
| TEST-2, TEST-12 | drop / open | pgTAP; Effect tests only if Q1 |
| SEC-1 | keep | no `innerHTML`/`dangerouslySetInnerHTML`: model files can carry hostile text |
| SEC-2, SEC-3 | adapt | if element descriptions render Markdown: same safe renderer; links open through a protocol allowlist |
| SEC-4…SEC-13 | drop | auth, Supabase, CSP headers, Sentry |
| OPS-1…OPS-3, OPS-5, OPS-6 | keep | CI least privilege, SHA-pinned actions, no injection, pnpm supply-chain settings |
| OPS-4, OPS-7 | drop | no deploy |
| DB-*, LLM-* | drop | not applicable |

## 7. Not verified

- Whether `too_many_lines` counts blank and comment lines.
- `cargo-udeps` needing nightly; `cargo mutants --in-diff`; Tauri IPC mocks in `@tauri-apps/api`; serde `null` behavior. All are from memory or not re-read.
- Behavior of a panic inside a Tauri command.
- Exact wording of *Clean Code* 2nd ed. (cited through the Ousterhout–Martin debate).
- Must/should levels of Microsoft's AI rules; the remaining edition-2024 chapter contents.
- Third-party summaries of the Google guide (for example braindetox.kr) were ignored; only the guide itself was used.

## Proposed rule outline for CODING_STANDARDS.md

Tags: [TS]=TigerStyle, [G]=Google Rust guide, [API]=Rust API Guidelines, [MS]=Microsoft Pragmatic Rust, [CL]=Clippy/rustc, [CC]=Clean Code, [APOSD]=Ousterhout, [RB]=raubritter, [TA]=Tauri docs, [P]=project decision. [auto]=enforced by `check`.

**G – Principles**
- G-1 must: no over-engineering; minimum of excellent abstractions; app code favors directness [TS][G][RB]
- G-2 must: zero technical debt: no `todo!`/`unimplemented!`/`dbg!` [auto], no TODO or commented-out code [TS][RB]
- G-3 must: comments say why; doc comments on public core APIs, with panics and errors documented; no trivial comments [TS][G][APOSD]
- G-4 should: one concern per PR; every new crate or npm package justified [RB][TS]

**SAFE – Assertions and panics**
- SAFE-1 must: core functions assert preconditions, postconditions and invariants with `assert!`; pair assertions across write/read [TS]
- SAFE-2 must [auto]: every `assert!`/`expect` has a message stating the invariant [G][MS][CL]
- SAFE-3 must: `debug_assert!` only for expensive checks, with a comment [std][TS]
- SAFE-4 must [auto]: no `unwrap` outside tests; pattern matching over check-then-unwrap [G][CL]
- SAFE-5 must: panic only for bugs; never `catch_unwind`; no panics as control flow [G][MS]
- SAFE-6 should: assert relationships of constants at compile time (`const _: () = assert!(..)`) [TS]
- SAFE-7 must [auto]: `overflow-checks = true` in release [TS][P]

**LIM – Limits**
- LIM-1 must: every loop, queue, channel and exploration has an explicit bound; hitting it yields `Unknown`, never a claimed result [TS][P]
- LIM-2 must: no recursion; traversals use an explicit stack; unavoidable recursion has an asserted depth limit [TS]
- LIM-3 must: input (files, IPC) is checked against size and count limits before allocation [TS][P]
- LIM-4 should: domain IDs and counts are `u32`/`u64` newtypes; `usize` only for in-memory indexing; casts via `try_from` [TS][G][CL]

**ERR – Errors**
- ERR-1 must: `Option` for obvious absence, `Result` for operating errors, panic for bugs [G][MS]
- ERR-2 must [auto]: no discarded errors (`let _ =`, `.ok()`, `map_err(|_|..)`) [TS][CL][MS]
- ERR-3 should: core crates use `thiserror` enums; `anyhow` only in `src-tauri` and tools [MS]
- ERR-4 must: Tauri command errors are a serializable tagged enum, never `String` [TA][P]

**FN – Functions and control flow**
- FN-1 must [auto]: at most 70 lines per function; never split only to shrink [TS][APOSD]
- FN-2 should: push `if`s up and `for`s down; leaves pure, parent owns state [TS]
- FN-3 should: nested `if/else` over compound conditions; handle or assert the `else` [TS]
- FN-4 must: mixable arguments become an options struct or newtypes; enum over bool [TS][G][API]
- FN-5 should: closed sets are enums with exhaustive `match`; traits only for real extension points [G][CC-critique]

**TY – Types, naming, modules**
- TY-1 must: Rust naming (C-CASE, `as_/to_/into_`, getters without `get_`); no abbreviations; units last (`states_max`) [API][G][TS]
- TY-2 must: no glob imports (except `use super::*` in tests); one module per `use`; qualify free functions [G]
- TY-3 should: newtypes for distinct domain values; named fields over tuples [G][API]
- TY-4 should: no weasel words (`Manager`, `Service`, `Factory`) [MS]
- TY-5 should: `#[must_use]` where ignoring a result is a bug; `Debug` on all types [G][API][MS]
- TY-6 should: avoid `Rc`/`Arc` and deep `.clone()` unless justified [G]

**DEP – Dependencies and unsafe**
- DEP-1 must [auto]: `unsafe_code = "forbid"`; lifting it needs an ADR, a separate crate, `SAFETY:` comments and Miri [G][MS][CL]
- DEP-2 must [auto]: cargo-deny (advisories, licenses, bans, sources) and cargo-machete green; `Cargo.lock` committed, `--locked` in CI [MS][P]
- DEP-3 must [auto]: lint overrides use `#[expect(.., reason)]` [MS][CL]
- DEP-4 should: no unstable features; toolchain pinned; MSRV raised in its own PR [G][MS]

**FMT – Model file format**
- FMT-1 must: `format_version` plus migrations for every released version with golden fixtures [RB][P]
- FMT-2 must: atomic save plus autosave; a crash never corrupts or loses a file [P]
- FMT-3 must: stable, never-reused element IDs; deterministic serialization [P]
- FMT-4 must: unknown version or element kind → refuse or read-only, never silent loss [RB][P]
- FMT-5 must: the parser never panics on malformed input; validate after read and before write [G][TS]

**IPC – Tauri boundary**
- IPC-1 must: least-privilege capabilities per window; no remote URLs; no fs plugin scope wider than needed [TA]
- IPC-2 must: every command validates its arguments; the WebView is a lower-trust group [TA]
- IPC-3 must: long work is an async command with Channel progress and cancel; the UI never blocks [TA][TS]
- IPC-4 must: no `std::process::Command` or shell with user-provided input (`disallowed-methods`) [P][CL]

**TEST**
- TEST-1 must: new core logic has unit tests; a bug fix has a test that was red before [RB]
- TEST-2 must: Petri-net and graph algorithms have property tests with valid and invalid inputs [TS][P]
- TEST-3 should: file parser fuzzed (nightly job) [TS][P]
- TEST-4 must: no tautological tests [MS]
- TEST-5 must: time and randomness are parameters [RB]
- TEST-6 should: mutation testing and coverage are reports, not gates [RB][P]

**TOOL**
- TOOL-1 must [auto]: one `check` runs fmt, clippy `-D warnings`, nextest, doc tests, deny, machete, TS lint/typecheck/test; CI runs it with no deploy [TS][RB]
- TOOL-2 must: CI least privilege, SHA-pinned actions, no untrusted context in `run:` [RB]

**TS – TypeScript** (from raubritter): TS-1 strict TS 7 config [RB]; TS-2 no `any`/`@ts-ignore` [RB][auto]; TS-3 unknown data decoded at the boundary [RB]; TS-4 no unchecked `as` [RB]; TS-5 `satisfies` [RB]; TS-6 exhaustive switch with `assertNever` (verdicts, IPC error kinds) [RB]; TS-7 no `enum`/`namespace` [RB][auto]; TS-8 interface vs type [RB]; TS-9 `import type` [RB][auto]; TS-10 no floating promises (`invoke`) [RB][auto]; TS-11 multi-phase state as tagged unions [RB]; TS-12 only `Error`s thrown, with `cause` [RB]; TS-13 no silent catch [RB]; TS-14 one null/undefined convention at the IPC boundary [RB][P]; TS-15 no `{}`/single-use generics [RB][auto]; TS-16 simplest type [RB]; TS-17 IPC types generated from Rust, never hand-written [P]; TS-18 no `console.*` [RB][auto].

**R – React** (from raubritter): R-1 state table (core / UI projection / UI state) [RB][P]; R-2 derive during render [RB]; R-3 `useEffect` only for external systems [RB]; R-6 no one-hook wrappers [RB]; R-8 Rules of React, StrictMode [RB]; R-9 React Compiler, memo only with reason (canvas) [RB]; R-10 stable keys from element IDs [RB]; R-11 composition over boolean props [RB]; R-12 variants not className overrides [RB]; R-13 mutations are IPC commands, undo in the core [RB][P]; R-14 long work never blocks input [RB][TA]; R-16 Context only for rarely changing values [RB]; R-18 `useSyncExternalStore` for Tauri events and channels [RB]; R-19 `useEffectEvent` [RB]; R-20 components at module level [RB]; R-22 loading, error, empty and Unknown states [RB][P]; R-23 tagged-union props [RB]; R-24 children before Context [RB]; R-25 no mutable module state [RB]; R-26 components never call `invoke` directly (`ipc/` module) [RB STR-7][P].

**UI – Accessibility**: UI-1 native buttons/links [RB][auto]; UI-2 labels, `lang="en"`, visible focus, keyboard path for canvas actions [RB][P]; UI-3 primitives from the component library [RB]; UI-4 verdicts not color-only and announced [RB]; UI-5 container queries for panes [RB].

**SEC (UI)**: SEC-1 no `innerHTML`/`dangerouslySetInnerHTML` [RB]; SEC-2 safe Markdown and link allowlist, if Markdown exists [RB].

## Open questions

1. **Effect 4 in the UI?** With Tauri IPC and no network, its main payoffs (services around Supabase/Dexie, retries) shrink. If no, all EFF rules and TEST-12 drop.
2. **TanStack Query (and a router) for IPC reads?** Alternative: one external store fed by core events (`useSyncExternalStore`). This decides R-4/R-5/R-17 and STR-5.
3. **Panics in the GUI:** `panic = "abort"` or unwind for the release profile, and what the user sees when the core hits an assertion. TigerStyle says crash; FMT-2 must make that safe. Needs a spike: panic inside a `#[tauri::command]`.
4. Enable `arithmetic_side_effects` and `indexing_slicing` only in the verification engine crate? It is safer, but noisy.
5. IPC type generation (tauri-specta, ts-rs or hand-written)? This decides TS-3, TS-17 and VAL-2.
6. Nightly rustfmt only for `imports_granularity = "Module"` (Google)? Or enforce it in review?
7. Should the assertion density target (TigerStyle 2 per function) be a review heuristic for core crates only, or dropped?
8. Default `too-many-arguments-threshold` (7) or lower (TigerStyle hourglass, Clean Code ≤3)?
9. Should Microsoft's `all.txt` and the Google guide be loaded into agent review sessions, or does `CODING_STANDARDS.md` stand alone?
10. Kani (0.68.0) or other Rust verifiers for the core: belongs to `docs/research/theorem-proving.md`, not here.
