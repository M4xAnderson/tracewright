//! D-72: the local log opens before `tauri::Builder`, so a panic while the window system starts is
//! logged too (spike E3 lost exactly this panic when the log opened in `setup`).
#![cfg(target_os = "linux")]

use std::fs;
use std::process::Command;

#[test]
#[expect(
    clippy::disallowed_methods,
    reason = "test harness: runs the app binary itself"
)]
fn a_panic_while_the_window_system_starts_is_logged_locally() {
    let data = tempfile::tempdir().expect("temp dir");

    let output = Command::new(env!("CARGO_BIN_EXE_tracewright-app"))
        .env("XDG_DATA_HOME", data.path())
        // No GDK backend has this name, so GTK fails to initialize inside the Tauri builder.
        .env("GDK_BACKEND", "tracewright-no-such-backend")
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .expect("the app binary starts");

    assert!(
        !output.status.success(),
        "the app must not exit cleanly without a window system"
    );
    let log_path = data.path().join("dev.tracewright.app/logs/tracewright.log");
    let log = fs::read_to_string(&log_path).expect("the log file exists before Tauri starts");
    let panic_line = log
        .lines()
        .find(|l| l.contains(" PANIC [main] "))
        .unwrap_or_else(|| {
            panic!(
                "no PANIC line from the main thread in {}:\n{log}",
                log_path.display()
            )
        });
    assert!(
        panic_line.contains("gtk"),
        "the PANIC line names the GTK failure: {panic_line}"
    );
}
