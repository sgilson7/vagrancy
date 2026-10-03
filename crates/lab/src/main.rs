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

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("stand") => stand(&args[1..]),
        Some("points") => points(&args[1..]),
        Some("recon-m1") => recon_m1(),
        Some("recon-step") => recon_step(),
        Some("swing") => swing(&args[1..]),
        Some("golden") => golden(),
        Some("script-checksum") => script_checksum(&args[1..]),
        _ => eprintln!("usage: lab stand [ticks]"),
    }
}
