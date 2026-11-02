//! `[[property]]` entries (file-format §6.8, PL §9.2). An entry is one question: a defect in it
//! is reported with its PL code and only that property is not checked (§3.3). An entry without a
//! usable ID, behavior or kind cannot be placed in the verdict panel, so it stays in the file and
//! appears only in the Problems list.

use toml_edit::Document;
use toml_edit::Item;
use toml_edit::TableLike;
use tracewright_model::Behavior;
use tracewright_model::ElementId;
use tracewright_model::Problem;
use tracewright_model::ProblemKind;
use tracewright_model::Property;
use tracewright_model::PropertyId;
use tracewright_model::PropertyKind;

/// PL E114: the longest predicate, checked in every slice before any parsing.
pub const PREDICATE_BYTES_MAX: usize = 4_096;

const KNOWN_KEYS: [&str; 16] = [
    "id",
    "label",
    "behavior",
    "kind",
    "predicate",
    "template",
    "template_version",
    // The template parameters of PL §8.2.
    "place",
    "terms",
    "at_most",
    "pool",
    "held_by",
    "holder",
    "k",
    "target",
    "preset",
];

// PL §8.2: the catalog templates, all at version 1. T6 is held back (D-11), so it is unknown.
const TEMPLATES: [&str; 5] = [
    "never-overflows",
    "never-more-than",
    "never-fills-up",
    "pool-share",
    "get-back-to",
];

/// The properties of `element` that could be kept, and the problems of entries that could not.
pub(crate) fn read(
    element: &ElementId,
    document: &Document<String>,
) -> (Vec<Property>, Vec<Problem>) {
    let mut properties = Vec::new();
    let mut dropped = Vec::new();
    let entries: Vec<&dyn TableLike> = match document.get("property") {
        None => Vec::new(),
        Some(Item::ArrayOfTables(array)) => array.iter().map(|t| t as &dyn TableLike).collect(),
        Some(item) => {
            // `property = [{ … }]` is the same data written as an array of inline tables.
            let tables: Option<Vec<&dyn TableLike>> = item.as_array().and_then(|array| {
                array
                    .iter()
                    .map(|value| value.as_inline_table().map(|t| t as &dyn TableLike))
                    .collect()
            });
            tables.unwrap_or_else(|| {
                dropped.push(problem(
                    element,
                    None,
                    None,
                    "property must be an array of tables",
                ));
                Vec::new()
            })
        }
    };
    let total = entries.len();
    let too_many = total > crate::PROPERTIES_PER_ELEMENT_MAX;
    for entry in entries {
        match property(element, entry) {
            Ok(mut property) => {
                if too_many {
                    property.problems.push(problem(
                        element,
                        Some(&property.id),
                        Some("E409"),
                        &format!(
                            "{element} has more than {} properties.",
                            crate::PROPERTIES_PER_ELEMENT_MAX
                        ),
                    ));
                }
                properties.push(property);
            }
            Err(problem) => dropped.push(*problem),
        }
    }
    assert!(
        properties.len() <= total,
        "property::read: at most one property per entry"
    );
    (properties, dropped)
}

fn problem(
    element: &ElementId,
    property: Option<&PropertyId>,
    code: Option<&'static str>,
    message: &str,
) -> Problem {
    let mut problem =
        Problem::error(ProblemKind::Property, code, message.to_owned()).on_element(element);
    problem.property = property.cloned();
    problem
}

fn text<'t>(entry: &'t dyn TableLike, key: &str) -> Option<Result<&'t str, ()>> {
    entry.get(key).map(|item| item.as_str().ok_or(()))
}

