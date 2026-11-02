//! The `CheckKey` (NS §11, ADR 0013): the identity of a check for caching and for deciding whether a
//! verdict is current.

use std::fmt;

use serde::Serialize;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::encode;
use ts_rs::TS;

/// Serialized as its hex string (a newtype struct). BLAKE3-256 of `canonical(flat) ++ canonical_property(p)`, as 64 lowercase hex characters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, TS)]
#[ts(export, export_to = "check-key.ts")]
pub struct CheckKey(String);

impl CheckKey {
    /// The first 8 characters, for thread names (NS §11).
    #[must_use]
    pub fn key8(&self) -> &str {
        &self.0[..8]
    }
}

impl fmt::Display for CheckKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The key of checking `property` on the flattened net `flat`. It covers IDs, capacities, when
/// full, arcs with weights, the initial marking and the property; never layout, display names,
/// assumptions or the budget.
///
/// # Panics
///
/// Only on a bug in the encoding: a net encoding without its final LF.
#[must_use]
pub fn check_key(flat: &Net, property: &NetProperty) -> CheckKey {
    let mut input = encode::canonical(flat);
    assert_eq!(
        input.last(),
        Some(&b'\n'),
        "check_key: the net encoding ends in LF (NS §14.4 f)"
    );
    input.extend(encode::canonical_property(property));
    let hex = blake3::hash(&input).to_hex().to_string();
    assert_eq!(hex.len(), 64, "check_key: BLAKE3-256 is 64 hex characters");
    CheckKey(hex)
}
