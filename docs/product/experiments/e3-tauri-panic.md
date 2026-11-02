# E3 Tauri spike: crash policy and dev setup (feasibility)

As of 2026-11-02. Experiment E3 from `../opportunity-solution-tree.md`. Code: `prototypes/e3-tauri-panic/` (a prototype; throwaway code). It tests ADR 0006 as amended by ADR 0012, and the dev setup in `docs/research/stack.md` §1.10.

**Status: PARTIAL.** The crash policy passes, measured headlessly in debug and release builds. The window criterion is **not measured on either platform**. The agent sandbox blocks every Unix socket connect, so no window could open under WSLg from here. Windows cannot run from here at all. Both GUI runs are pending the checklists in "User checklists" below. The theme question is pending for the same reason.

## Question

Do the crash policy and the dev setup work? Pass criterion: a check whose worker panics ends as Unknown "internal error", the app stays up, the panic is logged locally, and the window is usable under WSLg and on Windows WebView2. If the policy fails, revisit ADR 0006 with the architect.

## Method

- **App:** a minimal Tauri 2 app with a React and Vite frontend. Bun is the package manager and runner, and `@tauri-apps/cli` is a dev dependency.
- **Worker:** `run_check` spawns a new named thread `check-<id>` for each check. It owns an immutable snapshot and shares no lock with the app. Its toy exploration has an `assert_eq!` that stays on in release and is made to fail in "panic" mode. A `monitor-<id>` thread joins the worker and maps a join error to `VerdictDto::Unknown { category: "internalError", message: "internal error" }`, then sends the result through a Tauri `Channel`. The monitor also locks the stand-in Session to record the outcome, as `design.md` §4 describes.
- **Panic hook (ADR 0012):** every panic is logged with thread name, location, message and a forced backtrace, then flushed. A thread named `check-`, `lower-`, `derive-`, `scan-` or `save-` unwinds to its joiner. Any other thread aborts after logging.
- **Log:** `<local data dir>/dev.tracewright.e3prototype/logs/tracewright-e3.log`. No ADR names a log location. This is the directory Tauri's `app_log_dir()` returns on Linux and Windows, next to the journals in `docs/specs/file-format.md` (`<app local data dir>/journals/`). It is computed without an `AppHandle`, so the log is open before Tauri starts.
- **Headless verification:** the same binary has a `--headless` mode. It runs the same supervisor without a window: first a check that panics, then a normal check, then a Session lock. It exits 0 only if the first check is Unknown "internal error", the second is Proved, the Session is not poisoned and the log file is open. `--panic-main` and `--panic-command` add a panic on the main thread and on Tauri's async runtime (`tauri::async_runtime`, where async commands run). `headless-matrix.sh` builds and runs every case.
- **GUI verification:** with `E3_AUTOCHECK=exit`, the frontend runs the same sequence over real IPC on start (panicking check, normal check, `apply`). It reports each verdict and the theme values to the log, and the app exits 0 on pass or 1 on failure. With `E3_AUTOCHECK=stay`, it stays open for the manual checks.
- **Panic strategy:** the same code is built in two profiles, `release` (`panic = "unwind"`) and `release-abort` (`panic = "abort"`).

Versions used: Ubuntu 24.04.5 on WSL2 (kernel 6.18.54.1), WebKitGTK 2.52.6, GTK 3.24.41, Tauri 2.12.5 (wry 0.57.1, tao 0.37.2), `@tauri-apps/cli` and `@tauri-apps/api` 2.12.4, React 19.3.1, Vite 7.3.7, Bun 1.4.4, rustc 1.99.1. The brief assumed Bun 1.3 and Rust 1.93; the installed versions are newer, and nothing depended on the difference.

## Results per criterion

