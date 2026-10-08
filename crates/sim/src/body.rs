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
    /// The other side held out for the whole of a hold-out round.
    HeldOut,
    /// The other side reached the end of a stage to cross.
    Reached,
    /// Nobody had a blade: every fighter in the round had thrown its sword
    /// (or lost it), none was in the air, and nobody went out for
    /// `balance::DISARMED_DRAW_TICKS`. Only ever a draw.
    Disarmed,
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
    /// The second pair of arms' (a four-armed body).
    Shoulder2,
    Elbow2,
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
    /// Thrown, it comes back to the hand after `balance::RETURN_TICKS`
    /// (the boomerang).
    #[serde(default)]
    pub returns: bool,
    /// The cursed blade: it cuts as a sword does, and is drawn with two
    /// strands twisting along it (Sam, 2026-10-07). Only the drawing reads it.
    #[serde(default)]
    pub cursed: bool,
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
    /// The weapons in the other pairs of hands: one on a four-armed body,
    /// three on the eight-armed guardian deity's.
    #[serde(default)]
    pub more: Vec<SwordDef>,
    /// The elbow keys turn the second pair of arms at the shoulder, and no
    /// key bends an elbow: the player's four arms (Sam, 2026-10-06: "your
    /// elbow controls are instead shoulder controls for your other arms").
    #[serde(default)]
    pub elbow_keys_turn_upper: bool,
    /// Rigid plates of armor worn over the body (`PlateDef`). Empty for a
    /// body that wears none.
    #[serde(default)]
    pub armor: Vec<PlateDef>,
    /// Which costume the page draws over this body, from data/costumes.json;
    /// empty for none. Nothing in the simulation reads it.
    #[serde(default)]
    pub costume: String,
    /// A shield round the body (Sam, 2026-10-08, after Dune's: "if a blade
    /// moves too fast it gets like locked in place"): a blade meeting a part
    /// of it faster than this, in cm a tick, is stopped there and cuts
    /// nothing; a slower one cuts as it would. `None` for no shield.
    #[serde(default)]
    pub shield: Option<Fx>,
}

/// A plate of armor, a helmet or a breastplate (Sam, 2026-10-08: "it should
/// act like a sword in the sense that its rigid, and should defend the area
/// beneath it from getting cut"). Its points, in the rest pose, are joined
/// in order into edges. It rides rigidly on the part from `near` to `far`
/// (`World::wear_armor`), weighs nothing and pushes nothing. A blade meets
/// it as it meets another blade (D10), and it cuts nothing. When a cut takes
/// `far` off the body, the plate goes with it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PlateDef {
    pub points: Vec<V2>,
    pub near: u8,
    pub far: u8,
}

impl BodyDef {
    /// Its weapons, the first pair of hands' first.
    pub fn weapons(&self) -> Vec<&SwordDef> {
        self.sword.iter().chain(self.more.iter()).collect()
    }
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

/// One seat: which body stands in it. `x` is how far behind the middle it
/// starts, along its own facing: seat 0 at `x` stands at -x.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Seat {
    pub body: u8,
    /// Where this seat's feet start; mirrored for the right seat.
    pub x: Fx,
    /// Its side, when not the seat's own (`side`): story mode's ally on
    /// the player's side.
    pub side: Option<u8>,
    /// Its facing, when not the seat's own (`facing`): story mode's
    /// opponents ahead on a stage to cross face the player.
    pub facing: Option<i8>,
}

impl Seat {
    /// A seat with its own side and facing.
    pub const fn at(body: u8, x: Fx) -> Seat {
        Seat { body, x, side: None, facing: None }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Setup {
    pub seed: u64,
    pub mode: Mode,
    pub rounds_to_win: u32,
    pub physics: Physics,
    pub bodies: Vec<BodyDef>,
    /// Seat 0 faces +x on the left; seat 1 faces -x on the right; seat 2,
    /// when a fight has one, is a second opponent on the left facing +x
    /// (Sam, 2026-10-05: "an enemy on each side of you"). Seat 0 is one
    /// side; seats 1 and 2 are the other.
    pub seats: [Option<Seat>; SEATS],
    /// Ledges a fighter can stand on, besides the ground.
    pub platforms: Vec<Platform>,
    /// How a round is won, besides putting the other side out (story mode).
    pub objective: Objective,
    /// Half the arena's width: the walls stand at plus and minus this.
    /// `balance::ARENA_HALF` except on story mode's stages to cross.
    pub arena_half: Fx,
}

/// What else ends a round in the player's favor (Sam, 2026-10-06: story
/// mode, after Melee's Adventure Mode).
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Objective {
    /// Only putting the other side out, as in every match before story mode.
    #[default]
    Rounds,
    /// Seat 0 wins the round by still being in it after this many ticks.
    HoldOut { ticks: u32 },
    /// Seat 0 wins the round by getting its pelvis to `x` (a stage to cross).
    Reach { x: Fx },
}

/// How many fighters a world can hold.
pub const SEATS: usize = 3;

/// The side a seat fights on: 0 for seat 0, 1 for the others.
pub fn side(seat: usize) -> u8 {
    if seat == 0 { 0 } else { 1 }
}

/// Which way a seat faces: +1 (toward +x) or -1.
pub fn facing(seat: usize) -> i32 {
    if seat == 1 { -1 } else { 1 }
}

/// A one-way ledge: solid from above, passable from below and the sides.
/// Its top runs from `x0` to `x1` at height `y`, in cm.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Platform {
    pub x0: Fx,
    pub x1: Fx,
    pub y: Fx,
}
