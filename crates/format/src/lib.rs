//! Reads and writes a project as a directory of TOML files (ADR 0001, 0020, 0024).
//!
//! `load` turns the bytes of the project files into a `Model`; it does no IO itself, so the engine
//! reads the directory through its `Disk` port and tests use files in memory.
//!
//! Strictness is per section (file-format §3.3): only `arch.toml`, the project-wide limits and a
//! newer format stop the project from opening. Any other defect becomes a problem in the Model: a
//! file that does not load, a behavior that does not load, a property entry that is not checked.
//! What this slice does not read yet (state machines, scan attributes) makes its file or section
//! not load, so a check never runs on a smaller net (§10.5).
#![forbid(unsafe_code)]

mod element;
mod layout;
mod property;
mod read;

use std::collections::BTreeMap;

use tracewright_model::ElementId;
use tracewright_model::FileNotLoaded;
use tracewright_model::Model;
use tracewright_model::ParentOfNotLoaded;
use tracewright_model::Problem;
use tracewright_model::ProblemKind;
use tracewright_model::PropertyId;

pub use layout::LAYOUT_FILE_BYTES_MAX;
pub use property::PREDICATE_BYTES_MAX;
pub use read::read_dir;
pub use read::read_file;

/// The format version this build reads and writes (file-format §3.2).
pub const FORMAT_VERSION: u32 = 1;
/// LIM-2, file-format §3.5: `arch.toml` holds a version and a budget, nothing large.
pub const ARCH_FILE_BYTES_MAX: usize = 64 * 1024;
/// LIM-2, file-format §3.5: far above the 0.3–5 KB of a typical element file.
pub const ELEMENT_FILE_BYTES_MAX: usize = 4 * 1024 * 1024;
/// LIM-2, file-format §3.5: five times the 2,000 elements per project target.
pub const ELEMENTS_MAX: usize = 10_000;
/// LIM-2, file-format §3.5.
pub const RELATIONSHIPS_PER_FILE_MAX: usize = 1_024;
/// LIM-2, file-format §3.5.
pub const BINDINGS_PER_RELATIONSHIP_MAX: usize = 64;
/// LIM-2, file-format §3.5, per Petri net.
pub const PN_PLACES_MAX: usize = 4_096;
/// LIM-2, file-format §3.5, per Petri net.
pub const PN_TRANSITIONS_MAX: usize = 4_096;
/// LIM-2, file-format §3.5: pre plus post arcs of one transition.
pub const PN_ARCS_PER_TRANSITION_MAX: usize = 1_024;
/// LIM-2, file-format §3.5, per element (PL §9.2).
pub const PROPERTIES_PER_ELEMENT_MAX: usize = 256;
/// file-format §3.4: capacities and weights stop at 2³¹ − 1.
pub const COUNT_MAX: u32 = 2_147_483_647;

/// One project file: its path relative to the project directory, with `/` separators, and its
/// bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFile {
    /// For example `elements/billing.toml`.
    pub path: String,
    /// The file content.
    pub bytes: Vec<u8>,
}

/// Why a project does not open (file-format §10.1): `arch.toml`, the project-wide limits and newer
/// formats. Everything else is a problem in the Model.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LoadError {
    /// The folder has no `arch.toml`.
    #[error("this folder is not a project: it has no arch.toml")]
    NotAProject,
    /// LIM-2: more element files than `ELEMENTS_MAX`.
    #[error("more than {max} elements ({count})")]
    TooManyElements {
        /// Element files found.
        count: usize,
        /// The limit.
        max: usize,
    },
    /// LIM-2: `arch.toml` is larger than its limit; checked before parsing.
    #[error("{path} is {size_bytes} bytes; the limit is {max_bytes}")]
    TooLarge {
        /// The file.
        path: String,
        /// Its size.
        size_bytes: usize,
        /// The limit.
        max_bytes: usize,
    },
    /// `arch.toml` is not UTF-8.
    #[error("{path} is not UTF-8")]
    NotUtf8 {
        /// The file.
        path: String,
        /// Where the bytes stop being UTF-8.
        #[source]
        source: std::str::Utf8Error,
    },
    /// `arch.toml` has a TOML syntax error, a wrong type, an unknown key or a missing key.
    #[error("{path}: {message}")]
    Schema {
        /// The file.
        path: String,
        /// What is wrong, with the line where the parser knows it.
        message: String,
    },
    /// file-format §3.2, for `arch.toml`.
    #[error("{path}: format is missing; add `format = 1`")]
    FormatMissing {
        /// The file.
        path: String,
    },
    /// file-format §12.2: any project file.
    #[error("{path}: format version {found} is newer than this app supports ({supported})")]
    VersionTooNew {
        /// The file.
        path: String,
        /// Its version.
        found: i64,
        /// This app's version.
        supported: u32,
    },
}

