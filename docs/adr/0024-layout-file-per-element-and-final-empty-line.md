# One layout file per element, and every file ends with an empty line

Amends ADR 0001 and ADR 0020. Layout is stored in **one file per element**, `layouts/<element-id>.toml`, instead of one file per view. That file holds `[position]`, where the element is drawn in its parent's view; the bend points of the element's outgoing relationships, under `[relationships.<id>]` (the source's file, as for the relationship itself); and the element's behavior-model layout under `[behavior.*]`, as before. A view is drawn from the layout files of its children. A top-level element's `[position]` is its place in the landscape, so `layouts/_landscape.toml` goes away and the landscape has no file of its own. A layout-only Save still touches only `layouts/`; creating an element writes its first position into its own layout file; deleting an element deletes its element file and its layout file. **Every file the writer produces ends with a newline followed by one empty line**, and the in-place writer keeps that line last when it appends a table or a key. Spike E2 (`docs/product/experiments/e2-toml-merge.md`) merged 18 disjoint edit pairs with git on format 1 as first specified: 4 conflicted, two because an append at the end of a file sat next to an edit of the last line (2e, 5f), and two because two additions met at one sorted position in a shared file (3b in a view's layout file, 3c in an element file). With both changes, 17 of 18 merge clean, and every clean merge was byte-identical to the writer's output for both edits. The one left, two new relationships from the same source (3c), stays a conflict until the per-entity merge driver that D-19 deferred; it gets no further split. Format 1 is not frozen yet (`docs/specs/file-format.md` §12.5), so neither change needs a migration.

## Considered Options

- **Keep one layout file per view:** every child of a view shares the file, so two elements added to one view conflict (E2 3b), and resolving that conflict by keeping both sides can drop a line both sides share.
- **Only the empty last line:** fixes the end-of-file conflicts (2e, 5f) but not two elements added to one view (3b).
- **Also one file per relationship, to fix 3c:** already rejected in file-format §6.7 (about 4,000 more files, binding pairs far from both nets), and it would not cover places, properties or binding pairs that collide in the same way. The merge driver fixes all of them without a format change.
- **`[position.<view>]` keyed by view:** v1 draws each element in exactly one view, so the extra key buys nothing yet.

## Consequences

- At most one layout file per element: up to about 4,000 project files at the 2,000-element target. Slice 0 measures project open time at 4,000 files (`docs/product/v1-slices.md`, slice 0).
- `[position]` assumes one view per element. Showing an element in several views later needs `[position.<view>]`, a format bump with a migration.
- An editor that trims trailing blank lines removes the guard from a file until the app next writes it; the cost is a conflict, never a wrong merge.
- E2 did not run the in-place writer (toml_edit). That it keeps the empty last line when it appends a table is a slice 0 test.
- An insertion conflict is resolved by rebuilding both blocks from the two sides, not by keeping both sides of the hunk: git can move a line both sides share out of the hunk (E2 3b). A layout file that then does not load leaves positions unplaced, never a wrong verdict.
- `docs/specs/file-format.md` revision 4 (§0, §2, §3, §4, §7, §9, §10, §11, §13–§17, §19), `docs/design/design.md` §3 and §7, `docs/product/v1-slices.md` slice 0, `CODING_STANDARDS.md` FMT-3 and `docs/specs/decisions.md` D-70 carry the rules.
