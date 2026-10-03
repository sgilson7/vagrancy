//! Cuts, clashes, ink, and how a round and a match end (D10–D13).

use crate::balance;
use crate::body::{Cause, Mode};
use crate::contact::{self, Swept};
use crate::fx::{Fx, V2, ONE};
use crate::input::Input;
use crate::world::{Con, Drive, Owner, Part, Tag, World};
use serde::{Deserialize, Serialize};

/// A cut that ends the round, as it landed.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Fatal {
    pub cause: Cause,
    /// The `PartDef` index of the part it crossed.
    pub part: u8,
    /// Whose blade.
    pub by: u8,
}

/// How a round ended.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct RoundResult {
    /// The seat that lost; `None` when both stopped on the same tick (a draw,
    /// played again).
    pub loser: Option<u8>,
    pub cause: Cause,
    /// For a cut, the part it crossed; for ink, the part that spilled most.
    pub part: u8,
    /// Whose blade made the deciding cut (for ink, the loser's own seat).
    pub by: u8,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Phase {
    Fight,
    /// The round is over; the next starts when both seats have pressed ready.
    RoundOver { result: RoundResult, ready: [bool; 2] },
    MatchOver { result: RoundResult },
}

/// What happened this tick, for the page to draw and later to sound.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Event {
    /// A blade cut a part. `seat` is the part's fighter; `spilled` is false
    /// for a piece that had already dropped.
    Cut { seat: u8, part: u8, by: u8, at: V2, spilled: bool },
    /// Two blades met.
    Clash { at: V2 },
    RoundEnd { result: RoundResult },
    MatchEnd { winner: u8 },
}

/// How close two blades may come, centerline to centerline.
const BLADE_GAP: Fx = Fx::int(2);

impl World {
    /// The largest number of substeps a swept test may take: twice the cap
    /// over the smallest part radius, so a point at the cap cannot cross
    /// the thinnest part between two substeps (D10). Derived, not typed.
    fn s_max(&self) -> u32 {
        let min_r = self.parts.iter().map(|p| p.radius.0).min().unwrap_or(ONE.0).max(1);
        let cap = self.setup.physics.tuning.cap.0;
        ((2 * cap as i64 + min_r as i64 - 1) / min_r as i64).max(1) as u32
    }

    fn swept(&self, a: u16, b: u16) -> Swept {
        let (pa, pb) = (&self.particles[a as usize], &self.particles[b as usize]);
        Swept { a0: pa.q, b0: pb.q, a1: pa.p, b1: pb.p }
    }

    /// The cutting part of a sword: from the end of the hilt to the tip.
    fn blade(&self, si: usize) -> Swept {
        let s = &self.swords[si];
        let h = Fx::ratio(s.hilt.0 as i64, s.len.0.max(1) as i64);
        let whole = self.swept(s.butt, s.tip);
        Swept { a0: V2::lerp(whole.a0, whole.b0, h), b0: whole.b0, a1: V2::lerp(whole.a1, whole.b1, h), b1: whole.b1 }
    }

    /// A sword cuts only while a hand holds it (PLAN.md §8 Q15).
    pub fn held(&self, si: usize) -> bool {
        let f = self.swords[si].fighter;
        self.cons.iter().any(|c| matches!(c.tag, Tag::Grip { fighter, .. } if fighter == f))
    }

    /// Whether sword `si` can cut part `pi` at all: the sword is held (Q15),
    /// the part is not its own fighter's, and neither side is dodging. An
    /// opponent's blade cuts a hand (Q14). `cuts` reads this; so do the tests.
    pub fn may_cut(&self, si: usize, pi: usize) -> bool {
        let part = &self.parts[pi];
        let owner = self.swords[si].fighter;
        // A dodging fighter cannot be cut, and its blade cuts nothing.
        let dodged = part.attached && self.dodging(part.fighter as usize);
        // And a fighter's own blade never cuts that fighter: Sam removed
        // self-cuts on 2026-10-03, reversing his answer in brief 0.7, because
        // why one happened was too hard to see.
        part.fighter != owner && self.held(si) && !self.dodging(owner as usize) && !dodged
    }

