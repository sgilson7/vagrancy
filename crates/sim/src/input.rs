//! One input per fighter per tick (D6, widened to 16 bits for the jump and
//! the dodge Sam asked for). The only thing a player, a pilot or a peer can
//! give the world.

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Input(pub u16);

impl Input {
    pub const NONE: Input = Input(0);
    pub const SHOULDER_UP: u16 = 1 << 0;
    pub const SHOULDER_DOWN: u16 = 1 << 1;
    pub const ELBOW_IN: u16 = 1 << 2;
    pub const ELBOW_OUT: u16 = 1 << 3;
    pub const STEP_LEFT: u16 = 1 << 4;
    pub const STEP_RIGHT: u16 = 1 << 5;
    /// "Play the next round". Pressed by the page's button, or by a pilot.
    /// PLAN.md D6 left bits 6 and 7 spare; M1 found that a round cannot be
    /// restarted through the one door without an input for it
    /// (SECOND-ORDER-M1).
    pub const READY: u16 = 1 << 6;
    /// A jump: from the ground, or once in the air (Sam, 2026-10-03).
    pub const JUMP: u16 = 1 << 7;
    /// A Melee-style dodge: a roll or spot dodge on the ground, an air dodge
    /// in the air; briefly uncuttable (Sam, 2026-10-03).
    pub const DODGE: u16 = 1 << 8;
    /// Stand back up when knocked off one's feet with both legs (Sam,
    /// 2026-10-03).
    pub const STAND: u16 = 1 << 9;
    /// Let go of the sword, which flies on with the arm's swing and cuts
    /// until it first touches the ground (Sam's friend, 2026-10-05).
    pub const THROW: u16 = 1 << 10;
    /// The second pair of arms, on a four-armed body (the local deity, Sam
    /// 2026-10-06): its shoulder and elbow, as the first pair's. No key is
    /// bound to them; a pilot presses them.
    pub const SHOULDER2_UP: u16 = 1 << 11;
    pub const SHOULDER2_DOWN: u16 = 1 << 12;
    pub const ELBOW2_IN: u16 = 1 << 13;
    pub const ELBOW2_OUT: u16 = 1 << 14;
    /// Every other bit is spare and must be zero; a replay that sets one is
    /// refused.
    pub const SPARE: u16 = !0x7FFF;
    /// The first pair of arms' bits, and the second's in the same order.
    pub const ARMS: [u16; 4] = [Self::SHOULDER_UP, Self::SHOULDER_DOWN, Self::ELBOW_IN, Self::ELBOW_OUT];
    pub const ARMS2: [u16; 4] = [Self::SHOULDER2_UP, Self::SHOULDER2_DOWN, Self::ELBOW2_IN, Self::ELBOW2_OUT];

    /// The actions a key can be bound to, in the order Settings lists them.
    /// The page reads these names and bits from here rather than keeping a
    /// copy (CLAUDE.md: the page keeps no constant of its own).
    pub const ACTIONS: [(&'static str, u16); 10] = [
        ("shoulder_up", Self::SHOULDER_UP),
        ("shoulder_down", Self::SHOULDER_DOWN),
        ("elbow_in", Self::ELBOW_IN),
        ("elbow_out", Self::ELBOW_OUT),
        ("step_left", Self::STEP_LEFT),
        ("step_right", Self::STEP_RIGHT),
        ("jump", Self::JUMP),
        ("dodge", Self::DODGE),
        ("stand", Self::STAND),
        ("throw", Self::THROW),
    ];

    pub const fn has(self, bit: u16) -> bool {
        self.0 & bit != 0
    }

    /// +1, -1 or 0 for a pair of opposed bits; both held cancel.
    pub const fn axis(self, plus: u16, minus: u16) -> i32 {
        (self.has(plus) as i32) - (self.has(minus) as i32)
    }

    /// The two pairs of arms' bits traded: what the first pair was asked
    /// to do, the second does, and the other way round.
    pub const fn arms_swapped(self) -> Input {
        let mut b = self.0;
        let mut k = 0;
        while k < 4 {
            b &= !(Self::ARMS[k] | Self::ARMS2[k]);
            if self.0 & Self::ARMS[k] != 0 {
                b |= Self::ARMS2[k];
            }
            if self.0 & Self::ARMS2[k] != 0 {
                b |= Self::ARMS[k];
            }
            k += 1;
        }
        Input(b)
    }

    /// The same intent for a fighter facing the other way. Arm bits are
    /// already relative to facing; only the steps are absolute.
    pub const fn mirror(self) -> Input {
        let steps = Self::STEP_LEFT | Self::STEP_RIGHT;
        let mut b = self.0 & !steps;
        if self.has(Self::STEP_LEFT) {
            b |= Self::STEP_RIGHT;
        }
        if self.has(Self::STEP_RIGHT) {
            b |= Self::STEP_LEFT;
        }
        Input(b)
    }
}
