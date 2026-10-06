//! The world, and the one door it changes through: `World::step`.
//!
//! Particles joined by rigid sticks, moved by position Verlet and relaxed a
//! fixed number of times in a fixed order (D4; Jakobsen, "Advanced Character
//! Physics", GDC 2001). Velocity is implicit, `p - q`.
//!
//! **Every correction inside a fighter is an exact pair shift.** When a
//! constraint moves point `a` by `k·m_b` it moves point `b` by `-k·m_a`,
//! with the same integer `k`, so `m_a·Δa + m_b·Δb` is zero to the last raw
//! unit. Sticks, pins, hinges and the balance rule all work this
//! way. The arm motor does not: it pushes the arm and the sword and nothing
//! pushes back on the torso. That is the free-energy rule (D9), and because
//! nothing else creates momentum, H5 and its control H5c can show it.

use crate::balance::{self, ITERATIONS};
use crate::body::{BodyDef, Motor, Setup, SEATS};
use crate::fx::{Fx, ONE, V2};
use crate::input::Input;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

/// Who a particle belongs to. A cut moves particles from a body to a piece.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Owner {
    Body(u8),
    Sword(u8),
    Piece(u16),
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Particle {
    pub p: V2,
    /// Where it was last tick. `p - q` is its velocity.
    pub q: V2,
    /// Zero means anchored: nothing moves it.
    pub m: i32,
    pub rad: Fx,
    pub grip: Fx,
    /// A foot: the point the ground drives when the step keys are held.
    pub foot: bool,
    pub owner: Owner,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Part {
    /// Index into the fighter's `BodyDef::parts`.
    pub def: u8,
    pub fighter: u8,
    pub a: u16,
    pub b: u16,
    pub len: Fx,
    pub radius: Fx,
    pub mass: i32,
    /// Still part of its fighter's body, rather than a dropped piece.
    pub attached: bool,
    /// The cut end of an attached part, draining ink (D12).
    pub stump: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Con {
    Stick { a: u16, b: u16, len: Fx },
    /// Pushes apart only: `a` and `b` stay at least `len` apart.
    Min { a: u16, b: u16, len: Fx },
    /// `a` stays at the point `at` of the way from `b` to `c`, moving `b`,
    /// `c` and the `extra` points after `c` together (a weapon's own points).
    Pin { a: u16, b: u16, c: u16, at: Fx, extra: u8 },
    /// `j→c` may not bend past straight the wrong way from `a→j`.
    Hinge { a: u16, j: u16, c: u16, sign: i8 },
}

/// Why a constraint exists, which decides when a cut removes it.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Tag {
    /// Holds a fighter's shape.
    Body(u8),
    /// The sword's own stick.
    Sword(u8),
    /// A hand on its fighter's sword, and the stiff wrist that goes with it.
    Grip { fighter: u8, hand: u16 },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Constraint {
    pub con: Con,
    pub tag: Tag,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Fighter {
    pub seat: u8,
    /// 0 for seat 0, 1 for its opponents.
    pub side: u8,
    pub body: u8,
    /// +1 faces +x (the left seat), -1 faces -x.
    pub facing: i32,
    /// This fighter's points are `base .. base + n` in `particles`.
    pub base: u16,
    pub n: u16,
    pub sword: Option<u8>,
    pub ink: i32,
    /// How fast, and which way, the ground carries this fighter this tick, in
    /// cm per tick: the step keys at the run speed, or a roll (D8, as Sam
    /// revised it on 2026-10-03).
    pub drive: Fx,
    /// Jumps left before landing: one in the air (Sam, 2026-10-03).
    pub air_jumps: u8,
    /// Ticks left in a dodge (uncuttable, its own blade harmless), and
    /// before the next dodge may start.
    pub dodge: u8,
    pub dodge_cooldown: u8,
    pub dodge_dir: i32,
    /// Last tick's jump and dodge bits, so a held key acts once.
    pub held: u16,
    /// Rest distances the balance rule restores: shoulder over pelvis, head
    /// over shoulder.
    pub torso: Fx,
    pub neck: Fx,
    /// Ink drained by each of this body's parts (by `PartDef` index) this
    /// round, including stumps since removed. `results.road.lose_ink` names
    /// the part that spilled the most.
    pub spilled: Vec<i32>,
    /// The tick this fighter went out of the round, once it has.
    pub out_at: Option<u32>,
    /// A cut this tick that ends the round for this fighter.
    pub fatal: Option<crate::fight::Fatal>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Sword {
    pub fighter: u8,
    pub butt: u16,
    pub tip: u16,
    pub len: Fx,
    pub hilt: Fx,
    /// Every point of the weapon: the butt, the tip, then its extra points,
    /// in that order and contiguous.
    pub points: Vec<u16>,
    /// The cutting edges, as particle indices and where along `a`→`b` each
    /// begins to cut.
    pub edges: Vec<(u16, u16, Fx)>,
    /// Thrown and not yet down: it cuts though no hand holds it, until any
    /// point of it first touches the ground.
    pub flying: bool,
}

/// A held joint key this tick: the servo's target and who it turns.
/// Transient: built at the start of `step` and dropped at its end.
#[derive(Clone, Debug)]
pub(crate) struct Drive {
    seat: usize,
    pivot: u16,
    limb: u16,
    set: Vec<u16>,
    dir: i32,
    /// The angular speed the joint should reach this tick, rad/tick.
    target: Fx,
}

/// The whole state of a match. `checksum()` hashes all of it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct World {
    pub setup: Setup,
    pub tick: u32,
    pub rng: Rng,
    pub particles: Vec<Particle>,
    pub parts: Vec<Part>,
    pub cons: Vec<Constraint>,
    pub fighters: [Option<Fighter>; SEATS],
    pub swords: Vec<Sword>,
    pub phase: crate::fight::Phase,
    /// 1-based. A drawn round is played again under the same number.
    pub round: u32,
    /// Ticks fought in the round under way.
    pub round_ticks: u32,
    pub wins: [u32; 2],
    /// Blade and part pairs in contact at the end of last tick: one cut per
    /// blade, per part, per contact (D11).
    pub touching: Vec<(u8, u16)>,
    /// Pairs of swords, by index, in contact at the end of last tick.
    pub clashing: Vec<(u8, u8)>,
    /// The tick since which no fighter in the round has had a blade, held
    /// or in the air; `None` while one has.
    pub disarmed_since: Option<u32>,
    /// What happened this tick, for the page to draw and later to sound.
    pub events: Vec<crate::fight::Event>,
    pub next_piece: u16,
}

impl World {
    pub fn new(setup: Setup) -> World {
        let mut w = World {
            rng: Rng::new(setup.seed),
            setup,
            tick: 0,
            particles: Vec::new(),
            parts: Vec::new(),
            cons: Vec::new(),
            fighters: [None, None, None],
            swords: Vec::new(),
            phase: crate::fight::Phase::Fight,
            round: 1,
            round_ticks: 0,
            wins: [0, 0],
            touching: Vec::new(),
            clashing: Vec::new(),
            disarmed_since: None,
            events: Vec::new(),
            next_piece: 0,
        };
        w.spawn_round();
        w
    }

    /// Clear the arena and stand both seats up. One jitter draw per round,
    /// applied mirrored so neither seat is favored.
    pub(crate) fn spawn_round(&mut self) {
        self.touching.clear();
        self.clashing.clear();
        self.disarmed_since = None;
        self.round_ticks = 0;
        self.next_piece = 0;
        self.particles.clear();
        self.parts.clear();
        self.cons.clear();
        self.swords.clear();
        self.fighters = [None, None, None];
        let jitter = Fx::int(self.rng.range(-balance::SPAWN_JITTER_CM, balance::SPAWN_JITTER_CM));
        for seat in 0..SEATS as u8 {
            if let Some(s) = self.setup.seats[seat as usize] {
                let facing = s.facing.map(i32::from).unwrap_or(crate::body::facing(seat as usize));
                let x = (s.x + jitter) * -facing;
                let side = s.side.unwrap_or(crate::body::side(seat as usize));
                self.spawn(seat, s.body, facing, x, side);
            }
        }
    }

    fn spawn(&mut self, seat: u8, body: u8, facing: i32, x: Fx, side: u8) {
        let def: BodyDef = self.setup.bodies[body as usize].clone();
        let base = self.particles.len() as u16;
        let place = |at: V2| V2::new(x + at.x * facing, at.y);
        let mut mass = vec![0i32; def.points.len()];
        for part in &def.parts {
            mass[part.near as usize] += part.mass;
            mass[part.far as usize] += part.mass;
        }
        for (i, pt) in def.points.iter().enumerate() {
            let p = place(pt.at);
            let m = if def.anchored.contains(&(i as u8)) { 0 } else { mass[i].max(1) };
            let grip = if def.roles.feet.contains(&(i as u8)) { balance::GRIP_FOOT } else { balance::GRIP_BODY };
            let foot = def.roles.feet.contains(&(i as u8));
            self.particles.push(Particle { p, q: p, m, rad: pt.rad, grip, foot, owner: Owner::Body(seat) });
        }
        let at = |i: u8| def.points[i as usize].at;
        for (k, part) in def.parts.iter().enumerate() {
            self.parts.push(Part {
                def: k as u8,
                fighter: seat,
                a: base + part.near as u16,
                b: base + part.far as u16,
                len: (at(part.far) - at(part.near)).len(),
                radius: part.radius,
                mass: part.mass,
                attached: true,
                stump: false,
            });
        }
        for &(a, b) in &def.sticks {
            let len = (at(b) - at(a)).len();
            self.cons.push(Constraint { con: Con::Stick { a: base + a as u16, b: base + b as u16, len }, tag: Tag::Body(seat) });
        }
        for h in &def.hinges {
            let (a, j, c) = (base + h.a as u16, base + h.j as u16, base + h.c as u16);
            // The facing is folded into the sign here, once, so a hinge that a
            // cut drops into a piece still bends the way it did (a piece has
            // no fighter to ask; asking broke the mirror, SECOND-ORDER-M3).
            let sign = h.sign * facing as i8;
            self.cons.push(Constraint { con: Con::Hinge { a, j, c, sign }, tag: Tag::Body(seat) });
            self.cons.push(Constraint { con: Con::Min { a, b: c, len: h.min_dist }, tag: Tag::Body(seat) });
        }
        let mut sword = None;
        if let Some(sd) = &def.sword {
            let si = self.swords.len() as u8;
            let butt = self.particles.len() as u16;
            for end in [sd.butt, sd.tip] {
                let p = place(end);
                let grip = if end == sd.tip { balance::GRIP_TIP } else { balance::GRIP_BODY };
                self.particles.push(Particle { p, q: p, m: sd.mass, rad: Fx(0), grip, foot: false, owner: Owner::Sword(seat) });
            }
            let tip = butt + 1;
            let len = (sd.tip - sd.butt).len();
            self.cons.push(Constraint { con: Con::Stick { a: butt, b: tip, len }, tag: Tag::Sword(si) });
            // A weapon's other points, each held to the butt and the tip.
            for &e in &sd.extra {
                let i = self.particles.len() as u16;
                let p = place(e);
                self.particles.push(Particle { p, q: p, m: sd.mass, rad: Fx(0), grip: balance::GRIP_TIP, foot: false, owner: Owner::Sword(seat) });
                for (end, at_end) in [(butt, sd.butt), (tip, sd.tip)] {
                    self.cons.push(Constraint { con: Con::Stick { a: end, b: i, len: (e - at_end).len() }, tag: Tag::Sword(si) });
                }
            }
            let points: Vec<u16> = (butt..self.particles.len() as u16).collect();
            let edges = if sd.edges.is_empty() {
                vec![(butt, tip, Fx::ratio(sd.hilt.0 as i64, len.0.max(1) as i64))]
            } else {
                sd.edges.iter().map(|e| (butt + e.a as u16, butt + e.b as u16, e.from)).collect()
            };
            self.swords.push(Sword { fighter: seat, butt, tip, len, hilt: sd.hilt, points, edges, flying: false });
            let extra = sd.extra.len() as u8;
            for g in &sd.grips {
                let hand = base + g.hand as u16;
                self.cons.push(Constraint { con: Con::Pin { a: hand, b: butt, c: tip, at: g.at, extra }, tag: Tag::Grip { fighter: seat, hand } });
                if let Some(s) = g.stiff {
                    let len = (sd.tip - at(s)).len();
                    self.cons.push(Constraint { con: Con::Stick { a: base + s as u16, b: tip, len }, tag: Tag::Grip { fighter: seat, hand } });
                }
            }
            sword = Some(si);
        }
        let r = &def.roles;
        let dist = |a: Option<u8>, b: Option<u8>| match (a, b) {
            (Some(a), Some(b)) => (at(a) - at(b)).len(),
            _ => Fx(0),
        };
        let n = self.particles.len() as u16 - base;
        self.fighters[seat as usize] = Some(Fighter {
            seat,
            side,
            body,
            facing,
            base,
            n,
            sword,
            ink: def.ink,
            drive: Fx(0),
            air_jumps: 1,
            dodge: 0,
            dodge_cooldown: 0,
            dodge_dir: 0,
            held: 0,
            torso: dist(r.shoulder, r.pelvis),
            neck: dist(r.head, r.shoulder),
            spilled: vec![0; def.parts.len()],
            fatal: None,
            out_at: None,
        });
    }

    /// One tick of a fight between two seats; the third, if any, presses
    /// nothing.
    pub fn step(&mut self, inputs: [Input; 2]) {
        self.step_all([inputs[0], inputs[1], Input::NONE]);
    }

    /// The only way the world changes.
    pub fn step_all(&mut self, inputs: [Input; SEATS]) {
        self.events.clear();
        // Between rounds the fighters are limp and only "ready" counts.
        let inputs = if self.between_rounds(inputs) { [Input::NONE; SEATS] } else { inputs };
        let mut inputs = inputs;
        for seat in 0..SEATS {
            // A fighter already out of the round goes limp until it ends.
            if self.out(seat) {
                inputs[seat] = Input::NONE;
            }
            if self.fighters[seat].is_some() {
                self.throw_sword(seat, inputs[seat]);
                inputs[seat] = self.jump_and_dodge(seat, inputs[seat]);
            }
        }
        let mut acc = vec![V2::ZERO; self.particles.len()];
        let mut drives = Vec::new();
        for seat in 0..SEATS {
            if self.fighters[seat].is_some() {
                drives.extend(self.drive_motors(seat, inputs[seat], &mut acc));
                self.set_drive(seat, inputs[seat]);
            }
        }
        self.integrate(&acc);
        for _ in 0..ITERATIONS {
            self.relax(&drives);
        }
        self.clash(&drives);
        self.cuts();
        self.land_thrown();
        self.hold_to_cap();
        self.drain();
        self.judge();
        if matches!(self.phase, crate::fight::Phase::Fight) {
            self.round_ticks += 1;
        }
        self.tick += 1;
    }

    /// 64-bit FNV-1a over the `postcard` encoding of the whole world, so a
    /// field is covered the day it is added (Floodline `world.rs:777-784`).
    pub fn checksum(&self) -> u64 {
        fnv1a(&postcard::to_allocvec(self).expect("a world always encodes"))
    }

    // --- the motors (D9) -----------------------------------------------------

    /// Which particles a fighter's input would push, and about which pivot.
    /// Empty for every input that holds no arm bit. Public so that
    /// `only_arm_bits_drive_a_joint_motor` reads the same function `step` uses.
    pub fn motor_plan(&self, seat: usize, input: Input) -> Vec<(Motor, i32, u16, Vec<u16>)> {
        let Some(f) = &self.fighters[seat] else { return Vec::new() };
        let def = &self.setup.bodies[f.body as usize];
        let mut out = Vec::new();
        for (pi, part) in self.parts.iter().enumerate() {
            if part.fighter as usize != seat || !part.attached {
                continue;
            }
            let Some(motor) = def.parts[part.def as usize].motor else { continue };
            let axis = match motor {
                Motor::Shoulder => input.axis(Input::SHOULDER_UP, Input::SHOULDER_DOWN),
                Motor::Elbow => input.axis(Input::ELBOW_IN, Input::ELBOW_OUT),
            };
            if axis == 0 {
                continue;
            }
            out.push((motor, axis * f.facing, part.a, self.distal(pi)));
        }
        out
    }

    /// Every particle beyond part `pi`, and its fighter's sword if a hand in
    /// that set is holding it.
    fn distal(&self, pi: usize) -> Vec<u16> {
        let root = &self.parts[pi];
        let mut set = vec![root.b];
        let mut grew = true;
        while grew {
            grew = false;
            for (k, part) in self.parts.iter().enumerate() {
                if k != pi && part.attached && part.fighter == root.fighter && set.contains(&part.a) && !set.contains(&part.b) {
                    set.push(part.b);
                    grew = true;
                }
            }
        }
        for c in &self.cons {
            if let (Con::Pin { a, b, c: tip, extra, .. }, Tag::Grip { fighter, .. }) = (c.con, c.tag) {
                if fighter == root.fighter && set.contains(&a) && !set.contains(&b) {
                    set.push(b);
                    set.push(tip);
                    set.extend((0..extra as u16).map(|k| tip + 1 + k));
                }
            }
        }
        set
    }

    fn drive_motors(&self, seat: usize, input: Input, acc: &mut [V2]) -> Vec<Drive> {
        let mut drives = Vec::new();
        let t = self.setup.physics.tuning;
        let facing = self.fighters[seat].as_ref().map(|f| f.facing).unwrap_or(1);
        for (motor, dir, pivot, set) in self.motor_plan(seat, input) {
            let pv = &self.particles[pivot as usize];
            let limb = set[0] as usize;
            let r = self.particles[limb].p - pv.p;
            let r_prev = self.particles[limb].q - pv.q;
            let len_sq = r.len_sq_raw();
            if len_sq == 0 {
                continue;
            }
            // Angular speed of the limb about its joint, rad/tick as Fx.
            let omega = Fx(crate::fx::narrow(r_prev.cross_raw(r) * ONE.0 as i64 / len_sq));
            // A servo, not a constant push: the joint is driven as hard as
            // `motor_accel` allows until it turns at `motor_speed`. A free
            // swing tops out at that speed; a blocked one (a planted tip)
            // keeps pushing at full strength, which is what lifts a pogo.
            // M2.0 found a constant push about twenty times too weak to lift
            // the body (SECOND-ORDER-M2).
            let room = t.motor_speed - omega * dir;
            if room.0 <= 0 {
                continue;
            }
            // An elbow that is already straight does not push past straight.
            if motor == Motor::Elbow && dir * facing < 0 && self.elbow_is_straight(pivot, set[0], facing) {
                continue;
            }
            let a = room.min(t.motor_accel) * dir;
            for &k in &set {
                let arm = self.particles[k as usize].p - pv.p;
                acc[k as usize] += arm.perp() * a;
            }
            drives.push(Drive { seat, pivot, limb: set[0], set, dir, target: omega * dir + room.min(t.motor_accel) });
        }
        drives
    }

    /// A held joint that is blocked — the tip planted, the blade against the
    /// other blade — makes up what it could not turn as an exact pair shift
    /// against the rest of its body. Free, the push above has already reached
    /// the target and this does nothing, so a swing in the air still adds
    /// momentum from nowhere (H5). Blocked by the ground, the body is pushed
    /// instead, and the ground pushes back: "The sword cannot go into the
    /// ground, so the push lifts your fighter instead" (practice.step.plant).
    fn drive_joint(&mut self, d: &Drive) {
        let pv = &self.particles[d.pivot as usize];
        let lb = &self.particles[d.limb as usize];
        let r = lb.p - pv.p;
        let r0 = lb.q - pv.q;
        let len_sq = r.len_sq_raw();
        if len_sq == 0 {
            return;
        }
        let omega = Fx(crate::fx::narrow(r0.cross_raw(r) * ONE.0 as i64 / len_sq)) * d.dir;
        let short = d.target - omega;
        if short.0 <= 0 {
            return;
        }
        let turn = (short * balance::DRIVE_K) * d.dir;
        let pivot = self.particles[d.pivot as usize].p;
        // The far side turns by `turn` about the pivot; the near side (every
        // other attached point of the body) moves together the other way, so
        // that Σ m·Δp is zero to the raw unit.
        let near: Vec<u16> = (0..self.particles.len() as u16)
            .filter(|&i| self.particles[i as usize].owner == Owner::Body(d.seat as u8) && !d.set.contains(&i))
            .collect();
        let m_near: i32 = near.iter().map(|&i| self.particles[i as usize].m).sum();
        if m_near == 0 {
            return;
        }
        let mut moved = (0i64, 0i64);
        for &k in &d.set {
            let pt = &mut self.particles[k as usize];
            let step = (pt.p - pivot).perp() * turn;
            pt.p += step;
            moved.0 += pt.m as i64 * step.x.0 as i64;
            moved.1 += pt.m as i64 * step.y.0 as i64;
        }
        // Spread −moved over the near side by mass, in whole raw units per unit
        // mass, and give the rounding remainder back to the far side's first
        // point so the sum is exact.
        let kx = moved.0 / m_near as i64;
        let ky = moved.1 / m_near as i64;
        for &i in &near {
            let pt = &mut self.particles[i as usize];
            pt.p.x.0 -= crate::fx::narrow(kx);
            pt.p.y.0 -= crate::fx::narrow(ky);
        }
        let first = d.set[0] as usize;
        let m_first = self.particles[first].m as i64;
        let (rx, ry) = (moved.0 - kx * m_near as i64, moved.1 - ky * m_near as i64);
        if m_first > 0 {
            // Whatever does not divide evenly stays unbalanced by under one raw
            // unit of the first point's travel; record it there.
            self.particles[first].p.x.0 -= crate::fx::narrow(rx / m_first);
            self.particles[first].p.y.0 -= crate::fx::narrow(ry / m_first);
        }
    }

    fn elbow_is_straight(&self, elbow: u16, wrist: u16, facing: i32) -> bool {
        // The upper arm is the attached part whose far end is the elbow.
        let Some(upper) = self.parts.iter().find(|p| p.attached && p.b == elbow) else { return false };
        let u = self.particles[elbow as usize].p - self.particles[upper.a as usize].p;
        let f = self.particles[wrist as usize].p - self.particles[elbow as usize].p;
        u.cross_raw(f) * facing as i64 <= 0
    }

    // --- the legs (D8) ---------------------------------------------------------

    /// Whether a point stands on something: the ground, or the top of a
    /// platform it came down onto.
    pub fn supported(&self, i: u16) -> bool {
        let p = &self.particles[i as usize];
        if self.setup.physics.ground && p.p.y <= p.rad + ONE {
            return true;
        }
        p.foot && self.setup.platforms.iter().any(|pl| {
            let top = pl.y + p.rad;
            p.p.x >= pl.x0 && p.p.x <= pl.x1 && p.p.y <= top + ONE && p.p.y >= top - ONE && p.q.y >= top - ONE
        })
    }

    fn grounded(&self, i: u16) -> bool {
        self.supported(i)
    }

    /// What a fighter's feet stand on: the highest ledge top at or below its
    /// lowest attached foot, under its pelvis, or the ground. Measured from
    /// the pelvis instead, a fighter standing on the ground under a ledge
    /// lower than its pelvis counted as knocked over (row 58).
    pub fn floor_of(&self, seat: usize) -> Fx {
        let feet = self.feet(seat);
        let Some(pe) = self.role(seat, |r| r.pelvis) else { return Fx(0) };
        let low = feet.iter().map(|&i| self.particles[i as usize].p.y - self.particles[i as usize].rad).min();
        match low {
            Some(y) => self.surface_below(self.particles[pe as usize].p.x, y + ONE),
            None => Fx(0),
        }
    }

    /// The height of the surface under `x` at or below `y`: the highest
    /// platform top there, or the ground.
    pub fn surface_below(&self, x: Fx, y: Fx) -> Fx {
        self.setup.platforms.iter().filter(|pl| x >= pl.x0 && x <= pl.x1 && pl.y <= y).map(|pl| pl.y).fold(Fx(0), |a, b| a.max(b))
    }

    /// The attached feet of a fighter, in the order the body lists them.
    fn feet(&self, seat: usize) -> Vec<u16> {
        let Some(f) = &self.fighters[seat] else { return Vec::new() };
        let def = &self.setup.bodies[f.body as usize];
        def.roles.feet.iter().map(|&i| f.base + i as u16).filter(|&i| self.particles[i as usize].owner == Owner::Body(seat as u8)).collect()
    }

    fn role(&self, seat: usize, pick: fn(&crate::body::Roles) -> Option<u8>) -> Option<u16> {
        let f = self.fighters[seat].as_ref()?;
        let i = f.base + pick(&self.setup.bodies[f.body as usize].roles)? as u16;
        (self.particles[i as usize].owner == Owner::Body(seat as u8)).then_some(i)
    }

    /// Whether a fighter went out of the round (a fatal cut, or no ink) on
    /// an earlier tick, as the judge recorded it. With two seats the round
    /// ends that same tick, so this is never true while they fight; with
    /// three, an opponent who is out goes limp while its partner fights on.
    /// Read from the judge's record, not the fatal cut itself, so two
    /// fighters cut in one tick still both cut (a draw), as before.
    pub fn out(&self, seat: usize) -> bool {
        let Some(f) = &self.fighters[seat] else { return false };
        matches!(self.phase, crate::fight::Phase::Fight) && f.out_at.is_some()
    }

    /// The balance rule holds a fighter up while it stands on a foot.
    fn balanced(&self, seat: usize) -> bool {
        let Some(f) = &self.fighters[seat] else { return false };
        !self.out(seat)
            && self.setup.bodies[f.body as usize].balance
            && self.role(seat, |r| r.pelvis).is_some()
            && self.role(seat, |r| r.shoulder).is_some()
            && self.feet(seat).iter().any(|&i| self.grounded(i))
    }

    /// The step keys ask the ground to carry the fighter (D8, revised by
    /// Sam: rigid legs and quick, Nidhogg-like movement; the arms stay fully
    /// simulated). They drive no joint (PLAN.md D6, Q9): the drive acts only
    /// where a foot touches the ground, in `bounds`, so a fighter in the
    /// air cannot run.
    fn set_drive(&mut self, seat: usize, input: Input) {
        let balanced = self.balanced(seat);
        if let Some(f) = self.fighters[seat].as_mut() {
            f.drive = if !balanced {
                Fx(0)
            } else if f.dodge > 0 {
                balance::ROLL_SPEED * f.dodge_dir
            } else {
                balance::RUN_SPEED * input.axis(Input::STEP_RIGHT, Input::STEP_LEFT)
            };
        }
    }

    // --- the throw (Sam's friend, 2026-10-05) ---------------------------------------

    /// On the tick the throw key goes down, a fighter holding its sword lets
    /// go: the grips go, and the sword flies on with the velocity the swing
    /// gave it. Read before `jump_and_dodge`, which records the held keys.
    fn throw_sword(&mut self, seat: usize, input: Input) {
        let Some(f) = &self.fighters[seat] else { return };
        let pressed = input.has(Input::THROW) && f.held & Input::THROW == 0;
        if !pressed || !matches!(self.phase, crate::fight::Phase::Fight) {
            return;
        }
        let gripped = |c: &Constraint| matches!(c.tag, Tag::Grip { fighter, .. } if fighter as usize == seat);
        if !self.cons.iter().any(gripped) {
            return;
        }
        self.cons.retain(|c| !gripped(c));
        if let Some(s) = self.swords.iter_mut().find(|s| s.fighter as usize == seat) {
            s.flying = true;
        }
    }

    /// A thrown sword that has touched the ground is down: from then on it
    /// cuts nothing (the friend's rule: "once its hit the ground, then its
    /// no longer dangerous"). Read after `cuts`, so a blade that lands in a
    /// body on the tick it lands still cuts it.
    fn land_thrown(&mut self) {
        if !self.setup.physics.ground {
            return;
        }
        for k in 0..self.swords.len() {
            if self.swords[k].flying && self.swords[k].points.iter().any(|&i| {
                let p = &self.particles[i as usize];
                p.p.y <= p.rad + ONE
            }) {
                self.swords[k].flying = false;
            }
        }
    }

    // --- the jump, the dodge and getting up (Sam, 2026-10-03) ------------------------

    /// The fighter's own points and its sword's while a hand holds it.
    fn moving_with(&self, seat: usize) -> Vec<usize> {
        let held = self.cons.iter().any(|c| matches!(c.tag, Tag::Grip { fighter, .. } if fighter as usize == seat));
        (0..self.particles.len())
            .filter(|&i| {
                let p = &self.particles[i];
                p.m != 0 && (p.owner == Owner::Body(seat as u8) || (held && p.owner == Owner::Sword(seat as u8)))
            })
            .collect()
    }

    /// The velocity of those points' center of mass.
    fn com_velocity(&self, pts: &[usize]) -> V2 {
        let (mut x, mut y, mut m) = (0i64, 0i64, 0i64);
        for &i in pts {
            let p = &self.particles[i];
            let v = p.p - p.q;
            x += p.m as i64 * v.x.0 as i64;
            y += p.m as i64 * v.y.0 as i64;
            m += p.m as i64;
        }
        if m == 0 {
            return V2::ZERO;
        }
        V2::new(Fx(crate::fx::narrow(x / m)), Fx(crate::fx::narrow(y / m)))
    }

    /// Change every point's velocity by the same amount, so the fighter's
    /// center of mass changes by `dv` and its arm keeps swinging as it was.
    fn add_velocity(&mut self, pts: &[usize], dv: V2) {
        for &i in pts {
            self.particles[i].q -= dv;
        }
    }

    /// Act on a jump, dodge or stand press, count down a dodge, and return
    /// the input the rest of the tick should see: while dodging, a fighter's
    /// arm and step keys do nothing, as in Melee, where a dodge is a
    /// commitment.
    fn jump_and_dodge(&mut self, seat: usize, input: Input) -> Input {
        let grounded = self.feet(seat).iter().any(|&i| self.grounded(i)) && self.balanced(seat);
        let down = self.knocked_down(seat);
        // Stand works in the air over a platform, and sets the fighter on
        // it (Sam, 2026-10-05: the stand "sets you on top of the platform
        // thats immediately below you"); like getting up, it needs both
        // legs. Over open ground it does nothing in the air, as before: a
        // stand there would cancel any jump, and the tree pilots, pressing
        // it on what they saw some ticks ago, began to (SECOND-ORDER-M5 row 48).
        let pelvis = self.fighters[seat].as_ref().and_then(|f| self.setup.bodies[f.body as usize].roles.pelvis.map(|k| self.particles[(f.base + k as u16) as usize].p));
        let over_ledge = pelvis.is_some_and(|p| self.surface_below(p.x, p.y) > Fx(0));
        let aloft = over_ledge && !self.feet(seat).iter().any(|&i| self.grounded(i)) && self.feet(seat).len() == 2 && matches!(self.phase, crate::fight::Phase::Fight);
        let f = self.fighters[seat].as_mut().unwrap();
        let buttons = Input::JUMP | Input::DODGE | Input::STAND | Input::THROW;
        let pressed = input.0 & !f.held & buttons;
        f.held = input.0 & buttons;
        if grounded {
            f.air_jumps = 1;
        }
        f.dodge = f.dodge.saturating_sub(1);
        f.dodge_cooldown = f.dodge_cooldown.saturating_sub(1);
        let dir = input.axis(Input::STEP_RIGHT, Input::STEP_LEFT);
        enum Act {
            None,
            AirDodge,
            GroundJump,
            AirJump,
            Stand,
        }
        let mut act = Act::None;
        if pressed & Input::STAND != 0 && (down || aloft) && f.dodge == 0 {
            act = Act::Stand;
        } else if pressed & Input::DODGE != 0 && f.dodge == 0 && f.dodge_cooldown == 0 {
            f.dodge = balance::DODGE_TICKS;
            f.dodge_cooldown = balance::DODGE_COOLDOWN;
            f.dodge_dir = dir;
            if !grounded {
                // An air dodge: a burst the way the step keys point, the
                // fall stopped for the moment, and no air jump until landing.
                f.air_jumps = 0;
                act = Act::AirDodge;
            }
        } else if pressed & Input::JUMP != 0 && f.dodge == 0 {
            if grounded {
                act = Act::GroundJump;
            } else if f.air_jumps > 0 {
                f.air_jumps -= 1;
                act = Act::AirJump;
            }
        }
        let dodging = f.dodge > 0;
        let pts = self.moving_with(seat);
        let v = self.com_velocity(&pts);
        match act {
            Act::None => {}
            Act::Stand => self.stand_up(seat),
            Act::AirDodge => {
                let want = V2::new(balance::AIR_DODGE_SPEED * dir, Fx(0));
                self.add_velocity(&pts, want - v);
            }
            Act::GroundJump => {
                // The ground pushes: the fighter rises at the jump speed, or
                // faster if it was already rising faster.
                let up = (balance::JUMP_SPEED - v.y).max(Fx(0));
                self.add_velocity(&pts, V2::new(Fx(0), up));
            }
            Act::AirJump => {
                // As an elastic collision off a corner you cannot see (Sam,
                // 2026-10-03): a fall is reflected into a rise at the same
                // speed, and with a step key held, motion away from that side
                // is reflected toward it. Reflection keeps speed; then the
                // jump itself pushes at least AIR_JUMP_SPEED up and
                // AIR_JUMP_SIDE toward a held side.
                let mut w = v;
                if w.y.0 < 0 {
                    w.y = -w.y;
                }
                w.y = w.y.max(balance::AIR_JUMP_SPEED);
                if dir != 0 {
                    if w.x.signum() == -dir {
                        w.x = -w.x;
                    }
                    if (w.x * dir) < balance::AIR_JUMP_SIDE {
                        w.x = balance::AIR_JUMP_SIDE * dir;
                    }
                }
                self.add_velocity(&pts, w - v);
            }
        }
        if dodging {
            Input(input.0 & Input::READY)
        } else {
            input
        }
    }

    /// Off its feet but with both legs: what the stand key can mend. The
    /// torso leans more than about 45° or the pelvis is below 60 % of its
    /// standing height.
    pub fn knocked_down(&self, seat: usize) -> bool {
        let Some(f) = self.fighters[seat].as_ref() else { return false };
        if !matches!(self.phase, crate::fight::Phase::Fight) || f.dodge > 0 {
            return false;
        }
        let def = &self.setup.bodies[f.body as usize];
        let both_feet = def.roles.feet.len() == 2 && self.feet(seat).len() == 2;
        let (Some(pe), Some(sh)) = (self.role(seat, |r| r.pelvis), self.role(seat, |r| r.shoulder)) else { return false };
        if !both_feet {
            return false;
        }
        let (pp, sp) = (self.particles[pe as usize].p, self.particles[sh as usize].p);
        let rest_pelvis = def.points[def.roles.pelvis.unwrap() as usize].at.y;
        let upright = (sp.y - pp.y).scale(10, 7) >= f.torso;
        // Measured from what it stands on: the ground, or a platform.
        let tall = (pp.y - self.floor_of(seat)).scale(10, 6) >= rest_pelvis;
        !(upright && tall)
    }

    /// Stand the fighter back up where its pelvis is, in its rest pose, still.
    /// Parts already cut stay cut: a stump keeps its length along its part.
    fn stand_up(&mut self, seat: usize) {
        let f = self.fighters[seat].clone().unwrap();
        let def = self.setup.bodies[f.body as usize].clone();
        let pe = (f.base + def.roles.pelvis.unwrap() as u16) as usize;
        let rest_pelvis = def.points[def.roles.pelvis.unwrap() as usize].at;
        let lim = self.setup.arena_half - Fx::int(60);
        let x = (self.particles[pe].p.x - rest_pelvis.x * f.facing).clamp(-lim, lim);
        // In the air, on the surface immediately below the pelvis (a jump
        // and the stand key set a fighter on a ledge); with a foot down, on
        // what that foot stands on, so a fighter knocked over under a ledge
        // gets up under it, not on top of it (SECOND-ORDER-M5 row 58).
        let aloft = !self.feet(seat).iter().any(|&i| self.grounded(i));
        let base = if aloft { self.surface_below(self.particles[pe].p.x, self.particles[pe].p.y) } else { self.floor_of(seat) };
        let place = |at: V2| V2::new(x + at.x * f.facing, at.y + base);
        let mine = |w: &World, i: usize| w.particles[i].owner == Owner::Body(seat as u8);
        for (k, pt) in def.points.iter().enumerate() {
            let i = (f.base + k as u16) as usize;
            if mine(self, i) {
                let p = place(pt.at);
                self.particles[i].p = p;
                self.particles[i].q = p;
            }
        }
        // The cut end of a stump, along its part at the stump's length.
        for k in 0..self.parts.len() {
            let part = self.parts[k].clone();
            if part.fighter as usize != seat || !mine(self, part.b as usize) || (part.b as usize) < f.base as usize + def.points.len() && part.b >= f.base {
                continue;
            }
            let d = &def.parts[part.def as usize];
            let (a, b) = (place(def.points[d.near as usize].at), place(def.points[d.far as usize].at));
            let whole = (b - a).len();
            if whole.0 == 0 {
                continue;
            }
            let p = self.particles[part.a as usize].p + (b - a).scale(part.len.0 as i64, whole.0 as i64);
            self.particles[part.b as usize].p = p;
            self.particles[part.b as usize].q = p;
        }
        if let (Some(sd), Some(si)) = (def.sword.as_ref(), f.sword) {
            let s = self.swords[si as usize].clone();
            if self.cons.iter().any(|c| matches!(c.tag, Tag::Grip { fighter, .. } if fighter as usize == seat)) {
                let ats = [sd.butt, sd.tip].into_iter().chain(sd.extra.iter().copied());
                for (i, at) in s.points.iter().copied().zip(ats) {
                    let p = place(at);
                    self.particles[i as usize].p = p;
                    self.particles[i as usize].q = p;
                }
            }
        }
    }

    /// Whether a fighter is in the middle of a dodge.
    pub fn dodging(&self, seat: usize) -> bool {
        self.fighters[seat].as_ref().is_some_and(|f| f.dodge > 0)
    }

    // --- integration -------------------------------------------------------------

    /// Velocity first, then position: H1's order.
    fn integrate(&mut self, acc: &[V2]) {
        let ph = self.setup.physics;
        let cap_sq = ph.tuning.cap.0 as i64 * ph.tuning.cap.0 as i64;
        for (i, pt) in self.particles.iter_mut().enumerate() {
            if pt.m == 0 {
                pt.q = pt.p;
                continue;
            }
            let mut v = pt.p - pt.q;
            v -= v * ph.tuning.drag;
            v += acc[i];
            v.y -= ph.gravity;
            if v.len_sq_raw() > cap_sq {
                v = v.with_len(ph.tuning.cap);
            }
            pt.q = pt.p;
            pt.p += v;
        }
    }

    // --- relaxation, in a fixed order ------------------------------------------

    pub(crate) fn relax(&mut self, drives: &[Drive]) {
        for d in drives {
            self.drive_joint(d);
        }
        for k in 0..self.parts.len() {
            let (a, b, len) = (self.parts[k].a, self.parts[k].b, self.parts[k].len);
            self.stick(a, b, len, false);
        }
        for k in 0..self.cons.len() {
            match self.cons[k].con {
                Con::Stick { a, b, len } => self.stick(a, b, len, false),
                Con::Min { a, b, len } => self.stick(a, b, len, true),
                Con::Pin { a, b, c, at, extra } => {
                    let target = V2::lerp(self.particles[b as usize].p, self.particles[c as usize].p, at);
                    let d = target - self.particles[a as usize].p;
                    // The whole weapon moves as one: its butt, its tip, and
                    // the points after them.
                    if extra == 0 {
                        self.shift_group(a, &[b, c], d);
                    } else {
                        let group: Vec<u16> = [b, c].into_iter().chain((0..extra as u16).map(|k| c + 1 + k)).collect();
                        self.shift_group(a, &group, d);
                    }
                }
                Con::Hinge { a, j, c, sign } => {
                    let (pa, pj, pc) = (self.particles[a as usize].p, self.particles[j as usize].p, self.particles[c as usize].p);
                    let (u, f) = (pj - pa, pc - pj);
                    if u.cross_raw(f) * (sign as i64) < 0 {
                        let target = pj + u.with_len(f.len());
                        self.shift_pair(c, j, target - pc);
                    }
                }
            }
        }
        // Every seat: with 0..2 here, the third fighter on a flanked stop was
        // never balanced and fell over at the start of each round
        // (SECOND-ORDER-M5 row 58).
        for seat in 0..SEATS {
            self.balance(seat);
        }
        self.bounds();
    }

    /// The relaxation passes can push a point past the cap after integration
    /// clamped it; this holds every point's speed for the next tick to the
    /// cap, by moving where it was rather than where it is. `speed_never_passes_the_cap`
    /// checks it over a hundred seeded runs.
    fn hold_to_cap(&mut self) {
        let cap = self.setup.physics.tuning.cap;
        let cap_sq = cap.0 as i64 * cap.0 as i64;
        for pt in &mut self.particles {
            let v = pt.p - pt.q;
            if pt.m != 0 && v.len_sq_raw() > cap_sq {
                pt.q = pt.p - v.with_len(cap);
            }
        }
    }

    fn stick(&mut self, a: u16, b: u16, len: Fx, push_only: bool) {
        let d = self.particles[b as usize].p - self.particles[a as usize].p;
        let dist = d.len();
        if dist.0 == 0 || (push_only && dist >= len) {
            return;
        }
        let delta = d.scale((dist - len).0 as i64, dist.0 as i64);
        self.shift_pair(a, b, delta);
    }

    /// Move `i` by `+k·m_j` and `j` by `-k·m_i`, with one integer `k` per
    /// axis, so the pair's momentum does not change by a single raw unit. An
    /// anchored point (mass zero) does not move and its partner takes all of
    /// `d`: the anchor is outside the system.
    pub(crate) fn shift_pair(&mut self, i: u16, j: u16, d: V2) {
        let (mi, mj) = (self.particles[i as usize].m, self.particles[j as usize].m);
        match (mi, mj) {
            (0, 0) => {}
            (0, _) => self.particles[j as usize].p -= d,
            (_, 0) => self.particles[i as usize].p += d,
            _ => {
                let k = d / (mi + mj);
                self.particles[i as usize].p += k * mj;
                self.particles[j as usize].p -= k * mi;
            }
        }
    }

    /// `shift_pair` against a group that moves together.
    fn shift_group(&mut self, i: u16, group: &[u16], d: V2) {
        let mi = self.particles[i as usize].m;
        let mg: i32 = group.iter().map(|&g| self.particles[g as usize].m).sum();
        if mi == 0 || mg == 0 || group.iter().any(|&g| self.particles[g as usize].m == 0) {
            return;
        }
        let k = d / (mi + mg);
        self.particles[i as usize].p += k * mg;
        for &g in group {
            self.particles[g as usize].p -= k * mi;
        }
    }

    /// The balance rule, as pair shifts (D8).
    fn balance(&mut self, seat: usize) {
        if !self.balanced(seat) {
            return;
        }
        let k = balance::BALANCE_K;
        let f = self.fighters[seat].clone().unwrap();
        let (Some(sh), Some(pe)) = (self.role(seat, |r| r.shoulder), self.role(seat, |r| r.pelvis)) else { return };
        // The torso stands up.
        let target = self.particles[pe as usize].p + V2::new(Fx(0), f.torso);
        let d = (target - self.particles[sh as usize].p) * k;
        self.shift_pair(sh, pe, d);
        // The head sits over the shoulders.
        if let Some(hd) = self.role(seat, |r| r.head) {
            let target = self.particles[sh as usize].p + V2::new(Fx(0), f.neck);
            let d = (target - self.particles[hd as usize].p) * k;
            self.shift_pair(hd, sh, d);
        }
        // The pelvis keeps over the feet that are on the ground.
        let feet: Vec<u16> = self.feet(seat).into_iter().filter(|&i| self.grounded(i)).collect();
        if !feet.is_empty() {
            let sum: i64 = feet.iter().map(|&i| self.particles[i as usize].p.x.0 as i64).sum();
            let mid = Fx(crate::fx::narrow(sum / feet.len() as i64));
            let dx = (mid - self.particles[pe as usize].p.x) * k;
            self.shift_group(pe, &feet, V2::new(dx, Fx(0)));
        }
    }

    /// The ground and the walls. Both are half-planes, so clamping an
    /// endpoint is the swept answer for a straight stick too (D10).
    fn bounds(&mut self) {
        let ph = self.setup.physics;
        let drive = [0usize, 1, 2].map(|s| self.fighters[s].as_ref().map(|f| f.drive).unwrap_or(Fx(0)));
        let platforms = self.setup.platforms.clone();
        let arena_half = self.setup.arena_half;
        for pt in &mut self.particles {
            if pt.m == 0 {
                continue;
            }
            if ph.ground && pt.p.y <= pt.rad {
                pt.p.y = pt.rad;
                // Friction lives in the projection, every pass (Jakobsen):
                // a point on the ground gives back part of this tick's
                // sideways travel. Once per tick was too weak; the passes
                // pushed the feet apart and the legs splayed (SECOND-ORDER-M1).
                // A driven foot is carried at the run speed instead: the
                // ground pushes, which is a force from outside the fighter.
                let want = match pt.owner {
                    Owner::Body(s) if pt.foot => drive[s as usize],
                    _ => Fx(0),
                };
                let slide = pt.p.x - pt.q.x - want;
                pt.p.x -= slide * pt.grip;
            }
            // A platform holds a foot that comes down onto it from above,
            // with the ground's friction and drive; from below and the sides
            // it is passed through. Feet only: when it held every point, a
            // fighter standing under a ledge lower than its shoulders was
            // caught by an arm dipping onto the ledge and hoisted up
            // (SECOND-ORDER-M5 row 51).
            for pl in &platforms {
                let top = pl.y + pt.rad;
                if pt.foot && pt.p.x >= pl.x0 && pt.p.x <= pl.x1 && pt.q.y >= top - ONE && pt.p.y < top {
                    pt.p.y = top;
                    let want = match pt.owner {
                        Owner::Body(s) if pt.foot => drive[s as usize],
                        _ => Fx(0),
                    };
                    let slide = pt.p.x - pt.q.x - want;
                    pt.p.x -= slide * pt.grip;
                }
            }
            if ph.walls {
                let lim = arena_half - pt.rad;
                pt.p.x = pt.p.x.clamp(-lim, lim);
            }
        }
    }

    // --- reading the world ---------------------------------------------------------

    /// Total momentum, Σ m·v, in raw units times mass.
    pub fn momentum(&self) -> (i64, i64) {
        self.particles.iter().fold((0, 0), |(x, y), p| {
            let v = p.p - p.q;
            (x + p.m as i64 * v.x.0 as i64, y + p.m as i64 * v.y.0 as i64)
        })
    }

    /// Kinetic energy, Σ m·v², in raw units squared times mass.
    pub fn kinetic(&self) -> i64 {
        self.particles.iter().map(|p| p.m as i64 * (p.p - p.q).len_sq_raw()).sum()
    }
}

/// 64-bit FNV-1a (Floodline `world.rs:2180`).
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_matches_the_published_vectors() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(b"foobar"), 0x8594_4171_f739_67e8);
    }
}