    fn fighter_body(&self, seat: u8) -> usize {
        self.setup.seats[seat as usize].map(|s| s.body as usize).unwrap_or(0)
    }

    // --- cuts ---------------------------------------------------------------------

    /// Every held blade against every part: the first tick a blade touches a
    /// part, it cuts it (D11). Runs after relaxation, on where things started
    /// the tick (`q`) and where they ended it (`p`).
    pub(crate) fn cuts(&mut self) {
        if !matches!(self.phase, Phase::Fight) {
            self.touching.clear();
            return;
        }
        let s_max = self.s_max();
        let mut now: Vec<(u8, u16)> = Vec::new();
        for si in 0..self.swords.len() {
            if !self.held(si) {
                continue;
            }
            let owner = self.swords[si].fighter;
            let blade = self.blade(si);
            let n = self.parts.len();
            for pi in 0..n {
                let part = self.parts[pi].clone();
                if !self.may_cut(si, pi) {
                    continue;
                }
                let swept = self.swept(part.a, part.b);
                if !boxes_meet(&blade, &swept, part.radius) {
                    continue;
                }
                let Some(hit) = contact::sweep(&blade, &swept, part.radius, s_max, 0) else { continue };
                now.push((si as u8, pi as u16));
                if self.touching.contains(&(si as u8, pi as u16)) {
                    continue;
                }
                // A blade that drifts into a part rests against it; a blade
                // that arrives at speed cuts it. Without this every fighter
                // cut itself within a second of standing still (M3.0).
                let (speed, _) = self.contact_speed(si, &part, hit);
                if speed < balance::MIN_CUT_SPEED {
                    continue;
                }
                let f = hit.f.clamp(Fx::ratio(1, 20), Fx::ratio(19, 20));
                if let Some(far) = self.cut(pi, f, owner) {
                    now.push((si as u8, far));
                }
            }
        }
        self.touching = now;
    }

    /// How fast the blade's touching point moves against the part's touching
    /// point, cm per tick; and how much of that is along the blade toward the
    /// tip (a thrust).
    fn contact_speed(&self, si: usize, part: &Part, hit: contact::Hit) -> (Fx, Fx) {
        let s = &self.swords[si];
        let h = Fx::ratio(s.hilt.0 as i64, s.len.0.max(1) as i64);
        // The hit's `g` is along the cutting part; map it back onto the sword.
        let g = h + (ONE - h) * hit.g;
        let vel = |i: u16| self.particles[i as usize].p - self.particles[i as usize].q;
        let blade_v = V2::lerp(vel(s.butt), vel(s.tip), g);
        let part_v = V2::lerp(vel(part.a), vel(part.b), hit.f);
        let rel = blade_v - part_v;
        let axis = (self.particles[s.tip as usize].p - self.particles[s.butt as usize].p).with_len(ONE);
        (rel.len(), rel.dot(axis))
    }

