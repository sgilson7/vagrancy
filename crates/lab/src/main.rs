//! The bench. Recon, the ladder and the replay runner live here as
//! subcommands; nothing in this crate is shipped.
//!
//!     cargo run --release -p lab -- <command>

use sim::fx::Fx;
use sim::{Input, World};

fn cm(f: Fx) -> f64 {
    f.0 as f64 / 4096.0
}

fn pose(w: &World) -> String {
    let f = w.fighters[0].as_ref().unwrap();
    let def = &w.setup.bodies[f.body as usize];
    let at = |i: Option<u8>| w.particles[(f.base + i.unwrap() as u16) as usize].p;
    let (s, p, h) = (at(def.roles.shoulder), at(def.roles.pelvis), at(def.roles.head));
    format!(
        "pelvis ({:7.2},{:7.2})  shoulder ({:7.2},{:7.2})  head ({:7.2},{:7.2})",
        cm(p.x), cm(p.y), cm(s.x), cm(s.y), cm(h.x), cm(h.y)
    )
}

fn stand(args: &[String]) {
    let ticks: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(600);
    let mut w = World::new(content::setup::alone(1, sim::balance::DEFAULT_TUNING));
    println!("tick {:>4}: {}", 0, pose(&w));
    for t in 1..=ticks {
        w.step([Input::NONE; 2]);
        if t % (ticks / 10).max(1) == 0 {
            println!("tick {:>4}: {}", t, pose(&w));
        }
    }
}

fn points(args: &[String]) {
    let ticks: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(5);
    let mut w = World::new(content::setup::alone(1, sim::balance::DEFAULT_TUNING));
    for t in 0..=ticks {
        if t > 0 {
            w.step([Input::NONE; 2]);
        }
        let row: Vec<String> = w.particles.iter().map(|p| format!("({:.1},{:.1})", cm(p.p.x), cm(p.p.y))).collect();
        println!("{t:>3}: {}", row.join(" "));
    }
}

/// M1.0: how far a fighter left alone drifts, and what a tick costs.
fn recon_m1() {
    let mut w = World::new(content::setup::alone(1, sim::balance::DEFAULT_TUNING));
    let f = w.fighters[0].clone().unwrap();
    let pel = (f.base + w.setup.bodies[0].roles.pelvis.unwrap() as u16) as usize;
    let p0 = w.particles[pel].p;
    for _ in 0..600 {
        w.step([Input::NONE; 2]);
    }
    let p1 = w.particles[pel].p;
    println!(
        "frac_bits {}: pelvis after 600 ticks alone moved {:.3} cm sideways and {:.3} cm down",
        sim::fx::FRAC_BITS,
        cm(p1.x - p0.x),
        cm(p0.y - p1.y)
    );
    let mut w = World::new(content::setup::versus(1, sim::balance::DEFAULT_TUNING));
    let mut rng = sim::rng::Rng::new(7);
    let n = 20_000u32;
    let t0 = std::time::Instant::now();
    for _ in 0..n {
        let a = Input(rng.below(64) as u16);
        let b = Input(rng.below(64) as u16);
        w.step([a, b]);
    }
    let us = t0.elapsed().as_secs_f64() * 1e6 / n as f64;
    println!("one tick, two fighters, random input, native release: {us:.1} µs ({n} ticks)");
}

/// M1.0: how fast the step keys carry a fighter, and whether it stays up.
fn recon_step() {
    let mut w = World::new(content::setup::alone(1, sim::balance::DEFAULT_TUNING));
    let f = w.fighters[0].clone().unwrap();
    let pel = (f.base + w.setup.bodies[0].roles.pelvis.unwrap() as u16) as usize;
    let x0 = w.particles[pel].p.x;
    let mut log = Vec::new();
    for t in 1..=240u32 {
        // Right for 2 s, then stand still for 2 s.
        let i = if t <= 120 { Input(Input::STEP_RIGHT) } else { Input::NONE };
        w.step([i, Input::NONE]);
        if t % 30 == 0 {
            log.push(format!("{:.0}", cm(w.particles[pel].p.x - x0)));
        }
    }
    println!(
        "run {:.1} cm/tick: every half second, pelvis x moved {} cm; pelvis height after {:.1} cm",
        cm(sim::balance::RUN_SPEED),
        log.join(", "),
        cm(w.particles[pel].p.y)
    );
}

/// Hold one key and watch the sword tip and the pelvis.
fn swing(args: &[String]) {
    let key = match args.first().map(String::as_str) {
        Some("down") => Input::SHOULDER_DOWN,
        Some("in") => Input::ELBOW_IN,
        Some("out") => Input::ELBOW_OUT,
        _ => Input::SHOULDER_UP,
    };
    let ticks: u32 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(120);
    let mut w = World::new(content::setup::alone(1, sim::balance::DEFAULT_TUNING));
    let f = w.fighters[0].clone().unwrap();
    let pel = (f.base + w.setup.bodies[0].roles.pelvis.unwrap() as u16) as usize;
    let tip = w.swords[0].tip as usize;
    for t in 1..=ticks {
        w.step([Input(key), Input::NONE]);
        if t % 10 == 0 {
            let (p, q) = (w.particles[tip].p, w.particles[tip].q);
            println!(
                "{t:>4}: tip ({:7.1},{:6.1}) speed {:5.1} cm/tick   pelvis ({:7.1},{:5.1})",
                cm(p.x), cm(p.y), cm((p - q).len()), cm(w.particles[pel].p.x), cm(w.particles[pel].p.y)
            );
        }
    }
}

/// Record the golden replay: a fixed script on a fixed seed, long enough to
/// cover standing, running, swinging both joints and both seats.
fn golden() {
    let mut rec = sim::replay::Recording::new(content::setup::versus(2026, sim::balance::DEFAULT_TUNING));
    for t in 0..1_800u32 {
        rec.step(sim::replay::script(t));
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testing/replays/golden.replay");
    std::fs::write(path, rec.bytes()).unwrap();
    println!("wrote {path}: {} ticks, checksum {:016x}", rec.world.tick, rec.world.checksum());
}

/// The save files Sam asked for (2026-10-06), in testing/saves/: each fight
/// won 3-0 but the village deity, to beat it and see the guardian deity
/// appear; and each fight won 3-0 but the guardian deity, to fight it.
pub const TEST_SAVES: [(&str, &[&str]); 2] = [
    ("all-but-the-village-deity.save.json", &["local_deity", "guardian_deity"]),
    ("all-but-the-guardian-deity.save.json", &["guardian_deity"]),
];

fn test_saves() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testing/saves");
    std::fs::create_dir_all(dir).unwrap();
    for (name, unwon) in TEST_SAVES {
        let path = format!("{dir}/{name}");
        std::fs::write(&path, content::save::encode(&content::save::finished(unwon))).unwrap();
        println!("wrote {path}");
    }
}

/// The checksum the gate compares the browser against.
fn script_checksum(args: &[String]) {
    let ticks: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(600);
    println!("{}", sim::replay::script_checksum_of(content::setup::versus(2026, sim::balance::DEFAULT_TUNING), ticks));
}

/// Run a script of (input, ticks) and report the highest the pelvis rose
/// above where it stood, and how far it travelled.
fn run_script(tuning: u8, script: &[(u16, u32)]) -> (f64, f64, u32) {
    let mut w = World::new(content::setup::alone(1, tuning));
    let f = w.fighters[0].clone().unwrap();
    let pel = (f.base + w.setup.bodies[0].roles.pelvis.unwrap() as u16) as usize;
    let (x0, y0) = (w.particles[pel].p.x, w.particles[pel].p.y);
    let mut best = 0.0f64;
    let mut cap_tick = 0;
    let cap = w.setup.physics.tuning.cap;
    let mut t = 0;
    for &(b, n) in script {
        for _ in 0..n {
            w.step([Input(b), Input::NONE]);
            t += 1;
            best = best.max(cm(w.particles[pel].p.y - y0));
            if cap_tick == 0 && w.particles.iter().any(|p| (p.p - p.q).len() >= cap - Fx::int(1)) {
                cap_tick = t;
            }
        }
    }
    (best, cm(w.particles[pel].p.x - x0), cap_tick)
}

