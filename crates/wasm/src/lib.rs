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

/// An opponent's behavior tree (`pilot::view::describe`): node ids, kinds,
/// icons, and labels as copy keys with values, for the page to draw.
#[wasm_bindgen]
pub fn tree_json(opponent: &str) -> String {
    serde_json::to_string(&pilot::view::describe(&content::road::pilot(opponent))).unwrap()
}

/// One frame of the floating trees (content::bubbles): the bubbles as JSON
/// in, moved, as JSON out, within a `w` by `h` canvas.
#[wasm_bindgen]
pub fn bubbles_step(json: &str, w: f64, h: f64) -> String {
    let mut bs: Vec<content::bubbles::Bubble> = serde_json::from_str(json).unwrap_or_default();
    content::bubbles::step(&mut bs, w, h);
    serde_json::to_string(&bs).unwrap()
}

/// The grounds a match can be on (data/maps.json): the pickers list them.
#[wasm_bindgen]
pub fn maps_json() -> String {
    content::maps::MAPS_JSON.to_string()
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
    pub fn practice(seed: u32, tuning: u8, weapon: &str, four: bool) -> Game {
        Game { rec: Some(Recording::new(content::setup::practice_with(seed as u64, tuning, weapon, four))), play: None }
    }
    pub fn versus(seed: u32, tuning: u8, left: &str, right: &str, map: &str) -> Game {
        Game { rec: Some(Recording::new(content::setup::versus_with(seed as u64, tuning, [left, right], map))), play: None }
    }
    /// A replay file, or the copy key and values of the sentence that refuses it.
    pub fn load_replay(bytes: &[u8]) -> Result<Game, String> {
        replay::load(bytes)
            .map(|r| Game { rec: None, play: Some(Playback::new(r)) })
            .map_err(|e| content::messages::replay_error(e).to_string())
    }
    pub fn step(&mut self, a: u16, b: u16) {
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
    /// Host: what the match is built from, so it can be rebuilt with the
    /// joiner's weapon once the joiner says what it is.
    seed: u32,
    tuning: u8,
    weapon: String,
    map: String,
    armed: bool,
    /// The arena's side of a stream challenge: a road opponent's pilot
    /// presses this side's keys (Sam, 2026-10-07: viewers "fight live on
    /// the twitch stream").
    pilot: Option<Box<dyn pilot::Pilot>>,
}

#[wasm_bindgen]
impl Online {
    /// The host chooses the map; the welcome carries it to the joiner
    /// inside the setup.
    pub fn host(seed: u32, tuning: u8, build: &str, weapon: &str, map: &str) -> Online {
        let setup = content::setup::versus_with(seed as u64, tuning, [weapon, content::weapons::DEFAULT], map);
        Online { s: net::Session::host(setup, build), seed, tuning, weapon: weapon.into(), map: map.into(), armed: false, pilot: None }
    }
    pub fn join(build: &str, weapon: &str) -> Online {
        Online { s: net::Session::join(build, weapon), seed: 0, tuning: 0, weapon: weapon.into(), map: String::new(), armed: true, pilot: None }
    }
    pub fn receive(&mut self, now: f64, bytes: &[u8]) {
        self.s.receive(now as u64, bytes);
        // The joiner's hello names its weapon; the match is rebuilt with it
        // before the welcome carries it to both.
        if !self.armed {
            if let Some(w) = self.s.joiner_weapon.clone() {
                let setup = content::setup::versus_with(self.seed as u64, self.tuning, [&self.weapon, &w], &self.map);
                self.armed = self.s.rearm(setup);
            }
        }
    }
    pub fn poll(&mut self, now: f64) {
        self.s.poll(now as u64);
    }
    pub fn start(&mut self, now: f64) {
        self.s.start(now as u64);
    }
    pub fn ready(&mut self) {
        self.s.ready();
    }
    /// Let a road opponent's pilot play this side.
    pub fn set_pilot(&mut self, id: &str) {
        self.pilot = Some(pilot::build(&content::road::pilot(id)));
    }
    /// One tick with this side's keys from its pilot, which reads the
    /// match as both sides have it. True if the world stepped.
    pub fn step_piloted(&mut self, now: f64) -> bool {
        let seat = self.s.seat() as usize;
        let mine = match (self.pilot.as_mut(), self.s.world()) {
            (Some(p), Some(w)) => p.input(w, seat),
            _ => Input::NONE,
        };
        self.s.tick(now as u64, mine).is_some()
    }
    /// The pilot's last tick, as `Lab::report` gives it for one seat.
    pub fn pilot_report(&self) -> String {
        let Some(p) = self.pilot.as_ref() else { return "null".into() };
        let t = p.trace();
        json!({ "seat": self.s.seat(), "active": t.active, "held": t.held, "failed": t.failed }).to_string()
    }
    /// One tick of the page's clock with this side's input. True if the
    /// world stepped.
    pub fn step(&mut self, now: f64, mine: u16) -> bool {
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
        json!({ "kind": kind, "delay_ms": net::session::delay_ms(self.s.delay()), "tick": tick,
                "me_ready": self.s.me_ready, "them_ready": self.s.them_ready }).to_string()
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

/// A stop on the road: the player in seat 0, a pilot in seat 1 (D16), and
/// on a flanked stop a second in seat 2.
#[wasm_bindgen]
pub struct Road {
    rec: Recording,
    pilots: Vec<Box<dyn pilot::Pilot>>,
    last: [Input; sim::body::SEATS],
    feats: content::road::Feats,
    opponent: String,
    /// What the player carries, as the save will record a win.
    weapon: String,
}

#[wasm_bindgen]
impl Road {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, tuning: u8, opponent: &str, weapon: &str, four: bool) -> Road {
        Road {
            rec: Recording::new(content::setup::road_with(seed as u64, tuning, opponent, weapon, four)),
            pilots: content::road::crew(opponent).iter().map(pilot::build).collect(),
            last: [Input::NONE; sim::body::SEATS],
            feats: content::road::Feats::default(),
            opponent: opponent.into(),
            weapon: weapon.into(),
        }
    }
    /// One tick: each pilot sees every seat's last input and the world,
    /// and answers with an input of its own.
    pub fn step(&mut self, mine: u16, _other: u16) {
        let mut i = [Input(mine), Input::NONE, Input::NONE];
        for (k, p) in self.pilots.iter_mut().enumerate() {
            p.observe(self.last);
            i[k + 1] = p.input(&self.rec.world, k + 1);
        }
        self.rec.step_all(i);
        self.feats.observe(&self.rec.world);
        self.last = i;
    }
    pub fn frame(&self) -> String {
        serde_json::to_string(&frame::frame(&self.rec.world)).unwrap()
    }
    /// What each opponent's tree ran on the last tick, by seat from seat 1:
    /// `[{ seat, active, held, failed }]`.
    pub fn traces(&self) -> String {
        let all: Vec<serde_json::Value> = self
            .pilots
            .iter()
            .enumerate()
            .map(|(k, p)| {
                let t = p.trace();
                json!({ "seat": k + 1, "active": t.active, "held": t.held, "failed": t.failed })
            })
            .collect();
        serde_json::to_string(&all).unwrap()
    }
    pub fn phase_text(&self, _opponent: &str) -> String {
        content::messages::phase_text(&self.rec.world, content::messages::Audience::Road { opponent: &self.opponent }).to_string()
    }
    pub fn checksum(&self) -> String {
        format!("{:016x}", self.rec.world.checksum())
    }
    pub fn tick(&self) -> u32 {
        self.rec.world.tick
    }
    pub fn is_replay(&self) -> bool {
        false
    }
    pub fn done(&self) -> bool {
        false
    }
    pub fn replay_bytes(&self) -> Vec<u8> {
        self.rec.bytes()
    }
    pub fn recorded_checksum(&self) -> String {
        String::new()
    }
    /// The match is over and the player won it.
    pub fn won(&self) -> bool {
        matches!(self.rec.world.phase, sim::fight::Phase::MatchOver { .. }) && self.rec.world.wins[0] > self.rec.world.wins[1]
    }
    /// After a win: the save with this result kept, the fights it opened,
    /// and, when it opened none, the next goal, as `{ "save": text,
    /// "opened": [id], "next": { open, stop, key, vars } | null }`.
    pub fn record(&self, save_text: &str) -> Result<String, String> {
        let mut s = content::save::decode(save_text).map_err(|e| e.message().to_string())?;
        if !self.won() {
            return Ok(json!({ "save": content::save::encode(&s), "opened": [], "next": null }).to_string());
        }
        let w = &self.rec.world;
        let hidden: Vec<String> = content::road::road().into_iter().filter(|st| !content::road::visible(st, &s.road.best)).map(|st| st.id).collect();
        let opened = content::road::record(&mut s.road.best, &self.opponent, content::road::Best::won(w.wins[1], w.tick, &self.weapon).with_feats(self.feats));
        // The secret fights this win brought onto the chart.
        let revealed: Vec<String> = content::road::road().into_iter().filter(|st| hidden.contains(&st.id) && content::road::visible(st, &s.road.best)).map(|st| st.id).collect();
        // When the win opened nothing, the goal core picks for next.
        let next = if opened.is_empty() {
            content::road::next_goal(&s.road.best).map(|(target, r)| {
                let (key, vars) = r.sentence();
                json!({ "open": target, "stop": r.stop(), "key": key, "vars": vars })
            })
        } else {
            None
        };
        Ok(json!({ "save": content::save::encode(&s), "opened": opened, "next": next, "revealed": revealed }).to_string())
    }
}

/// Watch mode (Sam, 2026-10-06: "an enemy vs enemy ai watching mode"): two
/// road opponents, each run by its own pilot, in
/// the bodies they fight in on the road. The page's keys are ignored.
#[wasm_bindgen]
pub struct Exhibition {
    rec: Recording,
    pilots: Vec<Box<dyn pilot::Pilot>>,
    last: [Input; sim::body::SEATS],
    ids: [String; 2],
}

#[wasm_bindgen]
impl Exhibition {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, tuning: u8, left: &str, right: &str, map: &str) -> Exhibition {
        Exhibition {
            rec: Recording::new(content::setup::exhibition(seed as u64, tuning, [left, right], map)),
            pilots: [left, right].iter().map(|id| pilot::build(&content::road::pilot(id))).collect(),
            last: [Input::NONE; sim::body::SEATS],
            ids: [left.into(), right.into()],
        }
    }
    /// One tick: both pilots see the world and every seat's last input.
    pub fn step(&mut self, _a: u16, _b: u16) {
        let mut i = [Input::NONE; sim::body::SEATS];
        for (k, p) in self.pilots.iter_mut().enumerate() {
            p.observe(self.last);
            i[k] = p.input(&self.rec.world, k);
        }
        self.rec.step_all(i);
        self.last = i;
    }
    pub fn frame(&self) -> String {
        serde_json::to_string(&frame::frame(&self.rec.world)).unwrap()
    }
    /// What each side's tree ran on the last tick: `[{ seat, active, held,
    /// failed }]`, seats 0 and 1.
    pub fn traces(&self) -> String {
        let all: Vec<serde_json::Value> = self
            .pilots
            .iter()
            .enumerate()
            .map(|(k, p)| {
                let t = p.trace();
                json!({ "seat": k, "active": t.active, "held": t.held, "failed": t.failed })
            })
            .collect();
        serde_json::to_string(&all).unwrap()
    }
    pub fn phase_text(&self, _opponent: &str) -> String {
        let ids = [self.ids[0].as_str(), self.ids[1].as_str()];
        content::messages::phase_text(&self.rec.world, content::messages::Audience::Exhibition { ids }).to_string()
    }
    pub fn checksum(&self) -> String {
        format!("{:016x}", self.rec.world.checksum())
    }
    pub fn tick(&self) -> u32 {
        self.rec.world.tick
    }
    pub fn is_replay(&self) -> bool {
        false
    }
    pub fn done(&self) -> bool {
        false
    }
    pub fn replay_bytes(&self) -> Vec<u8> {
        self.rec.bytes()
    }
    pub fn recorded_checksum(&self) -> String {
        String::new()
    }
}

/// The BT Lab (Sam, 2026-10-06: a classroom demo of "how the high level
/// behaviors and nodes get converted into controls execution"): any road
/// opponent, watched against another or fought with the keys, with what
/// each pilot pressed and why on every tick. Nothing is saved.
#[wasm_bindgen]
pub struct Lab {
    rec: Recording,
    /// A pilot per seat; seat 0 has none when a person plays it.
    pilots: Vec<Option<Box<dyn pilot::Pilot>>>,
    last: [Input; sim::body::SEATS],
    ids: [String; 2],
    player: bool,
}

#[wasm_bindgen]
impl Lab {
    /// Two opponents, left against right.
    pub fn watch(seed: u32, tuning: u8, left: &str, right: &str, map: &str) -> Lab {
        Lab {
            rec: Recording::new(content::setup::exhibition(seed as u64, tuning, [left, right], map)),
            pilots: [left, right].iter().map(|id| Some(pilot::build(&content::road::pilot(id)))).collect(),
            last: [Input::NONE; sim::body::SEATS],
            ids: [left.into(), right.into()],
            player: false,
        }
    }
    /// You, with the sword, against `opponent` as arcade mode sets it.
    pub fn play(seed: u32, tuning: u8, opponent: &str) -> Lab {
        let mut pilots: Vec<Option<Box<dyn pilot::Pilot>>> = vec![None];
        pilots.extend(content::road::crew(opponent).iter().map(|s| Some(pilot::build(s))));
        Lab {
            rec: Recording::new(content::setup::road(seed as u64, tuning, opponent)),
            pilots,
            last: [Input::NONE; sim::body::SEATS],
            ids: [String::new(), opponent.into()],
            player: true,
        }
    }
    /// A tree written in the editor (content::custom), on the left with the
    /// sword, against a road opponent; or why it cannot run.
    pub fn watch_custom(seed: u32, tuning: u8, tree: &str, right: &str, map: &str) -> Result<Lab, String> {
        let spec = content::custom::parse(tree).map_err(|r| r.key.to_string())?;
        Ok(Lab {
            rec: Recording::new(content::custom::watch(seed as u64, tuning, right, map)),
            pilots: vec![Some(pilot::build(&spec)), Some(pilot::build(&content::road::pilot(right)))],
            last: [Input::NONE; sim::body::SEATS],
            ids: ["custom".into(), right.into()],
            player: false,
        })
    }
    /// You against a tree written in the editor; or why it cannot run.
    pub fn play_custom(seed: u32, tuning: u8, tree: &str, map: &str) -> Result<Lab, String> {
        let spec = content::custom::parse(tree).map_err(|r| r.key.to_string())?;
        Ok(Lab {
            rec: Recording::new(content::custom::play(seed as u64, tuning, map)),
            pilots: vec![None, Some(pilot::build(&spec))],
            last: [Input::NONE; sim::body::SEATS],
            ids: [String::new(), "custom".into()],
            player: true,
        })
    }
    /// One tick: a person's keys for seat 0 when playing, each pilot's for
    /// its seat.
    pub fn step(&mut self, mine: u16, _other: u16) {
        let mut i = [Input::NONE; sim::body::SEATS];
        if self.player {
            i[0] = Input(mine);
        }
        for (k, p) in self.pilots.iter_mut().enumerate() {
            if let Some(p) = p {
                p.observe(self.last);
                i[k] = p.input(&self.rec.world, k);
            }
        }
        self.rec.step_all(i);
        self.last = i;
    }
    pub fn frame(&self) -> String {
        serde_json::to_string(&frame::frame(&self.rec.world)).unwrap()
    }
    /// Each piloted seat's last tick: `[{ seat, id, keys, active, held,
    /// failed, explain: [moves::Explain] }]`.
    pub fn report(&self) -> String {
        let all: Vec<serde_json::Value> = self
            .pilots
            .iter()
            .enumerate()
            .filter_map(|(k, p)| p.as_ref().map(|p| (k, p)))
            .map(|(k, p)| {
                let t = p.trace();
                let id = if k < 2 && !self.ids[k].is_empty() { self.ids[k].clone() } else { String::new() };
                json!({ "seat": k, "id": id, "keys": self.last[k].0, "active": t.active, "held": t.held, "failed": t.failed, "explain": p.explain() })
            })
            .collect();
        serde_json::to_string(&all).unwrap()
    }
    /// The keys each seat pressed on the last tick.
    pub fn keys(&self) -> Vec<u16> {
        self.last.iter().map(|i| i.0).collect()
    }
    pub fn phase_text(&self, _opponent: &str) -> String {
        if self.player {
            // A written tree has no road name; its sentences name the colors.
            if self.ids[1] == "custom" {
                return content::messages::phase_text(&self.rec.world, content::messages::Audience::Versus).to_string();
            }
            content::messages::phase_text(&self.rec.world, content::messages::Audience::Road { opponent: &self.ids[1] }).to_string()
        } else {
            let ids = [self.ids[0].as_str(), self.ids[1].as_str()];
            content::messages::phase_text(&self.rec.world, content::messages::Audience::Exhibition { ids }).to_string()
        }
    }
    pub fn checksum(&self) -> String {
        format!("{:016x}", self.rec.world.checksum())
    }
    pub fn tick(&self) -> u32 {
        self.rec.world.tick
    }
    pub fn is_replay(&self) -> bool {
        false
    }
    pub fn done(&self) -> bool {
        false
    }
    pub fn replay_bytes(&self) -> Vec<u8> {
        self.rec.bytes()
    }
    pub fn recorded_checksum(&self) -> String {
        String::new()
    }
}

/// Each road opponent, secret ones included, for the BT Lab's pickers: id,
/// row, weapon and the values its introduction's placeholders take.
#[wasm_bindgen]
pub fn lab_roster_json() -> String {
    let all: Vec<serde_json::Value> = content::road::road()
        .iter()
        .map(|st| json!({ "id": st.id, "level": st.level(), "weapon": st.weapon, "numbers": content::road::intro_numbers(&st.id), "kind": content::road::pilot(&st.id).kind() }))
        .collect();
    serde_json::to_string(&all).unwrap()
}

/// Whether a written tree runs: `{ ok }`, or `{ ok: false, key, rule }`
/// naming the reason as a copy key and the rule it is in.
#[wasm_bindgen]
pub fn lab_check(tree: &str) -> String {
    match content::custom::parse(tree) {
        Ok(_) => json!({ "ok": true }).to_string(),
        Err(r) => json!({ "ok": false, "key": r.key, "rule": r.rule }).to_string(),
    }
}

/// A written tree as core draws a pilot (pilot::view::describe), or null.
#[wasm_bindgen]
pub fn lab_describe_json(tree: &str) -> String {
    match content::custom::parse(tree) {
        Ok(spec) => serde_json::to_string(&pilot::view::describe(&spec)).unwrap(),
        Err(_) => "null".into(),
    }
}

/// What the editor offers: the conditions (name, unit of its number), the
/// moves, and the limits on a written tree.
#[wasm_bindgen]
pub fn lab_conditions_json() -> String {
    use content::custom as c;
    let conds: Vec<serde_json::Value> = c::CONDITIONS.iter().map(|(n, u)| json!({ "id": n, "unit": u })).collect();
    json!({
        "conditions": conds, "moves": pilot::moves::MOVES,
        "max_rules": c::MAX_RULES, "max_conds": c::MAX_CONDS,
        "reaction": [c::REACTION_TICKS.0, c::REACTION_TICKS.1], "distance": [c::DISTANCE_CM.0, c::DISTANCE_CM.1],
    })
    .to_string()
}

/// The moves a tree may run and each one's recipe (pilot::moves).
#[wasm_bindgen]
pub fn lab_moves_json() -> String {
    let all: Vec<serde_json::Value> = pilot::moves::MOVES.iter().map(|m| json!({ "id": m, "recipe": pilot::moves::recipe(m) })).collect();
    let pose = json!({ "share": pilot::moves::POSE_SHARE, "max": pilot::moves::POSE_MAX, "band": pilot::moves::POSE_BAND });
    json!({ "moves": all, "arms2": sim::Input::ARMS2, "arms": sim::Input::ARMS, "pose": pose, "throw_window_cm": pilot::THROW_WINDOW_CM }).to_string()
}

/// A story mode run (content::story): the scene's fight being played, the
/// run's lives, and what comes after each fight, all decided in core.
#[wasm_bindgen]
pub struct StoryRun {
    story: content::story::Story,
    run: content::story::Run,
    rec: Recording,
    pilots: Vec<Box<dyn pilot::Pilot>>,
    last: [Input; sim::body::SEATS],
    seed: u32,
    tuning: u8,
    weapon: String,
    four: bool,
    last_next: Option<content::story::Next>,
}

#[wasm_bindgen]
impl StoryRun {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, tuning: u8, chapter: usize, weapon: &str, four: bool) -> StoryRun {
        let story = content::story::story();
        let run = content::story::Run::new(&story, chapter);
        StoryRun::with(story, run, seed, tuning, weapon, four)
    }
    /// A full run: from the first chapter, the lives carried through, no
    /// chapter started over.
    pub fn full(seed: u32, tuning: u8, weapon: &str, four: bool) -> StoryRun {
        let story = content::story::story();
        let run = content::story::Run::full(&story);
        StoryRun::with(story, run, seed, tuning, weapon, four)
    }
    /// One scene of the extra chapter.
    pub fn extra(seed: u32, tuning: u8, index: usize, weapon: &str, four: bool) -> StoryRun {
        let story = content::story::story();
        let run = content::story::Run::extra(&story, index);
        StoryRun::with(story, run, seed, tuning, weapon, four)
    }
    fn with(story: content::story::Story, run: content::story::Run, seed: u32, tuning: u8, weapon: &str, four: bool) -> StoryRun {
        let mut s = StoryRun {
            rec: Recording::new(content::setup::versus(0, tuning)),
            pilots: Vec::new(),
            last: [Input::NONE; sim::body::SEATS],
            story,
            run,
            seed,
            tuning,
            weapon: weapon.into(),
            four,
            last_next: None,
        };
        s.begin();
        s
    }
    fn begin(&mut self) {
        let scene = self.run.scene(&self.story).clone();
        self.seed = self.seed.wrapping_mul(1103515245).wrapping_add(12345);
        self.rec = Recording::new(content::story::setup(self.seed as u64, self.tuning, &scene, self.run.fight, &self.weapon, self.four));
        self.pilots = content::story::crew(&scene, self.run.fight).iter().map(pilot::build).collect();
        self.last = [Input::NONE; sim::body::SEATS];
    }
    fn outcome(&self) -> content::story::Outcome {
        content::story::outcome(&self.rec.world, self.run.seconds(&self.story))
    }
    /// One tick, while the fight is being played; a fight that is decided
    /// stands still until `after`.
    pub fn step(&mut self, mine: u16, _other: u16) {
        if self.outcome() != content::story::Outcome::Playing {
            return;
        }
        let mut i = [Input(mine), Input::NONE, Input::NONE];
        for (k, p) in self.pilots.iter_mut().enumerate() {
            p.observe(self.last);
            i[k + 1] = p.input(&self.rec.world, k + 1);
        }
        self.rec.step_all(i);
        self.last = i;
    }
    /// `{ chapter, region, scene, scene_id, kind, fight, fights, stop,
    /// lives, seconds_left, outcome }`.
    pub fn status(&self) -> String {
        let scene = self.run.scene(&self.story);
        let left = (self.run.seconds(&self.story) * balance::TICKS_PER_SECOND).saturating_sub(self.rec.world.tick).div_ceil(balance::TICKS_PER_SECOND);
        let outcome = match self.outcome() {
            content::story::Outcome::Playing => "playing",
            content::story::Outcome::Won => "won",
            content::story::Outcome::Lost => "lost",
            content::story::Outcome::OutOfTime => "out_of_time",
        };
        json!({
            "chapter": self.run.chapter, "region": self.story.chapters[self.run.chapter].region,
            "scene": self.run.scene, "scene_id": scene.id, "kind": format!("{:?}", scene.kind).to_lowercase(),
            "fight": self.run.fight, "fights": scene.fights.len(), "stop": scene.fights[self.run.fight],
            "lives": self.run.lives, "seconds_left": left, "outcome": outcome,
            "full": self.run.full, "extra": self.run.extra, "rounds": scene.rounds,
        })
        .to_string()
    }
    /// After a decided fight: move the run on, start what comes next, and
    /// say what that was (`fight`, `scene`, `chapter`, `end`, `retry`,
    /// `continue`).
    pub fn after(&mut self) -> String {
        let o = self.outcome();
        if o == content::story::Outcome::Playing {
            return String::new();
        }
        let next = self.run.after(&self.story, o);
        self.last_next = Some(next);
        if !matches!(next, content::story::Next::End | content::story::Next::GameOver) {
            self.begin();
        }
        match next {
            content::story::Next::GameOver => "game_over".into(),
            n => format!("{n:?}").to_lowercase(),
        }
    }
    /// How many chapters this run has cleared, after the last `after`.
    pub fn cleared(&self) -> u32 {
        self.last_next.map(|n| self.run.cleared(&self.story, n)).unwrap_or(0)
    }
    /// The save with what the last `after` finished: a chapter finished,
    /// and, in a full run, the chapters it has cleared.
    pub fn record(&self, save_text: &str) -> Result<String, String> {
        let mut s = content::save::decode(save_text).map_err(|e| e.message().to_string())?;
        let cleared = self.cleared();
        if self.run.extra.is_none() {
            s.story = s.story.max(cleared);
            if self.run.full {
                s.story_full = s.story_full.max(cleared);
            }
        }
        Ok(content::save::encode(&s))
    }
    /// The save with the chapters finished so far: `chapter` (0-based) done.
    pub fn record_chapter(save_text: &str, chapter: u32) -> Result<String, String> {
        let mut s = content::save::decode(save_text).map_err(|e| e.message().to_string())?;
        s.story = s.story.max(chapter + 1);
        Ok(content::save::encode(&s))
    }
    pub fn frame(&self) -> String {
        serde_json::to_string(&frame::frame(&self.rec.world)).unwrap()
    }
    pub fn phase_text(&self, _opponent: &str) -> String {
        let stop = self.run.scene(&self.story).fights[self.run.fight].clone();
        content::messages::phase_text(&self.rec.world, content::messages::Audience::Road { opponent: &stop }).to_string()
    }
    pub fn checksum(&self) -> String {
        format!("{:016x}", self.rec.world.checksum())
    }
    pub fn tick(&self) -> u32 {
        self.rec.world.tick
    }
    pub fn is_replay(&self) -> bool {
        false
    }
    pub fn done(&self) -> bool {
        false
    }
    pub fn replay_bytes(&self) -> Vec<u8> {
        self.rec.bytes()
    }
    pub fn recorded_checksum(&self) -> String {
        String::new()
    }
}

