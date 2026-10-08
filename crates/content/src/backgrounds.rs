//! `data/backgrounds.json`: the scenery behind a fight (Sam, 2026-10-08:
//! "some backgrounds during the fights that are pretty lightweight, built
//! the same way as the costumes"). Only the page reads it; the simulation
//! never does, so a background cannot change a fight.

pub const BACKGROUNDS_JSON: &str = include_str!("../../../data/backgrounds.json");

/// The file as the page reads it, checked to parse.
pub fn page_json() -> serde_json::Value {
    serde_json::from_str(BACKGROUNDS_JSON).expect("data/backgrounds.json is valid")
}
