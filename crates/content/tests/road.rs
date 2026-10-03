//! The road (D16; PLANNING-BRIEF M5): pilots, their introductions, and the
//! order of the stops.

use content::copy::{copy, placeholders};
use content::road::{intro_numbers, pilot, pilots, stops};
use pilot::{build, duel, gap, Pilot, Spec};
use sim::fight::Phase;
use sim::{Input, World};

fn road_world(id: &str, seed: u64) -> World {
    World::new(content::setup::road(seed, sim::balance::DEFAULT_TUNING, id))
}

/// Run a pilot in seat 1 against a player who does nothing, and keep its inputs.
fn watch(id: &str, ticks: u32) -> (World, Vec<Input>) {
    let mut w = road_world(id, 1);
    let mut p = build(&pilot(id));
    let mut seen = Vec::new();
    for _ in 0..ticks {
        p.observe(Input::NONE);
        let i = p.input(&w, 1);
        seen.push(i);
        w.step([Input::NONE, i]);
    }
    (w, seen)
}

#[test]
fn a_pilot_returns_an_input_and_nothing_else() {
    // The type says it: `Pilot::input` takes `&World` and returns `Input`.
    // What is left to check is that every byte is one a player could press,
    // and that a pilot is as deterministic as the world it reads.
    for id in pilots().keys() {
        let (w1, a) = watch(id, 600);
        let (w2, b) = watch(id, 600);
        assert!(a.iter().all(|i| i.0 & Input::SPARE == 0), "{id} pressed the spare bit");
        assert_eq!(a, b, "{id} chose differently on the same world");
        assert_eq!(w1.checksum(), w2.checksum(), "{id}'s matches differ");
    }
}

#[test]
fn every_number_in_an_introduction_comes_from_the_pilot_data() {
    let c = copy();
    for id in stops() {
        let numbers = intro_numbers(&id);
        for field in ["does", "try"] {
            let text = c["opponents"][&id][field].as_str().unwrap();
            for p in placeholders(text) {
                assert!(numbers.contains_key(p), "{id}.{field} wants {{{p}}} and the pilot data does not give it");
            }
        }
    }
    // Derived, never typed: 90 ticks is 1.5 s; a 138 cm sword against the
    // default 104 cm is round(100 · 34 / 104) = 33 % longer; 18 and 12 ticks
    // are 300 and 200 ms.
    assert_eq!(intro_numbers("thresher")["pause_s"], "1.5");
    assert_eq!(intro_numbers("ferryman")["reach_pct"], "33");
    assert_eq!(intro_numbers("reader")["horizon_ms"], "300");
    assert_eq!(intro_numbers("reader")["reaction_ms"], "200");
    // And they move when the data does.
    let changed = pilot::numbers(&Spec::Loop { pattern: "overhead".into(), pause_ticks: 120, drift: false }, 95);
    assert_eq!(changed, vec![("pause_s", "2".to_string())]);
}