/// The extra chapter for a save: each scene, what it needs and whether
/// it is open, and the save's best full run.
#[wasm_bindgen]
pub fn story_extra_json(save_text: &str) -> Result<String, String> {
    let s = content::save::decode(save_text).map_err(|e| e.message().to_string())?;
    let story = content::story::story();
    let extra: Vec<serde_json::Value> = story
        .extra
        .iter()
        .map(|x| json!({ "needs": x.needs, "open": s.story_full >= x.needs, "id": x.scene.id, "kind": format!("{:?}", x.scene.kind).to_lowercase(), "fights": x.scene.fights, "rounds": x.scene.rounds }))
        .collect();
    Ok(json!({ "best": s.story_full, "chapters": story.chapters.len(), "lives": story.lives, "extra": extra }).to_string())
}

/// Story mode's chapters and scenes, for its screen.
#[wasm_bindgen]
pub fn story_json() -> String {
    let s = content::story::story();
    json!({
        "lives": s.lives, "fight_seconds": s.fight_seconds,
        "chapters": s.chapters.iter().map(|c| json!({ "region": c.region, "scenes": c.scenes.iter().map(|sc| json!({ "id": sc.id, "kind": format!("{:?}", sc.kind).to_lowercase(), "fights": sc.fights, "hold_s": sc.hold_s, "rounds": sc.rounds })).collect::<Vec<_>>() })).collect::<Vec<_>>(),
    })
    .to_string()
}

