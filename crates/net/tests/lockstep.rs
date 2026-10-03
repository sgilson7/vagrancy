//! Two-seat lockstep on the loopback: no browser, no network, a clock the
//! test owns (D14; PLANNING-BRIEF M4 part one).

use net::loopback::{Conditions, Loopback};
use net::session::{Session, Status, DROP_MS, WARN_MS};
use net::wire::{decode, encode, Msg};
use sim::{Input, World};

const BUILD: &str = "testbuild";

struct Pair {
    host: Session,
    join: Session,
    wire: Loopback,
    now: u64,
    /// When set, nothing the joiner sends reaches the host.
    cut_join_to_host: bool,
    tamper: Option<fn(Msg) -> Msg>,
}

impl Pair {
    fn new(latency_ms: u64, jitter_ms: u64) -> Pair {
        let setup = content::setup::versus(7, sim::balance::DEFAULT_TUNING);
        Pair {
            host: Session::host(setup, BUILD),
            join: Session::join(BUILD),
            wire: Loopback::new(Conditions { latency_ms, jitter_ms }, 99),
            now: 0,
            cut_join_to_host: false,
            tamper: None,
        }
    }

    fn pump(&mut self) {
        for b in self.host.take_outbox() {
            let b = match (self.tamper, decode(&b)) {
                (Some(f), Some(m)) => encode(&f(m)),
                _ => b,
            };
            self.wire.send(0, self.now, b);
        }
        for b in self.join.take_outbox() {
            if !self.cut_join_to_host {
                self.wire.send(1, self.now, b);
            }
        }
        for b in self.wire.deliver(0, self.now) {
            self.host.receive(self.now, &b);
        }
        for b in self.wire.deliver(1, self.now) {
            self.join.receive(self.now, &b);
        }
    }

    /// The lobby on a 1 ms clock, so the measured round trip is not rounded
    /// up to a tick; then the host starts the match.
    fn connect(&mut self) -> u32 {
        for _ in 0..20_000 {
            self.now += 1;
            self.pump();
            self.host.poll(self.now);
            self.join.poll(self.now);
            if let (Status::Connected { delay }, Status::Connected { .. }) = (&self.host.status, &self.join.status) {
                let d = *delay;
                self.host.start(self.now);
                self.pump();
                return d;
            }
        }
        panic!("never connected: host {:?}, joiner {:?}", self.host.status, self.join.status);
    }

    fn connect_without_start(&mut self) {
        for _ in 0..20_000 {
            self.now += 1;
            self.pump();
            self.host.poll(self.now);
            self.join.poll(self.now);
            if matches!((&self.host.status, &self.join.status), (Status::Connected { .. }, Status::Connected { .. })) {
                return;
            }
        }
        panic!("never connected");
    }

    /// Play on a 60 Hz clock (17, 17, 16 ms: 50 ms every three frames).
    /// `script(seat, tick)` is each side's input at its own world tick.
    fn play(&mut self, frames: u32, script: impl Fn(usize, u32) -> Input) {
        for f in 0..frames {
            self.now += if f % 3 == 2 { 16 } else { 17 };
            self.pump();
            let ht = self.host.world().map(|w| w.tick).unwrap_or(0);
            self.host.tick(self.now, script(0, ht));
            let jt = self.join.world().map(|w| w.tick).unwrap_or(0);
            self.join.tick(self.now, script(1, jt));
            self.pump();
        }
    }
}

fn inputs(s: &Session) -> Vec<[u16; 2]> {
    s.recording().map(|r| r.inputs.clone()).unwrap_or_default()
}

#[test]
fn the_delay_is_chosen_from_the_measured_round_trip() {
    // PLAN.md D14 by hand: a 30 ms round trip gives 3 ticks; 80 ms gives 6.
    assert_eq!(Pair::new(15, 0).connect(), 3);
    assert_eq!(Pair::new(40, 0).connect(), 6);
    let mut p = Pair::new(40, 0);
    let d = p.connect();
    p.play(10, |_, _| Input::NONE);
    assert_eq!(p.join.status, Status::Playing, "the joiner begins on the host's Start");
    assert_eq!(p.join.delay(), d, "both sides use the delay the host chose");
}

