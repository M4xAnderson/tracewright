# A project is a directory of TOML files, saved explicitly

A project is stored as a directory of human-readable TOML files: one file per element (with its behavior model, ports and properties inline), layout in separate files per view, and stable slug IDs with a collision suffix, so git diffs and merges stay readable and a layout change never touches the meaning of the model. Saving is explicit: the user sees unsaved changes, and Save writes each changed file atomically (temp file + rename). Because a gesture can touch several files and a set of renames is not atomic, every gesture is also appended to a crash-recovery journal in the app data folder, and the loader reports dangling references instead of failing.

## Considered Options

- **One model file:** simple to save atomically, but every edit rewrites one large file and concurrent branches conflict constantly.
- **An existing DSL (Structurizr, LikeC4) as the native format:** no place for behavior models, ports, properties or verdict-relevant data; we would fork the language anyway. They stay export targets.
- **Autosave every gesture into the model files:** the git working tree would be dirty all the time and multi-file gestures would not be atomic.

## Consequences

The format is data that outlives a release: it carries a version, and every format change ships with a migration and a test that old projects still load.
