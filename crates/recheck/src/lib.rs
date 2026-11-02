//! The second explorer. It composes open nets itself, re-explores them breadth-first,
//! replays every counterexample and checks every certificate. It depends only on `tracewright-net`,
//! never on `verify` or `engine` (ADR 0008, 0009, 0010).
#![forbid(unsafe_code)]
