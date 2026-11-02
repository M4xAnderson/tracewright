//! A model that is incomplete or partly wrong still loads (D-26, file-format §10): problems go
//! to the Problems list of the view, and checks end with the right Unknown or no verdict instead
//! of a load failure.

use tracewright_engine::EngineEvent;
use tracewright_engine::KeyState;
use tracewright_engine::dto::ProblemDto;
use tracewright_engine::dto::ProblemKind;
use tracewright_engine::dto::Severity;
use tracewright_engine::dto::UnknownDto;

use super::support::MemDisk;
use super::support::Run;
use super::support::open;
use super::support::run_checks;

fn pool() -> MemDisk {
    MemDisk::fixture("e1c-worker-pool")
}

fn problems(disk: &MemDisk) -> Vec<ProblemDto> {
    open(disk).view().problems
}

// The one problem of `kind` among `problems`.
fn only(problems: &[ProblemDto], kind: ProblemKind) -> ProblemDto {
    let found: Vec<&ProblemDto> = problems.iter().filter(|p| p.kind == kind).collect();
    assert_eq!(found.len(), 1, "one {kind:?} in {problems:#?}");
    found[0].clone()
}

#[test]
fn a_project_without_defects_has_no_problems() {
    assert_eq!(problems(&pool()), []);
}

#[test]
fn a_place_without_when_full_is_a_problem_not_a_load_failure() {
    let disk = pool().edit(
        "elements/w.toml",
        "capacity = 2\nwhen_full = \"block\"",
        "capacity = 2",
    );
    let problem = only(&problems(&disk), ProblemKind::NoWhenFull);
    assert_eq!(problem.code.as_deref(), Some("L2"));
    assert_eq!(problem.severity, Severity::Error);
    assert_eq!(problem.file.as_deref(), Some("elements/w.toml"));
    assert_eq!(problem.element.as_deref(), Some("w"));
    assert_eq!(problem.place.as_deref(), Some("w/free"));
    assert_eq!(problem.message, "w/free has no when_full");
}

// The worker pool with an own-scope property on `w`, so a test sees both scopes.
fn pool_with_own() -> MemDisk {
    pool().edit(
        "elements/w.toml",
        "parent = \"sys\"\n",
        "parent = \"sys\"\n\n[[property]]\nid = \"w-own\"\nbehavior = \"own\"\nkind = \"deadlock_free\"\n",
    )
}

fn incomplete(run: &Run, property: &str) -> (Vec<String>, Vec<ProblemDto>) {
    match run.key_state(property) {
        KeyState::Unknown(UnknownDto::ModelIncomplete { details, causes }) => {
            (details.clone(), causes.clone())
        }
        other => panic!("{property}: {other:?}"),
    }
}

fn not_checkable(run: &Run, property: &str) -> Vec<String> {
    match run.key_state(property) {
        KeyState::Unknown(UnknownDto::PropertyNotCheckable { details }) => details.clone(),
        other => panic!("{property}: {other:?}"),
    }
}

#[test]
fn a_name_clash_is_a_problem_and_blocks_both_scopes_of_its_element() {
    let disk = pool_with_own().edit(
        "elements/w.toml",
        "[petri_net.transitions.acquire]",
        "[petri_net.transitions.free]\n\n[petri_net.transitions.acquire]",
    );
    let clash = only(&problems(&disk), ProblemKind::NameClash);
    assert_eq!(
        clash.message,
        "w has both a place and a transition named free"
    );
    assert_eq!(clash.place.as_deref(), Some("w/free"));
    let run = run_checks(&disk);
    for property in ["no-deadlock", "w-own"] {
        let (details, causes) = incomplete(&run, property);
        assert_eq!(details, ["w has both a place and a transition named free"]);
        assert_eq!(causes, std::slice::from_ref(&clash));
    }
}

#[test]
fn an_arc_to_a_missing_place_is_a_dangling_reference() {
    let disk = pool().edit(
        "elements/w.toml",
        "post = { free = 1 }",
        "post = { fre = 1 }",
    );
    let dangling = only(&problems(&disk), ProblemKind::Dangling);
    assert_eq!(dangling.place.as_deref(), Some("w/fre"));
    assert_eq!(dangling.message, "w/fre not found");
}

#[test]
fn a_relationship_to_a_missing_element_is_a_dangling_reference_that_blocks_its_scope() {
    let disk = pool().edit(
        "elements/c.toml",
        "[relationships.r2]\nto = \"w\"",
        "[relationships.r2]\nto = \"audit\"",
    );
    let dangling = only(&problems(&disk), ProblemKind::Dangling);
    assert_eq!(dangling.relationship.as_deref(), Some("r2"));
    assert_eq!(dangling.element.as_deref(), Some("c"));
    assert_eq!(dangling.message, "r2: target audit not found");
    let (_, causes) = incomplete(&run_checks(&disk), "no-deadlock");
    assert!(causes.contains(&dangling), "{causes:#?}");
}

