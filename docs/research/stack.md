# Research: application stack, canvas and file format

> Terms and decisions made after this research (GLOSSARY.md, CLAUDE.md, docs/adr/) take precedence over this file. It is kept as the research record.

As of 2026-11-02. Scope: local, single-user desktop app (Rust core + Tauri 2 + React) for C4 drill-down modeling with Petri-net / state-machine behavior models, verification in Rust, animated token traces, cloud icon palette, text files in git.

## Summary

- Tauri 2.12.5 is the stable line; 3.0 is alpha. Channels (not events) carry streamed verification data. Tauri has no built-in cancellation, so we build it.
- tauri-specta/specta 2.0 are still release candidates; ts-rs 12 is the stable fallback.
- React Flow 12 (MIT, no gated library features) is the best canvas fit. tldraw needs a license key (or a watermark under a hobby license), GoJS is commercial, and JointJS (MPL-2.0) is the strongest runner-up.
- ELK (elkjs, EPL-2.0) is the only layout engine that handles compound nodes, ports, orthogonal routing and labels together. No Rust-native crate matches it.
- Use our own format: a directory of TOML files, one file per element, with layout in separate files. Interop comes through importers and exporters (Structurizr, LikeC4, PlantUML/Mermaid, PNML, draw.io).
- The cloud vendors' icon terms do not clearly allow bundling the icons in software. Prefer letting users import the official ZIP files.
- shadcn/ui now defaults to Base UI (stable since Dec 2025). Use it with Tailwind v4 and a cmdk command palette.

Method: primary sources only (official docs, GitHub repos, license files, changelogs). Package versions and dates come from the npm registry (`https://registry.npmjs.org/<pkg>`) and the crates.io API (`https://crates.io/api/v1/crates/<crate>`), both queried on 2026-11-02; links below point to the human-readable package pages. Items marked **UNVERIFIED** could not be confirmed from a primary source.

---

## 1. Tauri 2

### 1.1 Version and release state

