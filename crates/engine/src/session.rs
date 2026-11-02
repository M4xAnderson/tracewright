//! The Session: the one authoritative model and the ports to the world (design §1, §3). So far
//! it opens a project through `Disk`, derives its problems and the keys of every property and runs
//! the checks, emitting events to `Sink`. The worker threads, the budget from `arch.toml`, edits, Save and the
//! remaining ports (Journal, Clock, Spawner) join it with their tickets.

use std::collections::BTreeMap;
use std::fmt;
use std::io;

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
use tracewright_net::Budget;
use tracewright_net::NetProperty;
use tracewright_net::UnknownReason;

use crate::check;
use crate::dto;
use crate::dto::CheckQueued;
use crate::dto::CurrentKeys;
use crate::dto::EngineEvent;
use crate::dto::KeyState;
use crate::dto::VerdictEvent;
use crate::dto::View;
use crate::key::CheckKey;
use crate::key::check_key;
use crate::lower;
use crate::lower::LowerError;
use crate::lower::Lowered;
use crate::origin;
use crate::problems::Blocking;

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

/// One open project.
pub struct Session {
    model: Model,
    // The revision of `model`; 0 at open. Edits advance it (design §4).
    rev: u64,
    sink: Box<dyn Sink>,
    budget: Budget,
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("model", &self.model)
            .field("rev", &self.rev)
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
    pub fn open(disk: &dyn Disk, sink: Box<dyn Sink>) -> Result<Session, OpenError> {
        let files = disk.read_tree().map_err(OpenError::Read)?;
        let model = tracewright_format::load(&files)?;
        Ok(Session {
            model,
            rev: 0,
            sink,
            budget: Budget {
                max_markings: DEFAULT_MAX_MARKINGS,
            },
        })
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
    /// for each keyed property (`CheckQueued`), runs it and emits its `Verdict`.
    ///
    /// # Panics
    ///
    /// Only on a bug: a check explored another net than its key names.
    pub fn run_checks(&self) {
        let blocking = Blocking::new(&self.model);
        let mut lowered: BTreeMap<Scope, Result<Lowered, LowerError>> = BTreeMap::new();
        let mut keys = BTreeMap::new();
        let mut requests = Vec::new();
        for element in self.model.elements.values() {
            for property in &element.properties {
                let scope = Scope {
                    element: element.id.clone(),
                    behavior: property.behavior,
                };
                // A problem that blocks the scope wins over lowering, also over nothing to check:
                // the scope could have more behavior than what loaded (file-format §10.5).
                if let Some(incomplete) = blocking.incomplete(&scope) {
                    keys.insert(property.id.to_string(), KeyState::Unknown(incomplete));
                    continue;
                }
                let lowering = lowered
                    .entry(scope.clone())
                    .or_insert_with(|| lower::lower(&self.model, &scope));
                let state = match lowering {
                    Err(LowerError::NothingToCheck) => KeyState::NothingToCheck,
                    Err(LowerError::ModelIncomplete(lines)) => KeyState::Unknown(
                        origin::unknown_dto(&UnknownReason::ModelIncomplete(lines.clone())),
                    ),
                    Err(LowerError::NotCheckedYet(lines)) => KeyState::Unknown(
                        origin::unknown_dto(&UnknownReason::PropertyNotCheckable(lines.clone())),
                    ),
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
                };
                keys.insert(property.id.to_string(), state);
            }
        }
        self.sink.emit(EngineEvent::CurrentKeys(CurrentKeys {
            rev: self.rev,
            keys,
        }));
        for request in &requests {
            self.sink.emit(EngineEvent::CheckQueued(CheckQueued {
                property: request.property.to_string(),
                key: request.key.clone(),
            }));
        }
        for request in requests {
            let scope_net = lowered[&request.scope]
                .as_ref()
                .expect("a request is made only for a scope that lowered");
            let checked = check::run(scope_net, &request.net_property, &self.budget);
            assert_eq!(
                checked.key, request.key,
                "run_checks: the worker explored another net than the key names"
            );
            self.sink.emit(EngineEvent::Verdict(VerdictEvent {
                property: request.property.to_string(),
                key: checked.key,
                verdict: origin::verdict_dto(&checked.verdict, scope_net),
            }));
        }
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
        // PL S-06: T5 is checked from slice 1; every other template from slice 4 (S-02).
        PropertyKind::Template { template, .. } => Err(vec![if template == "pool-share" {
            "never holds the whole pool is checked from slice 1".to_owned()
        } else {
            "user properties are checked from slice 4".to_owned()
        }]),
    }
}
