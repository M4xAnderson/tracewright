//! The vocabulary both explorers speak: nets, markings, properties, evidence and verdicts.
//!
//! Data only (ADR 0009): constructors, structural validation in `validate` and the canonical
//! encoding in `encode`. No firing rule, no predicate evaluation, no composition; CI fails on any
//! other `impl` block.
#![forbid(unsafe_code)]
