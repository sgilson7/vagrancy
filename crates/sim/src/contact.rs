//! Swept contact between a moving segment and a moving capsule (D10).
//!
//! A fast blade can jump a limb between two ticks (H2; gear-master-2d
//! SECOND-ORDER-M22 row 1, "a one-tile wall is not a wall to a ball"), so
//! contact is never tested only where things ended up. Both endpoints of both
//! segments are interpolated linearly from where they started the tick to
//! where they ended it, in `S` substeps, with
//!
//! ```text
//! S = ceil(largest relative endpoint travel / capsule radius)
//! ```
//!
//! capped by `s_max`, which the caller derives from the speed cap. The first
//! substep at which the segments come within the radius is the contact.
//! Everything is integer.

use crate::fx::{narrow, Fx, V2, ONE};

/// A segment at the start and at the end of a tick.
#[derive(Copy, Clone, Debug)]
pub struct Swept {
    pub a0: V2,
    pub b0: V2,
    pub a1: V2,
    pub b1: V2,
}

impl Swept {
    fn at(&self, k: i64, s: i64) -> (V2, V2) {
        (self.a0 + (self.a1 - self.a0).scale(k, s), self.b0 + (self.b1 - self.b0).scale(k, s))
    }
}

/// Where a swept contact happened.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Hit {
    /// The substep, `0..=s`, and how many there were.
    pub k: u32,
    pub s: u32,
    /// The fraction along the capsule's spine (`a` toward `b`) nearest the blade.
    pub f: Fx,
    /// That point, at the substep of contact.
    pub at: V2,
    /// The fraction along the blade.
    pub g: Fx,
}

/// How many substeps a pair needs: the larger of the relative travels of
/// the blade's ends against the capsule's ends, over the radius, rounded up.
pub fn substeps(blade: &Swept, part: &Swept, radius: Fx, s_max: u32) -> u32 {
    let travel = |p0: V2, p1: V2, q0: V2, q1: V2| ((p1 - p0) - (q1 - q0)).len();
    let d = [
        travel(blade.a0, blade.a1, part.a0, part.a1),
        travel(blade.a0, blade.a1, part.b0, part.b1),
        travel(blade.b0, blade.b1, part.a0, part.a1),
        travel(blade.b0, blade.b1, part.b0, part.b1),
    ]
    .into_iter()
    .max()
    .unwrap();
    let r = radius.0.max(1) as i64;
    (((d.0 as i64 + r - 1) / r).max(1) as u32).min(s_max.max(1))
}

/// The first substep at which `blade` comes within `radius` of `part`'s spine,
/// starting from substep `from` (0 includes the start of the tick).
pub fn sweep(blade: &Swept, part: &Swept, radius: Fx, s_max: u32, from: u32) -> Option<Hit> {
    let s = substeps(blade, part, radius, s_max);
    let r2 = radius.0 as i64 * radius.0 as i64;
    for k in from..=s {
        let (p0, p1) = blade.at(k as i64, s as i64);
        let (q0, q1) = part.at(k as i64, s as i64);
        let (g, f, pc, qc) = closest(p0, p1, q0, q1);
        if (pc - qc).len_sq_raw() <= r2 {
            return Some(Hit { k, s, f, at: qc, g });
        }
    }
    None
}

/// Whether two segments come within `gap` of each other at the end of the
/// tick, and if so the closest points.
pub fn near(p0: V2, p1: V2, q0: V2, q1: V2, gap: Fx) -> Option<(Fx, Fx, V2, V2)> {
    let c = closest(p0, p1, q0, q1);
    let gap2 = gap.0 as i64 * gap.0 as i64;
    ((c.2 - c.3).len_sq_raw() <= gap2).then_some(c)
}

