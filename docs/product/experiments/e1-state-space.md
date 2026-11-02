# E1 state-space spike: is an in-house explicit-state checker correct and fast enough? (A4, A5)

As of 2026-11-02. Experiment E1 from `../opportunity-solution-tree.md`. It also runs the TAPAAL check of D-35 and D-59 (`../../specs/decisions.md`). Code and fixtures: `prototypes/e1-state-space/` on branch `prototype/e1-state-space` (throwaway, see its `README.md`). Full run outputs: `prototypes/e1-state-space/results/`. Terms follow `GLOSSARY.md`; net notation follows NS §2.4 (`docs/specs/net-semantics.md`).

**Result: PASS.** All three E1 verdicts are correct, each takes at most 13 µs in a release build, and TAPAAL agrees with all of them. On the wider corpus (49 checks: the E1 nets, the slice 0 property vectors of NS §14.1 and one extra net), our verdicts, counterexample lengths, stored counts and |R|/|O| match NS §14 in every case. All 92 TAPAAL runs that could be compared agree. None disagrees. The D-35 encoding is right for V08, and the D-35 exclusion of drop nets is necessary for V12. Two things the corpus must do differently from a naive reading of D-35 are listed under "Surprises".

## Question

Assumptions A4 and A5: an explicit-state checker written in-house gives the right verdict, and gives it fast enough. Pass criterion: (a), (b) and (c) below get the expected verdicts, each within 2 s from a release build, and TAPAAL verifypn agrees on 100 % of the corpus. If it fails, the fallback is to narrow the v1 net class or pull S11 forward, never to relax the Proved rule.

## Method

- **Explorer.** A Rust crate written from NS alone: the firing rule with block, drop and overflow (§4), deadlock with F = none (§7), deadlock-free and no capacity exceeded (§8.1, §8.2, including the `NoOverflowPlace` certificate), BFS in the normative order with violation tests at discovery (§12), and `waits(M, t)` for deadlocks. Markings are `u32` vectors in canonical place order; tentative counts are computed in `u64`. The crate has no reductions, no budget and no second explorer (`recheck`).
- **Fixtures.**
  - (a)–(c), V08 and V12 are mini projects in the element TOML of `file-format.md` §6: `arch.toml` and `elements/*.toml` with `[petri_net]`, ports, `[relationships.*]` with binding pairs, and `[[property]]` entries. Each comes with an `expected.toml` that the app ignores.
  - The prototype lowers each property's scope (own or combined; only L1 and L4 are checked) and flattens it (NS §6.5).
  - The flattened nets are compared with the NS §14 nets, and all six are identical: PC(overflow), LOCKS(yx), POOL, POOL-LEAK, V08 and V12.
- **Wider corpus.** `fixtures/ns-vectors.toml` holds the 33 slice 0 property vectors of NS §14.1 that run on a flattened net (V01–V27, V43, V45, V48, V51–V53), written as flattened nets. It also holds **X1**, a net built for this spike: a deadlock that can only be reached through an overflow marking.
- **TAPAAL.**
  - verifypn **4.2.3** (`verifypn-linux64.zip`, sha256 `de71aae9…51246f3`), downloaded into `$TMPDIR`. It is the newest release with a Linux binary: v4.3.0 and v4.4.2 have no binaries attached.
  - Every corpus net is exported to PNML following D-35, and every query runs twice: with default options, and with `-r 0 -p -s BFS` (no structural reduction, no stubborn sets, BFS).
  - For bounded nets, `verifypn -e -r 0 -p` gives a state count that is compared with |R|.
- **Encoding (D-35).**
  - Each `block` place p with capacity K gets a complement place p̄ with initial K − M₀(p). For each transition t, with d = W⁺(t, p) − W⁻(p, t), t gets an arc p̄ → t of weight d when d > 0, or an arc t → p̄ of weight −d when d < 0. Then t is enabled only if M(p) − W⁻ + W⁺ ≤ K, which is (E3) exactly.
  - `overflow` places stay plain places.
  - Nets with a `drop` place are excluded.
  - PNML IDs map `/` to `__` and `+` to `.`, with `.cmp` for complements.
- **Queries (MCC XML).** inb is ∧ M(p) ≤ cap(p) over the overflow places.
  - No capacity exceeded: `AG ∧ M(p) ≤ cap(p)`. D-35 asks for the overflow places only. Querying every place also tests that the complements keep block places in bounds. True maps to Proved, false to Disproved.
  - Deadlock-free on a net without overflow places: `EF deadlock`. True maps to Disproved, false to Proved.
  - Deadlock-free on a net with overflow places: `E[inb U (inb ∧ deadlock)]`. True maps to Disproved. If it is false, `AG inb` decides: true maps to Proved, false to Unknown "capacity exceeded first". Surprise 2 explains why this form is needed.
