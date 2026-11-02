//! `recheck::compose` against the NS §14.3 composition vectors: fusions, asynchronous bindings
//! with their generated places, and the rejections of L6, L7, L8 and L12.

use tracewright_net::encode;
use tracewright_recheck::ComposeError;

#[test]
fn every_valid_vector_composes_to_its_exact_encoding() {
    for vector in super::support::composition_vectors().composition {
        let Some(expected) = &vector.encoding else {
            continue;
        };
        let net = tracewright_recheck::compose(&vector.open_nets())
            .unwrap_or_else(|error| panic!("{}: {error}", vector.id));
        assert_eq!(
            String::from_utf8(encode::canonical(&net)).unwrap(),
            *expected,
            "{}",
            vector.id
        );
    }
}

#[test]
fn every_rejected_vector_is_a_compose_error() {
    for vector in super::support::composition_vectors().composition {
        let Some(why) = &vector.rejected else {
            continue;
        };
        assert!(
            tracewright_recheck::compose(&vector.open_nets()).is_err(),
            "{} ({why})",
            vector.id
        );
    }
}

#[test]
fn the_order_of_parts_fusions_and_links_does_not_change_the_result() {
    for vector in super::support::composition_vectors().composition {
        if vector.encoding.is_none() {
            continue;
        }
        let mut open = vector.open_nets();
        let forward = tracewright_recheck::compose(&open).unwrap();
        open.parts.reverse();
        open.fusions.reverse();
        open.links.reverse();
        let backward = tracewright_recheck::compose(&open).unwrap();
        assert_eq!(
            encode::canonical(&forward),
            encode::canonical(&backward),
            "{}",
            vector.id
        );
    }
}

#[test]
fn rejections_name_the_rule() {
    let vectors = super::support::composition_vectors().composition;
    let error = |id: &str| {
        let vector = vectors.iter().find(|v| v.id == id).unwrap();
        tracewright_recheck::compose(&vector.open_nets()).unwrap_err()
    };
    assert_eq!(
        error("C7"),
        ComposeError::DuplicateFusedId {
            id: "r/call+serve".to_owned()
        }
    );
    assert!(matches!(error("C9"), ComposeError::SameElement { .. }));
    assert_eq!(
        error("C10"),
        ComposeError::CallerAndCallee {
            transition: "b/serve".to_owned()
        }
    );
    assert_eq!(
        error("C22"),
        ComposeError::DuplicateFusedId {
            id: "r/x+y".to_owned()
        }
    );
}

#[test]
fn rejections_of_asynchronous_bindings_name_the_rule() {
    let vectors = super::support::composition_vectors().composition;
    let error = |id: &str| {
        let vector = vectors.iter().find(|v| v.id == id).unwrap();
        tracewright_recheck::compose(&vector.open_nets()).unwrap_err()
    };
    assert_eq!(
        error("C8"),
        ComposeError::FusedAndLinked {
            transition: "a/call".to_owned()
        }
    );
    assert_eq!(
        error("C11"),
        ComposeError::DuplicateGeneratedPlace {
            relationship: "r".to_owned()
        }
    );
    assert_eq!(
        error("C19"),
        ComposeError::EmptyLinkSet {
            relationship: "jobs".to_owned(),
            side: "to"
        }
    );
    assert_eq!(
        error("C20"),
        ComposeError::InTwoLinks {
            transition: "consumer/run".to_owned()
        }
    );
    for id in ["unsorted-set", "duplicate-member"] {
        assert_eq!(
            error(id),
            ComposeError::UnsortedLinkSet {
                relationship: "jobs".to_owned()
            },
            "{id}"
        );
    }
    assert_eq!(
        error("unknown-member"),
        ComposeError::UnknownTransition {
            relationship: "jobs".to_owned(),
            transition: "consumer/ready".to_owned()
        }
    );
    assert!(matches!(
        error("unknown-port"),
        ComposeError::UnknownTransition { .. }
    ));
    assert!(matches!(
        error("invalid-part"),
        ComposeError::InvalidPart { part: 0, .. }
    ));
}
