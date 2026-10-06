//! What the page draws: positions and nothing it would have to work out.
//!
//! The page may interpolate between two of these; it may not integrate,
//! predict or detect a contact (D2). Positions are raw fixed point
//! (1/4096 cm); `numbers()` in the shim gives the page the scale.

use crate::world::World;
use serde::Serialize;

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub tick: u32,
    pub points: Vec<[i32; 2]>,
    pub parts: Vec<PartView>,
    pub swords: Vec<SwordView>,
    pub fighters: Vec<Option<FighterView>>,
    /// Ledges, as `[x0, x1, y]` in raw fixed point.
    pub platforms: Vec<[i32; 3]>,
    /// Half the arena's width, raw: wider than a screen on a stage to cross.
    pub arena_half: i32,
    /// Where a stage to cross ends, raw, if this is one.
    pub exit: Option<i32>,
    pub round: u32,
    pub wins: [u32; 2],
    /// "fight", "round_over" or "match_over".
    pub phase: &'static str,
    pub events: Vec<crate::fight::Event>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct PartView {
    pub a: u16,
    pub b: u16,
    pub r: i32,
    pub fighter: u8,
    /// Which body definition: 0 a fighter, 1 a post. The page picks colors
    /// by it, from the palette.
    pub body: u8,
    pub attached: bool,
    pub stump: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct SwordView {
    pub butt: u16,
    pub tip: u16,
    /// Where the hilt ends, as raw fixed-point fraction of the way to the tip.
    pub hilt: i32,
    pub fighter: u8,
    pub held: bool,
    /// Thrown and still cutting.
    pub flying: bool,
    /// A boomerang a blade met in flight: it can cut its thrower.
    pub turned: bool,
    /// Every cutting edge, as two particle indices: one for a plain sword.
    pub edges: Vec<[u16; 2]>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FighterView {
    pub body: u8,
    /// 0 for seat 0, 1 for its opponents: the page colors a fighter by it.
    pub side: u8,
    pub facing: i32,
    pub ink: i32,
    pub ink_max: i32,
    pub dodging: bool,
}

pub fn frame(w: &World) -> Frame {
    Frame {
        tick: w.tick,
        points: w.particles.iter().map(|p| [p.p.x.0, p.p.y.0]).collect(),
        parts: w
            .parts
            .iter()
            .map(|p| PartView {
                a: p.a,
                b: p.b,
                r: p.radius.0,
                fighter: p.fighter,
                body: w.fighters[p.fighter as usize].as_ref().map(|f| f.body).unwrap_or(0),
                attached: p.attached,
                stump: p.stump,
            })
            .collect(),
        swords: w
            .swords
            .iter()
            .enumerate()
            .map(|(k, s)| SwordView {
                butt: s.butt,
                tip: s.tip,
                hilt: (crate::fx::Fx::ratio(s.hilt.0 as i64, s.len.0.max(1) as i64)).0,
                fighter: s.fighter,
                held: w.held(k),
                edges: s.edges.iter().map(|e| [e.0, e.1]).collect(),
                flying: s.flying,
                turned: s.turned && s.flying,
            })
            .collect(),
        fighters: w
            .fighters
            .iter()
            .map(|f| {
                f.as_ref().map(|f| FighterView {
                    body: f.body,
                    side: f.side,
                    facing: f.facing,
                    ink: f.ink,
                    ink_max: w.setup.bodies[f.body as usize].ink,
                    dodging: f.dodge > 0,
                })
            })
            .collect(),
        platforms: w.setup.platforms.iter().map(|p| [p.x0.0, p.x1.0, p.y.0]).collect(),
        arena_half: w.setup.arena_half.0,
        exit: match w.setup.objective {
            crate::body::Objective::Reach { x } => Some(x.0),
            _ => None,
        },
        round: w.round,
        wins: w.wins,
        phase: match w.phase {
            crate::fight::Phase::Fight => "fight",
            crate::fight::Phase::RoundOver { .. } => "round_over",
            crate::fight::Phase::MatchOver { .. } => "match_over",
        },
        events: w.events.clone(),
    }
}
