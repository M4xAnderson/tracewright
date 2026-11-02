//! What the UI reads: the view of an opened project and the events of a check run, in element
//! terms (flows §4.7, §4.8; design §3). Names stay out of the events; the UI resolves IDs through
//! the view.

use tracewright_engine::EngineEvent;
use tracewright_engine::dto::Behavior;
use tracewright_engine::dto::DeadlockDto;
use tracewright_engine::dto::Outcome;
use tracewright_engine::dto::PlaceRef;
use tracewright_engine::dto::PropertyKind;
use tracewright_engine::dto::Technique;
use tracewright_engine::dto::TransitionOrigin;
use tracewright_engine::dto::TransitionRef;
use tracewright_engine::dto::ViolationDto;
use tracewright_engine::dto::WaitReason;
use tracewright_engine::dto::WaitingDto;

use super::support::MemDisk;
use super::support::one_job;
use super::support::open;
use super::support::run_checks;

fn own(element: &str, transition: &str) -> TransitionRef {
    TransitionRef {
        element: element.to_owned(),
        transition: transition.to_owned(),
    }
}

#[expect(
    clippy::similar_names,
    reason = "caller and callee are the terms of NS §6.3"
)]
fn sync(relationship: &str, caller: (&str, &str), callee: (&str, &str)) -> TransitionOrigin {
    TransitionOrigin::Sync {
        relationship: relationship.to_owned(),
        caller: own(caller.0, caller.1),
        callee: own(callee.0, callee.1),
    }
}

fn needs(element: &str, place: &str, needed: u32, holds: u32) -> tracewright_engine::dto::WaitDto {
    tracewright_engine::dto::WaitDto {
        place: PlaceRef {
            element: element.to_owned(),
            place: place.to_owned(),
        },
        reason: WaitReason::NeedsTokens { needed, holds },
    }
}

fn deadlock(outcome: &Outcome) -> &DeadlockDto {
    let Outcome::Disproved(disproved) = outcome else {
        panic!("not Disproved: {outcome:?}")
    };
    let ViolationDto::Deadlock(deadlock) = &disproved.violation;
    deadlock
}