#[test]
fn what_an_introduction_says_a_pilot_usually_does_is_what_it_does() {
    let arm = Input::SHOULDER_UP | Input::SHOULDER_DOWN | Input::ELBOW_IN | Input::ELBOW_OUT;
    // "The scarecrow does not move and does not fight back."
    let (_, s) = watch("scarecrow", 600);
    assert!(s.iter().all(|i| i.0 & !Input::READY == 0), "the scarecrow pressed a key");
    // "It is here so that your first cut costs you nothing": running into its
    // still sword for five seconds does not cut you.
    let mut w = road_world("scarecrow", 1);
    let mut p = build(&pilot("scarecrow"));
    for t in 0..300 {
        let i = p.input(&w, 1);
        w.step([Input(Input::STEP_RIGHT), i]);
        for e in &w.events {
            if let sim::fight::Event::Cut { seat: 0, by: 1, .. } = e {
                panic!("tick {t}: the scarecrow's still sword cut a player who ran into it");
            }
        }
    }
    // "The gatekeeper holds his sword out in front of him and keeps it there.
    // He does not swing." Held means the tip stays in one small place in
    // front of him once he has raised it: with tuning 2 a single tick of any
    // arm key moves a joint at up to 0.2 rad/tick and nothing holds an arm
    // against gravity, so a held pose is a stream of small corrections, not
    // stillness (SECOND-ORDER-M5).
    {
        let mut w = road_world("gatekeeper", 1);
        let mut p = build(&pilot("gatekeeper"));
        let (mut lo, mut hi) = (sim::fx::V2::cm(10_000, 10_000), sim::fx::V2::cm(-10_000, -10_000));
        let mut in_front = true;
        for t in 0..600 {
            let i = p.input(&w, 1);
            w.step([Input::NONE, i]);
            if t > 180 {
                let s = w.swords.iter().find(|s| s.fighter == 1).unwrap();
                let tip = w.particles[s.tip as usize].p;
                lo = sim::fx::V2::new(lo.x.min(tip.x), lo.y.min(tip.y));
                hi = sim::fx::V2::new(hi.x.max(tip.x), hi.y.max(tip.y));
                // Seat 1 faces -x: in front of him is toward smaller x.
                in_front &= tip.x < pilot::pelvis(&w, 1).unwrap().x;
            }
        }
        let (wide, tall) = ((hi.x - lo.x).trunc(), (hi.y - lo.y).trunc());
        // A swing carries the tip through an arc longer than the blade; a
        // held sword keeps it inside less than the blade's own length. This
        // build measured 24 by 67 cm (SECOND-ORDER-M5 row 2).
        let blade = content::road::default_sword_len();
        assert!(in_front, "the gatekeeper's sword went behind him");
        assert!(wide < blade && tall < blade, "the gatekeeper's tip wandered over {wide} by {tall} cm, more than the {blade} cm blade: that is a swing");
    }
    // "The thresher makes the same overhead swing again and again, with the
    // same pause after each swing", of `pause_ticks`.
    let (_, s) = watch("thresher", 900);
    let Spec::Loop { pause_ticks, .. } = pilot("thresher") else { panic!() };
    let mut quiet = Vec::new();
    let mut run = 0;
    for i in &s {
        if i.0 & arm == 0 {
            run += 1;
        } else if run > 0 {
            quiet.push(run);
            run = 0;
        }
    }
    // The thresher closes in and its swing reaches the motionless player
    // within the run, which ends the round; two whole pauses before that are
    // what there is to count.
    assert!(quiet.len() >= 2, "the thresher paused only {} times", quiet.len());
    assert!(quiet.iter().all(|&q| q == pause_ticks), "the thresher's pauses were {quiet:?}, not {pause_ticks} each");
    // "The windmill does not stop swinging ... and then drifts toward you."
    // Counted while the round is fought: a round it ends early is followed by
    // "ready", as for every pilot.
    let mut w = road_world("windmill", 1);
    let mut p = build(&pilot("windmill"));
    let start = gap(&w, 1);
    let mut closest = start;
    for t in 0..300 {
        let fighting = matches!(w.phase, Phase::Fight);
        let i = p.input(&w, 1);
        assert!(!fighting || i.has(Input::SHOULDER_UP), "the windmill stopped swinging at tick {t}");
        w.step([Input::NONE, i]);
        closest = closest.min(gap(&w, 1));
    }
    assert!(closest < start - 100, "the windmill did not drift toward the player: {start} cm to {closest}");
    // "The sampler ... watches the first round without attacking."
    let mut w = road_world("sampler", 1);
    let mut p = build(&pilot("sampler"));
    for _ in 0..300 {
        p.observe(Input(Input::SHOULDER_UP));
        let i = p.input(&w, 1);
        assert!(w.round > 1 || i.0 & arm == 0, "the sampler attacked in the first round");
        w.step([Input(Input::SHOULDER_UP), i]);
    }
}

#[test]
fn the_road_is_ordered_by_the_yardstick() {
    // A count comes from a command: `make ladder` plays 200 seeded matches
    // per stop and writes analysis/ladder.md. This checks that the table is
    // the current one and that the road's order follows it.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../analysis/ladder.md");
    let md = std::fs::read_to_string(path).expect("analysis/ladder.md: run `make ladder`");
    assert!(md.contains(&format!("fingerprint {}", content::road::ladder_fingerprint())), "analysis/ladder.md is stale for this data and simulation: run `make ladder`");
    let mut rates = Vec::new();
    for id in stops() {
        let row = md.lines().find(|l| l.starts_with(&format!("| {id} |"))).unwrap_or_else(|| panic!("no row for {id}"));
        let won: u32 = row.split('|').nth(2).unwrap().split_whitespace().next().unwrap().parse().unwrap();
        rates.push((id, won));
    }
    for pair in rates.windows(2) {
        assert!(pair[1].1 <= pair[0].1, "the yardstick wins more at {} ({}) than at {} ({}): reorder data/road.json", pair[1].0, pair[1].1, pair[0].0, pair[0].1);
    }
}

