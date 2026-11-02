//! The questions the TAPAAL corpus asks verifypn about a net written by [`crate::pnml`], and how
//! their answers decide a verdict kind (NS §18 item 10, D-71). TAPAAL's answer is only ever
//! compared with ours; it never feeds a verdict (D-35).

use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::WhenFull;

use crate::pnml::pnml_id;

/// One question for verifypn. `inb` is M(p) ≤ cap(p) for every overflow place, and true on a net
/// without one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Query {
    /// A deadlock is reachable through in-bounds markings only: `E[inb U (inb ∧ deadlock)]`, or
    /// `EF deadlock` on a net without an overflow place. verifypn explores past overflow
    /// markings, which NS §3.2 makes terminal, so a plain `EF deadlock` on a net with an overflow
    /// place can find a deadlock behind one (spike E1 net X1, E1 surprise 2).
    DeadlockReachable,
    /// `AG inb`: no overflow marking is reachable.
    AlwaysInBounds,
}

/// The verdict kind TAPAAL's answers stand for. TAPAAL gives no certificate and no
/// counterexample, so only the kind is compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictKind {
    /// The property holds.
    Proved,
    /// The property does not hold.
    Disproved,
    /// Unknown "capacity exceeded first": no violation in bounds, but an overflow marking is
    /// reachable (NS §8.1).
    CapacityExceededFirst,
}

/// Decides `property` on `net` from verifypn's answers, asking `ask` one [`Query`] at a time:
///
/// - deadlock-free: a reachable deadlock is Disproved; otherwise Proved on a net without an
///   overflow place, and on one with an overflow place `AG inb` decides between Proved and
///   capacity exceeded first (NS §8.1);
/// - no capacity exceeded: `AG inb` is Proved, its negation Disproved (NS §8.2).
///
/// # Errors
///
/// The first error `ask` returns, for example when verifypn gave no answer.
pub fn decide<E>(
    net: &Net,
    property: &NetProperty,
    mut ask: impl FnMut(Query) -> Result<bool, E>,
) -> Result<VerdictKind, E> {
    let has_overflow_place = net
        .places
        .iter()
        .any(|place| place.when_full == WhenFull::Overflow);
    match property {
        NetProperty::DeadlockFree => {
            if ask(Query::DeadlockReachable)? {
                return Ok(VerdictKind::Disproved);
            }
            if !has_overflow_place {
                return Ok(VerdictKind::Proved);
            }
            if ask(Query::AlwaysInBounds)? {
                Ok(VerdictKind::Proved)
            } else {
                Ok(VerdictKind::CapacityExceededFirst)
            }
        }
        NetProperty::CapacityNeverExceeded => {
            if ask(Query::AlwaysInBounds)? {
                Ok(VerdictKind::Proved)
            } else {
                Ok(VerdictKind::Disproved)
            }
        }
    }
}

/// The MCC XML property set of one `query` on `net`, naming places by their PNML IDs
/// ([`pnml_id`]). verifypn answers it with `FORMULA <id> TRUE` or `FALSE`, where the id is
/// [`Query::id`].
#[must_use]
pub fn query_xml(net: &Net, query: Query) -> String {
    let formula = match query {
        Query::DeadlockReachable => {
            if net.places.iter().any(|p| p.when_full == WhenFull::Overflow) {
                let inb = in_bounds(net);
                format!(
                    "<exists-path><until><before>{inb}</before>\
                     <reach><conjunction>{inb}<deadlock/></conjunction></reach>\
                     </until></exists-path>"
                )
            } else {
                "<exists-path><finally><deadlock/></finally></exists-path>".to_owned()
            }
        }
        Query::AlwaysInBounds => {
            format!(
                "<all-paths><globally>{}</globally></all-paths>",
                in_bounds(net)
            )
        }
    };
    let id = query.id();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <property-set xmlns=\"http://mcc.lip6.fr/\">\n\
         \x20 <property>\n\
         \x20   <id>{id}</id>\n\
         \x20   <description>{id}</description>\n\
         \x20   <formula>{formula}</formula>\n\
         \x20 </property>\n\
         </property-set>\n"
    )
}

impl Query {
    /// The property ID in the query file and in verifypn's `FORMULA` answer line.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Query::DeadlockReachable => "deadlock-reachable",
            Query::AlwaysInBounds => "always-in-bounds",
        }
    }
}

// inb: M(p) ≤ cap(p) for every overflow place in canonical order; `<true/>` without one.
fn in_bounds(net: &Net) -> String {
    let mut overflow: Vec<_> = net
        .places
        .iter()
        .filter(|p| p.when_full == WhenFull::Overflow)
        .collect();
    overflow.sort_by(|a, b| a.id.cmp(&b.id));
    let atoms: Vec<String> = overflow
        .iter()
        .map(|p| {
            format!(
                "<integer-le><tokens-count><place>{}</place></tokens-count>\
                 <integer-constant>{}</integer-constant></integer-le>",
                pnml_id(&p.id),
                p.capacity
            )
        })
        .collect();
    match atoms.as_slice() {
        [] => "<true/>".to_owned(),
        [atom] => atom.clone(),
        _ => format!("<conjunction>{}</conjunction>", atoms.concat()),
    }
}
