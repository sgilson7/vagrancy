//! Opponents (D16). A pilot returns an `Input` and nothing else, so it holds
//! nothing a player cannot press. It reads the world as a player's eyes
//! would, and it is shown every seat's last input (`observe`), as a
//! person beside you sees what you do.
//!
//! No learned policy: a search baseline must be beaten before any network is
//! justified (gear-master/design/rl-agent-plan.md:53).
#![forbid(unsafe_code)]

use serde::Deserialize;
use sim::fight::Phase;
use sim::fx::{cos_deg, sin_deg, V2, ONE};
use sim::world::Owner;
use sim::body::SEATS;
use sim::{Input, World};
use std::collections::VecDeque;

pub mod moves;
pub mod view;

/// One opponent's entry in `data/pilots.json`.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Spec {
    /// Does nothing. `unarmed`: stands in the arena without a sword.
    Still {
        #[serde(default)]
        unarmed: bool,
    },
    Pose { shoulder: i32, elbow: i32 },
    Loop { pattern: String, #[serde(default)] pause_ticks: u32, #[serde(default)] drift: bool },
    Machine { script: String },
    Replayer { first_round: String, mirror: bool },
    Search { horizon_ticks: u32, reaction_ticks: u32, branches: u32, #[serde(default = "default_period")] period: u32 },
    /// `search_period`: how many ticks the search act keeps a plan before
    /// it plans again (3 if unset). The local deity sets it higher, so its
    /// look-ahead costs a browser frame what one opponent's does.
    Tree { reaction_ticks: u32, rules: Vec<Rule>, #[serde(default)] salt: u64, #[serde(default)] search_period: Option<u32> },
    /// A four-armed body run by three pilots at once (the local deity, Sam
    /// 2026-10-06: "1 behavior tree per set of arms, and one behavior tree
    /// governing the legs"): `arms` turns the first pair, `upper` the
    /// second, `legs` the steps, the jump, the dodge and getting up.
    Many { arms: Box<Spec>, upper: Box<Spec>, legs: Box<Spec> },
}

impl Spec {
    /// Its kind, as data/pilots.json names it.
    pub fn kind(&self) -> &'static str {
        match self {
            Spec::Still { .. } => "still",
            Spec::Pose { .. } => "pose",
            Spec::Loop { .. } => "loop",
            Spec::Machine { .. } => "machine",
            Spec::Replayer { .. } => "replayer",
            Spec::Search { .. } => "search",
            Spec::Tree { .. } => "tree",
            Spec::Many { .. } => "many",
        }
    }
}

fn default_period() -> u32 {
    6
}

pub trait Pilot {
    /// This tick's input for `seat`.
    fn input(&mut self, w: &World, seat: usize) -> Input;
    /// Every seat's input last tick, its own included.
    fn observe(&mut self, _last: [Input; SEATS]) {}
    /// What ran on the last tick, by the ids of `view::describe` of this
    /// pilot's spec. Reading it changes nothing.
    fn trace(&self) -> view::Trace {
        view::Trace::default()
    }
    /// What it pressed on the last tick and why (moves::Explain), for the
    /// BT Lab; one for each tree of a many-armed pilot. Reading it changes
    /// nothing.
    fn explain(&self) -> Vec<moves::Explain> {
        Vec::new()
    }
}

pub fn build(spec: &Spec) -> Box<dyn Pilot> {
    match spec.clone() {
        Spec::Still { .. } => Box::new(Still),
        Spec::Pose { shoulder, elbow } => Box::new(Pose { shoulder, elbow }),
        Spec::Loop { pattern, pause_ticks, drift } => Box::new(Looper::new(&pattern, pause_ticks, drift)),
        Spec::Machine { script, .. } => Box::new(Machine::new(&script)),
        Spec::Replayer { mirror, .. } => Box::new(Replayer { mirror, now: Vec::new(), last: Vec::new(), seen: [Input::NONE; SEATS], round: 0, t: 0 }),
        Spec::Search { horizon_ticks, reaction_ticks, branches, period } => {
            Box::new(Search::new(horizon_ticks, reaction_ticks, branches, period))
        }
        Spec::Tree { reaction_ticks, rules, salt, search_period } => {
            let mut t = Tree::new(rules, reaction_ticks, salt);
            t.period = search_period.unwrap_or(view::TREE_SEARCH.2);
            Box::new(t)
        }
        Spec::Many { arms, upper, legs } => {
            Box::new(Many { arms: build(&arms), upper: build(&upper), legs: build(&legs), offsets: view::many_offsets(&arms, &upper) })
        }
    }
}

// --- three pilots in one body ---------------------------------------------------

pub struct Many {
    arms: Box<dyn Pilot>,
    upper: Box<dyn Pilot>,
    legs: Box<dyn Pilot>,
    /// Where each tree's nodes start in `view::describe`'s numbering.
    offsets: [u16; 3],
}

const ARM_BITS: u16 = Input::SHOULDER_UP | Input::SHOULDER_DOWN | Input::ELBOW_IN | Input::ELBOW_OUT;
const ARM2_BITS: u16 = Input::SHOULDER2_UP | Input::SHOULDER2_DOWN | Input::ELBOW2_IN | Input::ELBOW2_OUT;
const LEG_BITS: u16 = Input::STEP_LEFT | Input::STEP_RIGHT | Input::JUMP | Input::DODGE | Input::STAND;

/// The world as `seat`'s second pair of arms sees it: its two weapons and
/// its two pairs of arms traded, so a pilot written for one pair of arms
/// and one weapon runs the second pair unchanged, its look-ahead included.
/// Its first-pair keys are then the second pair's (`Input::arms_swapped`).
pub fn upper_view(w: &World, seat: usize) -> World {
    use sim::body::Motor;
    let mut v = w.clone();
    let mine = v.swords_of(seat);
    if let (Some(&a), Some(&b)) = (mine.first(), mine.get(1)) {
        v.swords.swap(a, b);
        let (a8, b8) = (a as u8, b as u8);
        let swap = |k: u8| if k == a8 { b8 } else if k == b8 { a8 } else { k };
        for pair in &mut v.clashing {
            let (x, y) = (swap(pair.0), swap(pair.1));
            *pair = (x.min(y), x.max(y));
        }
        v.clashing.sort();
        for t in &mut v.touching {
            t.0 = swap(t.0);
        }
    }
    if let Some(f) = v.fighters[seat].as_ref() {
        let def = &mut v.setup.bodies[f.body as usize];
        if let (Some(a), Some(b)) = (def.sword.as_mut(), def.more.first_mut()) {
            std::mem::swap(a, b);
        }
        for p in &mut def.parts {
            p.motor = p.motor.map(|m| match m {
                Motor::Shoulder => Motor::Shoulder2,
                Motor::Shoulder2 => Motor::Shoulder,
                Motor::Elbow => Motor::Elbow2,
                Motor::Elbow2 => Motor::Elbow,
            });
        }
    }
    v
}

impl Pilot for Many {
    fn input(&mut self, w: &World, seat: usize) -> Input {
        if let Some(i) = between(w) {
            return i;
        }
        let a = self.arms.input(w, seat).0 & ARM_BITS;
        let u = self.upper.input(&upper_view(w, seat), seat).arms_swapped().0 & ARM2_BITS;
        let l = self.legs.input(w, seat).0 & LEG_BITS;
        Input(a | u | l)
    }

    fn observe(&mut self, last: [Input; SEATS]) {
        self.arms.observe(last);
        self.upper.observe(last.map(Input::arms_swapped));
        self.legs.observe(last);
    }

    fn explain(&self) -> Vec<moves::Explain> {
        // The upper tree pressed the first pair's keys on its traded world;
        // the keys it gave the body are the second pair's.
        let part = |mut v: Vec<moves::Explain>, p: &'static str, swap: bool| {
            for e in &mut v {
                e.part = Some(p);
                if swap {
                    e.keys = Input(e.keys).arms_swapped().0;
                    if let Some(pw) = e.pose.as_mut() {
                        pw.shoulder.key = Input(pw.shoulder.key).arms_swapped().0;
                        pw.elbow.key = Input(pw.elbow.key).arms_swapped().0;
                    }
                }
            }
            v
        };
        let mut out = part(self.arms.explain(), "tree.many.arms", false);
        out.extend(part(self.upper.explain(), "tree.many.upper", true));
        out.extend(part(self.legs.explain(), "tree.many.legs", false));
        out
    }

    fn trace(&self) -> view::Trace {
        // The three trees hang from one root, numbered in this order.
        let mut t = view::Trace { active: vec![0], held: Vec::new(), failed: Vec::new() };
        for (sub, off) in [self.arms.trace(), self.upper.trace(), self.legs.trace()].into_iter().zip(self.offsets) {
            t.active.extend(sub.active.iter().map(|i| i + off));
            t.held.extend(sub.held.iter().map(|i| i + off));
            t.failed.extend(sub.failed.iter().map(|i| i + off));
        }
        t
    }
}