    /// Cut part `pi` straight across at fraction `f` from its near end, and
    /// drop everything beyond the cut as a piece. Returns the new far part.
    pub(crate) fn cut(&mut self, pi: usize, f: Fx, by: u8) -> Option<u16> {
        let part = self.parts[pi].clone();
        let (a, b) = (part.a as usize, part.b as usize);
        let at = V2::lerp(self.particles[a].p, self.particles[b].p, f);
        let at_q = V2::lerp(self.particles[a].q, self.particles[b].q, f);
        let owner = self.particles[a].owner;
        let def = self.setup.bodies[self.fighter_body(part.fighter)].parts[part.def as usize].clone();
        let attached = part.attached && owner == Owner::Body(part.fighter);

        // H3: a uniform part's mass splits by where it was cut.
        let near_mass = (part.mass as i64 * f.0 as i64 / ONE.0 as i64).max(1) as i32;
        let far_mass = (part.mass - near_mass).max(1);
        let near_pt = self.particles.len() as u16;
        let far_pt = near_pt + 1;
        let proto = self.particles[a].clone();
        for _ in 0..2 {
            let mut p = proto.clone();
            p.p = at;
            p.q = at_q;
            p.rad = Fx(0);
            p.foot = false;
            p.grip = balance::GRIP_BODY;
            self.particles.push(p);
        }
        let near_len = part.len * f;
        self.parts[pi].b = near_pt;
        self.parts[pi].len = near_len;
        self.parts[pi].mass = near_mass;
        self.parts[pi].stump = attached;
        let far_idx = self.parts.len() as u16;
        self.parts.push(Part { a: far_pt, b: part.b, len: part.len - near_len, mass: far_mass, stump: false, ..part.clone() });

        // Everything beyond the cut: the far half, and every part hanging off
        // it, by walking the tree outward from the cut (D11: "the piece that
        // drops is always the one farther from the heart").
        let piece = Owner::Piece(self.next_piece);
        self.next_piece += 1;
        let mut dropped = vec![far_pt, part.b];
        let mut grew = true;
        while grew {
            grew = false;
            for (k, p) in self.parts.iter().enumerate() {
                if k != pi && self.particles[p.a as usize].owner == owner && dropped.contains(&p.a) && !dropped.contains(&p.b) {
                    dropped.push(p.b);
                    grew = true;
                }
            }
        }
        for &d in &dropped {
            self.particles[d as usize].owner = piece;
        }
        for p in self.parts.iter_mut() {
            if dropped.contains(&p.a) {
                p.attached = false;
            }
        }
        self.reweigh(&[part.a, near_pt, far_pt, part.b]);
        // A constraint that now spans the cut goes, and so does a grip whose
        // hand has left its body.
        let owners: Vec<Owner> = self.particles.iter().map(|p| p.owner).collect();
        self.cons.retain(|c| {
            let pts: Vec<u16> = match c.con {
                Con::Stick { a, b, .. } | Con::Min { a, b, .. } => vec![a, b],
                Con::Pin { a, b, c, .. } => vec![a, b, c],
                Con::Hinge { a, j, c, .. } => vec![a, j, c],
            };
            match c.tag {
                Tag::Grip { fighter, hand } => owners[hand as usize] == Owner::Body(fighter)
                    && pts.iter().all(|&p| matches!(owners[p as usize], Owner::Body(f) | Owner::Sword(f) if f == fighter)),
                _ => pts.iter().all(|&p| owners[p as usize] == owners[pts[0] as usize]),
            }
        });

        // Ink and the fatal zones belong to a fighter's own body only.
        let spilled = attached && self.setup.bodies[self.fighter_body(part.fighter)].ink > 0;
        if spilled {
            for band in &def.fatal {
                // The band is along the original part, measured from its
                // near end: the same f the cut was made at.
                if f >= band.from && f <= band.to {
                    if let Some(fi) = self.fighters[part.fighter as usize].as_mut() {
                        fi.fatal.get_or_insert(Fatal { cause: band.cause, part: part.def, by });
                    }
                }
            }
        }
        self.events.push(crate::fight::Event::Cut { seat: part.fighter, part: part.def, by, at, spilled });
        Some(far_idx)
    }

    /// A point's mass is the sum of the parts that touch it; anchored points
    /// stay at zero.
    fn reweigh(&mut self, pts: &[u16]) {
        for &i in pts {
            if self.particles[i as usize].m == 0 {
                continue;
            }
            let m: i32 = self.parts.iter().filter(|p| p.a == i || p.b == i).map(|p| p.mass).sum();
            self.particles[i as usize].m = m.max(1);
        }
    }

    /// The mass of everything a piece or a body is made of, by owner: the sum
    /// of its parts. H3 reads it.
    pub fn parts_mass(&self, owner: Owner) -> i32 {
        self.parts.iter().filter(|p| self.particles[p.a as usize].owner == owner).map(|p| p.mass).sum()
    }

