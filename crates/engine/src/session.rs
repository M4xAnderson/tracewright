//! The Session: the one authoritative model and the ports to the world (design §1, §3). So far
//! it opens a project through `Disk`, derives the keys of every property and runs the checks,
//! emitting events to `Sink`. The worker threads, the budget from `arch.toml`, edits, Save and the
//! remaining ports (Journal, Clock, Spawner) join it with their tickets.

use std::collections::BTreeMap;
use std::fmt;
use std::io;

use tracewright_format::LoadError;
use tracewright_format::ProjectFile;
use tracewright_model::Model;
use tracewright_model::PropertyId;
use tracewright_model::PropertyKind;
use tracewright_model::Scope;
use tracewright_net::Budget;
use tracewright_net::NetProperty;
use tracewright_net::UnknownReason;

use crate::check;
use crate::check::Verdict;
use crate::key::CheckKey;
use crate::key::check_key;
use crate::lower;
use crate::lower::LowerError;
use crate::lower::Lowered;

/// The engine's markings budget per check until `arch.toml`'s `[budget]` is read: the
/// `max_markings` of the file-format §5 example. Spike E1 measured 0.2–1.3 million markings per
/// second per thread, so it stays in the seconds range.
pub const DEFAULT_MAX_MARKINGS: u64 = 2_000_000;

/// The project directory, as the engine sees it.
pub trait Disk {
    /// Every file under the project directory, with paths relative to it.
    ///
    /// # Errors
    ///
    /// The IO error that stopped the read.
    fn read_tree(&self) -> io::Result<Vec<ProjectFile>>;
}

/// Where the engine sends its events (the UI's Channel in the app, a recorder in tests).
pub trait Sink: Send + Sync {
    /// Delivers one event.
    fn emit(&self, event: EngineEvent);
}

/// What a property's check is keyed by, before any check runs (design §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyState {
    /// The property lowered; a check with this key decides it.
    Key(CheckKey),
    /// The property ends as Unknown without a check: model incomplete or not checkable.
    Unknown(UnknownReason),
    /// No behavior model to check: no verdict at all (D-13).
    NothingToCheck,
}

/// An event the Session emits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineEvent {
    /// The complete key set of the model, sent before any verdict.
    CurrentKeys {
        /// Per property.
        keys: BTreeMap<PropertyId, KeyState>,
    },
    /// The verdict of one check.
    Verdict {
        /// The property.
        property: PropertyId,
        /// The key of the net that was checked.
        key: CheckKey,
        /// The verdict.
        verdict: Verdict,
    },
}

/// Why a project does not open.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// The directory could not be read.
    #[error("could not read the project")]
    Read(#[source] io::Error),
    /// A file did not load.
    #[error(transparent)]
    Load(#[from] LoadError),
}

/// One open project.
pub struct Session {
    model: Model,
    sink: Box<dyn Sink>,
    budget: Budget,
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("model", &self.model)
            .field("budget", &self.budget)
            .finish_non_exhaustive()
    }
}

struct Request {
    property: PropertyId,
    key: CheckKey,
    scope: Scope,
    net_property: NetProperty,
}

impl Session {
    /// Reads and loads the project on `disk`.
    ///
    /// # Errors
    ///
    /// The read failed or a file did not load.
    pub fn open(disk: &dyn Disk, sink: Box<dyn Sink>) -> Result<Session, OpenError> {
        let files = disk.read_tree().map_err(OpenError::Read)?;
        let model = tracewright_format::load(&files)?;
        Ok(Session {
            model,
            sink,
            budget: Budget {
                max_markings: DEFAULT_MAX_MARKINGS,
            },
        })
    }

    /// Derives the key of every property, emits them as `CurrentKeys`, then checks each keyed
    /// property and emits its `Verdict`.
    ///
    /// # Panics
    ///
    /// Only on a bug: a check explored another net than its key names.
    pub fn run_checks(&self) {
        let mut lowered: BTreeMap<Scope, Result<Lowered, LowerError>> = BTreeMap::new();
        let mut keys = BTreeMap::new();
        let mut requests = Vec::new();
        for element in self.model.elements.values() {
            for property in &element.properties {
                let scope = Scope {
                    element: element.id.clone(),
                    behavior: property.behavior,
                };
                let lowering = lowered
                    .entry(scope.clone())
                    .or_insert_with(|| lower::lower(&self.model, &scope));
                let state = match lowering {
                    Err(LowerError::NothingToCheck) => KeyState::NothingToCheck,
                    Err(LowerError::ModelIncomplete(lines)) => {
                        KeyState::Unknown(UnknownReason::ModelIncomplete(lines.clone()))
                    }
                    Err(LowerError::NotCheckedYet(lines)) => {
                        KeyState::Unknown(UnknownReason::PropertyNotCheckable(lines.clone()))
                    }
                    Ok(scope_net) => match lower_property(&property.kind) {
                        Ok(net_property) => {
                            let key = check_key(&scope_net.flat, &net_property);
                            requests.push(Request {
                                property: property.id.clone(),
                                key: key.clone(),
                                scope,
                                net_property,
                            });
                            KeyState::Key(key)
                        }
                        Err(lines) => KeyState::Unknown(UnknownReason::PropertyNotCheckable(lines)),
                    },
                };
                keys.insert(property.id.clone(), state);
            }
        }
        self.sink.emit(EngineEvent::CurrentKeys { keys });
        for request in requests {
            let scope_net = lowered[&request.scope]
                .as_ref()
                .expect("a request is made only for a scope that lowered");
            let checked = check::run(scope_net, &request.net_property, &self.budget);
            assert_eq!(
                checked.key, request.key,
                "run_checks: the worker explored another net than the key names"
            );
            self.sink.emit(EngineEvent::Verdict {
                property: request.property,
                key: checked.key,
                verdict: checked.verdict,
            });
        }
    }
}

// The property of a scope that lowered (D-64). Slice 0 checks deadlock-free with `final none`
// (D-25); the other kinds are not checked yet, with the line the user sees.
fn lower_property(kind: &PropertyKind) -> Result<NetProperty, Vec<String>> {
    match kind {
        PropertyKind::DeadlockFree => Ok(NetProperty::DeadlockFree),
        PropertyKind::NoCapacityExceeded => {
            Err(vec!["no capacity exceeded is not checked yet".to_owned()])
        }
        PropertyKind::NoDeadTransitions
        | PropertyKind::Never { .. }
        | PropertyKind::AlwaysReachable { .. } => {
            Err(vec!["user properties are checked from slice 4".to_owned()])
        }
    }
}
