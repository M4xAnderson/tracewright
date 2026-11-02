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

## Develop

Prerequisites (Linux): rustup (the toolchain comes from `rust-toolchain.toml`), Node from `.node-version` (for example through fnm), Bun 1.4.4 (pinned in `package.json`), and `libwebkit2gtk-4.1-dev`, `build-essential` and `libssl-dev`. The smoke test needs a display or `xvfb-run`.

```sh
bun install                 # UI and tool dependencies
cd app/ui && bun run tauri dev   # the app with hot reload
tools/check.sh              # every CI check: rust, arch, ui, smoke
tools/check.sh rust ui      # or only some steps
```

Layout: Rust core crates in `crates/` (ADR 0004, 0009), the Tauri shell in `app/src-tauri`, the React UI in `app/ui`, and the architecture checks in `tools/arch`. CI (`.github/workflows/ci.yml`) runs each step of `tools/check.sh` as its own job.

## License

[MIT](LICENSE)