    // --- blade against blade (D10) ----------------------------------------------------

    /// Swords are solid against each other (D10). A swept test finds the
    /// first substep at which the two come within `BLADE_GAP`. Both swords
    /// then go back to where they were at the substep before it — the last
    /// pose in which they had not met — and the arms are relaxed to follow:
    /// a block stops the swing ("a firm block stops the turn"). If they are
    /// still closer than the gap, or crossed, they go back again, and the
    /// pose they end the tick in is one where they have not passed through.
    pub(crate) fn clash(&mut self, drives: &[Drive]) {
        if self.swords.len() < 2 {
            return;
        }
        let s_max = self.s_max().max(8);
        let (ib, it, jb, jt) = (self.swords[0].butt, self.swords[0].tip, self.swords[1].butt, self.swords[1].tip);
        let a = self.swept(ib, it);
        let b = self.swept(jb, jt);
        // Blades that start the tick already touching may slide along each
        // other and part freely; they are only stopped if they would cross.
        // Without this they glued together: every tick found contact at its
        // first substep and went back to where it began.
        let resting = contact::near(a.a0, a.b0, b.a0, b.b0, BLADE_GAP).is_some();
        let (k, s) = if resting {
            if !segments_cross(a.a1, a.b1, b.a1, b.b1) {
                self.clashing = contact::near(a.a1, a.b1, b.a1, b.b1, BLADE_GAP).is_some();
                return;
            }
            (0i64, 1i64)
        } else {
            let Some(hit) = contact::sweep(&a, &b, BLADE_GAP, s_max, 1) else {
                self.clashing = false;
                return;
            };
            (hit.k.saturating_sub(1) as i64, hit.s as i64)
        };
        let at = |w: &Swept| (w.a0 + (w.a1 - w.a0).scale(k, s), w.b0 + (w.b1 - w.b0).scale(k, s));
        let (safe_a, safe_b) = (at(&a), at(&b));
        let put = |w: &mut World, (p0, p1): (V2, V2), (q0, q1): (V2, V2)| {
            w.particles[ib as usize].p = p0;
            w.particles[it as usize].p = p1;
            w.particles[jb as usize].p = q0;
            w.particles[jt as usize].p = q1;
        };
        put(self, safe_a, safe_b);
        self.relax(drives);
        self.relax(drives);
        let p = |w: &World, i: u16| w.particles[i as usize].p;
        let ok = if resting {
            !segments_cross(p(self, ib), p(self, it), p(self, jb), p(self, jt))
        } else {
            contact::near(p(self, ib), p(self, it), p(self, jb), p(self, jt), BLADE_GAP).is_none()
        };
        if !ok {
            put(self, safe_a, safe_b);
        }
        if !self.clashing {
            let (_, _, pe, qe) = contact::closest(safe_a.0, safe_a.1, safe_b.0, safe_b.1);
            self.events.push(Event::Clash { at: V2::lerp(pe, qe, Fx(ONE.0 / 2)) });
        }
        self.clashing = true;
    }

    // --- ink and the end of a round (D12, D13) -----------------------------------------

    /// Every attached stump drains ink, once, at the end of every tick,
    /// including the tick its cut lands on (H4).
    pub(crate) fn drain(&mut self) {
        for seat in 0..2 {
            let Some(f) = self.fighters[seat].as_ref() else { continue };
            let defs = &self.setup.bodies[f.body as usize].parts;
            let mut flow = vec![0i32; defs.len()];
            for p in &self.parts {
                if p.fighter as usize == seat && p.stump && p.attached && self.particles[p.a as usize].owner == Owner::Body(seat as u8) {
                    flow[p.def as usize] += defs[p.def as usize].drain;
                }
            }
            let f = self.fighters[seat].as_mut().unwrap();
            for (k, d) in flow.iter().enumerate() {
                let d = (*d).min(f.ink);
                f.ink -= d;
                f.spilled[k] += d;
            }
        }
    }

