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
    /// What to say about the phase: `null` mid-round, else the round's
    /// sentence (and the match's), as copy keys and values chosen by
    /// `content`. `opponent` is a road opponent's id, or empty for versus.
    pub fn phase_text(&self, opponent: &str) -> String {
        let who = if opponent.is_empty() {
            content::messages::Audience::Versus
        } else {
            content::messages::Audience::Road { opponent }
        };
        content::messages::phase_text(self.world(), who).to_string()
    }

    /// The checksum the replay recorded, for a watcher to compare at the end.
    pub fn recorded_checksum(&self) -> String {
        self.play.as_ref().map(|p| format!("{:016x}", p.replay.checksum)).unwrap_or_default()
    }
}

/// One end of an online match (D14). The page carries the bytes and the
/// time; `net::Session` decides everything.
#[wasm_bindgen]
pub struct Online {
    s: net::Session,
}

#[wasm_bindgen]
impl Online {
    pub fn host(seed: u32, tuning: u8, build: &str) -> Online {
        Online { s: net::Session::host(content::setup::versus(seed as u64, tuning), build) }
    }
    pub fn join(build: &str) -> Online {
        Online { s: net::Session::join(build) }
    }
    pub fn receive(&mut self, now: f64, bytes: &[u8]) {
        self.s.receive(now as u64, bytes);
    }
    pub fn poll(&mut self, now: f64) {
        self.s.poll(now as u64);
    }
    pub fn start(&mut self, now: f64) {
        self.s.start(now as u64);
    }
    /// One tick of the page's clock with this side's input. True if the
    /// world stepped.
    pub fn step(&mut self, now: f64, mine: u8) -> bool {
        self.s.tick(now as u64, Input(mine)).is_some()
    }
    /// Everything to send, as [u32 length, little-endian][bytes]…
    pub fn outbox(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        for m in self.s.take_outbox() {
            out.extend_from_slice(&(m.len() as u32).to_le_bytes());
            out.extend_from_slice(&m);
        }
        out
    }
    pub fn refusal_full() -> Vec<u8> {
        net::Session::refusal_full()
    }
    pub fn seat(&self) -> u32 {
        self.s.seat() as u32
    }
    /// `{ "kind": …, "delay_ms": …, "tick": … }` for the lobby and the HUD.
    pub fn status(&self) -> String {
        use net::session::Status;
        let (kind, tick) = match &self.s.status {
            Status::Waiting => ("waiting", 0),
            Status::Measuring => ("measuring", 0),
            Status::Connected { .. } => ("connected", 0),
            Status::Playing => ("playing", 0),
            Status::WaitingOn => ("waiting_on", 0),
            Status::Desync { tick } => ("desync", *tick),
            Status::Left => ("left", 0),
            Status::Refused(net::wire::Bye::Full) => ("full", 0),
            Status::Refused(_) => ("build", 0),
        };
        json!({ "kind": kind, "delay_ms": net::session::delay_ms(self.s.delay()), "tick": tick }).to_string()
    }
    pub fn playing(&self) -> bool {
        self.s.world().is_some()
    }
    pub fn frame(&self) -> String {
        self.s.world().map(|w| serde_json::to_string(&frame::frame(w)).unwrap()).unwrap_or_default()
    }
    pub fn phase_text(&self) -> String {
        self.s.world().map(|w| content::messages::phase_text(w, content::messages::Audience::Versus).to_string()).unwrap_or_default()
    }
    pub fn checksum(&self) -> String {
        self.s.world().map(|w| format!("{:016x}", w.checksum())).unwrap_or_default()
    }
    pub fn tick(&self) -> u32 {
        self.s.world().map(|w| w.tick).unwrap_or(0)
    }
    pub fn replay_bytes(&self) -> Vec<u8> {
        self.s.recording().map(|r| r.bytes()).unwrap_or_default()
    }
}
