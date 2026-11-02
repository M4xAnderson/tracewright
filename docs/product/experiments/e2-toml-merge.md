# E2 spike: does the TOML directory merge cleanly in git? (assumption A7)

As of 2026-11-02. Experiment E2 from `../opportunity-solution-tree.md`. It tests A7: the project format of ADR 0001, as specified in `docs/specs/file-format.md` (cited as **FF §n**), diffs and three-way merges cleanly with plain git.

**Pass criterion (set before the run):** disjoint edits merge without a conflict, and a layout change touches only layout files. If the criterion is not met, the file split changes before format 1 freezes (FF §12.5, ADR 0001 migration rule).

**Result: PARTIAL.**

- The layout half passes. Every layout-only Save touched only `layouts/`, and no model edit touched a layout file. The only exception is creating an element, which by FF §7 also writes the element's first position.
- The disjoint-edit half fails for format 1 as specified. 4 of the 18 disjoint scenarios conflict, for two causes in the format: an append at the end of a file, and two insertions at the same sorted position.
- With two changes made before the freeze, 17 of 18 merge clean: one layout file per element, and an empty last line in every file. The one remaining conflict is two new entities added at the same sorted position inside one element file. A merge driver can fix that later without a format change.
- No clean merge produced a wrong result.

## Method

Code: `prototypes/e2-toml-merge/e2.py`, Python 3 stdlib (throwaway). Full output: `prototypes/e2-toml-merge/results.md`.

1. **Mock project.** The FF §14 Kinesis fixture without its comments, plus `breaker`, a container with a state machine (FF §6.5). That gives 9 elements in two systems, Petri nets with place and transition ports, 4 relationships with sync and async binding pairs, properties, a final predicate, a suppression, and 5 layout files: two views, the landscape, and two behavior layouts. In total 16 files and 333 lines.
2. **Writer.** A canonical writer built from FF §11.1: the key order, groups sorted bytewise, binding pairs sorted, defaults omitted by rule 8, one blank line before each table header, inline tables only for arc maps.
   - It is deterministic: reversing every dict's insertion order gives the same bytes.
   - It round-trips: rendering the loaded files gives the same bytes.
   - A Save writes only files whose bytes changed (FF §9.2). Every file the mock writes is canonical, so a full re-render gives the same bytes as the in-place editing of FF §11.2.
3. **Scenarios.** Each scenario gets a fresh git repository under `$TMPDIR`:
   - commit the base on `main`;
   - branch `a` and branch `b` each make their edit as one or two Saves and commits;
   - `git merge b` into `a`, once with `-X diff-algorithm=myers` and once with `histogram`. Both gave identical results in every scenario, so the outcome does not depend on the diff algorithm a user's git uses.
4. **Measured per scenario.** Conflict yes or no, the conflicted files and hunks, the files each branch touched, and the diff size (lines added/removed, `git diff --numstat` against the base).
   - After a clean merge, the merged tree had to be **byte-identical to the writer's output for the model with both edits applied**. So the merge is correct, canonical, and the next Save does not rewrite it.
   - Every layout-gesture commit was checked to touch only `layouts/`.
5. **Variants.** The same scenarios were run on four variants of the format: FF format 1 as specified, and three candidate fixes (see Recommendations).

Tools: git 2.43.0 (`ort` strategy), Python 3.12.3. The FF §11.2 in-place writer (toml_edit) was not exercised; see Limits.

## Results, format 1 as specified

Diff sizes are lines added/removed per branch against the base. "Merge OK" means the merged tree equals the writer's output for both edits.

