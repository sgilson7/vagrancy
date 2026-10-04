//! Opponents (D16). A pilot returns an `Input` and nothing else, so it holds
//! nothing a player cannot press. It reads the world as a player's eyes
//! would, and it is shown the other side's last input (`observe`), as a
//! person beside you sees what you do.
//!
//! No learned policy: a search baseline must be beaten before any network is
//! justified (gear-master/design/rl-agent-plan.md:53).
#![forbid(unsafe_code)]

use serde::Deserialize;
use sim::fight::Phase;
use sim::fx::{cos_deg, sin_deg, V2, ONE};
use sim::world::Owner;
use sim::{Input, World};
use std::collections::VecDeque;

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
    Machine { script: String, #[serde(default)] sword_len: Option<i32> },
    Replayer { first_round: String, mirror: bool },
    Search { horizon_ticks: u32, reaction_ticks: u32, branches: u32, #[serde(default = "default_period")] period: u32 },
    Tree { reaction_ticks: u32, rules: Vec<Rule>, #[serde(default)] salt: u64 },
}

fn default_period() -> u32 {
    6
}

pub trait Pilot {
    /// This tick's input for `seat`.
    fn input(&mut self, w: &World, seat: usize) -> Input;
    /// The other side's input last tick.
    fn observe(&mut self, _other: Input) {}
}

pub fn build(spec: &Spec) -> Box<dyn Pilot> {
    match spec.clone() {
        Spec::Still { .. } => Box::new(Still),
        Spec::Pose { shoulder, elbow } => Box::new(Pose { shoulder, elbow }),
        Spec::Loop { pattern, pause_ticks, drift } => Box::new(Looper::new(&pattern, pause_ticks, drift)),
        Spec::Machine { script, .. } => Box::new(Machine::new(&script)),
        Spec::Replayer { mirror, .. } => Box::new(Replayer { mirror, now: Vec::new(), last: Vec::new(), round: 0, t: 0 }),
        Spec::Search { horizon_ticks, reaction_ticks, branches, period } => {
            Box::new(Search::new(horizon_ticks, reaction_ticks, branches, period))
        }
        Spec::Tree { reaction_ticks, rules, salt } => Box::new(Tree::new(rules, reaction_ticks, salt)),
    }
}

// --- what a pilot can see ----------------------------------------------------------

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
    w.swords.iter().find(|s| s.fighter as usize == seat).map(|s| w.particles[s.tip as usize].p)
}

/// Horizontal distance between the two pelvises, in whole cm.
/// No foot still attached to the fighter is on the ground.
pub fn airborne(w: &World, seat: usize) -> bool {
    w.fighters[seat].as_ref().is_some_and(|f| {
        let def = &w.setup.bodies[f.body as usize];
        !def.roles.feet.iter().any(|&i| {
            let p = &w.particles[(f.base + i as u16) as usize];
            p.owner == Owner::Body(seat as u8) && p.p.y <= p.rad + ONE
        })
    })
}

pub fn gap(w: &World, seat: usize) -> i32 {
    match (pelvis(w, seat), pelvis(w, 1 - seat)) {
        (Some(a), Some(b)) => (b.x - a.x).abs().trunc(),
        _ => 0,
    }
}

/// The step bit that moves `seat` toward (or away from) the other fighter.
fn step_toward(w: &World, seat: usize, toward: bool) -> u16 {
    let (Some(a), Some(b)) = (pelvis(w, seat), pelvis(w, 1 - seat)) else { return 0 };
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
    let Some([(u, u0), (f, f0)]) = arm(w, seat) else { return 0 };
    let face = facing(w, seat) as i64;
    let want = V2::new(cos_deg(shoulder) * face as i32, sin_deg(shoulder));
    let steer = |err: i64, speed: i64, plus: u16, minus: u16| -> u16 {
        // Aim to close a tenth of the error each tick, at most 0.12 rad/tick.
        let aim = (err / 10).clamp(-120, 120);
        if speed < aim - 8 {
            plus
        } else if speed > aim + 8 {
            minus
        } else {
            0
        }
    };
    let mut b = 0;
    // Facing +x, counterclockwise is "up"; facing -x it is "down".
    let err = turn_milli(u, want) * face;
    let speed = turn_milli(u0, u) * face;
    b |= steer(err, speed, Input::SHOULDER_UP, Input::SHOULDER_DOWN);
    // The elbow's bend is the turn from the upper arm to the forearm.
    let bend = turn_milli(u, f) * face;
    let bend0 = turn_milli(u0, f0) * face;
    let target = (sin_deg(elbow).0 as i64 * 1000) / ONE.0 as i64;
    b |= steer(target - bend, bend - bend0, Input::ELBOW_IN, Input::ELBOW_OUT);
    b
}

// --- the kinds -------------------------------------------------------------------------

struct Still;
impl Pilot for Still {
    fn input(&mut self, w: &World, _: usize) -> Input {
        between(w).unwrap_or(Input::NONE)
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
}

/// A repeating sequence of keys, closing the distance when out of reach.
struct Looper {
    steps: Vec<(u16, u32)>,
    drift: bool,
    at: usize,
    left: u32,
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
        Looper { left: steps[0].1, steps, drift, at: 0 }
    }
}

impl Pilot for Looper {
    fn input(&mut self, w: &World, seat: usize) -> Input {
        if let Some(i) = between(w) {
            self.at = 0;
            self.left = self.steps[0].1;
            return i;
        }
        let (keys, _) = self.steps[self.at];
        self.left = self.left.saturating_sub(1);
        if self.left == 0 {
            self.at = (self.at + 1) % self.steps.len();
            self.left = self.steps[self.at].1;
        }
        // Close in to striking distance; the windmill drifts in regardless.
        let walk = if self.drift || gap(w, seat) > 200 { step_toward(w, seat, true) } else { 0 };
        Input(keys | walk)
    }
}

/// Short state machines for the ferryman and the vaulter.
struct Machine {
    script: String,
    t: u32,
    state: u8,
}

impl Machine {
    fn new(script: &str) -> Machine {
        Machine { script: script.into(), t: 0, state: 0 }
    }
}

impl Pilot for Machine {
    fn input(&mut self, w: &World, seat: usize) -> Input {
        if let Some(i) = between(w) {
            self.t = 0;
            self.state = 0;
            return i;
        }
        self.t += 1;
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
}

/// The sampler: still in the first round; after that, the previous round's
/// keys of the other side, mirrored.
struct Replayer {
    mirror: bool,
    now: Vec<Input>,
    last: Vec<Input>,
    round: u32,
    t: usize,
}

impl Pilot for Replayer {
    fn observe(&mut self, other: Input) {
        self.now.push(Input(other.0 & !Input::READY));
    }
    fn input(&mut self, w: &World, _: usize) -> Input {
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
    other: Input,
}

impl Search {
    pub fn new(horizon: u32, reaction: u32, branches: u32, period: u32) -> Search {
        Search { horizon, reaction, branches, period: period.max(1), seen: VecDeque::new(), chosen: Input::NONE, left: 0, other: Input::NONE }
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
        let them = 1 - seat;
        let mut s: i64 = 0;
        match w.phase {
            Phase::RoundOver { result, .. } | Phase::MatchOver { result } => match result.loser {
                Some(l) if l as usize == them => s += 100_000,
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
    fn observe(&mut self, other: Input) {
        self.other = Input(other.0 & !Input::READY);
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
            return self.chosen;
        }
        let base = self.seen.front().unwrap();
        let mut best = (i64::MIN, Input::NONE);
        for c in self.candidates(base, seat) {
            let mut trial = base.clone();
            let mut inputs = [Input::NONE; 2];
            inputs[seat] = c;
            inputs[1 - seat] = self.other;
            // Look ahead on the candidate. A cut that lands during the look-
            // ahead counts heavily, against the pilot when it is on itself:
            // without this the search stabbed toward the other head through
            // its own body and lost to a scarecrow that never moves
            // (`lab ladder 4`, SECOND-ORDER-M5).
            let mut cuts: i64 = 0;
            for _ in 0..self.horizon {
                trial.step(inputs);
                for e in &trial.events {
                    if let sim::fight::Event::Cut { seat: s, spilled: true, .. } = *e {
                        cuts += if s as usize == seat { -3_000 } else { 3_000 };
                    }
                }
                if !matches!(trial.phase, Phase::Fight) {
                    break;
                }
            }
            let s = Search::score(&trial, seat) + cuts;
            if s > best.0 {
                best = (s, c);
            }
        }
        self.chosen = best.1;
        self.left = self.period - 1;
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

/// Play a match between two pilots to its end, or to `max_ticks`.
pub fn duel(setup: sim::Setup, pilots: &mut [Box<dyn Pilot>; 2], max_ticks: u32) -> Outcome {
    let mut w = World::new(setup);
    let mut last = [Input::NONE; 2];
    while w.tick < max_ticks && !matches!(w.phase, Phase::MatchOver { .. }) {
        pilots[0].observe(last[1]);
        pilots[1].observe(last[0]);
        let i = [pilots[0].input(&w, 0), pilots[1].input(&w, 1)];
        w.step(i);
        last = i;
    }
    Outcome { wins: w.wins, ticks: w.tick, finished: matches!(w.phase, Phase::MatchOver { .. }) }
}

/// For the page and the tests: the integer value placeholders are filled
/// with, from a pilot's data (D16). `{pause_s}` and the rest are derived
/// here, never typed into a sentence.
pub fn numbers(spec: &Spec, default_sword_len: i32) -> Vec<(&'static str, String)> {
    let ms = |ticks: u32| (ticks * 1000).div_ceil(sim::balance::TICKS_PER_SECOND);
    match spec {
        Spec::Loop { pause_ticks, .. } if *pause_ticks > 0 => {
            let tenths = (pause_ticks * 10).div_ceil(sim::balance::TICKS_PER_SECOND);
            vec![("pause_s", if tenths % 10 == 0 { format!("{}", tenths / 10) } else { format!("{}.{}", tenths / 10, tenths % 10) })]
        }
        Spec::Machine { sword_len: Some(l), .. } => {
            let pct = ((l - default_sword_len) * 100 + default_sword_len / 2) / default_sword_len;
            vec![("reach_pct", pct.to_string())]
        }
        Spec::Search { horizon_ticks, reaction_ticks, .. } => {
            vec![("horizon_ms", ms(*horizon_ticks).to_string()), ("reaction_ms", ms(*reaction_ticks).to_string())]
        }
        Spec::Tree { reaction_ticks, .. } => vec![("reaction_ms", ms(*reaction_ticks).to_string())],
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
}

impl Tree {
    pub fn new(rules: Vec<Rule>, reaction: u32, salt: u64) -> Tree {
        Tree { rules, reaction, seen: VecDeque::new(), rng: sim::rng::Rng::new(0x7EE ^ salt), current: None, search: None }
    }

    fn holds(&mut self, c: &Cond, w: &World, me: usize) -> bool {
        let them = 1 - me;
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
            Cond::OppDodging => w.dodging(them),
            Cond::MyInkBelow(pct) => w.fighters[me].as_ref().is_some_and(|f| {
                let max = w.setup.bodies[f.body as usize].ink.max(1);
                f.ink * 100 < max * pct
            }),
            Cond::Chance(pct) => self.rng.below(100) < *pct,
        }
    }

    fn pick(&mut self, w: &World, me: usize, interrupts_only: bool) -> Option<String> {
        for i in 0..self.rules.len() {
            let r = self.rules[i].clone();
            if interrupts_only && !r.interrupt {
                continue;
            }
            if r.when.iter().all(|c| self.holds(c, w, me)) {
                return Some(r.act);
            }
        }
        None
    }

    /// One tick of a move, or `None` when it has finished.
    fn play(&mut self, name: &str, t: u32, w: &World, me: usize) -> Option<u16> {
        let to = step_toward(w, me, true);
        let away = step_toward(w, me, false);
        use Input as I;
        Some(match (name, t) {
            ("approach", 0..=7) => to,
            ("retreat", 0..=7) => away,
            ("guard", 0..=9) => pose_keys(w, me, 10, 10),
            ("high_guard", 0..=9) => pose_keys(w, me, 70, 20),
            ("low_guard", 0..=9) => pose_keys(w, me, -30, 10),
            ("overhead", 0..=15) => I::SHOULDER_UP,
            ("overhead", 16..=33) => I::SHOULDER_DOWN | I::ELBOW_OUT,
            ("low_sweep", 0..=19) => I::SHOULDER_DOWN | to,
            ("thrust", 0..=7) => I::ELBOW_IN,
            ("thrust", 8..=17) => I::ELBOW_OUT | to,
            ("spin", 0..=23) => I::SHOULDER_UP | to,
            ("jump_strike", 0) => I::JUMP | to,
            ("jump_strike", 1..=9) => I::SHOULDER_UP | to,
            ("jump_strike", 10..=29) => I::SHOULDER_DOWN | to,
            ("bounce_strike", 0) => I::JUMP,
            ("bounce_strike", 1..=13) => I::SHOULDER_UP,
            ("bounce_strike", 14) => I::JUMP | to,
            ("bounce_strike", 15..=35) => I::SHOULDER_DOWN | to,
            ("dodge_away", 0) => I::DODGE | away,
            ("dodge_away", 1..=17) => 0,
            ("dodge_in", 0) => I::DODGE | to,
            ("dodge_in", 1..=17) => 0,
            ("dodge_in", 18..=35) => I::SHOULDER_DOWN | I::ELBOW_OUT,
            ("pogo", 0..=12) => I::ELBOW_IN | to,
            ("pogo", 13..=21) => to,
            ("pogo", 22..=55) => I::SHOULDER_DOWN | I::ELBOW_OUT | to,
            ("stand", 0) => I::STAND,
            ("wait", 0..=9) => 0,
            ("search", 0..=11) => {
                // The search sees the world as old as the tree does: built
                // with a reaction of zero it read the present, and the
                // archivist won every match in about a second.
                let reaction = self.reaction;
                let s = self.search.get_or_insert_with(|| Search::new(18, reaction, 11, 3));
                s.input(w, me).0
            }
            _ => return None,
        })
    }
}

impl Pilot for Tree {
    fn observe(&mut self, other: Input) {
        if let Some(s) = self.search.as_mut() {
            s.observe(other);
        }
    }

    fn input(&mut self, w: &World, seat: usize) -> Input {
        self.seen.push_back(w.clone());
        while self.seen.len() as u32 > self.reaction + 1 {
            self.seen.pop_front();
        }
        if let Some(i) = between(w) {
            self.current = None;
            return i;
        }
        // Decide on what was seen; act on the world as it is.
        let seen = self.seen.front().unwrap().clone();
        let interrupt = if self.current.is_some() { self.pick(&seen, seat, true) } else { None };
        if let Some(name) = interrupt {
            if self.current.as_ref().map(|(n, _)| n != &name).unwrap_or(true) {
                self.current = Some((name, 0));
            }
        }
        if self.current.is_none() {
            self.current = self.pick(&seen, seat, false).map(|n| (n, 0));
        }
        let Some((name, t)) = self.current.clone() else { return Input::NONE };
        match self.play(&name, t, w, seat) {
            Some(b) => {
                self.current = Some((name, t + 1));
                Input(b)
            }
            None => {
                self.current = None;
                Input::NONE
            }
        }
    }
}
