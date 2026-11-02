//! What the Session last saw on disk (design §3 `DiskSync`, file-format §9): the content hash of
//! every project file it read, so Reload finds the files changed in an editor and Save never
//! overwrites one of them.

use std::collections::BTreeMap;
use std::fmt;

use tracewright_format::ProjectFile;

/// The BLAKE3 hash of a file's raw bytes (file-format §10.1 step 2).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FileHash([u8; 32]);

impl FileHash {
    /// The hash of `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> FileHash {
        FileHash(*blake3::hash(bytes).as_bytes())
    }
}

/// `b3:<64 lowercase hex>`, as file-format §10.1 writes it.
impl fmt::Display for FileHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("b3:")?;
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for FileHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// The hash of every project file as the Session last read it, by path.
#[derive(Debug, Clone, Default)]
pub(crate) struct DiskSync {
    last_seen: BTreeMap<String, FileHash>,
}

/// How the files on disk differ from what the Session last read, each list in path order.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Changes {
    /// Files that exist in both, with other bytes.
    pub(crate) modified: Vec<String>,
    /// Files that are new on disk.
    pub(crate) added: Vec<String>,
    /// Files that are gone from disk.
    pub(crate) removed: Vec<String>,
}

impl Changes {
    pub(crate) fn is_empty(&self) -> bool {
        self.modified.is_empty() && self.added.is_empty() && self.removed.is_empty()
    }
}

impl DiskSync {
    /// What the Session saw in `files`.
    pub(crate) fn seen(files: &[ProjectFile]) -> DiskSync {
        let last_seen: BTreeMap<String, FileHash> = files
            .iter()
            .map(|file| (file.path.clone(), FileHash::of(&file.bytes)))
            .collect();
        assert_eq!(
            last_seen.len(),
            files.len(),
            "DiskSync::seen: a disk lists each path once"
        );
        DiskSync { last_seen }
    }

    /// How `files`, read from disk now, differ from what was last seen.
    pub(crate) fn changes(&self, files: &[ProjectFile]) -> Changes {
        let mut changes = Changes::default();
        let mut on_disk = std::collections::BTreeSet::new();
        for file in files {
            on_disk.insert(file.path.as_str());
            match self.last_seen.get(&file.path) {
                None => changes.added.push(file.path.clone()),
                Some(seen) => {
                    if *seen != FileHash::of(&file.bytes) {
                        changes.modified.push(file.path.clone());
                    }
                }
            }
        }
        changes.removed = self
            .last_seen
            .keys()
            .filter(|path| !on_disk.contains(path.as_str()))
            .cloned()
            .collect();
        changes.modified.sort();
        changes.added.sort();
        changes
    }
}