// --- what a pilot can see ----------------------------------------------------------

/// A pilot's input with its ready bit kept only when a person asked for the
/// next round (`asked`): in watch mode the round's end runs on until the
/// viewer presses Enter (Sam, 2026-10-07: "the end of each round should run
/// in the simulation until the player presses enter, its only in streaming
/// mode that it should auto go to the next round").
pub fn ready_when_asked(i: Input, asked: bool) -> Input {
    if asked { i } else { Input(i.0 & !Input::READY) }
}

/// Between rounds every pilot presses ready; after the match, nothing.
fn between(w: &World) -> Option<Input> {
    match w.phase {
        Phase::Fight => None,
        Phase::RoundOver { .. } => Some(Input(Input::READY)),
        Phase::MatchOver { .. } => Some(Input::NONE),
    }
}

fn role(w: &World, seat: usize, pick: fn(&sim::body::Roles) -> Option<u8>) -> Option<V2> {
    let f = w.fighters[seat].as_ref()?;
    let i = (f.base + pick(&w.setup.bodies[f.body as usize].roles)? as u16) as usize;
    (w.particles[i].owner == Owner::Body(seat as u8)).then(|| w.particles[i].p)
}

pub fn head(w: &World, seat: usize) -> Option<V2> {
    role(w, seat, |r| r.head)
}

pub fn shoulder(w: &World, seat: usize) -> Option<V2> {
    role(w, seat, |r| r.shoulder)
}

pub fn pelvis(w: &World, seat: usize) -> Option<V2> {
    role(w, seat, |r| r.pelvis)
}

fn facing(w: &World, seat: usize) -> i32 {
    w.fighters[seat].as_ref().map(|f| f.facing).unwrap_or(1)
}

fn tip(w: &World, seat: usize) -> Option<V2> {
    w.swords.iter().find(|s| s.fighter as usize == seat && s.armor.is_none()).map(|s| w.particles[s.tip as usize].p)
}

/// Horizontal distance between the two pelvises, in whole cm.
/// No foot still attached to the fighter stands on the ground or a ledge.
pub fn airborne(w: &World, seat: usize) -> bool {
    w.fighters[seat].as_ref().is_some_and(|f| {
        let def = &w.setup.bodies[f.body as usize];
        !def.roles.feet.iter().any(|&i| {
            let k = (f.base + i as u16) as usize;
            w.particles[k].owner == Owner::Body(seat as u8) && w.supported(k as u16)
        })
    })
}

/// How far inside a ledge's ends a pilot must stand to climb onto it, cm.
const LEDGE_ROOM: i32 = 40;

/// How high the lowest attached foot's sole is above the ground.
fn feet_height(w: &World, seat: usize) -> sim::fx::Fx {
    let Some(f) = w.fighters[seat].as_ref() else { return sim::fx::Fx(0) };
    let def = &w.setup.bodies[f.body as usize];
    def.roles.feet.iter().map(|&i| &w.particles[(f.base + i as u16) as usize]).filter(|p| p.owner == Owner::Body(seat as u8)).map(|p| p.p.y - p.rad).min().unwrap_or(sim::fx::Fx(0))
}

/// A foot stands on a ledge rather than the ground.
pub fn on_ledge(w: &World, seat: usize) -> bool {
    w.fighters[seat].as_ref().is_some_and(|f| {
        let def = &w.setup.bodies[f.body as usize];
        def.roles.feet.iter().any(|&i| {
            let k = (f.base + i as u16) as usize;
            w.particles[k].owner == Owner::Body(seat as u8) && w.particles[k].p.y > w.particles[k].rad + ONE && w.supported(k as u16)
        })
    })
}

