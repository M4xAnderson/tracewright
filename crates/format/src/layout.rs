//! The layout file `layouts/<id>.toml` (file-format §7, ADR 0024): one element's position, the
//! bend points of its outgoing relationships and its behavior layout.
//!
//! Layout never changes meaning, so a layout file that does not load only leaves its element
//! unplaced (file-format §3.3); it never stops the project from opening.

use std::collections::BTreeMap;

use serde::Deserialize;
use tracewright_model::Layout;
use tracewright_model::Position;

use crate::LoadError;

/// LIM-2, file-format §3.5.
pub const LAYOUT_FILE_BYTES_MAX: usize = 4 * 1024 * 1024;
/// LIM-2, file-format §3.5: the position, relationship entries and behavior items of one file.
pub const LAYOUT_ENTRIES_MAX: usize = 20_000;
/// LIM-2, file-format §3.5: bend points of one relationship.
pub const BEND_POINTS_MAX: usize = 64;
/// file-format §3.4: `x`, `y` and bend points stay within ±1,000,000 canvas units.
pub const COORDINATE_MAX: i32 = 1_000_000;
/// file-format §3.4: `w` and `h` are 1 ..= 1,000,000.
pub const EXTENT_MAX: u32 = 1_000_000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LayoutFile {
    format: Option<i64>,
    position: Option<PositionTable>,
    // Bend points and behavior layout are validated so that a malformed file is treated as one
    // (file-format §3.3); the canvas draws them from the editor tickets on.
    #[serde(default)]
    relationships: BTreeMap<String, RelationshipLayout>,
    behavior: Option<BehaviorLayout>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PositionTable {
    x: i32,
    y: i32,
    w: Option<u32>,
    h: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RelationshipLayout {
    #[serde(default)]
    points: Vec<[i32; 2]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BehaviorLayout {
    #[serde(default)]
    places: BTreeMap<String, ItemLayout>,
    #[serde(default)]
    transitions: BTreeMap<String, ItemLayout>,
    #[serde(default)]
    states: BTreeMap<String, ItemLayout>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemLayout {
    x: Option<i32>,
    y: Option<i32>,
    w: Option<u32>,
    h: Option<u32>,
    #[serde(default)]
    points: Vec<[i32; 2]>,
}

/// Reads one layout file.
pub(crate) fn read(path: &str, text: &str) -> Result<Layout, LoadError> {
    let document: toml_edit::DocumentMut = text
        .parse()
        .map_err(|error: toml_edit::TomlError| crate::schema(path, &error.to_string()))?;
    check_counts(path, &document)?;
    let file: LayoutFile = toml_edit::de::from_document(document)
        .map_err(|error| crate::schema(path, &error.to_string()))?;
    crate::check_version(path, file.format)?;
    for (id, relationship) in &file.relationships {
        for point in &relationship.points {
            coordinates(path, &format!("bend point of {id}"), *point)?;
        }
    }
    if let Some(behavior) = &file.behavior {
        let items = behavior
            .places
            .iter()
            .chain(&behavior.transitions)
            .chain(&behavior.states);
        for (name, item) in items {
            item_ranges(path, name, item)?;
        }
    }
    let position = file
        .position
        .map(|table| {
            coordinates(path, "position", [table.x, table.y])?;
            extent(path, "position", table.w)?;
            extent(path, "position", table.h)?;
            Ok(Position {
                x: table.x,
                y: table.y,
                w: table.w,
                h: table.h,
            })
        })
        .transpose()?;
    Ok(Layout { position })
}

// LIM-2: the counts are taken on the parsed document, before anything is decoded.
fn check_counts(path: &str, document: &toml_edit::DocumentMut) -> Result<(), LoadError> {
    let table_len = |key: &str| {
        document
            .get(key)
            .and_then(toml_edit::Item::as_table_like)
            .map_or(0, toml_edit::TableLike::len)
    };
    let mut entries = usize::from(document.contains_key("position")) + table_len("relationships");
    if let Some(behavior) = document
        .get("behavior")
        .and_then(toml_edit::Item::as_table_like)
    {
        for (_, items) in behavior.iter() {
            entries += items.as_table_like().map_or(0, toml_edit::TableLike::len);
        }
    }
    if entries > LAYOUT_ENTRIES_MAX {
        return Err(too_many(path, "the layout", entries, LAYOUT_ENTRIES_MAX));
    }
    if let Some(relationships) = document
        .get("relationships")
        .and_then(toml_edit::Item::as_table_like)
    {
        for (id, relationship) in relationships.iter() {
            let points = relationship
                .get("points")
                .and_then(toml_edit::Item::as_array)
                .map_or(0, toml_edit::Array::len);
            if points > BEND_POINTS_MAX {
                let what = format!("relationships.{id}.points");
                return Err(too_many(path, &what, points, BEND_POINTS_MAX));
            }
        }
    }
    Ok(())
}

fn too_many(path: &str, what: &str, count: usize, max: usize) -> LoadError {
    LoadError::TooMany {
        path: path.to_owned(),
        what: what.to_owned(),
        count,
        max,
    }
}

fn item_ranges(path: &str, name: &str, item: &ItemLayout) -> Result<(), LoadError> {
    if let Some(x) = item.x {
        coordinates(path, name, [x, 0])?;
    }
    if let Some(y) = item.y {
        coordinates(path, name, [0, y])?;
    }
    extent(path, name, item.w)?;
    extent(path, name, item.h)?;
    for point in &item.points {
        coordinates(path, name, *point)?;
    }
    Ok(())
}

fn coordinates(path: &str, what: &str, [x, y]: [i32; 2]) -> Result<(), LoadError> {
    for value in [x, y] {
        if value.unsigned_abs() > COORDINATE_MAX.unsigned_abs() {
            return Err(crate::schema(
                path,
                &format!("{what}: {value} is outside -{COORDINATE_MAX} ..= {COORDINATE_MAX}"),
            ));
        }
    }
    Ok(())
}

fn extent(path: &str, what: &str, value: Option<u32>) -> Result<(), LoadError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value == 0 || value > EXTENT_MAX {
        return Err(crate::schema(
            path,
            &format!("{what}: a size of {value} is outside 1 ..= {EXTENT_MAX}"),
        ));
    }
    Ok(())
}
