//! Replay v1: the whole of a match, as the inputs that made it (D15).
//!
//! The simulation is deterministic, so the setup and the inputs are the
//! match. The file also carries the end it reached (`ticks`, `checksum`),
//! so playback can be checked against it, and a checksum every
//! `CHECKPOINT_EVERY` ticks, so a replay that drifts names the first second
//! that differs (`A3 Locate the first divergence`). PLAN.md §7 item 22.
//!
//! Encoded with `postcard`, so it lives here with `sim`'s two dependencies
//! and the shim decides nothing.

use crate::body::Setup;
use crate::input::Input;
use crate::world::World;
use crate::SIM_VERSION;
use serde::{Deserialize, Serialize};

/// A lowercase identifier, not a player-read string (PLAN.md §8 Q12).
pub const FORMAT: &str = "vagrancy.replay";
/// 2: inputs are 16 bits wide, for the jump and the dodge.
pub const VERSION: u32 = 2;
pub const CHECKPOINT_EVERY: u32 = 60;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Replay {
    pub format: String,
    pub version: u32,
    pub sim_version: u32,
    pub setup: Setup,
    pub inputs: Vec<[u16; 2]>,
    pub ticks: u32,
    pub checksum: u64,
    pub checkpoints: Vec<u64>,
}

/// The first three fields, read before anything else so that a file from
/// another simulation version is refused by name, not as damage.
#[derive(Deserialize)]
struct Envelope {
    format: String,
    version: u32,
    sim_version: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReplayError {
    /// Not a replay file at all. `replay.error.format`.
    Format,
    /// Another simulation version. `replay.error.sim`.
    Sim { theirs: u32, ours: u32 },
    /// Incomplete, damaged, or internally inconsistent. `replay.error.damaged`.
    Damaged,
}

/// A match being played and written down at the same time.
#[derive(Clone, Debug)]
pub struct Recording {
    pub world: World,
    pub inputs: Vec<[u16; 2]>,
    pub checkpoints: Vec<u64>,
}

impl Recording {
    pub fn new(setup: Setup) -> Recording {
        Recording { world: World::new(setup), inputs: Vec::new(), checkpoints: Vec::new() }
    }

    pub fn step(&mut self, inputs: [Input; 2]) {
        self.world.step(inputs);
        self.inputs.push([inputs[0].0, inputs[1].0]);
        if self.world.tick % CHECKPOINT_EVERY == 0 {
            self.checkpoints.push(self.world.checksum());
        }
    }

    pub fn replay(&self) -> Replay {
        Replay {
            format: FORMAT.into(),
            version: VERSION,
            sim_version: SIM_VERSION,
            setup: self.world.setup.clone(),
            inputs: self.inputs.clone(),
            ticks: self.world.tick,
            checksum: self.world.checksum(),
            checkpoints: self.checkpoints.clone(),
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        postcard::to_allocvec(&self.replay()).expect("a replay always encodes")
    }
}

/// Read a replay file, refusing rather than half-loading.
pub fn load(bytes: &[u8]) -> Result<Replay, ReplayError> {
    let env: Envelope = postcard::from_bytes::<Envelope>(bytes).map_err(|_| ReplayError::Format)?;
    if env.format != FORMAT {
        return Err(ReplayError::Format);
    }
    if env.sim_version != SIM_VERSION {
        return Err(ReplayError::Sim { theirs: env.sim_version, ours: SIM_VERSION });
    }
    if env.version != VERSION {
        return Err(ReplayError::Damaged);
    }
    let (r, rest): (Replay, &[u8]) = postcard::take_from_bytes(bytes).map_err(|_| ReplayError::Damaged)?;
    let consistent = rest.is_empty()
        && r.ticks as usize == r.inputs.len()
        && r.checkpoints.len() == (r.ticks / CHECKPOINT_EVERY) as usize
        && r.inputs.iter().all(|p| p.iter().all(|b| b & Input::SPARE == 0));
    if !consistent {
        return Err(ReplayError::Damaged);
    }
    Ok(r)
}

/// Where a played-back replay first parted from its recording.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Divergence {
    /// The first checkpoint tick whose checksum differed, or `ticks` if only
    /// the final checksum did.
    pub tick: u32,
}

/// Play a replay to its end and check it against what it recorded.
pub fn verify(r: &Replay) -> Result<World, Divergence> {
    let mut w = World::new(r.setup.clone());
    let mut next = 0;
    for pair in &r.inputs {
        w.step([Input(pair[0]), Input(pair[1])]);
        if w.tick % CHECKPOINT_EVERY == 0 {
            if r.checkpoints.get(next) != Some(&w.checksum()) {
                return Err(Divergence { tick: w.tick });
            }
            next += 1;
        }
    }
    if w.checksum() != r.checksum {
        return Err(Divergence { tick: w.tick });
    }
    Ok(w)
}

/// A replay being watched, one tick at a time.
#[derive(Clone, Debug)]
pub struct Playback {
    pub replay: Replay,
    pub world: World,
}

impl Playback {
    pub fn new(replay: Replay) -> Playback {
        let world = World::new(replay.setup.clone());
        Playback { replay, world }
    }
    /// Step once; false when the replay has ended.
    pub fn step(&mut self) -> bool {
        match self.replay.inputs.get(self.world.tick as usize) {
            Some(p) => {
                self.world.step([Input(p[0]), Input(p[1])]);
                true
            }
            None => false,
        }
    }
    pub fn done(&self) -> bool {
        self.world.tick as usize >= self.replay.inputs.len()
    }
}

/// A fixed input script: both seats, every arm key, both directions of
/// travel, and quiet spells. The golden replay and the gate's native-against-
/// wasm comparison both use it, so it lives here and not in two places.
pub fn script(t: u32) -> [Input; 2] {
    let phase = (t / 45) % 8;
    let a = [
        Input::SHOULDER_UP,
        Input::SHOULDER_UP | Input::STEP_RIGHT,
        Input::ELBOW_OUT,
        0,
        Input::SHOULDER_DOWN | Input::ELBOW_IN,
        Input::STEP_LEFT,
        Input::SHOULDER_UP | Input::ELBOW_OUT,
        0,
    ][phase as usize];
    let b = [0, Input::SHOULDER_DOWN, Input::STEP_LEFT, Input::ELBOW_IN, Input::SHOULDER_UP, 0, Input::ELBOW_OUT, Input::STEP_RIGHT]
        [((t / 37) % 8) as usize];
    [Input(a), Input(b)]
}

/// The checksum after `ticks` of `script` in a match on seed 2026, as hex.
/// The shim exposes the same function, so the browser and the native build
/// run exactly the same code from exactly the same start.
pub fn script_checksum_of(setup: Setup, ticks: u32) -> String {
    let mut w = World::new(setup);
    for t in 0..ticks {
        w.step(script(t));
    }
    alloc_hex(w.checksum())
}

fn alloc_hex(v: u64) -> String {
    let digits = b"0123456789abcdef";
    (0..16).rev().map(|i| digits[((v >> (i * 4)) & 0xf) as usize] as char).collect()
}


/// The bytes of a replay file.
pub fn encode(r: &Replay) -> Vec<u8> {
    postcard::to_allocvec(r).expect("a replay always encodes")
}
