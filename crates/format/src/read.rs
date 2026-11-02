//! Reading a project folder from disk: the only file reads of the core crates (ARCH-1).

use std::fs;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;

use crate::ARCH_FILE_BYTES_MAX;
use crate::ELEMENT_FILE_BYTES_MAX;
use crate::ELEMENTS_MAX;
use crate::ProjectFile;

/// The files of the project folder `root` that `load` reads: `arch.toml` and `elements/*.toml`
/// (dotfiles, such as the writer's temp files, are skipped). A missing `arch.toml` or `elements`
/// folder is not an error here; `load` reports it.
///
/// LIM-2: no file is read past its limit plus one byte, so `load` reports an oversized file
/// without the whole file in memory, and at most `ELEMENTS_MAX + 1` element files are read.
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
    let elements = match fs::read_dir(root.join("elements")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(files),
        Err(error) => return Err(error),
    };
    let mut element_files = 0;
    for entry in elements {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        // Case-sensitive like `load`, which reads `elements/<id>.toml` with a lowercase ID.
        let toml = Path::new(&name).extension().is_some_and(|e| e == "toml");
        if name.starts_with('.') || !toml || !entry.file_type()?.is_file() {
            continue;
        }
        element_files += 1;
        if element_files > ELEMENTS_MAX + 1 {
            break;
        }
        files.push(ProjectFile {
            bytes: read_bounded(&entry.path(), ELEMENT_FILE_BYTES_MAX)?,
            path: format!("elements/{name}"),
        });
    }
    assert!(
        files.len() <= ELEMENTS_MAX + 2,
        "read_dir: arch.toml plus at most ELEMENTS_MAX + 1 element files"
    );
    Ok(files)
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