| # | Branch A | Branch B | Files A / B | Diff A / B | Git | Merge OK | Cause in the format |
|---|---|---|---|---|---|---|---|
| 1 | `fe1/peer_threads` capacity 4 → 6 | `membership/members` initial 5 → 3, then (a second Save) move `cognito-web` | `elements/fe1.toml` / `elements/membership.toml`, `layouts/cognito.toml` | +1−1 / +3−3 | clean | yes | — |
| 1b | `fe1` capacity | move `fe1` in its view | `elements/fe1.toml` / `layouts/kinesis-data-streams.toml` | +1−1 / +2−2 | clean | yes | — |
| 2 | `fe1/peer_threads` capacity | `fe1/receive_info` post weight | `elements/fe1.toml` / same | +1−1 / +1−1 | clean | yes | — |
| 2b | last key of place `peer_threads` (`when_full`) | first key of the next table (`open_peer_thread` `port`, line removed) | `elements/fe1.toml` / same | +1−1 / +0−1 | clean | yes | — (blank line and header separate them) |
| 2c | add `initial = 1` to the end of a place | edit the next transition | `elements/fe1.toml` / same | +1−0 / +1−1 | clean | yes | — |
| 2d | `fe1` capacity | new relationship from `fe1` | `elements/fe1.toml` / same | +1−1 / +10−0 | clean | yes | — |
| **2e** | edit the **last line of the file** (`receive_info` post) | new relationship from `fe1`, **appended at the end** | `elements/fe1.toml` / same | +1−1 / +10−0 | **conflict** (1 hunk) | — | **End of file.** An append has no unchanged line between it and the last value line, so it is adjacent to an edit of that line. Mid-file insertions do not have this problem (2d, 5e): git's indent heuristic places the inserted block after a blank line, and that blank line stays unchanged. |
| 3a | new element `audit` with its layout entry (sorts first) | new element `shipping` with its layout entry (sorts last) | `elements/audit.toml`, `layouts/kinesis-data-streams.toml` / `elements/shipping.toml`, same layout | +16−0 / +16−0 | clean | yes | — (`arch.toml` lists no elements, FF §5) |
| **3b** | new element `audit` | new element `billing`; both layout entries sort before `fe-peers` | as 3a | +16−0 / +16−0 | **conflict** in `layouts/kinesis-data-streams.toml` (1 hunk) | — | **Same insertion point in a shared file.** The view's layout file is shared by every child of the view (FF §7). FF §15 S6b found the same. |
| **3c** | new element `audit`, called by `fe1` | new element `shipping`, called by `fe1` | adds `elements/fe1.toml` on both sides | +26−0 / +26−0 | **conflict** in `elements/fe1.toml` (1 hunk) | — | **Same insertion point in a shared file.** Both relationships live in the source's file (FF §6.7), and both IDs (`fe1-audit`, `fe1-shipping`) sort after every existing table. FF §15 S5b found the same. |
| 4 | rename `fe1` ("Front-end server (primary)") | new relationship `fe-peers-fe1` **to** `fe1`, with bend points | `elements/fe1.toml` / `elements/fe-peers.toml`, `layouts/kinesis-data-streams.toml` | +1−1 / +13−0 | clean | yes | — (a rename changes only `name`; the ID `fe1` that the new relationship names stays, D-12, FF §4.4) |
| 4b | rename `fe1` | new relationship **from** `fe1` (same file) | `elements/fe1.toml` / same | +1−1 / +10−0 | clean | yes | — |
| 5a | move `fe-peers` | move `fe1` (the adjacent entry) | `layouts/kinesis-data-streams.toml` / same | +2−2 / +2−2 | clean | yes | — |
| 5b | move `fe1` in x | move `fe1` in y (control: same entity, adjacent lines) | same file | +1−1 / +1−1 | conflict (expected) | — | Real conflict on one entity (FF §15 S2) |
| 5c | resize `fe-peers` (adds `w`, `h`) | move `fe1` | same file | +2−0 / +2−2 | clean | yes | — |
| 5d | move `membership` (last element entry) | bend points of `membership-updates` (next table) | same file | +2−2 / +1−1 | clean | yes | — |
| 5e | move `fe1` | place `fe2`, which sorts right after `fe1` | same file | +2−2 / +4−0 | clean | yes | — |
| **5f** | move `users`, the **last entry** of `layouts/cognito.toml` | place `webhooks`, which sorts after it (appended) | `layouts/cognito.toml` / same | +2−2 / +4−0 | **conflict** (1 hunk) | — | **End of file**, as 2e |
| 5g | move place `fe1/map_entries` | move place `fe1/peer_threads` (behavior layout) | `layouts/fe1.toml` / same | +2−2 / +2−2 | clean | yes | — |

**Layout half of the criterion: pass.**

- Every layout gesture changed only the one layout file of its view: moves, a resize, bend points, placing an element, behavior-layout moves. No `elements/*.toml` or `arch.toml` changed.
- No model edit touched a layout file. Creating an element (3a–3c) also writes its layout entry, as FF §7 specifies.
- Diffs are small and readable: one changed value is one line; a move is two lines; a new relationship with one binding pair is 10 lines; a new element with its layout entry is 16 lines.

**Disjoint-edit half: fail.** 2e, 3b, 3c and 5f are disjoint edits (different entities), and git reports a conflict for each. Two of the causes were already in FF §15 (S5b, S6b). The end-of-file case is new: FF §15 tested two appends at the end, not an edit of the last line together with an append.

**"Keep both sides" is not always correct.** In 3b, git factored the line shared by both sides (`y = 320`) out of the conflict hunk:

```text
<<<<<<< HEAD
[elements.audit]
x = 0
=======
[elements.billing]
x = 320
>>>>>>> b
y = 320
```

Keeping both sides of that hunk leaves `audit` without a `y`. The layout file then does not load (FF §3.3), and every position in the view becomes unplaced. It is never a wrong verdict, but FF §15's advice "keep both blocks" is wrong here: the user has to rebuild both blocks.

## Results with the candidate fixes

The same 19 scenarios (18 disjoint plus the 5b control) on four variants of the format. `.` = clean and correct, `C` = conflict, `*` = expected conflict (same key of the same entity).

