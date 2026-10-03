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
use crate::body::{BodyDef, Motor, Setup};
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
    /// `a` stays at the point `at` of the way from `b` to `c`.
    Pin { a: u16, b: u16, c: u16, at: Fx },
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
    pub body: u8,
    /// +1 faces +x (the left seat), -1 faces -x.
    pub facing: i32,
    /// This fighter's points are `base .. base + n` in `particles`.
    pub base: u16,
    pub n: u16,
    pub sword: Option<u8>,
    pub ink: i32,
    /// -1, 0 or +1: which way the step keys ask the ground to carry this
    /// fighter this tick (D8, as Sam revised it on 2026-10-03).
    pub drive: i32,
    /// Rest distances the balance rule restores: shoulder over pelvis, head
    /// over shoulder.
    pub torso: Fx,
    pub neck: Fx,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Sword {
    pub fighter: u8,
    pub butt: u16,
    pub tip: u16,
    pub len: Fx,
    pub hilt: Fx,
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
    pub fighters: [Option<Fighter>; 2],
    pub swords: Vec<Sword>,
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
            fighters: [None, None],
            swords: Vec::new(),
        };
        w.spawn_round();
        w
    }

    /// Clear the arena and stand both seats up. One jitter draw per round,
    /// applied mirrored so neither seat is favored.
    fn spawn_round(&mut self) {
        self.particles.clear();
        self.parts.clear();
        self.cons.clear();
        self.swords.clear();
        self.fighters = [None, None];
        let jitter = Fx::int(self.rng.range(-balance::SPAWN_JITTER_CM, balance::SPAWN_JITTER_CM));
        for seat in 0..2u8 {
            if let Some(s) = self.setup.seats[seat as usize] {
                let facing = if seat == 0 { 1 } else { -1 };
                let x = (s.x + jitter) * -facing;
                self.spawn(seat, s.body, facing, x);
            }
        }
    }

    fn spawn(&mut self, seat: u8, body: u8, facing: i32, x: Fx) {
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
            self.cons.push(Constraint { con: Con::Hinge { a, j, c, sign: h.sign }, tag: Tag::Body(seat) });
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
            self.swords.push(Sword { fighter: seat, butt, tip, len, hilt: sd.hilt });
            self.cons.push(Constraint { con: Con::Stick { a: butt, b: tip, len }, tag: Tag::Sword(si) });
            for g in &sd.grips {
                let hand = base + g.hand as u16;
                self.cons.push(Constraint { con: Con::Pin { a: hand, b: butt, c: tip, at: g.at }, tag: Tag::Grip { fighter: seat, hand } });
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
            body,
            facing,
            base,
            n,
            sword,
            ink: def.ink,
            drive: 0,
            torso: dist(r.shoulder, r.pelvis),
            neck: dist(r.head, r.shoulder),
        });
    }

    /// The only way the world changes.
    pub fn step(&mut self, inputs: [Input; 2]) {
        let mut acc = vec![V2::ZERO; self.particles.len()];
        for seat in 0..2 {
            if self.fighters[seat].is_some() {
                self.drive_motors(seat, inputs[seat], &mut acc);
                self.set_drive(seat, inputs[seat]);
            }
        }
        self.integrate(&acc);
        for _ in 0..ITERATIONS {
            self.relax();
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
            if let (Con::Pin { a, b, c: tip, .. }, Tag::Grip { fighter, .. }) = (c.con, c.tag) {
                if fighter == root.fighter && set.contains(&a) && !set.contains(&b) {
                    set.push(b);
                    set.push(tip);
                }
            }
        }
        set
    }

    fn drive_motors(&self, seat: usize, input: Input, acc: &mut [V2]) {
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
            if omega * dir >= t.motor_speed {
                continue;
            }
            // An elbow that is already straight does not push past straight.
            if motor == Motor::Elbow && dir * facing < 0 && self.elbow_is_straight(pivot, set[0], facing) {
                continue;
            }
            let a = t.motor_accel * dir;
            for &k in &set {
                let arm = self.particles[k as usize].p - pv.p;
                acc[k as usize] += arm.perp() * a;
            }
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

    fn grounded(&self, i: u16) -> bool {
        let p = &self.particles[i as usize];
        self.setup.physics.ground && p.p.y <= p.rad + ONE
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

    /// The balance rule holds a fighter up while it stands on a foot.
    fn balanced(&self, seat: usize) -> bool {
        let Some(f) = &self.fighters[seat] else { return false };
        self.setup.bodies[f.body as usize].balance
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
            f.drive = if balanced { input.axis(Input::STEP_RIGHT, Input::STEP_LEFT) } else { 0 };
        }
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

    fn relax(&mut self) {
        for k in 0..self.parts.len() {
            let (a, b, len) = (self.parts[k].a, self.parts[k].b, self.parts[k].len);
            self.stick(a, b, len, false);
        }
        for k in 0..self.cons.len() {
            match self.cons[k].con {
                Con::Stick { a, b, len } => self.stick(a, b, len, false),
                Con::Min { a, b, len } => self.stick(a, b, len, true),
                Con::Pin { a, b, c, at } => {
                    let target = V2::lerp(self.particles[b as usize].p, self.particles[c as usize].p, at);
                    let d = target - self.particles[a as usize].p;
                    self.shift_group(a, &[b, c], d);
                }
                Con::Hinge { a, j, c, sign } => {
                    let facing = match self.particles[j as usize].owner {
                        Owner::Body(s) => self.fighters[s as usize].as_ref().map(|f| f.facing).unwrap_or(1),
                        _ => 1,
                    };
                    let (pa, pj, pc) = (self.particles[a as usize].p, self.particles[j as usize].p, self.particles[c as usize].p);
                    let (u, f) = (pj - pa, pc - pj);
                    if u.cross_raw(f) * (sign as i64) * (facing as i64) < 0 {
                        let target = pj + u.with_len(f.len());
                        self.shift_pair(c, j, target - pc);
                    }
                }
            }
        }
        for seat in 0..2 {
            self.balance(seat);
        }
        self.bounds();
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
    fn shift_pair(&mut self, i: u16, j: u16, d: V2) {
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
        let drive = [0usize, 1].map(|s| self.fighters[s].as_ref().map(|f| f.drive).unwrap_or(0));
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
                    Owner::Body(s) if pt.foot => balance::RUN_SPEED * drive[s as usize],
                    _ => Fx(0),
                };
                let slide = pt.p.x - pt.q.x - want;
                pt.p.x -= slide * pt.grip;
            }
            if ph.walls {
                let lim = balance::ARENA_HALF - pt.rad;
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
