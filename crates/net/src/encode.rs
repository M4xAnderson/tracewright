//! The canonical encoding of a net and a property (NS §11): the input of the `CheckKey` and the
//! bytes `engine::flatten` and `recheck::compose` must agree on (NS §6.5). UTF-8, one item per
//! line, every line ending in LF, every list sorted by ID.

use crate::Net;
use crate::NetProperty;
use crate::WhenFull;

/// The header line; bumping it invalidates every cached verdict, which is the intended effect of
/// a semantics change (NS §11).
const HEADER: &str = "petrinet-net 1\n";

/// The canonical text of `net`. Independent of the order of places, transitions and arcs.
#[must_use]
pub fn canonical(net: &Net) -> Vec<u8> {
    let mut out = vec![HEADER.to_owned()];
    let mut places: Vec<_> = net.places.iter().collect();
    places.sort_by(|a, b| a.id.cmp(&b.id));
    for place in places {
        let initial = net.initial.0.get(&place.id).copied().unwrap_or(0);
        let when_full = match place.when_full {
            WhenFull::Block => "block",
        };
        out.push(format!(
            "place {} {} {when_full} {initial}\n",
            place.id.0, place.capacity
        ));
    }
    let mut transitions: Vec<_> = net.transitions.iter().collect();
    transitions.sort_by(|a, b| a.id.cmp(&b.id));
    for transition in transitions {
        out.push(format!("transition {}\n", transition.id.0));
        // BTreeMap iterates sorted by place ID.
        for (place, weight) in &transition.pre {
            out.push(format!("pre {} {weight}\n", place.0));
        }
        for (place, weight) in &transition.post {
            out.push(format!("post {} {weight}\n", place.0));
        }
    }
    out.concat().into_bytes()
}

/// The canonical property line, LF-terminated.
#[must_use]
pub fn canonical_property(property: &NetProperty) -> Vec<u8> {
    let line = match property {
        NetProperty::DeadlockFree => "property deadlock-free final none\n",
    };
    line.as_bytes().to_vec()
}
