//! The element file `elements/<id>.toml` (file-format §6), strict per section (§3.3): a defect at
//! the element level makes the file not load, one inside `[petri_net]` makes the behavior
//! `Unloaded`, and one inside a `[[property]]` entry keeps the entry and reports it. What the file
//! states but is wrong as a model (a missing `when_full`, a binding to a missing port) loads, and
//! `model::validate` reports it.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::de::IgnoredAny;
use toml_edit::Document;
use tracewright_model::BehaviorModel;
use tracewright_model::Binding;
use tracewright_model::BindingPair;
use tracewright_model::Element;
use tracewright_model::ElementId;
use tracewright_model::ElementKind;
use tracewright_model::FileNotLoaded;
use tracewright_model::FinalPredicates;
use tracewright_model::Name;
use tracewright_model::ParentOfNotLoaded;
use tracewright_model::PetriNet;
use tracewright_model::Place;
use tracewright_model::PortRef;
use tracewright_model::Problem;
use tracewright_model::ProblemKind;
use tracewright_model::Relationship;
use tracewright_model::RelationshipId;
use tracewright_model::Transition;
use tracewright_model::WhenFull;

use crate::LoadError;
use crate::property;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ElementFile {
    #[expect(
        dead_code,
        reason = "read from the document before decoding (file-format §3.2)"
    )]
    format: IgnoredAny,
    kind: String,
    name: Option<String>,
    parent: Option<String>,
    technology: Option<String>,
    description: Option<String>,
    // The two behavior sections and the properties are decoded on their own, so a defect there
    // never makes the file not load (§3.3).
    petri_net: Option<IgnoredAny>,
    state_machine: Option<IgnoredAny>,
    #[serde(default)]
    relationships: BTreeMap<String, RelationshipTable>,
    #[expect(dead_code, reason = "decoded entry by entry in `property`")]
    property: Option<IgnoredAny>,
    #[serde(rename = "final")]
    final_predicates: Option<FinalTable>,
}

