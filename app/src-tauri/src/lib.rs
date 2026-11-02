//! The Tauri shell: thin commands over the engine (IPC-1), the local log and the panic hook.

mod ipc;
mod log;

// What `run` installs first, public for the worker-panic test (ADR 0012), which needs the real
// hook and log on a real thread.
pub use log::log_panics;
pub use log::open as open_log_in;

use tauri::Manager;

/// The app identifier; `tauri.conf.json` must say the same, which `setup` asserts.
pub const IDENTIFIER: &str = "dev.tracewright.app";

/// Starts the app and returns when its window closes.
///
/// # Panics
///
/// Panics if Tauri cannot start, for example without a window system. The panic is logged first.
pub fn run() {
    open_log();
    log::log_panics();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(smoke_from_env())
        .manage(ipc::AppState::default())
        .invoke_handler(tauri::generate_handler![
            ipc::ui_ready,
            ipc::pick_project_folder,
            ipc::open_project,
            ipc::run_checks,
            ipc::reload_project
        ])
        .setup(|app| {
            assert_eq!(
                app.config().identifier,
                IDENTIFIER,
                "tauri.conf.json identifier"
            );
            compare_log_dir(app.path().app_log_dir().ok().as_deref());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri could not start the app");
}

// D-72: before `tauri::Builder`, so a start-up panic is logged. Without a log the app still runs;
// its lines then go to stderr only.
fn open_log() {
    let Some(dir) = log::dir(IDENTIFIER) else {
        log::line(
            log::Level::Error,
            "no local data directory; logging to stderr only",
        );
        return;
    };
    match log::open(&dir) {
        Ok(path) => log::line(log::Level::Info, &format!("log file {}", path.display())),
        Err(error) => log::line(
            log::Level::Error,
            &format!(
                "cannot open the log in {}: {error}; logging to stderr only",
                dir.display()
            ),
        ),
    }
}

// They differ on macOS (`~/Library/Logs/<identifier>`); E3 found them equal on Linux and Windows.
fn compare_log_dir(tauri_dir: Option<&std::path::Path>) {
    let ours = log::dir(IDENTIFIER);
    if tauri_dir != ours.as_deref() {
        log::line(
            log::Level::Warn,
            &format!("log directory {ours:?} differs from Tauri's app_log_dir() {tauri_dir:?}"),
        );
    }
}

// The CI smoke test sets `TRACEWRIGHT_SMOKE=exit-when-start-screen-shown` and checks the app's own
// exit code, because `tauri dev` exits 0 even when the app aborts (spike E3).
fn smoke_from_env() -> ipc::Smoke {
    match std::env::var("TRACEWRIGHT_SMOKE").as_deref() {
        Err(_) => ipc::Smoke::Off,
        Ok("exit-when-start-screen-shown") => {
            log::line(
                log::Level::Info,
                "smoke test: exit when the start screen is shown",
            );
            ipc::Smoke::ExitWhenStartScreenShown
        }
        Ok(other) => {
            log::line(
                log::Level::Warn,
                &format!("ignoring unknown TRACEWRIGHT_SMOKE={other:?}"),
            );
            ipc::Smoke::Off
        }
    }
}
