//! Story mode (Sam, 2026-10-06; analysis/story/PROPOSAL.md): chapters of
//! scenes along the road, three lives a run, a clock on every fight. The
//! rules of a run live here, where a test reaches them; the page shows the
//! state core reports and presses the buttons it offers.

use serde::Deserialize;
use sim::body::{Objective, Seat, Setup};
use sim::fx::Fx;
use sim::fight::Phase;
use sim::World;

pub const STORY_JSON: &str = include_str!("../../../data/story.json");

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Story {
    #[allow(dead_code)]
    _about: String,
    pub lives: u32,
    pub fight_seconds: u32,
    pub chapters: Vec<Chapter>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chapter {
    /// Which `road.region.*` names it.
    pub region: u8,
    pub scenes: Vec<Scene>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Duel,
    Waves,
    Hold,
    /// A stage to cross: reach its end, past the opponents.
    Cross,
    /// You and an ally against one.
    Team,
}

/// A stage to cross, in whole cm.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    /// The walls stand at plus and minus this.
    pub half: i32,
    /// Reach this to win the round.
    pub exit: i32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    /// The copy key `story.scene.<id>` says what it is.
    pub id: String,
    pub kind: Kind,
    /// Stops from data/road.json, fought in order.
    pub fights: Vec<String>,
    #[serde(default)]
    pub hold_s: Option<u32>,
    #[serde(default = "one")]
    pub rounds: u32,
    /// The opponent is a giant, half again its size (after Melee's giant
    /// opponents).
    #[serde(default)]
    pub giant: bool,
    /// Cross: the stage, and a second opponent (a stop) waiting near its end.
    #[serde(default)]
    pub stage: Option<Stage>,
    #[serde(default)]
    pub ahead: Option<String>,
    /// Team: the ally fighting beside you (a stop's pilot and weapon).
    #[serde(default)]
    pub ally: Option<String>,
}

fn one() -> u32 {
    1
}

/// How much bigger a giant is: half again.
pub const GIANT: (i64, i64) = (3, 2);

pub fn story() -> Story {
    serde_json::from_str(STORY_JSON).expect("data/story.json is valid")
}

/// The world for fight `fight` of a scene: the stop as arcade mode sets it,
/// played for the scene's rounds, with a hold-out objective if it has one.
pub fn setup(seed: u64, tuning: u8, scene: &Scene, fight: usize, weapon: &str) -> Setup {
    let mut s = crate::setup::road_with(seed, tuning, &scene.fights[fight], weapon);
    s.rounds_to_win = scene.rounds;
    if scene.giant {
        if let Some(seat) = s.seats[1].as_mut() {
            let big = crate::body::scaled(&s.bodies[seat.body as usize], GIANT.0, GIANT.1);
            s.bodies.push(big);
            seat.body = (s.bodies.len() - 1) as u8;
        }
    }
    // A second fighter in seat 2: the opponent ahead on a stage, facing the
    // player, or the ally beside the player, on the player's side.
    let extra = |s: &mut Setup, id: &str, x: i32, side: Option<u8>, facing: i8| {
        let w = crate::road::stop(id).and_then(|st| st.weapon).unwrap_or(crate::weapons::DEFAULT.to_string());
        let body = crate::setup::armed(&mut s.bodies, &w);
        s.seats[2] = Some(Seat { body, x: Fx::int(x), side, facing: Some(facing) });
    };
    if let (Kind::Cross, Some(st)) = (scene.kind, scene.stage.as_ref()) {
        s.arena_half = Fx::int(st.half);
        s.objective = Objective::Reach { x: Fx::int(st.exit) };
        // The player near the left wall, the opponent mid-stage facing it.
        if let Some(p) = s.seats[0].as_mut() {
            p.x = Fx::int(st.half - 300);
        }
        if let Some(o) = s.seats[1].as_mut() {
            o.x = Fx::int(0);
        }
        if let Some(a) = &scene.ahead {
            extra(&mut s, a, st.exit - 450, None, -1);
        }
    }
    if let (Kind::Team, Some(a)) = (scene.kind, scene.ally.as_ref()) {
        // Beside the player, a little behind it, facing the same way.
        let at = s.seats[0].map(|p| p.x).unwrap_or(sim::balance::START_X).trunc() + 160;
        extra(&mut s, a, at, Some(0), 1);
    }
    if let (Kind::Hold, Some(secs)) = (scene.kind, scene.hold_s) {
        s.objective = Objective::HoldOut { ticks: secs * sim::balance::TICKS_PER_SECOND };
    }
    s
}

/// Each opponent's and ally's pilot, by seat from seat 1: the stop's own,
/// then whoever sits in seat 2 (a companion, the opponent ahead, the ally).
pub fn crew(scene: &Scene, fight: usize) -> Vec<pilot::Spec> {
    let id = &scene.fights[fight];
    match (scene.kind, &scene.ahead, &scene.ally) {
        (Kind::Cross, Some(a), _) | (Kind::Team, _, Some(a)) => vec![crate::road::pilot(id), crate::road::pilot(a)],
        _ => crate::road::crew(id),
    }
}

/// How a fight stands.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Playing,
    Won,
    Lost,
    /// The fight's clock ran out first.
    OutOfTime,
}

/// A fight is won when its match is won, lost when its match is lost, and
/// lost too when its clock runs out first.
pub fn outcome(w: &World, fight_seconds: u32) -> Outcome {
    match w.phase {
        Phase::MatchOver { .. } if w.wins[0] > w.wins[1] => Outcome::Won,
        Phase::MatchOver { .. } => Outcome::Lost,
        _ if w.tick >= fight_seconds * sim::balance::TICKS_PER_SECOND => Outcome::OutOfTime,
        _ => Outcome::Playing,
    }
}

/// What comes after a fight.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Next {
    /// The scene's next fight (waves).
    Fight,
    Scene,
    /// The chapter is done; the run goes on into the next.
    Chapter,
    /// The last chapter is done.
    End,
    /// A life lost: the scene again from its first fight.
    Retry,
    /// No lives left: the chapter again from its first scene, lives restored.
    Continue,
}

/// Where a run stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub chapter: usize,
    pub scene: usize,
    pub fight: usize,
    pub lives: u32,
}

impl Run {
    pub fn new(story: &Story, chapter: usize) -> Run {
        Run { chapter, scene: 0, fight: 0, lives: story.lives }
    }

    pub fn scene<'a>(&self, story: &'a Story) -> &'a Scene {
        &story.chapters[self.chapter].scenes[self.scene]
    }

    /// Move on after a fight, and say where to.
    pub fn after(&mut self, story: &Story, o: Outcome) -> Next {
        match o {
            Outcome::Playing => unreachable!("a fight still being played has no after"),
            Outcome::Won => {
                if self.fight + 1 < self.scene(story).fights.len() {
                    self.fight += 1;
                    return Next::Fight;
                }
                self.fight = 0;
                if self.scene + 1 < story.chapters[self.chapter].scenes.len() {
                    self.scene += 1;
                    return Next::Scene;
                }
                self.scene = 0;
                if self.chapter + 1 < story.chapters.len() {
                    self.chapter += 1;
                    return Next::Chapter;
                }
                Next::End
            }
            Outcome::Lost | Outcome::OutOfTime => {
                self.fight = 0;
                if self.lives > 1 {
                    self.lives -= 1;
                    Next::Retry
                } else {
                    self.scene = 0;
                    self.lives = story.lives;
                    Next::Continue
                }
            }
        }
    }
}