/// A hand of `seat`'s holds its sword.
pub fn armed(w: &World, seat: usize) -> bool {
    w.swords.iter().position(|s| s.fighter as usize == seat && s.armor.is_none()).is_some_and(|si| w.held(si))
}

/// The fighter `seat` fights: the nearest one on the other side still in
/// the round, or, when every one of them is out, the nearest at all. With
/// two seats this is always the other seat.
pub fn foe(w: &World, seat: usize) -> usize {
    let me = pelvis(w, seat).map(|p| p.x);
    let mut best: Option<(bool, i32, usize)> = None;
    for k in 0..SEATS {
        if k == seat || w.fighters[k].is_none() || w.side_of(k) == w.side_of(seat) {
            continue;
        }
        let d = match (me, pelvis(w, k)) {
            (Some(a), Some(b)) => (b.x - a).abs().trunc(),
            _ => i32::MAX,
        };
        let key = (w.out(k), d, k);
        if best.is_none_or(|b| key < b) {
            best = Some(key);
        }
    }
    best.map(|b| b.2).unwrap_or(if seat == 0 { 1 } else { 0 })
}

pub fn gap(w: &World, seat: usize) -> i32 {
    match (pelvis(w, seat), pelvis(w, foe(w, seat))) {
        (Some(a), Some(b)) => (b.x - a.x).abs().trunc(),
        _ => 0,
    }
}

/// The step bit that moves `seat` toward (or away from) the other fighter.
fn step_toward(w: &World, seat: usize, toward: bool) -> u16 {
    let (Some(a), Some(b)) = (pelvis(w, seat), pelvis(w, foe(w, seat))) else { return 0 };
    let right = (b.x > a.x) == toward;
    if right { Input::STEP_RIGHT } else { Input::STEP_LEFT }
}

/// The upper arm and the forearm, now and a tick ago, from the motor parts.
fn arm(w: &World, seat: usize) -> Option<[(V2, V2); 2]> {
    let defs = &w.setup.bodies[w.fighters[seat].as_ref()?.body as usize].parts;
    let find = |m: sim::body::Motor| {
        w.parts.iter().find(|p| p.fighter as usize == seat && p.attached && defs[p.def as usize].motor == Some(m))
    };
    let up = find(sim::body::Motor::Shoulder)?;
    let fore = find(sim::body::Motor::Elbow)?;
    let v = |a: u16, b: u16| (w.particles[b as usize].p - w.particles[a as usize].p, w.particles[b as usize].q - w.particles[a as usize].q);
    Some([v(up.a, up.b), v(fore.a, fore.b)])
}

/// Signed angle from `a` to `b` (counterclockwise positive), in thousandths of
/// a radian, small-angle accurate and monotonic either side of zero: enough
/// to steer by.
fn turn_milli(a: V2, b: V2) -> i64 {
    let cross = a.cross_raw(b) as i128;
    let dot = a.dot_raw(b) as i128;
    let mag = (a.len().0 as i128 * b.len().0 as i128).max(1);
    // sin(θ) from the cross product; past a right angle, keep the sign and
    // add the rest so the error grows with the angle.
    let s = cross * 1000 / mag;
    if dot >= 0 { s as i64 } else { (s.signum() * (2000 - s.abs())) as i64 }
}

/// The keys that turn the arm toward a pose: the upper arm at `shoulder`
/// degrees above forward, the elbow bent `elbow` degrees from straight.
///
/// A speed controller, not a switch: each joint aims for a turning speed in
/// proportion to how far it is from the pose, and presses whichever key
/// closes the gap between that and the speed it has. A first version pressed
/// toward the pose until it got there and overshot every time; the
/// gatekeeper, who "does not swing", swung (SECOND-ORDER-M5).
fn pose_keys(w: &World, seat: usize, shoulder: i32, elbow: i32) -> u16 {
    pose_why(w, seat, shoulder, elbow).0
}

/// `pose_keys`, with the numbers each joint's key was chosen from.
fn pose_why(w: &World, seat: usize, shoulder: i32, elbow: i32) -> (u16, moves::PoseWhy) {
    let mut why = moves::PoseWhy { shoulder_deg: shoulder, elbow_deg: elbow, ..Default::default() };
    let Some([(u, u0), (f, f0)]) = arm(w, seat) else { return (0, why) };
    let face = facing(w, seat) as i64;
    let want = V2::new(cos_deg(shoulder) * face as i32, sin_deg(shoulder));
    let steer = |err: i64, speed: i64, plus: u16, minus: u16| -> moves::Joint {
        // Aim to close a share of the error each tick, up to a limit.
        let aim = (err / moves::POSE_SHARE).clamp(-moves::POSE_MAX, moves::POSE_MAX);
        let key = if speed < aim - moves::POSE_BAND {
            plus
        } else if speed > aim + moves::POSE_BAND {
            minus
        } else {
            0
        };
        moves::Joint { gap: err, aim, speed, key }
    };
    // Facing +x, counterclockwise is "up"; facing -x it is "down".
    let err = turn_milli(u, want) * face;
    let speed = turn_milli(u0, u) * face;
    why.shoulder = steer(err, speed, Input::SHOULDER_UP, Input::SHOULDER_DOWN);
    // The elbow's bend is the turn from the upper arm to the forearm.
    let bend = turn_milli(u, f) * face;
    let bend0 = turn_milli(u0, f0) * face;
    let target = (sin_deg(elbow).0 as i64 * 1000) / ONE.0 as i64;
    why.elbow = steer(target - bend, bend - bend0, Input::ELBOW_IN, Input::ELBOW_OUT);
    (why.shoulder.key | why.elbow.key, why)
}

// --- the kinds -------------------------------------------------------------------------

struct Still;
impl Pilot for Still {
    fn input(&mut self, w: &World, _: usize) -> Input {
        between(w).unwrap_or(Input::NONE)
    }
    fn trace(&self) -> view::Trace {
        view::Trace { active: vec![0], ..Default::default() }
    }
}

