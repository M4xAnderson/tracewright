//! IDs and names (`docs/specs/file-format.md` §4.1, FMT-3). Element, relationship and property
//! IDs use the ID alphabet; names inside a behavior model use the Name alphabet.

use std::fmt;

/// The longest ID or name (file-format §4.1): the path budget of §2.1.
pub const ID_LENGTH_MAX: usize = 48;

/// Why a string is not an ID or a name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdError {
    /// The string breaks the alphabet of its kind.
    #[error("`{text}` is not a valid {kind}")]
    Invalid {
        /// The string.
        text: String,
        /// "ID" or "name".
        kind: &'static str,
    },
}

const RESERVED: [&str; 3] = ["and", "or", "not"];
const DEVICE_NAMES: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

// `^[a-z](?:[a-z0-9_-]{0,46}[a-z0-9_])?$`, not reserved, not a Windows device name.
fn is_id(text: &str) -> bool {
    let bytes = text.as_bytes();
    let (Some(first), Some(last)) = (bytes.first(), bytes.last()) else {
        return false;
    };
    if bytes.len() > ID_LENGTH_MAX || !first.is_ascii_lowercase() || *last == b'-' {
        return false;
    }
    let alphabet =
        |b: &u8| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_' || *b == b'-';
    bytes.iter().all(alphabet) && !RESERVED.contains(&text) && !DEVICE_NAMES.contains(&text)
}

// `^[A-Za-z_](?:[A-Za-z0-9_-]{0,46}[A-Za-z0-9_])?$`, no `__`, not reserved.
fn is_name(text: &str) -> bool {
    let bytes = text.as_bytes();
    let (Some(first), Some(last)) = (bytes.first(), bytes.last()) else {
        return false;
    };
    if bytes.len() > ID_LENGTH_MAX
        || !(first.is_ascii_alphabetic() || *first == b'_')
        || *last == b'-'
    {
        return false;
    }
    let alphabet = |b: &u8| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-';
    bytes.iter().all(alphabet) && !text.contains("__") && !RESERVED.contains(&text)
}

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident, $check:ident, $kind:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Parses `text`, which must follow the alphabet of file-format §4.1.
            ///
            /// # Errors
            ///
            /// `IdError::Invalid` when it does not.
            pub fn parse(text: &str) -> Result<$name, IdError> {
                if $check(text) {
                    Ok($name(text.to_owned()))
                } else {
                    Err(IdError::Invalid { text: text.to_owned(), kind: $kind })
                }
            }

            /// The text of the ID.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_type!(
    /// The ID of an element; also its file stem. Shares one namespace with relationship IDs.
    ElementId, is_id, "ID"
);
id_type!(
    /// The ID of a relationship; also the qualified ID of its generated place.
    RelationshipId, is_id, "ID"
);
id_type!(
    /// The ID of a property, unique in the project.
    PropertyId, is_id, "ID"
);
id_type!(
    /// A name inside a behavior model: a place or transition of a Petri net. Keeps its case.
    Name, is_name, "name"
);