#[test]
fn a_parent_that_names_nothing_is_a_dangling_reference() {
    let disk = pool().put(
        "elements/x.toml",
        "format = 1\nkind = \"container\"\nparent = \"nowhere\"\n",
    );
    let dangling = only(&problems(&disk), ProblemKind::Dangling);
    assert_eq!(dangling.element.as_deref(), Some("x"));
    assert_eq!(dangling.message, "x: parent nowhere not found");
    assert!(
        matches!(run_checks(&disk).key_state("no-deadlock"), KeyState::Key(_)),
        "the system's scope is not touched"
    );
}

#[test]
fn a_child_without_a_behavior_model_next_to_one_with_is_model_incomplete_naming_it() {
    let disk = pool().put(
        "elements/db.toml",
        "format = 1\nkind = \"database\"\nname = \"Shipping\"\nparent = \"sys\"\n",
    );
    let missing = only(&problems(&disk), ProblemKind::NoBehavior);
    assert_eq!(missing.element.as_deref(), Some("db"));
    assert_eq!(missing.file.as_deref(), Some("elements/db.toml"));
    let (details, causes) = incomplete(&run_checks(&disk), "no-deadlock");
    assert_eq!(details, ["db has no behavior model"]);
    assert_eq!(causes, [missing]);
}

#[test]
fn with_no_child_having_a_behavior_model_there_is_nothing_to_check_and_no_problem() {
    let disk = MemDisk { files: vec![] }
        .put("arch.toml", "format = 1\n")
        .put(
            "elements/sys.toml",
            "format = 1\nkind = \"software_system\"\n\n[[property]]\nid = \"no-deadlock\"\nbehavior = \"combined\"\nkind = \"deadlock_free\"\n",
        )
        .put("elements/api.toml", "format = 1\nkind = \"container\"\nparent = \"sys\"\n")
        .put("elements/db.toml", "format = 1\nkind = \"database\"\nparent = \"sys\"\n");
    assert_eq!(
        problems(&disk),
        [],
        "PR-NOBM only when another child has a model"
    );
    let run = run_checks(&disk);
    assert_eq!(run.key_state("no-deadlock"), &KeyState::NothingToCheck);
    assert!(
        run.events
            .iter()
            .all(|e| !matches!(e, EngineEvent::Verdict(_))),
        "no verdict"
    );
}

#[test]
fn an_unbound_port_is_model_incomplete_with_the_port_but_leaves_the_own_scope_alone() {
    let disk = pool_with_own().edit(
        "elements/c.toml",
        "[[relationships.r2.binding]]\nmode = \"sync\"\nfrom = \"c/finish\"\nto = \"w/release\"\n",
        "",
    );
    let run = run_checks(&disk);
    let (details, causes) = incomplete(&run, "no-deadlock");
    assert_eq!(
        details,
        [
            "c/finish is a port but no relationship inside this scope binds it",
            "w/release is a port but no relationship inside this scope binds it",
        ]
    );
    assert!(causes.iter().all(|c| c.kind == ProblemKind::Unbound));
    assert_eq!(causes[1].port.as_deref(), Some("w/release"));
    assert_eq!(causes[1].element.as_deref(), Some("w"));
    assert!(
        matches!(run.key_state("w-own"), KeyState::Key(_)),
        "an own scope closes its ports (NS §6.6)"
    );
}

fn sys_with(entries: &str) -> MemDisk {
    pool().edit(
        "elements/sys.toml",
        "name = \"Worker pool\"\n",
        &format!("name = \"Worker pool\"\n{entries}"),
    )
}

#[test]
fn user_properties_and_templates_are_not_checked_before_their_slice() {
    let disk = sys_with(
        r#"
[[property]]
id = "never-three"
behavior = "combined"
kind = "never"
predicate = "c/running >= 3"

[[property]]
id = "back-home"
behavior = "combined"
kind = "always_reachable"
predicate = "c/idle = 3"

[[property]]
id = "all-fire"
behavior = "combined"
kind = "no_dead_transitions"

[[property]]
id = "fills"
behavior = "combined"
template = "never-fills-up"
template_version = 1
place = "w/free"

[[property]]
id = "pool"
behavior = "combined"
template = "pool-share"
template_version = 1
pool = "w/free"
held_by = ["c/running"]
"#,
    );
    let run = run_checks(&disk);
    for property in ["never-three", "back-home", "all-fire", "fills"] {
        assert_eq!(
            not_checkable(&run, property),
            ["user properties are checked from slice 4"],
            "{property}"
        );
    }
    assert_eq!(
        not_checkable(&run, "pool"),
        ["never holds the whole pool is checked from slice 1"]
    );
    assert!(matches!(run.key_state("no-deadlock"), KeyState::Key(_)));
    assert_eq!(problems(&disk), [], "gated properties are not problems");
}

