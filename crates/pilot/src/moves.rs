//! What each move of a tree pilot presses, tick by tick (its recipe), and
//! what a pilot did on the last tick and why (`Explain`). The recipes are
//! the table `Tree::play` reads, so the BT Lab page (Sam, 2026-10-06: "how
//! the high level behaviors and nodes get converted into controls
//! execution") shows the same keys the pilot presses, not a copy of them.

use serde::Serialize;
use sim::Input as I;

/// Whether a beat also steps, toward the opponent or away from it. Which
/// key that is depends on which side the opponent is on, so it is decided
/// each tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    None,
    Toward,
    Away,
}

/// Keys held from tick `from` to tick `to` of a move, both included.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Beat {
    pub from: u32,
    pub to: u32,
    pub keys: u16,
    pub step: Step,
}

const fn beat(from: u32, to: u32, keys: u16, step: Step) -> Beat {
    Beat { from, to, keys, step }
}

/// How a move makes its keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Recipe {
    /// Fixed keys by tick.
    Script { beats: &'static [Beat] },
    /// Hold the arm in a pose for `ticks`: the upper arm `shoulder` degrees
    /// above forward, the elbow bent `elbow` degrees from straight. A
    /// controller picks the keys each tick from how the arm lies and moves.
    Pose { shoulder: i32, elbow: i32, ticks: u32 },
    /// A short look-ahead: try each of a few key combinations on a copy of
    /// the world and keep the best, for `ticks`.
    Search { ticks: u32 },
    /// The throw's steps: settle, wind, watch for the window, let go.
    Throw,
}

use Step::{Away, None as Still, Toward};

static APPROACH: [Beat; 1] = [beat(0, 7, 0, Toward)];
static RETREAT: [Beat; 1] = [beat(0, 7, 0, Away)];
static OVERHEAD: [Beat; 2] = [beat(0, 15, I::SHOULDER_UP, Still), beat(16, 33, I::SHOULDER_DOWN | I::ELBOW_OUT, Still)];
static LOW_SWEEP: [Beat; 1] = [beat(0, 19, I::SHOULDER_DOWN, Toward)];
static THRUST: [Beat; 2] = [beat(0, 7, I::ELBOW_IN, Still), beat(8, 17, I::ELBOW_OUT, Toward)];
static SPIN: [Beat; 1] = [beat(0, 23, I::SHOULDER_UP, Toward)];
static JUMP_STRIKE: [Beat; 3] = [beat(0, 0, I::JUMP, Toward), beat(1, 9, I::SHOULDER_UP, Toward), beat(10, 29, I::SHOULDER_DOWN, Toward)];
static BOUNCE_STRIKE: [Beat; 4] = [
    beat(0, 0, I::JUMP, Still),
    beat(1, 13, I::SHOULDER_UP, Still),
    beat(14, 14, I::JUMP, Toward),
    beat(15, 35, I::SHOULDER_DOWN, Toward),
];
static DODGE_AWAY: [Beat; 2] = [beat(0, 0, I::DODGE, Away), beat(1, 17, 0, Still)];
static DODGE_IN: [Beat; 3] = [beat(0, 0, I::DODGE, Toward), beat(1, 17, 0, Still), beat(18, 35, I::SHOULDER_DOWN | I::ELBOW_OUT, Still)];
static POGO: [Beat; 3] = [beat(0, 12, I::ELBOW_IN, Toward), beat(13, 21, 0, Toward), beat(22, 55, I::SHOULDER_DOWN | I::ELBOW_OUT, Toward)];
static STAND: [Beat; 1] = [beat(0, 0, I::STAND, Still)];
// Up onto the ledge overhead: a jump, and the stand key once the pelvis has
// risen past the ledge's top (one jump lifts it about 120 cm in 27 ticks; by
// tick 16 it has risen 100).
static CLIMB: [Beat; 3] = [beat(0, 0, I::JUMP, Still), beat(1, 15, 0, Still), beat(16, 16, I::STAND, Still)];
static WAIT: [Beat; 1] = [beat(0, 9, 0, Still)];

