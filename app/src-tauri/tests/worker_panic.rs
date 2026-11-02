//! ADR 0006, 0012 and spike E3 on a real thread: a check whose worker panics ends as Unknown
//! "internal error", the panic is in the local log, the Session keeps working and the next check
//! succeeds. Its own test binary, because the panic hook it installs aborts on a panic on any
//! thread that is not a supervised worker, this test's own thread included.

use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;

use tracewright_engine::EngineEvent;
use tracewright_engine::Ports;
use tracewright_engine::Session;
use tracewright_engine::Sink;
use tracewright_engine::Spawner;
use tracewright_engine::WorkerHandle;
use tracewright_engine::adapters::StdDisk;
use tracewright_engine::adapters::SystemClock;
use tracewright_engine::adapters::ThreadSpawner;
use tracewright_engine::dto::Outcome;
use tracewright_engine::dto::UnknownDto;
use tracewright_engine::dto::VerdictDto;

// Std threads, except that the first check worker panics instead of checking.
#[derive(Debug, Default)]
struct FirstWorkerPanics {
    panicked: AtomicBool,
}

impl Spawner for FirstWorkerPanics {
    fn spawn(&self, name: String, work: Box<dyn FnOnce() + Send>) -> io::Result<WorkerHandle> {
        if name.starts_with("check-") && !self.panicked.swap(true, Ordering::SeqCst) {
            return ThreadSpawner.spawn(name, Box::new(|| panic!("injected worker panic")));
        }
        ThreadSpawner.spawn(name, work)
    }
}

// Hands each verdict to the test thread.
struct Verdicts(Mutex<mpsc::Sender<VerdictDto>>);

impl Sink for Verdicts {
    fn emit(&self, event: EngineEvent) {
        if let EngineEvent::Verdict(verdict) = event {
            self.0
                .lock()
                .expect("one sender")
                .send(verdict.verdict)
                .expect("the test waits for verdicts");
        }
    }
}

#[test]
fn a_worker_panic_is_unknown_internal_error_and_the_next_check_succeeds() {
    let logs = tempfile::tempdir().expect("temp dir");
    let log_file = tracewright_app::open_log_in(logs.path()).expect("the log opens");
    tracewright_app::log_panics();

    let (sender, verdicts) = mpsc::channel();
    let ports = Ports {
        sink: Arc::new(Verdicts(Mutex::new(sender))),
        clock: Arc::new(SystemClock),
        spawner: Arc::new(FirstWorkerPanics::default()),
    };
    let project = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/engine/tests/projects/e1c-worker-pool");
    let session = Session::open(&StdDisk::new(project), ports).expect("the project opens");
    // A bound for a hung worker, not a timing assumption: the verdicts arrive in milliseconds.
    let next = || {
        verdicts
            .recv_timeout(Duration::from_secs(60))
            .expect("a verdict arrives")
    };

    session.run_checks();
    let first = next();
    assert!(
        matches!(
            &first.outcome,
            Outcome::Unknown(unknown) if unknown.reason == UnknownDto::InternalError
        ),
        "{first:?}"
    );
    let log = fs::read_to_string(&log_file).expect("the log is readable");
    let panic_line = log
        .lines()
        .find(|line| line.contains(" PANIC [check-"))
        .unwrap_or_else(|| panic!("no PANIC line from a check worker in:\n{log}"));
    assert!(panic_line.contains("injected worker panic"), "{panic_line}");
    assert!(
        !log.contains(" FATAL "),
        "a worker panic must not abort:\n{log}"
    );

    session.run_checks();
    let second = next();
    let Outcome::Proved(proved) = &second.outcome else {
        panic!("the next check succeeds: {second:?}");
    };
    assert_eq!(proved.markings, 3);
}
