//! Lowering is total or fails (NS §6.6): every failing precondition gives the scope's properties
//! Unknown "model incomplete" with one line per cause, and no worker runs.

use tracewright_engine::EngineEvent;
use tracewright_engine::KeyState;
use tracewright_engine::dto::UnknownDto;

use super::support::MemDisk;
use super::support::run_checks;

fn incomplete(disk: &MemDisk, property: &str) -> Vec<String> {
    let run = run_checks(disk);
    let KeyState::Unknown(UnknownDto::ModelIncomplete { details: lines }) = run.key_state(property)
    else {
        panic!("{property}: {:?}", run.key_state(property));
    };
    assert!(
        run.events
            .iter()
            .all(|event| !matches!(event, EngineEvent::Verdict(_))),
        "no worker runs for a scope that does not lower"
    );
    lines.clone()
}

fn pool() -> MemDisk {
    MemDisk::fixture("e1c-worker-pool")
}

#[test]
fn v28_unbound_ports_name_both_ends() {
    let disk = pool().edit(
        "elements/c.toml",
        "[[relationships.r2.binding]]\nmode = \"sync\"\nfrom = \"c/finish\"\nto = \"w/release\"\n",
        "",
    );
    assert_eq!(
        incomplete(&disk, "no-deadlock"),
        [
            "c/finish is a port but no relationship inside this scope binds it",
            "w/release is a port but no relationship inside this scope binds it",
        ]
    );
}

#[test]
fn l1_a_child_without_a_behavior_model() {
    let disk = pool().put(
        "elements/db.toml",
        "format = 1\nkind = \"database\"\nparent = \"sys\"\n",
    );
    assert_eq!(
        incomplete(&disk, "no-deadlock"),
        ["db has no behavior model"]
    );
}

#[test]
fn nothing_to_check_when_no_child_has_a_behavior_model() {
    let disk = MemDisk { files: vec![] }
        .put("arch.toml", "format = 1\n")
        .put(
            "elements/sys.toml",
            "format = 1\nkind = \"software_system\"\n\n[[property]]\nid = \"no-deadlock\"\nbehavior = \"combined\"\nkind = \"deadlock_free\"\n",
        )
        .put("elements/api.toml", "format = 1\nkind = \"container\"\nparent = \"sys\"\n");
    let run = run_checks(&disk);
    assert_eq!(run.key_state("no-deadlock"), &KeyState::NothingToCheck);
    assert_eq!(run.events.len(), 1, "no verdict");
}

#[test]
fn l2_a_place_without_when_full() {
    let disk = pool().edit(
        "elements/w.toml",
        "capacity = 2\nwhen_full = \"block\"",
        "capacity = 2",
    );
    assert_eq!(
        incomplete(&disk, "no-deadlock"),
        ["w/free has no when_full"]
    );
}

#[test]
fn l3_a_binding_to_a_port_that_does_not_exist() {
    let disk = pool().edit(
        "elements/c.toml",
        "to = \"w/acquire\"",
        "to = \"w/acquire_one\"",
    );
    let lines = incomplete(&disk, "no-deadlock");
    assert!(
        lines.contains(&"w/acquire_one not found".to_owned()),
        "{lines:?}"
    );
    assert!(
        lines.contains(
            &"w/acquire is a port but no relationship inside this scope binds it".to_owned()
        ),
        "{lines:?}"
    );
}

#[test]
fn l3_an_arc_to_a_place_that_does_not_exist() {
    let disk = pool().edit(
        "elements/w.toml",
        "post = { free = 1 }",
        "post = { fre = 1 }",
    );
    assert_eq!(incomplete(&disk, "no-deadlock"), ["w/fre not found"]);
}

#[test]
fn l5_a_binding_to_an_internal_transition() {
    let disk = pool().edit(
        "elements/w.toml",
        "[petri_net.transitions.release]\nport = true",
        "[petri_net.transitions.release]",
    );
    assert_eq!(
        incomplete(&disk, "no-deadlock"),
        ["w/release is bound but is not a port"]
    );
}

