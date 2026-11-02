//! The markings budget of a check before an overflow, a capacity exceeded first and a certificate
//! (NS §9.2, §10.1, B13): verify gets ⌊N/2⌋. The producer-consumer project is PC(overflow) without
//! its item limit, so the stored counts differ from B1 and B2 but the rule they pin is the same.

use std::time::Duration;

use tracewright_engine::CheckBudget;
use tracewright_engine::dto::NoOverflowPlaceDto;
use tracewright_engine::dto::Outcome;
use tracewright_engine::dto::Phase;
use tracewright_engine::dto::Technique;
use tracewright_engine::dto::UnknownDto;
use tracewright_engine::dto::VerdictDto;

use super::support::FakeClock;
use super::support::MemDisk;
use super::support::Run;
use super::support::run_checks_within;

fn run(disk: &MemDisk, max_markings: u64) -> Run {
    let budget = CheckBudget {
        max_markings,
        time: Duration::from_secs(2),
    };
    run_checks_within(disk, budget, &FakeClock::new(), |_, _| {})
}

fn unknown(verdict: &VerdictDto) -> &UnknownDto {
    let Outcome::Unknown(unknown) = &verdict.outcome else {
        panic!("not Unknown: {verdict:?}")
    };
    &unknown.reason
}

#[test]
fn the_overflow_is_found_once_verify_may_store_the_overflow_marking() {
    // Stored at discovery: orders = 0, 1, 2, 3 and the overflow marking orders = 4.
    let disk = MemDisk::fixture("producer-consumer");
    assert!(matches!(
        run(&disk, 10).verdict("no-overflow").outcome,
        Outcome::Disproved(_)
    ));
    assert_eq!(
        unknown(run(&disk, 9).verdict("no-overflow")),
        &UnknownDto::BudgetExhausted {
            phase: Phase::Main,
            markings: 4
        }
    );
}

#[test]
fn capacity_exceeded_first_needs_the_complete_exploration() {
    let disk = MemDisk::fixture("producer-consumer");
    assert!(matches!(
        unknown(run(&disk, 10).verdict("no-deadlock")),
        UnknownDto::CapacityExceededFirst { .. }
    ));
    assert!(matches!(
        unknown(run(&disk, 9).verdict("no-deadlock")),
        UnknownDto::BudgetExhausted { .. }
    ));
}

#[test]
fn b13_the_certificate_needs_no_budget() {
    let run = run(&MemDisk::fixture("e1c-worker-pool"), 1);
    let Outcome::Proved(proof) = &run.verdict("no-overflow").outcome else {
        panic!("{:?}", run.verdict("no-overflow"))
    };
    assert_eq!(
        proof.technique,
        Technique::NoOverflowPlace(NoOverflowPlaceDto {
            drop_places: vec![]
        })
    );
    assert_eq!(proof.markings, 0);
}
