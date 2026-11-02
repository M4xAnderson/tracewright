//! Deadlock-free through the Rust core: the E1 projects, read, lowered and checked, with the
//! evidence re-checked by recheck before the Session emits a verdict (ADR 0008, 0010).

use tracewright_engine::EngineEvent;
use tracewright_engine::KeyState;
use tracewright_engine::dto::Outcome;
use tracewright_engine::dto::Technique;
use tracewright_engine::dto::TransitionOrigin;
use tracewright_engine::dto::UnknownDto;
use tracewright_engine::dto::VerdictDto;
use tracewright_engine::dto::ViolationDto;

use super::support::MemDisk;
use super::support::run_checks;

// The qualified ID of the flattened net (NS §6.1) that the origin names.
fn qualified(origin: &TransitionOrigin) -> String {
    match origin {
        TransitionOrigin::Own(t) => format!("{}/{}", t.element, t.transition),
        TransitionOrigin::Sync {
            relationship,
            caller,
            callee,
        } => format!("{relationship}/{}+{}", caller.transition, callee.transition),
    }
}

fn fired(verdict: &VerdictDto) -> Vec<String> {
    let Outcome::Disproved(disproved) = &verdict.outcome else {
        panic!("not Disproved: {verdict:?}")
    };
    disproved
        .frames
        .iter()
        .map(|f| qualified(&f.fired))
        .collect()
}

fn proved(verdict: &VerdictDto) -> (Technique, u64) {
    let Outcome::Proved(proof) = &verdict.outcome else {
        panic!("not Proved: {verdict:?}")
    };
    (proof.technique.clone(), proof.markings)
}

#[test]
fn opposite_locks_are_disproved_with_the_two_step_counterexample_of_v13() {
    let run = run_checks(&MemDisk::fixture("e1b-opposite-locks"));
    let verdict = run.verdict("no-deadlock");
    assert_eq!(fired(verdict), ["ra1/take_x+acq_x", "rb1/take_y+acq_y"]);
    let Outcome::Disproved(disproved) = &verdict.outcome else {
        unreachable!()
    };
    let ViolationDto::Deadlock(deadlock) = &disproved.violation else {
        panic!("not a deadlock: {:?}", disproved.violation)
    };
    let waiting_transitions: Vec<String> = deadlock
        .waiting
        .iter()
        .map(|w| qualified(&w.transition))
        .collect();
    assert_eq!(
        waiting_transitions,
        [
            "ra1/take_x+acq_x",
            "ra2/take_y+acq_y",
            "ra3/release+rel_xy",
            "rb1/take_y+acq_y",
            "rb2/take_x+acq_x",
            "rb3/release+rel_xy"
        ]
    );
}

#[test]
fn the_worker_pool_is_proved_by_exhaustive_exploration_of_its_three_markings() {
    let run = run_checks(&MemDisk::fixture("e1c-worker-pool"));
    assert_eq!(
        proved(run.verdict("no-deadlock")),
        (Technique::Exhaustive, 3)
    );
}

#[test]
fn a_worker_pool_whose_failure_path_also_releases_is_proved() {
    // e1c2: w/release is the callee of two relationships, so it fuses twice.
    let run = run_checks(&MemDisk::fixture("e1c2-worker-pool-fail-releases"));
    assert_eq!(proved(run.verdict("no-deadlock")).1, 3);
}

#[test]
fn a_worker_pool_that_leaks_on_failure_is_disproved_as_v24() {
    let run = run_checks(&MemDisk::fixture("e1c3-worker-pool-leak"));
    assert_eq!(
        fired(run.verdict("no-deadlock")),
        ["r1/start+acquire", "c/fail", "r1/start+acquire", "c/fail"]
    );
}

#[test]
fn an_own_scope_closes_its_ports_and_proves_v08() {
    let run = run_checks(&MemDisk::fixture("ns-v08"));
    assert_eq!(proved(run.verdict("x-no-deadlock")).1, 1);
}

#[test]
fn keys_come_first_and_each_verdict_carries_its_current_key() {
    let run = run_checks(&MemDisk::fixture("e1b-opposite-locks"));
    assert!(
        matches!(run.events[0], EngineEvent::CurrentKeys(_)),
        "currentKeys is the first event"
    );
    let EngineEvent::Verdict(verdict) = &run.events[2] else {
        panic!("{:?}", run.events[2])
    };
    assert_eq!(&verdict.key, run.key("no-deadlock"));
    assert_eq!(
        run.events.len(),
        3,
        "one verdict: only no-deadlock is checked so far"
    );
}

#[test]
fn no_capacity_exceeded_is_not_checked_yet() {
    // It arrives with the asynchronous-bindings and certificate tickets; until then the Session
    // says so instead of checking a property it cannot decide.
    let run = run_checks(&MemDisk::fixture("e1b-opposite-locks"));
    assert!(
        matches!(
            run.key_state("no-overflow"),
            KeyState::Unknown(UnknownDto::PropertyNotCheckable { .. })
        ),
        "{:?}",
        run.key_state("no-overflow")
    );
}

#[test]
fn a_finished_workload_counts_as_a_deadlock_while_final_none_is_lowered() {
    // D-23, D-25: `[final]` is kept but slice 0 lowers `final none`, so a pool whose clients all
    // stop is a deadlock.
    let disk = MemDisk::fixture("e1c-worker-pool")
        .edit("elements/c.toml", "post = { idle = 1 }", "post = { done = 1 }")
        .edit(
            "elements/c.toml",
            "[petri_net.transitions.start]",
            "[petri_net.places.done]\ncapacity = 3\nwhen_full = \"block\"\n\n[petri_net.transitions.start]",
        )
        .edit("elements/sys.toml", "name = \"Worker pool\"", "name = \"Worker pool\"\n\n[final]\ncombined = \"c/done = 3\"");
    let run = run_checks(&disk);
    assert_eq!(
        fired(run.verdict("no-deadlock")).len(),
        6,
        "start, finish three times"
    );
}
