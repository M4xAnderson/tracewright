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

// A copy of `one-job` in a temporary folder, plus a layout file and files the project ignores.
#[expect(
    clippy::disallowed_methods,
    reason = "builds a test folder, not a project file the app writes"
)]
fn one_job_folder() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temporary folder");
    for file in MemDisk::fixture("one-job").files {
        let path = dir.path().join(&file.path);
        std::fs::create_dir_all(path.parent().expect("a file in a folder")).expect("folder");
        std::fs::write(path, file.bytes).expect("test file");
    }
    std::fs::create_dir(dir.path().join("layouts")).expect("folder");
    std::fs::write(dir.path().join("layouts/job.toml"), "format = 1\n").expect("test file");
    std::fs::write(dir.path().join("layouts/.pn-0000abcd.tmp"), "x").expect("test file");
    std::fs::write(dir.path().join("README.md"), "# notes\n").expect("test file");
    dir
}

#[test]
fn reads_the_layout_files_too_so_save_knows_what_it_last_saw() {
    let dir = one_job_folder();

    let mut paths: Vec<String> = StdDisk::new(dir.path().to_path_buf())
        .read_tree()
        .expect("the folder is readable")
        .into_iter()
        .map(|file| file.path)
        .collect();
    paths.sort();

    assert_eq!(
        paths,
        [
            "arch.toml",
            "elements/job.toml",
            "elements/sys.toml",
            "layouts/job.toml"
        ]
    );
}

#[test]
fn reads_one_project_file_or_none_when_it_does_not_exist() {
    let dir = one_job_folder();
    let disk = StdDisk::new(dir.path().to_path_buf());

    assert_eq!(
        disk.read_file("layouts/job.toml").expect("readable"),
        Some(b"format = 1\n".to_vec())
    );
    assert_eq!(disk.read_file("layouts/sys.toml").expect("readable"), None);
}

#[test]
fn reads_no_file_outside_the_project_files() {
    let dir = one_job_folder();
    let disk = StdDisk::new(dir.path().to_path_buf());

    for path in [
        "README.md",
        "../arch.toml",
        "elements/../arch.toml",
        "layouts/.pn-0000abcd.tmp",
    ] {
        let read = disk.read_file(path);
        assert!(
            read.as_ref()
                .is_err_and(|error| error.kind() == std::io::ErrorKind::InvalidInput),
            "{path}: {read:?}"
        );
    }
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
