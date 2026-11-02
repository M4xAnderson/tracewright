//! Seam A: `engine::Session` with in-memory ports. Given project files, when the checks run,
//! then the events and verdicts it emits.

mod budget;
mod capacity;
mod changed_on_disk;
mod check_key;
mod checks;
mod disk;
mod events;
mod fixtures;
mod lowering;
mod pipeline;
mod problems;
mod reload;
mod support;