fn recon_m2() {
    use sim::Input as I;
    // The pogo the test uses, the spin, and a swing's carry in the air.
    let pogo: Vec<(u16, u32)> = vec![(I::ELBOW_IN, 13), (0, 9), (I::SHOULDER_DOWN | I::ELBOW_OUT, 40), (0, 60)];
    let spin: Vec<(u16, u32)> = vec![(I::SHOULDER_UP, 240)];
    for tuning in 0..sim::balance::TUNINGS.len() as u8 {
        let t = sim::balance::TUNINGS[tuning as usize];
        let (h, dx, _) = run_script(tuning, &pogo);
        let (_, sdx, cap) = run_script(tuning, &spin);
        println!(
            "tuning {tuning} (accel {:.3}, speed {:.3} rad/tick, cap {:.0} cm/tick, drag {:.4}): pogo script rises {h:.1} cm and carries {dx:.1} cm; \
             4 s of shoulder-up on the ground moves {sdx:.1} cm; a particle reaches the cap: {}",
            t.motor_accel.0 as f64 / 4096.0, t.motor_speed.0 as f64 / 4096.0, cm(t.cap), t.drag.0 as f64 / 4096.0,
            if cap == 0 { "never".to_string() } else { format!("at tick {cap}") }
        );
    }
}

/// Trace a script: tip, hand and pelvis every few ticks.
fn trace(args: &[String]) {
    // Script as "keys:ticks,keys:ticks" where keys are letters u d i o (arm) l r (steps) or 0.
    let script: Vec<(u16, u32)> = args.first().map(|s| s.split(',').map(|part| {
        let (k, n) = part.split_once(':').unwrap();
        let mut b = 0u16;
        for c in k.chars() {
            b |= match c { 'u' => Input::SHOULDER_UP, 'd' => Input::SHOULDER_DOWN, 'i' => Input::ELBOW_IN,
                           'o' => Input::ELBOW_OUT, 'l' => Input::STEP_LEFT, 'r' => Input::STEP_RIGHT, _ => 0 };
        }
        (b, n.parse().unwrap())
    }).collect()).unwrap_or_default();
    let tuning: u8 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(sim::balance::DEFAULT_TUNING);
    let mut w = World::new(content::setup::alone(1, tuning));
    let f = w.fighters[0].clone().unwrap();
    let def = &w.setup.bodies[0];
    let pel = (f.base + def.roles.pelvis.unwrap() as u16) as usize;
    let tip = w.swords[0].tip as usize;
    let butt = w.swords[0].butt as usize;
    let mut t = 0;
    for (b, n) in script {
        for _ in 0..n {
            w.step([Input(b), Input::NONE]);
            t += 1;
            if t % 5 == 0 {
                let (pt, pb, pp) = (w.particles[tip].p, w.particles[butt].p, w.particles[pel].p);
                println!("{t:>4} [{b:06b}] tip ({:6.1},{:6.1}) butt ({:6.1},{:6.1}) pelvis ({:6.1},{:6.1})",
                    cm(pt.x), cm(pt.y), cm(pb.x), cm(pb.y), cm(pp.x), cm(pp.y));
            }
        }
    }
}

/// M2.0: can any short key sequence lift the fighter? Random search over
/// arm-only scripts; reports the best rise per tuning and its script.
fn pogo_search(args: &[String]) {
    let tries: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(1500);
    let keys = [Input::SHOULDER_UP, Input::SHOULDER_DOWN, Input::ELBOW_IN, Input::ELBOW_OUT,
                Input::SHOULDER_UP | Input::ELBOW_OUT, Input::SHOULDER_DOWN | Input::ELBOW_OUT,
                Input::SHOULDER_DOWN | Input::ELBOW_IN, 0];
    for tuning in 0..sim::balance::TUNINGS.len() as u8 {
        let mut r = sim::rng::Rng::new(99);
        let mut best = (0.0f64, Vec::new());
        for _ in 0..tries {
            let n = 2 + r.below(4) as usize;
            let script: Vec<(u16, u32)> = (0..n).map(|_| (keys[r.below(8) as usize], 3 + r.below(40))).chain([(0, 60)]).collect();
            let (h, _, _) = run_script(tuning, &script);
            if h > best.0 {
                best = (h, script);
            }
        }
        println!("tuning {tuning}: best rise {:.1} cm with {:?}", best.0, best.1);
    }
}

/// M2.0: the pogo as the copy describes it — point the tip at the ground,
/// then straighten — over a grid of timings, counting only runs where the
/// fighter does not fall first and the tip is on the ground while it rises.
fn pogo_grid() {
    for tuning in 0..sim::balance::TUNINGS.len() as u8 {
        let mut best = (0.0f64, 0u32, 0u32, 0u16);
        for &first in &[Input::SHOULDER_DOWN, Input::SHOULDER_DOWN | Input::ELBOW_IN] {
            for n1 in (4..=60).step_by(2) {
                for n2 in (4..=40).step_by(2) {
                    let mut w = World::new(content::setup::alone(1, tuning));
                    let f = w.fighters[0].clone().unwrap();
                    let pel = (f.base + w.setup.bodies[0].roles.pelvis.unwrap() as u16) as usize;
                    let tip = w.swords[0].tip as usize;
                    let y0 = w.particles[pel].p.y;
                    let mut fell = false;
                    let mut planted_rise = 0.0f64;
                    let script = [(first, n1), (Input::ELBOW_OUT, n2), (0u16, 60)];
                    for (b, n) in script {
                        for _ in 0..n {
                            w.step([Input(b), Input::NONE]);
                            let rise = cm(w.particles[pel].p.y - y0);
                            if rise < -25.0 {
                                fell = true;
                            }
                            if !fell && w.particles[tip].p.y.0 <= 4096 {
                                planted_rise = planted_rise.max(rise);
                            }
                            if !fell && b == 0 {
                                planted_rise = planted_rise.max(if planted_rise > 1.0 { rise } else { 0.0 });
                            }
                        }
                    }
                    if !fell && planted_rise > best.0 {
                        best = (planted_rise, n1, n2, first);
                    }
                }
            }
        }
        println!("tuning {tuning}: planted pogo rises {:.1} cm: keys {:06b} for {} ticks, then elbow-out for {}", best.0, best.3, best.1, best.2);
    }
}

/// H5's numbers: momentum and kinetic energy after a held shoulder key, floating.
fn h5() {
    let mut setup = content::setup::alone(1, sim::balance::DEFAULT_TUNING);
    setup.physics.gravity = Fx(0);
    setup.physics.ground = false;
    setup.physics.walls = false;
    setup.physics.tuning.drag = Fx(0);
    setup.physics.tuning.cap = Fx::int(100_000);
    let mut w = World::new(setup);
    for t in 1..=60 {
        w.step([Input(Input::SHOULDER_UP), Input::NONE]);
        if t % 10 == 0 {
            let (px, py) = w.momentum();
            println!("tick {t}: momentum ({:.1}, {:.1}) mass·cm/tick, |p| {:.1}, kinetic {:.0}",
                px as f64 / 4096.0, py as f64 / 4096.0, ((px as f64).hypot(py as f64)) / 4096.0, w.kinetic() as f64 / 4096.0 / 4096.0);
        }
    }
}

/// Does the solver alone create energy? A floating fighter is set moving by
/// its arm for a while, then left alone with drag off: its kinetic energy
/// must not grow.
fn energy() {
    let mut setup = content::setup::alone(1, sim::balance::DEFAULT_TUNING);
    setup.physics.gravity = Fx(0);
    setup.physics.ground = false;
    setup.physics.walls = false;
    setup.physics.tuning.drag = Fx(0);
    setup.physics.tuning.cap = Fx::int(100_000);
    let mut w = World::new(setup);
    let mut r = sim::rng::Rng::new(4);
    for t in 0..400u32 {
        let i = if t < 40 { Input(r.below(16) as u16) } else { Input::NONE };
        w.step([i, Input::NONE]);
        if t % 40 == 39 {
            println!("tick {:>3}: kinetic {:>8.0}   momentum {:?}", t + 1, w.kinetic() as f64 / 4096.0 / 4096.0, w.momentum());
        }
    }
}

