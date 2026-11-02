//! `recheck::check_certificate` on its own (NS §9.5, seam B): every certificate vector is
//! accepted on recheck's own reading of the net, and every mutated certificate is rejected.

use tracewright_net::Certificate;
use tracewright_net::NetProperty;

use super::support::q;

fn certificate(drop_places: &[&str]) -> Certificate {
    Certificate::NoOverflowPlace {
        drop_places: drop_places.iter().map(|id| q(id)).collect(),
    }
}

fn accepts(net: &str, drop_places: &[&str]) -> bool {
    let net = super::support::property_vectors().nets[net].to_net();
    tracewright_recheck::check_certificate(
        &net,
        &NetProperty::CapacityNeverExceeded,
        &certificate(drop_places),
    )
}

#[test]
fn every_certificate_vector_is_accepted() {
    let file = super::support::property_vectors();
    for vector in file.vector.iter().filter(|v| v.expect == "certificate") {
        let drops: Vec<&str> = vector
            .drop_places
            .as_ref()
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect();
        assert!(accepts(&vector.net, &drops), "{}", vector.id);
    }
}

#[test]
fn a_net_with_an_overflow_place_has_no_certificate() {
    assert!(!accepts("PC_overflow", &[]));
    assert!(!accepts("BRANCH", &[]));
}

#[test]
fn the_drop_list_must_be_exactly_the_drop_places() {
    assert!(!accepts("PC_drop", &[]), "a drop place left out");
    assert!(!accepts("PC_block", &["r"]), "a block place listed");
    assert!(!accepts("PC_drop", &["r", "r"]), "a place listed twice");
    assert!(
        !accepts("PC_drop", &["ghost", "r"]),
        "a place the net lacks"
    );
}

#[test]
fn the_certificate_is_valid_only_for_no_capacity_exceeded() {
    let net = super::support::property_vectors().nets["PC_block"].to_net();
    assert!(!tracewright_recheck::check_certificate(
        &net,
        &NetProperty::DeadlockFree,
        &certificate(&[])
    ));
}

#[test]
fn an_invalid_net_is_never_certified() {
    assert!(!accepts("V29", &[]));
}