- **Timing.** Release build, AMD Ryzen 9 5900X (24 threads, one used), WSL2. "Our time" covers compiling the net, exploring and building the verdict, with no TAPAAL running at the same time. TAPAAL time is the wall time of the verifypn process or processes.

## The nets

- **(a) Producer faster than consumer** (`e1a-producer-consumer`, NS PC(overflow)).
  - Producer `p` has `p/items(5, block, 5)` and port `p/produce: p/items → ∅`. Consumer `c` has `c/got(5, block)` and port `c/consume: ∅ → c/got`.
  - Relationship `r` binds them asynchronously with capacity 3 and when full `overflow`, which generates the place `r`. Nothing makes the producer wait.
- **(b) Two locks in opposite order** (`e1b-opposite-locks`, NS LOCKS(yx)).
  - Lock service `l`: `l/x(1, block, 1)`, `l/y(1, block, 1)`, and ports `acq_x`, `acq_y`, `rel_xy`.
  - Components `a` and `b`, each with `idle → has_1 → has_2 → idle`. `a` takes x then y, `b` takes y then x.
  - Six synchronous relationships (`ra1`–`ra3`, `rb1`–`rb3`) fuse each step with the lock's port.
- **(c) Worker pool that releases on every path** (`e1c-worker-pool`, NS POOL; "the worker-pool example from E1" in the slice 0 acceptance criteria).
  - Clients `c`: `c/idle(3, block, 3)` and `c/running(3, block)`. Pool `w`: `w/free(2, block, 2)`.
  - `r1` fuses `c/start` with `w/acquire`, and `r2` fuses `c/finish` with `w/release`.
  - Two variants: `e1c2` adds a failure path `c/fail` fused with `w/release` through `r3` (also releases), and `e1c3` (NS POOL-LEAK) adds an internal `c/fail` that does not release.
- **V08** (`ns-v08`, own scope): `x/p(1, block, 1)`, `x/t: x/p → x/p`.
- **V12** (`ns-v12`, own scope): `x/p(1, drop, 1)`, `x/t: x/p → 2·x/p`.

## Results per check

"TAPAAL" is the verdict implied by verifypn, identical under both option sets. "Markings" is |R| of a full exploration (and |O| where it is not 0). For a Disproved row it is an oracle fact, because the check itself stops at discovery. The TAPAAL time gives both option sets.

### (a) Producer faster than consumer, no capacity exceeded

| expected | ours | TAPAAL | markings | our time | TAPAAL time |
|---|---|---|---|---|---|
| Disproved (V16) | Disproved: 4 steps (`p/produce` ×4), stored 9, `Overflow { r, 3, 4 }` | Disproved | 18, \|O\| = 2 | 7 µs | 2.8 / 2.4 ms |

On the same net, deadlock-free is Disproved after 10 steps with stored 20, the terminal marking `c/got = 5` (V17, F = none), and TAPAAL agrees (13 µs ours; 65 / 63 ms TAPAAL, CTL engine).

### (b) Two locks in opposite order, deadlock-free

| expected | ours | TAPAAL | markings | our time | TAPAAL time |
|---|---|---|---|---|---|
| Disproved (V13) | Disproved: 2 steps (`ra1/take_x+acq_x`, `rb1/take_y+acq_y`), stored 5, waiting as V13 | Disproved | 6 | 11 µs | 2.9 / 2.6 ms |

The waiting list matches V13 entry for entry, e.g. `ra1/take_x+acq_x ← a/has_1 OutputFull{1,2}, a/idle NeedsTokens{1,0}, l/x NeedsTokens{1,0}`. No capacity exceeded is Proved by the `NoOverflowPlace` certificate; TAPAAL agrees (6 states).

### (c) Worker pool, deadlock-free

| fixture | expected | ours | TAPAAL | markings | our time | TAPAAL time |
|---|---|---|---|---|---|---|
| e1c (V22) | Proved | Proved, exhaustive, 3 markings | Proved (3 states) | 3 | 4 µs | 2.6 / 2.6 ms |
| e1c2 (fail releases) | Proved | Proved, exhaustive, 3 markings | Proved (3 states) | 3 | 4 µs | 3.2 / 2.8 ms |
| e1c3 (V24, leak) | Disproved | Disproved: 4 steps (start, fail, start, fail), stored 6 | Disproved (6 states) | 6 | 7 µs | 2.4 / 2.4 ms |

No capacity exceeded is Proved by certificate on all three, and TAPAAL agrees.

### V08 and V12

