//! The road (D16; PLANNING-BRIEF M5): pilots, their introductions, and the
//! order of the stops.

use content::copy::{copy, placeholders};
use content::road::{intro_numbers, pilot, pilots, record, road, stops, Best, Req};
use std::collections::BTreeMap;
use pilot::{build, duel, gap, Spec};
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
        p.observe([Input::NONE; sim::body::SEATS]);
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
    // Derived, never typed: 90 ticks is 1.5 s; the ferryman's longsword,
    // 133 % of the default 104 cm, is 138 cm, round(100 · 34 / 104) = 33 %
    // longer (data/weapons.json); 18 and 12 ticks
    // are 300 and 200 ms.
    assert_eq!(intro_numbers("thresher")["pause_s"], "1.5");
    assert_eq!(intro_numbers("ferryman")["reach_pct"], "33");
    assert_eq!(intro_numbers("reader")["horizon_ms"], "300");
    assert_eq!(intro_numbers("reader")["reaction_ms"], "200");
    // And they move when the data does.
    let changed = pilot::numbers(&Spec::Loop { pattern: "overhead".into(), pause_ticks: 120, drift: false });
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
        p.observe([Input(Input::SHOULDER_UP), Input::NONE, Input::NONE]);
        let i = p.input(&w, 1);
        assert!(w.round > 1 || i.0 & arm == 0, "the sampler attacked in the first round");
        w.step([Input(Input::SHOULDER_UP), i]);
    }
}

/// The yardstick's wins out of 200 at each stop, from analysis/ladder.md,
/// after checking the table is the current one. A count comes from a
/// command: `make ladder`.
///
/// `None` while a ladder is pending for exactly this data: Sam asked to
/// deploy before a ladder finished (2026-10-05), so `make ladder-pending`
/// records the fingerprint the running ladder will measure, and the checks
/// that read the table wait for it. Any change to the pilots, the stops or
/// the simulation after that changes the fingerprint, and the stale table
/// fails again. `make ladder` removes the marker when it writes the table.
fn ladder() -> Option<std::collections::BTreeMap<String, u32>> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../analysis/");
    let md = std::fs::read_to_string(format!("{root}ladder.md")).expect("analysis/ladder.md: run `make ladder`");
    let now = content::road::ladder_fingerprint();
    if !md.contains(&format!("fingerprint {now}")) {
        let pending = std::fs::read_to_string(format!("{root}ladder.pending")).unwrap_or_default();
        if pending.trim() == now {
            eprintln!("analysis/ladder.md is pending for fingerprint {now}: the checks that read it wait for `make ladder`");
            return None;
        }
        panic!("analysis/ladder.md is stale for this data and simulation: run `make ladder`");
    }
    Some(
        stops()
            .into_iter()
            .map(|id| {
                let row = md.lines().find(|l| l.starts_with(&format!("| {id} |"))).unwrap_or_else(|| panic!("no row for {id}"));
                let won: u32 = row.split('|').nth(2).unwrap().split_whitespace().next().unwrap().parse().unwrap();
                (id, won)
            })
            .collect(),
    )
}

#[test]
fn the_tree_is_a_tree_whose_levels_are_counts_of_requirements() {
    // Sam: "a downwards facing tree ... think a hasse diagram for a poset
    // where each level is the number of requirements to unlock a fight".
    let road = road();
    let ids: Vec<&str> = road.iter().map(|s| s.id.as_str()).collect();
    let c = copy();
    let ps = pilots();
    for (i, s) in road.iter().enumerate() {
        assert!(!ids[..i].contains(&s.id.as_str()), "{} is on the road twice", s.id);
        assert!(ps.contains_key(&s.id), "{} has no pilot", s.id);
        assert!(c["opponents"][&s.id]["name"].is_string(), "{} has no strings", s.id);
        let mut named = Vec::new();
        for r in &s.requires {
            let from = road.iter().find(|t| t.id == r.stop()).unwrap_or_else(|| panic!("{} requires {}, which is not on the road", s.id, r.stop()));
            // Each requirement points up the tree, so nothing can require
            // itself, however indirectly, and every fight can be opened.
            assert!(from.level() < s.level(), "{} (level {}) requires {} (level {}), which is not above it", s.id, s.level(), from.id, from.level());
            assert!(!named.contains(&r.stop()), "{} asks twice about {}", s.id, r.stop());
            named.push(r.stop());
        }
    }
    // Open from the start: the scarecrow, and the fights added to carry
    // weapons (Sam: two per chapter).
    for s in road.iter().filter(|s| s.level() == 0) {
        assert!(s.id == "scarecrow" || s.weapon.is_some(), "{} is open from the start and carries no weapon", s.id);
    }
    // Every level from the top to the bottom has a fight in it.
    let deepest = road.iter().map(|s| s.level()).max().unwrap();
    for l in 0..=deepest {
        assert!(road.iter().any(|s| s.level() == l), "level {l} is empty");
    }
}

