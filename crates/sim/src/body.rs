//! What a body is made of, and the `Setup` a world starts from.
//!
//! `crates/content` builds these from `data/body.json`; `sim` never reads a
//! file. A replay carries the whole `Setup`, so a replay does not change when
//! `data/` does (D15); only `SIM_VERSION` can make one unplayable.

use crate::balance::{self, Tuning};
use crate::fx::{Fx, V2};
use serde::{Deserialize, Serialize};

/// Why a round ended for the fighter it ended for.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Cause {
    /// A cut across the neck or the head.
    Neck,
    /// A cut inside the heart band of the chest.
    Heart,
    /// The fighter ran out of ink.
    Ink,
}

/// A stretch of a part's spine, as fractions from its near end, where a cut
/// ends the round (D12).
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct FatalBand {
    pub from: Fx,
    pub to: Fx,
    pub cause: Cause,
}

/// Which joint a held key turns. A motor sits on the near end of its part.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Motor {
    Shoulder,
    Elbow,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PointDef {
    /// Where the point sits in the rest pose, in cm, facing +x, with the
    /// feet on the ground at `y = 0` and `x = 0` between them.
    pub at: V2,
    /// How far the point keeps from the ground and the walls.
    pub rad: Fx,
}

/// A body part: a capsule along a stick from `near` (toward the heart) to
/// `far`. The body is a tree; a cut drops everything beyond it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PartDef {
    /// The `parts.<id>` copy key the result names it by.
    pub copy: String,
    pub near: u8,
    pub far: u8,
    pub radius: Fx,
    /// Integer mass. A point's mass is the sum of the parts that touch it.
    pub mass: i32,
    /// Ink drained per tick by a stump on this part (D12).
    pub drain: i32,
    pub fatal: Vec<FatalBand>,
    pub motor: Option<Motor>,
    /// A hand. A hand holding its own fighter's sword is safe from that
    /// blade (D11).
    pub hand: bool,
}

/// A joint that bends one way only, between `max_bend` degrees and straight.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Hinge {
    pub a: u8,
    pub j: u8,
    pub c: u8,
    /// +1 if bending turns `j→c` counterclockwise from `a→j` when facing +x.
    pub sign: i8,
    /// The closest `a` and `c` may come, from the law of cosines at the
    /// hinge's largest bend; computed by `content` with the sine table.
    pub min_dist: Fx,
}

/// The particles the balance rule and the motors need by role.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Roles {
    pub shoulder: Option<u8>,
    pub pelvis: Option<u8>,
    pub head: Option<u8>,
    pub feet: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SwordDef {
    /// The pommel end and the tip, in the rest pose.
    pub butt: V2,
    pub tip: V2,
    /// The hilt runs this far from the butt and does not cut (D10).
    pub hilt: Fx,
    /// Mass of each of the sword's two particles.
    pub mass: i32,
    /// The hand points that hold it, and where along the sword (as a
    /// fraction from the butt) each one holds. The lead grip also fixes the
    /// wrist: `stiff` is the point whose distance to the tip is held (D7).
    pub grips: Vec<Grip>,
    /// More points of a weapon that is not one straight blade (a curved
    /// blade's belly, a fork's prongs), in the rest pose. Each is held to
    /// the butt and the tip by sticks, so the weapon keeps its shape.
    pub extra: Vec<V2>,
    /// The edges that cut and meet other blades, between the weapon's
    /// points: 0 is the butt, 1 the tip, 2 on the extra points. Each cuts
    /// from `from` (a fraction from `a`) to `b`. Empty means one edge, butt
    /// to tip, cutting from the end of the hilt: a plain sword.
    pub edges: Vec<BladeEdge>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BladeEdge {
    pub a: u8,
    pub b: u8,
    pub from: Fx,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Grip {
    pub hand: u8,
    pub at: Fx,
    pub stiff: Option<u8>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BodyDef {
    pub points: Vec<PointDef>,
    pub parts: Vec<PartDef>,
    /// Extra sticks that hold a shape, between two points of the body
    /// (straight wrist, straight neck, standing legs). Lengths come from the
    /// rest pose.
    pub sticks: Vec<(u8, u8)>,
    pub hinges: Vec<Hinge>,
    pub roles: Roles,
    /// Points pinned to where they start (a post's foot). Mass zero.
    pub anchored: Vec<u8>,
    /// Whether the balance rule holds this body up (a fighter) or not (a post).
    pub balance: bool,
    pub ink: i32,
    pub sword: Option<SwordDef>,
}

/// Everything about the physics that a test may need to change. A real
/// match takes `Physics::tuned`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Physics {
    pub gravity: Fx,
    pub ground: bool,
    pub walls: bool,
    pub tuning: Tuning,
}

impl Physics {
    pub fn tuned(i: u8) -> Physics {
        Physics {
            gravity: balance::GRAVITY,
            ground: true,
            walls: true,
            tuning: balance::TUNINGS[(i as usize).min(balance::TUNINGS.len() - 1)],
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Mode {
    /// Rounds and a match (D13).
    Match,
    /// The practice yard: no round ever ends.
    Practice,
}

/// One seat: which body stands in it.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Seat {
    pub body: u8,
    /// Where this seat's feet start; mirrored for the right seat.
    pub x: Fx,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Setup {
    pub seed: u64,
    pub mode: Mode,
    pub rounds_to_win: u32,
    pub physics: Physics,
    pub bodies: Vec<BodyDef>,
    /// Seat 0 faces +x on the left; seat 1 faces -x on the right.
    pub seats: [Option<Seat>; 2],
}
