# Issue tracker: GitHub (private planning repo)

Issues, specs and tickets for Tracewright live as GitHub issues in the **private** repo `M4xAnderson/tracewright-planning`, not in this public code repo. Use the `gh` CLI for all operations.

## Which repo

- `git remote` points at the public `M4xAnderson/tracewright`, so never let `gh` infer the repo. Pass `-R M4xAnderson/tracewright-planning` on every `gh issue` and `gh label` command.
- Code, docs and pull requests stay in `M4xAnderson/tracewright`.

## Privacy

This file is public; the issues are not. Never copy issue contents (titles, bodies, comments) into this repo: not into commits, PR bodies, code comments or docs. A PR refers to a ticket only by its reference, `Closes M4xAnderson/tracewright-planning#<n>`, which closes it on merge.

## Conventions

Below, `R=M4xAnderson/tracewright-planning`.

- **Create an issue**: `gh issue create -R $R --title "..." --body-file <file>`. Write multi-line bodies to a file under `$TMPDIR` first.
- **Read an issue**: `gh issue view <n> -R $R --json number,title,body,labels,comments`.
- **List issues**: `gh issue list -R $R --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Make an issue a sub-issue of a parent**: `gh issue create -R $R --parent <parent> ...`, or `gh issue edit <parent> -R $R --add-sub-issue <child>` afterwards.
- **Blocking**: GitHub's native issue dependencies. Add an edge with `gh issue create -R $R --blocked-by <n>,<n> ...` or `gh issue edit <child> -R $R --add-blocked-by <blocker>`. A ticket is unblocked when every blocker is closed.
- **Count open blockers**: `gh issue view <n> -R $R --json blockedBy --jq '[.blockedBy.nodes[] | select(.state == "OPEN")] | length'` (0 means startable).
- **Comment on an issue**: `gh issue comment <n> -R $R --body-file <file>`.
- **Apply / remove labels**: `gh issue edit <n> -R $R --add-label "..."` / `--remove-label "..."`.
- **Close**: post the explanation with `gh issue comment <n> -R $R --body "..."`, then `gh issue close <n> -R $R`. On an already-closed issue, `gh issue close --comment` drops the comment and still exits 0. Work done through a PR closes its ticket on merge via the `Closes` line instead.

`gh api` is left to the user; every operation above uses plain `gh` subcommands (`gh` 2.94 or later).

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

PRs live in `M4xAnderson/tracewright`, issues in `M4xAnderson/tracewright-planning`, so a bare `#42` is ambiguous: always write the full `owner/repo#n` reference.

## When a skill says "publish to the issue tracker"

Create a GitHub issue in `M4xAnderson/tracewright-planning`.

## When a skill says "fetch the relevant ticket"

Read it as in **Read an issue** above.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets. All commands take `-R M4xAnderson/tracewright-planning`.

- **Map**: a single issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body. `gh issue create -R $R --label wayfinder:map`.
- **Child ticket**: an issue linked to the map as a GitHub sub-issue (see **Make an issue a sub-issue of a parent**). Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev.
- **Blocking**: see **Blocking** under Conventions.
- **Frontier query**: list the map's open sub-issues, drop any with an open blocker (see **Count open blockers**) or an assignee; first in map order wins.
- **Claim**: `gh issue edit <n> -R $R --add-assignee @me`, the session's first write.
- **Resolve**: `gh issue comment <n> -R $R --body "<answer>"`, then `gh issue close <n> -R $R`, then append a context pointer (gist + link) to the map's Decisions-so-far.
