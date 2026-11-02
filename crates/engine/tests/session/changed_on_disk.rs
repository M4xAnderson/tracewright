//! Save never overwrites a file changed on disk (FMT-2, file-format §9.2, UX CNV-5). Save itself
//! arrives with its ticket; these tests drive the check it runs before each write.

use tracewright_engine::WriteCheck;

use super::support::one_job;
use super::support::open;

#[test]
fn save_skips_a_file_changed_on_disk_and_names_it() {
    let disk = one_job();
    let session = open(&disk);
    let edited = disk.edit("elements/job.toml", "initial = 1", "initial = 2");

    let check = session
        .check_writes(&edited, &["elements/job.toml", "elements/sys.toml"])
        .expect("the disk is readable");

    assert_eq!(
        check,
        WriteCheck {
            writable: vec!["elements/sys.toml".to_owned()],
            changed_on_disk: vec!["elements/job.toml".to_owned()],
        }
    );
}

#[test]
fn a_file_deleted_on_disk_or_created_there_since_the_read_is_changed() {
    let disk = one_job();
    let session = open(&disk);
    let mut changed = disk.put("layouts/job.toml", "format = 1\n");
    changed
        .files
        .retain(|file| file.path != "elements/sys.toml");

    let check = session
        .check_writes(
            &changed,
            &["elements/sys.toml", "layouts/job.toml", "layouts/sys.toml"],
        )
        .expect("the disk is readable");

    assert_eq!(
        check,
        WriteCheck {
            writable: vec!["layouts/sys.toml".to_owned()],
            changed_on_disk: vec![
                "elements/sys.toml".to_owned(),
                "layouts/job.toml".to_owned()
            ],
        },
        "a new file Save would create is writable only while it does not exist"
    );
}

#[test]
fn after_reload_the_file_on_disk_is_the_one_save_may_replace() {
    let disk = one_job();
    let mut session = open(&disk);
    let edited = disk.edit("elements/job.toml", "initial = 1", "initial = 0");
    session.reload(&edited).expect("the project reloads");

    let check = session
        .check_writes(&edited, &["elements/job.toml"])
        .expect("the disk is readable");

    assert_eq!(check.writable, ["elements/job.toml"]);
    assert_eq!(check.changed_on_disk, Vec::<String>::new());
}
