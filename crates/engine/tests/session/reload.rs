//! Reload (flows §4.5, file-format §9.3): edits made in a text editor reach the Session. Slice 0
//! has no edits in the app yet, so every changed file is adopted from disk.

use tracewright_engine::EngineEvent;
use tracewright_engine::KeyState;
use tracewright_engine::OpenError;
use tracewright_engine::ReloadOutcome;
use tracewright_engine::dto::Reloaded;
use tracewright_format::LoadError;

use super::support::FakeClock;
use super::support::RecordingSink;
use super::support::Run;
use super::support::one_job;
use super::support::ports;
use tracewright_engine::Session;

#[test]
fn reload_with_no_file_changed_on_disk_changes_nothing() {
    let disk = one_job();
    let sink = RecordingSink::default();
    let mut session =
        Session::open(&disk, ports(&sink, &FakeClock::new())).expect("the project opens");

    let outcome = session.reload(&disk).expect("the project reloads");

    assert_eq!(outcome, ReloadOutcome::NothingChanged);
    assert_eq!(session.view().rev, 0, "nothing changed, so no new revision");
    assert!(sink.events.lock().expect("test sink").is_empty());
}

#[test]
fn reload_adopts_a_net_edited_on_disk_and_derives_its_keys_without_checking() {
    let disk = one_job();
    let sink = RecordingSink::default();
    let mut session =
        Session::open(&disk, ports(&sink, &FakeClock::new())).expect("the project opens");
    session.run_checks();
    let before = Run {
        events: sink.events.lock().expect("test sink").drain(..).collect(),
    };
    let edited = disk.edit("elements/job.toml", "initial = 1", "initial = 0");

    let outcome = session.reload(&edited).expect("the project reloads");

    assert_eq!(
        outcome,
        ReloadOutcome::Reloaded(Reloaded {
            changed: vec!["job".to_owned()],
            added: vec![],
            removed: vec![],
            not_loaded: vec![],
        })
    );
    assert_eq!(
        session.view().rev,
        1,
        "a reload that changed files is a new revision"
    );
    let after = Run {
        events: sink.events.lock().expect("test sink").clone(),
    };
    assert_eq!(after.current_keys().rev, 1);
    assert_ne!(
        after.key("no-deadlock"),
        before.key("no-deadlock"),
        "the net changed, so its key did"
    );
    assert!(
        after
            .events
            .iter()
            .all(|event| matches!(event, EngineEvent::CurrentKeys(_))),
        "Reload derives the keys; Check is one key away (flows §4.5): {:?}",
        after.events
    );
}

#[test]
fn reload_twice_finds_nothing_new_the_second_time() {
    let disk = one_job();
    let mut session = super::support::open(&disk);
    let edited = disk.edit("elements/job.toml", "initial = 1", "initial = 0");
    session.reload(&edited).expect("the project reloads");

    assert_eq!(
        session.reload(&edited).expect("the project reloads"),
        ReloadOutcome::NothingChanged
    );
    assert_eq!(session.view().rev, 1);
}

#[test]
fn a_changed_file_that_does_not_parse_is_a_problem_and_blocks_its_scope() {
    let disk = one_job();
    let sink = RecordingSink::default();
    let mut session =
        Session::open(&disk, ports(&sink, &FakeClock::new())).expect("the project opens");
    let broken = disk.edit("elements/job.toml", "initial = 1", "initial = ");

    let ReloadOutcome::Reloaded(reloaded) = session.reload(&broken).expect("the project reloads")
    else {
        panic!("a changed file is reloaded");
    };

    assert_eq!(reloaded.changed, Vec::<String>::new());
    let [problem] = reloaded.not_loaded.as_slice() else {
        panic!("one file did not load: {:?}", reloaded.not_loaded);
    };
    assert_eq!(problem.file.as_deref(), Some("elements/job.toml"));
    assert_eq!(problem.line, Some(11), "the line of `initial = `");
    assert!(
        session.view().problems.contains(problem),
        "the Problems list shows it"
    );
    let keys = Run {
        events: sink.events.lock().expect("test sink").clone(),
    };
    assert!(
        matches!(keys.key_state("no-deadlock"), KeyState::Unknown(_)),
        "the system's scope is model incomplete while job.toml does not load: {:?}",
        keys.key_state("no-deadlock")
    );
}

#[test]
fn an_element_file_added_or_removed_on_disk_is_reported_by_id() {
    let disk = one_job();
    let mut session = super::support::open(&disk);
    let audit = "format = 1\nkind = \"container\"\nname = \"Audit\"\nparent = \"sys\"\n";
    let mut changed = disk.put("elements/audit.toml", audit);
    changed
        .files
        .retain(|file| file.path != "elements/job.toml");

    let outcome = session.reload(&changed).expect("the project reloads");

    assert_eq!(
        outcome,
        ReloadOutcome::Reloaded(Reloaded {
            changed: vec![],
            added: vec!["audit".to_owned()],
            removed: vec!["job".to_owned()],
            not_loaded: vec![],
        })
    );
    let names: Vec<String> = session
        .view()
        .elements
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert_eq!(names, ["Audit"]);
}

#[test]
fn a_project_that_no_longer_opens_keeps_the_model_it_had_until_it_is_fixed() {
    let disk = one_job();
    let mut session = super::support::open(&disk);
    let newer = disk.clone().edit("arch.toml", "format = 1", "format = 2");

    let failed = session.reload(&newer);

    assert!(
        matches!(
            failed,
            Err(OpenError::Load(LoadError::VersionTooNew { found: 2, .. }))
        ),
        "{failed:?}"
    );
    assert_eq!(session.view().rev, 0, "the Session keeps its model");
    assert_eq!(
        session.reload(&disk).expect("the project reloads"),
        ReloadOutcome::NothingChanged,
        "what the Session saw is still the files it loaded"
    );
}

#[test]
fn a_layout_change_reloads_without_changing_any_key_so_verdicts_stay_current() {
    let disk = one_job();
    let sink = RecordingSink::default();
    let mut session =
        Session::open(&disk, ports(&sink, &FakeClock::new())).expect("the project opens");
    session.run_checks();
    let before = Run {
        events: sink.events.lock().expect("test sink").drain(..).collect(),
    };
    let moved = disk.put(
        "layouts/job.toml",
        "format = 1\n\n[position]\nx = 40\ny = 80\n",
    );

    let outcome = session.reload(&moved).expect("the project reloads");

    assert_eq!(
        outcome,
        ReloadOutcome::Reloaded(Reloaded {
            changed: vec![],
            added: vec![],
            removed: vec![],
            not_loaded: vec![],
        })
    );
    let after = Run {
        events: sink.events.lock().expect("test sink").clone(),
    };
    assert_eq!(after.current_keys().rev, 1);
    assert_eq!(after.key("no-deadlock"), before.key("no-deadlock"));
}