/// M2.0/M3.0: how often a fighter cuts itself — standing still, running,
/// and under 100 runs of random input.
fn self_cuts() {
    use sim::fight::Event;
    let count = |w: &World| w.events.iter().filter(|e| matches!(e, Event::Cut { seat, by, spilled: true, .. } if seat == by)).count();
    for (label, keys) in [("standing still", vec![0u16]), ("running right", vec![Input::STEP_RIGHT])] {
        let mut w = World::new(content::setup::alone(1, sim::balance::DEFAULT_TUNING));
        let mut first = None;
        for t in 0..600u32 {
            w.step([Input(keys[0]), Input::NONE]);
            if first.is_none() && count(&w) > 0 {
                first = Some(t);
            }
        }
        println!("{label}: first self-cut at tick {first:?}");
    }
    let mut runs_with = 0;
    let mut ticks = Vec::new();
    for seed in 0..100u64 {
        let mut w = World::new(content::setup::alone(seed, sim::balance::DEFAULT_TUNING));
        let mut r = sim::rng::Rng::new(seed + 500);
        let mut held = Input::NONE;
        for t in 0..600u32 {
            if t % 15 == 0 {
                held = Input(r.below(64) as u16);
            }
            w.step([held, Input::NONE]);
            if count(&w) > 0 {
                runs_with += 1;
                ticks.push(t);
                break;
            }
        }
    }
    // Which parts: tally the first self-cut's part over the same runs.
    let mut tally = std::collections::BTreeMap::new();
    for seed in 0..100u64 {
        let mut w = World::new(content::setup::alone(seed, sim::balance::DEFAULT_TUNING));
        let mut r = sim::rng::Rng::new(seed + 500);
        let mut held = Input::NONE;
        'run: for t in 0..600u32 {
            if t % 15 == 0 {
                held = Input(r.below(64) as u16);
            }
            w.step([held, Input::NONE]);
            for e in &w.events {
                if let Event::Cut { seat, by, part, spilled: true, .. } = e {
                    if seat == by {
                        *tally.entry(*part).or_insert(0) += 1;
                        break 'run;
                    }
                }
            }
        }
    }
    let names: Vec<String> = w_names();
    println!("first self-cut by part: {}", tally.iter().map(|(k, v)| format!("{} {v}", names[*k as usize])).collect::<Vec<_>>().join(", "));
    ticks.sort();
    println!("random input, 100 runs of 600 ticks: {runs_with} cut themselves; median first self-cut at tick {:?}", ticks.get(ticks.len() / 2));
}

fn w_names() -> Vec<String> {
    let v: serde_json::Value = serde_json::from_str(content::body::BODY_JSON).unwrap();
    v["fighter"]["parts"].as_array().unwrap().iter().map(|p| p["id"].as_str().unwrap().to_string()).collect()
}

/// The roles a tree pilot holds through one match, as runs of ticks:
/// `roles <pilot> <opponent> <seed> [ticks]`, the tree in seat 0. Prints one
/// line per run: first tick, last tick, the move, and whether the run ended
/// before its move finished (cut short by an interrupt).
fn roles(args: &[String]) {
    let id = &args[0];
    let pilot::Spec::Tree { reaction_ticks, rules, salt, .. } = content::road::pilot(id) else { panic!("{id} is not a tree pilot") };
    let seed: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(0);
    let limit: u32 = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(1200);
    let mut tree = pilot::Tree::new(rules, reaction_ticks, salt);
    let mut other = pilot::build(&content::road::pilot(&args[1]));
    let mut w = World::new(content::setup::versus(seed, sim::balance::DEFAULT_TUNING));
    let mut last = [Input::NONE; sim::body::SEATS];
    let mut runs: Vec<(u32, u32, String, u32)> = Vec::new();
    while w.tick < limit && !matches!(w.phase, sim::fight::Phase::MatchOver { .. }) {
        use pilot::Pilot;
        tree.observe(last);
        other.observe(last);
        let i = [tree.input(&w, 0), other.input(&w, 1)];
        let fighting = matches!(w.phase, sim::fight::Phase::Fight);
        let now = if fighting { tree.current_move().map(|(m, t)| (m.to_string(), t)).unwrap_or(("idle".into(), 0)) } else { ("between rounds".into(), 0) };
        match runs.last_mut() {
            Some(r) if r.2 == now.0 && now.1 > r.3 => {
                r.1 = w.tick;
                r.3 = now.1;
            }
            _ => runs.push((w.tick, w.tick, now.0, now.1)),
        }
        w.step(i);
        for e in &w.events {
            if let sim::fight::Event::RoundEnd { result } = e {
                println!("round end at {}: loser {:?} {:?}", w.tick, result.loser, result.cause);
            }
        }
        last = [i[0], i[1], Input::NONE];
    }
    for (a, b, m, t) in runs {
        println!("{a} {b} {m} {t}");
    }
}

