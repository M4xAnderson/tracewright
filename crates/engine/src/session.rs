//! The Session: the one authoritative model and the ports to the world (design §1, §3). So far
//! it opens a project through `Disk`, derives its problems and the keys of every property and runs
//! the checks on supervised workers (`Spawner`, `Clock`), emitting events to `Sink`. The budget
//! from `arch.toml`, edits, Save and the Journal join it with their tickets.

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::sync::Arc;

use tracewright_format::LoadError;
use tracewright_format::ProjectFile;
use tracewright_model::Behavior;
use tracewright_model::Element;
use tracewright_model::ElementId;
use tracewright_model::ElementKind;
use tracewright_model::Model;
use tracewright_model::Property;
use tracewright_model::PropertyId;
use tracewright_model::PropertyKind;
use tracewright_model::Scope;
use tracewright_net::Clock;
use tracewright_net::NetProperty;
use tracewright_net::UnknownReason;

use crate::check::CheckBudget;
use crate::disk_sync::Changes;
use crate::disk_sync::DiskSync;
use crate::disk_sync::FileHash;
use crate::dto;
use crate::dto::CheckQueued;
use crate::dto::CurrentKeys;
use crate::dto::EngineEvent;
use crate::dto::KeyState;
use crate::dto::ProblemDto;
use crate::dto::ReloadOutcome;
use crate::dto::Reloaded;
use crate::dto::View;
use crate::key::CheckKey;
use crate::key::check_key;
use crate::lower;
use crate::lower::LowerError;
use crate::lower::Lowered;
use crate::origin;
use crate::problems;
use crate::problems::Blocking;
use crate::supervise;
use crate::supervise::Job;
use crate::supervise::Spawner;
use crate::supervise::Supervisor;

/// The project directory, as the engine sees it.
pub trait Disk {
    /// Every file under the project directory, with paths relative to it.
    ///
    /// # Errors
    ///
    /// The IO error that stopped the read.
    fn read_tree(&self) -> io::Result<Vec<ProjectFile>>;

    /// The bytes of the project file at `path`, relative to the project directory, or `None`
    /// when it does not exist. Save reads each target with it just before writing.
    ///
    /// The default reads the whole tree; an adapter on a real folder reads the one file.
    ///
    /// # Errors
    ///
    /// The IO error that stopped the read.
    fn read_file(&self, path: &str) -> io::Result<Option<Vec<u8>>> {
        let files = self.read_tree()?;
        Ok(files
            .into_iter()
            .find(|file| file.path == path)
            .map(|file| file.bytes))
    }
}

/// Where the engine sends its events (the UI's Channel in the app, a recorder in tests).
pub trait Sink: Send + Sync {
    /// Delivers one event.
    fn emit(&self, event: EngineEvent);
}

/// Which of the files Save would write it may write (file-format §9.2), each list in the
/// order asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteCheck {
    /// Files whose bytes on disk are the ones the Session last read; a file it never read is
    /// writable while it does not exist.
    pub writable: Vec<String>,
    /// Files changed on disk since the Session last read them. Save skips them and reports them
    /// as conflicts, "changed on disk since load; Reload first" (design §4).
    pub changed_on_disk: Vec<String>,
}

/// Why a project does not open.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// The directory could not be read.
    #[error("could not read the project")]
    Read(#[source] io::Error),
    /// `arch.toml` did not load, a project-wide limit was exceeded or a file is newer.
    #[error(transparent)]
    Load(#[from] LoadError),
}

/// The ports a Session talks through besides `Disk` (design §3). Each is behind an `Arc` because
/// the check workers and their monitor hold it too.
pub struct Ports {
    /// Where events go.
    pub sink: Arc<dyn Sink>,
    /// What deadlines are measured on.
    pub clock: Arc<dyn Clock>,
    /// What starts the check workers.
    pub spawner: Arc<dyn Spawner>,
}

impl fmt::Debug for Ports {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ports")
            .field("clock", &self.clock)
            .finish_non_exhaustive()
    }
}

