//! What crosses the wire, as `postcard` bytes. Every message goes on the
//! reliable, ordered channel (D14; Floodline sends every lockstep message
//! reliably, `lockstep.rs:247` and on).

use serde::{Deserialize, Serialize};
use sim::Setup;

/// Bumped when a message changes shape; a peer on another version is
/// refused, like a peer on another build.
pub const PROTO: u32 = 3;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Msg {
    /// Joiner to host, first: who I am.
    Hello { proto: u32, build: String },
    /// Host to joiner and back, to measure the round trip.
    Ping { n: u32 },
    Pong { n: u32 },
    /// Host to joiner: the match, and the input delay both will use.
    Welcome { delay: u32, setup: Setup },
    /// Either side, in the lobby: I am ready (Sam asked for a first exchange
    /// both players can see before the match).
    Ready,
    /// Host to joiner: tick 0 begins.
    Start,
    /// Joiner to host: my input for `tick`, and my checksum after `checked`.
    Input { tick: u32, input: u16, checked: u32, sum: u64 },
    /// Host to joiner: both inputs for `tick`.
    Bundle { tick: u32, inputs: [u16; 2], checked: u32, sum: u64 },
    /// The two copies disagreed after `tick`. Both stop there.
    Stop { tick: u32 },
    Bye { reason: Bye },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Bye {
    /// This match already has two players. `online.error.full`.
    Full,
    /// Different builds of the game. `online.error.build`.
    Build,
    /// The other side gave up on this one. `online.status.left`.
    Left,
}

pub fn encode(m: &Msg) -> Vec<u8> {
    postcard::to_allocvec(m).expect("a message always encodes")
}

pub fn decode(b: &[u8]) -> Option<Msg> {
    postcard::from_bytes(b).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_message_round_trips() {
        for m in [
            Msg::Hello { proto: PROTO, build: "3f9a01c2".into() },
            Msg::Ping { n: 7 },
            Msg::Pong { n: 7 },
            Msg::Start,
            Msg::Input { tick: 1204, input: 0b101, checked: 1200, sum: 0x8c1e_0000_0000_0077 },
            Msg::Bundle { tick: 1204, inputs: [0b101, 0b1_0000], checked: 1200, sum: 1 },
            Msg::Stop { tick: 1200 },
            Msg::Bye { reason: Bye::Full },
        ] {
            assert_eq!(decode(&encode(&m)), Some(m));
        }
        assert_eq!(decode(b"\xff\xff\xff not a message"), None);
    }
}
