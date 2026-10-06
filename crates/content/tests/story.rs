//! Story mode (Sam, 2026-10-06): its chapters, the rules of a run, and the
//! objectives a scene can have.

use content::story::{outcome, setup, story, Next, Outcome, Run};
use sim::fight::Event;
use sim::{Input, World};

#[test]
fn every_scene_builds_its_fights_and_has_its_words() {
    let s = story();
    let copy: serde_json::Value = serde_json::from_str(content::copy::COPY_JSON).unwrap();
    let regions: Vec<u8> = s.chapters.iter().map(|c| c.region).collect();
    assert_eq!(regions, (0..8).collect::<Vec<u8>>(), "the chapters do not walk the road's eight regions in order");
    for c in &s.chapters {
        assert!(copy["story"]["chapter_intro"][c.region.to_string()].is_string(), "chapter {} has no introduction", c.region);
        for sc in &c.scenes {
            assert!(copy["story"]["scene"][&sc.id].is_string(), "scene {} has no words", sc.id);
            for f in 0..sc.fights.len() {
                let w = World::new(setup(1, sim::balance::DEFAULT_TUNING, sc, f, "sword"));
                assert_eq!(w.setup.rounds_to_win, sc.rounds, "{}: fight {f} is not played for its rounds", sc.id);
            }
        }
    }
}

#[test]
fn a_run_moves_on_after_a_win_and_costs_a_life_after_a_loss() {
    let s = story();
    let mut r = Run::new(&s, 1);
    // Chapter 2 opens on three drovers, one after another.
    assert_eq!(r.after(&s, Outcome::Won), Next::Fight);
    assert_eq!(r.fight, 1);
    // A loss: a life, and the scene again from its first fight.
    assert_eq!(r.after(&s, Outcome::Lost), Next::Retry);
    assert_eq!((r.scene, r.fight, r.lives), (0, 0, s.lives - 1));
    // Running out of time is a loss too.
    assert_eq!(r.after(&s, Outcome::OutOfTime), Next::Retry);
    assert_eq!(r.lives, s.lives - 2);
    // The last life: the chapter again, lives restored.
    assert_eq!(r.after(&s, Outcome::Won), Next::Fight);
    assert_eq!(r.after(&s, Outcome::Lost), Next::Continue);
    assert_eq!((r.chapter, r.scene, r.fight, r.lives), (1, 0, 0, s.lives));
    // Through the chapter: the drovers, a loss to the courier, then the
    // courier beaten; the life lost carries into the next chapter.
    let drovers = s.chapters[1].scenes[0].fights.len();
    for _ in 0..drovers {
        r.after(&s, Outcome::Won);
    }
    assert_eq!(r.scene, 1);
    assert_eq!(r.after(&s, Outcome::Lost), Next::Retry);
    assert_eq!(r.after(&s, Outcome::Won), Next::Chapter);
    assert_eq!((r.chapter, r.lives), (2, s.lives - 1));
    // And the last chapter's last fight ends the story.
    let mut end = Run::new(&s, s.chapters.len() - 1);
    let fights: usize = s.chapters.last().unwrap().scenes.iter().map(|sc| sc.fights.len()).sum();
    let mut last = Next::Fight;
    for _ in 0..fights {
        last = end.after(&s, Outcome::Won);
    }
    assert_eq!(last, Next::End);
}

#[test]
fn a_hold_out_round_is_won_on_the_tick_its_time_is_up() {
    // Against the scarecrow, which does not fight back, standing still:
    // the round ends with the player the winner exactly when the time is up.
    let s = story();
    let hold = s.chapters.iter().flat_map(|c| &c.scenes).find(|sc| sc.hold_s.is_some()).unwrap().clone();
    let mut sc = hold.clone();
    sc.fights = vec!["scarecrow".into()];
    let mut w = World::new(setup(1, sim::balance::DEFAULT_TUNING, &sc, 0, "sword"));
    let want = hold.hold_s.unwrap() * sim::balance::TICKS_PER_SECOND;
    let mut ended = None;
    while w.tick < want + 10 && ended.is_none() {
        w.step([Input::NONE, Input::NONE]);
        if let Some(Event::RoundEnd { result }) = w.events.iter().find(|e| matches!(e, Event::RoundEnd { .. })) {
            ended = Some((w.round_ticks, *result));
        }
    }
    let (_, result) = ended.expect("the hold-out round never ended");
    assert_eq!(result.loser, Some(1));
    assert_eq!(result.cause, sim::body::Cause::HeldOut);
    assert_eq!(w.tick, want, "the hold-out round ended at tick {}, not {want}", w.tick);
    assert_eq!(outcome(&w, s.fight_seconds), Outcome::Won);
}