struct Pose {
    shoulder: i32,
    elbow: i32,
}
impl Pilot for Pose {
    fn input(&mut self, w: &World, seat: usize) -> Input {
        if let Some(i) = between(w) {
            return i;
        }
        Input(pose_keys(w, seat, self.shoulder, self.elbow))
    }
    fn trace(&self) -> view::Trace {
        view::Trace { active: vec![0], ..Default::default() }
    }
}

/// A repeating sequence of keys, closing the distance when out of reach.
struct Looper {
    steps: Vec<(u16, u32)>,
    drift: bool,
    at: usize,
    left: u32,
    /// For the trace: the step played and whether it walked, last tick.
    played: Option<(usize, bool)>,
}

impl Looper {
    fn new(pattern: &str, pause: u32, drift: bool) -> Looper {
        let steps = match pattern {
            // Raise the sword overhead, bring it down in one stroke, pause.
            "overhead" => vec![(Input::SHOULDER_UP, 20), (Input::SHOULDER_DOWN, 22), (0, pause.max(1))],
            // One direction, without stopping.
            "spin" => vec![(Input::SHOULDER_UP, 600)],
            _ => vec![(0, 60)],
        };
        Looper { left: steps[0].1, steps, drift, at: 0, played: None }
    }
}

impl Pilot for Looper {
    fn input(&mut self, w: &World, seat: usize) -> Input {
        if let Some(i) = between(w) {
            self.at = 0;
            self.left = self.steps[0].1;
            self.played = None;
            return i;
        }
        let (keys, _) = self.steps[self.at];
        let step = self.at;
        self.left = self.left.saturating_sub(1);
        if self.left == 0 {
            self.at = (self.at + 1) % self.steps.len();
            self.left = self.steps[self.at].1;
        }
        // Close in to striking distance; the windmill drifts in regardless.
        let walking = self.drift || gap(w, seat) > 200;
        self.played = Some((step, walking));
        let walk = if walking { step_toward(w, seat, true) } else { 0 };
        Input(keys | walk)
    }
    fn trace(&self) -> view::Trace {
        // Ids as `view::describe` numbers them: the parallel 0, the repeat
        // 1, its sequence 2, the steps from 3, then the walk.
        let Some((step, walking)) = self.played else { return view::Trace::default() };
        let n = self.steps.len() as u16;
        let mut t = view::Trace { active: vec![0, 1, 2, 3 + step as u16], ..Default::default() };
        if self.drift {
            t.active.push(3 + n);
        } else if walking {
            t.active.extend([3 + n, 4 + n, 5 + n]);
            t.held.push(4 + n);
        } else {
            t.failed.push(4 + n);
        }
        t
    }
}

/// Short state machines for the ferryman and the vaulter.
struct Machine {
    script: String,
    t: u32,
    state: u8,
    /// For the trace: the state that played last tick.
    played: Option<u8>,
}

impl Machine {
    fn new(script: &str) -> Machine {
        Machine { script: script.into(), t: 0, state: 0, played: None }
    }
}

impl Pilot for Machine {
    fn input(&mut self, w: &World, seat: usize) -> Input {
        if let Some(i) = between(w) {
            self.t = 0;
            self.state = 0;
            self.played = None;
            return i;
        }
        self.t += 1;
        self.played = Some(self.state);
        let d = gap(w, seat);
        let b = match self.script.as_str() {
            // The ferryman: guard high, wait for the other fighter to come
            // into his longer reach, then one heavy stroke down, then recover.
            "late_heavy" => match self.state {
                0 => {
                    if d < 230 {
                        self.state = 1;
                        self.t = 0;
                    }
                    pose_keys(w, seat, 70, 20) | if d > 260 { step_toward(w, seat, true) } else { 0 }
                }
                1 => {
                    if self.t > 24 {
                        self.state = 2;
                        self.t = 0;
                    }
                    Input::SHOULDER_DOWN | Input::ELBOW_OUT
                }
                _ => {
                    if self.t > 50 {
                        self.state = 0;
                    }
                    pose_keys(w, seat, 70, 20) | step_toward(w, seat, false)
                }
            },
            // The vaulter: close in, plant, vault, cut down on the way over.
            "vault" => match self.state {
                0 => {
                    if d < 210 {
                        self.state = 1;
                        self.t = 0;
                    }
                    step_toward(w, seat, true)
                }
                1 => {
                    // The plant-and-push the pogo test uses.
                    let k = match self.t {
                        0..=12 => Input::ELBOW_IN,
                        13..=21 => 0,
                        22..=55 => Input::SHOULDER_DOWN | Input::ELBOW_OUT,
                        _ => {
                            self.state = 2;
                            self.t = 0;
                            0
                        }
                    };
                    k | step_toward(w, seat, true)
                }
                _ => {
                    if self.t > 70 {
                        self.state = 0;
                    }
                    Input::SHOULDER_DOWN
                }
            },
            _ => 0,
        };
        Input(b)
    }
    fn trace(&self) -> view::Trace {
        // The repeat 0, its sequence 1, the states from 2.
        match self.played {
            Some(st) => view::Trace { active: vec![0, 1, 2 + st as u16], ..Default::default() },
            None => view::Trace::default(),
        }
    }
}

/// The sampler: still in the first round; after that, the previous round's
/// keys of the other side, mirrored.
struct Replayer {
    mirror: bool,
    now: Vec<Input>,
    last: Vec<Input>,
    seen: [Input; SEATS],
    round: u32,
    t: usize,
}

impl Pilot for Replayer {
    fn observe(&mut self, last: [Input; SEATS]) {
        self.seen = last;
    }
    fn input(&mut self, w: &World, seat: usize) -> Input {
        let other = self.seen[foe(w, seat)];
        self.now.push(Input(other.0 & !Input::READY));
        if w.round != self.round {
            // A new round (or a draw played again): what was just watched
            // becomes what is played.
            if !self.now.is_empty() {
                self.last = std::mem::take(&mut self.now);
            }
            self.round = w.round;
            self.t = 0;
        }
        if let Some(i) = between(w) {
            return i;
        }
        if w.round == 1 {
            return Input::NONE;
        }
        let i = self.last.get(self.t).copied().unwrap_or(Input::NONE);
        self.t += 1;
        if self.mirror { i.mirror() } else { i }
    }
    fn trace(&self) -> view::Trace {
        // The selector 0; its first branch 1 (the condition 2, watching 3);
        // replaying, 4.
        if self.round <= 1 {
            view::Trace { active: vec![0, 1, 2, 3], held: vec![2], failed: vec![] }
        } else {
            view::Trace { active: vec![0, 4], held: vec![], failed: vec![2] }
        }
    }
}

