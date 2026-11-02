//! Reading a project folder: only the files `load` reads, and none past its limit (LIM-2).

use std::fs;

use tracewright_format::ARCH_FILE_BYTES_MAX;
use tracewright_format::LoadError;

#[test]
#[expect(
    clippy::disallowed_methods,
    reason = "writes a test folder, not a project file"
)]
fn skips_dotfiles_other_files_and_other_folders() {
    let root = tempfile::tempdir().expect("temp dir");
    fs::write(root.path().join("arch.toml"), "format = 1\n").expect("write");
    fs::write(root.path().join("README.md"), "notes\n").expect("write");
    fs::create_dir_all(root.path().join("elements")).expect("mkdir");
    fs::create_dir_all(root.path().join("layouts")).expect("mkdir");
    fs::write(root.path().join("elements/api.toml"), "format = 1\n").expect("write");
    fs::write(root.path().join("elements/.pn-0a1b2c3d.tmp"), "x").expect("write");
    fs::write(root.path().join("elements/notes.txt"), "x").expect("write");
    fs::write(root.path().join("layouts/api.toml"), "format = 1\n").expect("write");
    fs::write(root.path().join("layouts/.pn-0a1b2c3d.tmp"), "x").expect("write");
    fs::create_dir_all(root.path().join("layouts/old")).expect("mkdir");
    fs::write(root.path().join("layouts/old/api.toml"), "format = 1\n").expect("write");
    let mut paths: Vec<String> = tracewright_format::read_dir(root.path())
        .expect("readable")
        .into_iter()
        .map(|file| file.path)
        .collect();
    paths.sort();
    assert_eq!(
        paths,
        ["arch.toml", "elements/api.toml", "layouts/api.toml"]
    );
}

#[test]
#[expect(
    clippy::disallowed_methods,
    reason = "writes a test folder, not a project file"
)]
fn an_oversized_file_is_read_only_one_byte_past_its_limit() {
    let root = tempfile::tempdir().expect("temp dir");
    fs::write(
        root.path().join("arch.toml"),
        vec![b'#'; ARCH_FILE_BYTES_MAX + 1000],
    )
    .expect("write");
    let files = tracewright_format::read_dir(root.path()).expect("readable");
    assert_eq!(files[0].bytes.len(), ARCH_FILE_BYTES_MAX + 1);
    assert!(
        matches!(
            tracewright_format::load(&files),
            Err(LoadError::TooLarge { .. })
        ),
        "load reports the oversized file"
    );
}
