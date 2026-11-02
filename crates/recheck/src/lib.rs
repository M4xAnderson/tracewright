//! The second explorer. It composes open nets itself, re-explores them breadth-first,
//! replays every counterexample and checks every certificate. It depends only on `tracewright-net`,
//! never on `verify` or `engine` (ADR 0008, 0009, 0010).
//!
//! Everything here is written from `docs/specs/net-semantics.md` (§2.3, §3.2, §4, §6.3–§6.5, §7,
//! §8.1, §8.2, §9.3, §9.5, §10.1, §12), never from `verify` or `engine`. It is deliberately plain:
//! dense pre and post matrices, the enabling and firing formulas of NS §4 term by term, and a
//! breadth-first search with no reductions.
#![forbid(unsafe_code)]

mod certificate;
mod compose;
mod explore;
mod replay;
mod semantics;

pub use certificate::check_certificate;
pub use compose::ComposeError;
pub use compose::compose;
pub use explore::explore;
pub use replay::ReplayError;
pub use replay::replay;
