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

pub mod balance;
pub mod body;
pub mod contact;
pub mod fight;
pub mod frame;
pub mod fx;
pub mod input;
pub mod replay;
pub mod rng;
pub mod world;

#[cfg(test)]
mod hand;

pub use body::Setup;
pub use input::Input;
pub use world::World;

/// Bumped whenever what the simulation does changes, together with the
/// golden replays, in the same commit (CLAUDE.md). A replay from another
/// version is refused with a sentence.
pub const SIM_VERSION: u32 = 6;