#[test]
fn every_opponent_can_be_beaten() {
    // By the yardstick at its strongest (D16): its configuration in
    // data/pilots.json and two that look further ahead, 16 seeds each; a
    // stop is beatable if any of them wins a match. A longer horizon alone is
    // not stronger here: at 24 ticks with a 4-tick reaction it never beat
    // the gatekeeper, where the data's own yardstick did (SECOND-ORDER-M5).
    let Spec::Search { horizon_ticks, reaction_ticks, branches, period } = pilot("yardstick") else { panic!("the yardstick is a search") };
    let configs = [
        Spec::Search { horizon_ticks, reaction_ticks, branches, period },
        Spec::Search { horizon_ticks: horizon_ticks + 10, reaction_ticks, branches, period },
        Spec::Search { horizon_ticks: horizon_ticks + 10, reaction_ticks: reaction_ticks / 2, branches, period: period.max(2) - 1 },
    ];
    let beaten: Vec<(String, Option<(usize, u64)>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = stops()
            .into_iter()
            .map(|id| {
                let configs = configs.clone();
                scope.spawn(move || {
                    for (c, cfg) in configs.iter().enumerate() {
                        for seed in 0..16u64 {
                            let mut ps: [Box<dyn Pilot>; 2] = [build(cfg), build(&pilot(&id))];
                            let o = duel(content::setup::road(seed, sim::balance::DEFAULT_TUNING, &id), &mut ps, 60 * 120);
                            if o.finished && o.wins[0] > o.wins[1] {
                                return (id, Some((c, seed)));
                            }
                        }
                    }
                    (id, None)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let unbeaten: Vec<&String> = beaten.iter().filter(|(_, w)| w.is_none()).map(|(id, _)| id).collect();
    assert!(unbeaten.is_empty(), "the yardstick at its strongest never beat {unbeaten:?}");
}

/// Each tree opponent's inputs over a few matches against the yardstick.
fn keys_pressed(id: &str) -> Vec<Input> {
    let mut all = Vec::new();
    for seed in 0..2u64 {
        let mut w = World::new(content::setup::road(seed, sim::balance::DEFAULT_TUNING, id));
        let mut me = build(&pilot("yardstick"));
        let mut them = build(&pilot(id));
        let mut last = [Input::NONE; 2];
        while w.tick < 1800 && !matches!(w.phase, Phase::MatchOver { .. }) {
            me.observe(last[1]);
            them.observe(last[0]);
            let i = [me.input(&w, 0), them.input(&w, 1)];
            if matches!(w.phase, Phase::Fight) {
                all.push(i[1]);
            }
            w.step(i);
            last = i;
        }
    }
    all
}

#[test]
fn each_new_opponent_uses_the_moves_its_introduction_names() {
    let uses = |id: &str, bit: u16| keys_pressed(id).iter().any(|i| i.has(bit));
    // "walks straight at you and swings low … He does not guard."
    let drover = keys_pressed("drover");
    assert!(drover.iter().any(|i| i.has(Input::SHOULDER_DOWN)), "the drover never swung low");
    assert!(!drover.iter().any(|i| i.has(Input::JUMP) || i.has(Input::DODGE)), "the drover jumped or dodged");
    // Jumps: the cooper, the ropewalker (twice, the second in the air).
    assert!(uses("cooper", Input::JUMP), "the cooper never jumped");
    let rope = keys_pressed("ropewalker");
    let presses = rope.windows(2).filter(|w| w[1].has(Input::JUMP) && !w[0].has(Input::JUMP)).count();
    assert!(presses >= 4, "the ropewalker jumped {presses} times; her bounce takes two presses");
    // Dodges: the bellringer, the courier, the salt trader, the smith, the
    // archivist and the watchman.
    for id in ["bellringer", "courier", "salt_trader", "smith", "archivist", "watchman"] {
        assert!(uses(id, Input::DODGE), "the {id} never dodged");
    }
    // The lamplighter thrusts: the elbow bends and straightens.
    assert!(uses("lamplighter", Input::ELBOW_OUT), "the lamplighter never thrust");
}