#[test]
fn the_chart_has_a_route_into_every_fight_from_the_row_above() {
    // The chart draws one route into each fight, from a requirement in the
    // row directly above it (Sam: "just one line per layer between nodes").
    let road = road();
    for s in road.iter().filter(|s| s.level() > 0) {
        let above = s.requires.iter().any(|r| road.iter().any(|t| t.id == r.stop() && t.level() + 1 == s.level()));
        assert!(above, "{} has no requirement in the row above it, so the chart has no route into it", s.id);
    }
}

#[test]
fn each_chapter_has_two_weapon_carriers_and_each_challenge_can_be_met() {
    // Sam: "add 2 enemies per chapter that use different weapons, that are
    // unlockable with weapon challenges like beat the courier with the
    // trident".
    let road = road();
    let deepest = road.iter().map(|s| s.level()).max().unwrap();
    // The final fight's row holds it alone (crates/content/tests/deity.rs).
    let last = content::road::last().map(|s| s.level());
    for l in (0..=deepest).filter(|&l| Some(l) != last) {
        let carriers = road.iter().filter(|s| s.level() == l && s.weapon.is_some()).count();
        assert!(carriers >= 2, "row {l} has {carriers} opponents carrying a weapon");
        if l > 0 {
            let challenged = road.iter().filter(|s| s.level() == l && s.requires.iter().any(|r| matches!(r, Req::With { .. }))).count();
            assert!(challenged >= 2, "row {l} has {challenged} fights opened by a weapon challenge");
        }
    }
    // A challenge's weapon is won in a row above the fight that asks for
    // it, so it can be in hand by then; and an enemy's weapon is never asked
    // for.
    let level = |id: &str| road.iter().find(|s| s.id == id).unwrap().level();
    for s in &road {
        for r in &s.requires {
            if let Req::With { weapon, .. } = r {
                let w = content::weapons::weapon(weapon).unwrap_or_else(|| panic!("{} asks for {weapon}, which is not a weapon", s.id));
                assert!(!w.enemy_only, "{} asks the player to carry the {weapon}, which is the enemies'", s.id);
                if let Some(u) = &w.unlock {
                    assert!(level(u.stop()) < s.level(), "{} asks for the {weapon}, which is won at {} in a row not above it", s.id, u.stop());
                }
            }
        }
    }
}

#[test]
fn a_win_opens_the_fights_that_asked_for_it_and_no_others() {
    let mut best = BTreeMap::new();
    let open_now = |b: &BTreeMap<String, Best>| road().into_iter().filter(|s| content::road::open(s, b)).map(|s| s.id).collect::<Vec<_>>();
    assert_eq!(open_now(&best), ["scarecrow", "tinker", "pilgrim"]);
    // A win with a round lost opens what a win opens; the drover asks for a
    // flawless one.
    let opened = record(&mut best, "scarecrow", Best::won(1, 2000, "sword"));
    assert_eq!(opened, ["thresher", "courier", "juggler"]);
    // A flawless win carrying the short sword opens the drover (flawless)
    // and the knife grinder (the short sword's challenge).
    let opened = record(&mut best, "scarecrow", Best::won(0, 3000, "short_sword"));
    assert_eq!(opened, ["drover", "knife_grinder"]);
    assert_eq!(best["scarecrow"], Best { losses: 0, ticks: 2000, with: vec!["short_sword".into(), "sword".into()], headshot: false, untouched: false, thrown: false, all_thrown: false }, "each part of the best is kept on its own");
    // A quick requirement: the ropewalker asks for the courier in 90 s.
    for id in ["thresher", "drover", "sampler", "salt_trader"] {
        record(&mut best, id, Best::won(2, 99_999, "sword"));
    }
    record(&mut best, "courier", Best::won(2, 90 * 60 + 1, "sword"));
    assert!(!open_now(&best).contains(&"ropewalker".to_string()), "a courier win a tick over 90 s opened the ropewalker");
    let opened = record(&mut best, "courier", Best::won(2, 90 * 60, "sword"));
    assert!(opened.contains(&"ropewalker".to_string()), "a courier win in 90 s did not open the ropewalker: {opened:?}");
}

