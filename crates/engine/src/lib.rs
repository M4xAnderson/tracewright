//! The Session that owns the one authoritative model, lowers it to nets, runs checks on
//! supervised worker threads and talks to the world through ports (ADR 0004, 0011, 0012).
#![forbid(unsafe_code)]

pub mod check;
pub mod key;
pub mod lower;

pub use check::Verdict;
pub use key::CheckKey;
