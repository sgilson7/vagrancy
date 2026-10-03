//! Two-seat lockstep (D14): the wire format, the session, and an in-process
//! loopback with latency and jitter.
//!
//! Floodline's star, cut down to two seats: the host relays and keeps a
//! clock, it is not an authority, nobody simulates ahead and nobody rolls
//! back (Floodline `crates/net/src/lockstep.rs:5-8`). Unlike Floodline the
//! session is a state machine the page feeds: bytes in, bytes out, and the
//! time in milliseconds, because nothing in Rust here may read a clock. The
//! whole of it is proven on `Loopback` in `cargo test` before any browser
//! carries it, so a networking regression and a lockstep regression can never
//! be mistaken for one another.
#![forbid(unsafe_code)]

pub mod loopback;
pub mod session;
pub mod wire;

pub use session::{Session, Status};
