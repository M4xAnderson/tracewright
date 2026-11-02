//! The check pipeline on a lowered scope with a markings budget (NS §9.2, §10.1): verify gets
//! ⌊N/2⌋, recheck the rest. The producer-consumer project is PC(overflow) without its item limit,
//! so the stored counts differ from B1 and B2 but the rule they pin is the same.

use tracewright_engine::Verdict;
use tracewright_engine::check;
use tracewright_engine::lower;
use tracewright_model::Behavior;
use tracewright_model::ElementId;
use tracewright_model::Scope;
use tracewright_net::Budget;
use tracewright_net::Certificate;
use tracewright_net::NetProperty;
use tracewright_net::Phase;
use tracewright_net::Technique;
use tracewright_net::UnknownReason;

use super::support::MemDisk;

fn checked(disk: &MemDisk, property: &NetProperty, max_markings: u64) -> Verdict {
    let model = tracewright_format::load(&disk.files).expect("the fixture loads");
    let scope = Scope {
        element: ElementId::parse("sys").expect("a valid ID"),
        behavior: Behavior::Combined,
    };
    let lowered = lower::lower(&model, &scope).expect("the fixture lowers");
    check::run(&lowered, property, &Budget { max_markings }).verdict
}

#[test]
fn the_overflow_is_found_once_verify_may_store_the_overflow_marking() {
    // Stored at discovery: orders = 0, 1, 2, 3 and the overflow marking orders = 4.
    let disk = MemDisk::fixture("producer-consumer");
    let property = NetProperty::CapacityNeverExceeded;
    assert!(matches!(
        checked(&disk, &property, 10),
        Verdict::Disproved(_)
    ));
    assert_eq!(
        checked(&disk, &property, 9),
        Verdict::Unknown(UnknownReason::BudgetExhausted {
            phase: Phase::Main,
            markings: 4
        })
    );
}

#[test]
fn capacity_exceeded_first_needs_the_complete_exploration() {
    let disk = MemDisk::fixture("producer-consumer");
    let property = NetProperty::DeadlockFree;
    assert!(matches!(
        checked(&disk, &property, 10),
        Verdict::Unknown(UnknownReason::CapacityExceededFirst { .. })
    ));
    assert!(matches!(
        checked(&disk, &property, 9),
        Verdict::Unknown(UnknownReason::BudgetExhausted { .. })
    ));
}

#[test]
fn b13_the_certificate_needs_no_budget() {
    let disk = MemDisk::fixture("e1c-worker-pool");
    let Verdict::Proved(proof) = checked(&disk, &NetProperty::CapacityNeverExceeded, 1) else {
        panic!("Proved by the certificate");
    };
    assert_eq!(
        proof.technique(),
        &Technique::Certificate(Certificate::NoOverflowPlace {
            drop_places: vec![]
        })
    );
    assert_eq!(proof.markings(), 0);
}
