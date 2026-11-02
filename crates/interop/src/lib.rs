//! Development-only exchange formats (ADR 0009): the PNML writer for `net::Net` and the TAPAAL
//! verifypn queries of the differential corpus (NS §18 item 10). Never a dependency of `engine`
//! or the app (`tools/arch/crate-edges.ts`); the corpus itself is `tests/tapaal_corpus.rs`, run
//! by `tools/check.sh tapaal`.
#![forbid(unsafe_code)]

pub mod pnml;
pub mod tapaal;