/// A short look-ahead on cloned worlds: try each of a few inputs for
/// `horizon` ticks, from what was seen `reaction` ticks ago, and keep the
/// best for `period` ticks.
pub struct Search {
    horizon: u32,
    reaction: u32,
    branches: u32,
    period: u32,
    seen: VecDeque<World>,
    chosen: Input,
    left: u32,
    others: [Input; SEATS],
    /// The last choice, for the BT Lab.
    pub why: moves::SearchWhy,
}

impl Search {
    pub fn new(horizon: u32, reaction: u32, branches: u32, period: u32) -> Search {
        Search { horizon, reaction, branches, period: period.max(1), seen: VecDeque::new(), chosen: Input::NONE, left: 0, others: [Input::NONE; SEATS], why: moves::SearchWhy::default() }
    }

    fn candidates(&self, w: &World, seat: usize) -> Vec<Input> {
        let toward = step_toward(w, seat, true);
        let away = step_toward(w, seat, false);
        let all = [
            0,
            Input::SHOULDER_UP,
            Input::SHOULDER_DOWN,
            Input::SHOULDER_DOWN | Input::ELBOW_OUT,
            Input::SHOULDER_UP | Input::ELBOW_OUT,
            Input::ELBOW_IN,
            toward,
            toward | Input::SHOULDER_DOWN,
            away,
            toward | Input::SHOULDER_UP,
            away | Input::SHOULDER_UP,
        ];
        all.iter().take(self.branches as usize).map(|&b| Input(b)).collect()
    }

    /// How good a world is for `seat`, in whole points.
    fn score(w: &World, seat: usize) -> i64 {
        let me = seat;
        let them = foe(w, seat);
        let mut s: i64 = 0;
        match w.phase {
            Phase::RoundOver { result, .. } | Phase::MatchOver { result } => match result.loser {
                Some(l) if l != w.side_of(me) => s += 100_000,
                Some(_) => s -= 100_000,
                None => s -= 20_000,
            },
            Phase::Fight => {}
        }
        let ink = |k: usize| w.fighters[k].as_ref().map(|f| f.ink as i64).unwrap_or(0);
        s += 20 * (ink(me) - ink(them));
        // Threaten: the point near their head. Stay safe: theirs far from mine.
        let dist = |a: Option<V2>, b: Option<V2>| match (a, b) {
            (Some(a), Some(b)) => (a - b).len().trunc() as i64,
            _ => 400,
        };
        s -= dist(tip(w, me), head(w, them));
        s += dist(tip(w, them), head(w, me)) / 2;
        // Keep a striking distance rather than charging onto the other blade.
        s -= 2 * (gap(w, me) as i64 - 180).abs();
        // Keep one's own point away from one's own body, every part of it:
        // the point is the only part of a blade that cuts its own fighter
        // (SECOND-ORDER-M3). Measured from the pelvis alone, the shins sat
        // outside the margin and were cut, match after match.
        if let Some(t) = tip(w, me) {
            let near = w
                .parts
                .iter()
                .filter(|p| p.fighter as usize == me && p.attached)
                .map(|p| {
                    let (_, _, c, _) = sim::contact::closest(w.particles[p.a as usize].p, w.particles[p.b as usize].p, t, t);
                    ((c - t).len() - p.radius).trunc() as i64
                })
                .min()
                .unwrap_or(400);
            s -= 40 * (40 - near).max(0);
        }
        s
    }
}

impl Pilot for Search {
    fn trace(&self) -> view::Trace {
        view::Trace { active: vec![0], ..Default::default() }
    }
    fn explain(&self) -> Vec<moves::Explain> {
        vec![moves::Explain { mv: Some("search".into()), keys: self.chosen.0, search: Some(self.why.clone()), ..Default::default() }]
    }
    fn observe(&mut self, last: [Input; SEATS]) {
        self.others = last.map(|i| Input(i.0 & !Input::READY));
    }

    fn input(&mut self, w: &World, seat: usize) -> Input {
        self.seen.push_back(w.clone());
        while self.seen.len() as u32 > self.reaction + 1 {
            self.seen.pop_front();
        }
        if let Some(i) = between(w) {
            return i;
        }
        if self.left > 0 {
            self.left -= 1;
            self.why.keeps = self.left;
            return self.chosen;
        }
        let base = self.seen.front().unwrap();
        let mut best = (i64::MIN, Input::NONE);
        let mut tried = Vec::new();
        for c in self.candidates(base, seat) {
            let mut trial = base.clone();
            let mut inputs = self.others;
            inputs[seat] = c;
            // Look ahead on the candidate. A cut that lands during the look-
            // ahead counts heavily, against the pilot when it is on itself:
            // without this the search stabbed toward the other head through
            // its own body and lost to a scarecrow that never moves
            // (`lab ladder 4`, SECOND-ORDER-M5).
            let mut cuts: i64 = 0;
            for _ in 0..self.horizon {
                trial.step_all(inputs);
                for e in &trial.events {
                    if let sim::fight::Event::Cut { seat: s, spilled: true, .. } = *e {
                        cuts += if s as usize == seat {
                            -3_000
                        } else if trial.side_of(s as usize) != trial.side_of(seat) {
                            3_000
                        } else {
                            0
                        };
                    }
                }
                if !matches!(trial.phase, Phase::Fight) {
                    break;
                }
            }
            let s = Search::score(&trial, seat) + cuts;
            tried.push((c.0, s));
            if s > best.0 {
                best = (s, c);
            }
        }
        self.chosen = best.1;
        self.left = self.period - 1;
        self.why = moves::SearchWhy { tried, chosen: self.chosen.0, keeps: self.left, horizon: self.horizon };
        self.chosen
    }
}

/// How a match between two pilots went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub wins: [u32; 2],
    pub ticks: u32,
    pub finished: bool,
}

