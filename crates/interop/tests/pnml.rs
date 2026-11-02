//! The PNML writer against golden files written by hand from the encoding rule of NS §18
//! item 10 (D-35): `block` places get a complement place with net-effect arcs, `overflow` places
//! stay plain, nets with a `drop` place or counts TAPAAL cannot hold are excluded.

use std::collections::BTreeMap;

use tracewright_interop::pnml;
use tracewright_net::Marking;
use tracewright_net::Net;
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

#[test]
fn a_read_arc_on_a_full_block_place_gets_no_complement_arc() {
    // NS V08: x/t has net effect 0 on x/p, so the self-loop at capacity stays enabled (E3). The
    // textbook complement would add a pre arc from the empty complement (E1 surprise 1).
    let v08 = net(
        vec![place("x/p", 1, WhenFull::Block)],
        vec![transition("x/t", &[("x/p", 1)], &[("x/p", 1)])],
        &[("x/p", 1)],
    );
    assert_eq!(
        pnml::write(&v08),
        Ok(include_str!("golden/v08.pnml").to_owned())
    );
}

#[test]
fn a_block_place_gets_a_complement_with_its_room_and_net_effect_arcs() {
    // NS V09: x/t takes 2 from x/p (its complement gains 2) and puts 1 into x/q (its complement,
    // initially 1, loses 1).
    let v09 = net(
        vec![
            place("x/p", 3, WhenFull::Block),
            place("x/q", 1, WhenFull::Block),
        ],
        vec![transition("x/t", &[("x/p", 2)], &[("x/q", 1)])],
        &[("x/p", 3)],
    );
    assert_eq!(
        pnml::write(&v09),
        Ok(include_str!("golden/v09.pnml").to_owned())
    );
}

#[test]
fn an_overflow_place_stays_plain_and_everything_is_in_canonical_order() {
    // NS PC(overflow): the generated place r has no complement; the queries bound it instead.
    // The places and transitions are given out of order.
    let pc = net(
        vec![
            place("r", 3, WhenFull::Overflow),
            place("p/items", 5, WhenFull::Block),
            place("c/got", 5, WhenFull::Block),
        ],
        vec![
            transition("p/produce", &[("p/items", 1)], &[("r", 1)]),
            transition("c/consume", &[("r", 1)], &[("c/got", 1)]),
        ],
        &[("p/items", 5)],
    );
    assert_eq!(
        pnml::write(&pc),
        Ok(include_str!("golden/pc-overflow.pnml").to_owned())
    );
}

#[test]
fn qualified_ids_map_to_xml_names() {
    assert_eq!(pnml::pnml_id(&q("x/p")), "x__p");
    assert_eq!(pnml::pnml_id(&q("r1/start+acquire")), "r1__start.acquire");
    assert_eq!(pnml::pnml_id(&q("jobs")), "jobs");
}

#[test]
fn a_net_with_a_drop_place_is_excluded() {
    // NS V12: every plain encoding of drop either reports a false deadlock or is unbounded (D-35).
    let v12 = net(
        vec![place("x/p", 1, WhenFull::Drop)],
        vec![transition("x/t", &[("x/p", 1)], &[("x/p", 2)])],
        &[("x/p", 1)],
    );
    assert_eq!(
        pnml::write(&v12),
        Err(pnml::Excluded::DropPlace { place: q("x/p") })
    );
}

#[test]
fn a_net_whose_counts_reach_past_i32_is_excluded() {
    // NS V51 and V52: capacity u32::MAX − 1 plus a post weight of 1 is beyond verifypn's i32.
    for when_full in [WhenFull::Overflow, WhenFull::Block] {
        let v51 = net(
            vec![
                place("x/a", 1, WhenFull::Block),
                place("x/p", 4_294_967_294, when_full),
            ],
            vec![transition("x/t", &[], &[("x/p", 1)])],
            &[("x/p", 4_294_967_294)],
        );
        assert_eq!(
            pnml::write(&v51),
            Err(pnml::Excluded::CountBeyondTapaal { place: q("x/p") }),
            "{when_full:?}"
        );
    }
}

#[test]
fn the_largest_count_verifypn_holds_is_written() {
    // cap + max W⁺ = 2³¹ − 1 exactly: the edge of the i32 range is still in the corpus.
    let edge = net(
        vec![place("x/p", 2_147_483_646, WhenFull::Overflow)],
        vec![transition("x/t", &[], &[("x/p", 1)])],
        &[],
    );
    assert!(pnml::write(&edge).is_ok());
    let past = net(
        vec![place("x/p", 2_147_483_647, WhenFull::Overflow)],
        vec![transition("x/t", &[], &[("x/p", 1)])],
        &[],
    );
    assert_eq!(
        pnml::write(&past),
        Err(pnml::Excluded::CountBeyondTapaal { place: q("x/p") })
    );
}

#[test]
fn a_pre_weight_beyond_i32_is_excluded() {
    // A transition that can never fire still has its arc written, and verifypn parses it as i32.
    let dead = net(
        vec![place("x/p", 1, WhenFull::Block)],
        vec![transition("x/t", &[("x/p", 2_147_483_648)], &[])],
        &[],
    );
    assert_eq!(
        pnml::write(&dead),
        Err(pnml::Excluded::CountBeyondTapaal { place: q("x/p") })
    );
}