    /// Whether either fighter cannot continue, and what follows (D12, D13).
    pub(crate) fn judge(&mut self) {
        if self.setup.mode != Mode::Match || !matches!(self.phase, Phase::Fight) {
            return;
        }
        let mut out: [Option<RoundResult>; 2] = [None, None];
        for seat in 0..2u8 {
            let Some(f) = self.fighters[seat as usize].as_ref() else { continue };
            if self.setup.bodies[f.body as usize].ink == 0 {
                continue;
            }
            if let Some(fatal) = f.fatal {
                out[seat as usize] = Some(RoundResult { loser: Some(seat), cause: fatal.cause, part: fatal.part, by: fatal.by });
            } else if f.ink <= 0 {
                let most = (0..f.spilled.len()).max_by_key(|&k| (f.spilled[k], std::cmp::Reverse(k))).unwrap_or(0) as u8;
                out[seat as usize] = Some(RoundResult { loser: Some(seat), cause: Cause::Ink, part: most, by: seat });
            }
        }
        let result = match out {
            [None, None] => return,
            [Some(a), Some(_)] => RoundResult { loser: None, ..a },
            [Some(r), None] | [None, Some(r)] => r,
        };
        if let Some(l) = result.loser {
            self.wins[1 - l as usize] += 1;
        }
        self.events.push(Event::RoundEnd { result });
        let winner = self.wins.iter().position(|&w| w >= self.setup.rounds_to_win);
        self.phase = match winner {
            Some(w) => {
                self.events.push(Event::MatchEnd { winner: w as u8 });
                Phase::MatchOver { result }
            }
            None => Phase::RoundOver { result, ready: [false, false] },
        };
    }

    /// Between rounds: latch each seat's ready bit, and start the next round
    /// once every seat with a fighter has pressed it. Returns whether the
    /// fighters should be limp this tick.
    pub(crate) fn between_rounds(&mut self, inputs: [Input; 2]) -> bool {
        match self.phase {
            Phase::Fight => false,
            Phase::MatchOver { .. } => true,
            Phase::RoundOver { result, mut ready } => {
                for s in 0..2 {
                    if inputs[s].has(Input::READY) || self.setup.seats[s].is_none() {
                        ready[s] = true;
                    }
                }
                if ready == [true, true] {
                    if result.loser.is_some() {
                        self.round += 1;
                    }
                    self.phase = Phase::Fight;
                    self.spawn_round();
                    false
                } else {
                    self.phase = Phase::RoundOver { result, ready };
                    true
                }
            }
        }
    }
}

/// A cheap first look: do the boxes swept by the two segments, grown by the
/// radius, overlap at all?
fn boxes_meet(a: &Swept, b: &Swept, r: Fx) -> bool {
    let bx = |w: &Swept| {
        let xs = [w.a0.x, w.b0.x, w.a1.x, w.b1.x];
        let ys = [w.a0.y, w.b0.y, w.a1.y, w.b1.y];
        (*xs.iter().min().unwrap(), *xs.iter().max().unwrap(), *ys.iter().min().unwrap(), *ys.iter().max().unwrap())
    };
    let (ax0, ax1, ay0, ay1) = bx(a);
    let (bx0, bx1, by0, by1) = bx(b);
    ax0 - r <= bx1 && bx0 - r <= ax1 && ay0 - r <= by1 && by0 - r <= ay1
}

/// Whether two segments properly cross: each one's ends lie on opposite sides
/// of the other.
pub fn segments_cross(a0: V2, a1: V2, b0: V2, b1: V2) -> bool {
    let side = |p: V2, q0: V2, q1: V2| (q1 - q0).cross_raw(p - q0).signum();
    side(b0, a0, a1) * side(b1, a0, a1) < 0 && side(a0, b0, b1) * side(a1, b0, b1) < 0
}
