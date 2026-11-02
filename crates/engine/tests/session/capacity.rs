//! No capacity exceeded and asynchronous bindings through the Session (seam A): a producer bound
//! to a consumer through a generated place, checked end to end with the evidence re-checked by
//! recheck before a verdict is emitted.

use tracewright_engine::dto::NoOverflowPlaceDto;
use tracewright_engine::dto::Outcome;
use tracewright_engine::dto::OverflowDto;
use tracewright_engine::dto::PlaceOrigin;
use tracewright_engine::dto::PlaceRef;
use tracewright_engine::dto::Technique;
use tracewright_engine::dto::TransitionOrigin;
use tracewright_engine::dto::TransitionRef;
use tracewright_engine::dto::UnknownDto;
use tracewright_engine::dto::VerdictDto;
use tracewright_engine::dto::ViolationDto;

use super::support::MemDisk;
use super::support::run_checks;

fn producer_consumer() -> MemDisk {
    MemDisk::fixture("producer-consumer")
}

// The binding of the fixture with a stated when_full.
fn with_when_full(when_full: &str) -> MemDisk {
    producer_consumer().edit(
        "elements/api.toml",
        "capacity = 3\n",
        &format!("capacity = 3\nwhen_full = \"{when_full}\"\n"),
    )
}

fn orders() -> PlaceOrigin {
    PlaceOrigin::Generated {
        relationship: "orders".to_owned(),
    }
}

fn publish() -> TransitionOrigin {
    TransitionOrigin::Own(TransitionRef {
        element: "api".to_owned(),
        transition: "publish".to_owned(),
    })
}

fn proved(verdict: &VerdictDto) -> (Technique, u64) {
    let Outcome::Proved(proof) = &verdict.outcome else {
        panic!("not Proved: {verdict:?}")
    };
    (proof.technique.clone(), proof.markings)
}

fn certificate(drop_places: Vec<PlaceOrigin>) -> (Technique, u64) {
    (
        Technique::NoOverflowPlace(NoOverflowPlaceDto { drop_places }),
        0,
    )
}

#[test]
fn a_queue_without_back_pressure_is_disproved_by_the_run_that_overflows_it() {
    // Given a producer bound to a consumer asynchronously with capacity 3 and no when_full, when
    // no capacity exceeded is checked, then the last step puts the fourth message into `orders`.
    let run = run_checks(&producer_consumer());
    let Outcome::Disproved(disproved) = &run.verdict("no-overflow").outcome else {
        panic!("{:?}", run.verdict("no-overflow"))
    };
    let fired: Vec<&TransitionOrigin> = disproved.frames.iter().map(|f| &f.fired).collect();
    assert_eq!(fired, [&publish(), &publish(), &publish(), &publish()]);
    assert_eq!(
        disproved.violation,
        ViolationDto::Overflow(OverflowDto {
            place: orders(),
            capacity: 3,
            count: 4,
        })
    );
}

#[test]
fn deadlock_free_stops_at_the_first_overflow_as_capacity_exceeded_first() {
    // The producer can always publish, so there is no deadlock, but the queue can overflow.
    let run = run_checks(&producer_consumer());
    let Outcome::Unknown(unknown) = &run.verdict("no-deadlock").outcome else {
        panic!("{:?}", run.verdict("no-deadlock"))
    };
    let UnknownDto::CapacityExceededFirst {
        place,
        capacity,
        count,
        frames,
    } = &unknown.reason
    else {
        panic!("{:?}", unknown.reason)
    };
    assert_eq!((place, *capacity, *count), (&orders(), 3, 4));
    assert_eq!(frames.len(), 4);
    assert!(frames.iter().all(|frame| frame.fired == publish()));
}

#[test]
fn a_blocking_queue_is_proved_by_the_certificate_and_never_deadlocks() {
    let run = run_checks(&with_when_full("block"));
    assert_eq!(proved(run.verdict("no-overflow")), certificate(vec![]));
    // Four counts of `orders` (0–3), each with the producer and consumer ready.
    assert_eq!(
        proved(run.verdict("no-deadlock")),
        (Technique::Exhaustive, 4)
    );
}

#[test]
fn a_dropping_queue_is_proved_and_the_certificate_names_where_messages_are_lost() {
    let run = run_checks(&with_when_full("drop"));
    assert_eq!(
        proved(run.verdict("no-overflow")),
        certificate(vec![orders()])
    );
}

#[test]
fn an_explicit_overflow_is_the_same_net_as_the_default() {
    // ADR 0014: lowering fills in the default, so the key does not depend on writing it.
    let default = run_checks(&producer_consumer());
    let explicit = run_checks(&with_when_full("overflow"));
    assert_eq!(default.key("no-overflow"), explicit.key("no-overflow"));
    assert_ne!(
        default.key("no-overflow"),
        run_checks(&with_when_full("block")).key("no-overflow"),
        "NS §14.4 (b): when full is part of the key"
    );
}