#[test]
fn an_input_takes_effect_delay_ticks_later_on_both_peers() {
    let mut p = Pair::new(20, 0);
    let delay = p.connect();
    // Each seat presses one key for one tick of its own: the host at its tick
    // 10, the joiner at its tick 20.
    p.play(120, |seat, t| match (seat, t) {
        (0, 10) => Input(Input::SHOULDER_UP),
        (1, 20) => Input(Input::ELBOW_IN),
        _ => Input::NONE,
    });
    for (who, s) in [("host", &p.host), ("joiner", &p.join)] {
        let i = inputs(s);
        assert!(i.len() > 60, "{who} only played {} ticks", i.len());
        let pressed: Vec<(usize, [u16; 2])> = i.iter().copied().enumerate().filter(|(_, x)| *x != [0, 0]).collect();
        assert_eq!(
            pressed,
            vec![(10 + delay as usize, [Input::SHOULDER_UP, 0]), (20 + delay as usize, [0, Input::ELBOW_IN])],
            "{who}: each key lands {delay} ticks after it was pressed"
        );
    }
}

#[test]
fn two_peers_stay_together_for_ten_thousand_ticks() {
    let mut p = Pair::new(35, 25);
    p.connect();
    let script = |seat: usize, t: u32| {
        // A cheap, deterministic stand-in for a person: different on each
        // seat, changing every few ticks.
        let x = (t / 9).wrapping_mul(2654435761).wrapping_add(seat as u32 * 97);
        Input(((x >> 7) & 0b11_1111) as u16)
    };
    p.play(12_000, script);
    let (h, j) = (inputs(&p.host), inputs(&p.join));
    assert!(h.len() >= 10_000 && j.len() >= 10_000, "only {} and {} ticks were played", h.len(), j.len());
    let n = h.len().min(j.len());
    assert_eq!(h[..n], j[..n], "the two peers stepped on different inputs");
    assert!(matches!(p.host.status, Status::Playing | Status::WaitingOn), "{:?}", p.host.status);
    assert!(matches!(p.join.status, Status::Playing | Status::WaitingOn), "{:?}", p.join.status);
    // Replay both up to the shorter, and they are the same world.
    let replay = |r: &[[u16; 2]]| {
        let mut w = World::new(content::setup::versus(7, sim::balance::DEFAULT_TUNING));
        for x in r {
            w.step([Input(x[0]), Input(x[1])]);
        }
        w.checksum()
    };
    assert_eq!(replay(&h[..n]), replay(&j[..n]));
}

#[test]
fn jitter_changes_nothing_but_the_wait() {
    let script = |seat: usize, t: u32| Input(((t * 7 + seat as u32 * 3) / 11 % 64) as u16);
    let run = |jitter: u64| {
        let mut p = Pair::new(25, 0);
        p.connect();
        // Jitter only after the lobby, so both runs choose the same delay.
        p.wire.conditions.jitter_ms = jitter;
        p.play(1_500, script);
        (inputs(&p.host), inputs(&p.join))
    };
    let (calm_h, calm_j) = run(0);
    let (rough_h, rough_j) = run(80);
    let m = calm_h.len().min(calm_j.len());
    assert_eq!(calm_h[..m], calm_j[..m]);
    let n = rough_h.len().min(rough_j.len());
    assert!(n > 500, "the rough run played only {n} ticks");
    assert_eq!(rough_h[..n], rough_j[..n], "jitter made the peers disagree");
    assert_eq!(rough_h[..n], calm_h[..n], "jitter changed what was played, not only when");
    assert!(rough_h.len() < calm_h.len(), "jitter should cost time: {} against {} ticks", rough_h.len(), calm_h.len());
}

