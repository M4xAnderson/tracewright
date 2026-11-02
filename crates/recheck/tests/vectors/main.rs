//! recheck on its own (seam B): `compose` against the NS §14.3 vectors, `explore` against the
//! NS §14.1 slice 0 vectors and the §14.2 budget vectors, `replay` (NS §9.3) and
//! `check_certificate` (NS §9.5). One test crate, so the vector reader in `support` is shared by
//! all of them without dead code.

mod budget;
mod certificate;
mod compose;
mod explore;
mod replay;
mod support;
