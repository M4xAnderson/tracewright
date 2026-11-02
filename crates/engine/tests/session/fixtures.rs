//! The IPC contract test, Rust side (design §8, seam C): the events of real check
//! runs, written as JSON for the UI tests, which replay them through mockIPC. `check` fails when a
//! regenerated fixture differs from the commit, so the UI is always tested against what this
//! engine sends.

use std::path::Path;

use serde_json::Value;
use serde_json::json;

use super::support::FakeClock;
use super::support::MemDisk;
use super::support::RecordingSink;
use super::support::one_job;
use super::support::ports;
use tracewright_engine::Session;

// Opens `disk`, runs the checks and writes `events-<name>.json`. The view reaches the UI inside
// the app's `Opened`, whose fixture the app writes.
#[expect(
    clippy::disallowed_methods,
    reason = "writes a test fixture for the UI, not a project file"
)]
fn write(name: &str, disk: &MemDisk) -> (Value, Value) {
    let sink = RecordingSink::default();
    let session = Session::open(disk, ports(&sink, &FakeClock::new())).expect("the project opens");
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
            "template": null,
            "scope": { "element": "sys", "behavior": "combined" }
        })
    );
    assert_eq!(events[0]["kind"], "currentKeys");
    assert_eq!(events[0]["rev"], 0);
    let key = events[0]["keys"]["no-deadlock"]["key"]
        .as_str()
        .expect("a keyed property is { key }");
    assert!(events[0]["keys"]["no-overflow"]["key"].is_string());
    assert_eq!(
        events[1],
        json!({ "kind": "checkQueued", "property": "no-deadlock", "key": key })
    );
    assert_eq!(
        events[3],
        json!({ "kind": "checkProgress", "property": "no-deadlock", "key": key, "phase": "main", "markings": 1 })
    );
    let verdict = verdict_event(&events, "no-deadlock");
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
            "place": { "kind": "own", "element": "a", "place": "has_1" },
            "reason": { "kind": "outputFull", "capacity": 1, "holds": 1 }
        })
    );
}

#[test]
fn worker_pool() {
    let (_, events) = write("worker-pool", &MemDisk::fixture("e1c-worker-pool"));
    assert_eq!(
        verdict_event(&events, "no-deadlock")["verdict"],
        json!({ "assumptions": [], "kind": "proved", "technique": "exhaustive", "markings": 3 })
    );
    assert_eq!(
        verdict_event(&events, "no-overflow")["verdict"],
        json!({
            "assumptions": [],
            "kind": "proved",
            "technique": { "noOverflowPlace": { "dropPlaces": [] } },
            "markings": 0
        })
    );
}

#[test]
fn producer_consumer() {
    let (_, events) = write("producer-consumer", &MemDisk::fixture("producer-consumer"));
    let overflow = &verdict_event(&events, "no-overflow")["verdict"];
    assert_eq!(overflow["kind"], "disproved");
    assert_eq!(
        overflow["violation"],
        json!({
            "kind": "overflow",
            "place": { "kind": "generated", "relationship": "orders" },
            "capacity": 3,
            "count": 4
        })
    );
    assert_eq!(
        overflow["frames"][3],
        json!({
            "step": 4,
            "fired": { "kind": "own", "element": "api", "transition": "publish" }
        })
    );
    let unknown = &verdict_event(&events, "no-deadlock")["verdict"];
    assert_eq!(unknown["kind"], "unknown");
    let reason = &unknown["reason"];
    assert_eq!(reason["category"], "capacityExceededFirst");
    assert_eq!(
        reason["place"],
        json!({ "kind": "generated", "relationship": "orders" })
    );
    assert_eq!(
        (&reason["capacity"], &reason["count"]),
        (&json!(3), &json!(4))
    );
    assert_eq!(reason["frames"].as_array().map(Vec::len), Some(4));
}

// The verdict event of `property` in a recorded run.
fn verdict_event<'a>(events: &'a Value, property: &str) -> &'a Value {
    events
        .as_array()
        .expect("the events are a list")
        .iter()
        .find(|event| event["kind"] == "verdict" && event["property"] == property)
        .unwrap_or_else(|| panic!("no verdict for {property}"))
}

#[test]
fn one_finished_job() {
    let (_, events) = write("one-job", &one_job());
    assert_eq!(
        verdict_event(&events, "no-deadlock")["verdict"]["violation"]["finished"],
        true
    );
}

// Every slice 0 problem at once: the system's scope is model incomplete with one typed cause per
// problem, and Gate's own scope still gets its verdict and its slice gate lines.
#[test]
fn half_done() {
    let (view, events) = write("half-done", &MemDisk::fixture("half-done"));
    let kinds: Vec<&str> = view["problems"]
        .as_array()
        .expect("problems")
        .iter()
        .map(|p| p["kind"].as_str().expect("kind"))
        .collect();
    for kind in [
        "noWhenFull",
        "nameClash",
        "unbound",
        "noBehavior",
        "unknownPort",
        "dangling",
        "finalNotChecked",
    ] {
        assert!(kinds.contains(&kind), "{kind} in {kinds:?}");
    }
    let keys = &events[0]["keys"];
    let incomplete = &keys["no-deadlock"]["unknown"];
    assert_eq!(incomplete["category"], "modelIncomplete");
    assert_eq!(
        incomplete["details"].as_array().map(Vec::len),
        incomplete["causes"].as_array().map(Vec::len),
        "one cause per detail line"
    );
    assert_eq!(
        keys["gate-pool"]["unknown"]["details"],
        json!(["never holds the whole pool is checked from slice 1"])
    );
    assert_eq!(
        keys["gate-never-two"]["unknown"]["details"],
        json!(["user properties are checked from slice 4"])
    );
    assert_eq!(
        verdict_event(&events, "gate-no-deadlock")["verdict"]["kind"],
        "proved"
    );
}

// A file with a TOML syntax error: the combined scope waits for it, Job's own scope is checked.
#[test]
fn broken_file() {
    let (view, events) = write("broken-file", &MemDisk::fixture("broken-file"));
    assert_eq!(view["problems"][0]["kind"], "parse");
    assert_eq!(view["problems"][0]["file"], "elements/billing.toml");
    assert_eq!(view["problems"][0]["line"], 7);
    assert_eq!(
        events[0]["keys"]["no-deadlock"]["unknown"]["details"],
        json!(["elements/billing.toml did not load"])
    );
    assert_eq!(
        verdict_event(&events, "job-no-deadlock")["verdict"]["kind"],
        "proved"
    );
}

// No element has a behavior model: nothing to check, no problem, no verdict (D-13).
#[test]
fn no_behavior() {
    let (view, events) = write("no-behavior", &MemDisk::fixture("no-behavior"));
    assert_eq!(view["problems"], json!([]));
    assert_eq!(events.as_array().map(Vec::len), Some(1));
    assert_eq!(events[0]["keys"]["no-deadlock"], "nothingToCheck");
}
