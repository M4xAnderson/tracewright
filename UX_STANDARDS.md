# UX Standards (UI review rulebook)

As of 2026-11-02. Adapted from the raubritter project. Domain terms follow `GLOSSARY.md`, which wins on any conflict. Rulebook for reviewing the rendered UI from screenshots. Code is reviewed against `CODING_STANDARDS.md`. The reviewer brings general UX heuristics (Nielsen, states, hierarchy); this file holds only what is binding for this project. UI copy is English.

## How to review

1. Read this whole file. Work with a fresh context: whoever built the UI does not review it.
2. Capture the views the diff touches (see Capture). Open at most about 25 images.
3. Check each image against the scenario's intent, then against every rule below whose area it touches.
4. Report each finding with rule ID, must/should, image file, view, state, window size, theme, observation and recommendation. Add `file:line` when you know it.
5. **must** blocks the merge, **should** is a note (at most 5 per review). Do not report taste without a rule. Do not edit files.

Finding format:
```
VRD-2 must · verify-panel__disproved__default__dark.png (Verify panel, disproved, 1440 × 900, dark)
Observation: the panel says "Failed" but offers no way to replay the counterexample.
Recommendation: add "Replay counterexample" next to the verdict.
Code: app/ui/src/features/verify/verdict-panel.tsx:42
```

## Capture

- Playwright against the Vite dev server with the Tauri IPC mocked from seed models. Native-only behavior (menus, file dialogs) is checked by hand and noted in the PR.
- Two window sizes: `min` 1024 × 640 (smallest supported) and `default` 1440 × 900. Light and dark theme, `reducedMotion: 'reduce'`.
- States: `ideal`, `empty`, `error`, `large-model` (≥ 1,000 elements), plus the verdict states `running`, `proved`, `disproved`, `unknown`.
- File name: `<view>__<state>__<size>__<theme>.png`. `.ux/` is gitignored. No pixel-diff baselines in the repo.

## A11Y – Accessibility basics

- **A11Y-1 must** Text contrast at least 4.5:1; borders, edges, ports, focus ring and verdict badges at least 3:1, in light and dark.
- **A11Y-2 must** Every focus stop is visible and not hidden by a panel or toast.
- **A11Y-3 must** The canvas works with the keyboard alone: move between elements, select, drill down, go up, connect, delete, undo. Every drag action also exists as a command (menu, command palette or inspector).
- **A11Y-4 should** With reduced motion, token animations become step-by-step playback without movement.

## WIN – Window and layout

- **WIN-1 must** At 1024 × 640 every view is usable: side panels collapse, nothing is cut off, the canvas keeps at least half the window width.
- **WIN-2 must** Running text (descriptions, finding explanations) is at most about 75 characters per line.
- **WIN-3 must** Nothing works only on hover; hover only adds emphasis or a preview.

## CNV – Canvas and drill-down

- **CNV-1 must** The current level and the path from the top (breadcrumb: System Context › Container › Component › Code) are always visible. Going up is one action away.
- **CNV-2 must** After drilling down and back up, the element you came from is selected and in view.
- **CNV-3 must** Element kinds differ by shape or icon plus label, never by color alone. Cloud provider icons are used unchanged and only for the service they stand for.
- **CNV-4 should** Pan and zoom stay smooth with the `large-model` seed.
- **CNV-5 must** Unsaved changes are visible. A change to the file on disk while it is open is announced and never silently overwritten or discarded.
- **CNV-6 should** Frequent canvas actions have keyboard shortcuts, listed in the command palette. Shortcuts never replace visible commands.

## VRD – Verdicts and findings

The tool never claims a result it did not get.

- **VRD-1 must** Every check shows exactly one verdict, *Proved*, *Disproved* or *Unknown*, in text with an icon, not by color alone. *Unknown* never looks like success. ❌ A green check mark for a check that timed out.
- **VRD-2 must** *Disproved* offers the counterexample: a step-by-step replay on the canvas with the involved elements highlighted, plus the steps as a text list.
- **VRD-3 must** *Unknown* says why (budget exhausted, unsupported construct, missing data, internal error) and what to do next.
- **VRD-4 must** A verdict names the property, the level and elements it covers. A verdict for an older version of the model is marked as stale.
- **VRD-5 must** An anti-pattern finding names the rule, severity, affected elements (selectable on the canvas) and evidence. Suppressed findings stay listed with their reason.
- **VRD-6 should** Long checks show elapsed time and can be cancelled. Cancelling gives *Unknown (cancelled)*.

## TXT – Language

- **TXT-1 must** UI text uses the terms from `GLOSSARY.md` and none of its _Avoid_ words.
- **TXT-2 should** Error messages give the cause and the next step in plain English, never raw Rust errors. ✅ "Could not save elements/order-api.toml: the file is read-only." ❌ "Os { code: 13, kind: PermissionDenied }"
- **TXT-3 should** Math terms get a short plain-language explanation on first use. ✅ "Not bounded: the queue between Order API and Billing can grow without limit."
