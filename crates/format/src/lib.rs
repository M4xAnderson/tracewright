//! Reads and writes a project as a directory of TOML files (ADR 0001, 0020, 0024).
//!
//! `load` turns the bytes of the project files into a `Model`; it does no IO itself, so the engine
//! reads the directory through its `Disk` port and tests use files in memory. So far the reader is
//! strict: anything it does not read yet (state machines, scan attributes, templates, layout) or
//! any schema defect makes the project not load, so a check never runs on a smaller net
//! (file-format §10.5). The per-section problems of file-format §3.3 and §10 replace that with the
//! Problems list in a later slice 0 ticket.
#![forbid(unsafe_code)]

mod element;
mod read;

use std::collections::BTreeMap;

use tracewright_model::ElementId;
use tracewright_model::Model;
use tracewright_model::id::IdError;

pub use read::read_dir;

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

/// Why a project does not load.
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
    /// LIM-2: a file is larger than its limit; checked before parsing.
    #[error("{path} is {size_bytes} bytes; the limit is {max_bytes}")]
    TooLarge {
        /// The file.
        path: String,
        /// Its size.
        size_bytes: usize,
        /// The limit.
        max_bytes: usize,
    },
    /// LIM-2: a table in a file has more entries than its limit.
    #[error("{path}: {what} has {count} entries; the limit is {max}")]
    TooMany {
        /// The file.
        path: String,
        /// What is counted.
        what: String,
        /// Entries found.
        count: usize,
        /// The limit.
        max: usize,
    },
    /// The file is not UTF-8.
    #[error("{path} is not UTF-8")]
    NotUtf8 {
        /// The file.
        path: String,
        /// Where the bytes stop being UTF-8.
        #[source]
        source: std::str::Utf8Error,
    },
    /// TOML syntax, a wrong type, an unknown key or a missing required key.
    #[error("{path}: {message}")]
    Schema {
        /// The file.
        path: String,
        /// What is wrong, with the line where the parser knows it.
        message: String,
    },
    /// file-format §3.2.
    #[error("{path}: format is missing; add `format = 1`")]
    FormatMissing {
        /// The file.
        path: String,
    },
    /// file-format §12.2.
    #[error("{path}: format version {found} is newer than this app supports ({supported})")]
    VersionTooNew {
        /// The file.
        path: String,
        /// Its version.
        found: i64,
        /// This app's version.
        supported: u32,
    },
    /// An ID or name breaks its alphabet (file-format §4.1).
    #[error("{path}: {source}")]
    InvalidId {
        /// The file.
        path: String,
        /// The ID error.
        #[source]
        source: IdError,
    },
    /// A binding port is not `element/name`.
    #[error("{path}: `{text}` is not a qualified port (element/name)")]
    InvalidPort {
        /// The file.
        path: String,
        /// The text.
        text: String,
    },
    /// Elements and relationships share one namespace; property IDs have their own.
    #[error("the ID {id} is used twice")]
    IdTaken {
        /// The ID.
        id: String,
    },
}

/// Reads a project from its files. Files outside the project set (file-format §2.1) are
/// ignored; layout files are not read yet.
///
/// # Errors
///
/// The first file that does not load, or a project-level limit.
pub fn load(files: &[ProjectFile]) -> Result<Model, LoadError> {
    let mut arch = None;
    let mut elements = BTreeMap::new();
    for file in files {
        if file.path == "arch.toml" {
            arch = Some(file);
        } else if let Some(stem) = element_stem(&file.path) {
            elements.insert(stem, file);
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
    for (stem, file) in elements {
        let id = ElementId::parse(stem).map_err(|source| LoadError::InvalidId {
            path: file.path.clone(),
            source,
        })?;
        let text = text(file, ELEMENT_FILE_BYTES_MAX)?;
        let (element, relationships) = element::read(&file.path, id, text)?;
        model.elements.insert(element.id.clone(), element);
        for relationship in relationships {
            let previous = model
                .relationships
                .insert(relationship.id.clone(), relationship);
            if let Some(previous) = previous {
                return Err(LoadError::IdTaken {
                    id: previous.id.to_string(),
                });
            }
        }
    }
    check_namespaces(&model)?;
    Ok(model)
}

// `elements/<stem>.toml`, not a dotfile.
fn element_stem(path: &str) -> Option<&str> {
    let name = path.strip_prefix("elements/")?;
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
    let text = text(file, ARCH_FILE_BYTES_MAX)?;
    let arch: Arch = parse(&file.path, text)?;
    check_version(&file.path, arch.format)
}

pub(crate) fn check_version(path: &str, format: Option<i64>) -> Result<(), LoadError> {
    match format {
        None => Err(LoadError::FormatMissing {
            path: path.to_owned(),
        }),
        Some(found) if found > i64::from(FORMAT_VERSION) => Err(LoadError::VersionTooNew {
            path: path.to_owned(),
            found,
            supported: FORMAT_VERSION,
        }),
        Some(found) if found == i64::from(FORMAT_VERSION) => Ok(()),
        Some(found) => Err(LoadError::Schema {
            path: path.to_owned(),
            message: format!("format {found} does not exist"),
        }),
    }
}

// The size check comes before the TOML parser sees anything (LIM-2).
fn text(file: &ProjectFile, max_bytes: usize) -> Result<&str, LoadError> {
    if file.bytes.len() > max_bytes {
        return Err(LoadError::TooLarge {
            path: file.path.clone(),
            size_bytes: file.bytes.len(),
            max_bytes,
        });
    }
    let text = std::str::from_utf8(&file.bytes).map_err(|source| LoadError::NotUtf8 {
        path: file.path.clone(),
        source,
    })?;
    Ok(text.strip_prefix('\u{feff}').unwrap_or(text))
}

pub(crate) fn parse<T: serde::de::DeserializeOwned>(
    path: &str,
    text: &str,
) -> Result<T, LoadError> {
    let document: toml_edit::DocumentMut = text
        .parse()
        .map_err(|error: toml_edit::TomlError| schema(path, &error.to_string()))?;
    toml_edit::de::from_document(document).map_err(|error| schema(path, &error.to_string()))
}

pub(crate) fn schema(path: &str, message: &str) -> LoadError {
    LoadError::Schema {
        path: path.to_owned(),
        message: message.trim_end().to_owned(),
    }
}

// Elements and relationships share one namespace (file-format §4.2); property IDs are unique in
// the project (PL §9.2).
fn check_namespaces(model: &Model) -> Result<(), LoadError> {
    for id in model.relationships.keys() {
        if ElementId::parse(id.as_str())
            .is_ok_and(|as_element| model.elements.contains_key(&as_element))
        {
            return Err(LoadError::IdTaken { id: id.to_string() });
        }
    }
    let mut properties = std::collections::BTreeSet::new();
    for property in model
        .elements
        .values()
        .flat_map(|element| &element.properties)
    {
        if !properties.insert(&property.id) {
            return Err(LoadError::IdTaken {
                id: property.id.to_string(),
            });
        }
    }
    Ok(())
}
