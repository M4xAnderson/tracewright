//! The Tauri shell: thin commands over the engine (IPC-1), the local log and the panic hook.

mod log;

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
