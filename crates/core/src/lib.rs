//! Rybalka core: game model and simulation.
//!
//! This crate owns all game rules and has no dependency on Tauri, so it can be
//! unit-tested and run headless.

pub mod clock;
pub mod content;
pub mod map;
pub mod rng;
pub mod sim;
pub mod weather;

pub use sim::{Input, Snapshot, World};
