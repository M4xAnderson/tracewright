//! The TAPAAL queries of NS §18 item 10 (D-71) and how their answers decide a verdict kind,
//! without running verifypn: the answers come from a script of expected questions.

use std::collections::BTreeMap;

use tracewright_interop::tapaal;
use tracewright_interop::tapaal::Query;
use tracewright_interop::tapaal::VerdictKind;
use tracewright_net::Marking;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::Place;
use tracewright_net::QualifiedId;
use tracewright_net::Transition;
use tracewright_net::WhenFull;

fn q(id: &str) -> QualifiedId {
    QualifiedId(id.to_owned())
}

fn place(id: &str, capacity: u32, when_full: WhenFull) -> Place {
    Place {
        id: q(id),
        capacity,
        when_full,
    }
}

fn transition(id: &str, pre: &[(&str, u32)], post: &[(&str, u32)]) -> Transition {
    let arcs = |arcs: &[(&str, u32)]| arcs.iter().map(|(p, w)| (q(p), *w)).collect();
    Transition {
        id: q(id),
        pre: arcs(pre),
        post: arcs(post),
    }
}

fn net(places: Vec<Place>, transitions: Vec<Transition>, initial: &[(&str, u32)]) -> Net {
    let initial = initial
        .iter()
        .map(|(p, n)| (q(p), *n))
        .collect::<BTreeMap<_, _>>();
    Net {
        places,
        transitions,
        initial: Marking(initial),
    }
}

// NS V08: block places only.
fn v08() -> Net {
    net(
        vec![place("x/p", 1, WhenFull::Block)],
        vec![transition("x/t", &[("x/p", 1)], &[("x/p", 1)])],
        &[("x/p", 1)],
    )
}

// NS V27: two overflow places.
fn v27() -> Net {
    net(
        vec![
            place("x/b", 1, WhenFull::Overflow),
            place("x/a", 1, WhenFull::Overflow),
        ],
        vec![transition("x/t", &[], &[("x/a", 1), ("x/b", 1)])],
        &[("x/a", 1), ("x/b", 1)],
    )
}

// Answers the questions in `script` in order and fails on any other question.
fn decide(net: &Net, property: &NetProperty, script: &[(Query, bool)]) -> VerdictKind {
    let mut asked = 0;
    let kind = tapaal::decide(net, property, |query| {
        let (expected, answer) = script.get(asked).copied().expect("an unscripted question");
        assert_eq!(query, expected, "question {asked}");
        asked += 1;
        Ok::<bool, ()>(answer)
    })
    .expect("a scripted answer is never an error");
    assert_eq!(asked, script.len(), "questions asked");
    kind
}

#[test]
fn deadlock_free_without_overflow_places_is_one_reachability_question() {
    let dl = NetProperty::DeadlockFree;
    let script = |answer| [(Query::DeadlockReachable, answer)];
    assert_eq!(decide(&v08(), &dl, &script(true)), VerdictKind::Disproved);
    assert_eq!(decide(&v08(), &dl, &script(false)), VerdictKind::Proved);
}

#[test]
fn deadlock_free_with_overflow_places_asks_whether_it_stays_in_bounds_next() {
    let dl = NetProperty::DeadlockFree;
    assert_eq!(
        decide(&v27(), &dl, &[(Query::DeadlockReachable, true)]),
        VerdictKind::Disproved
    );
    assert_eq!(
        decide(
            &v27(),
            &dl,
            &[
                (Query::DeadlockReachable, false),
                (Query::AlwaysInBounds, true)
            ]
        ),
        VerdictKind::Proved
    );
    assert_eq!(
        decide(
            &v27(),
            &dl,
            &[
                (Query::DeadlockReachable, false),
                (Query::AlwaysInBounds, false)
            ]
        ),
        VerdictKind::CapacityExceededFirst
    );
}

#[test]
fn no_capacity_exceeded_is_always_in_bounds() {
    let cap = NetProperty::CapacityNeverExceeded;
    for net in [v08(), v27()] {
        let script = |answer| [(Query::AlwaysInBounds, answer)];
        assert_eq!(decide(&net, &cap, &script(true)), VerdictKind::Proved);
        assert_eq!(decide(&net, &cap, &script(false)), VerdictKind::Disproved);
    }
}

#[test]
fn an_error_from_verifypn_ends_the_decision() {
    let result = tapaal::decide(&v27(), &NetProperty::DeadlockFree, |_| Err("exit 3"));
    assert_eq!(result, Err("exit 3"));
}

// Spike E1 net X1: a deadlock reachable only through an overflow marking.
fn x1() -> Net {
    net(
        vec![
            place("x/s", 1, WhenFull::Block),
            place("x/p", 1, WhenFull::Overflow),
        ],
        vec![
            transition("x/t1", &[("x/s", 1)], &[("x/p", 2)]),
            transition("x/t2", &[("x/p", 2)], &[]),
        ],
        &[("x/s", 1)],
    )
}

#[test]
fn a_deadlock_query_on_a_net_without_overflow_places_is_ef_deadlock() {
    assert_eq!(
        tapaal::query_xml(&v08(), Query::DeadlockReachable),
        include_str!("golden/deadlock-reachable-v08.xml")
    );
}

#[test]
fn a_deadlock_query_on_a_net_with_overflow_places_stays_in_bounds_until_the_deadlock() {
    assert_eq!(
        tapaal::query_xml(&v27(), Query::DeadlockReachable),
        include_str!("golden/deadlock-reachable-v27.xml")
    );
    assert_eq!(
        tapaal::query_xml(&x1(), Query::DeadlockReachable),
        include_str!("golden/deadlock-reachable-x1.xml")
    );
}

#[test]
fn always_in_bounds_bounds_every_overflow_place_by_its_capacity() {
    assert_eq!(
        tapaal::query_xml(&v27(), Query::AlwaysInBounds),
        include_str!("golden/always-in-bounds-v27.xml")
    );
    assert_eq!(
        tapaal::query_xml(&v08(), Query::AlwaysInBounds),
        include_str!("golden/always-in-bounds-v08.xml")
    );
}
