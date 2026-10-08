//! `data/duels.json`: duels that are not on the road, opened by progress
//! elsewhere (Sam, 2026-10-08: Paul against Feyd-Rautha, story mode's prize
//! for its first chapter, the player as Paul).

use serde::Deserialize;
use std::collections::BTreeMap;

pub const DUELS_JSON: &str = include_str!("../../../data/duels.json");

#[derive(Deserialize)]
struct File {
    duels: BTreeMap<String, Duel>,
}

#[derive(Clone, Deserialize)]
pub struct Duel {
    pub unlock: Unlock,
    /// The opponent's pilot, by road id.
    pub pilot: String,
    /// The player's weapon and the opponent's, written out whole.
    pub weapons: [serde_json::Value; 2],
    pub shield: bool,
    pub rounds: u32,
    /// How far apart they start, cm.
    pub gap: i32,
    /// What the page draws (web/draw.js `draw.guests`).
    pub page: serde_json::Value,
}

#[derive(Clone, Deserialize)]
pub struct Unlock {
    /// Story mode chapters finished.
    pub story: u32,
}

pub fn duels() -> BTreeMap<String, Duel> {
    serde_json::from_str::<File>(DUELS_JSON).expect("data/duels.json is valid").duels
}

pub fn duel(id: &str) -> Option<Duel> {
    duels().remove(id)
}

/// Whether a save has opened duel `d`.
pub fn open(d: &Duel, save: &crate::save::SaveState) -> bool {
    save.story >= d.unlock.story
}

/// The duel's setup: the player in seat 0.
pub fn setup(seed: u64, tuning: u8, d: &Duel) -> sim::Setup {
    let [a, b] = [d.weapons[0].to_string(), d.weapons[1].to_string()];
    crate::setup::duel(seed, tuning, [&a, &b], d.shield, d.rounds, d.gap)
}