/// Play a match between pilots, one per seat in order (two, or three when
/// the setup seats a second opponent), to its end or to `max_ticks`.
pub fn duel(setup: sim::Setup, pilots: &mut [Box<dyn Pilot>], max_ticks: u32) -> Outcome {
    let mut w = World::new(setup);
    let mut last = [Input::NONE; SEATS];
    while w.tick < max_ticks && !matches!(w.phase, Phase::MatchOver { .. }) {
        let mut i = [Input::NONE; SEATS];
        for (k, p) in pilots.iter_mut().enumerate() {
            p.observe(last);
            i[k] = p.input(&w, k);
        }
        w.step_all(i);
        last = i;
    }
    Outcome { wins: w.wins, ticks: w.tick, finished: matches!(w.phase, Phase::MatchOver { .. }) }
}

/// For the page and the tests: the integer value placeholders are filled
/// with, from a pilot's data (D16). `{pause_s}` and the rest are derived
/// here, never typed into a sentence.
pub fn numbers(spec: &Spec) -> Vec<(&'static str, String)> {
    let ms = |ticks: u32| (ticks * 1000).div_ceil(sim::balance::TICKS_PER_SECOND);
    match spec {
        Spec::Loop { pause_ticks, .. } if *pause_ticks > 0 => {
            let tenths = (pause_ticks * 10).div_ceil(sim::balance::TICKS_PER_SECOND);
            vec![("pause_s", if tenths % 10 == 0 { format!("{}", tenths / 10) } else { format!("{}.{}", tenths / 10, tenths % 10) })]
        }
        Spec::Search { horizon_ticks, reaction_ticks, .. } => {
            vec![("horizon_ms", ms(*horizon_ticks).to_string()), ("reaction_ms", ms(*reaction_ticks).to_string())]
        }
        Spec::Tree { reaction_ticks, .. } => vec![("reaction_ms", ms(*reaction_ticks).to_string())],
        Spec::Many { arms, .. } => numbers(arms),
        _ => vec![],
    }
}

// --- behavior trees (Sam, 2026-10-03: "about 10 more battles with enemies with
// different behavior trees, getting increasingly skilled / dangerous") -------------
//
// A tree pilot is data: an ordered list of rules, each a set of conditions and
// a move. The first rule whose conditions hold starts its move, and the pilot
// commits to it until it ends, as a person commits to a swing; a rule marked
// `interrupt` may break in. It reads the world as it was `reaction_ticks`
// ago, which is most of what separates a slow opponent from a quick one.

/// One condition on what the pilot sees.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Cond {
    /// The fighters are farther apart than this, cm.
    GapAbove(i32),
    GapBelow(i32),
    /// The other's sword point is within this of my head, cm.
    TipNearHead(i32),
    /// … or of my pelvis.
    TipNearBody(i32),
    MeAirborne,
    MeGrounded,
    OppAirborne,
    MeDown,
    OppDodging,
    /// My ink is below this share, in percent.
    MyInkBelow(i32),
    /// A seeded chance, in percent, drawn when the rule is considered.
    Chance(u32),
    /// A fighter on my side is within this of the one I fight, cm: a
    /// partner has engaged (flanked stops, Sam 2026-10-05).
    AllyEngaged(i32),
    /// The one I fight stands higher than me by more than this, cm.
    OppAbove(i32),
    /// … or lower.
    OppBelow(i32),
    /// A ledge spans where I stand, with room to spare, and its top is above
    /// my feet: a jump and a stand would set me on it.
    LedgeOverhead,
    /// I stand on a ledge.
    OnLedge,
    /// A hand of mine holds my sword: I have not thrown it this round.
    Armed,
    /// I have thrown my sword, or lost the hand that held it.
    Unarmed,
    /// The one I fight is knocked off its feet.
    OppDown,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Rule {
    #[serde(default, rename = "if")]
    pub when: Vec<Cond>,
    #[serde(rename = "do")]
    pub act: String,
    #[serde(default)]
    pub interrupt: bool,
}

pub struct Tree {
    rules: Vec<Rule>,
    reaction: u32,
    seen: VecDeque<World>,
    rng: sim::rng::Rng,
    current: Option<(String, u32)>,
    search: Option<Search>,
    /// For the trace: each rule's node ids, the rule running, and the
    /// conditions checked on the last decision with what each found.
    ids: Vec<view::RuleIds>,
    running: Option<usize>,
    checked: Vec<(u16, bool)>,
    /// How many ticks a search plan is kept (`Spec::Tree::search_period`).
    pub period: u32,
    /// The step of a throw under way, and the tick of the move it began on.
    throw: Option<(ThrowStep, u32)>,
    /// What the last tick pressed and why.
    why: moves::Explain,
}

/// The steps of a throw, after the two-handed axe throw as coaches teach it
/// (Sam, 2026-10-06: the throwers were "often thrown directly into the
/// floor"; "there should be a settling behavior or an arm watching
/// behavior"): stand square and still, bring the blade straight back over
/// the head, swing it forward with a step while watching it, and open the
/// hands in "the window", when the blade's flight from there meets the
/// target; then follow through. A release that comes late, with the blade
/// already moving down, is what sends it into the ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThrowStep {
    Settle,
    Wind,
    Watch,
    Follow,
}

impl ThrowStep {
    /// Its place among the throw's nodes in `view::tree_nodes`.
    pub fn index(self) -> usize {
        match self {
            ThrowStep::Settle => 0,
            ThrowStep::Wind => 1,
            ThrowStep::Watch => 2,
            ThrowStep::Follow => 3,
        }
    }
}

/// Settling ends when the thrower is this still, cm per tick, or after
/// `SETTLE_TICKS` however it stands.
const SETTLE_SPEED: i32 = 2;
const SETTLE_TICKS: u32 = 20;
/// The wind-up and the swing each give up after this long.
const WIND_TICKS: u32 = 30;
const SWING_TICKS: u32 = 30;
const FOLLOW_TICKS: u32 = 6;
/// The window: the blade's middle, flying on from where it is with gravity
/// and drag, passes this near the target's body before it is down, and
/// arrives fast enough to cut.
pub const THROW_WINDOW_CM: i32 = 30;
const ARRIVE_SPEED: i32 = 8;
const FLIGHT_TICKS: u32 = 120;