/// Reads a project from its files. Files outside the project set (file-format §2.1) are
/// ignored. A layout file that does not load, or names no element, leaves no layout
/// (file-format §3.3, §7).
///
/// # Errors
///
/// `arch.toml` does not load, a project-wide limit is exceeded, or a file has a newer format.
///
/// # Panics
///
/// Only on a bug: an element read under another ID than its file name.
pub fn load(files: &[ProjectFile]) -> Result<Model, LoadError> {
    let mut arch = None;
    let mut elements = BTreeMap::new();
    let mut layouts = Vec::new();
    for file in files {
        if file.path == "arch.toml" {
            arch = Some(file);
        } else if let Some(stem) = stem_in("elements/", &file.path) {
            elements.insert(stem, file);
        } else if let Some(stem) = stem_in("layouts/", &file.path) {
            layouts.push((stem, file));
        }
    }
    let arch = arch.ok_or(LoadError::NotAProject)?;
    if elements.len() > ELEMENTS_MAX {
        return Err(LoadError::TooManyElements {
            count: elements.len(),
            max: ELEMENTS_MAX,
        });
    }
    read_arch(arch)?;
    let mut model = Model::default();
    let mut relationships = Vec::new();
    for (stem, file) in elements {
        match read_element(file, stem)? {
            element::Read::Loaded {
                element,
                relationships: outgoing,
                problems,
            } => {
                assert_eq!(
                    element.id.as_str(),
                    stem,
                    "load: an element's ID is its file name"
                );
                model.elements.insert(element.id.clone(), *element);
                relationships.extend(outgoing);
                model.load_problems.extend(problems);
            }
            element::Read::NotLoaded(not_loaded) => {
                model.files_not_loaded.insert(file.path.clone(), not_loaded);
            }
        }
    }
    add_relationships(&mut model, relationships);
    unique_property_ids(&mut model);
    for (stem, file) in layouts {
        read_layout(&mut model, stem, file)?;
    }
    Ok(model)
}

fn read_element(file: &ProjectFile, stem: &str) -> Result<element::Read, LoadError> {
    // The size check comes before the TOML parser sees anything (LIM-2).
    let unreadable = |kind, code, message: String| {
        let mut problem = Problem::error(kind, code, message).in_file(&file.path);
        problem.element = ElementId::parse(stem).ok();
        element::Read::NotLoaded(FileNotLoaded {
            problem,
            parent: ParentOfNotLoaded::Unknown,
        })
    };
    if file.bytes.len() > ELEMENT_FILE_BYTES_MAX {
        return Ok(unreadable(
            ProblemKind::TooLarge,
            Some("LIM-2"),
            format!(
                "the file is {} bytes; the limit is {ELEMENT_FILE_BYTES_MAX}",
                file.bytes.len()
            ),
        ));
    }
    let Ok(text) = std::str::from_utf8(&file.bytes) else {
        return Ok(unreadable(
            ProblemKind::Parse,
            None,
            "the file is not UTF-8".to_owned(),
        ));
    };
    element::read(
        &file.path,
        stem,
        text.strip_prefix('\u{feff}').unwrap_or(text),
    )
}

// Elements and relationships share one namespace (file-format §4.2). A relationship ID used twice,
// or equal to an element ID, is `duplicateId`: none of the clashing relationships enters the
// model, and the scopes they could change are blocked (§10.3).
fn add_relationships(model: &mut Model, relationships: Vec<tracewright_model::Relationship>) {
    let mut by_id: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for relationship in relationships {
        by_id
            .entry(relationship.id.clone())
            .or_default()
            .push(relationship);
    }
    for (id, mut clashing) in by_id {
        let element_id = ElementId::parse(id.as_str()).ok();
        let names_element = element_id.is_some_and(|e| model.elements.contains_key(&e));
        if clashing.len() == 1 && !names_element {
            let relationship = clashing.pop().expect("one relationship has this ID");
            model.relationships.insert(id, relationship);
            continue;
        }
        for relationship in clashing {
            let what = if names_element {
                "is also an element ID"
            } else {
                "is used by more than one relationship"
            };
            let mut problem = Problem::error(
                ProblemKind::DuplicateId,
                Some("L3"),
                format!("the relationship ID {id} {what}"),
            )
            .on_element(&relationship.source);
            problem.relationship = Some(id.clone());
            model.load_problems.push(problem);
        }
    }
}

