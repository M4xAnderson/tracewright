//! `recheck::replay` on its own (NS §9.3, seam B): the V13 counterexample written out by hand,
//! every counterexample recheck itself finds, and mutations that each break one rule.

use super::support::q;
use tracewright_net::Budget;
use tracewright_net::Counterexample;
use tracewright_net::Evidence;
use tracewright_net::Marking;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::NetVerdict;
use tracewright_net::Step;
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
        let verdict = tracewright_recheck::explore(
            &net,
            &NetProperty::DeadlockFree,
            &Budget {
                max_markings: 1 << 20,
            },
        );
        let NetVerdict::Disproved(Evidence::Counterexample(cx)) = verdict else {
            panic!("{}", vector.id)
        };
        assert_eq!(replay(&net, &cx), Ok(()), "{}", vector.id);
    }
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
    let Violation::Deadlock { waiting } = &mut cx.violation;
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
    let Violation::Deadlock { waiting } = &mut cx.violation;
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
