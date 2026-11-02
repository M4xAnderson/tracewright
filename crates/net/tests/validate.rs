//! Structural validation (NS §2.3), with the L10 lines of V29 and V54.

use std::collections::BTreeMap;

use tracewright_net::Marking;
use tracewright_net::Net;
use tracewright_net::Place;
use tracewright_net::QualifiedId;
use tracewright_net::Transition;
use tracewright_net::WhenFull;
use tracewright_net::validate;

fn q(id: &str) -> QualifiedId {
    QualifiedId(id.to_owned())
}

type Arcs<'a> = &'a [(&'a str, u32)];

fn net(places: &[(&str, u32, u32)], transitions: &[(&str, Arcs, Arcs)]) -> Net {
    let arcs = |list: &[(&str, u32)]| {
        list.iter()
            .map(|(id, w)| (q(id), *w))
            .collect::<BTreeMap<_, _>>()
    };
    Net {
        places: places
            .iter()
            .map(|(id, capacity, _)| Place {
                id: q(id),
                capacity: *capacity,
                when_full: WhenFull::Block,
            })
            .collect(),
        transitions: transitions
            .iter()
            .map(|(id, pre, post)| Transition {
                id: q(id),
                pre: arcs(pre),
                post: arcs(post),
            })
            .collect(),
        initial: Marking(
            places
                .iter()
                .filter(|(_, _, initial)| *initial > 0)
                .map(|(id, _, initial)| (q(id), *initial))
                .collect(),
        ),
    }
}

fn lines(net: &Net) -> Vec<String> {
    validate::net(net)
        .expect_err("the net is invalid")
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn a_valid_net_passes() {
    assert_eq!(
        validate::net(&net(
            &[("x/p", 1, 1)],
            &[("x/t", &[("x/p", 1)], &[("x/p", 1)])]
        )),
        Ok(())
    );
}

#[test]
fn v29_initial_above_capacity() {
    assert_eq!(
        lines(&net(&[("x/p", 3, 5)], &[])),
        ["x/p: initial 5 exceeds capacity 3"]
    );
}

#[test]
fn v54_capacity_plus_largest_post_weight_exceeds_u32() {
    let net = net(&[("x/p", u32::MAX - 1, 0)], &[("x/t", &[], &[("x/p", 2)])]);
    assert_eq!(
        lines(&net),
        ["x/p: capacity 4294967294 plus largest post weight 2 exceeds u32"]
    );
}

#[test]
fn v52_capacity_plus_weight_at_exactly_u32_max_is_valid() {
    let net = net(
        &[("x/p", u32::MAX - 1, u32::MAX - 1)],
        &[("x/t", &[], &[("x/p", 1)])],
    );
    assert_eq!(validate::net(&net), Ok(()));
}

#[test]
fn places_and_transitions_share_one_namespace() {
    assert_eq!(
        lines(&net(&[("x/p", 1, 0)], &[("x/p", &[], &[])])),
        ["x/p names more than one place or transition"]
    );
}

#[test]
fn ids_have_one_of_the_three_shapes() {
    for id in ["x/p", "r/call+serve", "rel-7", "_a/B-2"] {
        assert!(validate::is_qualified_id(id), "{id}");
    }
    for id in [
        "", "x/", "/p", "x/p/q", "r/a+b+c", "x/1p", "and", "x/not", "a b", "r/+b",
    ] {
        assert!(!validate::is_qualified_id(id), "{id}");
    }
}

#[test]
fn arcs_must_name_places_of_the_net_with_a_weight_of_at_least_one() {
    let net = net(&[("x/p", 1, 0)], &[("x/t", &[("x/q", 1)], &[("x/p", 0)])]);
    assert_eq!(
        lines(&net),
        [
            "x/t has an arc to x/q, which is not a place of this net",
            "x/t has an arc of weight 0 to x/p"
        ]
    );
}

#[test]
fn capacity_zero_is_invalid() {
    assert_eq!(lines(&net(&[("x/p", 0, 0)], &[])), ["x/p: capacity 0"]);
}
