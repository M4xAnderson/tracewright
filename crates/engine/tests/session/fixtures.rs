//! The IPC contract test, Rust side (design §8, seam C): the events of real check
//! runs, written as JSON for the UI tests, which replay them through mockIPC. `check` fails when a
//! regenerated fixture differs from the commit, so the UI is always tested against what this
//! engine sends.

use std::path::Path;

use serde_json::Value;
use serde_json::json;

use super::support::MemDisk;
use super::support::RecordingSink;
use super::support::one_job;
use tracewright_engine::Session;

// Opens `disk`, runs the checks and writes `events-<name>.json`. The view reaches the UI inside
// the app's `Opened`, whose fixture the app writes.
#[expect(
    clippy::disallowed_methods,
    reason = "writes a test fixture for the UI, not a project file"
)]
fn write(name: &str, disk: &MemDisk) -> (Value, Value) {
    let sink = RecordingSink::default();
    let session = Session::open(disk, Box::new(sink.clone())).expect("the project opens");
    let view = serde_json::to_value(session.view()).expect("the view serializes");
    session.run_checks();
    let events = serde_json::to_value(&*sink.events.lock().expect("test fixture"))
        .expect("the events serialize");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/ui/src/ipc/fixtures");
    let text = serde_json::to_string_pretty(&events).expect("JSON");
    std::fs::write(dir.join(format!("events-{name}.json")), format!("{text}\n"))
        .expect("fixture written");
    (view, events)
}

#[test]
fn opposite_locks() {
    let (view, events) = write("opposite-locks", &MemDisk::fixture("e1b-opposite-locks"));
    assert_eq!(view["rev"], 0);
    assert_eq!(
        view["system"],
        json!({ "id": "sys", "name": "Two components, two locks" })
    );
    assert_eq!(
        view["properties"][0],
        json!({
            "id": "no-deadlock",
            "label": null,
            "kind": "deadlockFree",
            "scope": { "element": "sys", "behavior": "combined" }
        })
    );
    assert_eq!(events[0]["kind"], "currentKeys");
    assert_eq!(events[0]["rev"], 0);
    let key = events[0]["keys"]["no-deadlock"]["key"]
        .as_str()
        .expect("a keyed property is { key }");
    assert_eq!(
        events[0]["keys"]["no-overflow"]["unknown"]["category"],
        "propertyNotCheckable"
    );
    assert_eq!(
        events[1],
        json!({ "kind": "checkQueued", "property": "no-deadlock", "key": key })
    );
    let verdict = &events[2];
    assert_eq!(verdict["kind"], "verdict");
    assert_eq!(verdict["key"], key);
    assert_eq!(verdict["verdict"]["kind"], "disproved");
    assert_eq!(verdict["verdict"]["assumptions"], json!([]));
    assert_eq!(
        verdict["verdict"]["frames"][0],
        json!({
            "step": 1,
            "fired": {
                "kind": "sync",
                "relationship": "ra1",
                "caller": { "element": "a", "transition": "take_x" },
                "callee": { "element": "l", "transition": "acq_x" }
            }
        })
    );
    let violation = &verdict["verdict"]["violation"];
    assert_eq!(violation["kind"], "deadlock");
    assert_eq!(violation["finished"], false);
    assert_eq!(
        violation["waiting"][0]["waits"][0],
        json!({
            "place": { "element": "a", "place": "has_1" },
            "reason": { "kind": "outputFull", "capacity": 1, "wouldHold": 2 }
        })
    );
}

#[test]
fn worker_pool() {
    let (_, events) = write("worker-pool", &MemDisk::fixture("e1c-worker-pool"));
    assert_eq!(
        events[2]["verdict"],
        json!({ "assumptions": [], "kind": "proved", "technique": "exhaustive", "markings": 3 })
    );
}

#[test]
fn one_finished_job() {
    let (_, events) = write("one-job", &one_job("done"));
    assert_eq!(events[2]["verdict"]["violation"]["finished"], true);
}
