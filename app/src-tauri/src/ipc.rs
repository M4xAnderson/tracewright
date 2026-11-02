//! The commands the UI calls and the types they exchange. Bindings come from ts-rs (IPC-2); the
//! tests write JSON fixtures that the UI tests replay, so both sides agree on the wire format.

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tracewright_engine::EngineEvent;
use tracewright_engine::LoadError;
use tracewright_engine::OpenError;
use tracewright_engine::Ports;
use tracewright_engine::ReloadOutcome;
use tracewright_engine::Session;
use tracewright_engine::Sink;
use tracewright_engine::adapters::StdDisk;
use tracewright_engine::adapters::SystemClock;
use tracewright_engine::adapters::ThreadSpawner;
use tracewright_engine::dto::JS_SAFE_INTEGER_MAX;
use tracewright_engine::dto::View;
use ts_rs::TS;

use crate::log;

/// LIM-2: the longest folder path the UI may send. Linux's `PATH_MAX`; Windows paths are shorter.
pub const PATH_BYTES_MAX: usize = 4096;

/// The open project, if any.
#[derive(Default)]
pub struct AppState {
    session: Mutex<Option<Session>>,
    // The folder `session` was opened from, for Reload. Set and read only while holding the
    // `session` lock, so the two always belong to the same project.
    disk: Mutex<Option<StdDisk>>,
}

