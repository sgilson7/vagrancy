//! The weapons a player can carry (Sam, 2026-10-04, after Soul Calibur II's
//! Weapon Master): each reshapes the sword from data/body.json, and each but
//! the sword is unlocked by a requirement on the road tree.

use crate::road::{Best, Req};
use serde::Deserialize;
use sim::body::{BladeEdge, SwordDef};
use sim::fx::{Fx, V2};
use std::collections::BTreeMap;

pub const WEAPONS_JSON: &str = include_str!("../../../data/weapons.json");
pub const DEFAULT: &str = "sword";

/// The opponents a weapon is measured against: no search among them, so a
/// run is quick, and between them every kind of attack the road has.
pub const PANEL: &[&str] = &["thresher", "drover", "lamplighter", "ferryman", "windmill", "gatekeeper", "cooper", "bellringer"];

/// What analysis/weapons.md was measured on.
pub fn fingerprint() -> String {
    let data = format!("{}{}{}{}", WEAPONS_JSON, crate::road::PILOTS_JSON, PANEL.join(","), sim::SIM_VERSION);
    format!("{:016x}", sim::world::fnv1a(data.as_bytes()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[allow(dead_code)]
    _about: String,
    weapons: Vec<Weapon>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Weapon {
    pub id: String,
    pub length_pct: i64,
    pub mass_pct: i64,
    /// The hilt, which does not cut, as a percentage of the sword's.
    #[serde(default = "hundred")]
    pub hilt_pct: i64,
    pub shape: Shape,
    #[serde(default)]
    pub unlock: Option<Req>,
    /// Carried only by opponents (data/road.json's `weapon`), never chosen.
    #[serde(default)]
    pub enemy_only: bool,
    /// Thrown, it comes back to the hand (the boomerang).
    #[serde(default)]
    pub returns: bool,
    /// The final fight's prize (the cursed blade), won only after every
    /// other fight.
    #[serde(default)]
    pub prize: bool,
    /// How the player hears where it is: from the start, or from a villager
    /// they beat (Sam, 2026-10-07: "rumors you acquire from defeating
    /// enemies that can make the weapon boxes appear and tell you who to
    /// defeat to find them"). A weapon with none is not shown until won.
    #[serde(default)]
    pub rumor: Option<Rumor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Rumor {
    /// Known when the player arrives.
    Start,
    /// Heard from this villager once beaten.
    Beat(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Shape {
    Straight {},
    /// Points the blade bends through, each `[along, across]` as percentages
    /// of the length from the butt.
    Curve(Vec<[i64; 2]>),
    /// Prongs from a fork on the line from butt to tip.
    Prongs { fork_pct: i64, prongs: Vec<[i64; 2]> },
    /// A ring whose diameter runs from butt to tip, cutting all the way
    /// round (the chakram).
    Ring {},
    /// A second blade out behind the hands, `back_pct` of the front one's
    /// length, with the handle between them (the twin blade).
    Twin { back_pct: i64 },
}

fn hundred() -> i64 {
    100
}

pub fn weapons() -> Vec<Weapon> {
    serde_json::from_str::<File>(WEAPONS_JSON).expect("data/weapons.json is valid").weapons
}

pub fn weapon(id: &str) -> Option<Weapon> {
    weapons().into_iter().find(|w| w.id == id)
}

/// Whether a player with these results may carry it.
pub fn unlocked(w: &Weapon, best: &BTreeMap<String, Best>) -> bool {
    !w.enemy_only && w.unlock.as_ref().is_none_or(|r| r.met(best))
}

/// Whether the player has found the cursed blade: won the fight that gives it.
pub fn prize_found(best: &BTreeMap<String, Best>) -> bool {
    weapons().iter().any(|w| w.prize && unlocked(w, best))
}

/// Whether the player has heard where this weapon is.
pub fn rumored(w: &Weapon, best: &BTreeMap<String, Best>) -> bool {
    match &w.rumor {
        Some(Rumor::Start) => true,
        Some(Rumor::Beat(id)) => best.contains_key(id),
        None => false,
    }
}

/// Whether the weapon list shows it: the sword and the cursed blade from
/// the start, the blade there to taunt the player; any other once it is won
/// or rumored (Sam, 2026-10-07).
pub fn shown(w: &Weapon, best: &BTreeMap<String, Best>) -> bool {
    !w.enemy_only && (w.id == DEFAULT || w.prize || unlocked(w, best) || rumored(w, best))
}

/// How far the cursed blade has gathered: the fights won and the fights
/// there are, the secret one left out. It is found when the village deity
/// falls, which needs nearly each other fight won first.
pub fn prize_progress(best: &BTreeMap<String, Best>) -> (u32, u32) {
    let road = crate::road::road();
    let shown: Vec<_> = road.iter().filter(|s| !s.secret).collect();
    let won = shown.iter().filter(|s| best.contains_key(&s.id)).count() as u32;
    (won, shown.len() as u32)
}

/// Whether a weapon's card may say how it is won. Nothing tells the player
/// where the cursed blade is until they find it (Sam, 2026-10-07: "that is
/// the point of your rampage to some extent").
pub fn unlock_shown(w: &Weapon, best: &BTreeMap<String, Best>) -> bool {
    !w.prize && rumored(w, best) || unlocked(w, best)
}

/// Whether a stop's card may say it carries `weapon`. The village deity
/// wields the cursed blade, and its card is seen from the start, so until
/// the blade is found the card leaves it unsaid (Sam, 2026-10-07).
pub fn carried_shown(weapon: &str, best: &BTreeMap<String, Best>) -> bool {
    !self::weapon(weapon).is_some_and(|w| w.prize) || prize_found(best)
}

/// The weapon a player with these results carries when they ask for `id`:
/// that one if they may, the sword if not.
pub fn usable(id: &str, best: &BTreeMap<String, Best>) -> String {
    match weapon(id) {
        Some(w) if unlocked(&w, best) => w.id,
        _ => DEFAULT.to_string(),
    }
}

/// The sword reshaped into this weapon. The grips stay where they are on
/// the hand's side of the sword: their fractions are rescaled so the hands
/// do not move. Mass is shared among the weapon's points.
pub fn reshape(base: &SwordDef, w: &Weapon) -> SwordDef {
    let mut s = base.clone();
    s.returns = w.returns;
    s.cursed = w.prize;
    s.hilt = Fx((base.hilt.0 as i64 * w.hilt_pct / 100) as i32);
    let axis = base.tip - base.butt;
    let now = axis.len().trunc() as i64;
    let len = (now * w.length_pct / 100).max(1);
    s.tip = base.butt + axis.scale(len, now);
    for g in &mut s.grips {
        g.at = g.at.scale(now, len);
    }
    let along = s.tip - s.butt;
    // Across the blade: the line from butt to tip turned a quarter turn.
    let across = V2::new(Fx(-along.y.0), along.x);
    let point = |a: i64, c: i64| s.butt + along.scale(a, 100) + across.scale(c, 100);
    let hilt = s.hilt;
    let hilt_frac = |from: V2, to: V2| Fx::ratio(hilt.0 as i64, (to - from).len().0.max(1) as i64);
    match &w.shape {
        Shape::Straight {} => {}
        Shape::Curve(bends) => {
            s.extra = bends.iter().map(|&[a, c]| point(a, c)).collect();
            // butt → each bend → tip; the hilt is on the first stretch.
            let mut chain: Vec<u8> = vec![0];
            chain.extend((0..bends.len() as u8).map(|k| 2 + k));
            chain.push(1);
            let first = s.extra[0];
            s.edges = chain
                .windows(2)
                .enumerate()
                .map(|(k, p)| BladeEdge { a: p[0], b: p[1], from: if k == 0 { hilt_frac(s.butt, first) } else { Fx(0) } })
                .collect();
        }
        Shape::Prongs { fork_pct, prongs } => {
            let fork = point(*fork_pct, 0);
            s.extra = std::iter::once(fork).chain(prongs.iter().map(|&[a, c]| point(a, c))).collect();
            let mut edges = vec![BladeEdge { a: 0, b: 1, from: hilt_frac(s.butt, s.tip) }];
            edges.extend((0..prongs.len() as u8).map(|k| BladeEdge { a: 2, b: 3 + k, from: Fx(0) }));
            s.edges = edges;
        }
        Shape::Ring {} => {
            // Round the top half from the butt to the tip, then back along
            // the bottom: 45 degrees apart on a circle of the length across.
            let rim = [[15, 35], [50, 50], [85, 35], [85, -35], [50, -50], [15, -35]];
            s.extra = rim.iter().map(|&[a, c]| point(a, c)).collect();
            let chain: [u8; 9] = [0, 2, 3, 4, 1, 5, 6, 7, 0];
            let first = s.extra[0];
            s.edges = chain
                .windows(2)
                .enumerate()
                .map(|(k, p)| BladeEdge { a: p[0], b: p[1], from: if k == 0 { hilt_frac(s.butt, first) } else { Fx(0) } })
                .collect();
        }
        Shape::Twin { back_pct } => {
            // The butt goes back by the second blade's length; the old butt,
            // where the handle starts, stays as point 2. The hands keep their
            // distance from it, so their fractions along the longer weapon
            // change.
            let old = s.butt;
            let front = (s.tip - old).len();
            let back = (s.tip - old).scale(*back_pct, 100);
            s.butt = old - back;
            let whole = (s.tip - s.butt).len().0.max(1) as i64;
            for g in &mut s.grips {
                let d = (g.at * front).0 as i64 + back.len().0 as i64;
                g.at = Fx::ratio(d, whole);
            }
            s.extra = vec![old];
            s.edges = vec![
                // Behind the hands, cutting from half a handle back.
                BladeEdge { a: 2, b: 0, from: Fx::ratio(hilt.0 as i64 / 2, back.len().0.max(1) as i64) },
                BladeEdge { a: 2, b: 1, from: hilt_frac(old, s.tip) },
            ];
        }
    }
    let n = 2 + s.extra.len() as i64;
    s.mass = ((base.mass as i64 * 2 * w.mass_pct / 100) / n).max(1) as i32;
    s
}