// PL E408: property IDs are unique in the project. The first entry in path order stays and is not
// checked; the others cannot be told apart by ID, so they stay in their files only.
fn unique_property_ids(model: &mut Model) {
    let mut seen: BTreeMap<PropertyId, usize> = BTreeMap::new();
    for element in model.elements.values() {
        for property in &element.properties {
            *seen.entry(property.id.clone()).or_default() += 1;
        }
    }
    let mut kept = std::collections::BTreeSet::new();
    for element in model.elements.values_mut() {
        let mut properties = Vec::with_capacity(element.properties.len());
        for mut property in std::mem::take(&mut element.properties) {
            let count = seen.get(&property.id).copied().unwrap_or(0);
            let message = format!("Two properties have the ID {}.", property.id);
            let mut problem = Problem::error(ProblemKind::Property, Some("E408"), message)
                .on_element(&element.id);
            problem.property = Some(property.id.clone());
            if count < 2 {
                properties.push(property);
            } else if kept.insert(property.id.clone()) {
                property.problems.push(problem);
                properties.push(property);
            } else {
                model.load_problems.push(problem);
            }
        }
        element.properties = properties;
    }
}

// file-format §3.3: a layout file that does not load leaves its element unplaced and touches
// nothing else; one whose stem names no element is an orphan (§7), kept on disk and not read.
// Only a newer format stops the project, as for every project file (§3.2).
fn read_layout(model: &mut Model, stem: &str, file: &ProjectFile) -> Result<(), LoadError> {
    let Ok(id) = ElementId::parse(stem) else {
        return Ok(());
    };
    if !model.elements.contains_key(&id) {
        return Ok(());
    }
    match layout::read(&file.path, &file.bytes)? {
        Ok(layout) => {
            model.layouts.insert(id, layout);
        }
        // The element stays unplaced and the canvas places it (§7, "missing entry"). The
        // Problems list has no layout kind yet; `schema` would mark the element file as not
        // loaded.
        Err(_not_loaded) => {}
    }
    Ok(())
}

// `<folder><stem>.toml`, not a dotfile and not in a subfolder.
fn stem_in<'p>(folder: &str, path: &'p str) -> Option<&'p str> {
    let name = path.strip_prefix(folder)?;
    if name.contains('/') || name.starts_with('.') {
        return None;
    }
    name.strip_suffix(".toml")
}

fn read_arch(file: &ProjectFile) -> Result<(), LoadError> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Arch {
        format: Option<i64>,
        // [S1] Read so that a project that sets it opens; checks use the engine's budget until
        // the budget ticket reads it.
        #[expect(
            dead_code,
            reason = "accepted and validated, used from the budget ticket on"
        )]
        budget: Option<ArchBudget>,
    }
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ArchBudget {
        #[expect(
            dead_code,
            reason = "accepted and validated, used from the budget ticket on"
        )]
        time_ms: Option<u32>,
        #[expect(
            dead_code,
            reason = "accepted and validated, used from the budget ticket on"
        )]
        max_markings: Option<u32>,
    }
    let path = &file.path;
    if file.bytes.len() > ARCH_FILE_BYTES_MAX {
        return Err(LoadError::TooLarge {
            path: path.clone(),
            size_bytes: file.bytes.len(),
            max_bytes: ARCH_FILE_BYTES_MAX,
        });
    }
    let text = std::str::from_utf8(&file.bytes).map_err(|source| LoadError::NotUtf8 {
        path: path.clone(),
        source,
    })?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let schema = |message: &str| LoadError::Schema {
        path: path.clone(),
        message: message.trim_end().to_owned(),
    };
    let arch: Arch = toml_edit::de::from_str(text).map_err(|error| schema(&error.to_string()))?;
    let Some(found) = arch.format else {
        return Err(LoadError::FormatMissing { path: path.clone() });
    };
    check_version(path, found)?.map_err(|message| schema(&message))
}

// file-format §12.2: Err for a newer format (the project does not open), Ok(Err) for a version
// that never existed (the file does not load).
pub(crate) fn check_version(path: &str, found: i64) -> Result<Result<(), String>, LoadError> {
    if found > i64::from(FORMAT_VERSION) {
        return Err(LoadError::VersionTooNew {
            path: path.to_owned(),
            found,
            supported: FORMAT_VERSION,
        });
    }
    if found == i64::from(FORMAT_VERSION) {
        Ok(Ok(()))
    } else {
        Ok(Err(format!("format {found} does not exist")))
    }
}

// 1-based line and column of byte `offset` in `text` (file-format §10.1 step 3).
pub(crate) fn line_column(text: &str, offset: usize) -> (u32, u32) {
    let before = text.get(..offset).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count())
        + 1;
    (
        u32::try_from(line).expect("ELEMENT_FILE_BYTES_MAX keeps line numbers in u32"),
        u32::try_from(column).expect("ELEMENT_FILE_BYTES_MAX keeps columns in u32"),
    )
}
