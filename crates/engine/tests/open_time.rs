//! Dev-only measurement (v1-slices slice 0, ADR 0024, file-format §2.3): the open time of a
//! generated project of 2,000 elements with one element file and one layout file each, about
//! 4,000 files, against the target of under 1 s on an SSD. It prints the times and asserts only
//! what the project is, so a slow CI runner never fails it. Run it in release:
//!
//! ```text
//! cargo test --release -p tracewright-engine --test open_time -- --ignored --nocapture
//! ```

use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use tracewright_engine::Clock;
use tracewright_engine::EngineEvent;
use tracewright_engine::Ports;
use tracewright_engine::ReloadOutcome;
use tracewright_engine::Session;
use tracewright_engine::Sink;
use tracewright_engine::adapters::StdDisk;
use tracewright_engine::adapters::SystemClock;
use tracewright_engine::adapters::ThreadSpawner;

const CONTAINERS: usize = 37;
const COMPONENTS_PER_CONTAINER: usize = 53;
// A person, the system, its containers and their components.
const ELEMENTS: usize = 2 + CONTAINERS + CONTAINERS * COMPONENTS_PER_CONTAINER;
const REPEATS: usize = 5;

struct Dropped;
impl Sink for Dropped {
    fn emit(&self, _event: EngineEvent) {}
}

fn ports() -> Ports {
    Ports {
        sink: Arc::new(Dropped),
        clock: Arc::new(SystemClock) as Arc<dyn Clock>,
        spawner: Arc::new(ThreadSpawner),
    }
}

// A component with a two-place net whose `send` is bound asynchronously to the next sibling's
// `receive`: about 0.65 KB, inside the 0.3–5 KB of a typical element file (file-format §2.3).
fn component(id: &str, container: &str, next: &str) -> String {
    let mut text = format!(
        "format = 1\nkind = \"component\"\nname = \"Component {id}\"\nparent = \"{container}\"\n\
         description = \"Generated for the open-time measurement.\"\n\n\
         [petri_net.places.idle]\ncapacity = 1\nwhen_full = \"block\"\ninitial = 1\n\n\
         [petri_net.places.busy]\ncapacity = 1\nwhen_full = \"block\"\n\n\
         [petri_net.transitions.receive]\nport = true\npre = {{ idle = 1 }}\npost = {{ busy = 1 }}\n\n\
         [petri_net.transitions.send]\nport = true\npre = {{ busy = 1 }}\npost = {{ idle = 1 }}\n"
    );
    let rid = format!("{id}-to-{next}");
    write!(
        text,
        "\n[relationships.{rid}]\nto = \"{next}\"\nmeaning = \"publishes to\"\n\
         interaction = \"asynchronous\"\n\n[[relationships.{rid}.binding]]\nmode = \"async\"\n\
         from = \"{id}/send\"\nto = \"{next}/receive\"\ncapacity = 3\n"
    )
    .expect("writing to a String");
    text
}

fn layout(index: usize) -> String {
    let x = (index % 20) * 240;
    let y = (index / 20) * 160;
    format!("format = 1\n\n[position]\nx = {x}\ny = {y}\nw = 200\nh = 120\n")
}

#[expect(
    clippy::disallowed_methods,
    reason = "generates a measurement project, not a project file the app writes"
)]
fn generate(root: &Path) -> usize {
    std::fs::create_dir(root.join("elements")).expect("folder");
    std::fs::create_dir(root.join("layouts")).expect("folder");
    let mut files = vec![("arch.toml".to_owned(), "format = 1\n".to_owned())];
    files.push((
        "elements/sys.toml".to_owned(),
        "format = 1\nkind = \"software_system\"\nname = \"Generated platform\"\n".to_owned(),
    ));
    files.push((
        "elements/user.toml".to_owned(),
        "format = 1\nkind = \"person\"\nname = \"User\"\n".to_owned(),
    ));
    for c in 0..CONTAINERS {
        let container = format!("c{c:02}");
        files.push((
            format!("elements/{container}.toml"),
            format!(
                "format = 1\nkind = \"container\"\nname = \"Container {container}\"\nparent = \"sys\"\n"
            ),
        ));
        for k in 0..COMPONENTS_PER_CONTAINER {
            let id = format!("{container}-k{k:02}");
            // A ring, so every port is bound.
            let next = format!("{container}-k{:02}", (k + 1) % COMPONENTS_PER_CONTAINER);
            files.push((
                format!("elements/{id}.toml"),
                component(&id, &container, &next),
            ));
        }
    }
    let element_ids: Vec<String> = files
        .iter()
        .filter_map(|(path, _)| {
            path.strip_prefix("elements/")?
                .strip_suffix(".toml")
                .map(str::to_owned)
        })
        .collect();
    for (index, id) in element_ids.iter().enumerate() {
        files.push((format!("layouts/{id}.toml"), layout(index)));
    }
    for (path, text) in &files {
        std::fs::write(root.join(path), text).expect("measurement file");
    }
    files.len()
}

fn median(mut times: Vec<Duration>) -> Duration {
    times.sort();
    times[times.len() / 2]
}

#[test]
#[ignore = "dev-only measurement; run in release with --ignored --nocapture"]
#[expect(clippy::print_stdout, reason = "the measurement reports its times")]
fn open_time_of_a_project_with_4000_files() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let files = generate(dir.path());
    assert_eq!(
        files,
        1 + 2 * ELEMENTS,
        "arch.toml plus an element and a layout file each"
    );
    assert_eq!(ELEMENTS, 2_000);
    let disk = StdDisk::new(dir.path().to_path_buf());

    let mut open_times = Vec::new();
    let mut session = None;
    for _ in 0..REPEATS {
        let started = Instant::now();
        let opened = Session::open(&disk, ports()).expect("the generated project opens");
        let view = opened.view();
        open_times.push(started.elapsed());
        assert_eq!(view.elements.len(), CONTAINERS, "the system's containers");
        assert!(view.problems.is_empty(), "{:?}", view.problems.first());
        session = Some(opened);
    }
    let mut session = session.expect("opened at least once");

    let mut reload_times = Vec::new();
    for _ in 0..REPEATS {
        let started = Instant::now();
        let outcome = session.reload(&disk).expect("the project reloads");
        reload_times.push(started.elapsed());
        assert_eq!(outcome, ReloadOutcome::NothingChanged);
    }

    println!(
        "open time, {files} files ({ELEMENTS} elements, a layout file each), read + load + first \
         view, median of {REPEATS}: {:?} (all: {open_times:?})",
        median(open_times.clone())
    );
    println!(
        "Reload with nothing changed (read + hash every file), median of {REPEATS}: {:?}",
        median(reload_times)
    );
}