| Criterion | WSLg (measured here) | Windows WebView2 |
|---|---|---|
| Check ends Unknown "internal error" | **Pass**, headless, in debug and release (unwind). Log: `check-00000001 died: assertion ... failed: token conservation violated ...; reporting Unknown internal error`, then `-> Unknown { category: "internalError", ... }`. The GUI path over `Channel` is built but not run. | Pending checklist |
| App stays up; a second check works | **Pass**, headless. The process exits 0. The next check gets a new worker and is `Proved { markings: 6 }`, and the Session mutex is not poisoned. Nothing is reused: the dead worker's state is gone, as ADR 0006 requires. | Pending checklist |
| Panic logged locally | **Pass.** The log file has the PANIC line (thread, file:line:column, message, `supervised=true`, `panic=unwind`), a 14-frame backtrace with function names (`e3_lib::supervisor::explore` at frame 8 in release), the monitor's ERROR line and the verdict. The file is fsynced before the hook returns. | Pending checklist |
| Window usable | **Not measured.** See "Sandbox limits" below. `tauri dev` and `tauri build --no-bundle` both build and start the binary, and Vite serves on port 1420. The app then aborts in GTK initialization because it cannot reach the display: `tao-0.37.2/src/platform_impl/linux/event_loop.rs:219: Failed to initialize gtk backend!`. | Pending checklist |

### The abort branch of ADR 0012 (measured, release build)

| Case | Exit code | Log |
|---|---|---|
| Worker panics, then a normal check (`release`, unwind) | 0 | PANIC `[check-00000001]`, ERROR `[monitor-00000001]`, `pass=true` |
| Panic on the main thread | 134 (SIGABRT) | PANIC `[main]`, then `FATAL ... aborting (ADR 0012)` |
| Panic in a task on Tauri's async runtime | 134 | PANIC `[tokio-rt-worker]`, then `FATAL` |
| Worker panics (`release-abort`) | 134 | PANIC `[check-00000001] ... panic=abort` only; no monitor line, no verdict |

## Panic strategy: keep `panic = "unwind"` (ADR 0006 holds)

With `panic = "abort"`, the hook still runs and the panic is logged, but the process dies right after the hook returns. `join()` never returns, the user gets no Unknown, and the app is gone (exit 134). `catch_unwind` cannot help either, because under abort there is nothing to catch. Unwind is therefore a precondition for the policy, not a choice between two mechanisms. With unwind, thread join is enough, and the first-party code needs no `catch_unwind`. The cost measured here: the unstripped release binary is 11.4 MB with unwind and 8.2 MB with abort.

Three details for slice 0:

1. **Tokio would hide command-thread panics without the hook.** Tauri 2.12 names its runtime threads `tokio-rt-worker`, not `tokio-runtime-worker`. Tokio catches a panic in a spawned task and reports it only through the task's `JoinHandle`. The hook's abort branch fires first, because the hook runs before unwinding starts. The allowlist must stay "our prefixes unwind, everything else aborts" and must never be a list of foreign thread names. The claim that Tauri's IPC promise would otherwise never settle was not measured.
2. **Open the log before `tauri::Builder`.** The GTK initialization failure happened before `setup` ran. In the first version, which opened the log in `setup`, that panic reached stderr only. `app_log_dir()` needs an `AppHandle`, so slice 0 computes the same directory itself (`dirs::data_local_dir()/<identifier>/logs`) and warns in `setup` if Tauri's directory differs. On macOS, `app_log_dir()` is `~/Library/Logs/<identifier>` instead, so the two differ there.
3. **The hook must not deadlock.** The log writer uses `try_lock`, so a panic while a thread holds the log lock cannot hang the hook. Release backtraces carry function names but no line numbers (`debug = 0`, symbols kept). If the bundle strips symbols, the frames become `<unknown>`. Slice 0 decides between keeping symbols and shipping split debuginfo.

## Theme finding (`flows.md` §8, "[assumption, E3]")

**Not measured**, for the same reason as the window. What the sandbox shows: `gsettings` reports `color-scheme 'default'` and `gtk-theme 'Adwaita'`, but dconf cannot start here (`/run/user/1000/dconf` is read-only), so these may be the schema defaults rather than the user's values. WSLg runs no settings daemon or desktop portal that would pass the Windows app mode to GTK. The hypothesis is **that WebKitGTK under WSLg always reports light** through `prefers-color-scheme`, whatever the Windows setting. If that holds, the "Theme: System / Light / Dark" commands of §6.1 are needed.

The prototype measures this on the user's run. It logs `prefers-color-scheme` and Tauri's `window.theme()` at start and on every change. It also has buttons that call `window.setTheme("light" | "dark" | null)`. These show whether a Theme command can flip the webview's media query, so the CSS can keep one `prefers-color-scheme` path, or whether the UI needs its own theme class.

## Dev-setup gotchas slice 0 must know