#[test]
fn the_worker_pool_has_no_overflow_place_and_is_proved_by_the_certificate() {
    // V23: every place blocks.
    let run = run_checks(&MemDisk::fixture("e1c-worker-pool"));
    assert_eq!(proved(run.verdict("no-overflow")), certificate(vec![]));
}

#[test]
fn a_drop_place_inside_a_behavior_model_is_named_by_the_certificate() {
    let disk = MemDisk::fixture("e1c-worker-pool").edit(
        "elements/w.toml",
        "when_full = \"block\"",
        "when_full = \"drop\"",
    );
    let run = run_checks(&disk);
    let free = PlaceOrigin::Own(PlaceRef {
        element: "w".to_owned(),
        place: "free".to_owned(),
    });
    assert_eq!(proved(run.verdict("no-overflow")), certificate(vec![free]));
}

#[test]
fn an_overflow_place_inside_a_behavior_model_is_named_by_the_overflow() {
    // c/running can never exceed 3, so the scope is Proved by exhaustion: three markings.
    let disk = MemDisk::fixture("e1c-worker-pool").edit(
        "elements/c.toml",
        "capacity = 3\nwhen_full = \"block\"\n\n[petri_net.transitions",
        "capacity = 3\nwhen_full = \"overflow\"\n\n[petri_net.transitions",
    );
    let run = run_checks(&disk);
    assert_eq!(
        proved(run.verdict("no-overflow")),
        (Technique::Exhaustive, 3)
    );
}

#[test]
fn the_reply_of_an_asynchronous_binding_may_run_from_the_target_to_the_source() {
    // D-07: each pair has its own direction. Billing answers over the same relationship.
    let disk = producer_consumer()
        .edit(
            "elements/api.toml",
            "to = \"billing/receive\"\ncapacity = 3\n",
            "to = \"billing/receive\"\ncapacity = 3\nwhen_full = \"block\"\n\n[relationships.answers]\nto = \"billing\"\n\n[[relationships.answers.binding]]\nmode = \"async\"\nfrom = \"billing/answer\"\nto = \"api/hear\"\ncapacity = 1\nwhen_full = \"block\"\n",
        )
        .edit(
            "elements/api.toml",
            "[relationships.orders]",
            "[petri_net.transitions.hear]\nport = true\npre = { ready = 1 }\npost = { ready = 1 }\n\n[relationships.orders]",
        )
        .edit(
            "elements/billing.toml",
            "post = { idle = 1 }\n",
            "post = { idle = 1 }\n\n[petri_net.transitions.answer]\nport = true\npre = { idle = 1 }\npost = { idle = 1 }\n",
        );
    let run = run_checks(&disk);
    assert_eq!(proved(run.verdict("no-overflow")), certificate(vec![]));
    assert_eq!(proved(run.verdict("no-deadlock")).0, Technique::Exhaustive);
}

#[test]
fn a_call_and_its_asynchronous_reply_sit_on_one_relationship() {
    // D-07, D-16: a sync call pair and an async reply pair on `orders`; the reply's generated
    // place is `orders`.
    let disk = producer_consumer()
        .edit(
            "elements/api.toml",
            "[[relationships.orders.binding]]\nmode = \"async\"\nfrom = \"api/publish\"\nto = \"billing/receive\"\ncapacity = 3\n",
            "[[relationships.orders.binding]]\nmode = \"sync\"\nfrom = \"api/publish\"\nto = \"billing/receive\"\n\n[[relationships.orders.binding]]\nmode = \"async\"\nfrom = \"billing/answer\"\nto = \"api/hear\"\ncapacity = 1\n",
        )
        .edit(
            "elements/api.toml",
            "[relationships.orders]",
            "[petri_net.transitions.hear]\nport = true\npre = { ready = 1 }\npost = { ready = 1 }\n\n[relationships.orders]",
        )
        .edit(
            "elements/billing.toml",
            "post = { idle = 1 }\n",
            "post = { idle = 1 }\n\n[petri_net.transitions.answer]\nport = true\npre = { idle = 1 }\npost = { idle = 1 }\n",
        );
    let run = run_checks(&disk);
    let Outcome::Disproved(disproved) = &run.verdict("no-overflow").outcome else {
        panic!("{:?}", run.verdict("no-overflow"))
    };
    let ViolationDto::Overflow(overflow) = &disproved.violation else {
        panic!("{:?}", disproved.violation)
    };
    assert_eq!(
        (&overflow.place, overflow.capacity, overflow.count),
        (&orders(), 1, 2)
    );
}