| check | expected | ours | TAPAAL (D-35) | markings | our time | TAPAAL time |
|---|---|---|---|---|---|---|
| V08 deadlock-free | Proved | Proved, 1 marking | Proved (1 state) | 1 | 2 µs | 2.4 / 3.0 ms |
| V12 deadlock-free | Proved | Proved, 1 marking | excluded (drop net) | 1 | 2 µs | — |

### Wider corpus

- **Vectors.** All 33 NS vectors and X1 give the expected verdict, length, stored count, |R| and |O|. This includes:
  - V27: the smallest-ID exceeded place is `x/a`.
  - V48: 37 and 13 markings.
  - V51–V53: counts of 4294967295 handled without arithmetic overflow.
- **TAPAAL.** It answered every check it could take, and all 92 comparisons agree. Two kinds of check are left out:
  - drop nets under D-35;
  - V51–V53, whose counts TAPAAL cannot represent (surprise 3).
- **State counts.** Every state count from `-e` equals our |R|.

### Scale (beyond the criterion)

This shows how far a naive explorer reaches. All runs are Proved by exhaustion, TAPAAL agrees on all of them, and they run on one thread with no reductions (`results/scale.md`, `results/scale-solo.md`).

- **pool(n, m):** n client elements share m workers; it has symmetry but no independent parts.
- **pipeline(k, 3):** k independent producer/consumer pairs with an ack window.

| net | places | \|R\| | our time | markings/s | TAPAAL default |
|---|---|---|---|---|---|
| pool(16, 8) | 33 | 39,203 | 68 ms | 5.8 · 10⁵ | 183 ms |
| pool(20, 10) | 41 | 616,666 | 2.4 s | 2.5 · 10⁵ | 4.6 s |
| pool(22, 11) | 45 | 2,449,868 | 12.6 s | 1.9 · 10⁵ | 23.2 s |
| pipeline(6, 3) | 18 | 1,000,000 | 1.9 s | 5.4 · 10⁵ | 5 ms |
| pipeline(7, 3) | 21 | 10,000,000 | 36.9 s | 2.7 · 10⁵ | 5 ms |

Peak memory at pipeline(7, 3) was 2.63 GB for 10⁷ stored markings of 21 places. That is 3.1 times the NS §10.1 lower bound (stored × |P| × 4 bytes = 840 MB) for this prototype's layout, which stores every marking twice: once in the arena and once as the hash-map key.

## D-35 / D-59 verdict for V08 and V12

Deadlock-free under each encoding. The expected verdict is Proved with |R| = 1.

| vector | encoding | TAPAAL (default and `-r 0 -p -s BFS`) | states |
|---|---|---|---|
| V08 | **D-35 (net-effect complement)** | **Proved**: agrees | 1 |
| V08 | textbook complement (mirrored arcs) | Disproved: **false disagreement** | 1 |
| V12 | **D-35 (drop net excluded)** | **excluded** | — |
| V12 | drop written as block (net effect) | Disproved: **false disagreement** | 1 |
| V12 | drop written as a plain place, no capacity | no answer in 30 s (the net is unbounded) | — |
| V12 | drop through complement and read arc (spike extension, below) | Proved: agrees | 1 |

- **V08: the D-35 block encoding is right.**
  - `x/t` has net effect 0 on `x/p`, so it gets no complement arc, and TAPAAL keeps the self-loop at capacity enabled, which is (E3).
  - The textbook construction gives `x/t` a pre arc from the empty complement and reports a deadlock that does not exist. Without the net-effect rule, the corpus would fail CI on every read arc (V08, E5 `shed_a`).
- **V12: excluding drop nets is necessary** for the obvious encodings, which either flag a false deadlock or make the net unbounded so that TAPAAL never answers.
  - **Option, not a reopening:** drop *can* be written in plain P/T nets. A complement place, plus one copy of t per clamp amount v ∈ {0, …, d − 1}, with a pre arc of v on p̄ and a read arc of cap − v on p (p̄ ≤ v ⇔ p ≥ cap − v), reproduces `min(cap, M⁺)`.
  - That encoding agrees with us on all six drop vectors that fit TAPAAL (V04, V05, V11, V12, V20, V21) and gives equal state counts.
  - The number of copies grows as ∏ (W⁺ − W⁻ + 1) over the drop places of a transition.
  - Whether the corpus should take drop nets this way is the D-35 owner's call; D-35 as written stays valid.
- **Overflow places: "M(p) ≤ cap" is right for no capacity exceeded** (V06, V16, V26, V27, V48 and X1 all agree). For deadlock-free, the query needs the until form from the method (surprise 2).

