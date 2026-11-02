//! The ports on std (design §2): what the app plugs into a Session.

use std::io;
use std::path::PathBuf;

use tracewright_format::ProjectFile;

use crate::session::Disk;

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