#[test]
fn a_property_entry_with_an_unknown_key_is_kept_and_not_checked() {
    let disk = sys_with(
        "\n[[property]]\nid = \"never-three\"\nbehavior = \"combined\"\nkind = \"never\"\npredicate = \"c/running >= 3\"\nseverity = \"high\"\n",
    );
    let unknown = only(&problems(&disk), ProblemKind::Property);
    assert_eq!(unknown.code.as_deref(), Some("E407"));
    assert_eq!(unknown.property.as_deref(), Some("never-three"));
    // PL S-07: the coded line first, then the slice gate line.
    assert_eq!(
        not_checkable(&run_checks(&disk), "never-three"),
        [
            "Unknown key severity in property never-three. It is kept in the file but the property is not checked.",
            "user properties are checked from slice 4",
        ]
    );
}

#[test]
fn a_final_table_is_a_warning_and_deadlock_free_lowers_without_it() {
    let plain = run_checks(&pool());
    let disk = pool().edit(
        "elements/sys.toml",
        "name = \"Worker pool\"\n",
        "name = \"Worker pool\"\n\n[final]\ncombined = \"c/idle = 3\"\n",
    );
    let warning = only(&problems(&disk), ProblemKind::FinalNotChecked);
    assert_eq!(warning.severity, Severity::Warning);
    assert_eq!(warning.file.as_deref(), Some("elements/sys.toml"));
    let run = run_checks(&disk);
    assert_eq!(
        run.key("no-deadlock"),
        plain.key("no-deadlock"),
        "D-25: deadlock-free lowers with final none"
    );
    assert_eq!(run.verdict("no-deadlock"), plain.verdict("no-deadlock"));
}

#[test]
fn a_file_that_does_not_parse_opens_the_project_and_blocks_every_combined_scope() {
    let disk = pool_with_own().put(
        "elements/x.toml",
        "format = 1\nkind = \"container\"\nparent = \"sys\"\n[petri_net.places.p\ncapacity = 1\n",
    );
    let parse = only(&problems(&disk), ProblemKind::Parse);
    assert_eq!(parse.file.as_deref(), Some("elements/x.toml"));
    assert_eq!(parse.line, Some(4));
    let run = run_checks(&disk);
    let (details, causes) = incomplete(&run, "no-deadlock");
    assert_eq!(details, ["elements/x.toml did not load"]);
    assert_eq!(causes, [parse]);
    assert!(
        matches!(run.key_state("w-own"), KeyState::Key(_)),
        "own scopes of other elements stay checkable"
    );
    // Ports are closed in an own scope: acquire fires while free > 0, release while free < 2.
    assert!(matches!(
        run.verdict("w-own").outcome,
        tracewright_engine::dto::Outcome::Proved(_)
    ));
}

#[test]
fn a_file_that_fails_at_the_element_level_blocks_its_parents_combined_scope() {
    let disk = pool().edit(
        "elements/w.toml",
        "name = \"Worker pool\"",
        "nme = \"Worker pool\"",
    );
    let schema = only(&problems(&disk), ProblemKind::Schema);
    assert_eq!(schema.file.as_deref(), Some("elements/w.toml"));
    let (details, _) = incomplete(&run_checks(&disk), "no-deadlock");
    assert_eq!(details, ["elements/w.toml did not load"]);
}

#[test]
fn a_behavior_section_that_does_not_load_keeps_the_element() {
    let disk = pool_with_own().edit("elements/w.toml", "initial = 2", "initial = -2");
    let unloaded = only(&problems(&disk), ProblemKind::BehaviorNotLoaded);
    assert_eq!(unloaded.element.as_deref(), Some("w"));
    assert!(unloaded.line.is_some(), "{unloaded:?}");
    let run = run_checks(&disk);
    for property in ["no-deadlock", "w-own"] {
        let (details, _) = incomplete(&run, property);
        assert_eq!(details, ["w has a behavior model that did not load"]);
    }
}

