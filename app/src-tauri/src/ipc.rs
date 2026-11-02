//! The commands the UI calls and the types they exchange. Bindings come from ts-rs (IPC-2); the
//! tests write JSON fixtures that the UI tests replay, so both sides agree on the wire format.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tracewright_engine::EngineEvent;
use tracewright_engine::Session;
use tracewright_engine::Sink;
use tracewright_engine::adapters::StdDisk;
use tracewright_engine::dto::View;
use ts_rs::TS;

use crate::log;

/// LIM-2: the longest folder path the UI may send. Linux's `PATH_MAX`; Windows paths are shorter.
pub const PATH_BYTES_MAX: usize = 4096;

/// The open project, if any. Behind an `Arc` because the checks run on their own thread.
#[derive(Default)]
pub struct AppState {
    session: Arc<Mutex<Option<Session>>>,
}

/// The error of every command, tagged by `kind` (SAFE-3); the UI matches it exhaustively.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, thiserror::Error)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "command-error.ts")]
pub enum CommandError {
    /// The folder did not open as a project (`P-OPEN-ERR`).
    #[error("could not open {path}: {cause}")]
    OpenFailed {
        /// The folder.
        path: String,
        /// Why, in plain words.
        cause: String,
    },
    /// The UI sent an argument outside its limits (SEC-2).
    #[error("invalid argument: {reason}")]
    InvalidArgument {
        /// What is wrong.
        reason: String,
    },
    /// A command that needs an open project came without one.
    #[error("no project is open")]
    NoProjectOpen,
    /// A bug in the app; details are in the log.
    #[error("internal error: {message}")]
    Internal {
        /// What failed.
        message: String,
    },
}

/// A project the UI can show: its folder and the view of its model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "opened.ts")]
pub struct Opened {
    /// The folder's name, shown as the project name.
    pub name: String,
    /// The folder, as the user chose it.
    pub path: String,
    /// What the canvas and the verdict panel show.
    pub view: View,
}

// Sends the Session's events over the UI's Channel.
struct ChannelSink(Channel<EngineEvent>);

impl Sink for ChannelSink {
    fn emit(&self, event: EngineEvent) {
        if let Err(error) = self.0.send(event) {
            // The webview is gone or reloading; its next open registers a new Channel.
            log::line(log::Level::Warn, &format!("event not delivered: {error}"));
        }
    }
}

/// Opens the project folder at `path` with `sink` for its events.
///
/// # Errors
///
/// `InvalidArgument` for an overlong path, `OpenFailed` when the folder does not open.
pub fn open_at(path: &str, sink: Box<dyn Sink>) -> Result<(Session, Opened), CommandError> {
    if path.len() > PATH_BYTES_MAX {
        return Err(CommandError::InvalidArgument {
            reason: format!("the path is longer than {PATH_BYTES_MAX} bytes"),
        });
    }
    let failed = |cause: String| CommandError::OpenFailed {
        path: path.to_owned(),
        cause,
    };
    // SEC-1: from here on the project root is the canonical folder, symlinks resolved.
    let root = PathBuf::from(path)
        .canonicalize()
        .map_err(|error| failed(error.to_string()))?;
    if !root.is_dir() {
        return Err(failed("it is not a folder".to_owned()));
    }
    let session =
        Session::open(&StdDisk::new(root.clone()), sink).map_err(|error| failed(cause(&error)))?;
    let name = root.file_name().map_or_else(
        || root.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let opened = Opened {
        name,
        path: path.to_owned(),
        view: session.view(),
    };
    Ok((session, opened))
}

// The innermost message: "could not read the project: Permission denied (os error 13)" reads
// better as one sentence.
fn cause(error: &tracewright_engine::OpenError) -> String {
    match error {
        tracewright_engine::OpenError::Read(source) => source.to_string(),
        tracewright_engine::OpenError::Load(source) => source.to_string(),
    }
}

/// Shows the native folder dialog; `None` when the user cancels it. The path stays untrusted
/// until `open_project` checks it.
#[tauri::command]
pub async fn pick_project_folder(app: AppHandle) -> Result<Option<String>, CommandError> {
    let dialog = app.dialog().clone();
    let picked = tauri::async_runtime::spawn_blocking(move || dialog.file().blocking_pick_folder())
        .await
        .map_err(|error| CommandError::Internal {
            message: format!("the folder dialog failed: {error}"),
        })?;
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|error| CommandError::Internal {
        message: format!("the folder dialog returned no local path: {error}"),
    })?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

/// Opens the project at `path` in place of the open one; its events go to `events`.
#[tauri::command]
pub async fn open_project(
    state: State<'_, AppState>,
    path: String,
    events: Channel<EngineEvent>,
) -> Result<Opened, CommandError> {
    let (session, opened) = open_at(&path, Box::new(ChannelSink(events)))?;
    *lock(&state.session)? = Some(session);
    log::line(log::Level::Info, &format!("opened {}", opened.name));
    Ok(opened)
}

/// Checks every property of the open project. Returns at once; keys and verdicts arrive as
/// events (IPC-1).
#[tauri::command]
pub async fn run_checks(state: State<'_, AppState>) -> Result<(), CommandError> {
    if lock(&state.session)?.is_none() {
        return Err(CommandError::NoProjectOpen);
    }
    let session = Arc::clone(&state.session);
    thread::Builder::new()
        .name("checks".to_owned())
        .spawn(move || match lock(&session) {
            Ok(guard) => {
                if let Some(session) = guard.as_ref() {
                    session.run_checks();
                } else {
                    log::line(log::Level::Warn, "the project closed before its checks ran");
                }
            }
            Err(error) => log::line(log::Level::Error, &error.to_string()),
        })
        .map_err(|error| CommandError::Internal {
            message: format!("could not start the checks: {error}"),
        })?;
    Ok(())
}

// A poisoned lock means a check panicked while it held the Session; the panic is in the log.
fn lock(
    session: &Mutex<Option<Session>>,
) -> Result<std::sync::MutexGuard<'_, Option<Session>>, CommandError> {
    session.lock().map_err(|_poisoned| CommandError::Internal {
        message: "a check stopped because of a bug; see the log".to_owned(),
    })
}

