//! The local, append-only log file (ADR 0006: panics are logged locally; nothing leaves the
//! machine).
//!
//! It opens before `tauri::Builder` runs (D-72), so a panic during start-up, such as GTK failing to
//! initialize, is logged too. Tauri's `app_log_dir()` needs an `AppHandle`, which does not exist
//! yet, so the directory is computed the same way here and compared in `setup`.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::SystemTime;

const FILE_NAME: &str = "tracewright.log";

static FILE: OnceLock<Mutex<File>> = OnceLock::new();

/// How serious a log line is.
#[derive(Debug, Clone, Copy)]
pub enum Level {
    Info,
    Warn,
    Error,
    Panic,
    Fatal,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
            Level::Panic => "PANIC",
            Level::Fatal => "FATAL",
        })
    }
}

/// `<local data dir>/<identifier>/logs`, the directory Tauri's `app_log_dir()` returns on Linux
/// and Windows.
pub fn dir(identifier: &str) -> Option<PathBuf> {
    dirs::data_local_dir().map(|data| data.join(identifier).join("logs"))
}

/// Opens the log file in `dir` for appending; later calls keep the first file.
///
/// # Errors
///
/// Returns the IO error if the directory or the file cannot be created.
pub fn open(dir: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = dir.join(FILE_NAME);
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    if FILE.set(Mutex::new(file)).is_err() {
        line(
            Level::Warn,
            &format!("log already open; ignoring {}", path.display()),
        );
    }
    Ok(path)
}

/// Writes one line to the log file and to stderr.
pub fn line(level: Level, message: &str) {
    let now = humantime::format_rfc3339_millis(SystemTime::now());
    let current = thread::current();
    let thread = current.name().unwrap_or("<unnamed>");
    let text = format!("{now} {level} [{thread}] {message}\n");
    // Nothing useful can be done if stderr is gone; the log file below is the record.
    if let Err(_stderr_closed) = io::stderr().write_all(text.as_bytes()) {}
    if let Some(file) = FILE.get() {
        // try_lock, so a panic on a thread that holds the lock cannot deadlock the panic hook.
        if let Ok(mut file) = file.try_lock() {
            if let Err(error) = file
                .write_all(text.as_bytes())
                .and_then(|()| file.sync_data())
            {
                let note = format!("{now} ERROR [{thread}] log write failed: {error}\n");
                if let Err(_stderr_closed) = io::stderr().write_all(note.as_bytes()) {}
            }
        }
    }
}

/// The thread name prefixes of ADR 0012 whose panic unwinds to the thread that joins them.
pub const SUPERVISED: [&str; 5] = ["check-", "lower-", "derive-", "scan-", "save-"];

/// Whether a panic on the thread called `name` unwinds to its joiner (ADR 0012). Our prefixes
/// unwind and every other thread aborts, never a list of foreign thread names (D-72).
#[must_use]
pub fn is_supervised(name: &str) -> bool {
    SUPERVISED.iter().any(|prefix| name.starts_with(prefix))
}

/// Logs every panic with its thread, location and message. A supervised thread then unwinds to
/// its joiner, which turns it into Unknown "internal error"; any other thread aborts the app, so a
/// Session that panicked mid-command never serves a half-applied model (ADR 0012, spike E3).
pub fn log_panics() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map_or_else(|| "<unknown>".to_owned(), ToString::to_string);
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("<non-string panic payload>");
        line(Level::Panic, &format!("{location}: {message}"));
        default_hook(info);
        let current = thread::current();
        let name = current.name().unwrap_or("<unnamed>");
        if !is_supervised(name) {
            line(
                Level::Fatal,
                &format!("panic on {name}, which is not a supervised worker; aborting (ADR 0012)"),
            );
            std::process::abort();
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::is_supervised;

    #[test]
    fn our_worker_prefixes_unwind_and_every_other_thread_aborts() {
        assert!(is_supervised("check-f09e6538"));
        assert!(is_supervised("lower-12"));
        for other in [
            "main",
            "tokio-rt-worker",
            "monitor-checks",
            "checks",
            "<unnamed>",
        ] {
            assert!(!is_supervised(other), "{other}");
        }
    }
}