| Item | State | Source |
|---|---|---|
| `tauri` crate | 2.12.5 stable (2026-10-21) | [crates.io/crates/tauri](https://crates.io/crates/tauri), [releases](https://github.com/tauri-apps/tauri/releases) |
| Tauri 3 | `3.0.0-alpha.5` pre-release (14 Oct); 2.x still receives releases | [releases](https://github.com/tauri-apps/tauri/releases) |
| `@tauri-apps/cli` / `@tauri-apps/api` | 2.12.4 / 2.12.4 | [npm cli](https://www.npmjs.com/package/@tauri-apps/cli), [npm api](https://www.npmjs.com/package/@tauri-apps/api) |
| `tauri-plugin-fs` | 2.7.1 (3.0.0-alpha.3 exists) | [crates.io](https://crates.io/crates/tauri-plugin-fs) |

Implication: build on 2.x; a 3.0 alpha is in flight, so isolate Tauri-specific code in a thin adapter crate (see section 7) to keep a later migration cheap.

### 1.2 IPC: commands, channels, events

| Mechanism | Use it for | Notes | Source |
|---|---|---|---|
| Commands (`#[tauri::command]`) | Request/response (load model, save, run verification) | `async` commands run via `async_runtime::spawn`; non-async run on the main thread unless `#[tauri::command(async)]`. Borrowed args (`&str`, `State<'_,T>`) need owned types/`Result` in async. Errors must be `Serialize`; a tagged `thiserror` enum is the documented pattern. | [Calling Rust](https://v2.tauri.app/develop/calling-rust/) |
| Raw payloads | Large binary data | Return `tauri::ipc::Response` (ArrayBuffer) instead of JSON; JSON serialization "is slow for large payloads". Request side: `tauri::ipc::Request`. | [Calling Rust](https://v2.tauri.app/develop/calling-rust/) |
| Channels (`tauri::ipc::Channel<T>`) | Streaming progress, verification results, token-trace frames | "designed to be fast and deliver ordered data"; Tauri uses them for downloads and child-process output. Passed as a command argument. | [Calling the frontend](https://v2.tauri.app/develop/calling-frontend/) |
| Events (`emit`, `emit_to`, `listen`) | Broadcast notifications (e.g. "file changed on disk") | No strong typing on the frontend, JSON-string payloads, "not designed for low latency or high throughput", async listeners may see events out of order. | [Calling the frontend](https://v2.tauri.app/develop/calling-frontend/) |

### 1.3 Type-safe bindings

| Option | State | Fit | Source |
|---|---|---|---|
| **tauri-specta v2** | Still **release candidate**: crates.io max `2.0.0-rc.26` (2026-10-21); GitHub releases all pre-release. Generates TS for commands **and events**. MIT. | Best end-to-end typing (command signatures + event payloads). Pin an exact RC. Channel typing: **UNVERIFIED** in README. | [repo](https://github.com/specta-rs/tauri-specta), [releases](https://github.com/specta-rs/tauri-specta/releases), [crates.io](https://crates.io/crates/tauri-specta) |
| **specta** | `2.0.0-rc.27` (21 Oct); rc.25 notes call it the last before stable, still no 2.0.0 | Underlying type-export library for tauri-specta | [releases](https://github.com/specta-rs/specta/releases) |
| **ts-rs** | 12.0.1 stable (2026-01-31), MIT, ~21M downloads. `#[derive(TS)]` + `#[ts(export)]` writes `.ts` on `cargo test`; `serde-compat` honors rename/tag/flatten/skip etc. | Types only (no command wrappers). Ideal for keeping the core crate free of Tauri; command wrappers then hand-written. | [repo](https://github.com/Aleph-Alpha/ts-rs), [crates.io](https://crates.io/crates/ts-rs) |

### 1.4 Long-running background work and cancellation

- Tauri docs cover async commands and channels but **not cancellation** ([Calling Rust](https://v2.tauri.app/develop/calling-rust/)). Pattern to build ourselves: `start_verification(model_ref, on_event: Channel<VerifyEvent>) -> JobId`, store a `tokio_util::sync::CancellationToken` per job in managed `State`, and expose `cancel_job(JobId)`. The token supports `cancel()`, `cancelled()` future, `child_token()`, `is_cancelled()` ([docs.rs](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html)).
- CPU-bound state-space exploration should run on a blocking thread / worker pool and poll a cancel flag; the **core crate should depend on a tiny cancel trait or `AtomicBool`, not on tokio**, so it also compiles to WASM and works in a CLI (design recommendation, not a Tauri requirement).

### 1.5 Filesystem watching

- `tauri-plugin-fs` offers `watch` (debounced, `delayMs`) and `watchImmediate`, non-recursive by default, behind the `watch` Cargo feature, and requires an `allow` path scope in a capability ([fs plugin](https://v2.tauri.app/plugin/file-system/)).
- For a model repo the watcher belongs in Rust (the core must reconcile disk changes from `git checkout`/merges itself). Use `notify` 8.2.1 (9.0 in RC) + `notify-debouncer-full` ([crates.io notify](https://crates.io/crates/notify), [debouncer](https://crates.io/crates/notify-debouncer-full)). Known problems documented by notify: network mounts may emit no events, "especially" WSL programs watching Windows paths (use `PollWatcher`); editors save via truncate or rename-replace; inotify `max_user_watches` limits ([notify docs](https://docs.rs/notify/latest/notify/)).

### 1.6 Plugin ecosystem

Official `plugins-workspace` (MIT/Apache-2.0) has 31 plugins incl. `fs`, `dialog`, `store`, `window-state`, `single-instance`, `updater`, `log`, `cli`, `opener`, `clipboard-manager`, `global-shortcut`, `deep-link`, `process`, `shell` ([repo](https://github.com/tauri-apps/plugins-workspace)). Relevant for us: `dialog` (open repo folder), `window-state`, `single-instance`, `log`, `opener`, `updater` (optional), `clipboard-manager`.

### 1.7 Security model (capabilities)

- Capabilities (JSON/TOML in `src-tauri/capabilities`) map **permissions** (e.g. `core:window:allow-set-title`) to windows/webviews by **label**; can be restricted per platform; remote URLs get no API access unless listed in `remote.urls` ([Capabilities](https://v2.tauri.app/security/capabilities/)).
- All registered app commands are callable from every window by default; `AppManifest::commands` in `build.rs` restricts this ([Capabilities](https://v2.tauri.app/security/capabilities/)).
- Explicit non-goals: does not protect against insecure Rust code, overly broad scopes, WebView 0-days, supply chain ([Capabilities](https://v2.tauri.app/security/capabilities/)).
- For us: single main window, no remote content, no `shell` plugin, fs access only through our own commands that validate paths against the opened repo root.

### 1.8 Testing

| Layer | Tool | Source |
|---|---|---|
| Rust app glue | `tauri::test` (feature `test`, marked unstable): `mock_builder`, `mock_app`, `get_ipc_response`, `assert_ipc_response`, `MockRuntime` | [docs.rs tauri::test](https://docs.rs/tauri/latest/tauri/test/index.html) |
| Frontend unit | `@tauri-apps/api/mocks`: `mockIPC` (Vitest spies; `shouldMockEvents` since 2.7.0, no `emitTo`), `mockWindows`, `clearMocks` | [Mocking](https://v2.tauri.app/develop/tests/mocking/) |
| E2E | `@wdio/tauri-service` (embedded WebDriver server; works on Windows, Linux **and macOS**); plain `tauri-driver` only Windows/Linux because macOS has no WKWebView driver; CrabNebula fork needs a paid key on macOS | [WebDriver](https://v2.tauri.app/develop/tests/webdriver/), [WebdriverIO Tauri](https://webdriver.io/docs/desktop-testing/tauri) |
| CI | `tauri-action` for GitHub Actions | [Tests](https://v2.tauri.app/develop/tests/) |

The biggest testing lever is architectural: the core crate (model, file format, verification) is tested with plain `cargo test` without any webview.

### 1.9 Bundling

| Platform | Formats | Notes | Source |
|---|---|---|---|
| Linux | AppImage, .deb, .rpm; Flatpak, Snap, AUR documented | Needs `libwebkit2gtk-4.1-dev` etc. to build | [Distribute](https://v2.tauri.app/distribute/), [Prerequisites](https://v2.tauri.app/start/prerequisites/) |
| Windows | MSI (WiX), NSIS `-setup.exe` | MSI "can only be created on Windows"; NSIS cross-compile "possible with caveats", "last resort". WebView2 modes: downloadBootstrapper (default), embedBootstrapper, offlineInstaller, fixedVersion (+~180 MB), skip | [Windows installer](https://v2.tauri.app/distribute/windows-installer/) |
| macOS | .app, .dmg, App Store | Notarization required outside App Store | [Distribute](https://v2.tauri.app/distribute/) |

### 1.10 WSL2 development caveats (this machine is WSL2)

- `tauri dev` inside WSL builds and runs a **Linux** app rendered by **WebKitGTK** via WSLg, not the WebView2 build Windows users get ([Prerequisites](https://v2.tauri.app/start/prerequisites/) lists WebView2 for Windows, WebKitGTK deps for Linux). Rendering/perf bugs must be checked on real Windows too.
- WebKitGTK graphics issues (blank window, DMABUF errors, resize crashes) have documented env-var workarounds: `WEBKIT_DISABLE_DMABUF_RENDERER=1`, `WEBKIT_DISABLE_COMPOSITING_MODE=1`; the page warns WebGL/canvas can silently fall onto a slow path on Linux. The page does not mention WSL ([Linux graphics](https://v2.tauri.app/develop/debug/linux-graphics/)). Whether WSLg needs these is **UNVERIFIED**; test early.
- Keep the repo on the Linux filesystem (`/home/...`), not `/mnt/c` ([Microsoft: WSL filesystems](https://learn.microsoft.com/en-us/windows/wsl/filesystems)); file watching on Windows paths from WSL may produce no events ([notify](https://docs.rs/notify/latest/notify/)).
- Windows installers: MSI needs a Windows host ([Windows installer](https://v2.tauri.app/distribute/windows-installer/)) -> use CI (`tauri-action`) with a Windows runner.

---

## 2. Graph/diagram canvas for React

### 2.1 Overview

| Library | Version (npm) | License | Model | Source |
|---|---|---|---|---|
| **React Flow** (`@xyflow/react`) | 12.12.0 (2026-09-24) | MIT; Pro = paid examples/templates/support, **no library features gated** | DOM/SVG, React components as nodes | [npm](https://www.npmjs.com/package/@xyflow/react), [Pro](https://reactflow.dev/pro) |
| **tldraw** | 5.6.0 (2026-10-28) | **Proprietary "tldraw license"**: production needs a license key; hobby (non-commercial, discretionary) requires "made with tldraw" watermark; OSS redistributors' users need their own license | Infinite-canvas whiteboard SDK | [license](https://tldraw.dev/community/license), [LICENSE.md](https://github.com/tldraw/tldraw/blob/main/LICENSE.md), [pricing](https://tldraw.dev/pricing) |
| **JointJS** (`@joint/core`, `@joint/react`) | 4.3.4 / 4.3.7 (2026-10-27) | Core MPL-2.0; JointJS+ commercial ($289/mo over year 1 = $3,468 perpetual per dev) | SVG, MVC; React package | [license](https://www.jointjs.com/license), [pricing](https://www.jointjs.com/pricing), [releases](https://github.com/clientIO/joint/releases) |
| **Rete.js** | 2.0.6 (2025-06-30) | MIT | Visual-programming node editor (dataflow/control-flow) | [repo](https://github.com/retejs/rete), [npm](https://www.npmjs.com/package/rete) |
| **Cytoscape.js** | 3.34.4 (2026-10-15) | MIT | Graph analysis + canvas renderer; WebGL renderer preview since 3.31 (2025-01-13) | [repo](https://github.com/cytoscape/cytoscape.js), [blog](https://blog.js.cytoscape.org/) |
| **Sigma.js** | 4.0.4 (2026-10-27); `@react-sigma/core` 6.0.0 | MIT | WebGL, "thousands of nodes", graphology | [repo](https://github.com/jacomyal/sigma.js), [npm](https://www.npmjs.com/package/@react-sigma/core) |
| **GoJS** | 4.0.5 (2026-10-02) | Commercial: Individual $3,995, Team $6,990 (3 devs); no redistribution of the library | Canvas/SVG diagramming | [sales](https://nwoods.com/sales/index.html), [license](https://gojs.net/latest/license.html) |
| **maxGraph** (`@maxgraph/core`) | 0.25.1 (2026-10-21), pre-1.0 | Apache-2.0; TS successor of archived mxGraph | SVG | [repo](https://github.com/maxGraph/maxGraph), [releases](https://github.com/maxGraph/maxGraph/releases) |
| **draw.io** (app) | n/a | Apache-2.0 code; icon/stencil libraries under separate terms; no PRs accepted | Full app, not a library | [repo](https://github.com/jgraph/drawio) |
| **Excalidraw** | 0.18.1 (2026-04-20) | MIT | Hand-drawn whiteboard | [repo](https://github.com/excalidraw/excalidraw), [npm](https://www.npmjs.com/package/@excalidraw/excalidraw) |

### 2.2 Comparison against our requirements

| Criterion | React Flow | JointJS (OSS) | tldraw | Cytoscape / Sigma | maxGraph | Rete | GoJS | Excalidraw |
|---|---|---|---|---|---|---|---|---|
| Nesting / drill-down | `parentId`, `extent:'parent'`, any node can be parent; parents must precede children in array ([sub flows](https://reactflow.dev/learn/layouting/sub-flows)). Drill-down = switch view (our own state). | Embedding (cells in cells); **UNVERIFIED** detail | Frames/groups | Cytoscape compound nodes (**UNVERIFIED** in README); Sigma none | Groups/swimlanes (mxGraph heritage) | Scopes plugin (**UNVERIFIED**) | Groups, subgraphs | Frames |
| Edge routing | Built-in bezier/step/smoothstep; obstacle-avoiding "Edge Routing" with libavoid is a **Pro example** ([examples](https://reactflow.dev/examples)) | Built-in `manhattan`/`metro` avoid obstacles; `rightAngle`; `@joint/router-avoid` (libavoid) ([routers](https://docs.jointjs.com/api/routers/)) | Arrows, no orthogonal router | Straight/bezier/taxi | Orthogonal edge styles | Curves | Rich (AvoidsNodes) | None |
| Auto layout | dagre/ELK free examples; "Auto Layout", "Expand and Collapse" are Pro examples ([examples](https://reactflow.dev/examples)) | `@joint/layout-directed-graph`, `@joint/layout-elk`, `@joint/layout-msagl` ([packages](https://github.com/clientIO/joint/tree/master/packages)) | none | many plugins | built-in | arrange plugin | built-in | none |
| Custom node rendering | Any React component | SVG markup / `@joint/react` | Custom shapes (React) | Style sheets, limited | HTML labels | React/Vue | Templates | none |
| Token animation along edges | Free example: SVG `<animateMotion>` and Web Animations `offsetPath` ([animating edges](https://reactflow.dev/examples/edges/animating-edges)) | Link `getPointAtLength`, token examples **UNVERIFIED** | possible | hard | manual | manual | built-in animation | no |
| Accessibility | Keyboard focus/select/move, `ariaLabelConfig`, aria-live announcements ([a11y](https://reactflow.dev/learn/advanced-use/accessibility)) | **UNVERIFIED** | partial | canvas = poor | poor | poor | **UNVERIFIED** | partial |
| 1k–5k nodes | DOM-based; guidance = memoize, avoid reading `nodes` in components, collapse trees, simplify CSS; **no official numbers** ([performance](https://reactflow.dev/learn/advanced-use/performance)) | SVG, similar DOM limits | good (culling) | Sigma/Cytoscape-WebGL best | moderate | low | good | moderate |
| Dark mode | `colorMode` light/dark/system ([v12](https://reactflow.dev/learn/troubleshooting/migrate-to-v12)) | CSS | yes | styles | CSS | CSS | yes | yes |
| Fit for an "architecture editor" | High; LikeC4's diagram package is built on `@xyflow/react` ([npm @likec4/diagram deps](https://www.npmjs.com/package/@likec4/diagram)) | High (diagramming-first) | License blocker | Viewer/analysis, not editor | Viable, pre-1.0 | Wrong domain | License cost | Wrong aesthetic |

Notes:
- 1k–5k nodes **in one view** is unlikely in a C4 drill-down tool (each level shows one scope), but the Code level and big Petri nets can get large. Must be benchmarked in a spike (DOM + WebKitGTK on Linux is the worst case, see 1.10).
- Petri-net views (places, transitions, arcs, token markings) fit React Flow custom nodes + custom edges with `animateMotion`/`offsetPath` tokens.

---

## 3. Automatic layout

| Engine | Version / license | Compound (nested) | Edge routing | Runtime | Source |
|---|---|---|---|---|---|
| **ELK via elkjs** | 0.12.1 (2026-10-28), **EPL-2.0 OR GPL-3.0-or-later** | Yes: `hierarchyHandling = INCLUDE_CHILDREN` lays out a node and all descendants in one run, cross-hierarchy edges supported | Layered: ORTHOGONAL (default), splines, polyline; ports with constraints; edge labels; self loops; model-order options for stable layouts; interactive/semi-interactive options | GWT-compiled JS, Web Worker out of the box | [elkjs](https://github.com/kieler/elkjs), [npm](https://www.npmjs.com/package/elkjs), [Layered](https://eclipse.dev/elk/reference/algorithms/org-eclipse-elk-layered.html), [hierarchyHandling](https://eclipse.dev/elk/reference/options/org-eclipse-elk-hierarchyHandling.html) |
| **dagre** (`@dagrejs/dagre`) | 3.1.3 (2026-10-24), MIT; DagreJs org fork is the maintained one | Clusters (limited; **UNVERIFIED** in README) | Polyline points, no orthogonal | JS | [repo](https://github.com/dagrejs/dagre), [npm](https://www.npmjs.com/package/@dagrejs/dagre) |
| **d3-hierarchy** | 3.1.2 (2022), ISC | Trees only (hierarchical data) | n/a | JS | [repo](https://github.com/d3/d3-hierarchy) |
| **Graphviz (WASM)** | `@hpcc-js/wasm-graphviz` 1.30.0 Apache-2.0 wrapper; `@viz-js/viz` 3.31.0 MIT wrapper | Clusters (dot) | splines/ortho | WASM | [npm hpcc](https://www.npmjs.com/package/@hpcc-js/wasm-graphviz), [npm viz](https://www.npmjs.com/package/@viz-js/viz); LikeC4 uses it ([npm @likec4/layouts deps](https://www.npmjs.com/package/@likec4/layouts)) |
| **MSAGL-JS** | `@msagl/core` 1.1.24, MIT | **UNVERIFIED** from README | **UNVERIFIED** | JS | [repo](https://github.com/microsoft/msagljs) |
| **libavoid (adaptagrams)** | C++, **LGPL-2.1+** (commercial license available) | n/a (router, not layouter) | Object-avoiding orthogonal & polyline connectors for interactive editors (used by Inkscape, Dunnart) | `libavoid-js` 0.5.0-beta.5 WASM (LGPL-2.1) | [adaptagrams](https://github.com/mjwybrow/adaptagrams), [libavoid-js](https://github.com/Aksem/libavoid-js), [npm](https://www.npmjs.com/package/libavoid-js) |
| **Rust: rust-sugiyama** | 0.4.0, MIT; petgraph `StableDiGraph` in, coordinates out | Not mentioned | Not documented | native/WASM | [repo](https://github.com/paddison/rust-sugiyama) |
| **Rust: layout-rs** | 0.1.3, MIT; dot parser + SVG | Not documented | **UNVERIFIED** | native | [repo](https://github.com/nadavrot/layout) |

Findings:
- Only ELK combines **compound layout + ports + orthogonal routing + labels** in one engine. No Rust-native crate found that does compound + orthogonal; Rust layout would mean porting ELK-layered (large).
- License notes: EPL-2.0 is weak (file-level) copyleft; shipping unmodified elkjs inside an app is generally compatible with MIT/Apache app code, but get confirmation (**not legal advice**). libavoid's LGPL in a bundled WASM blob raises relinking/replacement obligations; keep it optional/replaceable.
- Stability matters for diffs and mental maps: use ELK's model-order options and seed from stored positions; run ELK only on explicit "auto-layout" or for new elements, never silently on every edit.

---

## 4. File format and interop

### 4.1 Existing formats

| Format | What it is | Layout | License / tooling | Fit as **native** format | Source |
|---|---|---|---|---|---|
| **C4 model** | Abstractions + 4 hierarchical diagrams (context, container, component, code) + landscape/dynamic/deployment; notation- and tool-independent; site CC BY 4.0. Code level "very much optional", "not recommended for anything but the most important or complex components", ideally generated | n/a | n/a | Conceptual basis, not a format | [c4model.com](https://c4model.com/), [code](https://c4model.com/diagrams/code) |
| **Structurizr DSL** | `workspace { model { person/softwareSystem/container/component, ->, deploymentEnvironment } views { ... } }`, `!include`, `!identifiers hierarchical|flat`, `autoLayout` | Manual layout lives in `workspace.json` ("compiled", "not designed to be edited by hand"); view keys must be stable or layout is lost | Apache-2.0; code moved to `structurizr/structurizr` monorepo (Java), old `structurizr/java` archived 2026-03-28 | No: Java parser, no behavior models, layout in non-hand-editable JSON | [DSL](https://docs.structurizr.com/dsl/language), [file types](https://docs.structurizr.com/workspaces/file-types), [repo](https://github.com/structurizr/structurizr), [archived](https://github.com/structurizr/java) |
| **LikeC4** | `specification` (custom kinds), `model`, `views`, `global`; many `.c4`/`.likec4` files merged into one model; CLI validates incl. "layout drift"; draw.io round-trip | Manual layouts supported (exact storage **UNVERIFIED**) | MIT, TypeScript, v1.60.0; VS Code extension | No as native (TS toolchain, no behavior models), **yes as import/export** target | [DSL intro](https://likec4.dev/dsl/intro/), [repo](https://github.com/likec4/likec4), [CLI](https://likec4.dev/tooling/cli/), [draw.io](https://likec4.dev/tooling/drawio/), [npm](https://www.npmjs.com/package/likec4) |
| **C4-PlantUML** | PlantUML macros: context/landscape, container, component, dynamic, deployment, C4-sequence; `Rel_U/D/L/R`, `Lay_*` hints | Graphviz via PlantUML | MIT | Export only | [repo](https://github.com/plantuml-stdlib/C4-PlantUML) |
| **Mermaid C4** | `C4Context/Container/Component/Dynamic/Deployment`; "does not use a fully automated layout algorithm" (statement order); several features unchecked | weak | MIT, mermaid 12.2.0 | Export only (renders in GitHub/GitLab Markdown) | [docs](https://mermaid.js.org/syntax/c4.html), [npm](https://www.npmjs.com/package/mermaid) |
| **ArchiMate Exchange Format** | Open Group XML/XSD; model, view, diagram exchange schemas; ArchiMate 3.1/3.2; "not intended as a persistent file format" | in view/diagram schema | Standard needs free Open Group login; schema license **UNVERIFIED** | Export only (enterprise users) | [Open Group](https://www.opengroup.org/open-group-archimate-model-exchange-file-format) |
| **PNML (ISO/IEC 15909-2)** | XML transfer syntax (RELAX NG) for P/T nets, symmetric nets, high-level nets; extensible via PNTDs; Part 2 (2011) under revision, 2009 grammar latest, 2026 version in progress | Graphics annotations exist in PNML (**UNVERIFIED** on pnml.org page) | ISO standard; grammars hosted on pnml.org | Import/export for P/T nets (tool interop with Petri-net checkers) | [pnml.org](https://www.pnml.org/) |
| **draw.io XML** | mxGraph-based XML | full geometry | Apache-2.0 app; format spec not documented in repo | Export (and LikeC4 shows round-trip is possible) | [drawio](https://github.com/jgraph/drawio), [LikeC4 draw.io](https://likec4.dev/tooling/drawio/) |

### 4.2 Existing DSL vs own format

Requirements that existing DSLs do not cover: behavior models (Petri net / state machine) attached to any element, Code level, verification settings, token-trace scenarios, and app-owned stable IDs. Structurizr keeps manual layout in a machine JSON; LikeC4 is a TypeScript/VS Code ecosystem we would have to reimplement in Rust. **Recommendation: own format, with importers/exporters for Structurizr DSL, LikeC4, PlantUML/Mermaid C4, PNML, draw.io.**

### 4.3 Serialization syntax candidates (Rust side)

| Syntax | Rust support | Comments / format-preserving edit | Diff/merge | Risks | Source |
|---|---|---|---|---|---|
| **TOML** | `toml` 1.1.8 (spec 1.1.0), `toml_edit` 0.25.17 format-preserving, serde; ~1B downloads | Comments yes; `toml_edit` keeps user formatting | Line-oriented, flat tables merge well | Deep nesting awkward -> use flat elements with `parent` refs | [toml](https://crates.io/crates/toml), [toml_edit](https://crates.io/crates/toml_edit) |
| **KDL 2.0** | `kdl` 6.7.1 (format/comment-preserving, incl. `kdl-lsp`); serde-like `knus` supports **KDL v1 only** | Yes | Node-per-line, good | Small ecosystem, manual mapping code, unfamiliar to users | [kdl.dev](https://kdl.dev/), [crates.io kdl](https://crates.io/crates/kdl) |
| **YAML** | `serde_yaml` is **deprecated** (`0.9.34+deprecated`); successors `serde_saphyr` 2.0.0, `serde_norway`, `serde_yml` | Comments lost on serde round-trip | Indentation conflicts, implicit typing pitfalls | Library churn | [serde_yaml](https://crates.io/crates/serde_yaml), [serde_saphyr](https://crates.io/crates/serde_saphyr) |
| **JSON** | `serde_json` 1.0.154; `schemars` 1.2.3 for JSON Schema | No comments | OK if canonical (sorted keys, one item per line); brace/comma conflicts | Hand-editing unpleasant | [serde_json](https://crates.io/crates/serde_json), [schemars](https://crates.io/crates/schemars) |
| **RON** | `ron` 0.12.3 | Comments yes | OK | Niche outside Rust | [ron](https://crates.io/crates/ron) |
| Own DSL | hand parser + LSP | full control | best readability | Largest effort (parser, formatter, LSP, errors) | (LikeC4/Structurizr show the cost) |

### 4.4 Format design rules (recommendation)

1. **Directory = model.** e.g. `arch/` with `arch.toml` (manifest: format version, specification of element kinds), `elements/<id>.toml` (one file per C4 element: kind, name, description, technology, tags, `parent`, outgoing relations, optional `[behavior]`), `views/<id>.layout.toml` (positions, sizes, bend points, collapsed state). One file per element minimizes merge conflicts; relations live with their source element.
2. **Stable IDs** chosen once and never derived from display names (rename = edit `name`, not file move). Use a human-readable slug plus collision suffix; references are by ID. Avoid Structurizr's problem of unstable view keys losing layout ([Structurizr DSL](https://docs.structurizr.com/dsl/language)).
3. **Layout separate from semantics** (as Structurizr does with `workspace.json`, [file types](https://docs.structurizr.com/workspaces/file-types)) but **hand-mergeable**: integer coordinates on a grid, keys sorted by element ID, missing entries = "needs layout" (ELK places them), orphaned entries ignored with a warning.
4. **Canonical writer**: deterministic ordering and formatting so a no-op save is a no-op diff; preserve user comments via `toml_edit` when editing existing files.
5. **Schema version** field + migration functions in the core; JSON Schema (via `schemars`) for editor validation.
6. **Behavior models** embedded per element (`[behavior] kind = "petri" | "statemachine"`, `[[behavior.place]]`, `[[behavior.transition]]`, `[[behavior.arc]]`) with PNML import/export for P/T nets.

---

## 5. Cloud icon sets

| Set | Terms (verbatim where possible) | Cadence / latest | Formats | Source |
|---|---|---|---|---|
| **AWS Architecture Icons** | "We allow customers and partners to use these toolkits and assets to create architecture diagrams." Also allowed in whitepapers, presentations, data sheets, posters. No explicit grant for redistribution inside software. AWS Trademark Guidelines: no alteration of logos (s.9), no implied endorsement (s.5), "product icons" are trade dress (s.10), AWS "does not provide licenses ... for use of AWS content in third party publications" (s.15); contact trademarks@amazon.com | Quarterly: end of Jan, Apr, Jul; no Q4. Latest package file dated 07312026 | PPTX toolkit ZIP, icon package ZIP (SVG/PNG inside: **UNVERIFIED** from page text) | [AWS icons](https://aws.amazon.com/architecture/icons/), [trademark guidelines](https://aws.amazon.com/trademark-guidelines/) |
| **Azure architecture icons** | "Microsoft permits the use of these icons in architectural diagrams, training materials, or documentation. You can copy, distribute, and display the icons only for the permitted use unless granted explicit permission by Microsoft." Don't crop/flip/rotate/distort; don't use to represent your own product; download requires "I agree" | Irregular (Nov 2024, Mar 2025, Aug 2025, Nov 2025, Jul 2026); current ZIP `Azure_Public_Service_Icons_V24.zip` | SVG only, no Visio stencils | [Azure icons](https://learn.microsoft.com/en-us/azure/architecture/icons/) |
| **Google Cloud icons** | Page offers icons "for your diagrams, technical documentation, and more"; **no license text found on the page** -> **UNVERIFIED** | New iconography "early 2025"; core product icons + product category icons + legacy console icons | SVG + PNG ZIPs, overview PDF | [Google Cloud icons](https://cloud.google.com/icons) |

Precedents (not legal clearance): LikeC4 ships `@likec4/icons` (MIT package) with aws/gcp packs sourced from third-party sites aws-icons.com / gcpicons.com and Azure from Microsoft, with no separate license notes for those packs ([likec4 icons](https://github.com/likec4/likec4/tree/main/packages/icons)); mingrammer/diagrams (MIT) supports AWS/Azure/GCP nodes without stating icon terms ([repo](https://github.com/mingrammer/diagrams)); draw.io states its icon/stencil libraries have separate terms ([drawio](https://github.com/jgraph/drawio)).

**Recommendation:** do not commit vendor icons into our repo/binary by default. Ship generic infrastructure icons we own (or permissively licensed sets) and an **"Import official icon pack"** action that takes the user-downloaded vendor ZIP (user accepts vendor terms, e.g. Azure's "I agree"), indexes it into app data, and references icons by stable key (`aws:lambda`) in model files so models stay portable. Never modify the SVG artwork (scale only), keep the product name next to the icon (Azure guidance). Revisit bundling only after written confirmation from vendors.

---

## 6. UI kit and visual design

| Item | State | Source |
|---|---|---|
| **shadcn/ui** | Copy-in components; CLI `shadcn` 4.23.0; Tailwind v4 support since Feb 2025; React 19 since Oct 2024; "shadcn/cli v4" (Mar 2026); **Base UI is the default for new projects since July 2026**, Radix "still fully supported", no forced migration; components now offered for Base UI, React Aria and Radix variants | [changelog](https://ui.shadcn.com/docs/changelog), [Base UI default](https://ui.shadcn.com/docs/changelog/2026-07-base-ui-default), [npm](https://www.npmjs.com/package/shadcn) |
| **Base UI** (`@base-ui/react`) | 1.0.0 stable 2025-12-11; 1.9.0 (2026-10-09); MUI-hosted repo; MIT | [releases](https://base-ui.com/react/overview/releases), [npm](https://www.npmjs.com/package/@base-ui/react) |
| **Radix** (`radix-ui` unified pkg) | 1.8.1 (2026-10-12), MIT | [npm](https://www.npmjs.com/package/radix-ui) |
| **Tailwind CSS** | v4.0 (2025-01-22), v4.1 (2025-04-03), v4.3 (2026-05-08); npm 4.3.3 | [blog](https://tailwindcss.com/blog), [npm](https://www.npmjs.com/package/tailwindcss) |
| **Command palette** | `cmdk` 1.1.1 (MIT): unstyled, filters/sorts, accessible combobox, tested with VoiceOver; shadcn `<Command />` "uses the cmdk component" | [cmdk](https://github.com/pacocoursey/cmdk), [shadcn Command](https://ui.shadcn.com/docs/components/command) |
| **Canvas theming** | React Flow `colorMode` light/dark/system; drive both shadcn tokens and canvas styles from the same CSS variables | [React Flow v12](https://reactflow.dev/learn/troubleshooting/migrate-to-v12) |

Design references:
- **IcePanel**: one shared model, diagrams are views; zoom between levels; flows compared to sequence diagrams ([icepanel.io](https://icepanel.io/)). Closest to our drill-down + behavior animation idea.
- **Ilograph**: perspectives, contexts, walkthroughs, sequence perspectives, icon sets ([docs](https://www.ilograph.com/docs/)).
- **Structurizr**: model-first, many views from one model ([repo](https://github.com/structurizr/structurizr)).
- **LikeC4**: React Flow based, clean defaults, dynamic views ([repo](https://github.com/likec4/likec4)).
- Linear-like aesthetic (dense, neutral palette, keyboard-first, Cmd-K): no primary spec exists; treat as a style goal implemented with shadcn tokens + cmdk.

Choice Radix vs Base UI: Base UI is now shadcn's default and stable, so new code should start there; Radix remains a safe fallback (behavior differences exist, e.g. Tabs activation, per third-party guides; **UNVERIFIED** in primary docs).

---

## 7. Rust core architecture

### 7.1 Workspace layout (recommended)

```
crates/
  model/      # domain types: elements, relations, views, behavior (serde, ts-rs/specta derives); no I/O
  format/     # read/write the on-disk directory format, canonical writer, migrations, schema (toml_edit, schemars)
  verify/     # Petri-net / state-machine analysis (reachability, deadlock, boundedness), counterexample traces; petgraph
  layout/     # optional: Rust-side helpers (grid snapping, layout diffing); real layout runs in elkjs
  interop/    # Structurizr DSL, LikeC4, PlantUML/Mermaid export, PNML (quick-xml), draw.io
  engine/     # facade: open repo, watch (notify), job runner + cancellation, change events
  cli/        # `arch check|verify|export` for CI and pre-commit hooks
  wasm/       # optional wasm-bindgen wrapper of model+format+verify (web viewer, tests in browser)
app/
  src-tauri/  # thin adapter: commands -> engine, Channels for progress/traces, capabilities
  src/        # React + React Flow + shadcn
```

Rules: `model`, `format`, `verify` have **no** dependency on Tauri or tokio; cancellation and progress are passed as small traits so the same code runs in CLI, tests, Tauri and WASM.

### 7.2 Key crates

| Crate | Version | Purpose | Source |
|---|---|---|---|
| serde / serde_json | 1.0.229 / 1.0.154 | serialization | [serde](https://crates.io/crates/serde), [serde_json](https://crates.io/crates/serde_json) |
| petgraph | 0.8.3; Apache-2.0/MIT; `Graph`, `StableGraph`, `GraphMap`, `MatrixGraph`; `serde-1` feature; `no_std` capable | graph structures, SCC/paths for analysis | [repo](https://github.com/petgraph/petgraph), [crates.io](https://crates.io/crates/petgraph) |
| toml_edit | 0.25.17 | format-preserving file writes | [crates.io](https://crates.io/crates/toml_edit) |
| notify (+ debouncer-full) | 8.2.1 (+0.7.0) | repo watching | [crates.io](https://crates.io/crates/notify) |
| quick-xml | 0.42.0 | PNML, draw.io, ArchiMate XML | [crates.io](https://crates.io/crates/quick-xml) |
| schemars | 1.2.3 | JSON Schema for files | [crates.io](https://crates.io/crates/schemars) |
| gix / git2 | 0.90.0 / 0.21.0 | optional git status/blame integration | [gix](https://crates.io/crates/gix), [git2](https://crates.io/crates/git2) |
| wasm-bindgen | 0.2.130 | optional WASM target | [crates.io](https://crates.io/crates/wasm-bindgen) |
| tokio-util | 0.7.20 | `CancellationToken` in the Tauri adapter | [docs.rs](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html) |

### 7.3 WASM option

Compiling `model`/`format`/`verify` to WASM keeps a path open for a read-only web viewer or running verification in the browser for demos. Constraint: no threads/filesystem in those crates (I/O lives in `engine`). Not needed for v1 because Tauri calls native Rust directly.

---

## Recommendations for this project

| Area | Pick | Why | Runner-up |
|---|---|---|---|
| Shell | **Tauri 2.12.x**, thin `src-tauri` adapter | Stable line, 3.0 only alpha; adapter isolates migration | (none; fixed by brief) |
| IPC | Commands for request/response, **Channels** for verification progress + trace frames, events only for "repo changed" broadcasts | Docs: channels are ordered/fast, events are untyped and not for throughput | Raw `ipc::Response` for very large payloads |
| Bindings | **tauri-specta pinned to an exact RC** (commands + events) | Compile-checked TS for commands and events | **ts-rs 12** (stable) for types + hand-written wrappers if specta RC churn hurts |
| Background jobs | Job registry in managed state + `CancellationToken`; core polls a cancel trait | Tauri has no built-in cancellation | — |
| Watching | `notify` in the Rust engine with debounce; `PollWatcher` fallback | Must reconcile git checkouts/merges; WSL/Windows-path caveat | `tauri-plugin-fs` `watch` (frontend-driven) |
| Canvas | **React Flow 12 (MIT)** | React components as nodes, sub flows, free token-animation pattern, a11y, dark mode, proven by LikeC4 | **JointJS core + @joint/react (MPL-2.0)** if routing quality becomes the main pain (built-in manhattan router, ELK and libavoid packages) |
| Not chosen | tldraw (license key + watermark), GoJS (commercial, no redistribution), Cytoscape/Sigma (analysis viewers), Rete (dataflow editor), Excalidraw (whiteboard), maxGraph (pre-1.0) | see section 2 | — |
| Layout | **elkjs layered in a Web Worker**, `INCLUDE_CHILDREN` for nested views, orthogonal routing, render ELK bend points in custom edges; run on demand, persist results | Only engine with compound + ports + orthogonal + labels | dagre (MIT, simple trees); Graphviz WASM (LikeC4 approach); libavoid-js (LGPL) later for interactive re-routing |
| Native format | **Own format**: directory of **TOML** files (one per element) + separate `*.layout.toml` per view; stable IDs; canonical writer; `toml_edit` to keep comments | Line-diffable, merge-friendly, mature Rust tooling, comments | **KDL 2** (nicer for nested node data, but small ecosystem, no serde for v2) |
| Interop | Import: Structurizr DSL, LikeC4, PNML. Export: LikeC4, Structurizr DSL, PlantUML C4, Mermaid C4, PNML, draw.io, SVG/PNG | Meet users where they are without inheriting their limits | ArchiMate exchange later |
| Icons | Own generic set bundled; AWS/Azure/GCP via user-run **import of official ZIP** | Vendor terms do not clearly allow redistribution in software | Bundle after written vendor confirmation |
| UI | **shadcn/ui on Base UI** + Tailwind v4 + cmdk palette; shared CSS variables for app + canvas | Base UI stable and now shadcn default | shadcn on Radix |
| Core | Cargo workspace: `model`/`format`/`verify`/`interop`/`engine`/`cli`, optional `wasm`; petgraph | Headless tests, CLI for CI, future web viewer | — |
| Testing | `cargo test` on core (bulk), Vitest + `mockIPC`, `@wdio/tauri-service` E2E on Linux/Windows/macOS CI | Docs-backed, cross-platform | `tauri-driver` (no macOS) |
| Dev on WSL2 | Repo on Linux FS; WebKitGTK env-var workarounds ready; Windows/macOS builds via CI | MSI needs Windows; WSLg renders with WebKitGTK | Native Windows toolchain for UI perf checks |

## Open questions

1. **Icon licensing**: Is bundling AWS/Azure/GCP icons inside an open-source diagramming tool allowed? AWS page and trademark guidelines are silent/restrictive, Azure limits copying "only for the permitted use", Google page shows no license. Needs written confirmation (AWS: trademarks@amazon.com) or the import-ZIP approach.
2. **elkjs EPL-2.0 / libavoid LGPL-2.1**: confirm obligations for distributing them inside Tauri bundles (notice files, source offer, replaceability for LGPL WASM). Not legal advice.
3. **React Flow at scale on WebKitGTK**: spike with 1k/2.5k/5k nodes, animated tokens, on WSLg, native Linux, Windows WebView2, macOS. No official numbers exist.
4. **tauri-specta / specta 2.0 still RC**: accept pinned RC or start with ts-rs? Does tauri-specta type `Channel<T>` payloads (README silent)?
5. **TOML vs KDL**: confirm with a mock of a real model (C4 element + Petri net + layout) which reads and merges better; check how a three-way git merge of concurrent edits behaves.
6. **LikeC4 manual layout storage and draw.io round-trip details** were not verifiable from the docs fetched; needed if we promise lossless LikeC4 interop.
7. **PNML graphics and PNTD for state machines**: confirm graphics annotations from the ISO 15909-2 grammar; decide whether state machines get their own export (e.g. SCXML; not researched here).
8. **Code level**: C4 says it is optional and ideally generated. Do we generate it from source (language parsers) or keep it hand-modeled? Affects node counts and performance targets.
9. **Tauri 3 timeline**: alpha.5 exists; when does stable land and what breaks for plugins/IPC?
10. **Mermaid C4 status**: the current docs page shows no "experimental" label but lists unfinished features; confirm before relying on it as an export target.
