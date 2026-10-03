//! `data/*.json` into the things core needs, and the copy file.
//!
//! This crate may use `serde_json` and `sim` may not, so everything that reads
//! a human-written file lives here.

pub mod copy;
pub mod body;
pub mod setup;
pub mod messages;
