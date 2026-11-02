//! The Session that owns the one authoritative model, lowers it to nets, runs checks on
//! supervised worker threads and talks to the world through ports (ADR 0004, 0011, 0012).
#![forbid(unsafe_code)]

pub mod adapters;
pub mod check;
pub mod dto;
pub mod key;
pub mod lower;
mod origin;
mod problems;
mod session;

pub use check::Verdict;
pub use dto::EngineEvent;
pub use dto::KeyState;
pub use key::CheckKey;
pub use session::DEFAULT_MAX_MARKINGS;
pub use session::Disk;
pub use session::OpenError;
pub use session::Session;
pub use session::Sink;