/// One open project.
pub struct Session {
    model: Model,
    // The revision of `model`; 0 at open. Edits advance it (design §4).
    rev: u64,
    ports: Ports,
    budget: CheckBudget,
    disk: DiskSync,
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("model", &self.model)
            .field("rev", &self.rev)
            .field("ports", &self.ports)
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
    /// The read failed or the project does not open (`LoadError`). A file that does not load
    /// is a problem, not an error.
    pub fn open(disk: &dyn Disk, ports: Ports) -> Result<Session, OpenError> {
        let files = disk.read_tree().map_err(OpenError::Read)?;
        let model = tracewright_format::load(&files)?;
        Ok(Session {
            model,
            rev: 0,
            ports,
            budget: CheckBudget::default(),
            disk: DiskSync::seen(&files),
        })
    }

    /// Re-reads the project on `disk`, the folder it was opened from (flows §4.5).
    ///
    /// # Errors
    ///
    /// The read failed or the project no longer opens (`LoadError`); the Session then keeps the
    /// model it had.
    pub fn reload(&mut self, disk: &dyn Disk) -> Result<ReloadOutcome, OpenError> {
        let files = disk.read_tree().map_err(OpenError::Read)?;
        let changes = self.disk.changes(&files);
        if changes.is_empty() {
            return Ok(ReloadOutcome::NothingChanged);
        }
        let model = tracewright_format::load(&files)?;
        let reloaded = reloaded(&model, &changes);
        self.model = model;
        self.disk = DiskSync::seen(&files);
        self.rev += 1;
        // Slice 0 drops the verdicts of a changed scope (`V-NOV-2`); Check is one key away. A
        // scope whose net did not change keeps its key, so its verdict stays current.
        self.derive_and_emit_keys();
        Ok(ReloadOutcome::Reloaded(reloaded))
    }

    /// Splits `paths`, the files Save would write, into the ones it may write and the ones that
    /// changed on disk since the Session last read them (FMT-2: never overwrite them silently).
    /// Save calls it right before writing; a Reload makes a changed file writable again.
    ///
    /// # Errors
    ///
    /// The IO error that stopped reading a file; Save then writes nothing.
    ///
    /// # Panics
    ///
    /// Only on a bug: a path that ends in neither list.
    pub fn check_writes(&self, disk: &dyn Disk, paths: &[&str]) -> io::Result<WriteCheck> {
        let mut check = WriteCheck {
            writable: Vec::new(),
            changed_on_disk: Vec::new(),
        };
        for path in paths {
            let on_disk = disk.read_file(path)?.map(|bytes| FileHash::of(&bytes));
            if on_disk == self.disk.last_seen(path) {
                check.writable.push((*path).to_owned());
            } else {
                check.changed_on_disk.push((*path).to_owned());
            }
        }
        assert_eq!(
            check.writable.len() + check.changed_on_disk.len(),
            paths.len(),
            "check_writes: every path is writable or changed"
        );
        Ok(check)
    }

    /// Replaces the budget of every check started from now on.
    ///
    /// # Panics
    ///
    /// If the budget allows no marking or no time: such a check could never decide anything.
    pub fn set_budget(&mut self, budget: CheckBudget) {
        assert!(budget.max_markings >= 1, "set_budget: at least one marking");
        assert!(!budget.time.is_zero(), "set_budget: some time");
        self.budget = budget;
    }

    /// The view of the open project at the current revision.
    #[must_use]
    pub fn view(&self) -> View {
        let problems = Blocking::new(&self.model).dtos();
        let system = self
            .model
            .elements
            .values()
            .find(|element| element.kind == ElementKind::SoftwareSystem);
        let Some(system) = system else {
            return View {
                rev: self.rev,
                system: None,
                elements: Vec::new(),
                relationships: Vec::new(),
                properties: Vec::new(),
                problems,
            };
        };
        let children: Vec<&Element> = self
            .model
            .elements
            .values()
            .filter(|element| element.parent.as_ref() == Some(&system.id))
            .collect();
        let is_child =
            |id: &tracewright_model::ElementId| children.iter().any(|child| child.id == *id);
        let relationships = self
            .model
            .relationships
            .values()
            .filter(|r| is_child(&r.source) || is_child(&r.target))
            .map(|r| dto::RelationshipView {
                id: r.id.to_string(),
                source: r.source.to_string(),
                target: r.target.to_string(),
                meaning: r.meaning.clone(),
            })
            .collect();
        let properties = std::iter::once(system)
            .chain(children.iter().copied())
            .flat_map(|element| {
                element.properties.iter().map(|property| dto::PropertyView {
                    id: property.id.to_string(),
                    label: property.label.clone(),
                    kind: property_kind(&property.kind),
                    template: match &property.kind {
                        PropertyKind::Template { template, .. } => Some(template.clone()),
                        PropertyKind::DeadlockFree
                        | PropertyKind::NoCapacityExceeded
                        | PropertyKind::NoDeadTransitions
                        | PropertyKind::Never { .. }
                        | PropertyKind::AlwaysReachable { .. } => None,
                    },
                    scope: dto::ScopeView {
                        element: element.id.to_string(),
                        behavior: match property.behavior {
                            Behavior::Own => dto::Behavior::Own,
                            Behavior::Combined => dto::Behavior::Combined,
                        },
                    },
                })
            })
            .collect();
        View {
            rev: self.rev,
            system: Some(element_view(system)),
            elements: children.iter().map(|child| element_view(child)).collect(),
            relationships,
            properties,
            problems,
        }
    }

