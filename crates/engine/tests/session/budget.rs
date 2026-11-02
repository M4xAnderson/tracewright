//! The budget of a check through the Session (NS §10.1, ADR 0006): N markings and T of time, split
//! between the main and the recheck phase, with a fake Clock and an inline Spawner. A check that
//! runs out is Unknown "budget exhausted", never anything that looks like success.

use std::time::Duration;

use tracewright_engine::CheckBudget;
use tracewright_engine::EngineEvent;
use tracewright_engine::dto::CheckProgress;
use tracewright_engine::dto::Outcome;
use tracewright_engine::dto::Phase;
use tracewright_engine::dto::Technique;
use tracewright_engine::dto::UnknownDto;
use tracewright_engine::dto::VerdictDto;

use super::support::FakeClock;
use super::support::MemDisk;
use super::support::one_job;
use super::support::run_checks_within;

const T: Duration = Duration::from_secs(2);
const JUST_BEFORE_HALF: Duration = Duration::from_nanos(999_999_999);
const _: () = assert!(
    JUST_BEFORE_HALF.as_nanos() + 1 == T.as_nanos() / 2,
    "one nanosecond before T/2"
);

fn markings(max_markings: u64) -> CheckBudget {
    CheckBudget {
        max_markings,
        time: T,
    }
}

fn exhausted(verdict: &VerdictDto) -> (Phase, u64) {
    match &verdict.outcome {
        Outcome::Unknown(unknown) => match unknown.reason {
            UnknownDto::BudgetExhausted { phase, markings } => (phase, markings),
            ref other => panic!("not budget exhausted: {other:?}"),
        },
        other => panic!("not Unknown: {other:?}"),
    }
}

// One element whose transition `arrive` adds a token to `queue(3000, block)` until it is full: a
// chain of 3,001 markings, one expansion each, that ends in a deadlock at `queue = 3000`.
fn chain() -> MemDisk {
    one_job().put(
        "elements/job.toml",
        r#"format = 1
kind = "container"
name = "Job"
parent = "sys"

[petri_net.places.queue]
capacity = 3000
when_full = "block"

[petri_net.transitions.arrive]
post = { queue = 1 }
"#,
    )
}

fn progress(events: &[EngineEvent]) -> Vec<(Phase, u64)> {
    events
        .iter()
        .filter_map(|event| match event {
            EngineEvent::CheckProgress(CheckProgress {
                phase, markings, ..
            }) => Some((*phase, *markings)),
            _ => None,
        })
        .collect()
}

#[test]
fn given_too_few_markings_for_the_main_phase_then_budget_exhausted_with_its_half() {
    // B5 through the engine: the worker pool has 3 reachable markings; N = 5 gives verify 2.
    let clock = FakeClock::new();
    let run = run_checks_within(
        &MemDisk::fixture("e1c-worker-pool"),
        markings(5),
        &clock,
        |_, _| {},
    );
    assert_eq!(exhausted(run.verdict("no-deadlock")), (Phase::Main, 2));
}

#[test]
fn given_enough_markings_for_both_halves_then_proved() {
    // B6: N = 6 gives verify 3 and recheck 3, both complete.
    let clock = FakeClock::new();
    let run = run_checks_within(
        &MemDisk::fixture("e1c-worker-pool"),
        markings(6),
        &clock,
        |_, _| {},
    );
    let Outcome::Proved(proof) = &run.verdict("no-deadlock").outcome else {
        panic!("not Proved: {:?}", run.verdict("no-deadlock"));
    };
    assert_eq!(
        (proof.technique, proof.markings),
        (Technique::Exhaustive, 3)
    );
}

#[test]
fn given_the_deadline_passes_after_the_main_phase_then_budget_exhausted_while_re_checking() {
    // B9 with deadlock-free: the main phase proves the worker pool before t0 + T/2; when recheck
    // first polls, the clock reads t0 + T.
    let clock = FakeClock::new();
    let t0 = clock.now_reading();
    let run = run_checks_within(
        &MemDisk::fixture("e1c-worker-pool"),
        markings(100),
        &clock,
        move |clock, event| {
            if let EngineEvent::CheckProgress(CheckProgress {
                phase: Phase::Main, ..
            }) = event
            {
                clock.set(t0 + T);
            }
        },
    );
    assert_eq!(exhausted(run.verdict("no-deadlock")), (Phase::Recheck, 1));
}

#[test]
fn given_the_clock_reaches_half_the_time_then_the_main_phase_ends_at_its_next_poll() {
    // The first poll (expansion 0) reports progress; the clock then reads t0 + T/2, and the next
    // poll, at expansion 1024 with markings 0 to 1024 stored, ends the main phase.
    let clock = FakeClock::new();
    let t0 = clock.now_reading();
    let run = run_checks_within(
        &chain(),
        markings(1_000_000),
        &clock,
        move |clock, event| {
            if let EngineEvent::CheckProgress(_) = event {
                clock.set(t0 + T / 2);
            }
        },
    );
    assert_eq!(exhausted(run.verdict("no-deadlock")), (Phase::Main, 1025));
}

#[test]
fn given_the_clock_stays_just_before_half_the_time_then_the_main_phase_finishes() {
    let clock = FakeClock::new();
    let t0 = clock.now_reading();
    let run = run_checks_within(
        &chain(),
        markings(1_000_000),
        &clock,
        move |clock, event| {
            if let EngineEvent::CheckProgress(_) = event {
                clock.set(t0 + JUST_BEFORE_HALF);
            }
        },
    );
    let Outcome::Disproved(disproved) = &run.verdict("no-deadlock").outcome else {
        panic!("not Disproved: {:?}", run.verdict("no-deadlock"));
    };
    assert_eq!(disproved.frames.len(), 3000);
}

#[test]
fn a_running_check_reports_the_markings_explored_at_most_four_times_a_second() {
    // The clock stands still, so after the first count only the 250 ms throttle decides.
    let clock = FakeClock::new();
    let run = run_checks_within(&chain(), markings(1_000_000), &clock, |_, _| {});
    assert_eq!(progress(&run.events), [(Phase::Main, 1)]);
}

#[test]
fn every_poll_is_reported_when_a_quarter_second_passes_between_them() {
    let clock = FakeClock::new();
    let run = run_checks_within(
        &chain(),
        markings(1_000_000),
        &clock,
        move |clock, event| {
            if let EngineEvent::CheckProgress(_) = event {
                clock.advance(Duration::from_millis(250));
            }
        },
    );
    assert_eq!(
        progress(&run.events),
        [(Phase::Main, 1), (Phase::Main, 1025), (Phase::Main, 2049)]
    );
}

#[test]
fn a_proved_check_reports_the_recheck_phase_too() {
    let clock = FakeClock::new();
    let run = run_checks_within(
        &MemDisk::fixture("e1c-worker-pool"),
        markings(100),
        &clock,
        |_, _| {},
    );
    assert_eq!(
        progress(&run.events),
        [(Phase::Main, 1), (Phase::Recheck, 1)]
    );
    // Progress comes after the check was queued and before its verdict.
    let kinds: Vec<&str> = run
        .events
        .iter()
        .map(|event| match event {
            EngineEvent::CurrentKeys(_) => "keys",
            EngineEvent::CheckQueued(_) => "queued",
            EngineEvent::CheckProgress(_) => "progress",
            EngineEvent::Verdict(_) => "verdict",
        })
        .collect();
    assert_eq!(kinds, ["keys", "queued", "progress", "progress", "verdict"]);
}
