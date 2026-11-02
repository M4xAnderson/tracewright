//! The budget of NS §10.1 in recheck: the §14.2 vectors, the deadline and the cancel flag polled
//! every 1,024 expansions, and progress at every poll.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use std::time::Instant;

use tracewright_net::Budget;
use tracewright_net::Clock;
use tracewright_net::Deadline;
use tracewright_net::Evidence;
use tracewright_net::Net;
use tracewright_net::NetProperty;
use tracewright_net::NetVerdict;
use tracewright_net::Phase;
use tracewright_net::Place;
use tracewright_net::Technique;
use tracewright_net::Transition;
use tracewright_net::UnknownReason;
use tracewright_net::WhenFull;

use super::support::budget_vectors;
use super::support::q;

fn explore(net: &Net, budget: &Budget) -> NetVerdict {
    tracewright_recheck::explore(net, &NetProperty::DeadlockFree, budget)
}

fn markings(max_markings: u64, cancel: bool) -> Budget {
    Budget {
        max_markings,
        cancel: Arc::new(AtomicBool::new(cancel)),
        ..Budget::default()
    }
}

#[test]
fn the_budget_vectors_hold_on_the_main_share_and_recheck_completes_on_its_own() {
    // Both explorers follow the same marking accounting, so on ⌊N/2⌋ recheck ends like the main
    // phase; on its own share N − ⌊N/2⌋ it completes every exploration the main phase completed.
    let file = budget_vectors();
    for vector in &file.vector {
        let net = file.nets[&vector.net].to_net();
        let id = &vector.id;
        let verdict = explore(&net, &markings(vector.n / 2, vector.cancel));
        match vector.expect.as_str() {
            "exhausted" => assert_eq!(
                verdict,
                NetVerdict::Unknown(UnknownReason::BudgetExhausted {
                    phase: Phase::Recheck,
                    markings: vector.markings.unwrap(),
                }),
                "{id}"
            ),
            "proved" => {
                let proved = NetVerdict::Proved {
                    technique: Technique::Exhaustive,
                    markings: vector.markings.unwrap(),
                };
                assert_eq!(verdict, proved, "{id}");
                let rest = vector.n - vector.n / 2;
                assert_eq!(explore(&net, &markings(rest, false)), proved, "{id}");
            }
            "disproved" => {
                let NetVerdict::Disproved(Evidence::Counterexample(cx)) = verdict else {
                    panic!("{id}: {verdict:?}");
                };
                assert_eq!(cx.steps.len(), vector.steps.unwrap(), "{id}");
            }
            "cancelled" => assert_eq!(
                verdict,
                NetVerdict::Unknown(UnknownReason::Cancelled),
                "{id}"
            ),
            other => panic!("{id}: unknown expectation {other}"),
        }
    }
}

// Returns the readings in order, then repeats the last one.
#[derive(Debug)]
struct Script(Mutex<Vec<Instant>>);

impl Clock for Script {
    fn now(&self) -> Instant {
        let mut left = self.0.lock().expect("test clock");
        if left.len() > 1 {
            left.remove(0)
        } else {
            left[0]
        }
    }
}

fn timed(at: Instant, readings: Vec<Instant>, cancel: bool) -> Budget {
    Budget {
        max_markings: 1 << 20,
        deadline: Some(Deadline {
            at,
            clock: Arc::new(Script(Mutex::new(readings))),
        }),
        cancel: Arc::new(AtomicBool::new(cancel)),
    }
}

fn pool() -> Net {
    let file = budget_vectors();
    file.nets["POOL"].to_net()
}

// A chain of 3,001 markings: `x/p(3000, block)`, `x/t: ∅ → x/p`; a deadlock at `x/p = 3000`.
fn chain() -> Net {
    Net {
        places: vec![Place {
            id: q("x/p"),
            capacity: 3000,
            when_full: WhenFull::Block,
        }],
        transitions: vec![Transition {
            id: q("x/t"),
            pre: BTreeMap::new(),
            post: BTreeMap::from([(q("x/p"), 1)]),
        }],
        ..Net::default()
    }
}

#[test]
fn a_deadline_passed_at_the_first_poll_is_budget_exhausted_while_re_checking() {
    // B9's recheck half: the deadline has passed when recheck first polls, after storing M0.
    let t0 = Instant::now();
    assert_eq!(
        explore(&pool(), &timed(t0, vec![t0], false)),
        NetVerdict::Unknown(UnknownReason::BudgetExhausted {
            phase: Phase::Recheck,
            markings: 1
        })
    );
}

#[test]
fn a_deadline_not_reached_lets_recheck_finish() {
    let t0 = Instant::now();
    let budget = timed(t0 + Duration::from_secs(1), vec![t0], false);
    assert!(matches!(
        explore(&pool(), &budget),
        NetVerdict::Proved { markings: 3, .. }
    ));
}

#[test]
fn cancel_wins_over_the_deadline_at_the_same_poll() {
    let t0 = Instant::now();
    assert_eq!(
        explore(&pool(), &timed(t0, vec![t0], true)),
        NetVerdict::Unknown(UnknownReason::Cancelled)
    );
}

#[test]
fn the_second_poll_comes_after_1024_expansions() {
    let t0 = Instant::now();
    let at = t0 + Duration::from_secs(1);
    assert_eq!(
        explore(&chain(), &timed(at, vec![t0, at], false)),
        NetVerdict::Unknown(UnknownReason::BudgetExhausted {
            phase: Phase::Recheck,
            markings: 1025
        })
    );
}

#[test]
fn progress_is_reported_at_every_poll() {
    let mut seen = Vec::new();
    let verdict = tracewright_recheck::explore_reporting(
        &chain(),
        &NetProperty::DeadlockFree,
        &markings(1 << 20, false),
        &mut |progress| seen.push(progress.markings),
    );
    assert!(matches!(verdict, NetVerdict::Disproved(_)));
    assert_eq!(seen, [1, 1025, 2049]);
}
