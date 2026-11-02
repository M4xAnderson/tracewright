# E4 canvas scale spike: React Flow + elkjs at 300 elements per view

As of 2026-11-02. Experiment E4 from `../opportunity-solution-tree.md`. Prototype: `prototypes/e4-canvas-scale/` (throwaway code, see its README). Raw results: `prototypes/e4-canvas-scale/results/*.json`.

**Question.** Does the canvas hold the performance targets? React Flow 12 + elkjs, 300 elements in one view, 2,000 in the project, in the real Linux webview (WebKitGTK through Tauri 2).
**Pass criterion (set before the run).** An ELK layout of 300 elements in under 2 s, run in a web worker, and pan and zoom without visible stutter. Fallback if it fails: JointJS (`docs/research/stack.md`).

**Status: PARTIAL.** Every number below comes from headless Chromium. The WebKitGTK window could not be opened from the agent sandbox (see "WebKitGTK: blocked"). The harness for it is built, and one command produces the missing numbers. Layout passes clearly in Chromium. Pan and zoom are smooth in Chromium at normal CPU speed. Whether they stutter in WebKitGTK still needs your eye and the WebKitGTK run.

## Method

- **Synthetic project** (`src/gen.ts`, seeded): 2,000 elements (72 % containers, 16 % databases, 12 % queues; 60 % carry a small Petri-net stub), 3,000 relationships (1.5 per element, 35 % asynchronous with capacity and when full), 2–4 ports per element, 12 views. View 0 has **300 elements, 438 relationships, 899 ports**. The other 1,700 elements stay in memory as the project (1.8 MB as JSON).
- **Layout** (`src/layout.worker.ts`): elkjs 0.12.1 `layered` in a dedicated module worker. Settings: orthogonal routing, `FIXED_SIDE` ports (in ports on the west side, out ports on the east), edge labels sized from their text, `considerModelOrder = NODES_AND_EDGES`. Cold means the first run in a fresh worker. Warm is the median of the next 5 runs. Six more generated projects (seeds) show how much the graph shape matters. The 300-element layout is 7,490 × 7,226 px, so fit view is at about 11 % zoom.
- **Canvas**: `@xyflow/react` 12.12.1 and React 19.3. Custom element nodes follow the 2b mockup, with one `Handle` per port at ELK's port position. Custom edges draw ELK's bend points; labels are HTML through `EdgeLabelRenderer`. Production build (Vite 8).
- **Frames**: a scripted 16 s sequence drives `setViewport` once per `requestAnimationFrame`: hold at fit, zoom to 100 %, pan across, pan down, zoom out to fit, zoom to 200 %, zoom to 60 %, back to fit. Reported per frame: rAF deltas, the share of frames over 16.7, 25 and 33.4 ms, and **dropped frames** (a delta of k refresh intervals hides k − 1 frames; refresh comes from 1.5 s of idle frames). React work per frame: from `setViewport` to after React's microtask flush.
- **End to end** (U10, U13): gesture → IPC → reducer (keeps object identity for unchanged nodes) → `flushSync` commit → the frame after it is painted (rAF, then a posted message). Two gestures: 30 renames, and 10 moves of all 300 elements, alternating between a left-to-right and a top-down ELK layout. The 300-element move stands in for "apply an ELK layout". In Tauri, the IPC is a real command that returns the full view projection, as `design.md` plans for slice 0. In the browser, a JSON round trip stands in for it.
- **Memory**: in Tauri, RSS and PSS of the app process and its WebKit child processes from `/proc`. In Chromium, `performance.memory`.
- **Runs**: 10 variants in headless Chromium 149 (Playwright's headless shell, 1440 × 900, DPR 1), the recommended one 3 times. Then 7 variants again with CDP CPU throttling at 4×, as a stand-in for a slower machine or a slower engine. Throttling slows the main thread only, not the layout worker. Machine: 24 cores, WSL2.

## WebKitGTK: blocked by the sandbox

`tauri build` works, and the binary is at `src-tauri/target/release/e4-canvas-scale`. Starting it from the agent's shell fails in `gtk::rt::init`: "Failed to initialize GTK", under both `GDK_BACKEND=wayland` and `GDK_BACKEND=x11`. The cause is the Claude Code sandbox's seccomp filter. `socket(AF_UNIX, …)` returns `EPERM`, so no client can connect to `/run/user/1000/wayland-0` or `/tmp/.X11-unix/X0`, even though both are visible. `socketpair()` and `AF_INET` sockets are allowed. I did not try any workaround (Xvfb over TCP, for example), as the brief asked. **The WebKitGTK column below is empty until `bun run bench:webkit` runs outside the sandbox.**

## Numbers (headless Chromium; WebKitGTK pending)

Recommended settings (`onlyRenderVisibleElements`, memo, one zoom class on the wrapper, labels on). For 1× the table shows the range over 3 runs; 4× is one run.

| Measure | Chromium 1× | Chromium 4× CPU | WebKitGTK (Tauri) | Target |
|---|---|---|---|---|
| ELK layout 300, cold, in worker (round trip) | 0.99–1.24 s | 1.16 s | *pending* | < 2 s |
| ELK layout 300, warm, median | 0.71–0.83 s | 0.83 s | *pending* | < 2 s |
| ELK layout 300, six other graphs | 0.61–1.13 s (warm); one other generator run: 1.71 s cold, 1.53 s warm | 0.76–1.46 s | *pending* | < 2 s |
| Worker spawn to ready (elkjs is 1.47 MB of JS) | 155–170 ms | 253 ms | *pending* | — |
| Time from layout result to first painted frame | 177–220 ms | 861 ms | *pending* | — |
| Time from navigation to first painted frame (includes the cold layout) | 1.37–1.70 s | 2.43 s | *pending* | — |
| Pan and zoom: median / p95 frame | 16.7 / 16.7–16.8 ms | 16.7 / 66.6 ms | *pending* | p95 < 33 ms (U10) |
| Pan and zoom: frames over 16.7 / 25 / 33.4 ms | 41–46 % (jitter, see below) / 0.6–1.6 % / 0.1–0.2 % | 53 / 13 / 9 % | *pending* | — |
| Pan and zoom: dropped frames | 0.6–1.6 % | 25 % | *pending* | "no visible stutter" |
| React work per pan/zoom frame, median / p95 | 1.1 / 4.1–4.4 ms | — / 17.9 ms | *pending* | < 16.7 ms |
| Rename, gesture to painted frame, median / p95 | 33 / 36–38 ms | 135 / 158 ms | *pending* | p95 < 50 ms (U10) |
| Move all 300 (ELK layout applied), median / p95 | 105–115 / 120–137 ms | — / 593 ms | *pending* | p95 < 100 ms (U13) |
| Its parts: IPC / reducer / React commit | 1.1 / 0.8 / 34 ms | — / — / 153 ms | *pending* | — |
| Project load over IPC (2,000 elements, 1.8 MB) | n/a (no Tauri) | n/a | *pending* | < ~50 ms, else raw `ipc::Response` |
| JS heap: 2,000-element project held / after first render | 7.9 / 25 MB | — | RSS/PSS *pending* | — |
| JS heap, same with 300 held | 2.9 / 20 MB | — | *pending* | — |

How to read the frame rows: headless Chromium drives rAF from an exact 16.67 ms clock. So "over 16.7 ms" counts deltas of 16.7–16.8 ms, which is jitter, not lost frames. Dropped frames are the honest measure. Headless Chromium also hardly rasterizes and composites compared with a real window on a GPU, so the 1× frame numbers mostly measure JavaScript and style/layout work. They are a lower bound for WebKitGTK.

Per segment, at 1× with the recommended settings, all drops happen in **"zoom out to fit"** and **"back to fit"** (5.8 % dropped each, p95 33 ms). Panning at 100 % drops nothing. At 4×, the pans at 100 % still drop nothing, while the zoom-out-to-fit segments drop 45–69 %.

**Layout scaling** (Chromium, warm worker): 600 elements in 2.0 s, 1,000 in 7.5 s, **2,000 in 74 s**. Growth is superlinear. Layout must stay per view; never lay out the whole project at once.

## Levers

Dropped frames during the pan-and-zoom script, and React work per frame (p95). All runs keep labels on unless noted.

| Variant | 1×: dropped / work p95 | 4×: dropped / work p95 | Effect |
|---|---|---|---|
| Recommended (`onlyRenderVisibleElements`, memo, zoom class on wrapper) | 0.6–1.6 % / 4.1–4.4 ms | 25 % / 18 ms | Baseline |
| Render all elements (`onlyRenderVisibleElements` off) | 0.3 % / 3.2 ms | **54 %** / 14 ms | At 1× it is smoother when zooming out to fit (no mounting). At 4×, every frame of the 100 % pans costs about 33 ms (50 % dropped against 0 %). Keep it **on**. |
| No `React.memo` on node and edge components | 1.6 % / 4.5 ms | 24 % / 17 ms | **No measurable effect**, most likely because React Flow already memoizes its node and edge wrappers and our data objects keep their identity. Harmless to keep. |
| Per-node `useStore(s => s.transform[2])` (a number) | **6.1 %** / **13 ms** | 28 % / **34 ms** | The worst single lever: every node re-renders on every zoom frame. It drops 8–21 % of frames in the zoom segments even at 1×. **Never do this** (as `flows.md` §4.3 already says). |
| Per-node `useStore(s => s.transform[2] >= 1)` (a boolean) | 1.7 % / 4.5 ms | 24 % / 17.5 ms | About the same as one class on the wrapper. Only the selector runs per node; nodes re-render only when the threshold is crossed. The wrapper class is still simpler. |
| Naive (all rendered, no memo, per-node zoom number) | **13.4 %** / **16.6 ms** | **66 %** / 63 ms | Visible stutter, even in Chromium at 1×. |
| No relationship labels | 0.2 % / 3.8 ms | 21 % / 15 ms | Without labels, the zoom-out segments drop about 10–15 % fewer frames at 4×, and the end-to-end times fall by 22–30 % (rename p95 110 against 158 ms; move 460 against 593 ms). |
| Hold 300 instead of 2,000 elements | 1.0 % / 4.3 ms | — | Holding the other 1,700 elements costs about 5 MB of JS heap and nothing measurable in frames. |

React Flow measures each node when it mounts. With `onlyRenderVisibleElements`, that is where the remaining cost sits: zooming out to fit mounts up to 300 nodes, 438 edges and their labels within a few frames. The obvious next lever is untested: a cheap level of detail below about 35 % zoom (no ports, no tech line, no labels; at 11 % the text is unreadable anyway).

## Against the criterion

| Part | Result |
|---|---|
| Layout of 300 elements < 2 s in a worker | **Pass in Chromium**: cold 1.0–1.2 s, warm 0.6–1.1 s over seven graphs. One other generated graph: 1.7 s cold, 1.5 s warm, so the margin depends on the graph shape. **WebKitGTK pending**: JavaScriptCore runs the GWT-compiled ELK code; if it is more than about 1.6× slower than V8, some graphs will miss 2 s. |
| Pan and zoom without visible stutter | **Pending your look in WebKitGTK.** In Chromium at 1×: smooth (0.6–1.6 % dropped, all of it while zooming out to fit). At 4× CPU: pans at 100 % stay smooth; zooming out to fit stutters. |
| Also measured (not E4 criteria) | U10 rename p95 36–38 ms: passes in Chromium. U13 "layout-only edit pending < 100 ms p95": the 300-element move takes p95 120–137 ms in Chromium, and most of it is React commit and node measurement, not IPC (about 1 ms) or the reducer (under 1 ms). A one-element move is a rename-sized change (p95 under 40 ms), so U13 holds for the usual gesture but not for "apply an ELK layout to 300 elements". |

**Overall: PARTIAL.** Nothing measured points to JointJS. The open risk is WebKitGTK raster and compositing on WSLg, which this run could not see.

## Settings slice 0 should adopt

1. `onlyRenderVisibleElements` **on**.
2. **One** zoom subscription for the whole canvas: a boolean selector (`zoom >= 1`) that toggles a class on the wrapper. Never a zoom selector inside a node component.
3. Node and edge components with stable `nodeTypes` and `edgeTypes` (defined once, outside render). `React.memo` is optional; it did not matter.
4. The projection reducer keeps object identity for unchanged elements and relationships. Here a rename changes exactly 1 node object out of 300. This, not memo, keeps the common gesture cheap. A full projection per `apply` is fine: the IPC and reducer cost about 2 ms at 300 elements. **No need for a projection-diff patch** (design.md's E4 gate), unless the WebKitGTK IPC number says otherwise.
5. ELK in a dedicated module worker, created at startup (spawn to ready is about 160 ms, and the bundle is 1.5 MB), with `layered`, orthogonal routing, `FIXED_SIDE` ports and model order. Edges draw ELK's bend points.
6. Lay out per view only, on demand. Never lay out the whole project: 74 s at 2,000 elements.
7. Relationship labels as HTML through `EdgeLabelRenderer` are acceptable, but they are the largest single cost after the per-node zoom mistake. A level of detail below about 35 % zoom that hides labels, ports and the tech line is the first thing to try if WebKitGTK stutters when zooming out.

## For you: the look, and the WebKitGTK numbers

Run outside the Claude sandbox, from the repo root (the Rust release build takes about 1–2 min the first time):

```sh
cd prototypes/e4-canvas-scale && bun install && bun run window
```

It opens the 300-element view at fit. Try:
- Drag-pan fast across the whole view at fit zoom, then zoom in with the wheel to about 100 % (port names appear) and drag-pan across again.
- Zoom out from 100 % to fit quickly with the wheel. This is the expensive moment in Chromium.
- Select a node and drag it.
- Compare with the naive settings: `E4_VARIANT=visibleOnly=0,memo=0,zoomSub=number bun run window`. If that does not visibly stutter for you, the recommended settings will not either.

Then `bun run bench:webkit` (about 10 min). It runs every variant in `scripts/variants.txt` in the real window, including `WEBKIT_DISABLE_COMPOSITING_MODE=1` and `WEBKIT_DISABLE_DMABUF_RENDERER=1`. It writes `results/webkit-*.json` and prints a table. Paste the `rec` row into the WebKitGTK column above.

## Threats to validity

- No WebKitGTK numbers, which was the point of the spike. Headless Chromium hardly rasterizes, so frame costs that come from paint and compositing are invisible here.
- The pan and zoom are driven by `setViewport`, not by real pointer and wheel events through d3-zoom, so input handling is not included.
- The synthetic graph is layered by construction (8 tiers, 10 % back edges). Real designs may have more cycles or denser hubs, which affects ELK time; the seven graphs already spread from 0.6 to 1.7 s.
- The fake engine returns the view from memory; the real `apply` adds `model::apply` and projection under the lock (design target < 10 ms).
- Not measured: journal fsync on WSL2 (`design.md` lists it under E4; it belongs in a separate check); Windows WebView2; macOS.
- Tool versions differ from the brief: Bun 1.4.4 (not 1.3) and Rust 1.99.1 (not 1.93), as installed. Vite 8 needs a newer Node, so the scripts run Vite and the Tauri CLI through Bun.
