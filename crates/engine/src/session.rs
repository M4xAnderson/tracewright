//! The Session: the one authoritative model and the ports to the world (design §1, §3). So far
//! it opens a project through `Disk`, derives the keys of every property and runs the checks on
//! supervised workers (`Spawner`, `Clock`), emitting events to `Sink`. The budget from
//! `arch.toml`, edits, Save and the Journal join it with their tickets.

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
    /// A file did not load.
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
    /// The read failed or a file did not load.
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
        let mut lowered: BTreeMap<Scope, Result<Arc<Lowered>, LowerError>> = BTreeMap::new();
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
                    .or_insert_with(|| lower::lower(&self.model, &scope).map(Arc::new));
                let state = match lowering {
                    Err(LowerError::NothingToCheck) => KeyState::NothingToCheck,
                    Err(LowerError::ModelIncomplete(lines)) => KeyState::Unknown(
                        origin::unknown_dto(&UnknownReason::ModelIncomplete(lines.clone())),
                    ),
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
                        Err(lines) => KeyState::Unknown(origin::unknown_dto(
                            &UnknownReason::PropertyNotCheckable(lines),
                        )),
                    },
                };
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
    }
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
    }
}