**D-35 and D-59 are closed:** the encoding holds for V08 and V12, provided the deadlock-free query on overflow nets is written as above.

## Pass / fail

| criterion | result |
|---|---|
| all three verdicts correct | **yes**: (a) Disproved, (b) Disproved, (c) Proved, with the NS §14 lengths, stored counts and markings |
| each within 2 s, release build | **yes**: at most 13 µs (our check); TAPAAL takes 2–65 ms per check, mostly process start-up |
| 100 % agreement with TAPAAL on the corpus | **yes**: 92 of 92 comparable runs. Excluded: drop nets under D-35, and V51–V53 (beyond TAPAAL's token limit) |

**E1: PASS.** The fallback (narrow the net class or pull S11 forward) is not triggered by E1. The scale numbers below are input for E9, not a failure of E1.

## Surprises slice 0 must know about

1. **Use net-effect complements, never the textbook ones.** Confirmed on V08: mirrored arcs give a false Disproved for every read arc. `interop` should have V08 as a golden test.
2. **Plain `EF deadlock` is wrong on nets with overflow places.**
   - TAPAAL explores past overflow markings; NS §3.2 makes them terminal. On X1 (`x/s(1, block, 1)`, `x/p(1, overflow)`, `x/t1: x/s → 2·x/p`, `x/t2: 2·x/p → ∅`), `EF deadlock` is true, a false Disproved. The NS verdict is Unknown "capacity exceeded first".
   - The corpus must ask `E[inb U (inb ∧ deadlock)]` and then `AG inb`. The D-35 owner should write these exact query forms into NS §18 or the `interop` docs.
   - The until query runs on TAPAAL's CTL engine: about 60 ms against about 2 ms for reachability. That cost is irrelevant in CI.
3. **TAPAAL counts tokens in i32, not u32.** verifypn stops with "Number of tokens in x__p exceeded 2147483647" (exit 3) on V51–V53. `interop` must skip nets with cap + max W⁺ > 2³¹ − 1, and the u32 edge stays covered by the golden vectors only.
4. **verifypn 4.2.3 mishandles a normal arc and an inhibitor arc between the same place and transition.**
   - Here (inhibitor variant of the drop encoding) it crashed (exit 139) on V11 under every option set.
   - On a one-place net, `p(2)`, `t` with a pre arc of 1 and an inhibitor arc of 2 from `p`, it answered `EF deadlock` = false, although M₀ is a deadlock. `--write-reduced` shows the pre arc lost.
   - `interop` must never emit inhibitor arcs; the read-arc form above needs none.
   - This is the case D-35 means by "a corpus disagreement is a CI failure to investigate and never feeds a verdict": the external engine can be the wrong side.
5. **Pin the TAPAAL build.** The newest Linux binary is 4.2.3 from 2022. CI should pin that zip by sha256 or build a newer release from source.
6. **The time budget, not the marking budget, decides first.**
   - This naive layout explores 2–13 · 10⁵ markings per second, and the rate falls as the state space grows (hashing boxed vectors). With the default budget (T = 2,000 ms, N = 2,000,000), the main phase gets T/2 = 1 s, so roughly 3–6 · 10⁵ markings, well under N/2 = 10⁶.
   - Expect "budget exhausted" from the deadline on large nets, and set the default budget accordingly.
   - Storing each marking once (an arena plus a hash set of indices) and a flat queue are the obvious first optimizations.
7. **The memory factor for NS §10.1 is about 3.1 for this layout.** Use 3 as the lower calibration for the registry. A layout that stores each marking once should come in under 2; measure it again in `verify`.
8. **Independent parts are where explicit-state loses.**
   - On pipeline(7, 3), TAPAAL needs 5 ms where we need 37 s, because its structural reductions and stubborn sets collapse independent components.
   - On the symmetric pool, we are about twice as fast as TAPAAL.
   - Designs with many independent elements will hit the budget early. That is the signal for S11 (stubborn sets) that E9 should watch for.
9. **The fixtures work as written.**
   - The element TOML of file-format §6 expressed (a)–(c) with no extension. Lowering them reproduces the NS §14 nets exactly, including fused IDs such as `ra1/take_x+acq_x` and the generated place `r`.
   - The V08 and V12 fixtures need a `software_system` element as parent so that `kindLevel` is satisfied.
   - The per-fixture `expected.toml` is not a project file (file-format §2.1), so the fixtures load in the app unchanged.
10. **Not covered here:**
    - the budget vectors B1–B13;
    - `recheck` (composition, replay, certificate check) and the CheckKey;
    - the lowering failures (V28, V29, V54, V56, V57);
    - the E5 vectors V49 and V50.

    E1 says nothing about those beyond the composition match above.