#[test]
fn l6_a_pair_inside_one_element() {
    let disk = pool().edit("elements/c.toml", "to = \"w/acquire\"", "to = \"c/finish\"");
    let lines = incomplete(&disk, "no-deadlock");
    assert!(
        lines.contains(&"c/start is bound to c/finish (same element)".to_owned()),
        "{lines:?}"
    );
}

#[test]
fn l7_a_port_that_is_caller_and_callee() {
    // w/acquire calls c/finish over a new relationship while c/start calls it over r1.
    let disk = pool().edit(
        "elements/w.toml",
        "[petri_net.transitions.release]",
        "[relationships.r9]\nto = \"c\"\n\n[[relationships.r9.binding]]\nmode = \"sync\"\nfrom = \"w/acquire\"\nto = \"c/finish\"\n\n[petri_net.transitions.release]",
    );
    let lines = incomplete(&disk, "no-deadlock");
    assert!(
        lines.contains(&"w/acquire is both caller and callee".to_owned()),
        "{lines:?}"
    );
}

#[test]
fn l10_initial_above_capacity_v29() {
    let disk = pool().edit("elements/w.toml", "initial = 2", "initial = 5");
    assert_eq!(
        incomplete(&disk, "no-deadlock"),
        ["w/free: initial 5 exceeds capacity 2"]
    );
}

#[test]
fn l10_a_name_used_by_a_place_and_a_transition() {
    let disk = pool().edit(
        "elements/w.toml",
        "[petri_net.transitions.acquire]",
        "[petri_net.transitions.free]\n\n[petri_net.transitions.acquire]",
    );
    assert_eq!(
        incomplete(&disk, "no-deadlock"),
        ["w has both a place and a transition named free"]
    );
}

#[test]
fn l11_a_pair_that_does_not_connect_the_relationships_two_elements() {
    let disk = pool()
        .put(
            "elements/x.toml",
            "format = 1\nkind = \"container\"\nparent = \"sys\"\n\n[petri_net.places.s]\ncapacity = 1\nwhen_full = \"block\"\ninitial = 1\n\n[petri_net.transitions.go]\nport = true\npre = { s = 1 }\npost = { s = 1 }\n",
        )
        .edit("elements/c.toml", "from = \"c/start\"", "from = \"x/go\"");
    let lines = incomplete(&disk, "no-deadlock");
    assert!(
        lines.contains(&"r1 (c ↔ w) binds x/go + w/acquire".to_owned()),
        "{lines:?}"
    );
}

#[test]
fn l12_two_pairs_with_the_same_fused_id() {
    let disk = pool().edit(
        "elements/c.toml",
        "[relationships.r2]",
        "[[relationships.r1.binding]]\nmode = \"sync\"\nfrom = \"c/start\"\nto = \"w/acquire\"\n\n[relationships.r2]",
    );
    let lines = incomplete(&disk, "no-deadlock");
    assert!(
        lines.contains(&"r1 binds start + acquire twice".to_owned()),
        "{lines:?}"
    );
}

#[test]
fn a_relationship_that_binds_nothing_is_an_assumption_not_a_failure() {
    // C12: the net is unchanged, so the key is too.
    let plain = run_checks(&pool());
    let disk = pool().edit(
        "elements/c.toml",
        "[relationships.r1]",
        "[relationships.r9]\nto = \"w\"\n\n[relationships.r1]",
    );
    let with_assumption = run_checks(&disk);
    assert_eq!(with_assumption.key("no-deadlock"), plain.key("no-deadlock"));
}

#[test]
fn drop_and_overflow_places_are_not_checked_yet() {
    let disk = pool().edit(
        "elements/w.toml",
        "when_full = \"block\"",
        "when_full = \"drop\"",
    );
    let run = run_checks(&disk);
    assert!(
        matches!(
            run.key_state("no-deadlock"),
            KeyState::Unknown(UnknownDto::PropertyNotCheckable { .. })
        ),
        "{:?}",
        run.key_state("no-deadlock")
    );
}