    /// Derives the key of every property and emits them as `CurrentKeys`, then queues a check
    /// for each keyed property (`CheckQueued`). The checks run on supervised workers, which emit
    /// `CheckProgress` and then the `Verdict`; this returns once they are started.
    ///
    /// # Panics
    ///
    /// Only on a bug: a request for a scope that did not lower.
    pub fn run_checks(&self) {
        let Derived { lowered, requests } = self.derive_and_emit_keys();
        for request in &requests {
            self.ports.sink.emit(EngineEvent::CheckQueued(CheckQueued {
                property: request.property.to_string(),
                key: request.key.clone(),
            }));
        }
        let jobs = requests
            .into_iter()
            .map(|request| Job {
                property: request.property.to_string(),
                key: request.key,
                lowered: Arc::clone(
                    lowered[&request.scope]
                        .as_ref()
                        .expect("a request is made only for a scope that lowered"),
                ),
                net_property: request.net_property,
            })
            .collect();
        let supervisor = Supervisor {
            sink: Arc::clone(&self.ports.sink),
            clock: Arc::clone(&self.ports.clock),
            spawner: Arc::clone(&self.ports.spawner),
            budget: self.budget,
        };
        supervise::submit(supervisor, jobs);
    }
}

// The lowered scopes of one revision and the check each keyed property asks for.
struct Derived {
    lowered: BTreeMap<Scope, Result<Arc<Lowered>, LowerError>>,
    requests: Vec<Request>,
}

impl Session {
    // Derives the key of every property and emits them as `CurrentKeys` (design §4: the complete
    // set of the revision, before any verdict for it).
    fn derive_and_emit_keys(&self) -> Derived {
        let blocking = Blocking::new(&self.model);
        let mut lowered = BTreeMap::new();
        let mut keys = BTreeMap::new();
        let mut requests = Vec::new();
        for element in self.model.elements.values() {
            for property in &element.properties {
                let scope = Scope {
                    element: element.id.clone(),
                    behavior: property.behavior,
                };
                let state = self.key_state(&blocking, &mut lowered, property, scope, &mut requests);
                keys.insert(property.id.to_string(), state);
            }
        }
        assert!(
            requests.len() <= keys.len(),
            "derive_and_emit_keys: at most one check per property"
        );
        self.ports.sink.emit(EngineEvent::CurrentKeys(CurrentKeys {
            rev: self.rev,
            keys,
        }));
        Derived { lowered, requests }
    }

    // The key state of one property (design §3). A problem that blocks the scope wins over a
    // lowered net, also over nothing to check: the scope could have more behavior than what
    // loaded (file-format §10.5). A keyed property adds its check to `requests`.
    fn key_state(
        &self,
        blocking: &Blocking,
        lowered: &mut BTreeMap<Scope, Result<Arc<Lowered>, LowerError>>,
        property: &Property,
        scope: Scope,
        requests: &mut Vec<Request>,
    ) -> KeyState {
        let lowering = lowered
            .entry(scope.clone())
            .or_insert_with(|| lower::lower(&self.model, &scope).map(Arc::new));
        let lowering_lines = match &*lowering {
            Err(LowerError::ModelIncomplete(lines)) => lines.as_slice(),
            Err(LowerError::NothingToCheck) | Ok(_) => &[],
        };
        if let Some(incomplete) = blocking.incomplete(&scope, lowering_lines) {
            return KeyState::Unknown(incomplete);
        }
        match lowering {
            Err(LowerError::NothingToCheck) => KeyState::NothingToCheck,
            Err(LowerError::ModelIncomplete(lines)) => KeyState::Unknown(origin::unknown_dto(
                &UnknownReason::ModelIncomplete(lines.clone()),
            )),
            Ok(scope_net) => match checked_property(property) {
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
                Err(lines) => KeyState::Unknown(origin::unknown_dto(
                    &UnknownReason::PropertyNotCheckable(lines),
                )),
            },
        }
    }
}

