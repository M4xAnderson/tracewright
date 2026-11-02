# Assertions stay on in release; a panicking check becomes Unknown

Assertions in core crates stay enabled in release builds, because a wrong Proved is worse than a crash. Each check runs on its own named worker thread that owns an immutable snapshot of the model and shares no locks with the app; if the thread panics, joining it returns an error and the check ends as Unknown with the reason "internal error", the panic is logged locally and the user sees that it is a bug. This follows the Google Rust style guide's ban on `catch_unwind` in first-party code, because no state is recovered: the worker's state is discarded, as a process supervisor would. Release builds therefore keep `panic = "unwind"`. A panic on the main thread (while editing) still ends the app; the recovery journal (ADR 0001) limits the loss to the last gesture.

Amended by ADR 0012.
