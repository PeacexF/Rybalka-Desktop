//! Rybalka core: game model and simulation.
//!
//! This crate owns all game rules and has no dependency on Tauri, so it can be
//! unit-tested and run headless.

pub mod sim;

pub use sim::{Snapshot, World};