- **Vite runs on the system Node 18 under `bun run`.** A script `"dev": "vite"` runs Vite's `#!/usr/bin/env node` entry with `/usr/bin/node` 18.19.1, and Vite 7 warns that it needs Node 20.19+ or 22.12+. `bun --bun run build` and `bunx --bun vite build` did not change that here. That may be a sandbox effect, because Bun's node shim lives in a temp directory, but it is unverified. The prototype calls the entry file directly: `"dev": "bun ./node_modules/vite/bin/vite.js"`. Slice 0 should do the same or pin Node 22 with fnm, which is installed.
- **Linux packages:** the build needed only `libwebkit2gtk-4.1-dev` with `build-essential` and `libssl-dev`. `librsvg2-dev`, `libayatana-appindicator3-dev` and `libxdo-dev` from Tauri's prerequisites list are not installed and were not needed without a tray or menus.
- **Release builds through plain cargo** need `--features tauri/custom-protocol` to embed `dist/`. `bun run tauri build --no-bundle` adds it. Run `bun run build` before any release `cargo build`.
- **`tauri dev` exited 0 although the app aborted with 134.** A CI smoke test must check the app's own exit code, not the CLI's.
- **Windows release builds** use `windows_subsystem = "windows"`. `--headless` then prints nothing, and PowerShell does not wait for the process. Use `Start-Process -Wait -PassThru` and read `.ExitCode`. Rust's `abort()` on Windows ends with `0xC0000409` (-1073740791), not 134.
- **Build cost on this machine:** cold debug build 52 s, release 66 s, `release-abort` 71 s; `target/` reaches 4.7 GB for the three profiles.
- **WebKitGTK workarounds** (`WEBKIT_DISABLE_DMABUF_RENDERER=1`, `WEBKIT_DISABLE_COMPOSITING_MODE=1`) remain untested. They are fallback steps in the WSLg checklist, and the app logs which of them are set.

### Sandbox limits (for agent-run spikes such as E4)

- Every `AF_UNIX` `connect` and `bind` fails with EPERM: the X11 socket `/tmp/.X11-unix/X0`, the Wayland socket `/run/user/1000/wayland-0` and the D-Bus session bus. `socketpair` works. `/mnt/wslg` appears empty in the sandbox. As a result, **no agent can open a window or take a screenshot here**. E4's frame-time harness needs the user's terminal or a sandbox exception for the display sockets, which is the user's decision.
- `~/.cargo`, `~/.bun/install/cache` and `~/.local/share` are read-only. The builds used `CARGO_HOME=$TMPDIR/cargo-home`, `BUN_INSTALL_CACHE_DIR=$TMPDIR/bun-cache` and `XDG_DATA_HOME=$TMPDIR/xdg`. The default log location failed with EROFS, and the headless run then exits 1 because nothing was logged.
- The sandbox leaves `/dev/null` placeholder entries (character devices owned by `nobody`) for denied paths at the worktree root (`.bashrc`, `.gitconfig`, `.mcp.json`, `.claude/settings.json` and others). They are not build output and are not committed.

### E3 items from `flows.md` this run did not cover

`flows.md` also assigns these checks to E3. This run did not cover them:

- claiming webview shortcuts (Ctrl+R, Ctrl+P, F5 and the others) on WebKitGTK and WebView2;
- the xdg-open handler for `tauri-plugin-opener` under WSLg;
- Orca announcements for `aria-live`.

They need a window, so they belong in the same user run or in E4.

## User checklists

Report back for each platform: every exit code, the log lines that contain `UI`, `PANIC`, `FATAL` or `result`, and one sentence on whether the window felt usable (renders, clicks, typing, resize).

### WSLg (from a normal WSL terminal, not an agent session)

```sh
cd ~/.t3/worktrees/tracewright/t3-cf2371ca/prototypes/e3-tauri-panic   # or check out prototype/e3-tauri-panic anywhere
bun install
bun run tauri build --no-bundle
./headless-matrix.sh                                     # expect 0, 134, 134, 134
E3_AUTOCHECK=exit ./src-tauri/target/release/e3-tauri-panic; echo "exit $?"   # expect: window flashes, exit 0
E3_AUTOCHECK=stay ./src-tauri/target/release/e3-tauri-panic &
```

