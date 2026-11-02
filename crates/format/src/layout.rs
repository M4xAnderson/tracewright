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

/// Why a layout file does not load, in plain words; the element stays unplaced.
type NotLoaded = String;

/// Reads one layout file. The outer error stops the project (a newer format, file-format §3.2);
/// the inner one only leaves the element unplaced (§3.3).
pub(crate) fn read(path: &str, bytes: &[u8]) -> Result<Result<Layout, NotLoaded>, LoadError> {
    if bytes.len() > LAYOUT_FILE_BYTES_MAX {
        return Ok(Err(format!(
            "the file is {} bytes; the limit is {LAYOUT_FILE_BYTES_MAX}",
            bytes.len()
        )));
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Ok(Err("the file is not UTF-8".to_owned()));
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let document: toml_edit::DocumentMut = match text.parse() {
        Ok(document) => document,
        Err(error) => return Ok(Err(error.to_string())),
    };
    if let Err(reason) = check_counts(&document) {
        return Ok(Err(reason));
    }
    let file: LayoutFile = match toml_edit::de::from_document(document) {
        Ok(file) => file,
        Err(error) => return Ok(Err(error.to_string())),
    };
    let Some(found) = file.format else {
        return Ok(Err("format is missing; add `format = 1`".to_owned()));
    };
    if let Err(reason) = crate::check_version(path, found)? {
        return Ok(Err(reason));
    }
    Ok(decode(&file))
}

fn decode(file: &LayoutFile) -> Result<Layout, NotLoaded> {
    for (id, relationship) in &file.relationships {
        for point in &relationship.points {
            coordinates(&format!("bend point of {id}"), *point)?;
        }
    }
    if let Some(behavior) = &file.behavior {
        let items = behavior
            .places
            .iter()
            .chain(&behavior.transitions)
            .chain(&behavior.states);
        for (name, item) in items {
            item_ranges(name, item)?;
        }
    }
    let position = file
        .position
        .as_ref()
        .map(|table| -> Result<Position, NotLoaded> {
            coordinates("position", [table.x, table.y])?;
            extent("position", table.w)?;
            extent("position", table.h)?;
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
fn check_counts(document: &toml_edit::DocumentMut) -> Result<(), NotLoaded> {
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
        return Err(format!(
            "the layout has {entries} entries; the limit is {LAYOUT_ENTRIES_MAX}"
        ));
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
                return Err(format!(
                    "relationships.{id} has {points} bend points; the limit is {BEND_POINTS_MAX}"
                ));
            }
        }
    }
    Ok(())
}

fn item_ranges(name: &str, item: &ItemLayout) -> Result<(), NotLoaded> {
    if let Some(x) = item.x {
        coordinates(name, [x, 0])?;
    }
    if let Some(y) = item.y {
        coordinates(name, [0, y])?;
    }
    extent(name, item.w)?;
    extent(name, item.h)?;
    for point in &item.points {
        coordinates(name, *point)?;
    }
    Ok(())
}

fn coordinates(what: &str, [x, y]: [i32; 2]) -> Result<(), NotLoaded> {
    for value in [x, y] {
        if value.unsigned_abs() > COORDINATE_MAX.unsigned_abs() {
            return Err(format!(
                "{what}: {value} is outside -{COORDINATE_MAX} ..= {COORDINATE_MAX}"
            ));
        }
    }
    Ok(())
}

fn extent(what: &str, value: Option<u32>) -> Result<(), NotLoaded> {
    let Some(value) = value else {
        return Ok(());
    };
    if value == 0 || value > EXTENT_MAX {
        return Err(format!(
            "{what}: a size of {value} is outside 1 ..= {EXTENT_MAX}"
        ));
    }
    Ok(())
}
