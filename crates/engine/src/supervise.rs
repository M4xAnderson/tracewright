//! Supervised check workers (ADR 0006, 0012; design §4, §5). Each check runs on its own named
//! worker thread `check-<key8>` that owns an `Arc` of the lowered scope and shares no lock with the
//! app. A monitor joins it: a worker that panicked ends its check as Unknown "internal error"
//! (the panic hook has logged it), and nothing of its state survives (SAFE-4).
//!
//! The monitor runs the checks of one `run_checks` one after another, so at most one worker per
//! run explores at a time and its memory is bounded by one budget.

use std::io;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use tracewright_net::Clock;
use tracewright_net::NetProperty;
use tracewright_net::Phase;
use tracewright_net::UnknownReason;

use crate::check;
use crate::check::CheckBudget;
use crate::check::Verdict;
use crate::dto;
use crate::dto::CheckProgress;
use crate::dto::EngineEvent;
use crate::dto::VerdictEvent;
use crate::key::CheckKey;
use crate::lower::Lowered;
use crate::origin;
use crate::session::Sink;

/// Progress events of one check are at least this far apart, except the first of each phase
/// (design §3: at most 4 per second per check).
pub const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

/// Starts work on a named thread (design §3): std threads in the app, inline in tests.
pub trait Spawner: Send + Sync {
    /// Starts `work` on a thread called `name`. Threads named `check-*` unwind when they panic;
    /// the panic hook aborts on every thread outside the supervised prefixes (ADR 0012).
    ///
    /// # Errors
    ///
    /// The thread could not be started.
    fn spawn(&self, name: String, work: Box<dyn FnOnce() + Send>) -> io::Result<WorkerHandle>;
}

/// What a [`Spawner`] returns for the work it started.
#[derive(Debug)]
pub enum WorkerHandle {
    /// The work runs on its own thread; joining it tells whether it panicked.
    Thread(thread::JoinHandle<()>),
    /// The work already ran to its end on the caller's thread.
    Finished,
}

// How a worker ended.
enum Exit {
    Finished,
    Panicked,
}

impl WorkerHandle {
    fn join(self) -> Exit {
        match self {
            WorkerHandle::Thread(handle) => match handle.join() {
                Ok(()) => Exit::Finished,
                // The panic hook has logged the payload; the worker's state is discarded with
                // the thread, so nothing is recovered (ADR 0006).
                Err(_payload) => Exit::Panicked,
            },
            WorkerHandle::Finished => Exit::Finished,
        }
    }
}

/// One check to run: a property whose scope lowered, with its key.
pub(crate) struct Job {
    pub property: String,
    pub key: CheckKey,
    /// Shared by the checks of one scope and held by the worker after the Session moved on.
    pub lowered: Arc<Lowered>,
    pub net_property: NetProperty,
}

/// What the monitor and its workers share with the Session: copies of its ports and budget.
pub(crate) struct Supervisor {
    pub sink: Arc<dyn Sink>,
    pub clock: Arc<dyn Clock>,
    pub spawner: Arc<dyn Spawner>,
    pub budget: CheckBudget,
}

/// Runs `jobs` in order on a monitor thread and returns at once (or, with an inline Spawner, when
/// every verdict is emitted).
pub(crate) fn submit(supervisor: Supervisor, jobs: Vec<Job>) {
    let supervisor = Arc::new(supervisor);
    // Kept to report every job as Unknown if the monitor cannot start.
    let pending: Vec<(String, CheckKey, Arc<Lowered>)> = jobs
        .iter()
        .map(|job| {
            (
                job.property.clone(),
                job.key.clone(),
                Arc::clone(&job.lowered),
            )
        })
        .collect();
    let monitor = Arc::clone(&supervisor);
    let started = supervisor.spawner.spawn(
        "monitor-checks".to_owned(),
        Box::new(move || run_all(&monitor, jobs)),
    );
    match started {
        // A dropped JoinHandle detaches the monitor; it ends after the last verdict.
        Ok(_detached) => {}
        Err(_no_thread) => {
            for (property, key, lowered) in pending {
                supervisor
                    .sink
                    .emit(internal_error(property, key, &lowered));
            }
        }
    }
}

// Bounded by the number of jobs, one worker at a time (LIM-1).
fn run_all(supervisor: &Supervisor, jobs: Vec<Job>) {
    for job in jobs {
        let property = job.property.clone();
        let key = job.key.clone();
        let lowered = Arc::clone(&job.lowered);
        let (result, received) = mpsc::sync_channel(1);
        let worker = Worker {
            sink: Arc::clone(&supervisor.sink),
            clock: Arc::clone(&supervisor.clock),
            budget: supervisor.budget,
        };
        let name = format!("check-{}", key.key8());
        let started = supervisor.spawner.spawn(
            name,
            Box::new(move || {
                result
                    .send(worker.check(&job))
                    .expect("the monitor waits for this verdict");
            }),
        );
        let event = match started.map(WorkerHandle::join) {
            Ok(Exit::Finished) => match received.try_recv() {
                Ok(verdict) => EngineEvent::Verdict(verdict),
                Err(_no_verdict) => internal_error(property, key, &lowered),
            },
            Ok(Exit::Panicked) | Err(_) => internal_error(property, key, &lowered),
        };
        supervisor.sink.emit(event);
    }
}

fn internal_error(property: String, key: CheckKey, lowered: &Lowered) -> EngineEvent {
    EngineEvent::Verdict(VerdictEvent {
        property,
        key,
        verdict: origin::verdict_dto(&Verdict::Unknown(UnknownReason::InternalError), lowered),
    })
}

// Everything a worker owns: no lock of the app, no reference into the Session.
struct Worker {
    sink: Arc<dyn Sink>,
    clock: Arc<dyn Clock>,
    budget: CheckBudget,
}

impl Worker {
    fn check(&self, job: &Job) -> VerdictEvent {
        let mut last: Option<(Phase, Instant)> = None;
        let mut report = |phase: Phase, progress: tracewright_net::Progress| {
            let now = self.clock.now();
            let due = match last {
                None => true,
                Some((previous, at)) => {
                    previous != phase || now.saturating_duration_since(at) >= PROGRESS_INTERVAL
                }
            };
            if due {
                last = Some((phase, now));
                self.sink.emit(EngineEvent::CheckProgress(CheckProgress {
                    property: job.property.clone(),
                    key: job.key.clone(),
                    phase: dto_phase(phase),
                    markings: progress.markings,
                }));
            }
        };
        let checked = check::run(
            &job.lowered,
            &job.net_property,
            &self.budget,
            &self.clock,
            &mut report,
        );
        assert_eq!(
            checked.key, job.key,
            "check: the worker explored another net than the key names"
        );
        VerdictEvent {
            property: job.property.clone(),
            key: checked.key,
            verdict: origin::verdict_dto(&checked.verdict, &job.lowered),
        }
    }
}

fn dto_phase(phase: Phase) -> dto::Phase {
    match phase {
        Phase::Main => dto::Phase::Main,
        Phase::Recheck => dto::Phase::Recheck,
    }
}
