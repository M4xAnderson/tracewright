//! In-memory ports and fixture projects for the Session tests.

use std::io;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use tracewright_engine::CheckKey;
use tracewright_engine::Disk;
use tracewright_engine::EngineEvent;
use tracewright_engine::KeyState;
use tracewright_engine::Session;
use tracewright_engine::Sink;
use tracewright_engine::dto::CurrentKeys;
use tracewright_engine::dto::VerdictDto;
use tracewright_format::ProjectFile;

/// The project as files in memory.
#[derive(Clone)]
pub struct MemDisk {
    pub files: Vec<ProjectFile>,
}

impl Disk for MemDisk {
    fn read_tree(&self) -> io::Result<Vec<ProjectFile>> {
        Ok(self.files.clone())
    }
}

impl MemDisk {
    /// One of the fixture projects under `tests/projects`.
    pub fn fixture(name: &str) -> MemDisk {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/projects")
            .join(name);
        let mut files = Vec::new();
        let mut pending = vec![root.clone()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).expect("test fixture") {
                let path = entry.expect("test fixture").path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    let relative = path
                        .strip_prefix(&root)
                        .expect("test fixture")
                        .to_str()
                        .expect("test fixture")
                        .replace('\\', "/");
                    files.push(ProjectFile {
                        path: relative,
                        bytes: std::fs::read(&path).expect("test fixture"),
                    });
                }
            }
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        MemDisk { files }
    }

    /// Replaces `from` by `to` in one file, which must contain it.
    pub fn edit(mut self, path: &str, from: &str, to: &str) -> MemDisk {
        let file = self
            .files
            .iter_mut()
            .find(|f| f.path == path)
            .unwrap_or_else(|| panic!("no file {path}"));
        let text = String::from_utf8(file.bytes.clone()).expect("test fixture");
        assert!(text.contains(from), "{path} does not contain {from:?}");
        file.bytes = text.replace(from, to).into_bytes();
        self
    }

    /// Adds or replaces a file.
    pub fn put(mut self, path: &str, text: &str) -> MemDisk {
        self.files.retain(|f| f.path != path);
        self.files.push(ProjectFile {
            path: path.to_owned(),
            bytes: text.as_bytes().to_vec(),
        });
        self
    }
}

/// A system with one child whose single transition `work` moves the token of `todo` to `done`.
pub fn one_job() -> MemDisk {
    MemDisk::fixture("one-job")
}

/// Records every event the Session emits.
#[derive(Clone, Default)]
pub struct RecordingSink {
    pub events: Arc<Mutex<Vec<EngineEvent>>>,
}

impl Sink for RecordingSink {
    fn emit(&self, event: EngineEvent) {
        self.events.lock().expect("test fixture").push(event);
    }
}

/// What one run of the checks emitted.
pub struct Run {
    pub events: Vec<EngineEvent>,
}

impl Run {
    pub fn current_keys(&self) -> &CurrentKeys {
        self.events
            .iter()
            .find_map(|event| match event {
                EngineEvent::CurrentKeys(keys) => Some(keys),
                EngineEvent::CheckQueued(_) | EngineEvent::Verdict(_) => None,
            })
            .expect("the run emitted currentKeys")
    }

    pub fn key_state(&self, property: &str) -> &KeyState {
        &self.current_keys().keys[property]
    }

    pub fn key(&self, property: &str) -> &CheckKey {
        match self.key_state(property) {
            KeyState::Key(key) => key,
            other => panic!("{property} has no key: {other:?}"),
        }
    }

    pub fn verdict(&self, property: &str) -> &VerdictDto {
        self.events
            .iter()
            .find_map(|event| match event {
                EngineEvent::Verdict(v) if v.property == property => Some(&v.verdict),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no verdict for {property}"))
    }
}

/// Given the project on `disk`, the opened Session (its events are dropped).
pub fn open(disk: &MemDisk) -> Session {
    Session::open(disk, Box::new(RecordingSink::default())).expect("the project opens")
}

/// Given the project on `disk`, when the checks run, the events emitted.
pub fn run_checks(disk: &MemDisk) -> Run {
    let sink = RecordingSink::default();
    let session = Session::open(disk, Box::new(sink.clone())).expect("the project opens");
    session.run_checks();
    let events = sink.events.lock().expect("test fixture").clone();
    Run { events }
}