/// Whether `seat`'s held blade, let go now, would fly into its opponent's
/// body before it reaches the ground: the throw's "window". The thrower
/// feels its own arm in `w`, the world as it is now, as in every other
/// move; it aims at its opponent where it saw it, in `seen`, its reaction
/// time ago, so a slow thrower misses a moving target more often.
pub fn throw_window(w: &World, seen: &World, seat: usize) -> bool {
    let Some(k) = w.swords_of(seat).into_iter().find(|&k| w.held(k)) else { return false };
    let them = foe(seen, seat);
    let (Some(a), Some(b)) = (head(seen, them), pelvis(seen, them)) else { return false };
    let pts = &w.swords[k].points;
    let n = pts.len() as i64;
    let sum = |f: &dyn Fn(&sim::world::Particle) -> V2| pts.iter().fold(V2::ZERO, |acc, &i| acc + f(&w.particles[i as usize])).scale(1, n);
    let mut c = sum(&|p| p.p);
    let mut v = sum(&|p| p.p - p.q);
    let ph = w.setup.physics;
    let ab = b - a;
    for _ in 0..FLIGHT_TICKS {
        v -= v * ph.tuning.drag;
        v.y -= ph.gravity;
        c += v;
        if c.y.trunc() < 10 {
            return false;
        }
        let den = ab.len_sq_raw().max(1);
        let near = a + ab.scale((c - a).dot_raw(ab).clamp(0, den), den);
        if (c - near).len().trunc() < THROW_WINDOW_CM {
            return v.len().trunc() >= ARRIVE_SPEED;
        }
    }
    false
}

impl Tree {
    /// The throw, step by step; `None` when it is over or given up.
    fn throw_keys(&mut self, t: u32, w: &World, me: usize) -> Option<u16> {
        use Input as I;
        if t == 0 {
            self.throw = Some((ThrowStep::Settle, 0));
        }
        let (step, since) = self.throw?;
        let held = armed(w, me);
        if !held && step != ThrowStep::Follow {
            self.throw = None;
            return None;
        }
        let to = step_toward(w, me, true);
        let facing = facing(w, me);
        let (s, tip) = (shoulder(w, me)?, tip(w, me)?);
        let next = |this: &mut Tree, st: ThrowStep| this.throw = Some((st, t));
        Some(match step {
            ThrowStep::Settle => {
                let speed = |i: Option<V2>, j: Option<V2>| i.zip(j).map(|(x, y)| (x - y).len().trunc()).unwrap_or(0);
                let f = w.fighters[me].as_ref()?;
                let pe = f.base + w.setup.bodies[f.body as usize].roles.pelvis.unwrap_or(0) as u16;
                let moving = speed(Some(w.particles[pe as usize].p), Some(w.particles[pe as usize].q));
                let still = !airborne(w, me) && moving <= SETTLE_SPEED;
                if still || t - since >= SETTLE_TICKS {
                    next(self, ThrowStep::Wind);
                }
                0
            }
            ThrowStep::Wind => {
                // Back over the head: the tip above the shoulder and behind it.
                let over = (tip.y - s.y).trunc() > 40 && (tip.x - s.x).trunc() * facing < 0;
                if over || t - since >= WIND_TICKS {
                    next(self, ThrowStep::Watch);
                }
                I::SHOULDER_UP | I::ELBOW_OUT
            }
            ThrowStep::Watch => {
                let seen = self.seen.front().cloned().unwrap_or_else(|| w.clone());
                self.why.window = throw_window(w, &seen, me);
                if self.why.window {
                    next(self, ThrowStep::Follow);
                    return Some(I::SHOULDER_DOWN | I::ELBOW_OUT | I::THROW | to);
                }
                // Past the window: the tip in front of the thrower and
                // below its shoulder, or the swing taking too long. Keep
                // the blade rather than throw it at the ground.
                let past = (tip.y - s.y).trunc() < -20 && (tip.x - s.x).trunc() * facing > 0;
                if past || t - since >= SWING_TICKS {
                    self.throw = None;
                    return None;
                }
                I::SHOULDER_DOWN | I::ELBOW_OUT | to
            }
            ThrowStep::Follow => {
                if t - since >= FOLLOW_TICKS {
                    self.throw = None;
                    return None;
                }
                I::SHOULDER_DOWN | to
            }
        })
    }

    /// The step of the throw under way, if a throw is.
    pub fn throw_step(&self) -> Option<ThrowStep> {
        self.throw.map(|(s, _)| s)
    }
}

impl Tree {
    /// The move it is running and the tick within it, for a trace (lab
    /// roles); reading it changes nothing.
    pub fn current_move(&self) -> Option<(&str, u32)> {
        self.current.as_ref().map(|(n, t)| (n.as_str(), *t))
    }

    pub fn new(rules: Vec<Rule>, reaction: u32, salt: u64) -> Tree {
        let ids = view::tree_nodes(&rules, reaction).1;
        Tree { rules, reaction, seen: VecDeque::new(), rng: sim::rng::Rng::new(0x7EE ^ salt), current: None, search: None, ids, running: None, checked: Vec::new(), period: view::TREE_SEARCH.2, throw: None, why: moves::Explain::default() }
    }

