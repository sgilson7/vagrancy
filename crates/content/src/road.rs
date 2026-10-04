//! The road: its order, its opponents' pilots, and the numbers their
//! introductions state (D16; PLANNING-BRIEF M5).

use pilot::Spec;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub const PILOTS_JSON: &str = include_str!("../../../data/pilots.json");
pub const ROAD_JSON: &str = include_str!("../../../data/road.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Road {
    #[allow(dead_code)]
    _about: String,
    stops: Vec<Stop>,
}

/// A fight on the road and what opens it (Sam: a tree whose levels are the
/// number of requirements).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stop {
    pub id: String,
    pub requires: Vec<Req>,
    #[serde(default)]
    pub condition: Option<Condition>,
    /// What the opponent carries, from data/weapons.json; the sword if none.
    #[serde(default)]
    pub weapon: Option<String>,
}

impl Stop {
    /// The row of the tree it sits in: how many requirements it has.
    pub fn level(&self) -> usize {
        self.requires.len()
    }
}

/// One thing a player must have done before a fight opens.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Req {
    /// Won the match at that stop.
    Beat(String),
    /// Won it without losing a round.
    Flawless(String),
    /// Won it with the match lasting no longer than this.
    Quick { stop: String, seconds: u32 },
}

impl Req {
    pub fn stop(&self) -> &str {
        match self {
            Req::Beat(s) | Req::Flawless(s) | Req::Quick { stop: s, .. } => s,
        }
    }

    pub fn met(&self, best: &BTreeMap<String, Best>) -> bool {
        let Some(b) = best.get(self.stop()) else { return false };
        match self {
            Req::Beat(_) => true,
            Req::Flawless(_) => b.losses == 0,
            Req::Quick { seconds, .. } => (b.ticks as u64) <= *seconds as u64 * sim::balance::TICKS_PER_SECOND as u64,
        }
    }

    /// The copy key that states it and its values, without the opponent's
    /// name, which the page reads from the copy by `stop`.
    pub fn sentence(&self) -> (&'static str, BTreeMap<String, String>) {
        match self {
            Req::Beat(_) => ("road.req.beat", BTreeMap::new()),
            Req::Flawless(_) => ("road.req.flawless", BTreeMap::new()),
            Req::Quick { seconds, .. } => ("road.req.quick", BTreeMap::from([("seconds".to_string(), seconds.to_string())])),
        }
    }
}

/// A special rule for one fight, after Soul Calibur II's Weapon Master map.
/// There was a third, one round decides the match, and the ladder showed it
/// decided by the opponent's opening move: the yardstick won 0 of 200 at
/// the cooper with it (SECOND-ORDER-M5 row 35).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    /// The opponent starts each round with twice the ink. (Halving the
    /// player's instead drew a full meter of a smaller tank, which reads as
    /// no change.)
    DeepInk,
    /// Gravity is half.
    Light,
}

impl Condition {
    pub fn copy_key(self) -> &'static str {
        match self {
            Condition::DeepInk => "road.condition.deep_ink",
            Condition::Light => "road.condition.light",
        }
    }
}

/// The best won match at a stop. A save written before these were recorded
/// keeps its wins with `u32::MAX` in both, so a win it holds opens what a
/// win opens and nothing that asks for more.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Best {
    /// Rounds lost in that match.
    pub losses: u32,
    /// How long the match took, in ticks.
    pub ticks: u32,
}

impl Best {
    pub const UNKNOWN: Best = Best { losses: u32::MAX, ticks: u32::MAX };

    /// The better of two results, each part on its own: the fewest losses
    /// and the shortest match need not be the same match.
    pub fn merge(self, other: Best) -> Best {
        Best { losses: self.losses.min(other.losses), ticks: self.ticks.min(other.ticks) }
    }
}

/// Every stop, in the order data/road.json lists them.
pub fn road() -> Vec<Stop> {
    serde_json::from_str::<Road>(ROAD_JSON).expect("data/road.json is valid").stops
}

/// The stops' ids, in that order.
pub fn stops() -> Vec<String> {
    road().into_iter().map(|s| s.id).collect()
}

pub fn stop(id: &str) -> Option<Stop> {
    road().into_iter().find(|s| s.id == id)
}

/// Keep a won match's result at `id`, and name the fights it opened.
pub fn record(best: &mut BTreeMap<String, Best>, id: &str, this: Best) -> Vec<String> {
    let road = road();
    let before: Vec<bool> = road.iter().map(|st| open(st, best)).collect();
    let merged = best.get(id).map(|b| b.merge(this)).unwrap_or(this);
    best.insert(id.to_string(), merged);
    road.into_iter().zip(before).filter(|(st, was)| !was && open(st, best)).map(|(st, _)| st.id).collect()
}

/// Whether a fight is open, given the player's best results.
pub fn open(stop: &Stop, best: &BTreeMap<String, Best>) -> bool {
    stop.requires.iter().all(|r| r.met(best))
}

/// Every pilot, the yardstick included, by id.
pub fn pilots() -> BTreeMap<String, Spec> {
    let v: BTreeMap<String, Value> = serde_json::from_str(PILOTS_JSON).expect("data/pilots.json is valid");
    v.into_iter()
        .filter(|(k, _)| !k.starts_with('_'))
        .map(|(k, v)| {
            let spec: Spec = serde_json::from_value(v).unwrap_or_else(|e| panic!("pilot {k}: {e}"));
            (k, spec)
        })
        .collect()
}

pub fn pilot(id: &str) -> Spec {
    pilots().remove(id).unwrap_or_else(|| panic!("no pilot named {id}"))
}

/// The default sword's length in whole cm, from `data/body.json`.
pub fn default_sword_len() -> i32 {
    let bodies = crate::body::bodies();
    let s = bodies[0].sword.as_ref().expect("the fighter has a sword");
    (s.tip - s.butt).len().trunc()
}

/// The values for an opponent's introduction placeholders: from its pilot,
/// and `{reach_pct}` from the weapon it carries, how much longer than the
/// sword, rounded.
pub fn intro_numbers(id: &str) -> BTreeMap<String, String> {
    let mut n: BTreeMap<String, String> = pilot::numbers(&pilot(id)).into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    if let Some(w) = stop(id).and_then(|s| s.weapon).and_then(|w| crate::weapons::weapon(&w)) {
        let base = default_sword_len();
        let len = base * w.length_pct as i32 / 100;
        if len != base {
            n.insert("reach_pct".into(), (((len - base) * 100 + base / 2) / base).to_string());
        }
    }
    n
}

/// What a ladder was measured on: the pilots, the set of stops with their
/// conditions (not their order or requirements, which change no match) and
/// the simulation. A ladder whose
/// fingerprint differs is stale.
pub fn ladder_fingerprint() -> String {
    // A stop's condition changes its matches; its requirements do not.
    let mut stops: Vec<String> = road().iter().map(|s| format!("{}:{:?}:{:?}", s.id, s.condition, s.weapon)).collect();
    stops.sort();
    let data = format!("{}{}{}", PILOTS_JSON, stops.join(","), sim::SIM_VERSION);
    format!("{:016x}", sim::world::fnv1a(data.as_bytes()))
}
