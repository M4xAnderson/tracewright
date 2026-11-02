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

// NS §14.3 C4: PC(overflow), the asynchronous binding lowered with its default when_full.
fn pc(when_full: WhenFull) -> Net {
    Net {
        places: vec![
            place("p/items", 5),
            place("c/got", 5),
            Place {
                id: q("r"),
                capacity: 3,
                when_full,
            },
        ],
        transitions: vec![
            transition("p/produce", &[("p/items", 1)], &[("r", 1)]),
            transition("c/consume", &[("r", 1)], &[("c/got", 1)]),
        ],
        initial: Marking(arcs(&[("p/items", 5)])),
    }
}

#[test]
fn c4_a_generated_overflow_place_encodes_to_the_vector_text() {
    let expected = "petrinet-net 1\n\
                    place c/got 5 block 0\n\
                    place p/items 5 block 5\n\
                    place r 3 overflow 0\n\
                    transition c/consume\n\
                    pre r 1\n\
                    post c/got 1\n\
                    transition p/produce\n\
                    pre p/items 1\n\
                    post r 1\n";
    assert_eq!(
        String::from_utf8(encode::canonical(&pc(WhenFull::Overflow))).unwrap(),
        expected
    );
}

#[test]
fn when_full_is_part_of_the_encoding() {
    // NS §14.4 (b) and C5: block, drop and overflow give three different encodings.
    let line = |when_full| {
        let text = String::from_utf8(encode::canonical(&pc(when_full))).unwrap();
        text.lines()
            .find(|line| line.starts_with("place r "))
            .unwrap()
            .to_owned()
    };
    assert_eq!(line(WhenFull::Block), "place r 3 block 0");
    assert_eq!(line(WhenFull::Drop), "place r 3 drop 0");
    assert_eq!(line(WhenFull::Overflow), "place r 3 overflow 0");
}

#[test]
fn no_capacity_exceeded_is_encoded_as_capacity_never_exceeded() {
    assert_eq!(
        encode::canonical_property(&NetProperty::CapacityNeverExceeded),
        b"property capacity-never-exceeded\n"
    );
}