#[test]
fn the_tree_gets_no_easier_going_down() {
    // Along every requirement, the fight below is no easier for the
    // yardstick than the fight that opens it, within the noise of two
    // 200-match rates (two standard errors of their difference near 40 %:
    // 2·√(2·0.24/200) ≈ 10 points, 20 wins). And each level, on average, is
    // no easier than the level above it.
    let Some(rates) = ladder() else { return };
    const NOISE: u32 = 20;
    let road = road();
    for s in &road {
        for r in &s.requires {
            let (above, below) = (rates[r.stop()], rates[&s.id]);
            assert!(below <= above + NOISE, "the yardstick wins {below} of 200 at {} and {above} at {}, which opens it", s.id, r.stop());
        }
    }
    let deepest = road.iter().map(|s| s.level()).max().unwrap();
    let mean = |l: usize| {
        let v: Vec<u32> = road.iter().filter(|s| s.level() == l).map(|s| rates[&s.id]).collect();
        v.iter().sum::<u32>() as f64 / v.len() as f64
    };
    for l in 1..=deepest {
        assert!(mean(l) <= mean(l - 1), "level {l} averages {:.0} wins and level {} {:.0}", mean(l), l - 1, mean(l - 1));
    }
}

#[test]
fn the_ten_and_the_five_added_with_the_tree_are_as_hard_as_the_reader_and_the_archivist() {
    // Sam: "10 more fights that are as difficult as the reader, and 5 that
    // are as difficult as the archivist". As hard as: within the same noise
    // of the reader's or the archivist's own rate.
    let Some(rates) = ladder() else { return };
    const NOISE: i64 = 20;
    for (like, ids) in [
        ("reader", &["dyer", "potter", "carpenter", "mason", "weaver", "falconer", "boatwright", "brewer", "herbalist", "cartographer"][..]),
        ("archivist", &["magistrate", "abbot", "warden", "hermit", "tanner"][..]),
    ] {
        for id in ids {
            let d = rates[*id] as i64 - rates[like] as i64;
            assert!(d.abs() <= NOISE, "the yardstick wins {} of 200 at {id} and {} at the {like}", rates[*id], rates[like]);
        }
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
                            let mut ps = content::road::lineup(cfg, &id);
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

/// Each tree opponent's inputs over a few matches against the yardstick,
/// and whether it ever went from the ground to standing on a ledge, which
/// takes a jump (and for a high ledge the stand key: crates/content/tests/
/// maps.rs). On a flanked stop, the opponent's own (seat 1), not its
/// companion's.
fn keys_and_ledge(id: &str) -> (Vec<Input>, bool) {
    let mut all = Vec::new();
    let mut ledge = false;
    for seed in 0..2u64 {
        let mut w = World::new(content::setup::road(seed, sim::balance::DEFAULT_TUNING, id));
        let mut ps = content::road::lineup(&pilot("yardstick"), id);
        let mut last = [Input::NONE; sim::body::SEATS];
        while w.tick < 1800 && !matches!(w.phase, Phase::MatchOver { .. }) {
            let mut i = [Input::NONE; sim::body::SEATS];
            for (k, p) in ps.iter_mut().enumerate() {
                p.observe(last);
                i[k] = p.input(&w, k);
            }
            if matches!(w.phase, Phase::Fight) {
                all.push(i[1]);
            }
            w.step_all(i);
            ledge |= pilot::on_ledge(&w, 1);
            last = i;
        }
    }
    (all, ledge)
}

fn keys_pressed(id: &str) -> Vec<Input> {
    keys_and_ledge(id).0
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
    for id in ["bellringer", "courier", "salt_trader", "smith", "archivist", "watchman", "dyer", "herbalist", "magistrate", "abbot", "warden", "hermit"] {
        assert!(uses(id, Input::DODGE), "the {id} never dodged");
    }
    // The tree's: the dyer sweeps low, the potter and the boatwright thrust,
    // the falconer and the carpenter leave the ground.
    assert!(keys_pressed("dyer").iter().any(|i| i.has(Input::SHOULDER_DOWN)), "the dyer never swept low");
    for id in ["potter", "boatwright"] {
        assert!(uses(id, Input::ELBOW_OUT), "the {id} never thrust");
    }
    assert!(uses("falconer", Input::JUMP), "the falconer never jumped");
    // The lamplighter thrusts: the elbow bends and straightens.
    assert!(uses("lamplighter", Input::ELBOW_OUT), "the lamplighter never thrust");
    // "he sometimes throws it", "he often throws it", "usually throws it".
    for id in ["juggler", "woodcutter", "harpooner"] {
        assert!(uses(id, Input::THROW), "the {id} never threw");
    }
    // The flanked fights. The well digger sweeps low; the charcoal burner
    // thrusts; the stone cutter swings overhead; the tea picker, the net
    // mender, the shrine keeper and the toll collector dodge.
    assert!(uses("well_digger", Input::SHOULDER_DOWN), "the well digger never swept low");
    assert!(uses("charcoal_burner", Input::ELBOW_OUT), "the charcoal burner never thrust");
    assert!(uses("stone_cutter", Input::SHOULDER_UP), "the stone cutter never swung overhead");
    for id in ["tea_picker", "net_mender", "shrine_keeper", "toll_collector"] {
        assert!(uses(id, Input::DODGE), "the {id} never dodged");
    }
}

#[test]
fn each_opponent_that_climbs_gets_up_onto_a_ledge() {
    // "jumps and stands on it", in the introductions of five of the flanked
    // fights.
    for id in ["roofer", "tea_picker", "bridge_keeper", "kite_maker", "toll_collector"] {
        let (keys, ledge) = keys_and_ledge(id);
        assert!(keys.iter().any(|i| i.has(Input::JUMP)), "the {id} never jumped");
        assert!(ledge, "the {id} never stood on a ledge");
    }
}

#[test]
fn after_a_win_that_opens_nothing_the_next_goal_is_the_nearest_locked_fight() {
    // Sam: "if you dont have any new fights available, prompt you to do the
    // next fight that you need to do that has constraints you havent met
    // yet".
    use content::road::next_goal;
    let mut best = BTreeMap::new();
    // A fresh road: the scarecrow's fights each ask for one thing. The
    // thresher, first in the road's order, asks for a win at the scarecrow.
    assert_eq!(next_goal(&best), Some(("thresher".to_string(), Req::Beat("scarecrow".into()))));
    // After a win with a round lost, the goal is unmet, for a fight still
    // locked, and at a fight that is open now.
    record(&mut best, "scarecrow", Best::won(1, 2000, "sword"));
    let (open_next, req) = next_goal(&best).unwrap();
    assert!(!req.met(&best), "the next goal, {req:?}, is already met");
    let st = content::road::stop(&open_next).unwrap();
    assert!(!content::road::open(&st, &best), "{open_next} is already open");
    let from = content::road::stop(req.stop()).unwrap();
    assert!(content::road::open(&from, &best), "the next goal asks for a fight at {}, which is not open", req.stop());
    // A weapon challenge is suggested only with a weapon already won: with
    // the first fight won every way and nothing else, the ox herd's
    // challenge asks for the scimitar, which the pilgrim has not yet given.
    let mut first = BTreeMap::new();
    let all: Vec<String> = content::weapons::weapons().into_iter().filter(|w| !w.enemy_only).map(|w| w.id).collect();
    first.insert("scarecrow".to_string(), Best { losses: 0, ticks: 1, with: all.clone(), headshot: true, untouched: true, thrown: true, all_thrown: true });
    let (_, req) = next_goal(&first).expect("a next goal");
    if let Req::With { weapon, .. } = &req {
        assert_eq!(&content::weapons::usable(weapon, &first), weapon, "the next goal asks for the {weapon}, which is locked");
    }
    // With the scimitar won at the pilgrim, the ox herd's challenge (beat
    // the tinker carrying the scimitar) is the goal: a weapon challenge the
    // player can take up, which the page switches the weapon for.
    let mut armed = first.clone();
    armed.insert("pilgrim".to_string(), Best::won(1, 9999, "sword"));
    let (opens, req) = next_goal(&armed).expect("a next goal");
    assert_eq!((opens.as_str(), req), ("ox_herd", Req::With { stop: "tinker".into(), weapon: "scimitar".into() }));
    // Nothing is suggested once every fight is open.
    for s in road() {
        best.insert(s.id, Best { losses: 0, ticks: 1, with: content::weapons::weapons().into_iter().map(|w| w.id).collect(), headshot: true, untouched: true, thrown: true, all_thrown: true });
    }
    assert_eq!(next_goal(&best), None);
}
