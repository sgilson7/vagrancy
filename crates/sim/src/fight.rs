//! Cuts, clashes, ink, and how a round and a match end (D10–D13).

use crate::balance;
use crate::body::{Cause, Mode, SEATS};
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
    /// The side that lost (0 is seat 0; 1 is its opponents); `None` when
    /// both stopped on the same tick (a draw, played again). With two seats
    /// a side is a seat.
    pub loser: Option<u8>,
    /// The fighter whose fall decided it: the last of the losing side out.
    pub seat: u8,
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
    RoundOver { result: RoundResult, ready: [bool; SEATS] },
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

    /// The cutting part of one edge of a weapon: from where it begins to
    /// cut to its end. A plain sword's one edge runs from the end of the
    /// hilt to the tip.
    fn edge(&self, (a, b, from): (u16, u16, Fx)) -> Swept {
        let whole = self.swept(a, b);
        Swept { a0: V2::lerp(whole.a0, whole.b0, from), b0: whole.b0, a1: V2::lerp(whole.a1, whole.b1, from), b1: whole.b1 }
    }

    /// A sword cuts only while a hand holds it (PLAN.md §8 Q15).
    pub fn held(&self, si: usize) -> bool {
        let f = self.swords[si].fighter;
        self.cons.iter().any(|c| matches!(c.tag, Tag::Grip { fighter, .. } if fighter == f))
    }

    /// Whether sword `si` can cut part `pi` at all: the sword is held (Q15)
    /// or thrown and not yet down,
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
        // Nor a fighter on its own side. A fighter already out of the round
        // cuts nothing and is cut no more: with three seats it lies there
        // while the round goes on, and a blade resting in it cut it into
        // slivers every tick until the particle count overflowed
        // (SECOND-ORDER-M5 row 49).
        self.side_of(part.fighter as usize) != self.side_of(owner as usize)
            && (self.held(si) || self.swords[si].flying)
            && !self.dodging(owner as usize)
            && !dodged
            && !self.out(owner as usize)
            && !self.out(part.fighter as usize)
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
            // A held sword cuts, and so does a thrown one until it is down.
            if !self.held(si) && !self.swords[si].flying {
                continue;
            }
            let owner = self.swords[si].fighter;
            let n = self.parts.len();
            for pi in 0..n {
                // The first edge, in the weapon's order, that touches the
                // part is the one that cuts it.
                for e in self.swords[si].edges.clone() {
                    let blade = self.edge(e);
                    let part = self.parts[pi].clone();
                    if !self.may_cut(si, pi) {
                        break;
                    }
                    let swept = self.swept(part.a, part.b);
                    if !boxes_meet(&blade, &swept, part.radius) {
                        continue;
                    }
                    let Some(hit) = contact::sweep(&blade, &swept, part.radius, s_max, 0) else { continue };
                    now.push((si as u8, pi as u16));
                    if self.touching.contains(&(si as u8, pi as u16)) {
                        break;
                    }
                    // A blade that drifts into a part rests against it; a
                    // blade that arrives at speed cuts it. Without this every
                    // fighter cut itself within a second of standing still
                    // (M3.0).
                    let (speed, _) = self.contact_speed(e, &part, hit);
                    if speed < balance::MIN_CUT_SPEED {
                        break;
                    }
                    let f = hit.f.clamp(Fx::ratio(1, 20), Fx::ratio(19, 20));
                    if let Some(far) = self.cut(pi, f, owner) {
                        now.push((si as u8, far));
                    }
                    break;
                }
            }
        }
        self.touching = now;
    }

    /// How fast the blade's touching point moves against the part's touching
    /// point, cm per tick; and how much of that is along the blade toward the
    /// tip (a thrust).
    fn contact_speed(&self, (a, b, from): (u16, u16, Fx), part: &Part, hit: contact::Hit) -> (Fx, Fx) {
        // The hit's `g` is along the cutting part; map it back onto the edge.
        let g = from + (ONE - from) * hit.g;
        let vel = |i: u16| self.particles[i as usize].p - self.particles[i as usize].q;
        let blade_v = V2::lerp(vel(a), vel(b), g);
        let part_v = V2::lerp(vel(part.a), vel(part.b), hit.f);
        let rel = blade_v - part_v;
        let axis = (self.particles[b as usize].p - self.particles[a as usize].p).with_len(ONE);
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
                Con::Pin { a, b, c, extra, .. } => [a, b, c].into_iter().chain((0..extra as u16).map(|k| c + 1 + k)).collect(),
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
        // Every pair of swords, in index order: with two seats, the one pair.
        let n = self.swords.len();
        for ia in 0..n {
            for ib in ia + 1..n {
                self.clash_pair(ia, ib, drives);
            }
        }
    }

    fn is_clashing(&self, ia: usize, ib: usize) -> bool {
        self.clashing.contains(&(ia as u8, ib as u8))
    }

    fn set_clashing(&mut self, ia: usize, ib: usize, on: bool) {
        let k = (ia as u8, ib as u8);
        self.clashing.retain(|&x| x != k);
        if on {
            self.clashing.push(k);
            self.clashing.sort();
        }
    }

    fn clash_pair(&mut self, ia: usize, ib: usize, drives: &[Drive]) {
        let s_max = self.s_max().max(8);
        // Every edge of one weapon against every edge of the other, whole,
        // hilt included: a plain sword is one edge, butt to tip.
        let ends = |si: usize| self.swords[si].edges.iter().map(|e| (e.0, e.1)).collect::<Vec<_>>();
        let (ea, eb) = (ends(ia), ends(ib));
        let pairs: Vec<((u16, u16), (u16, u16))> = ea.iter().flat_map(|&x| eb.iter().map(move |&y| (x, y))).collect();
        let points: Vec<u16> = self.swords[ia].points.iter().chain(self.swords[ib].points.iter()).copied().collect();
        // Blades that start the tick already touching may slide along each
        // other and part freely; they are only stopped if they would cross.
        // Without this they glued together: every tick found contact at its
        // first substep and went back to where it began.
        let resting = pairs.iter().any(|&(x, y)| {
            let (a, b) = (self.swept(x.0, x.1), self.swept(y.0, y.1));
            contact::near(a.a0, a.b0, b.a0, b.b0, BLADE_GAP).is_some()
        });
        let p = |w: &World, i: u16| w.particles[i as usize].p;
        let crossing = |w: &World| pairs.iter().copied().find(|&(x, y)| segments_cross(p(w, x.0), p(w, x.1), p(w, y.0), p(w, y.1)));
        let near_now = |w: &World| pairs.iter().copied().find(|&(x, y)| contact::near(p(w, x.0), p(w, x.1), p(w, y.0), p(w, y.1), BLADE_GAP).is_some());
        let (k, s, pair) = if resting {
            let Some(pair) = crossing(self) else {
                let near = near_now(self).is_some();
                self.set_clashing(ia, ib, near);
                return;
            };
            (0i64, 1i64, pair)
        } else {
            // The earliest contact of any pair of edges.
            let mut first: Option<(u32, u32, ((u16, u16), (u16, u16)))> = None;
            for &(x, y) in &pairs {
                let (a, b) = (self.swept(x.0, x.1), self.swept(y.0, y.1));
                if let Some(hit) = contact::sweep(&a, &b, BLADE_GAP, s_max, 1) {
                    let earlier = first.is_none_or(|(k, s, _)| (hit.k as u64) * (s as u64) < (k as u64) * (hit.s as u64));
                    if earlier {
                        first = Some((hit.k, hit.s, (x, y)));
                    }
                }
            }
            let Some((hk, hs, pair)) = first else {
                self.set_clashing(ia, ib, false);
                return;
            };
            (hk.saturating_sub(1) as i64, hs as i64, pair)
        };
        // Both weapons, every point, back to that substep.
        let safe: Vec<(u16, V2)> = points
            .iter()
            .map(|&i| {
                let pt = &self.particles[i as usize];
                (i, pt.q + (pt.p - pt.q).scale(k, s))
            })
            .collect();
        let put = |w: &mut World| {
            for &(i, at) in &safe {
                w.particles[i as usize].p = at;
            }
        };
        put(self);
        self.relax(drives);
        self.relax(drives);
        let ok = if resting { crossing(self).is_none() } else { near_now(self).is_none() };
        if !ok {
            put(self);
        }
        if !self.is_clashing(ia, ib) {
            let at = |i: u16| safe.iter().find(|(j, _)| *j == i).map(|(_, v)| *v).unwrap();
            let ((a0, a1), (b0, b1)) = pair;
            let (_, _, pe, qe) = contact::closest(at(a0), at(a1), at(b0), at(b1));
            self.events.push(Event::Clash { at: V2::lerp(pe, qe, Fx(ONE.0 / 2)) });
        }
        self.set_clashing(ia, ib, true);
    }

    // --- ink and the end of a round (D12, D13) -----------------------------------------

    /// Every attached stump drains ink, once, at the end of every tick,
    /// including the tick its cut lands on (H4).
    pub(crate) fn drain(&mut self) {
        for seat in 0..SEATS {
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

    /// Whether a side cannot continue, and what follows (D12, D13). A side
    /// is beaten when every fighter on it is out; the fighter who went out
    /// last decides how the round is told.
    /// Which side a seat fights on: its fighter's, or the seat's own.
    pub fn side_of(&self, seat: usize) -> u8 {
        self.fighters[seat].as_ref().map(|f| f.side).unwrap_or(crate::body::side(seat))
    }

    /// Story mode's stage to cross: seat 0's pelvis has got to the end.
    fn reached(&self) -> bool {
        let crate::body::Objective::Reach { x } = self.setup.objective else { return false };
        let Some(f) = self.fighters[0].as_ref() else { return false };
        let def = &self.setup.bodies[f.body as usize];
        def.roles.pelvis.is_some_and(|p| self.particles[(f.base + p as u16) as usize].p.x >= x)
    }

    pub(crate) fn judge(&mut self) {
        if self.setup.mode != Mode::Match || !matches!(self.phase, Phase::Fight) {
            return;
        }
        let tick = self.tick;
        let mut fell: [Option<(u32, RoundResult)>; SEATS] = [None, None, None];
        for seat in 0..SEATS as u8 {
            let Some(f) = self.fighters[seat as usize].as_ref() else { continue };
            if self.setup.bodies[f.body as usize].ink == 0 {
                continue;
            }
            let side = f.side;
            let r = if let Some(fatal) = f.fatal {
                Some(RoundResult { loser: Some(side), seat, cause: fatal.cause, part: fatal.part, by: fatal.by })
            } else if f.ink <= 0 {
                let most = (0..f.spilled.len()).max_by_key(|&k| (f.spilled[k], std::cmp::Reverse(k))).unwrap_or(0) as u8;
                Some(RoundResult { loser: Some(side), seat, cause: Cause::Ink, part: most, by: seat })
            } else {
                None
            };
            if let Some(r) = r {
                let f = self.fighters[seat as usize].as_mut().unwrap();
                let at = *f.out_at.get_or_insert(tick);
                fell[seat as usize] = Some((at, r));
            }
        }
        // Each side's result, if every fighter on it is out: the last out.
        let side_result = |side: u8| -> Option<RoundResult> {
            let seats: Vec<usize> = (0..SEATS).filter(|&s| self.fighters[s].as_ref().is_some_and(|f| f.side == side && self.setup.bodies[f.body as usize].ink > 0)).collect();
            if seats.is_empty() || seats.iter().any(|&s| fell[s].is_none()) {
                return None;
            }
            seats.iter().map(|&s| fell[s].unwrap()).max_by_key(|(at, r)| (*at, std::cmp::Reverse(r.seat))).map(|(_, r)| r)
        };
        let result = match (side_result(0), side_result(1)) {
            (None, None) if self.reached() => RoundResult { loser: Some(1), seat: 1, cause: Cause::Reached, part: 0, by: 0 },
            (None, None) if matches!(self.setup.objective, crate::body::Objective::HoldOut { ticks } if self.round_ticks + 1 >= ticks) => {
                // Story mode's hold-out: seat 0 is still in the round when
                // the time is up, so its side has won it.
                RoundResult { loser: Some(1), seat: 1, cause: Cause::HeldOut, part: 0, by: 0 }
            }
            (None, None) => {
                // With every sword thrown and down, nobody can cut anybody,
                // so the round could not end (Sam, 2026-10-05): after
                // DISARMED_DRAW_TICKS of it, it is a draw. Ink still runs
                // out meanwhile, and a fighter who empties loses as ever.
                let bladed = (0..SEATS).any(|s| {
                    self.fighters[s].as_ref().is_some_and(|f| self.setup.bodies[f.body as usize].ink > 0)
                        && !self.out(s)
                        && self.swords.iter().position(|w| w.fighter as usize == s).is_some_and(|si| self.held(si) || self.swords[si].flying)
                });
                if bladed {
                    self.disarmed_since = None;
                    return;
                }
                let since = *self.disarmed_since.get_or_insert(tick);
                if tick - since < balance::DISARMED_DRAW_TICKS {
                    return;
                }
                RoundResult { loser: None, seat: 0, cause: Cause::Disarmed, part: 0, by: 0 }
            }
            (Some(a), Some(_)) => RoundResult { loser: None, ..a },
            (Some(r), None) | (None, Some(r)) => r,
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
            None => Phase::RoundOver { result, ready: [false; SEATS] },
        };
    }

    /// Between rounds: latch each seat's ready bit, and start the next round
    /// once every seat with a fighter has pressed it. Returns whether the
    /// fighters should be limp this tick.
    pub(crate) fn between_rounds(&mut self, inputs: [Input; SEATS]) -> bool {
        match self.phase {
            Phase::Fight => false,
            Phase::MatchOver { .. } => true,
            Phase::RoundOver { result, mut ready } => {
                for s in 0..SEATS {
                    if inputs[s].has(Input::READY) || self.setup.seats[s].is_none() {
                        ready[s] = true;
                    }
                }
                if ready == [true; SEATS] {
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