/// The tree of fights for a save: each stop with its level, whether it is
/// open, how it has been won, its condition, the values its introduction's
/// placeholders take, and each requirement with whether it is met.
#[wasm_bindgen]
pub fn road_json(save_text: &str) -> Result<String, String> {
    let s = content::save::decode(save_text).map_err(|e| e.message().to_string())?;
    let best = &s.road.best;
    // A secret fight is left out until the final fight is won.
    let stops: Vec<serde_json::Value> = content::road::road()
        .iter()
        .filter(|st| content::road::visible(st, best))
        .map(|st| {
            let won = best.get(&st.id);
            let requires: Vec<serde_json::Value> = st
                .requires
                .iter()
                .map(|r| {
                    let (key, vars) = r.sentence();
                    json!({ "stop": r.stop(), "key": key, "vars": vars, "met": r.met(best) })
                })
                .collect();
            // The weapons a win here unlocks, Weapon Master's rewards.
            let rewards: Vec<serde_json::Value> = content::weapons::weapons()
                .into_iter()
                .filter(|w| !w.enemy_only && w.unlock.as_ref().is_some_and(|r| r.stop() == st.id) && content::weapons::unlock_shown(w, best))
                .map(|w| {
                    let r = w.unlock.clone().unwrap();
                    let (key, vars) = r.sentence();
                    json!({ "weapon": w.id, "key": key, "vars": vars, "stop": r.stop(), "met": r.met(best) })
                })
                .collect();
            json!({
                "id": st.id,
                "level": st.level(),
                "named_row": st.row.is_some(),
                "rewards": rewards,
                "weapon": st.weapon.as_deref().filter(|w| content::weapons::carried_shown(w, best)),
                "open": content::road::open(st, best),
                "won": won.is_some(),
                "flawless": won.is_some_and(|b| b.losses == 0),
                "condition": st.condition.map(|c| c.copy_key()),
                "companion": st.companion.as_ref().map(|c| json!({ "pilot": c.pilot, "weapon": c.weapon.clone().unwrap_or(content::weapons::DEFAULT.into()) })),
                "map": st.map,
                "numbers": content::road::intro_numbers(&st.id),
                "requires": requires,
            })
        })
        .collect();
    Ok(serde_json::Value::Array(stops).to_string())
}

