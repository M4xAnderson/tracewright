//! The canonical encoding (NS §11) against the C1 vector and the encoding vectors of NS §14.4.

use std::collections::BTreeMap;

use tracewright_net::Marking;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::Place;
use tracewright_net::QualifiedId;
use tracewright_net::Transition;
use tracewright_net::WhenFull;
use tracewright_net::encode;

fn q(id: &str) -> QualifiedId {
    QualifiedId(id.to_owned())
}

fn place(id: &str, capacity: u32) -> Place {
    Place {
        id: q(id),
        capacity,
        when_full: WhenFull::Block,
    }
}

fn arcs(list: &[(&str, u32)]) -> BTreeMap<QualifiedId, u32> {
    list.iter().map(|(id, weight)| (q(id), *weight)).collect()
}

fn transition(id: &str, pre: &[(&str, u32)], post: &[(&str, u32)]) -> Transition {
    Transition {
        id: q(id),
        pre: arcs(pre),
        post: arcs(post),
    }
}

// The flattened net of NS §14.3 C1.
fn c1() -> Net {
    Net {
        places: vec![place("b/free", 1), place("a/idle", 1), place("a/busy", 1)],
        transitions: vec![
            transition(
                "r/call+serve",
                &[("b/free", 1), ("a/idle", 1)],
                &[("b/free", 1), ("a/busy", 1)],
            ),
            transition("a/done", &[("a/busy", 1)], &[("a/idle", 1)]),
        ],
        initial: Marking(arcs(&[("a/idle", 1), ("b/free", 1)])),
    }
}

#[test]
fn c1_encodes_to_the_vector_text() {
    let expected = "petrinet-net 1\n\
                    place a/busy 1 block 0\n\
                    place a/idle 1 block 1\n\
                    place b/free 1 block 1\n\
                    transition a/done\n\
                    pre a/busy 1\n\
                    post a/idle 1\n\
                    transition r/call+serve\n\
                    pre a/idle 1\n\
                    pre b/free 1\n\
                    post a/busy 1\n\
                    post b/free 1\n";
    assert_eq!(
        String::from_utf8(encode::canonical(&c1())).unwrap(),
        expected
    );
}

#[test]
fn insertion_order_does_not_change_the_encoding() {
    let mut reversed = c1();
    reversed.places.reverse();
    reversed.transitions.reverse();
    assert_eq!(encode::canonical(&reversed), encode::canonical(&c1()));
}

#[test]
fn a_zero_initial_count_and_a_missing_one_encode_the_same() {
    let mut explicit = c1();
    explicit.initial.0.insert(q("a/busy"), 0);
    assert_eq!(encode::canonical(&explicit), encode::canonical(&c1()));
}

#[test]
fn a_transition_without_arcs_is_one_line() {
    let net = Net {
        places: vec![],
        transitions: vec![transition("x/t", &[], &[])],
        initial: Marking::default(),
    };
    assert_eq!(encode::canonical(&net), b"petrinet-net 1\ntransition x/t\n");
}

#[test]
fn deadlock_free_is_encoded_with_final_none() {
    assert_eq!(
        encode::canonical_property(&NetProperty::DeadlockFree),
        b"property deadlock-free final none\n"
    );
}

#[test]
fn the_net_encoding_ends_in_lf_so_the_property_line_stands_alone() {
    // NS §14.4 (f)
    assert_eq!(encode::canonical(&c1()).last(), Some(&b'\n'));
}
