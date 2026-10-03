//! The fight.
//!
//! Everything a round contains — the bodies, the swords, the cuts and the ink —
//! is integer arithmetic in this crate, and the world changes only through
//! `World::step([Input; 2])`. No floats, no clock, no `HashMap`, one random
//! stream, and two dependencies (`serde`, `postcard`). `tests/boundary.rs`
//! holds all of that, because the way these rules get broken is not a
//! decision to break them.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