/// The error of every command, tagged by `kind` (SAFE-3); the UI matches it exhaustively.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, thiserror::Error)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "command-error.ts")]
pub enum CommandError {
    /// The folder has no `arch.toml` (`P-OPEN-ERR-NOTPROJECT`).
    #[error("{path} is not a project")]
    NotAProject {
        /// The folder.
        path: String,
    },
    /// The folder or a project file in it cannot be read (`P-OPEN-ERR-PERM`).
    #[error("no permission to read {path}")]
    PermissionDenied {
        /// The folder.
        path: String,
    },
    /// A project file has a format newer than this app reads (`P-OPEN-ERR-NEWER`).
    #[error("{path} was saved in format {format}")]
    NewerFormat {
        /// The folder.
        path: String,
        /// The format of the first newer file in path order (file-format §12.2).
        #[ts(type = "number")]
        format: i64,
    },
    /// The folder did not open as a project for another reason (`P-OPEN-ERR`).
    #[error("could not open {path}: {cause}")]
    OpenFailed {
        /// The folder.
        path: String,
        /// Why, in plain words.
        cause: String,
    },
    /// Reload found a project that does not open; the open one stays as it was.
    #[error("could not reload: {cause}")]
    ReloadFailed {
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

/// What Reload found, with the view of the model it left (flows §4.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "reload-reply.ts")]
pub struct ReloadReply {
    /// The changed elements, or nothing changed.
    pub outcome: ReloadOutcome,
    /// What the canvas and the verdict panel show now.
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

/// The app's ports: events to `sink`, the system clock, a std thread per check.
pub fn std_ports(sink: Arc<dyn Sink>) -> Ports {
    Ports {
        sink,
        clock: Arc::new(SystemClock),
        spawner: Arc::new(ThreadSpawner),
    }
}

/// Opens the project folder at `path` with `ports`; the `StdDisk` is the folder Reload reads.
///
/// # Errors
///
/// `InvalidArgument` for an overlong path, `OpenFailed` when the folder does not open.
pub fn open_at(path: &str, ports: Ports) -> Result<(Session, StdDisk, Opened), CommandError> {
    if path.len() > PATH_BYTES_MAX {
        return Err(CommandError::InvalidArgument {
            reason: format!("the path is longer than {PATH_BYTES_MAX} bytes"),
        });
    }
    // SEC-1: from here on the project root is the canonical folder, symlinks resolved.
    let root = PathBuf::from(path)
        .canonicalize()
        .map_err(|error| open_error(path, &OpenError::Read(error)))?;
    if !root.is_dir() {
        return Err(CommandError::OpenFailed {
            path: path.to_owned(),
            cause: "it is not a folder".to_owned(),
        });
    }
    let disk = StdDisk::new(root.clone());
    let session = Session::open(&disk, ports).map_err(|error| open_error(path, &error))?;
    let name = root.file_name().map_or_else(
        || root.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let opened = Opened {
        name,
        path: path.to_owned(),
        view: session.view(),
    };
    Ok((session, disk, opened))
}

// flows §4.1: the four ways a folder fails to open, each with its own copy in the UI.
fn open_error(path: &str, error: &OpenError) -> CommandError {
    let path = path.to_owned();
    match error {
        OpenError::Load(LoadError::NotAProject) => CommandError::NotAProject { path },
        OpenError::Load(LoadError::VersionTooNew { found, .. })
            if found.unsigned_abs() <= JS_SAFE_INTEGER_MAX =>
        {
            CommandError::NewerFormat {
                path,
                format: *found,
            }
        }
        OpenError::Read(source) if source.kind() == io::ErrorKind::PermissionDenied => {
            CommandError::PermissionDenied { path }
        }
        // TXT-2: the cause in plain words, not the OS error.
        OpenError::Read(source) if source.kind() == io::ErrorKind::NotFound => {
            CommandError::OpenFailed {
                path,
                cause: "the folder does not exist".to_owned(),
            }
        }
        OpenError::Read(source) => CommandError::OpenFailed {
            path,
            cause: source.to_string(),
        },
        OpenError::Load(source) => CommandError::OpenFailed {
            path,
            cause: source.to_string(),
        },
    }
}

/// Re-reads the folder `disk` that `session` was opened from.
///
/// # Errors
///
/// `ReloadFailed` when the folder no longer reads or opens as a project.
pub fn reload_in(session: &mut Session, disk: &StdDisk) -> Result<ReloadReply, CommandError> {
    let outcome = session
        .reload(disk)
        .map_err(|error| CommandError::ReloadFailed {
            cause: match &error {
                // TXT-2: the cause in plain words, not the OS error.
                OpenError::Read(source) if source.kind() == io::ErrorKind::PermissionDenied => {
                    "you do not have permission to read the project folder".to_owned()
                }
                OpenError::Read(source) if source.kind() == io::ErrorKind::NotFound => {
                    "the project folder no longer exists".to_owned()
                }
                OpenError::Load(LoadError::NotAProject) => "arch.toml is missing".to_owned(),
                OpenError::Read(source) => source.to_string(),
                OpenError::Load(source) => source.to_string(),
            },
        })?;
    Ok(ReloadReply {
        outcome,
        view: session.view(),
    })
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
    let (session, disk, opened) = open_at(&path, std_ports(Arc::new(ChannelSink(events))))?;
    let mut guard = lock(&state.session)?;
    *lock(&state.disk)? = Some(disk);
    *guard = Some(session);
    drop(guard);
    log::line(log::Level::Info, &format!("opened {}", opened.name));
    Ok(opened)
}

/// Checks every property of the open project. Returns once the checks are started on their
/// workers; keys, progress and verdicts arrive as events (IPC-1).
#[tauri::command]
pub async fn run_checks(state: State<'_, AppState>) -> Result<(), CommandError> {
    let guard = lock(&state.session)?;
    let Some(session) = guard.as_ref() else {
        return Err(CommandError::NoProjectOpen);
    };
    session.run_checks();
    Ok(())
}

/// Reload (flows §4.5): re-reads the files changed on disk. The new keys arrive as an event;
/// checks wait for Check.
#[tauri::command]
pub async fn reload_project(state: State<'_, AppState>) -> Result<ReloadReply, CommandError> {
    let mut guard = lock(&state.session)?;
    let Some(session) = guard.as_mut() else {
        return Err(CommandError::NoProjectOpen);
    };
    let disk = lock(&state.disk)?;
    let Some(disk) = disk.as_ref() else {
        return Err(CommandError::NoProjectOpen);
    };
    let reply = reload_in(session, disk)?;
    log::line(log::Level::Info, "reloaded");
    Ok(reply)
}

// A poisoned lock means a command panicked while it held the Session; the panic hook aborts on
// command threads (ADR 0012), so this is only reachable if that policy changes.
fn lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, CommandError> {
    mutex.lock().map_err(|_poisoned| CommandError::Internal {
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
        let (_session, _disk, mut opened) =
            open_at(&engine_project(project), std_ports(Arc::new(Dropped)))
                .expect("the fixture opens");
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
        opened_fixture("opened-producer-consumer.json", "producer-consumer");
        opened_fixture("opened-half-done.json", "half-done");
        opened_fixture("opened-broken-file.json", "broken-file");
        opened_fixture("opened-no-behavior.json", "no-behavior");
    }

    // Opens `root`, which must fail, and writes the error as `name` with the stand-in path.
    fn open_error_fixture(name: &str, root: &Path) -> serde_json::Value {
        let Err(error) = open_at(
            root.to_str().expect("UTF-8 path"),
            std_ports(Arc::new(Dropped)),
        ) else {
            panic!("{} does not open", root.display());
        };
        let mut json = serde_json::to_value(&error).expect("serializes");
        json["path"] = serde_json::json!("/home/designer/notes");
        write_fixture(name, &json);
        json
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "builds a test folder, not a project file the app writes"
    )]
    fn folder_with_arch(arch: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("a temporary folder");
        std::fs::write(dir.path().join("arch.toml"), arch).expect("test file");
        dir
    }

    #[test]
    fn a_folder_that_is_not_a_project_does_not_open() {
        let json = open_error_fixture("open-not-a-project.json", Path::new(&engine_project("")));
        assert_eq!(
            json,
            serde_json::json!({ "kind": "notAProject", "path": "/home/designer/notes" })
        );
    }

    #[test]
    fn a_project_saved_by_a_newer_format_does_not_open() {
        let dir = folder_with_arch("format = 7\n");
        let json = open_error_fixture("open-newer-format.json", dir.path());
        assert_eq!(
            json,
            serde_json::json!({ "kind": "newerFormat", "path": "/home/designer/notes", "format": 7 })
        );
    }

    #[test]
    fn a_folder_that_cannot_be_read_is_a_permission_error() {
        let path = "/home/designer/notes";
        let denied = tracewright_engine::OpenError::Read(std::io::Error::from(
            std::io::ErrorKind::PermissionDenied,
        ));
        let error = open_error(path, &denied);
        assert_eq!(
            error,
            CommandError::PermissionDenied {
                path: path.to_owned()
            }
        );
        write_fixture(
            "open-permission-denied.json",
            &serde_json::to_value(&error).expect("serializes"),
        );
    }

    #[test]
    fn any_other_reason_is_named_in_plain_words() {
        let dir = folder_with_arch("[budget]\ntime_ms = 500\n");
        let json = open_error_fixture("open-failed.json", dir.path());
        assert_eq!(
            json,
            serde_json::json!({
                "kind": "openFailed",
                "path": "/home/designer/notes",
                "cause": "arch.toml: format is missing; add `format = 1`"
            })
        );
    }

    #[test]
    fn a_missing_folder_says_so() {
        let dir = tempfile::tempdir().expect("a temporary folder");
        let gone = dir.path().join("gone");
        let path = gone.to_str().expect("UTF-8 path");
        let opened = open_at(path, std_ports(Arc::new(Dropped)));
        assert_eq!(
            opened.err(),
            Some(CommandError::OpenFailed {
                path: path.to_owned(),
                cause: "the folder does not exist".to_owned()
            })
        );
    }

    #[test]
    fn an_overlong_path_is_rejected_before_it_is_used() {
        let path = "a/".repeat(PATH_BYTES_MAX);
        assert!(matches!(
            open_at(&path, std_ports(Arc::new(Dropped))),
            Err(CommandError::InvalidArgument { .. })
        ));
    }

    #[derive(Default)]
    struct Recorded(Mutex<Vec<EngineEvent>>);
    impl Sink for Recorded {
        fn emit(&self, event: EngineEvent) {
            self.0.lock().expect("test sink").push(event);
        }
    }

    // A copy of the engine fixture `project` in a temporary folder, opened, with its events
    // recorded from then on.
    fn open_copy(project: &str) -> (tempfile::TempDir, Session, StdDisk, Arc<Recorded>) {
        let dir = tempfile::tempdir().expect("a temporary folder");
        let from = PathBuf::from(engine_project(project));
        std::fs::create_dir(dir.path().join("elements")).expect("folder");
        for file in ["arch.toml", "elements/job.toml", "elements/sys.toml"] {
            std::fs::copy(from.join(file), dir.path().join(file)).expect("test file");
        }
        let sink = Arc::new(Recorded::default());
        let (session, disk, _opened) = open_at(
            dir.path().to_str().expect("UTF-8 path"),
            std_ports(Arc::clone(&sink) as Arc<dyn Sink>),
        )
        .expect("the copy opens");
        (dir, session, disk, sink)
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "edits a test project the way a text editor would"
    )]
    fn edit(dir: &tempfile::TempDir, file: &str, from: &str, to: &str) {
        let path = dir.path().join(file);
        let text = std::fs::read_to_string(&path).expect("test file");
        assert!(text.contains(from), "{file} contains {from:?}");
        std::fs::write(path, text.replace(from, to)).expect("test file");
    }

    // Reloads and writes the reply as `reload-<name>.json` and the events it emitted, if any, as
    // `reload-events-<name>.json`.
    fn reload_fixture(
        name: &str,
        session: &mut Session,
        disk: &StdDisk,
        sink: &Recorded,
    ) -> serde_json::Value {
        let reply = reload_in(session, disk).expect("the project reloads");
        let json = serde_json::to_value(&reply).expect("serializes");
        write_fixture(&format!("reload-{name}.json"), &json);
        let events = std::mem::take(&mut *sink.0.lock().expect("test sink"));
        if !events.is_empty() {
            write_fixture(
                &format!("reload-events-{name}.json"),
                &serde_json::to_value(&events).expect("serializes"),
            );
        }
        json
    }

    #[test]
    fn reload_reads_the_files_edited_on_disk_and_returns_the_new_view() {
        let (dir, mut session, disk, sink) = open_copy("one-job");

        let unchanged = reload_fixture("nothing-changed", &mut session, &disk, &sink);
        assert_eq!(
            unchanged["outcome"],
            serde_json::json!({ "kind": "nothingChanged" })
        );
        assert_eq!(unchanged["view"]["rev"], 0);

        edit(
            &dir,
            "elements/job.toml",
            "name = \"Job\"",
            "name = \"Nightly job\"",
        );
        edit(&dir, "elements/job.toml", "initial = 1", "initial = 0");
        let changed = reload_fixture("one-job", &mut session, &disk, &sink);
        assert_eq!(
            changed["outcome"],
            serde_json::json!({ "kind": "reloaded", "changed": ["job"], "added": [], "removed": [], "notLoaded": [] })
        );
        assert_eq!(changed["view"]["rev"], 1);
        assert_eq!(changed["view"]["elements"][0]["name"], "Nightly job");

        edit(&dir, "elements/job.toml", "initial = 0", "initial = ");
        let broken = reload_fixture("parse-error", &mut session, &disk, &sink);
        assert_eq!(broken["outcome"]["kind"], "reloaded");
        assert_eq!(
            broken["outcome"]["notLoaded"][0]["file"],
            "elements/job.toml"
        );
        assert_eq!(broken["outcome"]["notLoaded"][0]["line"], 11);
    }

    #[test]
    fn a_reload_that_finds_a_project_it_cannot_open_keeps_the_open_one() {
        let (dir, mut session, disk, _sink) = open_copy("one-job");
        edit(&dir, "arch.toml", "format = 1", "format = 2");

        let failed = reload_in(&mut session, &disk);

        let Err(error) = failed else {
            panic!("a newer arch.toml does not reload");
        };
        assert_eq!(
            error,
            CommandError::ReloadFailed {
                cause: "arch.toml: format version 2 is newer than this app supports (1)".to_owned()
            }
        );
        write_fixture(
            "reload-failed.json",
            &serde_json::to_value(&error).expect("serializes"),
        );
        assert_eq!(session.view().rev, 0);
    }
}
