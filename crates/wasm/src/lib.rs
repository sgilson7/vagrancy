//! The shim. It moves bytes across the boundary and decides nothing: an `if`
//! here is a rule that belongs in `sim`, where the test suite can reach it.

use serde_json::json;
use sim::replay::{self, Playback, Recording};
use sim::{balance, frame, fx, Input};
use wasm_bindgen::prelude::*;

/// The copy file, shipped inside the module so the strings and the build that
/// uses them cannot drift apart.
#[wasm_bindgen]
pub fn copy_json() -> String {
    content::copy::COPY_JSON.to_string()
}

#[wasm_bindgen]
pub fn palette_json() -> String {
    include_str!("../../../data/palette.json").to_string()
}

#[wasm_bindgen]
pub fn controls_json() -> String {
    include_str!("../../../data/controls.json").to_string()
}

/// Every constant the page needs, from where it is decided.
#[wasm_bindgen]
pub fn numbers() -> String {
    json!({
        "ticks_per_second": balance::TICKS_PER_SECOND,
        "rounds_to_win": balance::ROUNDS_TO_WIN,
        "frac_bits": fx::FRAC_BITS,
        "arena_half": balance::ARENA_HALF.0,
        "sim_version": sim::SIM_VERSION,
        "actions": Input::ACTIONS.iter().map(|(n, b)| json!([n, b])).collect::<Vec<_>>(),
        "ready_bit": Input::READY,
        "tunings": balance::TUNINGS.len(),
        "default_tuning": balance::DEFAULT_TUNING,
    })
    .to_string()
}

/// The fixed script's checksum after `ticks`, computed in the browser. The
/// gate compares it with `lab script-checksum`, the same code run natively.
#[wasm_bindgen]
pub fn script_checksum(ticks: u32) -> String {
    replay::script_checksum_of(content::setup::versus(2026, balance::DEFAULT_TUNING), ticks)
}

/// A match being played and recorded, or a replay being watched.
#[wasm_bindgen]
pub struct Game {
    rec: Option<Recording>,
    play: Option<Playback>,
}

#[wasm_bindgen]
impl Game {
    pub fn alone(seed: u32, tuning: u8) -> Game {
        Game { rec: Some(Recording::new(content::setup::alone(seed as u64, tuning))), play: None }
    }
    pub fn practice(seed: u32, tuning: u8) -> Game {
        Game { rec: Some(Recording::new(content::setup::practice(seed as u64, tuning))), play: None }
    }
    pub fn versus(seed: u32, tuning: u8) -> Game {
        Game { rec: Some(Recording::new(content::setup::versus(seed as u64, tuning))), play: None }
    }
    /// A replay file, or the copy key and values of the sentence that refuses it.
    pub fn load_replay(bytes: &[u8]) -> Result<Game, String> {
        replay::load(bytes)
            .map(|r| Game { rec: None, play: Some(Playback::new(r)) })
            .map_err(|e| content::messages::replay_error(e).to_string())
    }
    pub fn step(&mut self, a: u8, b: u8) {
        match (&mut self.rec, &mut self.play) {
            (Some(r), _) => r.step([Input(a), Input(b)]),
            (_, Some(p)) => {
                p.step();
            }
            _ => {}
        }
    }
    fn world(&self) -> &sim::World {
        match (&self.rec, &self.play) {
            (Some(r), _) => &r.world,
            (_, Some(p)) => &p.world,
            _ => unreachable!("a Game is always one or the other"),
        }
    }
    pub fn frame(&self) -> String {
        serde_json::to_string(&frame::frame(self.world())).unwrap()
    }
    pub fn tick(&self) -> u32 {
        self.world().tick
    }
    pub fn checksum(&self) -> String {
        format!("{:016x}", self.world().checksum())
    }
    pub fn is_replay(&self) -> bool {
        self.play.is_some()
    }
    pub fn done(&self) -> bool {
        self.play.as_ref().is_some_and(|p| p.done())
    }
    pub fn replay_bytes(&self) -> Vec<u8> {
        match (&self.rec, &self.play) {
            (Some(r), _) => r.bytes(),
            (_, Some(p)) => replay::encode(&p.replay),
            _ => Vec::new(),
        }
    }
    /// The checksum the replay recorded, for a watcher to compare at the end.
    pub fn recorded_checksum(&self) -> String {
        self.play.as_ref().map(|p| format!("{:016x}", p.replay.checksum)).unwrap_or_default()
    }
}
