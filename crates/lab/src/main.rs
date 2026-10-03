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
        let a = Input(rng.below(64) as u8);
        let b = Input(rng.below(64) as u8);
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

/// The checksum the gate compares the browser against.
fn script_checksum(args: &[String]) {
    let ticks: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(600);
    println!("{}", sim::replay::script_checksum_of(content::setup::versus(2026, sim::balance::DEFAULT_TUNING), ticks));
}

/// Run a script of (input, ticks) and report the highest the pelvis rose
/// above where it stood, and how far it travelled.
fn run_script(tuning: u8, script: &[(u8, u32)]) -> (f64, f64, u32) {
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
    let pogo: Vec<(u8, u32)> = vec![(I::ELBOW_IN, 13), (0, 9), (I::SHOULDER_DOWN | I::ELBOW_OUT, 40), (0, 60)];
    let spin: Vec<(u8, u32)> = vec![(I::SHOULDER_UP, 240)];
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
    let script: Vec<(u8, u32)> = args.first().map(|s| s.split(',').map(|part| {
        let (k, n) = part.split_once(':').unwrap();
        let mut b = 0u8;
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
            let script: Vec<(u8, u32)> = (0..n).map(|_| (keys[r.below(8) as usize], 3 + r.below(40))).chain([(0, 60)]).collect();
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
        let mut best = (0.0f64, 0u32, 0u32, 0u8);
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
                    let script = [(first, n1), (Input::ELBOW_OUT, n2), (0u8, 60)];
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
        let i = if t < 40 { Input(r.below(16) as u8) } else { Input::NONE };
        w.step([i, Input::NONE]);
        if t % 40 == 39 {
            println!("tick {:>3}: kinetic {:>8.0}   momentum {:?}", t + 1, w.kinetic() as f64 / 4096.0 / 4096.0, w.momentum());
        }
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
        Some("golden") => golden(),
        Some("script-checksum") => script_checksum(&args[1..]),
        _ => eprintln!("usage: lab stand [ticks]"),
    }
}
