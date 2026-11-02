//! The certificate check of NS §9.5, on recheck's own composition: polynomial in the net, no
//! exploration.

use tracewright_net::Certificate;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::QualifiedId;
use tracewright_net::WhenFull;
use tracewright_net::validate;

/// Whether `certificate` proves `property` on `net` (normally recheck's own composition).
///
/// `NoOverflowPlace { drop_places }` is accepted iff the property is no capacity exceeded, no
/// place of the net overflows when full and `drop_places` is exactly the sorted list of the
/// places that drop. Then no overflow marking exists (NS §3.2), so the property holds.
#[must_use]
pub fn check_certificate(net: &Net, property: &NetProperty, certificate: &Certificate) -> bool {
    if validate::net(net).is_err() {
        return false;
    }
    let Certificate::NoOverflowPlace { drop_places } = certificate;
    match property {
        NetProperty::CapacityNeverExceeded => {}
        NetProperty::DeadlockFree => return false,
    }
    if net
        .places
        .iter()
        .any(|place| place.when_full == WhenFull::Overflow)
    {
        return false;
    }
    let mut drops: Vec<&QualifiedId> = net
        .places
        .iter()
        .filter(|place| place.when_full == WhenFull::Drop)
        .map(|place| &place.id)
        .collect();
    drops.sort();
    drops.len() == drop_places.len() && drops.iter().zip(drop_places).all(|(a, b)| *a == b)
}
