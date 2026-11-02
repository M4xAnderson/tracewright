<img src="assets/icon.svg" width="64" height="64" alt="">

# Tracewright

Tracewright is a local desktop tool for designing software architecture you can check. You draw a system across the C4 levels, attach behavior (Petri nets or state machines) to its parts, and Tracewright proves properties such as "never deadlocks" or "this queue never overflows". When it finds a problem, it replays the counterexample step by step on your architecture. When it cannot decide, it says Unknown and why. A separate scan flags architecture anti-patterns.

**Status:** design phase. There is no application code yet; the specs and decisions below are what slice 0 will be built from.

## Where to read

| What | Where |
|---|---|
| Vocabulary | [GLOSSARY.md](GLOSSARY.md) |
| Decisions and reasons | [docs/adr/](docs/adr/) |
| Specs (net semantics, properties, state machines, scan rules, file format) | [docs/specs/](docs/specs/) |
| Software design | [docs/design/design.md](docs/design/design.md) |
| Product: north star, problems, slices | [docs/product/](docs/product/) |
| UX flows and the visual baseline | [docs/ux/](docs/ux/) |
| Code and UI review rules | [CODING_STANDARDS.md](CODING_STANDARDS.md), [UX_STANDARDS.md](UX_STANDARDS.md) |
| Research record | [docs/research/](docs/research/) |

## Stack

Rust core, Tauri 2, React with React Flow and elkjs. Runs locally: no accounts, no cloud backend, no telemetry. Linux and Windows first, macOS where cheap.

## License

[MIT](LICENSE)