/// One task of a tutorial mission: the yard or a fight on the road, with a
/// tracker in core that says when its goal is met.
#[wasm_bindgen]
pub struct Mission {
    rec: Recording,
    pilots: Vec<Box<dyn pilot::Pilot>>,
    last: [Input; sim::body::SEATS],
    tracker: content::tutorial::Tracker,
    mission: content::tutorial::Mission,
    part: usize,
}

#[wasm_bindgen]
impl Mission {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, tuning: u8, id: &str, part: usize) -> Mission {
        let mission = content::tutorial::mission(id).expect("a mission in data/tutorial.json");
        let task = mission.tasks[part].clone();
        let (setup, pilots) = if task.at == "yard" {
            (content::maps::on(content::setup::practice(seed as u64, tuning), task.map.as_deref().unwrap_or(content::maps::FLAT)), Vec::new())
        } else {
            (content::setup::road(seed as u64, tuning, &task.at), content::road::crew(&task.at).iter().map(pilot::build).collect())
        };
        Mission { rec: Recording::new(setup), pilots, last: [Input::NONE; sim::body::SEATS], tracker: content::tutorial::Tracker::new(task.goal), mission, part }
    }
    pub fn step(&mut self, mine: u16, _other: u16) {
        let mut i = [Input(mine), Input::NONE, Input::NONE];
        for (k, p) in self.pilots.iter_mut().enumerate() {
            p.observe(self.last);
            i[k + 1] = p.input(&self.rec.world, k + 1);
        }
        self.rec.step_all(i);
        self.tracker.observe(&self.rec.world, Input(mine));
        self.last = i;
    }
    pub fn frame(&self) -> String {
        serde_json::to_string(&frame::frame(&self.rec.world)).unwrap()
    }
    pub fn phase_text(&self, _opponent: &str) -> String {
        let task = &self.mission.tasks[self.part];
        if task.at == "yard" {
            return "{}".into();
        }
        content::messages::phase_text(&self.rec.world, content::messages::Audience::Road { opponent: &task.at }).to_string()
    }
    pub fn checksum(&self) -> String {
        format!("{:016x}", self.rec.world.checksum())
    }
    pub fn tick(&self) -> u32 {
        self.rec.world.tick
    }
    pub fn is_replay(&self) -> bool {
        false
    }
    pub fn done(&self) -> bool {
        false
    }
    pub fn replay_bytes(&self) -> Vec<u8> {
        self.rec.bytes()
    }
    pub fn recorded_checksum(&self) -> String {
        String::new()
    }
    /// The task's goal has been met.
    pub fn met(&self) -> bool {
        self.tracker.met()
    }
    /// After the goal is met: the save with this task kept, and the missions
    /// it opened, as `{ "save": text, "opened": [id] }`.
    pub fn record(&self, save_text: &str) -> Result<String, String> {
        let mut s = content::save::decode(save_text).map_err(|e| e.message().to_string())?;
        let opened = if self.met() { content::tutorial::record(&mut s.tutorial, &self.mission, self.part) } else { Vec::new() };
        Ok(json!({ "save": content::save::encode(&s), "opened": opened }).to_string())
    }
}

