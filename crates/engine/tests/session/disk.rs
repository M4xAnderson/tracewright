//! `StdDisk`, the Disk port on a real folder: it reads what the loader needs and nothing else.

use std::path::Path;
use std::path::PathBuf;

use tracewright_engine::Disk;
use tracewright_engine::OpenError;
use tracewright_engine::Session;
use tracewright_engine::adapters::StdDisk;
use tracewright_format::LoadError;

use super::support::FakeClock;
use super::support::MemDisk;
use super::support::RecordingSink;
use super::support::ports;

fn projects() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/projects")
}

#[test]
fn reads_arch_toml_and_the_element_files_of_a_project_folder() {
    let mut files = StdDisk::new(projects().join("e1b-opposite-locks"))
        .read_tree()
        .expect("the fixture folder is readable");
    files.sort_by(|a, b| a.path.cmp(&b.path));
    assert_eq!(files, MemDisk::fixture("e1b-opposite-locks").files);
}

#[test]
fn a_folder_without_arch_toml_is_not_a_project() {
    let opened = Session::open(
        &StdDisk::new(projects()),
        ports(&RecordingSink::default(), &FakeClock::new()),
    );
    assert!(
        matches!(opened, Err(OpenError::Load(LoadError::NotAProject))),
        "{opened:?}"
    );
}

#[test]
fn a_missing_folder_is_a_read_error() {
    let opened = Session::open(
        &StdDisk::new(projects().join("no-such-project")),
        ports(&RecordingSink::default(), &FakeClock::new()),
    );
    assert!(matches!(opened, Err(OpenError::Read(_))), "{opened:?}");
}
