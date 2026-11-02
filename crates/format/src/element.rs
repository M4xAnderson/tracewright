//! The element file `elements/<id>.toml` (file-format §6): top-level keys, `[petri_net]`,
//! `[relationships.*]` with binding pairs, `[[property]]` and `[final]`.

use std::collections::BTreeMap;

use serde::Deserialize;
use tracewright_model::Behavior;
use tracewright_model::BehaviorModel;
use tracewright_model::Binding;
use tracewright_model::BindingPair;
use tracewright_model::Element;
use tracewright_model::ElementId;
use tracewright_model::ElementKind;
use tracewright_model::FinalPredicates;
use tracewright_model::Name;
use tracewright_model::PetriNet;
use tracewright_model::Place;
use tracewright_model::PortRef;
use tracewright_model::Property;
use tracewright_model::PropertyId;
use tracewright_model::PropertyKind;
use tracewright_model::Relationship;
use tracewright_model::RelationshipId;
use tracewright_model::Transition;
use tracewright_model::WhenFull;

use crate::LoadError;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ElementFile {
    format: Option<i64>,
    kind: String,
    name: Option<String>,
    parent: Option<String>,
    technology: Option<String>,
    description: Option<String>,
    petri_net: Option<PetriNetTable>,
    #[serde(default)]
    relationships: BTreeMap<String, RelationshipTable>,
    #[serde(default)]
    property: Vec<PropertyTable>,
    #[serde(rename = "final")]
    final_predicates: Option<FinalTable>,
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
struct PropertyTable {
    id: String,
    label: Option<String>,
    behavior: String,
    kind: String,
    predicate: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FinalTable {
    combined: Option<String>,
    own: Option<String>,
}

/// Reads one element file into the element and its outgoing relationships.
pub(crate) fn read(
    path: &str,
    id: ElementId,
    text: &str,
) -> Result<(Element, Vec<Relationship>), LoadError> {
    let file: ElementFile = crate::parse(path, text)?;
    crate::check_version(path, file.format)?;
    let invalid_id = |source| LoadError::InvalidId {
        path: path.to_owned(),
        source,
    };
    let kind = kind(path, &file.kind)?;
    let parent = file
        .parent
        .as_deref()
        .map(ElementId::parse)
        .transpose()
        .map_err(invalid_id)?;
    let behavior = file
        .petri_net
        .map(|net| petri_net(path, net))
        .transpose()?
        .map(BehaviorModel::PetriNet);
    limit(
        path,
        "relationships",
        file.relationships.len(),
        crate::RELATIONSHIPS_PER_FILE_MAX,
    )?;
    let mut relationships = Vec::with_capacity(file.relationships.len());
    for (key, table) in file.relationships {
        relationships.push(relationship(path, &id, &key, table)?);
    }
    limit(
        path,
        "property",
        file.property.len(),
        crate::PROPERTIES_PER_ELEMENT_MAX,
    )?;
    let properties = file
        .property
        .into_iter()
        .map(|table| property(path, table))
        .collect::<Result<_, _>>()?;
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
    Ok((element, relationships))
}

fn limit(path: &str, what: &str, count: usize, max: usize) -> Result<(), LoadError> {
    if count > max {
        return Err(LoadError::TooMany {
            path: path.to_owned(),
            what: what.to_owned(),
            count,
            max,
        });
    }
    Ok(())
}

fn kind(path: &str, text: &str) -> Result<ElementKind, LoadError> {
    Ok(match text {
        "person" => ElementKind::Person,
        "software_system" => ElementKind::SoftwareSystem,
        "container" => ElementKind::Container,
        "database" => ElementKind::Database,
        "queue" => ElementKind::Queue,
        "component" => ElementKind::Component,
        "code" => ElementKind::Code,
        other => {
            return Err(crate::schema(
                path,
                &format!("unknown element kind `{other}`"),
            ));
        }
    })
}

fn when_full(path: &str, text: &str) -> Result<WhenFull, LoadError> {
    Ok(match text {
        "block" => WhenFull::Block,
        "drop" => WhenFull::Drop,
        "overflow" => WhenFull::Overflow,
        other => {
            return Err(crate::schema(
                path,
                &format!("unknown when_full `{other}` (block, drop or overflow)"),
            ));
        }
    })
}

// file-format §3.4: capacities and weights are 1 ..= 2^31 - 1, initial counts 0 ..= 2^31 - 1.
fn count(path: &str, what: &str, value: u32, min: u32) -> Result<u32, LoadError> {
    if value < min || value > crate::COUNT_MAX {
        return Err(crate::schema(
            path,
            &format!(
                "{what} is {value}; it must be {min} ..= {}",
                crate::COUNT_MAX
            ),
        ));
    }
    Ok(value)
}

fn name(path: &str, text: &str) -> Result<Name, LoadError> {
    Name::parse(text).map_err(|source| LoadError::InvalidId {
        path: path.to_owned(),
        source,
    })
}

fn petri_net(path: &str, table: PetriNetTable) -> Result<PetriNet, LoadError> {
    limit(
        path,
        "petri_net.places",
        table.places.len(),
        crate::PN_PLACES_MAX,
    )?;
    limit(
        path,
        "petri_net.transitions",
        table.transitions.len(),
        crate::PN_TRANSITIONS_MAX,
    )?;
    let mut net = PetriNet::default();
    for (key, place) in table.places {
        let what = format!("place {key}");
        let read = Place {
            name: place.name.unwrap_or_else(|| key.clone()),
            capacity: place
                .capacity
                .map(|c| count(path, &format!("{what} capacity"), c, 1))
                .transpose()?,
            when_full: place
                .when_full
                .as_deref()
                .map(|w| when_full(path, w))
                .transpose()?,
            initial: count(
                path,
                &format!("{what} initial"),
                place.initial.unwrap_or(0),
                0,
            )?,
            port: place.port.unwrap_or(false),
        };
        net.places.insert(name(path, &key)?, read);
    }
    for (key, transition) in table.transitions {
        limit(
            path,
            &format!("transition {key} arcs"),
            transition.pre.len() + transition.post.len(),
            crate::PN_ARCS_PER_TRANSITION_MAX,
        )?;
        let arcs = |map: BTreeMap<String, u32>| -> Result<BTreeMap<Name, u32>, LoadError> {
            map.into_iter()
                .map(|(place, weight)| {
                    Ok((
                        name(path, &place)?,
                        count(path, &format!("arc {key} {place}"), weight, 1)?,
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
        net.transitions.insert(name(path, &key)?, read);
    }
    Ok(net)
}

fn port(path: &str, text: &str) -> Result<PortRef, LoadError> {
    let invalid = || LoadError::InvalidPort {
        path: path.to_owned(),
        text: text.to_owned(),
    };
    let (element, port_name) = text.split_once('/').ok_or_else(invalid)?;
    Ok(PortRef {
        element: ElementId::parse(element).map_err(|_id| invalid())?,
        name: Name::parse(port_name).map_err(|_name| invalid())?,
    })
}

fn relationship(
    path: &str,
    source: &ElementId,
    key: &str,
    table: RelationshipTable,
) -> Result<Relationship, LoadError> {
    let invalid_id = |source| LoadError::InvalidId {
        path: path.to_owned(),
        source,
    };
    limit(
        path,
        &format!("relationships.{key}.binding"),
        table.binding.len(),
        crate::BINDINGS_PER_RELATIONSHIP_MAX,
    )?;
    let mut bindings = Vec::with_capacity(table.binding.len());
    for pair in table.binding {
        let binding = match pair.mode.as_str() {
            "sync" => {
                if pair.capacity.is_some() || pair.when_full.is_some() {
                    return Err(crate::schema(
                        path,
                        &format!(
                            "relationships.{key}: a sync binding has no capacity or when_full"
                        ),
                    ));
                }
                Binding::Sync
            }
            "async" => Binding::Async {
                capacity: pair
                    .capacity
                    .map(|c| count(path, &format!("relationships.{key} capacity"), c, 1))
                    .transpose()?,
                when_full: pair
                    .when_full
                    .as_deref()
                    .map(|w| when_full(path, w))
                    .transpose()?
                    .unwrap_or(WhenFull::Overflow),
            },
            other => {
                return Err(crate::schema(
                    path,
                    &format!("relationships.{key}: unknown mode `{other}` (sync or async)"),
                ));
            }
        };
        bindings.push(BindingPair {
            from: port(path, &pair.from)?,
            to: port(path, &pair.to)?,
            binding,
        });
    }
    Ok(Relationship {
        id: RelationshipId::parse(key).map_err(invalid_id)?,
        source: source.clone(),
        target: ElementId::parse(&table.to).map_err(invalid_id)?,
        meaning: table.meaning,
        technology: table.technology,
        interaction: table.interaction,
        timeout: table.timeout,
        bindings,
    })
}

fn property(path: &str, table: PropertyTable) -> Result<Property, LoadError> {
    let behavior = match table.behavior.as_str() {
        "own" => Behavior::Own,
        "combined" => Behavior::Combined,
        other => {
            return Err(crate::schema(
                path,
                &format!("property {}: unknown behavior `{other}`", table.id),
            ));
        }
    };
    let predicate = |kind: &str| {
        table.predicate.clone().ok_or_else(|| {
            crate::schema(
                path,
                &format!("property {}: {kind} needs a predicate", table.id),
            )
        })
    };
    let kind = match table.kind.as_str() {
        "deadlock_free" => PropertyKind::DeadlockFree,
        "no_capacity_exceeded" => PropertyKind::NoCapacityExceeded,
        "no_dead_transitions" => PropertyKind::NoDeadTransitions,
        "never" => PropertyKind::Never {
            predicate: predicate("never")?,
        },
        "always_reachable" => PropertyKind::AlwaysReachable {
            predicate: predicate("always_reachable")?,
        },
        other => {
            return Err(crate::schema(
                path,
                &format!("property {}: unknown kind `{other}`", table.id),
            ));
        }
    };
    Ok(Property {
        id: PropertyId::parse(&table.id).map_err(|source| LoadError::InvalidId {
            path: path.to_owned(),
            source,
        })?,
        label: table.label,
        behavior,
        kind,
    })
}
