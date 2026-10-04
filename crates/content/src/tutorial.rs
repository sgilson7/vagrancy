//! The tutorial: a map of missions in the order the knowledge-component graph
//! teaches (Sam, 2026-10-04; data/kc_graph.json, data/tutorial.json,
//! analysis/kc/RESULTS.md). Each mission teaches one or more components,
//! names the edge back to one the player already has, and makes it concrete
//! in a drill in the yard or a fight on the road. Whether a mission's goal is
//! met is decided here, from the world, tick by tick; the page only asks.

use pilot::{airborne, pelvis, shoulder};
use serde::Deserialize;
use sim::fight::{Event, Phase};
use sim::{Input, World};
use std::collections::BTreeMap;

pub const TUTORIAL_JSON: &str = include_str!("../../../data/tutorial.json");
pub const KC_GRAPH_JSON: &str = include_str!("../../../data/kc_graph.json");

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Tutorial {
    #[allow(dead_code)]
    _about: String,
    chapters: Vec<String>,
    missions: Vec<Mission>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mission {
    pub id: String,
    /// A copy key for its name, for a mission whose first component would
    /// name it the same as another. Otherwise it is named by that
    /// component, or a comparison by its two opponents.
    #[serde(default)]
    pub name: Option<String>,
    /// `tutorial.chapter.<id>`.
    pub chapter: String,
    /// The knowledge components it teaches.
    pub teaches: Vec<String>,
    /// The edges, as `src_dst`, that tie what it teaches to what the player
    /// already has; their sentences are `kc_edge.<src>_<dst>`.
    #[serde(default)]
    pub builds_on: Vec<String>,
    /// Missions to finish first.
    #[serde(default)]
    pub requires: Vec<String>,
    /// Where it is played and what counts as done. A comparison is two of
    /// them, played one after the other (`kc_compare.<id>` says what they
    /// share).
    pub tasks: Vec<Task>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    /// `yard`, or a fight on the road by its id.
    pub at: String,
    pub goal: Goal,
    /// For a task in the yard, the practice step it repeats, if any. One
    /// without is an item the game had nowhere else.
    #[serde(default)]
    pub drill: Option<String>,
}

impl Task {
    /// The instance in the knowledge-component graph this task is, if the
    /// graph has one.
    pub fn instance(&self) -> Option<String> {
        if self.at == "yard" {
            self.drill.as_ref().map(|d| format!("drill/{d}"))
        } else {
            Some(format!("fight/{}", self.at))
        }
    }
}

/// What a task asks for. Each is read from the world, never from the keys
/// alone, except that a rise may not use the jump key.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Goal {
    /// The sword all the way round the shoulder in one direction. (A goal of
    /// where the tip is was met by standing still: the arm falls and swings
    /// like a pendulum, back and forth, and never adds up to a turn.)
    FullTurn,
    /// The tip this far in front of the pelvis, cm.
    Reach(i32),
    /// The pelvis this far from where it started, cm.
    Step(i32),
    /// Both feet off the ground.
    Airborne,
    /// The second jump spent while in the air.
    AirJump,
    /// A dodge started.
    Dodge,
    /// A dodge started on the ground with a direction held: a roll.
    Roll,
    /// A dodge started in the air.
    AirDodge,
    /// This many dodges started.
    Dodges(u32),
    /// A cut on the other fighter or the post.
    Cut,
    /// A cut on the other fighter that spills ink.
    SpillCut,
    /// Two blades met, this many times.
    Clashes(u32),
    /// The pelvis moving this fast, in cm per tick.
    Speed(i32),
    /// The sword's tip on the ground, then the pelvis this far above where
    /// it was at that moment within a second, without the jump key: a
    /// plant. (A rise alone could not tell a plant from a swing, which also
    /// leaves the ground.)
    Plant(i32),

    /// The match won.
    Win,
}