// One entry: Ok with the problems that only stop its check, Err when it cannot be kept.
fn property(element: &ElementId, entry: &dyn TableLike) -> Result<Property, Box<Problem>> {
    let unkeyed = |message: &str| Box::new(problem(element, None, None, message));
    let id = match text(entry, "id") {
        Some(Ok(text)) => {
            PropertyId::parse(text).map_err(|error| unkeyed(&format!("property: {error}")))?
        }
        Some(Err(())) => return Err(unkeyed("property id must be a string")),
        None => return Err(unkeyed("a property entry needs an id")),
    };
    let fail = |code, message: &str| problem(element, Some(&id), Some(code), message);
    let behavior = match text(entry, "behavior") {
        Some(Ok("own")) => Behavior::Own,
        Some(Ok("combined")) => Behavior::Combined,
        Some(_) | None => {
            return Err(Box::new(fail(
                "E403",
                &format!(
                    "Say whether this property is about {element}'s own behavior (behavior = \"own\") or its children (behavior = \"combined\")."
                ),
            )));
        }
    };
    let mut problems = Vec::new();
    for (key, _) in entry.iter() {
        if key == "final" {
            problems.push(fail(
                "E407",
                "A property cannot set its own done when. Use the element's done when.",
            ));
        } else if !KNOWN_KEYS.contains(&key) {
            problems.push(fail(
                "E407",
                &format!(
                    "Unknown key {key} in property {id}. It is kept in the file but the property is not checked."
                ),
            ));
        }
    }
    let mut found = Vec::new();
    let kind = kind(entry, &mut found).map_err(|(code, message)| Box::new(fail(code, &message)))?;
    problems.extend(
        found
            .into_iter()
            .map(|(code, message)| fail(code, &message)),
    );
    let label = match text(entry, "label") {
        Some(Ok(label)) => Some(label.to_owned()),
        Some(Err(())) => {
            problems.push(problem(element, Some(&id), None, "label must be a string"));
            None
        }
        None => None,
    };
    Ok(Property {
        id,
        label,
        behavior,
        kind,
        problems,
    })
}

type Found = (&'static str, String);

// E401, E402, E411 and E114 go to `found`; Err (E406, E410) when the entry has no usable kind.
fn kind(entry: &dyn TableLike, found: &mut Vec<Found>) -> Result<PropertyKind, Found> {
    let predicate = match text(entry, "predicate") {
        Some(Ok(predicate)) => {
            if predicate.len() > PREDICATE_BYTES_MAX {
                found.push((
                    "E114",
                    format!("The predicate is longer than {PREDICATE_BYTES_MAX} bytes."),
                ));
            }
            Some(predicate.to_owned())
        }
        Some(Err(())) => {
            found.push(("E401", "The predicate must be a string.".to_owned()));
            None
        }
        None => None,
    };
    let (kind, template) = (text(entry, "kind"), text(entry, "template"));
    match (kind, template) {
        (Some(_), Some(_)) => Err((
            "E410",
            "A property has either kind or template, not both.".to_owned(),
        )),
        (None, None) => Err(("E410", "A property needs a kind or a template.".to_owned())),
        (Some(kind), None) => {
            let kind = kind.map_err(|()| ("E406", "kind must be a string".to_owned()))?;
            built_in(kind, predicate, found)
        }
        (None, Some(template)) => {
            let template =
                template.map_err(|()| ("E411", "template must be a string".to_owned()))?;
            let version = entry.get("template_version").and_then(Item::as_integer);
            Ok(template_kind(template, version, predicate.is_some(), found))
        }
    }
}

fn built_in(
    kind: &str,
    predicate: Option<String>,
    found: &mut Vec<Found>,
) -> Result<PropertyKind, Found> {
    let needs_predicate = matches!(kind, "never" | "always_reachable");
    if needs_predicate && predicate.is_none() {
        found.push(("E401", format!("{kind} needs a predicate.")));
    }
    if !needs_predicate && predicate.is_some() {
        found.push((
            "E402",
            format!("{kind} takes no predicate. Remove the predicate line."),
        ));
    }
    Ok(match kind {
        "deadlock_free" => PropertyKind::DeadlockFree,
        "no_capacity_exceeded" => PropertyKind::NoCapacityExceeded,
        "no_dead_transitions" => PropertyKind::NoDeadTransitions,
        "never" => PropertyKind::Never {
            predicate: predicate.unwrap_or_default(),
        },
        "always_reachable" => PropertyKind::AlwaysReachable {
            predicate: predicate.unwrap_or_default(),
        },
        other => return Err(("E406", unknown_kind(other))),
    })
}

// Slice 0 keeps a template by ID and version; its parameters stay in the file, unparsed (D-49).
fn template_kind(
    template: &str,
    version: Option<i64>,
    has_predicate: bool,
    found: &mut Vec<Found>,
) -> PropertyKind {
    let known = TEMPLATES.contains(&template) && version == Some(1);
    if !known {
        let version = version.map_or_else(|| "(none)".to_owned(), |v| v.to_string());
        found.push((
            "E411",
            format!("Unknown template {template} version {version}."),
        ));
    }
    if has_predicate {
        found.push((
            "E402",
            format!("{template} takes no predicate. Remove the predicate line."),
        ));
    }
    PropertyKind::Template {
        template: template.to_owned(),
        version,
    }
}

fn unknown_kind(kind: &str) -> String {
    format!(
        "Unknown kind {kind}. Use deadlock_free, no_capacity_exceeded, no_dead_transitions, never or always_reachable."
    )
}