1. Look for a red "PROTOTYPE" banner. The results list should show `check-00000001 (panic): Unknown: internal error...`, then `Proved (6 markings)` and `autocheck pass=true`. If the window is blank, close it and rerun with `WEBKIT_DISABLE_DMABUF_RENDERER=1`, then also with `WEBKIT_DISABLE_COMPOSITING_MODE=1`. Note which one was needed.
2. Click "Run check (worker panics)" twice and "Run check (normal)" once. Each click adds a row and the window stays responsive. Click "Apply an edit": `rev` goes up and `poisoned=false`. Type in the text field. Resize the window.
3. Theme: read the "prefers-color-scheme" and "Tauri window theme" rows. Switch Windows Settings > Personalization > Colors > "Choose your app mode" between Dark and Light while the app is open, and note whether either row changes. Click the theme buttons light, dark and system, and note what each reports. Then rerun once with `GTK_THEME=Adwaita:dark` and note the start values.
4. Click "Panic on a command thread". The window disappears. The log ends with `FATAL [tokio-rt-worker]`.
5. Run `bun run tauri dev` once, change the `<h1>` text in `src/App.jsx` and check that the window updates (HMR).
6. Log: `~/.local/share/dev.tracewright.e3prototype/logs/tracewright-e3.log`. Example: `grep -E "UI|PANIC|FATAL|result" <log> | cut -c1-200`.

### Windows WebView2 (PowerShell, on the Windows file system)

Install once:

1. Visual Studio 2022 Build Tools with the C++ workload: `winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`.
2. Rust: `winget install Rustlang.Rustup`, then in a new shell `rustup default stable-msvc`.
3. Bun: `winget install Oven-sh.Bun` (or `powershell -c "irm bun.sh/install.ps1 | iex"`).
4. Git: `winget install Git.Git`. WebView2 is preinstalled on Windows 10 and 11. To check: `Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' | Select-Object pv` prints a version.

Get the code. Don't build through `\\wsl.localhost`; clone to a Windows path instead. The branch lives in the WSL repository. Find the distro name with `wsl -l -q`:

```powershell
git clone \\wsl.localhost\<distro>\home\nils\.t3\projects\tracewright C:\src\tracewright-e3
cd C:\src\tracewright-e3
git checkout prototype/e3-tauri-panic
cd prototypes\e3-tauri-panic
bun install
bun run tauri build --no-bundle
```

If `git clone` reports "dubious ownership", rerun it as `git -c safe.directory='*' clone ...`.

Run:

```powershell
$exe = ".\src-tauri\target\release\e3-tauri-panic.exe"
(Start-Process $exe -ArgumentList '--headless' -Wait -PassThru).ExitCode                    # expect 0
(Start-Process $exe -ArgumentList '--headless','--panic-main' -Wait -PassThru).ExitCode     # expect -1073740791
(Start-Process $exe -ArgumentList '--headless','--panic-command' -Wait -PassThru).ExitCode  # expect -1073740791
$env:E3_AUTOCHECK = 'exit'; (Start-Process $exe -Wait -PassThru).ExitCode                   # expect: window flashes, 0
$env:E3_AUTOCHECK = 'stay'; Start-Process $exe
```

1. Repeat steps 1–4 of the WSLg list. Skip the WebKitGTK and `GTK_THEME` variants. For the theme step, the Windows app mode toggle is the real test here.
2. Optional, the abort profile: `cd src-tauri; cargo build --profile release-abort --features tauri/custom-protocol; (Start-Process .\target\release-abort\e3-tauri-panic.exe -ArgumentList '--headless' -Wait -PassThru).ExitCode`. Expect -1073740791 and a PANIC line with `panic=abort` and no verdict.
3. Log: `Get-Content "$env:LOCALAPPDATA\dev.tracewright.e3prototype\logs\tracewright-e3.log" | Select-String 'UI|PANIC|FATAL|result'`. If the log has a `WARN ... app_log_dir()` line, Tauri's log directory differs from the one the prototype computed; report it.
4. `Remove-Item Env:E3_AUTOCHECK` when done.

## Decision

The crash policy holds as written in ADR 0006 and 0012, so there is no reason to revisit them. E3 closes as **PASS** when both checklists report exit codes as expected, the "internal error" row and a usable window. If the window fails on either platform, the fallback named in the opportunity tree applies to the dev setup, not to the crash policy. Then record the failing platform and its env-var workaround here before slice 0 starts.
