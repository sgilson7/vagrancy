//! One end of a two-seat match: the host (seat 0) or the joiner (seat 1).
//!
//! The page calls `receive` for every message, `poll` once a frame, and
//! `tick` once for every tick its clock says is due; it sends whatever
//! `take_outbox` hands back. Nothing here reads a clock or a socket.
//!
//! Inputs take effect `delay` ticks after they are pressed, on both peers
//! (D14). The first `delay` ticks run on empty input, which both sides know
//! without being told. Each side sends the checksum of a tick it has
//! finished, and whichever side first finds the two disagree stops and tells
//! the other the tick; both then show the same tick (PLAN.md §7 item 7: in
//! Floodline only the host could tell).

use crate::wire::{decode, encode, Bye, Msg, PROTO};
use sim::balance::TICKS_PER_SECOND;
use sim::replay::Recording;
use sim::{Input, Setup, World};
use std::collections::BTreeMap;

/// Round trips the host measures before it chooses the delay.
pub const PINGS: u32 = 20;
/// The input delay is never shorter or longer than this, in ticks (D14).
pub const MIN_DELAY: u32 = 2;
pub const MAX_DELAY: u32 = 8;
/// Waiting this long on the other side shows `online.status.waiting_on`;
/// this long ends the match (`online.status.left`). Floodline waits 5 s and
/// 30 s at 20 ticks a second (`lockstep.rs:25-28`); a sword fight cannot
/// freeze that long. *(guess; M4.0)*
pub const WARN_MS: u64 = 1_000;
pub const DROP_MS: u64 = 10_000;
/// A lobby that hears nothing from the other side for this long gives up.
pub const LOBBY_SILENCE_MS: u64 = 15_000;
/// While connected and waiting for Start, the host pings this often and the
/// joiner answers, so a lobby that is merely waiting is never silent. Without
/// it the connected lobby went quiet and gave up after 15 s on both sides
/// (SECOND-ORDER-M5, reported by Sam between two computers).
pub const KEEPALIVE_MS: u64 = 1_000;

/// The delay for a measured round trip: the joiner's input goes to the host
/// and comes back in a bundle, one round trip, so the delay covers the
/// slower round trips (the 90th percentile) plus a tick, clamped to 2..=8.
/// 30 ms → 3 ticks; 80 ms → 6 (PLAN.md D14, by hand).
pub fn delay_for(rtts_ms: &[u64]) -> u32 {
    if rtts_ms.is_empty() {
        return MAX_DELAY;
    }
    let mut v = rtts_ms.to_vec();
    v.sort();
    let p90 = v[((v.len() * 9).div_ceil(10)).saturating_sub(1).min(v.len() - 1)];
    let ticks = (p90 * TICKS_PER_SECOND as u64).div_ceil(1000) as u32 + 1;
    ticks.clamp(MIN_DELAY, MAX_DELAY)
}