// design §8: every `validate` problem makes lowering fail for the scopes it touches. Each case is
// one defect in the worker pool; `no-deadlock` is the system's combined scope, `w-own` the own
// scope of w (file-format §10.5: an element blocks its own scope and its parent's combined scope,
// a relationship the combined scope of its ends' parent, and L1 and L4 only the combined scope).
#[test]
fn every_problem_makes_lowering_fail_for_the_scopes_it_touches() {
    const BOTH: &[&str] = &["no-deadlock", "w-own"];
    const COMBINED: &[&str] = &["no-deadlock"];
    let r1_pair = "from = \"c/start\"\nto = \"w/acquire\"\n";
    let r2_pair = "from = \"c/finish\"\nto = \"w/release\"\n";
    let async_pair = |capacity: &str| {
        format!(
            "{r2_pair}\n[[relationships.r2.binding]]\nmode = \"async\"\nfrom = \"c/finish\"\nto = \"w/release\"\n{capacity}"
        )
    };
    let x = "format = 1\nkind = \"container\"\nparent = \"sys\"\n\n[petri_net.places.s]\ncapacity = 1\nwhen_full = \"block\"\ninitial = 1\n\n[petri_net.transitions.go]\nport = true\npre = { s = 1 }\npost = { s = 1 }\n";
    let base = pool_with_own;
    let cases: Vec<(&str, MemDisk, &[&str])> = vec![
        ("noWhenFull", base().edit("elements/w.toml", "when_full = \"block\"\n", ""), BOTH),
        ("noCapacity", base().edit("elements/w.toml", "capacity = 2\n", ""), BOTH),
        ("badInitial", base().edit("elements/w.toml", "initial = 2", "initial = 5"), BOTH),
        (
            "nameClash",
            base().edit("elements/w.toml", "[petri_net.transitions.acquire]", "[petri_net.transitions.free]\n\n[petri_net.transitions.acquire]"),
            BOTH,
        ),
        ("dangling arc", base().edit("elements/w.toml", "post = { free = 1 }", "post = { fre = 1 }"), BOTH),
        ("behaviorNotLoaded", base().edit("elements/w.toml", "initial = 2", "initial = \"two\""), BOTH),
        ("parentCycle", base().edit("elements/sys.toml", "kind = \"software_system\"\n", "kind = \"software_system\"\nparent = \"w\"\n"), BOTH),
        ("noBehavior", base().put("elements/db.toml", "format = 1\nkind = \"database\"\nparent = \"sys\"\n"), COMBINED),
        ("unbound", base().edit("elements/c.toml", &format!("[[relationships.r2.binding]]\nmode = \"sync\"\n{r2_pair}"), ""), COMBINED),
        ("unknownPort", base().edit("elements/c.toml", "to = \"w/acquire\"", "to = \"w/acquire_one\""), COMBINED),
        ("notAPort", base().edit("elements/w.toml", "[petri_net.transitions.release]\nport = true", "[petri_net.transitions.release]"), COMBINED),
        ("bindingSelf", base().edit("elements/c.toml", "to = \"w/acquire\"", "to = \"c/finish\""), COMBINED),
        ("bindingDirection", base().put("elements/x.toml", x).edit("elements/c.toml", "from = \"c/start\"", "from = \"x/go\""), COMBINED),
        ("bindingDuplicate", base().edit("elements/c.toml", r1_pair, &format!("{r1_pair}\n[[relationships.r1.binding]]\nmode = \"sync\"\n{r1_pair}")), COMBINED),
        ("badBinding", base().edit("elements/c.toml", r1_pair, &format!("{r1_pair}capacity = 1\n")), COMBINED),
        ("bindingAsync", base().edit("elements/c.toml", r2_pair, &format!("{}\n[[relationships.r2.binding]]\nmode = \"async\"\nfrom = \"w/release\"\nto = \"c/finish\"\ncapacity = 1\n", async_pair("capacity = 1\n"))), COMBINED),
        ("noCapacity async", base().edit("elements/c.toml", r2_pair, &async_pair("")), COMBINED),
        ("dangling target", base().edit("elements/c.toml", "[relationships.r2]\nto = \"w\"", "[relationships.r2]\nto = \"audit\""), COMBINED),
        ("duplicateId", base().edit("elements/w.toml", "[petri_net.places.free]", "[relationships.r1]\nto = \"c\"\n\n[petri_net.places.free]"), COMBINED),
        ("parse", base().put("elements/x.toml", "format = 1\n[petri_net"), COMBINED),
    ];
    for (case, disk, blocked) in cases {
        let view = open(&disk).view();
        let errors: Vec<&ProblemDto> = view
            .problems
            .iter()
            .filter(|p| p.severity == Severity::Error)
            .collect();
        assert!(!errors.is_empty(), "{case}: the defect is a problem");
        let run = run_checks(&disk);
        let mut found = Vec::new();
        for property in ["no-deadlock", "w-own"] {
            if blocked.contains(&property) {
                found.extend(incomplete(&run, property).1);
            } else {
                assert!(
                    !matches!(
                        run.key_state(property),
                        KeyState::Unknown(UnknownDto::ModelIncomplete { .. })
                    ),
                    "{case}: {property} is not touched: {:?}",
                    run.key_state(property)
                );
            }
        }
        for problem in errors {
            assert!(
                found.contains(problem),
                "{case}: {problem:?} is a cause of a scope it touches; causes {found:#?}"
            );
        }
    }
}
