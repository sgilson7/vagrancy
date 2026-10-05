//! The grounds a fight can be on: the flat arena, or one with ledges to
//! stand on (data/maps.json).

use serde::Deserialize;
use sim::body::{Platform, Setup};
use sim::fx::Fx;

pub const MAPS_JSON: &str = include_str!("../../../data/maps.json");
/// The map with no ledges, for a fight that names none.
pub const FLAT: &str = "flat";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Maps {
    #[allow(dead_code)]
    _about: String,
    maps: Vec<Map>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Map {
    pub id: String,
    /// `[x0, x1, y]` in whole cm.
    pub platforms: Vec<[i32; 3]>,
}

/// Every map, in the order the file lists them.
pub fn maps() -> Vec<Map> {
    serde_json::from_str::<Maps>(MAPS_JSON).expect("data/maps.json is valid").maps
}

pub fn map(id: &str) -> Option<Map> {
    maps().into_iter().find(|m| m.id == id)
}

/// The setup on map `id`; a map this build does not know is the flat one.
pub fn on(mut s: Setup, id: &str) -> Setup {
    s.platforms = map(id).map(|m| m.platforms.iter().map(|&[x0, x1, y]| Platform { x0: Fx::int(x0), x1: Fx::int(x1), y: Fx::int(y) }).collect()).unwrap_or_default();
    s
}
