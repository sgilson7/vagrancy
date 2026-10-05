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
    /// A second opponent, at the player's back (Sam, 2026-10-05: "an enemy
    /// on each side of you").
    #[serde(default)]
    pub companion: Option<Companion>,
    /// The ground the fight is on, from data/maps.json; flat if none.
    #[serde(default)]
    pub map: Option<String>,
}

/// The opponent at the player's back on a flanked stop: another stop's
/// pilot, by that stop's id, carrying its own weapon.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Companion {
    pub pilot: String,
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
    /// Won it carrying this weapon (Sam: "weapon challenges like beat the
    /// courier with the trident").
    With { stop: String, weapon: String },
    /// Won it, in a match where the player ended a round with a cut across
    /// an opponent's head.
    Headshot(String),
    /// Won it, in a match where the player won a round without being cut.
    Untouched(String),
}

impl Req {
    pub fn stop(&self) -> &str {
        match self {
            Req::Beat(s) | Req::Flawless(s) | Req::Quick { stop: s, .. } | Req::With { stop: s, .. } | Req::Headshot(s) | Req::Untouched(s) => s,
        }
    }

    pub fn met(&self, best: &BTreeMap<String, Best>) -> bool {
        let Some(b) = best.get(self.stop()) else { return false };
        match self {
            Req::Beat(_) => true,
            Req::Flawless(_) => b.losses == 0,
            Req::Quick { seconds, .. } => (b.ticks as u64) <= *seconds as u64 * sim::balance::TICKS_PER_SECOND as u64,
            Req::With { weapon, .. } => b.with.iter().any(|w| w == weapon),
            Req::Headshot(_) => b.headshot,
            Req::Untouched(_) => b.untouched,
        }
    }

    /// The copy key that states it and its values, without the opponent's
    /// name, which the page reads from the copy by `stop`.
    pub fn sentence(&self) -> (&'static str, BTreeMap<String, String>) {
        match self {
            Req::Beat(_) => ("road.req.beat", BTreeMap::new()),
            Req::Flawless(_) => ("road.req.flawless", BTreeMap::new()),
            Req::Quick { seconds, .. } => ("road.req.quick", BTreeMap::from([("seconds".to_string(), seconds.to_string())])),
            // The page names the weapon from `weapons.<id>.name`.
            Req::With { weapon, .. } => ("road.req.with", BTreeMap::from([("weapon".to_string(), weapon.clone())])),
            Req::Headshot(_) => ("road.req.headshot", BTreeMap::new()),
            Req::Untouched(_) => ("road.req.untouched", BTreeMap::new()),
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
/// keeps its wins with `u32::MAX` in both and no weapons, so a win it holds
/// opens what a win opens and nothing that asks for more.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Best {
    /// Rounds lost in that match.
    pub losses: u32,
    /// How long the match took, in ticks.
    pub ticks: u32,
    /// Every weapon a match here has been won with, sorted. A save from
    /// before weapons were recorded has none.
    #[serde(default)]
    pub with: Vec<String>,
    /// A won match here had a round ended by the player's cut across an
    /// opponent's head. A save from before version 6 has none.
    #[serde(default)]
    pub headshot: bool,
    /// A won match here had a round the player won without being cut.
    #[serde(default)]
    pub untouched: bool,
}

impl Best {
    pub const UNKNOWN: Best = Best { losses: u32::MAX, ticks: u32::MAX, with: Vec::new(), headshot: false, untouched: false };

    /// One won match, carrying `weapon`, with no feats.
    pub fn won(losses: u32, ticks: u32, weapon: &str) -> Best {
        Best { losses, ticks, with: vec![weapon.to_string()], headshot: false, untouched: false }
    }

    /// The same result with the feats a match showed.
    pub fn with_feats(self, f: Feats) -> Best {
        Best { headshot: f.headshot, untouched: f.untouched, ..self }
    }

    /// The better of two results, each part on its own: the fewest losses
    /// and the shortest match need not be the same match, and every weapon
    /// either was won with counts.
    pub fn merge(self, other: Best) -> Best {
        let mut with = self.with;
        with.extend(other.with);
        with.sort();
        with.dedup();
        Best {
            losses: self.losses.min(other.losses),
            ticks: self.ticks.min(other.ticks),
            with,
            headshot: self.headshot || other.headshot,
            untouched: self.untouched || other.untouched,
        }
    }
}

/// What the player did in a match beyond winning it, watched tick by tick
/// from the world's events. The road's newer requirements ask for these.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Feats {
    pub headshot: bool,
    pub untouched: bool,
    /// The player has been cut in the round under way.
    cut: bool,
}

impl Feats {
    /// Read one tick's events. Call it after every step.
    pub fn observe(&mut self, w: &sim::World) {
        for e in &w.events {
            match *e {
                sim::fight::Event::Cut { seat: 0, spilled: true, .. } => self.cut = true,
                sim::fight::Event::RoundEnd { result } => {
                    if result.loser == Some(1) {
                        self.headshot |= result.by == 0 && crate::messages::headshot(w, &result);
                        self.untouched |= !self.cut;
                    }
                    self.cut = false;
                }
                _ => {}
            }
        }
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
    let merged = match best.get(id) {
        Some(b) => b.clone().merge(this),
        None => this,
    };
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

/// The pilots of a stop's opponents, by seat from seat 1: the stop's own,
/// then its companion's if it has one.
pub fn crew(id: &str) -> Vec<Spec> {
    let mut v = vec![pilot(id)];
    if let Some(c) = stop(id).and_then(|s| s.companion) {
        v.push(pilot(&c.pilot));
    }
    v
}

/// Every seat's pilot at a stop, `player` in seat 0, built and ready for
/// `pilot::duel`.
pub fn lineup(player: &Spec, id: &str) -> Vec<Box<dyn pilot::Pilot>> {
    std::iter::once(player.clone()).chain(crew(id)).map(|s| pilot::build(&s)).collect()
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
    let mut stops: Vec<String> = road().iter().map(|s| format!("{}:{:?}:{:?}:{:?}:{:?}", s.id, s.condition, s.weapon, s.companion, s.map)).collect();
    stops.sort();
    let data = format!("{}{}{}{}", PILOTS_JSON, stops.join(","), crate::maps::MAPS_JSON, sim::SIM_VERSION);
    format!("{:016x}", sim::world::fnv1a(data.as_bytes()))
}