// The elements behind the changed files, in the model loaded from them. Only element files name
// elements; a changed `arch.toml` or layout file reloads without one.
fn reloaded(model: &Model, changes: &Changes) -> Reloaded {
    let mut reloaded = Reloaded {
        changed: Vec::new(),
        added: Vec::new(),
        removed: Vec::new(),
        not_loaded: Vec::new(),
    };
    let present = |path: &str, into: &mut Vec<String>, not_loaded: &mut Vec<ProblemDto>| {
        if let Some(not) = model.files_not_loaded.get(path) {
            not_loaded.push(problems::problem_dto(&not.problem));
        } else if let Some(id) = element_id(path) {
            if model.elements.contains_key(&id) {
                into.push(id.to_string());
            }
        }
    };
    for path in &changes.modified {
        present(path, &mut reloaded.changed, &mut reloaded.not_loaded);
    }
    for path in &changes.added {
        present(path, &mut reloaded.added, &mut reloaded.not_loaded);
    }
    reloaded.removed = changes
        .removed
        .iter()
        .filter_map(|path| element_id(path))
        .map(|id| id.to_string())
        .collect();
    reloaded.not_loaded.sort_by(|a, b| a.file.cmp(&b.file));
    reloaded
}

// `elements/<id>.toml` names element `<id>`.
fn element_id(path: &str) -> Option<ElementId> {
    let stem = path.strip_prefix("elements/")?.strip_suffix(".toml")?;
    ElementId::parse(stem).ok()
}

fn element_view(element: &Element) -> dto::ElementView {
    dto::ElementView {
        id: element.id.to_string(),
        name: element.name.clone(),
    }
}

fn property_kind(kind: &PropertyKind) -> dto::PropertyKind {
    match kind {
        PropertyKind::DeadlockFree => dto::PropertyKind::DeadlockFree,
        PropertyKind::NoCapacityExceeded => dto::PropertyKind::NoCapacityExceeded,
        PropertyKind::NoDeadTransitions => dto::PropertyKind::NoDeadTransitions,
        PropertyKind::Never { .. } => dto::PropertyKind::Never,
        PropertyKind::AlwaysReachable { .. } => dto::PropertyKind::AlwaysReachable,
        PropertyKind::Template { .. } => dto::PropertyKind::Template,
    }
}

// PL §10.1: an entry with E4xx or E114 problems is not checked; its lines come first, ordered by
// code, then the slice gate line (S-07).
fn checked_property(property: &Property) -> Result<NetProperty, Vec<String>> {
    let lowered = lower_property(&property.kind);
    if property.problems.is_empty() {
        return lowered;
    }
    let mut problems: Vec<_> = property.problems.iter().collect();
    problems.sort_by_key(|problem| (problem.code, &problem.message));
    let mut lines: Vec<String> = problems.iter().map(|p| p.message.clone()).collect();
    if let Err(gate) = lowered {
        lines.extend(gate);
    }
    Err(lines)
}

// The property of a scope that lowered (D-64). Slice 0 checks deadlock-free with `final none`
// (D-25) and no capacity exceeded; user properties wait for slice 4, with the line the user sees
// (V57).
fn lower_property(kind: &PropertyKind) -> Result<NetProperty, Vec<String>> {
    match kind {
        PropertyKind::DeadlockFree => Ok(NetProperty::DeadlockFree),
        PropertyKind::NoCapacityExceeded => Ok(NetProperty::CapacityNeverExceeded),
        PropertyKind::NoDeadTransitions
        | PropertyKind::Never { .. }
        | PropertyKind::AlwaysReachable { .. } => {
            Err(vec!["user properties are checked from slice 4".to_owned()])
        }
        // PL S-06: T5 is checked from slice 1; every other template from slice 4 (S-02).
        PropertyKind::Template { template, .. } => Err(vec![if template == "pool-share" {
            "never holds the whole pool is checked from slice 1".to_owned()
        } else {
            "user properties are checked from slice 4".to_owned()
        }]),
    }
}
