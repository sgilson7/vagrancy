//! Trees people write in the BT Lab's editor (Sam, 2026-10-07: "a custom
//! behavior tree editor that people can use to create their own
//! characters"). A written tree has the shape of a tree pilot in
//! data/pilots.json, and runs on the same pilot code; this module reads one,
//! refuses one it cannot run with the reason the page says, and builds the
//! setups it fights in. A tree submitted on the stream is read here too.

use pilot::{Cond, Rule, Spec};
use serde::Deserialize;

/// The limits on a written tree, so any tree that is accepted runs in time.
pub const MAX_RULES: usize = 12;
pub const MAX_CONDS: usize = 4;
pub const REACTION_TICKS: (u32, u32) = (4, 60);
pub const DISTANCE_CM: (i32, i32) = (0, 1000);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Written {
    #[serde(default)]
    #[allow(dead_code)]
    name: Option<String>,
    reaction_ticks: u32,
    rules: Vec<Rule>,
}

/// Why a written tree was refused: a copy key under `btlab.editor.refuse`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub key: &'static str,
    pub rule: Option<usize>,
}

fn refuse(key: &'static str, rule: Option<usize>) -> Refusal {
    Refusal { key, rule }
}

/// The pilot a written tree makes, or why it cannot be run.
pub fn parse(json: &str) -> Result<Spec, Refusal> {
    let w: Written = serde_json::from_str(json).map_err(|_| refuse("btlab.editor.refuse.shape", None))?;
    if !(REACTION_TICKS.0..=REACTION_TICKS.1).contains(&w.reaction_ticks) {
        return Err(refuse("btlab.editor.refuse.reaction", None));
    }
    if w.rules.is_empty() || w.rules.len() > MAX_RULES {
        return Err(refuse("btlab.editor.refuse.rules", None));
    }
    for (k, r) in w.rules.iter().enumerate() {
        if r.when.len() > MAX_CONDS {
            return Err(refuse("btlab.editor.refuse.conditions", Some(k)));
        }
        if pilot::moves::recipe(&r.act).is_none() {
            return Err(refuse("btlab.editor.refuse.move", Some(k)));
        }
        for c in &r.when {
            let ok = match c {
                Cond::GapAbove(v) | Cond::GapBelow(v) | Cond::TipNearHead(v) | Cond::TipNearBody(v) | Cond::AllyEngaged(v) | Cond::OppAbove(v) | Cond::OppBelow(v) => {
                    (DISTANCE_CM.0..=DISTANCE_CM.1).contains(v)
                }
                Cond::MyInkBelow(p) => (0..=100).contains(p),
                Cond::Chance(p) => *p <= 100,
                _ => true,
            };
            if !ok {
                return Err(refuse("btlab.editor.refuse.number", Some(k)));
            }
        }
    }
    Ok(Spec::Tree { reaction_ticks: w.reaction_ticks, rules: w.rules, salt: 900, search_period: None })
}

/// The conditions an editor offers: each one's name as data/pilots.json
/// writes it, and the unit of its number if it takes one.
pub const CONDITIONS: [(&str, Option<&str>); 19] = [
    ("gap_above", Some("cm")),
    ("gap_below", Some("cm")),
    ("tip_near_head", Some("cm")),
    ("tip_near_body", Some("cm")),
    ("me_airborne", None),
    ("me_grounded", None),
    ("opp_airborne", None),
    ("me_down", None),
    ("opp_dodging", None),
    ("my_ink_below", Some("pct")),
    ("chance", Some("pct")),
    ("ally_engaged", Some("cm")),
    ("opp_above", Some("cm")),
    ("opp_below", Some("cm")),
    ("ledge_overhead", None),
    ("on_ledge", None),
    ("armed", None),
    ("unarmed", None),
    ("opp_down", None),
];

/// A written tree's fighter, with the sword, against a road opponent in its
/// road body, on a map: the tree on the left.
pub fn watch(seed: u64, tuning: u8, opponent: &str, map: &str) -> sim::Setup {
    let mut s = crate::setup::exhibition(seed, tuning, [opponent, opponent], map);
    if let Some(seat) = s.seats[0].as_mut() {
        seat.body = crate::setup::FIGHTER;
    }
    s
}

/// A person, with the sword, against a written tree's fighter.
pub fn play(seed: u64, tuning: u8, map: &str) -> sim::Setup {
    crate::setup::versus_with(seed, tuning, [crate::weapons::DEFAULT, crate::weapons::DEFAULT], map)
}
