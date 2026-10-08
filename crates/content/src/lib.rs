//! `data/*.json` into the things core needs, and the copy file.
//!
//! This crate may use `serde_json` and `sim` may not, so everything that reads
//! a human-written file lives here.

pub mod copy;
pub mod costumes;
pub mod custom;
pub mod agent;
pub mod body;
pub mod bubbles;
pub mod setup;
pub mod slosh;
pub mod maps;
pub mod messages;
pub mod road;
pub mod save;
pub mod story;
pub mod trees;
pub mod tutorial;
pub mod weapons;
