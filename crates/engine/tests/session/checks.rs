//! Deadlock-free through the Rust core: the E1 projects, read, lowered and checked, with the
//! evidence re-checked by recheck before the Session emits a verdict (ADR 0008, 0010).

use tracewright_engine::EngineEvent;
use tracewright_engine::KeyState;
use tracewright_engine::Verdict;
use tracewright_net::Evidence;
use tracewright_net::Technique;
use tracewright_net::UnknownReason;
use tracewright_net::Violation;

use super::support::MemDisk;
use super::support::run_checks;

fn fired(verdict: &Verdict) -> Vec<String> {
    let Verdict::Disproved(violation) = verdict else {
        panic!("not Disproved: {verdict:?}")
    };
    let Evidence::Counterexample(cx) = violation.evidence();
    cx.steps.iter().map(|step| step.fired.0.clone()).collect()
}

#[test]
fn opposite_locks_are_disproved_with_the_two_step_counterexample_of_v13() {
    let run = run_checks(&MemDisk::fixture("e1b-opposite-locks"));
    let verdict = run.verdict("no-deadlock");
    assert_eq!(fired(verdict), ["ra1/take_x+acq_x", "rb1/take_y+acq_y"]);
    let Verdict::Disproved(violation) = verdict else {
        unreachable!()
    };
    let Evidence::Counterexample(cx) = violation.evidence();
    let Violation::Deadlock { waiting } = &cx.violation;
    let waiting_transitions: Vec<&str> = waiting.iter().map(|w| w.transition.0.as_str()).collect();
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
    let Verdict::Proved(proof) = run.verdict("no-deadlock") else {
        panic!("{:?}", run.verdict("no-deadlock"))
    };
    assert_eq!(*proof.technique(), Technique::Exhaustive);
    assert_eq!(proof.markings(), 3);
}

#[test]
fn a_worker_pool_whose_failure_path_also_releases_is_proved() {
    // e1c2: w/release is the callee of two relationships, so it fuses twice.
    let run = run_checks(&MemDisk::fixture("e1c2-worker-pool-fail-releases"));
    let Verdict::Proved(proof) = run.verdict("no-deadlock") else {
        panic!("{:?}", run.verdict("no-deadlock"))
    };
    assert_eq!(proof.markings(), 3);
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
    let Verdict::Proved(proof) = run.verdict("x-no-deadlock") else {
        panic!("{:?}", run.verdict("x-no-deadlock"))
    };
    assert_eq!(proof.markings(), 1);
}

#[test]
fn keys_come_first_and_each_verdict_carries_its_current_key() {
    let run = run_checks(&MemDisk::fixture("e1b-opposite-locks"));
    assert!(
        matches!(run.events[0], EngineEvent::CurrentKeys { .. }),
        "currentKeys is the first event"
    );
    let EngineEvent::Verdict { key, .. } = &run.events[1] else {
        panic!("{:?}", run.events[1])
    };
    assert_eq!(key, run.key("no-deadlock"));
    assert_eq!(
        run.events.len(),
        2,
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
            KeyState::Unknown(UnknownReason::PropertyNotCheckable(_))
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
