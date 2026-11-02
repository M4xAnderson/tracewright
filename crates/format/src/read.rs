//! Reading a project folder from disk: the only file reads of the core crates (ARCH-1).

use std::fs;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;

use crate::ARCH_FILE_BYTES_MAX;
use crate::ELEMENT_FILE_BYTES_MAX;
use crate::ELEMENTS_MAX;
use crate::LAYOUT_FILE_BYTES_MAX;
use crate::ProjectFile;

/// The files of the project folder `root` that `load` reads: `arch.toml`, `elements/*.toml` and
/// `layouts/*.toml` (dotfiles, such as the writer's temp files, are skipped). A missing
/// `arch.toml`, `elements` or `layouts` folder is not an error here; `load` reports what it needs.
///
/// LIM-2: no file is read past its limit plus one byte, so `load` reports an oversized file
/// without the whole file in memory, and at most `ELEMENTS_MAX + 1` files are read per folder.
/// Layout files past that are not read: each element has at most one, so the extra files are
/// orphans or leave an element unplaced, and layout never changes meaning (file-format §3.3).
///
/// # Errors
///
/// The IO error that stopped the read, for example a missing or unreadable `root`.
///
/// # Panics
///
/// Only on a bug: more files read than the limits allow.
pub fn read_dir(root: &Path) -> io::Result<Vec<ProjectFile>> {
    // Fails for a missing or unreadable root before anything else is read.
    fs::read_dir(root)?;
    let mut files = Vec::new();
    match read_bounded(&root.join("arch.toml"), ARCH_FILE_BYTES_MAX) {
        Ok(bytes) => files.push(ProjectFile {
            path: "arch.toml".to_owned(),
            bytes,
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    read_folder(root, "elements", ELEMENT_FILE_BYTES_MAX, &mut files)?;
    read_folder(root, "layouts", LAYOUT_FILE_BYTES_MAX, &mut files)?;
    assert!(
        files.len() <= 2 * (ELEMENTS_MAX + 1) + 1,
        "read_dir: arch.toml plus at most ELEMENTS_MAX + 1 files per folder"
    );
    Ok(files)
}

// The `*.toml` files directly in `root/folder`, at most `ELEMENTS_MAX + 1` of them.
fn read_folder(
    root: &Path,
    folder: &str,
    max_bytes: usize,
    files: &mut Vec<ProjectFile>,
) -> io::Result<()> {
    let entries = match fs::read_dir(root.join(folder)) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let mut read = 0;
    for entry in entries {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        // Case-sensitive like `load`, which reads `<folder>/<id>.toml` with a lowercase ID.
        let toml = Path::new(&name).extension().is_some_and(|e| e == "toml");
        if name.starts_with('.') || !toml || !entry.file_type()?.is_file() {
            continue;
        }
        read += 1;
        if read > ELEMENTS_MAX + 1 {
            break;
        }
        files.push(ProjectFile {
            bytes: read_bounded(&entry.path(), max_bytes)?,
            path: format!("{folder}/{name}"),
        });
    }
    Ok(())
}

fn read_bounded(path: &Path, max_bytes: usize) -> io::Result<Vec<u8>> {
    let limit = u64::try_from(max_bytes).expect("file limits fit in u64") + 1;
    let mut bytes = Vec::new();
    File::open(path)?.take(limit).read_to_end(&mut bytes)?;
    assert!(
        bytes.len() <= max_bytes + 1,
        "read_bounded: take() stops at the limit plus one byte"
    );
    Ok(bytes)
}