| Variant | 1 | 1b | 2 | 2b | 2c | 2d | 2e | 3a | 3b | 3c | 4 | 4b | 5a | 5b | 5c | 5d | 5e | 5f | 5g |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| FF format 1 | . | . | . | . | . | . | C | . | C | C | . | . | . | C* | . | . | . | C | . |
| + empty last line | . | . | . | . | . | . | . | . | C | C | . | . | . | C* | . | . | . | . | . |
| + layout file per element | . | . | . | . | . | . | C | . | . | C | . | . | . | C* | . | . | . | . | . |
| + both | . | . | . | . | . | . | . | . | . | C | . | . | . | C* | . | . | . | . | . |

In every variant, every clean merge was byte-identical to the writer's output for both edits, and every layout gesture touched only `layouts/`. In "+ both", the file count of the mock goes from 16 to 20 (5 → 9 layout files) and the line count from 333 to 357.

## Recommendations (for `docs/specs/file-format.md`; not applied here)

### R1. One layout file per element, not per view (the split change)

Change FF §7, and with it §2.1, §2.2, §2.3, §4.2 (the View row), §4.4 (the re-parent row), §9.1, §11.1 rule 2, the §14 layout files, §15 and §16.

`layouts/<element-id>.toml` holds everything about where **this** element and its inside are drawn:

```toml
# layouts/membership.toml
format = 1

[position]                            # where membership is drawn in its parent's view (the landscape when it has no parent)
x = 0
y = 0

[relationships.membership-updates]    # bend points of membership's outgoing relationships (the source's file, as in §6.7)
points = [[160, 40], [320, 40]]
```

`[behavior.*]` stays as it is in the element's file (for example `layouts/fe1.toml`). `layouts/_landscape.toml` goes away. A view is drawn from its children's layout files.

Why:

- A layout entry gets the same ownership as everything else in the format: one element, one file. Moving, resizing or placing two different elements touches two different files, so it merges clean wherever the IDs sort (3b, 5a, 5e, 5f).
- Adding elements to a view never touches a shared file (3b).
- Bend points follow the source, like the relationship table (FF §6.7), so a reconnected source moves both together.
- Simpler gestures:
  - Re-parenting deletes `[position]` in the element's own file (it becomes unplaced). No entry moves between two view files.
  - Deleting an element deletes `elements/<id>.toml` and `layouts/<id>.toml`.
  - A layout file whose stem names no element is `orphanLayout`.
- Entity keys (FF §9.1): `layout:<view>/<id>` becomes `layout:<id>` for the position and `layout:<rid>` for bend points, in the source's layout file. `behavior_layout:<id>` is unchanged.

Cost:

- At most one layout file per element. FF §2.3 already expects one per element with children or a behavior layout, so only leaf elements without a behavior layout are added. At the target size that is up to about 2,000 layout files, about 4,000 files in total. **E4 should measure open time at 4,000 files, not 2,000.**
- `[position]` assumes that v1 draws each element in exactly one view (FF §7). If a later version shows one element in several views, it becomes `[position.<view>]`, which is a format bump.

### R2. Every file ends with an empty line (writer rule)

FF §11.1 rule 1 says "A final newline". Change it to: "The text ends with a newline followed by one empty line (`…\n\n`)". FF §11.2 rule 4 then appends a table before that empty line.

- This fixes the end-of-file adjacency (2e, 5f). The scan found it in element files and layout files alike.
- It works for the same reason mid-file insertions already merge (2d, 5e): there is an unchanged blank line between the last value line and the appended block.
- It is cheap, and it is a writer change, which FF §12.4 ties to the format version. So it belongs before the freeze too.
- An editor that trims trailing blank lines removes the guard from that file. The cost is a conflict, never a wrong merge.

### Not a split question: two additions at one sorted position inside one element file (3c)

Two new relationships from the same element, two new places, properties or binding pairs whose IDs sort into the same gap (3c; FF §15 S5b, S13b, S17) still conflict. I recommend **no split change** for this:

- One file per relationship was already rejected in FF §6.7.
- That split would not cover places, properties or binding pairs anyway.
- The fix is the per-entity merge driver of FF §15 and D-19. It is tooling, not format, so it can come after the freeze. Keep the D-19 revisit rule.

Correct FF §15's resolution advice, given the 3b excerpt above: "rebuild both blocks from the two sides", not "keep both".

## Limits

- The FF §11.2 in-place writer (toml_edit 0.25) was not exercised. The mock writes only canonical files, where the two writers give the same bytes. A hand-edited file with comments or an unusual order may diff differently, but only inside that file. Not verified: that toml_edit keeps an empty last line when it appends a table (R2).
- One mock project, one git version (2.43.0), and LF line endings only. CRLF (FF §11.3) and `core.autocrlf` were not tested.
- These are textual merges. Clean but semantically broken merges (FF §15 S8, S15, S20) were not re-run here; the loader catches them (FF §10.5).
- Scenarios 5e and 5f add a layout entry without its element file. That is enough for the textual merge of the layout file, which is the thing under test.
