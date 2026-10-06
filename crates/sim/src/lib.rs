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
/// 7: a weapon may have more points and edges than a straight sword (Sam,
/// 2026-10-04); a plain sword plays exactly as in 6.
/// 8: a third seat on the opponents' side, and ledges a fighter stands on
/// (Sam, 2026-10-05). A replay carries three inputs a tick. A duel on open
/// ground plays exactly as in 7.
/// 9: a sword can be thrown (bit 10, `Input::THROW`); it cuts in flight and
/// nothing once it has touched the ground. Ledges hold feet only.
/// 10: a round with every sword thrown and down for DISARMED_DRAW_TICKS is a
/// draw (`Cause::Disarmed`).
/// 11: the third seat is balanced like the others, and a fighter's height
/// for "knocked over" is measured from what its feet stand on.
/// 12: a setup carries an objective (story mode's hold-out), and the world
/// counts the ticks of the round under way. A match with no objective plays
/// exactly as in 11.
/// 13: a seat may set its side and facing, the arena's width is in the
/// setup, and a stage to cross is won by reaching its end. A match that sets
/// none of them plays exactly as in 12.
/// 14: a returning weapon (the boomerang) comes back to the hand, and a
/// blade that meets it in flight turns it on its thrower; a result says
/// whether a thrown blade made the deciding cut. A match with no boomerang
/// in it plays exactly as in 13.
/// 15: a body may hold a second weapon in a second pair of arms, with their
/// own motor keys, and a player's four arms turn the second pair with the
/// elbow keys; a throw lets go of every weapon in hand.
pub const SIM_VERSION: u32 = 15;
