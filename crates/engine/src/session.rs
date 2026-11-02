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
use crate::dto;
use crate::dto::CheckQueued;
use crate::dto::CurrentKeys;
use crate::dto::EngineEvent;
use crate::dto::KeyState;
use crate::dto::View;
use crate::key::CheckKey;
use crate::key::check_key;
use crate::lower;
use crate::lower::LowerError;
use crate::lower::Lowered;
use crate::origin;
use crate::problems::Blocking;
use crate::projection;
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
}

/// Where the engine sends its events (the UI's Channel in the app, a recorder in tests).
pub trait Sink: Send + Sync {
    /// Delivers one event.
    fn emit(&self, event: EngineEvent);
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
        })
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
            .map(projection::relationship_view)
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
            system: Some(projection::element_view(&self.model, system)),
            elements: children
                .iter()
                .map(|child| projection::element_view(&self.model, child))
                .collect(),
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
        let blocking = Blocking::new(&self.model);
        let mut lowered: BTreeMap<Scope, Result<Arc<Lowered>, LowerError>> = BTreeMap::new();
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
        self.ports.sink.emit(EngineEvent::CurrentKeys(CurrentKeys {
            rev: self.rev,
            keys,
        }));
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

impl Session {
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
