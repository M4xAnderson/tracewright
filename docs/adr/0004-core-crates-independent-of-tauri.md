# Core crates do not depend on Tauri or tokio, and the checker knows nothing about C4

The Cargo workspace is split into `crates/model` (domain types, no IO), `crates/format` (TOML read and write, migrations, recovery journal), `crates/verify` (Petri net exploration, invariants, counterexamples, independent replay), `crates/scan` (rules), `crates/engine` (lowering the model to nets, job registry with cancellation, undo history) and `app/src-tauri` (thin command layer), with the React UI in `app/ui`. `verify` takes a plain Petri net and depends on no other workspace crate; `scan` depends only on `model`. Core crates use std threads and an atomic cancel flag; tokio stays in `app/src-tauri`. This keeps the checker testable headless, lets a CLI sit on `engine` later, and makes it possible to cross-check `verify` against external tools on plain PNML. CI enforces the boundaries with a dependency check.

Amended by ADR 0009.