#[test]
fn the_view_names_the_system_its_children_their_relationships_and_the_properties() {
    let session = open(&MemDisk::fixture("e1c-worker-pool"));
    let view = session.view();
    assert_eq!(view.rev, 0);
    let system = view
        .system
        .as_ref()
        .expect("the project has a software system");
    assert_eq!(
        (system.id.as_str(), system.name.as_str()),
        ("sys", "Worker pool")
    );
    let elements: Vec<(&str, &str)> = view
        .elements
        .iter()
        .map(|e| (e.id.as_str(), e.name.as_str()))
        .collect();
    assert_eq!(elements, [("c", "Clients"), ("w", "Worker pool")]);
    let relationships: Vec<(&str, &str, &str, Option<&str>)> = view
        .relationships
        .iter()
        .map(|r| {
            (
                r.id.as_str(),
                r.source.as_str(),
                r.target.as_str(),
                r.meaning.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        relationships,
        [
            ("r1", "c", "w", Some("takes a worker from")),
            ("r2", "c", "w", Some("gives the worker back to")),
        ]
    );
    let properties: Vec<(&str, PropertyKind, &str, Behavior)> = view
        .properties
        .iter()
        .map(|p| {
            (
                p.id.as_str(),
                p.kind,
                p.scope.element.as_str(),
                p.scope.behavior,
            )
        })
        .collect();
    assert_eq!(
        properties,
        [
            (
                "no-deadlock",
                PropertyKind::DeadlockFree,
                "sys",
                Behavior::Combined
            ),
            (
                "no-overflow",
                PropertyKind::NoCapacityExceeded,
                "sys",
                Behavior::Combined
            ),
        ]
    );
}

#[test]
fn own_scope_properties_of_children_are_in_the_view() {
    let view = open(&MemDisk::fixture("ns-v08")).view();
    let properties: Vec<(&str, &str, Behavior)> = view
        .properties
        .iter()
        .map(|p| (p.id.as_str(), p.scope.element.as_str(), p.scope.behavior))
        .collect();
    assert_eq!(
        properties,
        [
            ("x-no-deadlock", "x", Behavior::Own),
            ("x-no-overflow", "x", Behavior::Own),
        ]
    );
}

#[test]
fn current_keys_carry_the_rev_of_the_view_they_belong_to() {
    let run = run_checks(&MemDisk::fixture("e1c-worker-pool"));
    assert_eq!(run.current_keys().rev, 0);
}

#[test]
fn each_keyed_property_is_queued_before_its_verdict_arrives() {
    let run = run_checks(&MemDisk::fixture("e1b-opposite-locks"));
    let kinds: Vec<&str> = run
        .events
        .iter()
        .map(|event| match event {
            EngineEvent::CurrentKeys(_) => "currentKeys",
            EngineEvent::CheckQueued(_) => "checkQueued",
            EngineEvent::Verdict(_) => "verdict",
        })
        .collect();
    assert_eq!(kinds, ["currentKeys", "checkQueued", "verdict"]);
    let EngineEvent::CheckQueued(queued) = &run.events[1] else {
        unreachable!()
    };
    assert_eq!(queued.property, "no-deadlock");
    assert_eq!(&queued.key, run.key("no-deadlock"));
}

#[test]
fn the_opposite_locks_counterexample_names_each_step_by_relationship_and_element() {
    let run = run_checks(&MemDisk::fixture("e1b-opposite-locks"));
    let Outcome::Disproved(disproved) = &run.verdict("no-deadlock").outcome else {
        panic!("{:?}", run.verdict("no-deadlock"))
    };
    let fired: Vec<(u32, &TransitionOrigin)> = disproved
        .frames
        .iter()
        .map(|frame| (frame.step, &frame.fired))
        .collect();
    assert_eq!(
        fired,
        [
            (1, &sync("ra1", ("a", "take_x"), ("l", "acq_x"))),
            (2, &sync("rb1", ("b", "take_y"), ("l", "acq_y"))),
        ]
    );
}

#[test]
fn the_opposite_locks_deadlock_lists_one_waiting_line_per_waiting_step() {
    let run = run_checks(&MemDisk::fixture("e1b-opposite-locks"));
    let deadlock = deadlock(&run.verdict("no-deadlock").outcome);
    assert_eq!(deadlock.waiting.len(), 6);
    assert_eq!(
        deadlock.waiting[0],
        WaitingDto {
            transition: sync("ra1", ("a", "take_x"), ("l", "acq_x")),
            waits: vec![
                tracewright_engine::dto::WaitDto {
                    place: PlaceRef {
                        element: "a".to_owned(),
                        place: "has_1".to_owned(),
                    },
                    reason: WaitReason::OutputFull {
                        capacity: 1,
                        holds: 1,
                    },
                },
                needs("a", "idle", 1, 0),
                needs("l", "x", 1, 0),
            ],
        }
    );
    assert!(
        !deadlock.finished,
        "a and b each hold a lock, so tokens sit in pre sets"
    );
}

#[test]
fn a_run_that_ends_with_no_token_in_any_pre_set_is_finished() {
    // D-23: the caveat V-DIS-NOFINAL applies only to this case.
    let run = run_checks(&one_job());
    let deadlock = deadlock(&run.verdict("no-deadlock").outcome);
    assert!(deadlock.finished, "the job is done");
}

#[test]
fn a_run_stuck_with_tokens_in_a_pre_set_is_not_finished() {
    // `done` is already full, so `work` cannot fire although todo holds its token.
    let run = run_checks(&one_job().edit(
        "elements/job.toml",
        "[petri_net.places.done]\ncapacity = 1\nwhen_full = \"block\"\n",
        "[petri_net.places.done]\ncapacity = 1\nwhen_full = \"block\"\ninitial = 1\n",
    ));
    let deadlock = deadlock(&run.verdict("no-deadlock").outcome);
    assert!(!deadlock.finished, "todo still holds its token");
}

#[test]
fn the_worker_pool_is_proved_with_its_technique_and_reachable_markings() {
    let run = run_checks(&MemDisk::fixture("e1c-worker-pool"));
    let verdict = run.verdict("no-deadlock");
    let Outcome::Proved(proved) = &verdict.outcome else {
        panic!("{verdict:?}")
    };
    assert_eq!(proved.technique, Technique::Exhaustive);
    assert_eq!(proved.markings, 3);
    assert!(
        verdict.assumptions.is_empty(),
        "both relationships bind ports"
    );
}

#[test]
fn a_relationship_that_binds_no_ports_is_listed_as_an_assumption() {
    let disk = MemDisk::fixture("e1c-worker-pool").edit(
        "elements/c.toml",
        "[relationships.r2]",
        "[relationships.r0]\nto = \"w\"\nmeaning = \"logs to\"\n\n[relationships.r2]",
    );
    let run = run_checks(&disk);
    assert_eq!(run.verdict("no-deadlock").assumptions, ["r0"]);
}