#[test]
fn a_fight_still_undecided_when_its_clock_runs_out_is_lost() {
    let s = story();
    let sc = &s.chapters[0].scenes[0];
    let mut w = World::new(setup(1, sim::balance::DEFAULT_TUNING, sc, 0, "sword"));
    while outcome(&w, s.fight_seconds) == Outcome::Playing {
        w.step([Input::NONE, Input::NONE]);
    }
    assert_eq!(outcome(&w, s.fight_seconds), Outcome::OutOfTime);
    assert_eq!(w.tick, s.fight_seconds * sim::balance::TICKS_PER_SECOND);
}

#[test]
fn a_giant_stands_half_again_as_tall_and_can_be_beaten() {
    // A scene's giant, left alone for five seconds, is still on its feet,
    // and its head stands about half again as high as the player's.
    let mut sc = story().chapters[0].scenes[1].clone();
    sc.giant = true;
    sc.fights = vec!["cooper".into()];
    let mut w = World::new(setup(1, sim::balance::DEFAULT_TUNING, &sc, 0, "sword"));
    for _ in 0..300 {
        w.step([Input::NONE, Input::NONE]);
    }
    assert!(!w.knocked_down(1), "the giant fell over standing still");
    let h = |s| pilot::head(&w, s).unwrap().y.trunc();
    let (me, it) = (h(0), h(1));
    assert!(it * 10 >= me * 14 && it * 10 <= me * 16, "the giant's head is at {it} cm and the player's at {me}");
    // The yardstick beats it in at least one of eight matches.
    let won = (0..8u64).any(|seed| {
        let mut ps = content::road::lineup(&content::road::pilot("yardstick"), "cooper");
        let o = pilot::duel(setup(seed, sim::balance::DEFAULT_TUNING, &sc, 0, "sword"), &mut ps, 60 * 120);
        o.finished && o.wins[0] > o.wins[1]
    });
    assert!(won, "the yardstick never beat the giant cooper");
}

fn scene(id: &str) -> content::story::Scene {
    story().chapters.iter().flat_map(|c| c.scenes.clone()).find(|s| s.id == id).unwrap_or_else(|| panic!("no scene {id}"))
}

#[test]
fn a_stage_to_cross_is_won_by_walking_to_its_end() {
    // The first chapter's stage: the player starts near the left wall of an
    // arena three screens wide and wins the round on reaching the shrine,
    // walking past opponents who press nothing.
    let sc = scene("c1_cross");
    let st = sc.stage.as_ref().unwrap();
    let mut w = World::new(setup(1, sim::balance::DEFAULT_TUNING, &sc, 0, "sword"));
    let start = pilot::pelvis(&w, 0).unwrap().x.trunc();
    assert!(start < -st.half + 400, "the player starts at {start} cm, not near the left wall");
    let ahead = pilot::pelvis(&w, 2).unwrap().x.trunc();
    assert!(w.fighters[2].as_ref().unwrap().facing == -1 && ahead > 0, "the opponent ahead stands at {ahead} cm facing away");
    let mut ended = None;
    while w.tick < 60 * 60 && ended.is_none() {
        w.step_all([Input(Input::STEP_RIGHT), Input::NONE, Input::NONE]);
        if let Some(Event::RoundEnd { result }) = w.events.iter().find(|e| matches!(e, Event::RoundEnd { .. })) {
            ended = Some(*result);
        }
    }
    let r = ended.expect("walking right never ended the round");
    assert_eq!((r.loser, r.cause), (Some(1), sim::body::Cause::Reached), "the stage ended {r:?}");
    assert!(pilot::pelvis(&w, 0).unwrap().x.trunc() >= st.exit);
}

#[test]
fn in_a_team_fight_the_ally_fights_the_opponent_and_never_the_player() {
    let sc = scene("c3_team");
    let mut w = World::new(setup(1, sim::balance::DEFAULT_TUNING, &sc, 0, "sword"));
    assert_eq!((w.side_of(0), w.side_of(1), w.side_of(2)), (0, 1, 0));
    assert_eq!(pilot::foe(&w, 2), 1, "the ally does not fight the opponent");
    let mut ps: Vec<Box<dyn pilot::Pilot>> = content::story::crew(&sc, 0).iter().map(pilot::build).collect();
    let mut last = [Input::NONE; sim::body::SEATS];
    let mut ally_cuts = 0;
    while w.tick < 60 * 30 && matches!(w.phase, sim::fight::Phase::Fight) {
        let mut i = [Input::NONE; sim::body::SEATS];
        for (k, p) in ps.iter_mut().enumerate() {
            p.observe(last);
            i[k + 1] = p.input(&w, k + 1);
        }
        w.step_all(i);
        last = i;
        for e in &w.events {
            if let Event::Cut { seat, by: 2, .. } = *e {
                assert_ne!(seat, 0, "the ally cut the player at tick {}", w.tick);
                ally_cuts += 1;
            }
            if let Event::RoundEnd { result } = e {
                assert!(result.loser != Some(0) || w.out(2) || w.fighters[2].as_ref().unwrap().out_at.is_some(), "the player's side lost with the ally still in");
            }
        }
    }
    assert!(ally_cuts > 0, "the ally never cut the opponent");
}
