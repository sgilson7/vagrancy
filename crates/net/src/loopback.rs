//! Two peers in one process, with latency and jitter, on a clock the test
//! owns.
//!
//! After Floodline's `crates/net/src/loopback.rs`, which has latency and loss
//! but no jitter (PLAN.md §7 item 6). Every lockstep message here is
//! reliable and ordered, so jitter may delay a message but never reorder it:
//! each message arrives no earlier than the one sent before it on the same
//! side. The jitter is drawn from a seeded generator, so a test that fails
//! once fails every time.

use sim::rng::Rng;
use std::collections::VecDeque;

#[derive(Copy, Clone, Debug)]
pub struct Conditions {
    /// One-way delay in milliseconds.
    pub latency_ms: u64,
    /// Up to this much extra, drawn per message.
    pub jitter_ms: u64,
}

/// A pipe each way between peer 0 (the host) and peer 1.
pub struct Loopback {
    pub conditions: Conditions,
    rng: Rng,
    /// (arrival time, bytes), in send order, for each receiver.
    queues: [VecDeque<(u64, Vec<u8>)>; 2],
    last_arrival: [u64; 2],
    pub sent: [u64; 2],
}

impl Loopback {
    pub fn new(conditions: Conditions, seed: u64) -> Loopback {
        Loopback { conditions, rng: Rng::new(seed), queues: [VecDeque::new(), VecDeque::new()], last_arrival: [0, 0], sent: [0, 0] }
    }

    /// `from` sends to the other peer at time `now`.
    pub fn send(&mut self, from: usize, now: u64, bytes: Vec<u8>) {
        let to = 1 - from;
        let jitter = if self.conditions.jitter_ms > 0 { self.rng.below(self.conditions.jitter_ms as u32 + 1) as u64 } else { 0 };
        let at = (now + self.conditions.latency_ms + jitter).max(self.last_arrival[to]);
        self.last_arrival[to] = at;
        self.queues[to].push_back((at, bytes));
        self.sent[from] += 1;
    }

    /// Everything that has arrived for `to` by `now`, in order.
    pub fn deliver(&mut self, to: usize, now: u64) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        while matches!(self.queues[to].front(), Some((at, _)) if *at <= now) {
            out.push(self.queues[to].pop_front().unwrap().1);
        }
        out
    }
}
