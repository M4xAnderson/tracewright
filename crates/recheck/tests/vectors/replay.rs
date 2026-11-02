//! `recheck::replay` on its own (NS §9.3, seam B): the V13 and V16 counterexamples written out by
//! hand, every counterexample and capacity-exceeded-first run recheck itself finds, and mutations
//! that each break one rule.

use super::support::q;
use tracewright_net::Budget;
use tracewright_net::Counterexample;
use tracewright_net::Evidence;
use tracewright_net::Marking;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::NetVerdict;
use tracewright_net::Step;
use tracewright_net::UnknownReason;
use tracewright_net::Violation;
use tracewright_net::WaitReason;
use tracewright_recheck::ReplayError;

fn marking(counts: &[(&str, u32)]) -> Marking {
    Marking(counts.iter().map(|(id, count)| (q(id), *count)).collect())
}

fn net(name: &str) -> Net {
    super::support::property_vectors().nets[name].to_net()
}

fn replay(net: &Net, cx: &Counterexample) -> Result<(), ReplayError> {
    tracewright_recheck::replay(net, cx, &NetProperty::DeadlockFree)
}

// V13 as NS §14.1 states it, with the markings after each step worked out by hand.
fn v13() -> Counterexample {
    let file = super::support::property_vectors();
    let vector = file
        .vector
        .iter()
        .find(|v| v.id == "V13")
        .expect("V13 is a vector");
    Counterexample {
        steps: vec![
            Step {
                fired: q("ra1/take_x+acq_x"),
                marking: marking(&[("a/has_1", 1), ("b/idle", 1), ("l/y", 1)]),
            },
            Step {
                fired: q("rb1/take_y+acq_y"),
                marking: marking(&[("a/has_1", 1), ("b/has_1", 1)]),
            },
        ],
        violation: Violation::Deadlock {
            waiting: super::support::waiting(
                vector.waiting.as_ref().expect("V13 has a waiting list"),
            ),
        },
    }
}

#[test]
fn the_hand_written_v13_counterexample_replays() {
    assert_eq!(replay(&net("LOCKS_yx"), &v13()), Ok(()));
}

#[test]
fn every_counterexample_recheck_finds_replays() {
    let file = super::support::property_vectors();
    for vector in file.vector.iter().filter(|v| v.expect == "disproved") {
        let net = file.nets[&vector.net].to_net();
        let property = vector.property();
        let verdict = tracewright_recheck::explore(
            &net,
            &property,
            &Budget {
                max_markings: 1 << 20,
            },
        );
        let NetVerdict::Disproved(Evidence::Counterexample(cx)) = verdict else {
            panic!("{}", vector.id)
        };
        assert_eq!(
            tracewright_recheck::replay(&net, &cx, &property),
            Ok(()),
            "{}",
            vector.id
        );
    }
}

// V16 as NS §14.1 states it: produce four times from p/items = 5 into r of capacity 3.
fn v16() -> Counterexample {
    let step = |items: u32, r: u32| Step {
        fired: q("p/produce"),
        marking: marking(&[("p/items", items), ("r", r)]),
    };
    Counterexample {
        steps: vec![step(4, 1), step(3, 2), step(2, 3), step(1, 4)],
        violation: Violation::Overflow {
            place: q("r"),
            capacity: 3,
            count: 4,
        },
    }
}

fn replay_overflow(net: &Net, cx: &Counterexample) -> Result<(), ReplayError> {
    tracewright_recheck::replay(net, cx, &NetProperty::CapacityNeverExceeded)
}

#[test]
fn the_hand_written_v16_run_replays_to_its_overflow_marking() {
    assert_eq!(replay_overflow(&net("PC_overflow"), &v16()), Ok(()));
}

#[test]
fn every_capacity_exceeded_first_run_replays_as_a_no_capacity_exceeded_counterexample() {
    // NS §10.2: the run of an Unknown capacity exceeded first is replayed like V06.
    let file = super::support::property_vectors();
    for vector in file
        .vector
        .iter()
        .filter(|v| v.expect == "capacity-exceeded-first")
    {
        let net = file.nets[&vector.net].to_net();
        let verdict = tracewright_recheck::explore(
            &net,
            &NetProperty::DeadlockFree,
            &Budget {
                max_markings: 1 << 20,
            },
        );
        let NetVerdict::Unknown(UnknownReason::CapacityExceededFirst { place, steps }) = verdict
        else {
            panic!("{}: {verdict:?}", vector.id)
        };
        let last = &steps.last().expect("an overflow run has a step").marking;
        let capacity = net
            .places
            .iter()
            .find(|p| p.id == place)
            .expect("a place of the net")
            .capacity;
        let count = last.0[&place];
        let cx = Counterexample {
            steps,
            violation: Violation::Overflow {
                place,
                capacity,
                count,
            },
        };
        assert_eq!(replay_overflow(&net, &cx), Ok(()), "{}", vector.id);
    }
}

#[test]
fn a_run_that_stops_before_the_overflow_fails_rule_3() {
    let mut cx = v16();
    cx.steps.pop();
    assert_eq!(
        replay_overflow(&net("PC_overflow"), &cx),
        Err(ReplayError::NotAnOverflow)
    );
}

#[test]
fn a_step_after_the_overflow_marking_fails_rule_a() {
    let mut cx = v16();
    cx.steps.push(Step {
        fired: q("c/consume"),
        marking: marking(&[("p/items", 1), ("r", 3), ("c/got", 1)]),
    });
    assert_eq!(
        replay_overflow(&net("PC_overflow"), &cx),
        Err(ReplayError::FromOverflowMarking { step: 5 })
    );
}

