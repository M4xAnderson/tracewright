//! The Session that owns the one authoritative model, lowers it to nets, runs checks on
//! supervised worker threads and talks to the world through ports (ADR 0004, 0011, 0012).
#![forbid(unsafe_code)]

pub mod adapters;
pub mod check;
mod disk_sync;
pub mod dto;
pub mod key;
pub mod lower;
mod origin;
mod problems;
mod session;
mod supervise;

pub use check::CheckBudget;
pub use check::DEFAULT_MAX_MARKINGS;
pub use check::DEFAULT_TIME;
pub use check::Verdict;
pub use disk_sync::FileHash;
pub use dto::EngineEvent;
pub use dto::KeyState;
pub use dto::ReloadOutcome;
pub use key::CheckKey;
pub use session::Disk;
pub use session::OpenError;
pub use session::Ports;
pub use session::Session;
pub use session::Sink;
pub use session::WriteCheck;
pub use supervise::Spawner;
pub use supervise::WorkerHandle;
pub use tracewright_net::Clock;