/// The goal's copy key and its values.
impl Goal {
    pub fn sentence(&self) -> (&'static str, Vec<(&'static str, String)>) {
        match self {
            Goal::FullTurn => ("tutorial.goal.full_turn", vec![]),
            Goal::Reach(cm) => ("tutorial.goal.reach", vec![("cm", cm.to_string())]),
            Goal::Step(cm) => ("tutorial.goal.step", vec![("cm", cm.to_string())]),
            Goal::Airborne => ("tutorial.goal.airborne", vec![]),
            Goal::AirJump => ("tutorial.goal.air_jump", vec![]),
            Goal::Dodge => ("tutorial.goal.dodge", vec![]),
            Goal::Roll => ("tutorial.goal.roll", vec![]),
            Goal::AirDodge => ("tutorial.goal.air_dodge", vec![]),
            Goal::Dodges(n) => ("tutorial.goal.dodges", vec![("count", n.to_string())]),
            Goal::Cut => ("tutorial.goal.cut", vec![]),
            Goal::SpillCut => ("tutorial.goal.spill_cut", vec![]),
            Goal::Clashes(n) => ("tutorial.goal.clashes", vec![("count", n.to_string())]),
            // cm a tick to metres a second, to one decimal place.
            Goal::Speed(cm) => {
                let tenths = cm * sim::balance::TICKS_PER_SECOND as i32 / 10;
                ("tutorial.goal.speed", vec![("speed", format!("{}.{}", tenths / 10, tenths % 10))])
            }
            Goal::Plant(cm) => ("tutorial.goal.plant", vec![("cm", cm.to_string())]),
            Goal::Win => ("tutorial.goal.win", vec![]),
        }
    }
}

fn tutorial() -> Tutorial {
    serde_json::from_str(TUTORIAL_JSON).expect("data/tutorial.json is valid")
}

pub fn chapters() -> Vec<String> {
    tutorial().chapters
}

pub fn missions() -> Vec<Mission> {
    tutorial().missions
}

pub fn mission(id: &str) -> Option<Mission> {
    missions().into_iter().find(|m| m.id == id)
}

/// The key a finished task is saved under: the mission, or for one part of
/// a comparison, `<mission>/<part>`.
pub fn part_key(m: &Mission, part: usize) -> String {
    if m.tasks.len() == 1 {
        m.id.clone()
    } else {
        format!("{}/{}", m.id, part)
    }
}

/// A mission is done when each of its tasks is.
pub fn done(m: &Mission, finished: &[String]) -> bool {
    (0..m.tasks.len()).all(|i| finished.contains(&part_key(m, i)))
}

/// Keep a finished task, and name the missions that opened because of it.
pub fn record(finished: &mut Vec<String>, m: &Mission, part: usize) -> Vec<String> {
    let all = missions();
    let before: Vec<bool> = all.iter().map(|x| open(x, finished)).collect();
    let key = part_key(m, part);
    if !finished.contains(&key) {
        finished.push(key);
    }
    all.into_iter().zip(before).filter(|(x, was)| !was && open(x, finished)).map(|(x, _)| x.id).collect()
}

/// The row a mission sits in on the map: one below the lowest mission it
/// requires.
pub fn rows() -> BTreeMap<String, usize> {
    let ms = missions();
    let mut row: BTreeMap<String, usize> = BTreeMap::new();
    // The file lists a mission after those it requires, which the tests
    // check, so one pass is enough.
    for m in &ms {
        let r = m.requires.iter().map(|x| row.get(x).copied().expect("a mission listed after those it requires") + 1).max().unwrap_or(0);
        row.insert(m.id.clone(), r);
    }
    row
}

/// The map, for a player's finished tasks.
pub fn map(finished: &[String]) -> serde_json::Value {
    let rows = rows();
    let ms = missions();
    let list: Vec<serde_json::Value> = ms
        .iter()
        .map(|m| {
            let tasks: Vec<serde_json::Value> = m
                .tasks
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let (key, vars) = t.goal.sentence();
                    let vars: BTreeMap<&str, String> = vars.into_iter().collect();
                    serde_json::json!({ "at": t.at, "goal": key, "vars": vars, "done": finished.contains(&part_key(m, i)) })
                })
                .collect();
            let requires: Vec<serde_json::Value> = m
                .requires
                .iter()
                .map(|r| serde_json::json!({ "id": r, "done": ms.iter().find(|x| &x.id == r).is_some_and(|x| done(x, finished)) }))
                .collect();
            serde_json::json!({
                "id": m.id,
                "name": m.name,
                "row": rows[&m.id],
                "chapter": m.chapter,
                "teaches": m.teaches,
                "builds_on": m.builds_on,
                "tasks": tasks,
                "requires": requires,
                "open": open(m, finished),
                "done": done(m, finished),
            })
        })
        .collect();
    serde_json::Value::Array(list)
}

/// A mission is open when each mission it requires is done.
pub fn open(m: &Mission, finished: &[String]) -> bool {
    let all = missions();
    m.requires.iter().all(|r| all.iter().find(|x| &x.id == r).is_some_and(|x| done(x, finished)))
}

