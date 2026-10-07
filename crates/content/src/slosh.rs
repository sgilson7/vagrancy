//! The liquid in the cursed blade's box (Sam, 2026-10-07: "whenever you
//! defeat an enemy, there should be a liquid that starts to build up in the
//! cursed blade weapon box that sloshes around"). Its surface is a row of
//! columns, each a height above the level and a speed: each column is drawn
//! toward its neighbors and back to the level, and keeps most of its speed.
//! A rise in the level throws the surface toward one side, and a push tips
//! it, so it slops about and settles.
//!
//! The page's picture, not the match: plain floating point, in core because
//! the page integrates nothing (CLAUDE.md), as `bubbles` is.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Slosh {
    /// How full, from 0 to 1, now.
    pub level: f64,
    /// Each column's height above the level, as a fraction of the box.
    pub h: Vec<f64>,
    /// Each column's speed.
    pub v: Vec<f64>,
}

/// Columns across the box.
pub const COLUMNS: usize = 24;
/// How hard a column is drawn toward its neighbors.
pub const COUPLE: f64 = 0.18;
/// How hard a column is drawn back to the level.
pub const SPRING: f64 = 0.012;
/// How much speed a column keeps each step.
pub const KEEP: f64 = 0.985;
/// How much of the way to its new level the liquid rises each step.
pub const RISE: f64 = 0.03;

impl Slosh {
    pub fn new(level: f64) -> Slosh {
        Slosh { level, h: vec![0.0; COLUMNS], v: vec![0.0; COLUMNS] }
    }
}

/// One step toward `target` fullness, tipped by `push` (positive raises the
/// right side). A state of the wrong shape starts over, still.
pub fn step(s: &mut Slosh, target: f64, push: f64) {
    let target = target.clamp(0.0, 1.0);
    if s.h.len() != COLUMNS || s.v.len() != COLUMNS {
        *s = Slosh::new(s.level.clamp(0.0, 1.0));
    }
    let rise = (target - s.level) * RISE;
    s.level += rise;
    // Liquid pouring in lands on the left and runs right.
    s.v[0] += rise * 3.0;
    let n = COLUMNS as f64;
    for i in 0..COLUMNS {
        let left = if i == 0 { s.h[i] } else { s.h[i - 1] };
        let right = if i + 1 == COLUMNS { s.h[i] } else { s.h[i + 1] };
        let tilt = push * (i as f64 / (n - 1.0) - 0.5);
        s.v[i] += COUPLE * (left + right - 2.0 * s.h[i]) - SPRING * s.h[i] + tilt;
        s.v[i] *= KEEP;
    }
    for i in 0..COLUMNS {
        s.h[i] = (s.h[i] + s.v[i]).clamp(-0.25, 0.25);
    }
    // The liquid neither appears nor goes: the surface is kept centred on
    // the level, so what one column gains the others give back. (Keeping
    // the speeds centred instead let a clamped column leave the whole
    // surface sitting off its level.)
    let mean = s.h.iter().sum::<f64>() / n;
    for x in &mut s.h {
        *x -= mean;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rise_in_the_level_sloshes_and_then_settles() {
        let mut s = Slosh::new(0.2);
        for _ in 0..30 {
            step(&mut s, 0.5, 0.0);
        }
        let rough = s.h.iter().fold(0.0f64, |m, x| m.max(x.abs()));
        assert!(rough > 0.005, "pouring in did not move the surface");
        for _ in 0..3000 {
            step(&mut s, 0.5, 0.0);
        }
        assert!((s.level - 0.5).abs() < 0.01, "the level did not reach its target");
        assert!(s.h.iter().all(|x| x.abs() < 0.002), "the surface did not settle");
    }

    #[test]
    fn the_surface_neither_gains_nor_loses_liquid() {
        let mut s = Slosh::new(0.4);
        for k in 0..500 {
            step(&mut s, 0.4, if k < 40 { 0.01 } else { 0.0 });
            let sum: f64 = s.h.iter().sum();
            assert!(sum.abs() < 1e-6, "the surface held {sum} more than its level");
        }
    }
}