#[test]
fn a_mismatch_stops_both_peers_on_the_same_tick_and_names_it() {
    let mut p = Pair::new(20, 0);
    // The joiner is welcomed into a different match: its spawn differs, so the
    // two copies part on the first tick.
    p.tamper = Some(|m| match m {
        Msg::Welcome { delay, mut setup } => {
            setup.seed ^= 1;
            Msg::Welcome { delay, setup }
        }
        m => m,
    });
    p.connect();
    p.tamper = None;
    p.play(120, |_, _| Input::NONE);
    match (&p.host.status, &p.join.status) {
        (Status::Desync { tick: a }, Status::Desync { tick: b }) => {
            assert_eq!(a, b, "the two sides named different ticks");
            assert!(*a >= 1 && *a <= 10, "the first differing tick should be early, was {a}");
        }
        other => panic!("both sides should have stopped: {other:?}"),
    }
}

#[test]
fn a_silent_peer_is_waited_for_and_then_dropped() {
    let mut p = Pair::new(20, 0);
    p.connect();
    p.play(120, |_, _| Input::NONE);
    assert_eq!(p.host.status, Status::Playing);
    p.cut_join_to_host = true;
    let frames = |ms: u64| (ms * 60 / 1000) as u32;
    p.play(frames(WARN_MS) + 30, |_, _| Input::NONE);
    assert_eq!(p.host.status, Status::WaitingOn, "after a second of silence the host is waiting");
    p.play(frames(DROP_MS - WARN_MS) + 30, |_, _| Input::NONE);
    assert_eq!(p.host.status, Status::Left, "after ten seconds the host gives up");
    p.play(120, |_, _| Input::NONE);
    assert_eq!(p.join.status, Status::Left, "and the joiner hears that it did");
}

#[test]
fn a_connected_lobby_waits_for_the_host_to_start_however_long_it_takes() {
    // Two people on two computers take longer than a moment to press Start.
    // The connected lobby went quiet and its silence timer ended the match
    // after 15 s, on both sides, with nothing wrong (reported by Sam between
    // two computers; reproduced on this machine).
    let mut p = Pair::new(30, 10);
    let mut connected = false;
    for _ in 0..20_000 {
        p.now += 1;
        p.pump();
        p.host.poll(p.now);
        p.join.poll(p.now);
        if matches!((&p.host.status, &p.join.status), (Status::Connected { .. }, Status::Connected { .. })) {
            connected = true;
            break;
        }
    }
    assert!(connected);
    // A minute in the lobby, the page polling ten times a second.
    for _ in 0..600 {
        p.now += 100;
        p.pump();
        p.host.poll(p.now);
        p.join.poll(p.now);
    }
    assert!(matches!(p.host.status, Status::Connected { .. }), "host: {:?}", p.host.status);
    assert!(matches!(p.join.status, Status::Connected { .. }), "joiner: {:?}", p.join.status);
    p.host.start(p.now);
    p.play(60, |_, _| Input::NONE);
    assert_eq!(p.join.status, Status::Playing, "and the match still starts");
}

#[test]
fn a_lobby_whose_other_side_has_really_gone_still_gives_up() {
    let mut p = Pair::new(30, 0);
    p.connect_without_start();
    p.cut_join_to_host = true;
    for _ in 0..300 {
        p.now += 100;
        p.pump();
        p.host.poll(p.now);
        p.join.poll(p.now);
    }
    assert_eq!(p.host.status, Status::Left, "the host heard nothing for 30 s");
}

#[test]
fn a_start_that_is_lost_is_sent_again_and_the_match_begins() {
    let mut p = Pair::new(30, 0);
    p.connect_without_start();
    // The host presses Start, and the message is lost on the way.
    p.host.start(p.now);
    let lost = p.host.take_outbox();
    assert!(!lost.is_empty());
    for _ in 0..30 {
        p.now += 100;
        p.pump();
        p.host.poll(p.now);
        p.join.poll(p.now);
        p.play(1, |_, _| Input::NONE);
    }
    assert_eq!(p.join.status, Status::Playing, "the joiner never began: {:?}", p.join.status);
    p.play(120, |_, _| Input::NONE);
    assert!(p.host.world().unwrap().tick > 60, "the match is stuck at tick {}", p.host.world().unwrap().tick);
    assert_eq!(p.host.status, Status::Playing);
}