    fn holds(&mut self, c: &Cond, w: &World, me: usize) -> bool {
        let them = foe(w, me);
        let near = |a: Option<V2>, b: Option<V2>, cm: i32| match (a, b) {
            (Some(a), Some(b)) => (a - b).len().trunc() < cm,
            _ => false,
        };
        let airborne = |s: usize| airborne(w, s);
        match c {
            Cond::GapAbove(cm) => gap(w, me) > *cm,
            Cond::GapBelow(cm) => gap(w, me) < *cm,
            Cond::TipNearHead(cm) => near(tip(w, them), head(w, me), *cm),
            Cond::TipNearBody(cm) => near(tip(w, them), pelvis(w, me), *cm),
            Cond::MeAirborne => airborne(me),
            Cond::MeGrounded => !airborne(me),
            Cond::OppAirborne => airborne(them),
            Cond::MeDown => w.knocked_down(me),
            Cond::Armed => armed(w, me),
            Cond::Unarmed => !armed(w, me),
            Cond::OppDown => w.knocked_down(them),
            Cond::OppDodging => w.dodging(them),
            Cond::MyInkBelow(pct) => w.fighters[me].as_ref().is_some_and(|f| {
                let max = w.setup.bodies[f.body as usize].ink.max(1);
                f.ink * 100 < max * pct
            }),
            Cond::Chance(pct) => self.rng.below(100) < *pct,
            Cond::AllyEngaged(cm) => (0..SEATS).any(|k| k != me && w.fighters[k].is_some() && w.side_of(k) == w.side_of(me) && !w.out(k) && {
                matches!((pelvis(w, k), pelvis(w, them)), (Some(a), Some(b)) if (a.x - b.x).abs().trunc() < *cm)
            }),
            Cond::OppAbove(cm) => matches!((pelvis(w, me), pelvis(w, them)), (Some(a), Some(b)) if (b.y - a.y).trunc() > *cm),
            Cond::OppBelow(cm) => matches!((pelvis(w, me), pelvis(w, them)), (Some(a), Some(b)) if (a.y - b.y).trunc() > *cm),
            // With room to spare: a pilot acts on what it saw some ticks ago, and
            // a climb begun at a ledge's very end came down beside it.
            Cond::LedgeOverhead => pelvis(w, me).is_some_and(|p| {
                let room = sim::fx::Fx::int(LEDGE_ROOM);
                w.setup.platforms.iter().any(|pl| p.x > pl.x0 + room && p.x < pl.x1 - room && pl.y > feet_height(w, me))
            }),
            Cond::OnLedge => on_ledge(w, me),
        }
    }

    fn pick(&mut self, w: &World, me: usize, interrupts_only: bool) -> Option<(usize, String)> {
        // Each condition is checked exactly as before, in order, stopping at
        // the first that fails; the trace only writes down what was found.
        self.checked.clear();
        for i in 0..self.rules.len() {
            let r = self.rules[i].clone();
            if interrupts_only && !r.interrupt {
                continue;
            }
            let mut all = true;
            for (j, c) in r.when.iter().enumerate() {
                let ok = self.holds(c, w, me);
                self.checked.push((self.ids[i].conds[j], ok));
                if !ok {
                    all = false;
                    break;
                }
            }
            if all {
                return Some((i, r.act));
            }
        }
        None
    }

    /// One tick of a move, or `None` when it has finished.
    fn play(&mut self, name: &str, t: u32, w: &World, me: usize) -> Option<u16> {
        // One table of moves (crate::moves), read here and by the BT Lab.
        let step = |st: moves::Step| match st {
            moves::Step::None => 0,
            moves::Step::Toward => step_toward(w, me, true),
            moves::Step::Away => step_toward(w, me, false),
        };
        Some(match moves::recipe(name)? {
            moves::Recipe::Script { beats } => {
                let (k, b) = beats.iter().enumerate().find(|(_, b)| b.from <= t && t <= b.to)?;
                self.why.beat = Some(k);
                self.why.step_keys = step(b.step);
                b.keys | self.why.step_keys
            }
            moves::Recipe::Pose { shoulder, elbow, ticks } => {
                if t > ticks {
                    return None;
                }
                let (k, why) = pose_why(w, me, shoulder, elbow);
                self.why.pose = Some(why);
                k
            }
            moves::Recipe::Search { ticks } => {
                if t > ticks {
                    return None;
                }
                // The search sees the world as old as the tree does: built
                // with a reaction of zero it read the present, and the
                // archivist won every match in about a second.
                let reaction = self.reaction;
                let s = self.search.get_or_insert_with(|| Search::new(view::TREE_SEARCH.0, reaction, view::TREE_SEARCH.1, self.period));
                let k = s.input(w, me).0;
                self.why.search = Some(s.why.clone());
                k
            }
            moves::Recipe::Throw => {
                let k = self.throw_keys(t, w, me)?;
                self.why.throw_step = self.throw_step().map(|st| st.index());
                k
            }
        })
    }

}

impl Pilot for Tree {
    fn trace(&self) -> view::Trace {
        let mut t = view::Trace::default();
        for &(id, ok) in &self.checked {
            if ok { t.held.push(id) } else { t.failed.push(id) }
        }
        if let Some(r) = self.running {
            let ids = &self.ids[r];
            t.active.push(0);
            if let Some(sq) = ids.seq {
                t.active.push(sq);
                t.active.extend(ids.conds.iter().copied());
            }
            t.active.push(ids.act);
            // A throw lights the step it is on (view::tree_nodes).
            if let (Some(step), false) = (self.throw_step(), ids.steps.is_empty()) {
                t.active.push(ids.steps[step.index()]);
            }
        }
        t
    }
    fn explain(&self) -> Vec<moves::Explain> {
        vec![self.why.clone()]
    }
    fn observe(&mut self, last: [Input; SEATS]) {
        if let Some(s) = self.search.as_mut() {
            s.observe(last);
        }
    }

    fn input(&mut self, w: &World, seat: usize) -> Input {
        self.why = moves::Explain::default();
        self.seen.push_back(w.clone());
        while self.seen.len() as u32 > self.reaction + 1 {
            self.seen.pop_front();
        }
        if let Some(i) = between(w) {
            self.current = None;
            self.running = None;
            return i;
        }
        // Decide on what was seen; act on the world as it is.
        let seen = self.seen.front().unwrap().clone();
        let interrupt = if self.current.is_some() { self.pick(&seen, seat, true) } else { None };
        if let Some((rule, name)) = interrupt {
            if self.current.as_ref().map(|(n, _)| n != &name).unwrap_or(true) {
                self.current = Some((name, 0));
                self.running = Some(rule);
            }
        }
        if self.current.is_none() {
            match self.pick(&seen, seat, false) {
                Some((rule, n)) => {
                    self.current = Some((n, 0));
                    self.running = Some(rule);
                }
                None => {
                    self.current = None;
                    self.running = None;
                }
            }
        }
        let Some((name, t)) = self.current.clone() else { return Input::NONE };
        match self.play(&name, t, w, seat) {
            Some(b) => {
                self.why.mv = Some(name.clone());
                self.why.t = t;
                self.why.keys = b;
                self.current = Some((name, t + 1));
                Input(b)
            }
            None => {
                self.current = None;
                self.running = None;
                Input::NONE
            }
        }
    }
}