/// A tree pilot's moves, by the name a rule's `do` gives.
pub fn recipe(name: &str) -> Option<Recipe> {
    let script = |beats: &'static [Beat]| Some(Recipe::Script { beats });
    match name {
        "approach" => script(&APPROACH),
        "retreat" => script(&RETREAT),
        "guard" => Some(Recipe::Pose { shoulder: 10, elbow: 10, ticks: 9 }),
        "high_guard" => Some(Recipe::Pose { shoulder: 70, elbow: 20, ticks: 9 }),
        "low_guard" => Some(Recipe::Pose { shoulder: -30, elbow: 10, ticks: 9 }),
        "overhead" => script(&OVERHEAD),
        "low_sweep" => script(&LOW_SWEEP),
        "thrust" => script(&THRUST),
        "spin" => script(&SPIN),
        "jump_strike" => script(&JUMP_STRIKE),
        "bounce_strike" => script(&BOUNCE_STRIKE),
        "dodge_away" => script(&DODGE_AWAY),
        "dodge_in" => script(&DODGE_IN),
        "pogo" => script(&POGO),
        "stand" => script(&STAND),
        "climb" => script(&CLIMB),
        "wait" => script(&WAIT),
        "search" => Some(Recipe::Search { ticks: 11 }),
        "throw" => Some(Recipe::Throw),
        _ => None,
    }
}

/// Every move a tree may name, in the order the BT Lab lists them.
pub const MOVES: [&str; 19] = [
    "approach", "retreat", "guard", "high_guard", "low_guard", "overhead", "low_sweep", "thrust", "spin", "jump_strike",
    "bounce_strike", "dodge_away", "dodge_in", "pogo", "stand", "climb", "wait", "search", "throw",
];

/// The pose controller (`pose_keys`): each tick a joint aims to turn
/// 1/POSE_SHARE of its gap to the pose, at most POSE_MAX milliradians, and
/// a key is pressed when it turns more than POSE_BAND off that aim. A first
/// version pressed toward the pose until it arrived and overshot every time
/// (SECOND-ORDER-M5).
pub const POSE_SHARE: i64 = 10;
pub const POSE_MAX: i64 = 120;
pub const POSE_BAND: i64 = 8;

/// One joint of a pose: how far it is from the pose (milliradians, toward
/// the key that closes it positive), the turn a tick it aims for (a tenth
/// of that, at most 120), the turn a tick it has, and the key that follows:
/// "plus" when it turns too slowly toward the pose, "minus" when too fast,
/// "none" within 8 of the aim.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Joint {
    pub gap: i64,
    pub aim: i64,
    pub speed: i64,
    pub key: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct PoseWhy {
    pub shoulder_deg: i32,
    pub elbow_deg: i32,
    pub shoulder: Joint,
    pub elbow: Joint,
}

/// The search's last choice: each key combination it tried with its score,
/// and how many ticks more it keeps the one it chose.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SearchWhy {
    pub tried: Vec<(u16, i64)>,
    pub chosen: u16,
    pub keeps: u32,
    pub horizon: u32,
}

/// What a pilot pressed on its last tick, and why.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Explain {
    /// Which of a many-armed pilot's trees ("tree.many.arms" and so on).
    pub part: Option<&'static str>,
    /// The move running, and its tick.
    pub mv: Option<String>,
    pub t: u32,
    pub keys: u16,
    /// For a script, the beat that gave the keys, and the step key it added.
    pub beat: Option<usize>,
    pub step_keys: u16,
    pub pose: Option<PoseWhy>,
    pub search: Option<SearchWhy>,
    /// For a throw, its step (0 settle, 1 wind, 2 watch, 3 follow through)
    /// and whether the window was open this tick.
    pub throw_step: Option<usize>,
    pub window: bool,
}