/// A screen the UI can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "screen.ts")]
pub enum Screen {
    Start,
    /// A project is open: header, canvas and verdict panel.
    Project,
}

/// Sent once the UI has rendered its first screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "ui-ready.ts")]
pub struct UiReady {
    pub screen: Screen,
}

/// What the app does once the UI is ready; chosen at start from `TRACEWRIGHT_SMOKE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Smoke {
    Off,
    /// The real-window smoke test: exit 0 once the start screen is shown.
    ExitWhenStartScreenShown,
}

/// The UI rendered its first screen; logged, so a blank window is visible in the log.
#[tauri::command]
pub async fn ui_ready(app: AppHandle, ready: UiReady) {
    log::line(
        log::Level::Info,
        &format!("UI ready: {:?} screen shown", ready.screen),
    );
    let smoke = *app.state::<Smoke>();
    if smoke == Smoke::ExitWhenStartScreenShown && ready.screen == Screen::Start {
        log::line(log::Level::Info, "smoke test passed; exiting");
        app.exit(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    #[expect(
        clippy::disallowed_methods,
        reason = "writes a test fixture for the UI, not a project file"
    )]
    fn ui_ready_fixture() {
        let ready = UiReady {
            screen: Screen::Start,
        };
        let json = serde_json::to_string_pretty(&ready).expect("UiReady serializes");
        assert_eq!(
            json, "{\n  \"screen\": \"start\"\n}",
            "the UI sends the screen in camelCase"
        );

        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/src/ipc/fixtures/ui-ready.json");
        std::fs::write(&fixture, format!("{json}\n")).expect("fixture written");
        let parsed: UiReady = serde_json::from_str(&json).expect("UiReady deserializes");
        assert_eq!(parsed, ready, "the fixture round-trips");
    }

    fn engine_project(name: &str) -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/engine/tests/projects")
            .join(name)
            .to_str()
            .expect("UTF-8 path")
            .to_owned()
    }

    struct Dropped;
    impl Sink for Dropped {
        fn emit(&self, _event: EngineEvent) {}
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "writes a test fixture for the UI, not a project file"
    )]
    fn write_fixture(name: &str, json: &serde_json::Value) {
        let text = serde_json::to_string_pretty(json).expect("JSON");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../ui/src/ipc/fixtures")
            .join(name);
        std::fs::write(&fixture, format!("{text}\n")).expect("fixture written");
    }

    // The path differs per checkout, so the fixture carries a stand-in the UI tests show.
    fn opened_fixture(name: &str, project: &str) -> Opened {
        let (_session, mut opened) =
            open_at(&engine_project(project), Box::new(Dropped)).expect("the fixture opens");
        assert_eq!(
            opened.name, project,
            "the project is named after its folder"
        );
        opened.path = format!("/home/designer/{}", opened.name);
        write_fixture(name, &serde_json::to_value(&opened).expect("serializes"));
        opened
    }

    #[test]
    fn opening_a_project_folder_returns_its_name_and_view() {
        let opened = opened_fixture("opened-opposite-locks.json", "e1b-opposite-locks");
        let system = opened
            .view
            .system
            .expect("the fixture has a software system");
        assert_eq!(system.name, "Two components, two locks");
        opened_fixture("opened-worker-pool.json", "e1c-worker-pool");
        opened_fixture("opened-ns-v08.json", "ns-v08");
        opened_fixture("opened-one-job.json", "one-job");
        opened_fixture("opened-half-done.json", "half-done");
        opened_fixture("opened-broken-file.json", "broken-file");
        opened_fixture("opened-no-behavior.json", "no-behavior");
    }

    #[test]
    fn a_folder_that_is_not_a_project_does_not_open() {
        let path = engine_project("");
        let Err(error) = open_at(&path, Box::new(Dropped)) else {
            panic!("tests/projects has no arch.toml");
        };
        let json = serde_json::to_value(&error).expect("serializes");
        assert_eq!(json["kind"], "openFailed");
        assert_eq!(
            json["cause"],
            "this folder is not a project: it has no arch.toml"
        );
        write_fixture(
            "open-failed.json",
            &serde_json::json!({ "kind": "openFailed", "path": "/home/designer/notes", "cause": json["cause"] }),
        );
    }

    #[test]
    fn an_overlong_path_is_rejected_before_it_is_used() {
        let path = "a/".repeat(PATH_BYTES_MAX);
        assert!(matches!(
            open_at(&path, Box::new(Dropped)),
            Err(CommandError::InvalidArgument { .. })
        ));
    }
}
