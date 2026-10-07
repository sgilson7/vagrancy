//! Where the behavior trees float over the arena (Sam, 2026-10-06: two trees
//! "often overlapping each other, so they should act as floating bubbles
//! that push each other away, a little like the way the slush works in the
//! slushline game"). After SlurpeeGame's `sim::slush::contact`: each bubble
//! is a point with where it is and where it was, so it keeps its motion
//! from one frame to the next; a spring draws it toward its place over its
//! fighter; and two that overlap are pushed apart along the line between
//! them, the push split between the two.
//!
//! This is the page's picture, not the match: nothing here reaches the
//! world, so it is plain floating point. It lives in core because the page
//! integrates nothing (CLAUDE.md).

use serde::{Deserialize, Serialize};

/// One tree's card: its centre now (`x`, `y`) and a frame ago (`px`, `py`),
/// its size, and its home, centred over its fighter's head. Pixels.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bubble {
    pub x: f64,
    pub y: f64,
    pub px: f64,
    pub py: f64,
    pub w: f64,
    pub h: f64,
    pub hx: f64,
    pub hy: f64,
}

/// How much of last frame's motion a bubble keeps.
pub const KEEP: f64 = 0.8;
/// How hard a bubble is drawn home, as a fraction of the distance a frame.
pub const SPRING: f64 = 0.08;
/// The space kept between two bubbles, and between a bubble and the edge.
pub const GAP: f64 = 8.0;
/// Passes of pushing apart each frame.
pub const PASSES: usize = 8;

/// One frame: each bubble moves on and toward home, then overlapping pairs
/// are pushed apart, then each is kept inside the `w` by `h` area.
pub fn step(bs: &mut [Bubble], w: f64, h: f64) {
    for b in bs.iter_mut() {
        let (vx, vy) = ((b.x - b.px) * KEEP, (b.y - b.py) * KEEP);
        b.px = b.x;
        b.py = b.y;
        b.x += vx + (b.hx - b.x) * SPRING;
        b.y += vy + (b.hy - b.y) * SPRING;
    }
    for _ in 0..PASSES {
        for i in 0..bs.len() {
            for j in i + 1..bs.len() {
                let (a, b) = (bs[i], bs[j]);
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let ox = (a.w + b.w) / 2.0 + GAP - dx.abs();
                let oy = (a.h + b.h) / 2.0 + GAP - dy.abs();
                if ox <= 0.0 || oy <= 0.0 {
                    continue;
                }
                // Apart along the axis that frees them soonest; exactly on
                // top of each other, the lower index goes left or up, so
                // the choice depends on nothing else.
                let (mx, my) = if ox < oy { (if dx < 0.0 { -ox } else { ox }, 0.0) } else { (0.0, if dy < 0.0 { -oy } else { oy }) };
                bs[i].x -= mx / 2.0;
                bs[i].y -= my / 2.0;
                bs[j].x += mx / 2.0;
                bs[j].y += my / 2.0;
            }
        }
        // Inside each pass, so a bubble held by an edge leaves the next
        // pass to move the other one the rest of the way: kept to the end,
        // the edge pushed a bubble back into the one it had left.
        for b in bs.iter_mut() {
            inside(b, w, h);
        }
    }
}

fn inside(b: &mut Bubble, w: f64, h: f64) {
    b.x = b.x.clamp(b.w / 2.0 + GAP, (w - b.w / 2.0 - GAP).max(b.w / 2.0 + GAP));
    b.y = b.y.clamp(b.h / 2.0 + GAP, (h - b.h / 2.0 - GAP).max(b.h / 2.0 + GAP));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f64, y: f64, w: f64, h: f64) -> Bubble {
        Bubble { x, y, px: x, py: y, w, h, hx: x, hy: y }
    }

    fn apart(a: &Bubble, b: &Bubble) -> bool {
        (a.x - b.x).abs() >= (a.w + b.w) / 2.0 + GAP - 0.5 || (a.y - b.y).abs() >= (a.h + b.h) / 2.0 + GAP - 0.5
    }

    #[test]
    fn two_trees_with_the_same_home_float_apart_and_stay_apart() {
        let mut bs = [at(400.0, 120.0, 300.0, 150.0), at(400.0, 120.0, 260.0, 120.0)];
        for _ in 0..120 {
            step(&mut bs, 960.0, 540.0);
        }
        assert!(apart(&bs[0], &bs[1]), "they still overlap: {bs:?}");
        // And settled: a frame later they hardly move.
        let before = bs;
        step(&mut bs, 960.0, 540.0);
        for k in 0..2 {
            assert!((bs[k].x - before[k].x).abs() < 1.0 && (bs[k].y - before[k].y).abs() < 1.0, "bubble {k} is still moving");
        }
    }

    #[test]
    fn a_tree_alone_goes_home_and_a_moving_home_is_followed() {
        let mut bs = [at(100.0, 100.0, 200.0, 100.0)];
        bs[0].hx = 500.0;
        bs[0].hy = 200.0;
        for _ in 0..180 {
            step(&mut bs, 960.0, 540.0);
        }
        assert!((bs[0].x - 500.0).abs() < 2.0 && (bs[0].y - 200.0).abs() < 2.0, "{:?}", bs[0]);
    }

    #[test]
    fn a_bubble_stays_inside_the_arena() {
        let mut bs = [at(10.0, 10.0, 300.0, 200.0), at(950.0, 530.0, 300.0, 200.0)];
        bs[0].hx = -500.0;
        bs[1].hx = 2000.0;
        for _ in 0..60 {
            step(&mut bs, 960.0, 540.0);
            for b in &bs {
                assert!(b.x - b.w / 2.0 >= GAP - 0.01 && b.x + b.w / 2.0 <= 960.0 - GAP + 0.01, "{b:?}");
                assert!(b.y - b.h / 2.0 >= GAP - 0.01 && b.y + b.h / 2.0 <= 540.0 - GAP + 0.01, "{b:?}");
            }
        }
    }
}