/// Two pilots head to head at one keyboard's setup, both with the sword:
/// `versus <matches> <a> <b>`, each a pilot id or a pilot spec as JSON. For
/// a strategist against a baseline, Homework 6 style. Seeds 0.., the first
/// pilot in seat 0; prints wins, losses and unfinished matches.
fn versus(args: &[String]) {
    let matches: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(200);
    let spec = |s: &str| -> pilot::Spec {
        if s.trim_start().starts_with('{') { serde_json::from_str(s).expect("a pilot spec") } else { content::road::pilot(s) }
    };
    let (a, b) = (spec(&args[1]), spec(&args[2]));
    let threads = 8u64;
    let rows: Vec<(u32, u32, u32)> = std::thread::scope(|scope| {
        let hs: Vec<_> = (0..threads)
            .map(|k| {
                let (a, b) = (a.clone(), b.clone());
                scope.spawn(move || {
                    let (mut w, mut l, mut u) = (0, 0, 0);
                    for seed in (k..matches).step_by(threads as usize) {
                        let mut ps: [Box<dyn pilot::Pilot>; 2] = [pilot::build(&a), pilot::build(&b)];
                        let o = pilot::duel(content::setup::versus(seed, sim::balance::DEFAULT_TUNING), &mut ps, 60 * 120);
                        if !o.finished {
                            u += 1;
                        } else if o.wins[0] > o.wins[1] {
                            w += 1;
                        } else {
                            l += 1;
                        }
                    }
                    (w, l, u)
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let (w, l, u) = rows.iter().fold((0, 0, 0), |t, r| (t.0 + r.0, t.1 + r.1, t.2 + r.2));
    println!("{} vs {}: {w} won, {l} lost, {u} unfinished of {matches}", args[1], args[2]);
}

/// The wildest round endings among stream-like matches, for short clips
/// (Sam, 2026-10-07: "a shorts version of some of the craziest stuff that is
/// happening on stream"). Each match is two road opponents, as the stream's
/// exhibitions are, from a seed; each round's end is scored by what it does
/// to the loser: pieces cut off by the end, more cut while the round plays
/// out for 2.5 s (the stream's run-on), a cut across the head, a thrown
/// blade. Prints the best as JSON lines: left, right, seed, round, the tick
/// of the deciding cut, and why.
fn highlights(args: &[String]) {
    use sim::fight::{Event, Phase};
    let matches: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(400);
    let top: usize = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(12);
    // Where the seeds start, for fresh footage past an earlier scan.
    let first: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(0);
    let ids: Vec<String> = content::road::road().into_iter().filter(|s| !s.secret && s.companion.is_none() && !s.four_arms && !s.eight_arms).map(|s| s.id).collect();
    let threads = 8u64;
    let mut rows: Vec<(i64, String)> = std::thread::scope(|scope| {
        let hs: Vec<_> = (0..threads)
            .map(|k| {
                let ids = ids.clone();
                scope.spawn(move || {
                    let mut out = Vec::new();
                    for seed in (first + k..first + matches).step_by(threads as usize) {
                        let n = ids.len() as u64;
                        let (l, r) = (&ids[(seed * 7919 % n) as usize], &ids[((seed * 104729 + 13) % n) as usize]);
                        if l == r {
                            continue;
                        }
                        let mut w = sim::World::new(content::setup::exhibition(seed, sim::balance::DEFAULT_TUNING, [l, r], "flat"));
                        let mut pilots: Vec<Box<dyn pilot::Pilot>> = [l, r].iter().map(|id| pilot::build(&content::road::pilot(id))).collect();
                        let mut last = [Input::NONE; sim::body::SEATS];
                        let mut round = 1u32;
                        // Within the round under way: when it began, blades
                        // meeting, and the cuts each fighter took.
                        let (mut began, mut clashes, mut cut) = (0u32, 0u32, [0u32; 2]);
                        while w.tick < 60 * 150 && !matches!(w.phase, Phase::MatchOver { .. }) {
                            let mut i = [Input::NONE; sim::body::SEATS];
                            for (k, p) in pilots.iter_mut().enumerate() {
                                p.observe(last);
                                i[k] = p.input(&w, k);
                            }
                            let was_fight = matches!(w.phase, Phase::Fight);
                            w.step_all(i);
                            last = i;
                            if !was_fight && matches!(w.phase, Phase::Fight) {
                                (began, clashes, cut) = (w.tick, 0, [0; 2]);
                            }
                            for e in &w.events {
                                match e {
                                    Event::Clash { .. } => clashes += 1,
                                    Event::Cut { seat, spilled: true, .. } if (*seat as usize) < 2 => cut[*seat as usize] += 1,
                                    _ => {}
                                }
                            }
                            if !was_fight || matches!(w.phase, Phase::Fight) {
                                continue;
                            }
                            let (Phase::RoundOver { result, .. } | Phase::MatchOver { result }) = w.phase else { continue };
                            let Some(_) = result.loser else { round += 1; continue };
                            let loser = result.seat;
                            let severed = w.parts.iter().filter(|p| p.fighter == loser && !p.attached).count() as i64;
                            // The follow-through: the round played on, limp.
                            let mut on = w.clone();
                            let mut after = 0i64;
                            for _ in 0..150 {
                                on.step_all([Input::NONE; sim::body::SEATS]);
                                after += on.events.iter().filter(|e| matches!(e, Event::Cut { seat, spilled: true, .. } if *seat == loser)).count() as i64;
                            }
                            let head = content::messages::headshot(&w, &result);
                            let score = severed * 3 + after * 2 + if head { 6 } else { 0 } + if result.thrown { 5 } else { 0 };
                            let killer = if result.by == 0 { l } else { r };
                            let nodes = pilot::view::walk(&pilot::view::describe(&content::road::pilot(killer))).len();
                            out.push((score, serde_json::json!({ "left": l, "right": r, "seed": seed, "round": round, "kill": w.tick, "by": result.by, "killer_nodes": nodes, "killer_kind": content::road::pilot(killer).kind(),
                                "began": began, "clashes": clashes, "cuts": cut,
                                "severed": severed, "after": after, "headshot": head, "thrown": result.thrown, "score": score }).to_string()));
                            round += 1;
                        }
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, line) in rows.into_iter().take(top) {
        println!("{line}");
    }
}

/// The player's run through arcade mode (content::agent), as the video will
/// play it: each fight's seed is the run's base plus the fights played
/// before it, so the page plays the same matches. Prints each fight and the
/// run's length.
fn agent_run() {
    use content::road::{Best, Feats};
    let base: u64 = std::env::args().nth(2).and_then(|a| a.parse().ok()).unwrap_or(7);
    // The tuning the page plays at: with no ?tuning= in the address, the
    // page's tuning() reads Number(null), which is 0 (not DEFAULT_TUNING;
    // reported to Sam, 2026-10-07).
    let tuning: u8 = std::env::args().nth(3).and_then(|a| a.parse().ok()).unwrap_or(0);
    let mut best = std::collections::BTreeMap::new();
    let (mut played, mut ticks) = (0u32, 0u64);
    println!("targets: {:?}", content::agent::targets());
    while let Some(stop) = content::agent::next(&best) {
        let seed = base + played as u64;
        let mut rec = sim::replay::Recording::new(content::setup::road_with(seed, tuning, &stop, "sword", false));
        let mut pilots: Vec<Box<dyn pilot::Pilot>> = vec![pilot::build(&content::agent::player(played))];
        pilots.extend(content::road::crew(&stop).iter().map(pilot::build));
        let mut last = [Input::NONE; sim::body::SEATS];
        let mut feats = Feats::default();
        for _ in 0..60 * 60 * 6 {
            let mut i = [Input::NONE; sim::body::SEATS];
            for (k, p) in pilots.iter_mut().enumerate() {
                p.observe(last);
                i[k] = p.input(&rec.world, k);
            }
            // Between rounds the player presses ready, as the page's Enter does.
            if !matches!(rec.world.phase, sim::fight::Phase::Fight) {
                i[0] = Input(i[0].0 | sim::Input::READY);
            }
            rec.step_all(i);
            feats.observe(&rec.world);
            last = i;
            if matches!(rec.world.phase, sim::fight::Phase::MatchOver { .. }) {
                break;
            }
        }
        let w = &rec.world;
        let won = matches!(w.phase, sim::fight::Phase::MatchOver { .. }) && w.wins[0] > w.wins[1];
        println!("{:>2} {:<14} level {:>2} {} {}-{} in {:>5} ticks", played + 1, stop, played, if won { "won " } else { "lost" }, w.wins[0], w.wins[1], w.tick);
        if won {
            content::road::record(&mut best, &stop, Best::won(w.wins[1], w.tick, "sword").with_feats(feats));
        }
        ticks += w.tick as u64;
        played += 1;
        if played > 80 {
            println!("stopped after 80 fights");
            break;
        }
    }
    println!("{played} fights, {:.1} minutes of play", ticks as f64 / 60.0 / 60.0);
}

/// A replay whose first round ends with a cut across the head, so the gate
/// can see the clock held for a headshot and then run on (Sam: "if someone
/// gets headshot, stop the simulation to focus on it"). Written to
/// testing/replays/headshot.replay: the first seed whose flailing does it,
/// played on four seconds after it, with no other round ending.
fn fixture_headshot() {
    use sim::fight::{Event, Phase};
    for seed in 0..2000u64 {
        let mut rec = sim::replay::Recording::new(content::setup::versus(seed, sim::balance::DEFAULT_TUNING));
        let mut r = sim::rng::Rng::new(seed ^ 0xBEEF);
        let mut held = [Input::NONE; 2];
        let mut first: Option<(u32, String)> = None;
        let mut ends = 0;
        for t in 0..2400u32 {
            if t % 12 == 0 {
                let toward = [Input::STEP_RIGHT, Input::STEP_LEFT];
                held = [0, 1].map(|s| Input((r.below(16) as u16) | if r.below(3) > 0 { toward[s] } else { 0 }));
            }
            let ready = matches!(rec.world.phase, Phase::RoundOver { .. });
            rec.step(if ready { [Input(Input::READY); 2] } else { held });
            for e in &rec.world.events {
                if let Event::RoundEnd { result } = e {
                    ends += 1;
                    if first.is_none() {
                        let part = result.loser.map(|l| {
                            let body = rec.world.setup.seats[l as usize].unwrap().body as usize;
                            rec.world.setup.bodies[body].parts[result.part as usize].copy.clone()
                        });
                        first = Some((rec.world.tick, part.unwrap_or_default()));
                    }
                }
            }
            if let Some((at, part)) = &first {
                // One round end only, so the replay's one headshot is the
                // one the gate sees (CI's Firefox once caught a second, on
                // the replay's last ticks, where the clock stops anyway).
                if part != "head" || ends > 1 {
                    break;
                }
                if rec.world.tick >= at + 60 * 4 && matches!(rec.world.phase, Phase::Fight) {
                    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testing/replays/headshot.replay");
                    std::fs::write(path, rec.bytes()).unwrap();
                    println!("seed {seed}: a headshot at tick {at}; written to {} ticks", rec.world.tick);
                    return;
                }
            }
        }
    }
    panic!("no seed's first round ended with a cut across the head");
}

/// The gate's scripted match: both seats flail toward each other and press
/// ready between rounds, on the seed that reaches the end of a match soonest.
/// Written to testing/replays/match.replay, with what it should say.
fn fixture_match() {
    use sim::fight::Phase;
    let play = |seed: u64| {
        let mut rec = sim::replay::Recording::new(content::setup::versus(seed, sim::balance::DEFAULT_TUNING));
        let mut r = sim::rng::Rng::new(seed ^ 0xABCD);
        let mut held = [Input::NONE; 2];
        for t in 0..2400u32 {
            if t % 12 == 0 {
                let toward = [Input::STEP_RIGHT, Input::STEP_LEFT];
                held = [0, 1].map(|s| Input((r.below(16) as u16) | if r.below(3) > 0 { toward[s] } else { 0 }));
            }
            let ready = matches!(rec.world.phase, Phase::RoundOver { .. });
            let i = if ready { [Input(Input::READY); 2] } else { held };
            rec.step(i);
            if matches!(rec.world.phase, Phase::MatchOver { .. }) {
                return Some(rec);
            }
        }
        None
    };
    let mut best: Option<(u64, sim::replay::Recording)> = None;
    for seed in 0..200u64 {
        if let Some(rec) = play(seed) {
            if best.as_ref().is_none_or(|(_, b)| rec.world.tick < b.world.tick) {
                best = Some((seed, rec));
            }
        }
    }
    let (seed, rec) = best.expect("no seed ended a match in 2400 ticks");
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testing/replays/match.replay");
    std::fs::write(path, rec.bytes()).unwrap();
    let said = content::messages::phase_text(&rec.world, content::messages::Audience::Versus);
    println!("seed {seed}: the match ended at tick {} with wins {:?}, checksum {:016x}", rec.world.tick, rec.world.wins, rec.world.checksum());
    println!("{said}");
}

/// M5.0: headless ticks per second, and what the search pilot costs a tick.
fn recon_m5() {
    let mut w = World::new(content::setup::versus(1, sim::balance::DEFAULT_TUNING));
    let mut r = sim::rng::Rng::new(3);
    let t0 = std::time::Instant::now();
    let n = 30_000u32;
    for _ in 0..n {
        w.step([Input(r.below(64) as u16), Input(r.below(64) as u16)]);
    }
    let per_s = n as f64 / t0.elapsed().as_secs_f64();
    println!("headless: {per_s:.0} ticks a second, two fighters, random input (native release)");
    for id in ["yardstick", "reader"] {
        let spec = content::road::pilot(id);
        let mut p = pilot::build(&spec);
        let mut w = World::new(content::setup::road(1, sim::balance::DEFAULT_TUNING, "scarecrow"));
        let t0 = std::time::Instant::now();
        let n = 1_200u32;
        for _ in 0..n {
            let a = p.input(&w, 0);
            w.step([a, Input::NONE]);
        }
        let us = t0.elapsed().as_secs_f64() * 1e6 / n as f64;
        println!("{id}: {us:.0} µs a tick, its own step included (native release; a tick has 16,667 µs)");
    }
}

/// Play the yardstick against every stop on the road and write
/// analysis/ladder.md. `lab ladder [matches]`.
fn ladder(args: &[String]) {
    let matches: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(200);
    let stops = content::road::stops();
    let max_ticks = 60 * 120;
    let t0 = std::time::Instant::now();
    let rows: Vec<(String, u32, u32, u64)> = std::thread::scope(|scope| {
        let handles: Vec<_> = stops
            .iter()
            .map(|id| {
                let id = id.clone();
                scope.spawn(move || {
                    let (mut won, mut unfinished, mut ticks) = (0u32, 0u32, 0u64);
                    for seed in 0..matches {
                        let mut pilots = content::road::lineup(&content::road::pilot("yardstick"), &id);
                        let o = pilot::duel(content::setup::road(seed, sim::balance::DEFAULT_TUNING, &id), &mut pilots, max_ticks);
                        ticks += o.ticks as u64;
                        if !o.finished {
                            unfinished += 1;
                        } else if o.wins[0] > o.wins[1] {
                            won += 1;
                        }
                    }
                    (id, won, unfinished, ticks)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut md = format!(
        "# Ladder\n\nThe yardstick pilot against every fight on the road, {matches} seeded matches each, written by `make ladder` \
         ({:.0} s). A match unfinished after {} ticks is not a win. Each fight is played with its condition. Along every \
         requirement in the tree, the yardstick should not win much more often below than above \
         (`the_tree_gets_no_easier_going_down`).\n",
        t0.elapsed().as_secs_f64(), max_ticks
    );
    md += &format!("\nfingerprint {} (data/pilots.json, the set of stops with their conditions, SIM_VERSION {})\n\n", content::road::ladder_fingerprint(), sim::SIM_VERSION);
    md += "| stop | yardstick wins | unfinished | mean ticks |\n|---|---|---|---|\n";
    for (id, won, unf, ticks) in &rows {
        md += &format!("| {id} | {won} of {matches} ({:.0} %) | {unf} | {} |\n", 100.0 * *won as f64 / matches as f64, ticks / matches);
    }
    print!("{md}");
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../analysis/ladder.md");
    std::fs::write(path, md).unwrap();
    // A finished ladder ends the wait `ladder-pending` began.
    let _ = std::fs::remove_file(concat!(env!("CARGO_MANIFEST_DIR"), "/../../analysis/ladder.pending"));
}

/// Every pilot as a behavior tree (`pilot::view::describe`), as JSON for
/// analysis/trees/render.py, which draws the figures and the icons.
fn trees() {
    print!("{}", content::trees::json());
}

/// Mark the ladder as pending for the current data and simulation, so the
/// checks that read it wait for it (`make ladder-pending`).
fn ladder_pending() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../analysis/ladder.pending");
    std::fs::write(path, format!("{}\n", content::road::ladder_fingerprint())).unwrap();
    println!("analysis/ladder.pending: {}", content::road::ladder_fingerprint());
}

/// The yardstick's win rate against a few stops, for tuning a pilot: `rate
/// <matches> <stop>...`. Prints only; the ladder is what the tests read.
fn rate(args: &[String]) {
    let matches: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(40);
    let max_ticks = 60 * 120;
    let rows: Vec<(String, u32, u32)> = std::thread::scope(|scope| {
        let handles: Vec<_> = args[1..]
            .iter()
            .map(|id| {
                let id = id.clone();
                scope.spawn(move || {
                    let (mut won, mut unfinished) = (0u32, 0u32);
                    for seed in 0..matches {
                        let mut pilots = content::road::lineup(&content::road::pilot("yardstick"), &id);
                        let o = pilot::duel(content::setup::road(seed, sim::balance::DEFAULT_TUNING, &id), &mut pilots, max_ticks);
                        if !o.finished {
                            unfinished += 1;
                        } else if o.wins[0] > o.wins[1] {
                            won += 1;
                        }
                    }
                    (id, won, unfinished)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for (id, won, unf) in rows {
        println!("{id}: {won} of {matches} ({:.0} %), {unf} unfinished", 100.0 * won as f64 / matches as f64);
    }
}

/// The yardstick carrying each weapon against a panel of opponents, written
/// to analysis/weapons.md (Sam: "the normal sword is the most powerful /
/// balanced sword"; `the_sword_is_the_strongest_weapon` reads it).
fn weapons(args: &[String]) {
    let matches: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(50);
    let max_ticks = 60 * 120;
    let t0 = std::time::Instant::now();
    let panel = content::weapons::PANEL;
    // The weapons a player can carry; the enemies' own are not the player's
    // to compare.
    let ids: Vec<String> = content::weapons::weapons().into_iter().filter(|w| !w.enemy_only).map(|w| w.id).collect();
    let jobs: Vec<(String, &str)> = ids.iter().flat_map(|w| panel.iter().map(move |o| (w.clone(), *o))).collect();
    let rows: Vec<(String, &str, u32)> = std::thread::scope(|scope| {
        let handles: Vec<_> = jobs
            .into_iter()
            .map(|(w, o)| {
                scope.spawn(move || {
                    let mut won = 0u32;
                    for seed in 0..matches {
                        let mut pilots = content::road::lineup(&content::road::pilot("yardstick"), o);
                        let r = pilot::duel(content::setup::road_with(seed, sim::balance::DEFAULT_TUNING, o, &w, false), &mut pilots, max_ticks);
                        if r.finished && r.wins[0] > r.wins[1] {
                            won += 1;
                        }
                    }
                    (w, o, won)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let total = matches * panel.len() as u64;
    let mut md = format!(
        "# Weapons\n\nThe yardstick carrying each weapon against {} opponents, {matches} seeded matches each, written by `make weapons` \
         ({:.0} s). The sword should win most (`the_sword_is_the_strongest_weapon`).\n\nfingerprint {} (data/weapons.json, data/pilots.json, the panel, SIM_VERSION {})\n\n",
        panel.len(), t0.elapsed().as_secs_f64(), content::weapons::fingerprint(), sim::SIM_VERSION
    );
    md += &format!("| weapon | wins of {total} | {} |\n|---|---|{}\n", panel.join(" | "), "---|".repeat(panel.len()));
    for id in &ids {
        let per: Vec<u32> = panel.iter().map(|o| rows.iter().find(|(w, p, _)| w == id && p == o).unwrap().2).collect();
        let sum: u32 = per.iter().sum();
        md += &format!("| {id} | {sum} ({:.0} %) | {} |\n", 100.0 * sum as f64 / total as f64, per.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" | "));
    }
    print!("{md}");
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../analysis/weapons.md");
    std::fs::write(path, md).unwrap();
}

/// One road match between the yardstick (seat 0) and a stop, told as events.
fn duel_trace(args: &[String]) {
    use sim::fight::Event;
    let id = args.first().map(String::as_str).unwrap_or("scarecrow");
    let seed: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(0);
    let names = w_names();
    let mut w = World::new(content::setup::road(seed, sim::balance::DEFAULT_TUNING, id));
    let mut ps: [Box<dyn pilot::Pilot>; 2] = [pilot::build(&content::road::pilot("yardstick")), pilot::build(&content::road::pilot(id))];
    let mut last = [Input::NONE; sim::body::SEATS];
    while w.tick < 4000 && !matches!(w.phase, sim::fight::Phase::MatchOver { .. }) {
        ps[0].observe(last);
        ps[1].observe(last);
        let i = [ps[0].input(&w, 0), ps[1].input(&w, 1)];
        w.step(i);
        last = [i[0], i[1], Input::NONE];
        for e in &w.events {
            match e {
                Event::Cut { seat, part, by, spilled, .. } => println!("{:>5}: seat {by}'s blade cut seat {seat}'s {}{}", w.tick, names[*part as usize], if *spilled { "" } else { " (dropped piece)" }),
                Event::RoundEnd { result } => println!("{:>5}: round over: {:?}; gap {} cm", w.tick, result, pilot::gap(&w, 0)),
                _ => {}
            }
        }
        if w.tick % 120 == 0 {
            println!("{:>5}: gap {} cm, ink {:?}", w.tick, pilot::gap(&w, 0), w.fighters.iter().map(|f| f.as_ref().map(|f| f.ink)).collect::<Vec<_>>());
        }
    }
}

/// Record one match for showing: `film <a> <b> <seeds> <out.replay> [map]`
/// plays opponent `a` against `b` (or, with `a` = `road`, the yardstick at
/// stop `b`, companion and ground included) on each seed, and keeps the
/// longest finished match with the most changes of lead.
fn film(args: &[String]) {
    let (a, b) = (args[0].as_str(), args[1].as_str());
    let seeds: u64 = args[2].parse().unwrap();
    let out = &args[3];
    let map = args.get(4).map(String::as_str).unwrap_or(content::maps::FLAT);
    let best = std::thread::scope(|scope| {
        let hs: Vec<_> = (0..seeds)
            .map(|seed| {
                scope.spawn(move || {
                    let (setup, mut ps) = if a == "road" {
                        (content::setup::road(seed, sim::balance::DEFAULT_TUNING, b), content::road::lineup(&content::road::pilot("yardstick"), b))
                    } else {
                        let ps: Vec<Box<dyn pilot::Pilot>> = vec![pilot::build(&content::road::pilot(a)), pilot::build(&content::road::pilot(b))];
                        (content::setup::exhibition(seed, sim::balance::DEFAULT_TUNING, [a, b], map), ps)
                    };
                    let mut rec = sim::replay::Recording::new(setup);
                    let mut last = [Input::NONE; sim::body::SEATS];
                    let (mut changes, mut lead) = (0u32, 0i32);
                    while rec.world.tick < 60 * 120 && !matches!(rec.world.phase, sim::fight::Phase::MatchOver { .. }) {
                        let mut i = [Input::NONE; sim::body::SEATS];
                        for (k, p) in ps.iter_mut().enumerate() {
                            p.observe(last);
                            i[k] = p.input(&rec.world, k);
                        }
                        rec.step_all(i);
                        last = i;
                        let now = (rec.world.wins[0] as i32 - rec.world.wins[1] as i32).signum();
                        if now != 0 && now != lead {
                            changes += 1;
                            lead = now;
                        }
                    }
                    let done = matches!(rec.world.phase, sim::fight::Phase::MatchOver { .. });
                    (done, changes, rec.world.wins[0].min(rec.world.wins[1]), rec.world.tick, seed, rec)
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).filter(|r| r.0).max_by_key(|r| (r.1, r.2, std::cmp::Reverse(r.3))).expect("a finished match")
    });
    let (_, changes, close, ticks, seed, rec) = best;
    std::fs::write(out, rec.bytes()).unwrap();
    println!("{a} vs {b}: seed {seed}, wins {:?}, {changes} changes of lead, loser's rounds {close}, {ticks} ticks -> {out}", rec.world.wins);
}

/// Costumes and armor, measured (Sam, 2026-10-08: "place a large emphasis on
/// testing for whether the changes cause the game to lag, or the gameplay to
/// change meaningfully moment to moment, and whether the new armor is
/// actually any fun at all"). For each opponent that wears a plate, and for
/// a few that wear only a costume: the yardstick against it on the road, the
/// same seeds dressed and undressed. Writes analysis/armor.md.
fn armor(args: &[String]) {
    use sim::body::Cause;
    use sim::fight::{Event, Phase};
    let matches: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(100);
    let max_ticks = 60 * 120;
    let f = content::costumes::file();
    // The opponents named after the count, or every armored one and four
    // in costume only.
    let mut ids: Vec<String> = args[1.min(args.len())..].to_vec();
    if ids.is_empty() {
        ids = f.costumes.iter().filter(|(_, c)| !c.armor.is_empty()).map(|(id, _)| id.clone()).collect();
        ids.extend(["herbalist", "abbot", "scarecrow", "pilgrim"].iter().map(|s| s.to_string()));
    }
    #[derive(Default, Clone)]
    struct Tally {
        won: u32,
        unfinished: u32,
        ticks: u64,
        rounds: u32,
        neck: u32,
        heart: u32,
        ink: u32,
        plate_hits: u32,
        clashes: u32,
        cuts_taken: u32,
        nanos: u128,
        trace: Vec<(u32, u32)>,
    }
    let run = |id: &str, dressed: bool| -> Tally {
        let mut t = Tally::default();
        for seed in 0..matches {
            let mut s = content::setup::road(seed, sim::balance::DEFAULT_TUNING, id);
            if !dressed {
                for seat in 1..sim::body::SEATS {
                    if let Some(b) = s.seats[seat].map(|x| x.body as usize) {
                        s.bodies[b].armor.clear();
                        s.bodies[b].costume.clear();
                    }
                }
            }
            let mut w = sim::World::new(s);
            let mut pilots = content::road::lineup(&content::road::pilot("yardstick"), id);
            let mut last = [Input::NONE; sim::body::SEATS];
            let plates: Vec<u8> = (0..w.swords.len()).filter(|&k| w.swords[k].armor.is_some()).map(|k| k as u8).collect();
            let t0 = std::time::Instant::now();
            while w.tick < max_ticks && !matches!(w.phase, Phase::MatchOver { .. }) {
                let mut i = [Input::NONE; sim::body::SEATS];
                for (k, p) in pilots.iter_mut().enumerate() {
                    p.observe(last);
                    i[k] = p.input(&w, k);
                }
                let before = w.clashing.clone();
                let was_fight = matches!(w.phase, Phase::Fight);
                w.step_all(i);
                last = i;
                for pair in &w.clashing {
                    if !before.contains(pair) && (plates.contains(&pair.0) || plates.contains(&pair.1)) {
                        t.plate_hits += 1;
                    }
                }
                for e in &w.events {
                    match e {
                        Event::Clash { .. } => t.clashes += 1,
                        Event::Cut { seat, spilled: true, .. } if *seat != 0 => t.cuts_taken += 1,
                        _ => {}
                    }
                }
                if was_fight && !matches!(w.phase, Phase::Fight) {
                    if let Phase::RoundOver { result, .. } | Phase::MatchOver { result } = w.phase {
                        if result.loser.is_some() {
                            t.rounds += 1;
                            if result.loser == Some(1) {
                                match result.cause {
                                    Cause::Neck => t.neck += 1,
                                    Cause::Heart => t.heart += 1,
                                    _ => t.ink += 1,
                                }
                            }
                        }
                    }
                }
            }
            t.nanos += t0.elapsed().as_nanos();
            t.ticks += w.tick as u64;
            t.trace.push((w.tick, w.wins[0] * 10 + w.wins[1]));
            if !matches!(w.phase, Phase::MatchOver { .. }) {
                t.unfinished += 1;
            } else if w.wins[0] > w.wins[1] {
                t.won += 1;
            }
        }
        t
    };
    let rows: Vec<(String, Tally, Tally)> = std::thread::scope(|scope| {
        let hs: Vec<_> = ids.iter().map(|id| { let id = id.clone(); scope.spawn(move || { let on = run(&id, true); let off = run(&id, false); (id, on, off) }) }).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let pct = |a: u32, b: u32| if b == 0 { 0.0 } else { 100.0 * a as f64 / b as f64 };
    let mut md = format!("# Armor\n\nWritten by `lab armor {matches}` (SIM_VERSION {}). The yardstick against each opponent on the road, the same {matches} seeds with the opponent dressed (data/costumes.json as it stands) and undressed (no plate, no costume). A plate hit is a blade first meeting a plate. Step time is the simulation alone plus both pilots, per tick, on this machine, threads running side by side.\n\n", sim::SIM_VERSION);
    md += "| opponent | plates | yardstick wins (dressed / bare) | mean ticks a match | rounds lost to the head or neck | plate hits a round | cuts it took a round | µs a tick (dressed / bare) | same matches |\n|---|---|---|---|---|---|---|---|---|\n";
    for (id, on, off) in &rows {
        let plates = f.costumes[id].armor.join(", ");
        let us = |t: &Tally| t.nanos as f64 / 1000.0 / t.ticks.max(1) as f64;
        let lost = |t: &Tally| t.neck + t.heart + t.ink;
        let same = on.trace.iter().zip(&off.trace).filter(|(a, b)| a == b).count();
        md += &format!(
            "| {id} | {} | {} ({:.0} %) / {} ({:.0} %) | {} / {} | {:.0} % / {:.0} % | {:.2} | {:.2} / {:.2} | {:.1} / {:.1} | {same} of {matches} |\n",
            if plates.is_empty() { "none".to_string() } else { plates },
            on.won, pct(on.won, matches as u32), off.won, pct(off.won, matches as u32),
            on.ticks / matches, off.ticks / matches,
            pct(on.neck, lost(on)), pct(off.neck, lost(off)),
            on.plate_hits as f64 / on.rounds.max(1) as f64,
            on.cuts_taken as f64 / on.rounds.max(1) as f64, off.cuts_taken as f64 / off.rounds.max(1) as f64,
            us(on), us(off),
        );
    }
    print!("{md}");
    if args.len() <= 1 {
        std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/../../analysis/armor.md"), md).unwrap();
    }
}

/// The checksum each of `n` exhibitions ends on, for showing that a change
/// to the simulation that should change nothing changes nothing: armored
/// opponents against each other and against the rest.
fn checksums(args: &[String]) {
    let n: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(24);
    let ids = ["gatekeeper", "watchman", "general", "bridge_keeper", "ox_herd", "duelist", "herbalist", "smith"];
    for seed in 0..n {
        let (l, r) = (ids[(seed % 8) as usize], ids[((seed * 3 + 1) % 8) as usize]);
        let mut w = sim::World::new(content::setup::exhibition(seed, sim::balance::DEFAULT_TUNING, [l, r], "flat"));
        let mut pilots: Vec<Box<dyn pilot::Pilot>> = [l, r].iter().map(|id| pilot::build(&content::road::pilot(id))).collect();
        let mut last = [Input::NONE; sim::body::SEATS];
        while w.tick < 60 * 40 {
            let mut i = [Input::NONE; sim::body::SEATS];
            for (k, p) in pilots.iter_mut().enumerate() {
                p.observe(last);
                i[k] = p.input(&w, k);
            }
            w.step_all(i);
            last = i;
        }
        println!("{seed} {l} {r} {:016x}", w.checksum());
    }
}

/// What a tick of the simulation alone costs, pilots left out, with the
/// opponents dressed and bare: the lag the page would feel (Sam,
/// 2026-10-08). The inputs are recorded from the dressed run's pilots and
/// replayed into both.
fn step_cost(args: &[String]) {
    let n: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(20);
    let pairs = [["general", "bridge_keeper"], ["watchman", "gatekeeper"], ["ox_herd", "temple_guard"], ["warden", "general"]];
    let (mut on, mut off, mut ticks) = (0u128, 0u128, 0u64);
    for seed in 0..n {
        let [l, r] = pairs[(seed % 4) as usize];
        let setup = content::setup::exhibition(seed, sim::balance::DEFAULT_TUNING, [l, r], "flat");
        let mut w = sim::World::new(setup.clone());
        let mut pilots: Vec<Box<dyn pilot::Pilot>> = [l, r].iter().map(|id| pilot::build(&content::road::pilot(id))).collect();
        let mut last = [Input::NONE; sim::body::SEATS];
        let mut inputs = Vec::new();
        while w.tick < 60 * 60 {
            let mut i = [Input::NONE; sim::body::SEATS];
            for (k, p) in pilots.iter_mut().enumerate() {
                p.observe(last);
                i[k] = p.input(&w, k);
            }
            w.step_all(i);
            last = i;
            inputs.push(i);
        }
        let mut bare = setup.clone();
        for b in bare.bodies.iter_mut() {
            b.armor.clear();
        }
        for (s, out) in [(setup, &mut on), (bare, &mut off)] {
            let mut w = sim::World::new(s);
            let t0 = std::time::Instant::now();
            for i in &inputs {
                w.step_all(*i);
            }
            *out += t0.elapsed().as_nanos();
        }
        ticks += inputs.len() as u64;
    }
    let us = |x: u128| x as f64 / 1000.0 / ticks as f64;
    println!("{ticks} ticks of armored pairs: {:.1} us a tick dressed, {:.1} us bare ({:+.0} %); a frame at 60 a second has 16667 us", us(on), us(off), 100.0 * (us(on) / us(off) - 1.0));
}

/// Where a fight's time goes, tick by tick: the simulation and each pilot,
/// dressed or bare. `profile <left> <right> <seed> [bare]`.
fn profile(args: &[String]) {
    let (l, r) = (args[0].as_str(), args[1].as_str());
    let seed: u64 = args[2].parse().unwrap();
    let mut s = content::setup::exhibition(seed, sim::balance::DEFAULT_TUNING, [l, r], "flat");
    if args.get(3).is_some() {
        for b in s.bodies.iter_mut() {
            b.armor.clear();
        }
    }
    let mut w = sim::World::new(s);
    let mut pilots: Vec<Box<dyn pilot::Pilot>> = [l, r].iter().map(|id| pilot::build(&content::road::pilot(id))).collect();
    let mut last = [Input::NONE; sim::body::SEATS];
    let mut t = [0u128; 3];
    while w.tick < 240 {
        let mut i = [Input::NONE; sim::body::SEATS];
        for (k, p) in pilots.iter_mut().enumerate() {
            let t0 = std::time::Instant::now();
            p.observe(last);
            i[k] = p.input(&w, k);
            t[1 + k] += t0.elapsed().as_nanos();
        }
        let t0 = std::time::Instant::now();
        w.step_all(i);
        t[0] += t0.elapsed().as_nanos();
        last = i;
    }
    println!("{l} v {r} seed {seed}: us a tick: sim {:.0}, {l} {:.0}, {r} {:.0}", t[0] as f64 / 240e3, t[1] as f64 / 240e3, t[2] as f64 / 240e3);
}

/// Every round of the exhibition `left` against `right` over `seeds` seeds,
/// as JSON lines: seed, the rounds' winners (0 left, 1 right), and each
/// round's start and deciding tick. For picking one whole match for a short.
fn rounds(args: &[String]) {
    use sim::fight::Phase;
    let (l, r) = (args[0].as_str(), args[1].as_str());
    let seeds: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(200);
    for seed in 0..seeds {
        let mut w = sim::World::new(content::setup::exhibition(seed, sim::balance::DEFAULT_TUNING, [l, r], "flat"));
        let mut pilots: Vec<Box<dyn pilot::Pilot>> = [l, r].iter().map(|id| pilot::build(&content::road::pilot(id))).collect();
        let mut last = [Input::NONE; sim::body::SEATS];
        let (mut out, mut began, mut clashes, mut cuts, mut throws) = (Vec::new(), 0u32, 0u32, 0u32, 0u32);
        while w.tick < 60 * 150 && !matches!(w.phase, Phase::MatchOver { .. }) {
            let mut i = [Input::NONE; sim::body::SEATS];
            for (k, p) in pilots.iter_mut().enumerate() {
                p.observe(last);
                i[k] = p.input(&w, k);
            }
            let was = matches!(w.phase, Phase::Fight);
            w.step_all(i);
            last = i;
            if !was && matches!(w.phase, Phase::Fight) {
                (began, clashes, cuts, throws) = (w.tick, 0, 0, 0);
            }
            // How busy the round is: blades meeting, cuts, weapons thrown.
            for e in &w.events {
                match e {
                    sim::fight::Event::Clash { .. } => clashes += 1,
                    sim::fight::Event::Cut { spilled: true, .. } => cuts += 1,
                    _ => {}
                }
            }
            throws += w.swords.iter().filter(|s| s.flying).count() as u32;
            if was && !matches!(w.phase, Phase::Fight) {
                if let Phase::RoundOver { result, .. } | Phase::MatchOver { result } = w.phase {
                    if result.loser.is_some() {
                        out.push(serde_json::json!({ "by": result.by, "began": began, "kill": w.tick, "clashes": clashes, "cuts": cuts, "flying_ticks": throws, "severed": w.parts.iter().filter(|p| p.fighter == result.seat && !p.attached).count() }));
                    }
                }
            }
        }
        println!("{}", serde_json::json!({ "left": l, "right": r, "seed": seed, "wins": w.wins, "rounds": out }));
    }
}

/// A shielded duel to one round won (content::setup::duel), over `seeds`
/// seeds, as JSON lines: who won, when, and what the round held. `duels
/// <left pilot> <right pilot> <left weapon> <right weapon> <seeds> [first]`.
/// For picking a clip's fight.
fn duels(args: &[String]) {
    use sim::fight::{Event, Phase};
    let (l, r, lw, rw) = (args[0].as_str(), args[1].as_str(), args[2].as_str(), args[3].as_str());
    let seeds: u64 = args.get(4).and_then(|a| a.parse().ok()).unwrap_or(200);
    let first: u64 = args.get(5).and_then(|a| a.parse().ok()).unwrap_or(0);
    let threads = 8u64;
    let rows: Vec<String> = std::thread::scope(|scope| {
        let hs: Vec<_> = (0..threads).map(|k| scope.spawn(move || {
            let mut out = Vec::new();
            for seed in (first + k..first + seeds).step_by(threads as usize) {
                let mut w = sim::World::new(content::setup::duel(seed, sim::balance::DEFAULT_TUNING, [lw, rw], true, 1));
                let mut pilots: Vec<Box<dyn pilot::Pilot>> = [l, r].iter().map(|id| pilot::build(&content::road::pilot(id))).collect();
                let mut last = [Input::NONE; sim::body::SEATS];
                let (mut clashes, mut stops, mut cuts) = (0u32, [0u32; 2], [0u32; 2]);
                while w.tick < 60 * 60 && matches!(w.phase, Phase::Fight) {
                    let mut i = [Input::NONE; sim::body::SEATS];
                    for (k, p) in pilots.iter_mut().enumerate() {
                        p.observe(last);
                        i[k] = p.input(&w, k);
                    }
                    w.step_all(i);
                    last = i;
                    for e in &w.events {
                        match e {
                            Event::Clash { .. } => clashes += 1,
                            Event::Shield { seat, .. } if (*seat as usize) < 2 => stops[*seat as usize] += 1,
                            Event::Cut { seat, spilled: true, .. } if (*seat as usize) < 2 => cuts[*seat as usize] += 1,
                            _ => {}
                        }
                    }
                }
                let (Phase::RoundOver { result, .. } | Phase::MatchOver { result }) = w.phase else { continue };
                let Some(_) = result.loser else { continue };
                let kill = w.tick;
                let loser = result.seat;
                let mut on = w.clone();
                let mut after = 0;
                for _ in 0..150 {
                    on.step_all([Input::NONE; sim::body::SEATS]);
                    after += on.events.iter().filter(|e| matches!(e, Event::Cut { seat, spilled: true, .. } if *seat == loser)).count();
                }
                let severed = w.parts.iter().filter(|p| p.fighter == loser && !p.attached).count();
                out.push(serde_json::json!({ "seed": seed, "winner": 1 - loser, "by": result.by, "kill": kill, "cause": format!("{:?}", result.cause),
                    "headshot": content::messages::headshot(&w, &result), "clashes": clashes, "stops": stops, "cuts": cuts, "severed": severed, "after": after, "thrown": result.thrown }).to_string());
            }
            out
        })).collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    for r in rows {
        println!("{r}");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("stand") => stand(&args[1..]),
        Some("points") => points(&args[1..]),
        Some("recon-m1") => recon_m1(),
        Some("recon-step") => recon_step(),
        Some("swing") => swing(&args[1..]),
        Some("recon-m2") => recon_m2(),
        Some("trace") => trace(&args[1..]),
        Some("pogo-search") => pogo_search(&args[1..]),
        Some("pogo-grid") => pogo_grid(),
        Some("h5") => h5(),
        Some("energy") => energy(),
        Some("self-cuts") => self_cuts(),
        Some("fixture-match") => fixture_match(),
        Some("recon-m5") => recon_m5(),
        Some("ladder") => ladder(&args[1..]),
        Some("ladder-pending") => ladder_pending(),
        Some("trees") => trees(),
        Some("rate") => rate(&args[1..]),
        Some("armor") => armor(&args[1..]),
        Some("duels") => duels(&args[1..]),
        Some("rounds") => rounds(&args[1..]),
        Some("profile") => profile(&args[1..]),
        Some("step-cost") => step_cost(&args[1..]),
        Some("checksums") => checksums(&args[1..]),
        Some("weapons") => weapons(&args[1..]),
        Some("fixture-headshot") => fixture_headshot(),
        Some("agent-run") => agent_run(),
        Some("highlights") => highlights(&args[1..]),
        Some("versus") => versus(&args[1..]),
        Some("film") => film(&args[1..]),
        Some("roles") => roles(&args[1..]),
        Some("duel") => duel_trace(&args[1..]),
        Some("golden") => golden(),
        Some("test-saves") => test_saves(),
        Some("script-checksum") => script_checksum(&args[1..]),
        _ => eprintln!("usage: lab stand [ticks]"),
    }
}