/// Watches one task's world, tick by tick, for its goal (seat 0 is the
/// player).
#[derive(Clone, Debug)]
pub struct Tracker {
    goal: Goal,
    start: Option<(i32, i32)>,
    last_pelvis: Option<(i32, i32)>,
    dodges: u32,
    was_dodging: bool,
    clashes: u32,
    jumped: bool,
    /// When the tip last touched the ground: the tick, the pelvis height,
    /// and the pelvis speed then.
    planted: Option<(u32, i32, i32)>,
    tip_down: bool,
    /// The quadrant the sword points into from the shoulder, and quarter
    /// turns counted, plus one way and minus the other.
    quadrant: Option<i32>,
    quarters: i32,
    met: bool,
}

impl Tracker {
    pub fn new(goal: Goal) -> Tracker {
        Tracker { goal, start: None, last_pelvis: None, dodges: 0, was_dodging: false, clashes: 0, jumped: false, planted: None, tip_down: false, quadrant: None, quarters: 0, met: false }
    }

    pub fn met(&self) -> bool {
        self.met
    }

    /// After each step: the world as it now is, and the player's input that
    /// stepped it.
    pub fn observe(&mut self, w: &World, mine: Input) {
        if self.met {
            return;
        }
        if mine.has(Input::JUMP) {
            self.jumped = true;
        }
        let Some(f) = w.fighters[0].as_ref() else { return };
        let Some(p) = pelvis(w, 0) else { return };
        let here = (p.x.trunc(), p.y.trunc());
        let start = *self.start.get_or_insert(here);
        let speed = self.last_pelvis.map(|l| (here.0 - l.0).abs().max((here.1 - l.1).abs())).unwrap_or(0);
        self.last_pelvis = Some(here);
        let dodging = w.dodging(0);
        let began_dodge = dodging && !self.was_dodging;
        self.was_dodging = dodging;
        if began_dodge {
            self.dodges += 1;
        }
        let tip = w.swords.iter().find(|s| s.fighter == 0).map(|s| w.particles[s.tip as usize].p);
        for e in &w.events {
            if let Event::Clash { .. } = e {
                self.clashes += 1;
            }
        }
        // The tick a tip comes within a few cm of the ground starts a plant;
        // the push that follows keeps it there, and is not a new start.
        let down = tip.is_some_and(|t| t.y.trunc() <= 6);
        if down && !self.tip_down {
            self.planted = Some((w.tick, here.1, speed));
        }
        self.tip_down = down;
        if let (Some(t), Some(sh)) = (tip, shoulder(w, 0)) {
            let (dx, dy) = ((t.x - sh.x).0, (t.y - sh.y).0);
            let q = match (dx >= 0, dy >= 0) {
                (true, true) => 0,
                (false, true) => 1,
                (false, false) => 2,
                (true, false) => 3,
            };
            if let Some(was) = self.quadrant {
                match (q - was).rem_euclid(4) {
                    1 => self.quarters += 1,
                    3 => self.quarters -= 1,
                    _ => {}
                }
            }
            self.quadrant = Some(q);
        }
        let lifted = |cm: i32| !self.jumped && self.planted.is_some_and(|(t, y, _)| w.tick <= t + 60 && here.1 - y >= cm);
        let cut = |spilled_only: bool| {
            w.events.iter().any(|e| matches!(e, Event::Cut { seat: 1, by: 0, spilled, .. } if *spilled || !spilled_only))
        };
        self.met = match &self.goal {
            Goal::FullTurn => self.quarters.abs() >= 4,
            Goal::Reach(cm) => tip.is_some_and(|t| (t.x - p.x).trunc() * f.facing >= *cm),
            Goal::Step(cm) => (here.0 - start.0).abs() >= *cm,
            Goal::Airborne => airborne(w, 0),
            Goal::AirJump => airborne(w, 0) && f.air_jumps == 0,
            Goal::Dodge => began_dodge,
            Goal::Roll => began_dodge && !airborne(w, 0) && f.dodge_dir != 0,
            Goal::AirDodge => began_dodge && airborne(w, 0),
            Goal::Dodges(n) => self.dodges >= *n,
            Goal::Cut => cut(false),
            Goal::SpillCut => cut(true),
            Goal::Clashes(n) => self.clashes >= *n,
            Goal::Speed(cm) => speed >= *cm,
            Goal::Plant(cm) => lifted(*cm),
            Goal::Win => matches!(w.phase, Phase::MatchOver { .. }) && w.wins[0] > w.wins[1],
        };
    }
}