#[test]
fn a_wrong_overflow_count_or_capacity_fails_rule_3() {
    for (capacity, count) in [(3, 5), (4, 4)] {
        let mut cx = v16();
        cx.violation = Violation::Overflow {
            place: q("r"),
            capacity,
            count,
        };
        assert_eq!(
            replay_overflow(&net("PC_overflow"), &cx),
            Err(ReplayError::ViolationDiffers),
            "{capacity}, {count}"
        );
    }
}

#[test]
fn an_overflow_that_names_a_larger_id_than_the_smallest_exceeded_place_fails_rule_3() {
    // V27: both x/a and x/b exceed their capacity; the violation must name x/a.
    let cx = Counterexample {
        steps: vec![Step {
            fired: q("x/t"),
            marking: marking(&[("x/a", 2), ("x/b", 2)]),
        }],
        violation: Violation::Overflow {
            place: q("x/b"),
            capacity: 1,
            count: 2,
        },
    };
    assert_eq!(
        replay_overflow(&net("V27"), &cx),
        Err(ReplayError::ViolationDiffers)
    );
}

#[test]
fn a_deadlock_counterexample_does_not_prove_an_overflow_nor_the_other_way_round() {
    assert_eq!(
        replay_overflow(&net("LOCKS_yx"), &v13()),
        Err(ReplayError::NotAnOverflow)
    );
    assert_eq!(
        replay(&net("PC_overflow"), &v16()),
        Err(ReplayError::NotADeadlock)
    );
}

#[test]
fn a_drop_place_replays_with_its_clamp() {
    // V20: five produces into r (drop, capacity 3) leave r at 3, not 5.
    let mut steps = Vec::new();
    for items in (0..5).rev() {
        steps.push(Step {
            fired: q("p/produce"),
            marking: marking(&[("p/items", items), ("r", (5 - items).min(3))]),
        });
    }
    let after_produce = steps.clone();
    let mut cx = Counterexample {
        steps,
        violation: Violation::Overflow {
            place: q("r"),
            capacity: 3,
            count: 4,
        },
    };
    assert_eq!(
        replay_overflow(&net("PC_drop"), &cx),
        Err(ReplayError::NotAnOverflow)
    );
    cx.steps = after_produce;
    cx.steps[4].marking.0.insert(q("r"), 5);
    assert_eq!(
        replay_overflow(&net("PC_drop"), &cx),
        Err(ReplayError::MarkingDiffers { step: 5 })
    );
}

#[test]
fn a_transition_the_net_does_not_have_fails_rule_b() {
    let mut cx = v13();
    cx.steps[1].fired = q("rb1/take_x+acq_x");
    assert_eq!(
        replay(&net("LOCKS_yx"), &cx),
        Err(ReplayError::UnknownTransition {
            step: 2,
            transition: "rb1/take_x+acq_x".to_owned()
        })
    );
}

#[test]
fn a_step_that_is_not_enabled_fails_rule_c() {
    let mut cx = v13();
    cx.steps[1].fired = q("ra1/take_x+acq_x");
    assert_eq!(
        replay(&net("LOCKS_yx"), &cx),
        Err(ReplayError::NotEnabled {
            step: 2,
            transition: "ra1/take_x+acq_x".to_owned()
        })
    );
}

#[test]
fn a_wrong_recorded_marking_fails_rule_e() {
    let mut cx = v13();
    cx.steps[0].marking.0.insert(q("l/x"), 1);
    assert_eq!(
        replay(&net("LOCKS_yx"), &cx),
        Err(ReplayError::MarkingDiffers { step: 1 })
    );
}

#[test]
fn a_recorded_marking_that_names_a_place_outside_the_net_fails_rule_e() {
    let mut cx = v13();
    cx.steps[0].marking.0.insert(q("z/ghost"), 1);
    assert_eq!(
        replay(&net("LOCKS_yx"), &cx),
        Err(ReplayError::MarkingDiffers { step: 1 })
    );
}

#[test]
fn a_run_that_stops_short_of_the_deadlock_fails_rule_3() {
    let mut cx = v13();
    cx.steps.pop();
    assert_eq!(
        replay(&net("LOCKS_yx"), &cx),
        Err(ReplayError::NotADeadlock)
    );
}

#[test]
fn a_wrong_waiting_list_fails_rule_3() {
    let mut cx = v13();
    let Violation::Deadlock { waiting } = &mut cx.violation else {
        unreachable!("v13 is a deadlock")
    };
    waiting[1].waits[0].reason = WaitReason::NeedsTokens {
        needed: 2,
        holds: 0,
    };
    assert_eq!(
        replay(&net("LOCKS_yx"), &cx),
        Err(ReplayError::WaitingDiffers)
    );
}

#[test]
fn a_waiting_list_missing_a_transition_fails_rule_3() {
    let mut cx = v13();
    let Violation::Deadlock { waiting } = &mut cx.violation else {
        unreachable!("v13 is a deadlock")
    };
    waiting.pop();
    assert_eq!(
        replay(&net("LOCKS_yx"), &cx),
        Err(ReplayError::WaitingDiffers)
    );
}

#[test]
fn an_invalid_net_is_refused() {
    assert_eq!(replay(&net("V29"), &v13()), Err(ReplayError::InvalidNet));
}
