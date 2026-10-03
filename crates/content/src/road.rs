//! The road: its order, its opponents' pilots, and the numbers their
//! introductions state (D16; PLANNING-BRIEF M5).

use pilot::Spec;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub const PILOTS_JSON: &str = include_str!("../../../data/pilots.json");
pub const ROAD_JSON: &str = include_str!("../../../data/road.json");

#[derive(Deserialize)]
struct Road {
    stops: Vec<String>,
}

/// The stops, in order.
pub fn stops() -> Vec<String> {
    serde_json::from_str::<Road>(ROAD_JSON).expect("data/road.json is valid").stops
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

/// The values for an opponent's introduction placeholders, from its pilot.
pub fn intro_numbers(id: &str) -> BTreeMap<String, String> {
    pilot::numbers(&pilot(id), default_sword_len()).into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// What a ladder was measured on: the pilots, the set of stops (not their
/// order, which changes no match) and the simulation. A ladder whose
/// fingerprint differs is stale.
pub fn ladder_fingerprint() -> String {
    let mut stops = stops();
    stops.sort();
    let data = format!("{}{}{}", PILOTS_JSON, stops.join(","), sim::SIM_VERSION);
    format!("{:016x}", sim::world::fnv1a(data.as_bytes()))
}