/// The delay in milliseconds, for `{delay_ms}`: converted by core so the page
/// keeps no copy of the tick rate.
pub fn delay_ms(delay: u32) -> u32 {
    (delay * 1000).div_ceil(TICKS_PER_SECOND)
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Status {
    /// Host: waiting for someone to join. Joiner: waiting to be welcomed.
    Waiting,
    /// Host: measuring the round trip.
    Measuring,
    /// Both: the match is set; the host may start it.
    Connected { delay: u32 },
    Playing,
    /// Playing, but held up on the other side for over `WARN_MS`.
    WaitingOn,
    /// The two copies of the match disagreed after this tick.
    Desync { tick: u32 },
    /// The other side stopped answering, or left.
    Left,
    /// The host turned this joiner away.
    Refused(Bye),
}

#[derive(Debug)]
pub struct Session {
    host: bool,
    build: String,
    pub status: Status,
    setup: Option<Setup>,
    delay: u32,
    rec: Option<Recording>,
    /// Inputs known for a tick, by seat.
    inputs: BTreeMap<u32, [Option<u16>; 2]>,
    /// Ticks the host has already bundled.
    bundled: u32,
    /// My checksum after each tick I have finished, and the other side's.
    mine: BTreeMap<u32, u64>,
    theirs: BTreeMap<u32, u64>,
    outbox: Vec<Vec<u8>>,
    // The round trip.
    pings_sent: BTreeMap<u32, u64>,
    rtts: Vec<u64>,
    next_ping: u32,
    // Silence.
    heard_at: u64,
    blocked_since: Option<u64>,
    kept_at: u64,
    /// Host: whether the joiner's first input has arrived, which is the only
    /// proof that it heard Start.
    heard_input: bool,
    /// Who has said they are ready, in the lobby.
    pub me_ready: bool,
    pub them_ready: bool,
}

impl Session {
    fn new(host: bool, build: &str, setup: Option<Setup>) -> Session {
        Session {
            host,
            build: build.into(),
            status: Status::Waiting,
            setup,
            delay: 0,
            rec: None,
            inputs: BTreeMap::new(),
            bundled: 0,
            mine: BTreeMap::new(),
            theirs: BTreeMap::new(),
            outbox: Vec::new(),
            pings_sent: BTreeMap::new(),
            rtts: Vec::new(),
            next_ping: 0,
            heard_at: 0,
            blocked_since: None,
            kept_at: 0,
            heard_input: false,
            me_ready: false,
            them_ready: false,
        }
    }

    /// The host, with the match it will offer. It plays seat 0.
    pub fn host(setup: Setup, build: &str) -> Session {
        Session::new(true, build, Some(setup))
    }

    /// A joiner. It says hello at once; it plays seat 1.
    pub fn join(build: &str) -> Session {
        let mut s = Session::new(false, build, None);
        s.send(Msg::Hello { proto: PROTO, build: build.into() });
        s
    }

    pub fn is_host(&self) -> bool {
        self.host
    }

    pub fn seat(&self) -> usize {
        if self.host { 0 } else { 1 }
    }

    pub fn delay(&self) -> u32 {
        self.delay
    }

    pub fn world(&self) -> Option<&World> {
        self.rec.as_ref().map(|r| &r.world)
    }

    pub fn recording(&self) -> Option<&Recording> {
        self.rec.as_ref()
    }

    pub fn take_outbox(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.outbox)
    }

    fn send(&mut self, m: Msg) {
        self.outbox.push(encode(&m));
    }

    /// The host turns away anyone after the first (`online.error.full`).
    pub fn refusal_full() -> Vec<u8> {
        encode(&Msg::Bye { reason: Bye::Full })
    }

    pub fn stopped(&self) -> bool {
        matches!(self.status, Status::Desync { .. } | Status::Left | Status::Refused(_))
    }

    /// One message from the other side.
    pub fn receive(&mut self, now: u64, bytes: &[u8]) {
        self.heard_at = now;
        let Some(m) = decode(bytes) else { return };
        if self.stopped() {
            return;
        }
        match m {
            Msg::Hello { proto, build } if self.host => {
                if proto != PROTO || build != self.build {
                    self.send(Msg::Bye { reason: Bye::Build });
                } else if self.status == Status::Waiting {
                    self.status = Status::Measuring;
                    self.next_ping = 0;
                    self.ping(now);
                }
            }
            Msg::Ping { n } if !self.host => self.send(Msg::Pong { n }),
            Msg::Ready => self.them_ready = true,
            // Pongs after the measurement are keep-alives: heard, and that is all.
            Msg::Pong { .. } if self.host && self.status != Status::Measuring => {}
            Msg::Pong { n } if self.host => {
                if let Some(at) = self.pings_sent.remove(&n) {
                    self.rtts.push(now.saturating_sub(at));
                }
                if self.rtts.len() as u32 >= PINGS {
                    self.welcome();
                } else {
                    self.ping(now);
                }
            }
            Msg::Welcome { delay, setup } if !self.host => {
                self.delay = delay;
                self.setup = Some(setup);
                self.status = Status::Connected { delay };
            }
            // Start may arrive more than once (the host repeats it until it
            // hears an input); only the first, in the lobby, begins the match.
            Msg::Start if !self.host && matches!(self.status, Status::Connected { .. }) => self.begin(now),
            Msg::Input { tick, input, checked, sum } if self.host => {
                self.heard_input = true;
                self.inputs.entry(tick).or_insert([None, None])[1] = Some(input);
                self.theirs.insert(checked, sum);
                self.compare();
                self.bundle();
            }
            Msg::Bundle { tick, inputs, checked, sum } if !self.host => {
                self.inputs.insert(tick, [Some(inputs[0]), Some(inputs[1])]);
                self.theirs.insert(checked, sum);
                self.compare();
            }
            Msg::Stop { tick } => self.status = Status::Desync { tick },
            Msg::Bye { reason } => {
                self.status = if self.host || reason == Bye::Left { Status::Left } else { Status::Refused(reason) };
            }
            _ => {}
        }
    }

    fn ping(&mut self, now: u64) {
        let n = self.next_ping;
        self.next_ping += 1;
        self.pings_sent.insert(n, now);
        self.send(Msg::Ping { n });
    }

    fn welcome(&mut self) {
        self.delay = delay_for(&self.rtts);
        let setup = self.setup.clone().expect("the host has a setup");
        self.send(Msg::Welcome { delay: self.delay, setup });
        self.status = Status::Connected { delay: self.delay };
    }

    /// Tell the other side this one is ready. Repeated on every poll until
    /// the match starts would be noise; the reliable channel carries it once.
    pub fn ready(&mut self) {
        if matches!(self.status, Status::Connected { .. }) && !self.me_ready {
            self.me_ready = true;
            self.send(Msg::Ready);
        }
    }

    /// Host: begin the match on both sides, once both have said they are
    /// ready.
    pub fn start(&mut self, now: u64) {
        if self.host && matches!(self.status, Status::Connected { .. }) && self.me_ready && self.them_ready {
            self.send(Msg::Start);
            self.begin(now);
        }
    }

    fn begin(&mut self, now: u64) {
        let setup = self.setup.clone().expect("a setup before the start");
        self.rec = Some(Recording::new(setup));
        self.status = Status::Playing;
        self.heard_at = now;
        // The first `delay` ticks run on empty input, on both sides, untold.
        for t in 0..self.delay {
            self.inputs.insert(t, [Some(0), Some(0)]);
        }
        self.bundled = self.delay;
    }

    /// The lobby's clock: give up on a silent other side.
    pub fn poll(&mut self, now: u64) {
        // A Start the joiner never received would freeze the match with both
        // sides waiting (reported by Sam between two computers): repeat it
        // every second until the joiner's first input proves it arrived.
        if self.host && self.rec.is_some() && !self.heard_input && now.saturating_sub(self.kept_at) >= KEEPALIVE_MS {
            self.kept_at = now;
            self.send(Msg::Start);
        }
        if self.host && matches!(self.status, Status::Connected { .. }) && now.saturating_sub(self.kept_at) >= KEEPALIVE_MS {
            self.kept_at = now;
            let n = self.next_ping;
            self.next_ping += 1;
            self.send(Msg::Ping { n });
        }
        if matches!(self.status, Status::Measuring | Status::Connected { .. }) && self.heard_at > 0 && now.saturating_sub(self.heard_at) > LOBBY_SILENCE_MS {
            self.status = Status::Left;
        }
    }

    /// One tick of the local clock. `mine` is this side's input now; it takes
    /// effect `delay` ticks later. Returns the inputs the world stepped on, or
    /// `None` while waiting on the other side.
    pub fn tick(&mut self, now: u64, mine: Input) -> Option<[Input; 2]> {
        if !matches!(self.status, Status::Playing | Status::WaitingOn) {
            return None;
        }
        let t = self.rec.as_ref()?.world.tick;
        let ready = match self.inputs.get(&t) {
            Some([Some(a), Some(b)]) => Some([*a, *b]),
            _ => None,
        };
        let Some([a, b]) = ready else {
            let since = *self.blocked_since.get_or_insert(now);
            if now.saturating_sub(since) > DROP_MS {
                self.status = Status::Left;
                self.send(Msg::Bye { reason: Bye::Left });
            } else if now.saturating_sub(since) > WARN_MS {
                self.status = Status::WaitingOn;
            }
            return None;
        };
        self.blocked_since = None;
        self.status = Status::Playing;
        // My input for `t + delay`, sent now (the joiner) or kept for the
        // bundle (the host).
        let target = t + self.delay;
        let seat = self.seat();
        self.inputs.entry(target).or_insert([None, None])[seat] = Some(mine.0);
        let inputs = [Input(a), Input(b)];
        let rec = self.rec.as_mut().unwrap();
        rec.step(inputs);
        let done = rec.world.tick;
        let sum = rec.world.checksum();
        self.mine.insert(done, sum);
        if self.host {
            self.bundle();
        } else {
            self.send(Msg::Input { tick: target, input: mine.0, checked: done, sum });
        }
        self.compare();
        // Old entries are not needed once both sides are past them.
        let keep_from = done.saturating_sub(4 * MAX_DELAY);
        self.inputs = self.inputs.split_off(&keep_from);
        Some(inputs)
    }

    /// Host: send a bundle for every tick whose two inputs are both known,
    /// in order, with the host's latest checksum.
    fn bundle(&mut self) {
        if !self.host || self.rec.is_none() {
            return;
        }
        while let Some([Some(a), Some(b)]) = self.inputs.get(&self.bundled).copied() {
            let (checked, sum) = self.mine.iter().next_back().map(|(k, v)| (*k, *v)).unwrap_or((0, 0));
            let tick = self.bundled;
            self.send(Msg::Bundle { tick, inputs: [a, b], checked, sum });
            self.bundled += 1;
        }
    }

    /// Compare every tick both sides have a checksum for; the first that
    /// differs stops the match on both, named by its tick.
    fn compare(&mut self) {
        if self.stopped() {
            return;
        }
        let mut bad = None;
        for (tick, sum) in &self.theirs {
            if let Some(m) = self.mine.get(tick) {
                if m != sum {
                    bad = Some(*tick);
                    break;
                }
            }
        }
        if let Some(tick) = bad {
            self.status = Status::Desync { tick };
            self.send(Msg::Stop { tick });
        }
        // Forget what has been compared.
        let both: Vec<u32> = self.theirs.keys().filter(|t| self.mine.contains_key(t)).copied().collect();
        for t in both {
            self.theirs.remove(&t);
            if t + 4 * MAX_DELAY < self.mine.keys().next_back().copied().unwrap_or(0) {
                self.mine.remove(&t);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_delay_covers_a_round_trip_and_a_tick() {
        // PLAN.md D14 by hand: 30 ms → ceil(30·60/1000) + 1 = 2 + 1 = 3;
        // 80 ms → ceil(4.8) + 1 = 6; anything tiny → the floor of 2; a slow
        // link → the ceiling of 8.
        assert_eq!(delay_for(&[30; 20]), 3);
        assert_eq!(delay_for(&[80; 20]), 6);
        assert_eq!(delay_for(&[1; 20]), 2);
        assert_eq!(delay_for(&[900; 20]), 8);
        // The 90th percentile: of twenty, the eighteenth fastest. Two slow
        // trips in twenty fall above it; three reach it.
        let mut v = vec![20u64; 18];
        v.extend([200, 200]);
        assert_eq!(delay_for(&v), 3, "two slow round trips in twenty do not set the delay");
        let mut v = vec![20u64; 17];
        v.extend([200, 200, 200]);
        assert_eq!(delay_for(&v), 8, "three do");
        assert_eq!(delay_ms(4), 67);
    }
}