#[derive(Deserialize)]
struct PetriNetSection {
    petri_net: PetriNetTable,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PetriNetTable {
    #[serde(default)]
    places: BTreeMap<String, PlaceTable>,
    #[serde(default)]
    transitions: BTreeMap<String, TransitionTable>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlaceTable {
    name: Option<String>,
    capacity: Option<u32>,
    when_full: Option<String>,
    initial: Option<u32>,
    port: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TransitionTable {
    name: Option<String>,
    port: Option<bool>,
    #[serde(default)]
    pre: BTreeMap<String, u32>,
    #[serde(default)]
    post: BTreeMap<String, u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RelationshipTable {
    to: String,
    meaning: Option<String>,
    technology: Option<String>,
    interaction: Option<String>,
    timeout: Option<String>,
    #[serde(default)]
    binding: Vec<BindingTable>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingTable {
    mode: String,
    from: String,
    to: String,
    capacity: Option<u32>,
    when_full: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FinalTable {
    combined: Option<String>,
    own: Option<String>,
}

/// What reading one element file gave.
pub(crate) enum Read {
    /// The element, its outgoing relationships, and problems that belong to no entity of the
    /// model (a synchronous pair with a capacity, a property entry that could not be kept).
    Loaded {
        element: Box<Element>,
        relationships: Vec<Relationship>,
        problems: Vec<Problem>,
    },
    /// The file contributes no entities (file-format §10.5).
    NotLoaded(FileNotLoaded),
}

// An element-level defect: the file does not load.
struct FileError {
    kind: ProblemKind,
    code: Option<&'static str>,
    message: String,
    span: Option<std::ops::Range<usize>>,
}

impl FileError {
    fn schema(message: String) -> FileError {
        FileError {
            kind: ProblemKind::Schema,
            code: None,
            message,
            span: None,
        }
    }

    fn too_large(what: &str, count: usize, max: usize) -> FileError {
        FileError {
            kind: ProblemKind::TooLarge,
            code: Some("LIM-2"),
            message: format!("{what} has {count} entries; the limit is {max}"),
            span: None,
        }
    }

    fn from_toml(kind: ProblemKind, message: &str, span: Option<std::ops::Range<usize>>) -> Self {
        FileError {
            kind,
            code: None,
            message: message.trim_end().to_owned(),
            span,
        }
    }
}

/// Reads one element file. `stem` is the file name without `.toml`.
///
/// # Errors
///
/// Only a `format` newer than this app's, which stops the whole project from opening (§12.2).
pub(crate) fn read(path: &str, stem: &str, text: &str) -> Result<Read, LoadError> {
    let document = match Document::parse(text.to_owned()) {
        Ok(document) => document,
        Err(error) => {
            let error = FileError::from_toml(ProblemKind::Parse, error.message(), error.span());
            return Ok(not_loaded(
                path,
                stem,
                text,
                error,
                ParentOfNotLoaded::Unknown,
            ));
        }
    };
    let parent = lenient_parent(&document);
    match decode(path, stem, &document)? {
        Ok(read) => Ok(read),
        Err(error) => Ok(not_loaded(path, stem, text, error, parent)),
    }
}

fn not_loaded(
    path: &str,
    stem: &str,
    text: &str,
    error: FileError,
    parent: ParentOfNotLoaded,
) -> Read {
    let mut problem = Problem::error(error.kind, error.code, error.message).in_file(path);
    problem.element = ElementId::parse(stem).ok();
    if let Some(span) = error.span {
        let (line, column) = crate::line_column(text, span.start);
        problem.line = Some(line);
        problem.column = Some(column);
    }
    Read::NotLoaded(FileNotLoaded { problem, parent })
}

// file-format §10.5: the loader still reads `parent` of a file that fails at the element level.
fn lenient_parent(document: &Document<String>) -> ParentOfNotLoaded {
    match document.get("parent") {
        None => ParentOfNotLoaded::TopLevel,
        Some(item) => match item.as_str().map(ElementId::parse) {
            Some(Ok(parent)) => ParentOfNotLoaded::Element(parent),
            Some(Err(_)) | None => ParentOfNotLoaded::Unknown,
        },
    }
}

fn decode(
    path: &str,
    stem: &str,
    document: &Document<String>,
) -> Result<Result<Read, FileError>, LoadError> {
    let format = document.get("format").map(toml_edit::Item::as_integer);
    match format {
        None => {
            return Ok(Err(FileError::schema(
                "format is missing; add `format = 1`".to_owned(),
            )));
        }
        Some(None) => {
            return Ok(Err(FileError::schema(
                "format must be an integer".to_owned(),
            )));
        }
        Some(Some(found)) => {
            if let Err(message) = crate::check_version(path, found)? {
                return Ok(Err(FileError::schema(message)));
            }
        }
    }
    Ok(element(path, stem, document))
}

fn element(path: &str, stem: &str, document: &Document<String>) -> Result<Read, FileError> {
    let file: ElementFile = toml_edit::de::from_document(document.clone()).map_err(|error| {
        FileError::from_toml(ProblemKind::Schema, error.message(), error.span())
    })?;
    let id = ElementId::parse(stem)
        .map_err(|error| FileError::schema(format!("the file name: {error}")))?;
    let kind = kind(&file.kind)?;
    let parent = file
        .parent
        .as_deref()
        .map(ElementId::parse)
        .transpose()
        .map_err(|error| FileError::schema(format!("parent: {error}")))?;
    let behavior = match (file.petri_net, file.state_machine) {
        (Some(_), Some(_)) => {
            return Err(FileError::schema(
                "an element has at most one behavior model: [petri_net] or [state_machine]"
                    .to_owned(),
            ));
        }
        (Some(_), None) => Some(petri_net_section(path, &id, document)),
        (None, Some(_)) => Some(unloaded(
            path,
            &id,
            "state machines are read from slice 2".to_owned(),
            None,
            document,
        )),
        (None, None) => None,
    };
    limit(
        "relationships",
        file.relationships.len(),
        crate::RELATIONSHIPS_PER_FILE_MAX,
    )?;
    let mut problems = Vec::new();
    let mut relationships = Vec::with_capacity(file.relationships.len());
    for (key, table) in file.relationships {
        relationships.push(relationship(&id, &key, table, &mut problems)?);
    }
    let (properties, dropped) = property::read(&id, document);
    problems.extend(dropped);
    let final_predicates = file
        .final_predicates
        .map(|table| FinalPredicates {
            combined: table.combined,
            own: table.own,
        })
        .unwrap_or_default();
    let element = Element {
        name: file.name.unwrap_or_else(|| id.to_string()),
        id,
        kind,
        parent,
        technology: file.technology,
        description: file.description,
        behavior,
        properties,
        final_predicates,
    };
    Ok(Read::Loaded {
        element: Box::new(element),
        relationships,
        problems,
    })
}

fn limit(what: &str, count: usize, max: usize) -> Result<(), FileError> {
    if count > max {
        return Err(FileError::too_large(what, count, max));
    }
    Ok(())
}

fn kind(text: &str) -> Result<ElementKind, FileError> {
    Ok(match text {
        "person" => ElementKind::Person,
        "software_system" => ElementKind::SoftwareSystem,
        "container" => ElementKind::Container,
        "database" => ElementKind::Database,
        "queue" => ElementKind::Queue,
        "component" => ElementKind::Component,
        "code" => ElementKind::Code,
        other => return Err(FileError::schema(format!("unknown element kind `{other}`"))),
    })
}

fn when_full(text: &str) -> Result<WhenFull, String> {
    Ok(match text {
        "block" => WhenFull::Block,
        "drop" => WhenFull::Drop,
        "overflow" => WhenFull::Overflow,
        other => {
            return Err(format!(
                "unknown when_full `{other}` (block, drop or overflow)"
            ));
        }
    })
}

// file-format §3.4: capacities and weights are 1 ..= 2^31 - 1, initial counts 0 ..= 2^31 - 1.
fn count(what: &str, value: u32, min: u32) -> Result<u32, String> {
    if value < min || value > crate::COUNT_MAX {
        return Err(format!(
            "{what} is {value}; it must be {min} ..= {}",
            crate::COUNT_MAX
        ));
    }
    Ok(value)
}

// The behavior section that did not load (SM-E10): its problem, with the line where the decoder
// stopped.
fn unloaded(
    path: &str,
    element: &ElementId,
    message: String,
    span: Option<std::ops::Range<usize>>,
    document: &Document<String>,
) -> BehaviorModel {
    let mut problem =
        Problem::error(ProblemKind::BehaviorNotLoaded, Some("SM-E10"), message).on_element(element);
    assert_eq!(
        problem.file.as_deref(),
        Some(path),
        "an element's problems name its own file"
    );
    if let Some(span) = span {
        let (line, column) = crate::line_column(document.raw(), span.start);
        problem.line = Some(line);
        problem.column = Some(column);
    }
    BehaviorModel::Unloaded { problem }
}

fn petri_net_section(path: &str, id: &ElementId, document: &Document<String>) -> BehaviorModel {
    let section: PetriNetSection = match toml_edit::de::from_document(document.clone()) {
        Ok(section) => section,
        Err(error) => {
            return unloaded(
                path,
                id,
                error.message().trim_end().to_owned(),
                error.span(),
                document,
            );
        }
    };
    match petri_net(section.petri_net) {
        Ok(net) => BehaviorModel::PetriNet(net),
        Err(message) => unloaded(path, id, message, None, document),
    }
}

fn name(text: &str) -> Result<Name, String> {
    Name::parse(text).map_err(|error| error.to_string())
}

// Limits, value ranges and names inside `[petri_net]` (§3.4, §3.5): any failure makes the
// behavior `Unloaded`. Missing capacities and when-full choices are model problems.
fn petri_net(table: PetriNetTable) -> Result<PetriNet, String> {
    for (what, count, max) in [
        ("petri_net.places", table.places.len(), crate::PN_PLACES_MAX),
        (
            "petri_net.transitions",
            table.transitions.len(),
            crate::PN_TRANSITIONS_MAX,
        ),
    ] {
        if count > max {
            return Err(format!("{what} has {count} entries; the limit is {max}"));
        }
    }
    let mut net = PetriNet::default();
    for (key, place) in table.places {
        let what = format!("place {key}");
        let read = Place {
            name: place.name.unwrap_or_else(|| key.clone()),
            capacity: place
                .capacity
                .map(|c| count(&format!("{what} capacity"), c, 1))
                .transpose()?,
            when_full: place.when_full.as_deref().map(when_full).transpose()?,
            initial: count(&format!("{what} initial"), place.initial.unwrap_or(0), 0)?,
            port: place.port.unwrap_or(false),
        };
        net.places.insert(name(&key)?, read);
    }
    for (key, transition) in table.transitions {
        let arcs = transition.pre.len() + transition.post.len();
        if arcs > crate::PN_ARCS_PER_TRANSITION_MAX {
            return Err(format!(
                "transition {key} has {arcs} arcs; the limit is {}",
                crate::PN_ARCS_PER_TRANSITION_MAX
            ));
        }
        let arcs = |map: BTreeMap<String, u32>| -> Result<BTreeMap<Name, u32>, String> {
            map.into_iter()
                .map(|(place, weight)| {
                    Ok((
                        name(&place)?,
                        count(&format!("arc {key} {place}"), weight, 1)?,
                    ))
                })
                .collect()
        };
        let read = Transition {
            name: transition.name.unwrap_or_else(|| key.clone()),
            port: transition.port.unwrap_or(false),
            pre: arcs(transition.pre)?,
            post: arcs(transition.post)?,
        };
        net.transitions.insert(name(&key)?, read);
    }
    Ok(net)
}

fn port(text: &str) -> Result<PortRef, FileError> {
    let invalid = || FileError::schema(format!("`{text}` is not a qualified port (element/name)"));
    let (element, port_name) = text.split_once('/').ok_or_else(invalid)?;
    Ok(PortRef {
        element: ElementId::parse(element).map_err(|_id| invalid())?,
        name: Name::parse(port_name).map_err(|_name| invalid())?,
    })
}

// A relationship and its pairs. Structure (mode, qualified ports, value ranges) is element level;
// a capacity on a synchronous pair is the model problem `badBinding` (L13), kept out of the net.
fn relationship(
    source: &ElementId,
    key: &str,
    table: RelationshipTable,
    problems: &mut Vec<Problem>,
) -> Result<Relationship, FileError> {
    let id = RelationshipId::parse(key)
        .map_err(|error| FileError::schema(format!("relationships: {error}")))?;
    let schema = |message: String| FileError::schema(format!("relationships.{key}: {message}"));
    limit(
        &format!("relationships.{key}.binding"),
        table.binding.len(),
        crate::BINDINGS_PER_RELATIONSHIP_MAX,
    )?;
    let mut bindings = Vec::with_capacity(table.binding.len());
    for pair in table.binding {
        let binding = match pair.mode.as_str() {
            "sync" => {
                if pair.capacity.is_some() || pair.when_full.is_some() {
                    let mut problem = Problem::error(
                        ProblemKind::BadBinding,
                        Some("L13"),
                        format!("{key}: a synchronous pair has no capacity or when_full"),
                    )
                    .on_element(source);
                    problem.relationship = Some(id.clone());
                    problems.push(problem);
                }
                Binding::Sync
            }
            "async" => Binding::Async {
                capacity: pair
                    .capacity
                    .map(|c| count("capacity", c, 1))
                    .transpose()
                    .map_err(schema)?,
                when_full: pair
                    .when_full
                    .as_deref()
                    .map(when_full)
                    .transpose()
                    .map_err(schema)?
                    .unwrap_or(WhenFull::Overflow),
            },
            other => return Err(schema(format!("unknown mode `{other}` (sync or async)"))),
        };
        bindings.push(BindingPair {
            from: port(&pair.from)?,
            to: port(&pair.to)?,
            binding,
        });
    }
    Ok(Relationship {
        id,
        source: source.clone(),
        target: ElementId::parse(&table.to).map_err(|error| schema(format!("to: {error}")))?,
        meaning: table.meaning,
        technology: table.technology,
        interaction: table.interaction,
        timeout: table.timeout,
        bindings,
    })
}