/// The tutorial's map for a save: each mission with its row, its chapter,
/// whether it is open and done, what it teaches and builds on, its tasks
/// with their sentences, and what it requires with whether each is done.
#[wasm_bindgen]
pub fn tutorial_json(save_text: &str) -> Result<String, String> {
    let s = content::save::decode(save_text).map_err(|e| e.message().to_string())?;
    Ok(content::tutorial::map(&s.tutorial).to_string())
}

/// The weapons for a save: each a player can carry, whether it is
/// unlocked, the requirement that unlocks it with whether it is met, and
/// whether it is the one carried.
#[wasm_bindgen]
pub fn weapons_json(save_text: &str) -> Result<String, String> {
    let s = content::save::decode(save_text).map_err(|e| e.message().to_string())?;
    let best = &s.road.best;
    // The weapons this player may see (content::weapons::shown): the sword,
    // the cursed blade, and those won or heard of by rumor.
    let list: Vec<serde_json::Value> = content::weapons::weapons()
        .into_iter()
        .filter(|w| content::weapons::shown(w, best))
        .map(|w| {
            let unlock = w.unlock.as_ref().filter(|_| content::weapons::unlock_shown(&w, best)).map(|r| {
                let (key, vars) = r.sentence();
                json!({ "stop": r.stop(), "key": key, "vars": vars, "met": r.met(best) })
            });
            let rumor = match &w.rumor {
                Some(content::weapons::Rumor::Start) if content::weapons::rumored(&w, best) => json!({ "from": null }),
                Some(content::weapons::Rumor::Beat(id)) if content::weapons::rumored(&w, best) => json!({ "from": id }),
                _ => serde_json::Value::Null,
            };
            json!({
                "id": w.id,
                "unlocked": content::weapons::unlocked(&w, best),
                "carried": s.weapon == w.id,
                "unlock": unlock,
                "rumor": rumor,
                "prize": w.prize,
            })
        })
        .collect();
    let (won, total) = content::weapons::prize_progress(best);
    Ok(json!({ "weapons": list, "prize": { "won": won, "total": total, "found": content::weapons::prize_found(best) } }).to_string())
}

