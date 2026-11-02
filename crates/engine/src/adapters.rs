//! The ports on std (design §2): what the app plugs into a Session.

use std::io;
use std::path::PathBuf;
use std::thread;
use std::time::Instant;

use tracewright_format::ProjectFile;
use tracewright_net::Clock;

use crate::session::Disk;
use crate::supervise::Spawner;
use crate::supervise::WorkerHandle;

/// A project folder on the local file system.
#[derive(Debug, Clone)]
pub struct StdDisk {
    root: PathBuf,
}

impl StdDisk {
    /// The folder `root`; nothing is read until the Session opens it.
    #[must_use]
    pub fn new(root: PathBuf) -> StdDisk {
        StdDisk { root }
    }
}

impl Disk for StdDisk {
    fn read_tree(&self) -> io::Result<Vec<ProjectFile>> {
        tracewright_format::read_dir(&self.root)
    }
}

/// The monotonic clock of the operating system.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Starts every piece of work on a new named std thread.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThreadSpawner;

impl Spawner for ThreadSpawner {
    fn spawn(&self, name: String, work: Box<dyn FnOnce() + Send>) -> io::Result<WorkerHandle> {
        thread::Builder::new()
            .name(name)
            .spawn(work)
            .map(WorkerHandle::Thread)
    }
}