/// Closest points between segments `p0→p1` and `q0→q1`: the fractions along
/// each, as `Fx` in `0..=ONE`, and the points. After Ericson, *Real-Time
/// Collision Detection* §5.1.9, in integers. Parallel segments that overlap
/// return the middle of the overlap, so a blade lying along a limb cuts it in
/// the middle of where they touch rather than at an end chosen by rounding.
pub fn closest(p0: V2, p1: V2, q0: V2, q1: V2) -> (Fx, Fx, V2, V2) {
    let d1 = p1 - p0;
    let d2 = q1 - q0;
    let r = p0 - q0;
    let a = d1.dot_raw(d1);
    let e = d2.dot_raw(d2);
    let f = d2.dot_raw(r);
    let one = ONE.0 as i64;
    let clamp01 = |num: i64, den: i64| -> i64 {
        if den == 0 {
            0
        } else {
            // num/den in Fx, clamped to [0, ONE], computed in i128 to keep the
            // products of squared raw lengths exact.
            let v = (num as i128 * one as i128 / den as i128) as i64;
            v.clamp(0, one)
        }
    };
    let (s, t);
    if a == 0 && e == 0 {
        return (Fx(0), Fx(0), p0, q0);
    }
    if a == 0 {
        s = 0;
        t = clamp01(f, e);
    } else {
        let c = d1.dot_raw(r);
        if e == 0 {
            t = 0;
            s = clamp01(-c, a);
        } else {
            let b = d1.dot_raw(d2);
            let denom = a as i128 * e as i128 - b as i128 * b as i128;
            if denom.abs() <= (a as i128 * e as i128) >> 24 {
                // Parallel, or nearly. Project q's ends onto p's line and take
                // the middle of the overlap.
                let s_q0 = clamp01(-c, a);
                let s_q1 = clamp01(d1.dot_raw(q1 - p0), a);
                s = (s_q0 + s_q1) / 2;
                let ps = p0 + d1.scale(s, one);
                t = clamp01(d2.dot_raw(ps - q0), e);
            } else {
                let mut sv = ((b as i128 * f as i128 - c as i128 * e as i128) * one as i128 / denom) as i64;
                sv = sv.clamp(0, one);
                // t = (b·s + f) / e, with s as a fraction.
                let tn = (b as i128 * sv as i128 / one as i128) as i64 + f;
                let mut tv = (tn as i128 * one as i128 / e as i128) as i64;
                if tv < 0 {
                    tv = 0;
                    sv = clamp01(-c, a);
                } else if tv > one {
                    tv = one;
                    sv = clamp01(b - c, a);
                }
                s = sv;
                t = tv;
            }
        }
    }
    let ps = p0 + d1.scale(s, one);
    let qt = q0 + d2.scale(t, one);
    (Fx(narrow(s)), Fx(narrow(t)), ps, qt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cm(x: i32, y: i32) -> V2 {
        V2::cm(x, y)
    }

    /// H2 — a blade that jumps a limb. The limb's spine runs from (4, 0) to
    /// (4, 10), radius 1. A blade of length 10 along y = 5 spans x = −10..0 on
    /// tick k and x = 10..20 on tick k + 1 (it moves +20). At neither end of
    /// the tick does it touch the limb (gaps 3 and 5). The cut must land at
    /// (4, 5), the middle of the spine. A test of end-of-tick positions finds
    /// nothing.
    #[test]
    fn h2_a_blade_that_jumps_a_limb_cuts_it_in_the_middle() {
        let blade = Swept { a0: cm(-10, 5), b0: cm(0, 5), a1: cm(10, 5), b1: cm(20, 5) };
        let limb = Swept { a0: cm(4, 0), b0: cm(4, 10), a1: cm(4, 0), b1: cm(4, 10) };
        assert!(near(blade.a0, blade.b0, limb.a0, limb.b0, ONE).is_none(), "touching at the start");
        assert!(near(blade.a1, blade.b1, limb.a1, limb.b1, ONE).is_none(), "touching at the end");
        let hit = sweep(&blade, &limb, ONE, 64, 1).expect("the swept test finds the limb");
        assert_eq!(hit.at, cm(4, 5), "the cut lands at the middle of the spine");
        assert_eq!(hit.f, Fx(ONE.0 / 2));
    }

    /// H2b — the agent's case (PLAN.md §4). H2's blade moves along its own
    /// length, so any substep under 12 catches it. Here a blade from (0, 3) to
    /// (0, 7) moves +20 across a limb at x = 10, radius 1: substeps of 4
    /// sample x = 4, 8, 12, 16, 20 and miss. S = 20 / 1 = 20, and the cut
    /// lands at (10, 5).
    #[test]
    fn h2b_a_blade_moving_sideways_is_not_missed_between_substeps() {
        let blade = Swept { a0: cm(0, 3), b0: cm(0, 7), a1: cm(20, 3), b1: cm(20, 7) };
        let limb = Swept { a0: cm(10, 0), b0: cm(10, 10), a1: cm(10, 0), b1: cm(10, 10) };
        assert_eq!(substeps(&blade, &limb, ONE, 64), 20);
        let coarse = sweep(&blade, &limb, ONE, 5, 1);
        assert!(coarse.is_none(), "five substeps of 4 cm should miss it, and found {coarse:?}");
        let hit = sweep(&blade, &limb, ONE, 64, 1).expect("twenty substeps find it");
        assert_eq!(hit.at, cm(10, 5));
    }

    #[test]
    fn closest_points_of_crossing_and_apart_segments() {
        let (s, t, p, q) = closest(cm(0, 0), cm(10, 0), cm(5, -5), cm(5, 5));
        assert_eq!((s, t, p, q), (Fx(ONE.0 / 2), Fx(ONE.0 / 2), cm(5, 0), cm(5, 0)));
        let (_, _, p, q) = closest(cm(0, 0), cm(10, 0), cm(20, 3), cm(30, 3));
        assert_eq!((p, q), (cm(10, 0), cm(20, 3)));
    }
}