/// One step of the liquid in the cursed blade's box (content::slosh): the
/// state as JSON in, moved toward `target` fullness and tipped by `push`,
/// as JSON out.
#[wasm_bindgen]
pub fn slosh_step(json: &str, target: f64, push: f64) -> String {
    let mut s: content::slosh::Slosh = serde_json::from_str(json).unwrap_or_else(|_| content::slosh::Slosh::new(target));
    content::slosh::step(&mut s, target, push);
    serde_json::to_string(&s).unwrap()
}

/// Whether the player has won four arms and fights with them, and what
/// wins them.
#[wasm_bindgen]
pub fn arms_json(save_text: &str) -> Result<String, String> {
    let s = content::save::decode(save_text).map_err(|e| e.message().to_string())?;
    let last = content::road::last().map(|st| st.id);
    // How many arms, counted from the body as hands, for the copy to say.
    let arms = content::body::four_armed(true).parts.iter().filter(|p| p.hand).count();
    Ok(json!({ "open": content::road::four_arms_open(&s.road.best), "on": s.four_arms, "stop": last, "arms": arms }).to_string())
}

/// The save with four arms on or off; on only once they are won.
#[wasm_bindgen]
pub fn save_choose_arms(save_text: &str, four: bool) -> Result<String, String> {
    let mut s = content::save::decode(save_text).map_err(|e| e.message().to_string())?;
    s.four_arms = four && content::road::four_arms_open(&s.road.best);
    Ok(content::save::encode(&s))
}

/// The save with `weapon` carried, if the player may carry it.
#[wasm_bindgen]
pub fn save_choose_weapon(save_text: &str, weapon: &str) -> Result<String, String> {
    let mut s = content::save::decode(save_text).map_err(|e| e.message().to_string())?;
    s.weapon = content::weapons::usable(weapon, &s.road.best);
    Ok(content::save::encode(&s))
}

/// A new save file's state, as JSON.
#[wasm_bindgen]
pub fn save_fresh() -> String {
    content::save::encode(&content::save::fresh())
}

/// Read a save file: its normalized text, or the sentence that refuses it.
#[wasm_bindgen]
pub fn save_read(text: &str) -> Result<String, String> {
    content::save::decode(text).map(|s| content::save::encode(&s)).map_err(|e| e.message().to_string())
}
