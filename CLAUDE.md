# Tracewright

Tracewright is a local desktop tool for modeling software architecture across C4 drill-down levels. It proves properties of behavior models (Petri nets, state machines) and scans the architecture for anti-patterns. Status: concept phase, no application code yet.

## Language

Everything is English (US spelling): docs, code, comments, UI copy, commits. Domain terms follow `GLOSSARY.md` exactly.

## Decisions

Already decided; do not reopen. Reasons are in `docs/adr/`.

- Purpose: a daily-use tool for one developer. v1 succeeds when it finds a real deadlock or overflow in a design the user drew. A wrong Proved must never happen (ADR 0008). Product docs: `docs/product/`.
- Local only: no accounts, no cloud backend, no telemetry.
- Stack: Rust core, Tauri 2, React (React Flow, elkjs, shadcn/ui on Base UI, Tailwind v4, Zustand, ts-rs, bundled IBM Plex Sans and Mono). Visual baseline: `docs/ux/mockups/option-2b-drafting-instrument.html`. Software design: `docs/design/design.md` (Rust-owned model snapshots, hexagonal ports, UI as a projection). Workspace layout: ADR 0004, 0009.
- Platforms: Linux and Windows; macOS where cheap (unsigned CI builds until there is a distribution).
- v1 is model-only. Code import, conformance and code proofs come later together (`docs/research/theorem-proving.md`).
- Behavior: Petri nets and state machines, combined through ports; queues are bags in v1 (ADR 0002, 0003).
- Verdicts: Proved, Disproved or Unknown, only with evidence checked by independent code (ADR 0008, 0010); never persisted (ADR 0005, 0013). The scan produces findings, never verdicts.
- Full places: block, drop or overflow; generated places overflow by default (ADR 0014).
- Files: a TOML directory per project, explicit Save, recovery journal (ADR 0001).
- No bundled cloud provider icons (ADR 0007). Targets: 300 elements per view, 2,000 per project.

## Where to look

- Code rules and review: `CODING_STANDARDS.md`. UI review: `UX_STANDARDS.md`.
- Research record: `docs/research/`. Later decisions here, in `GLOSSARY.md` and in `docs/adr/` win over it.

## Repositories

- Code, docs and PRs: `M4xAnderson/tracewright` (public). Issues and tickets: `M4xAnderson/tracewright-planning` (private). Never put issue contents into the public repo; PRs close tickets with `Closes M4xAnderson/tracewright-planning#<n>`.
- The local folder is still named `petrinet`; the product, crates (`tracewright-*`) and app are Tracewright.

## Working

- Work on a feature branch with a PR, never directly on `main`.
- Keep this file under 200 lines and limited to what every session needs. Put reasons in an ADR, terms in `GLOSSARY.md`, rules in the standards files, and link them from here.
