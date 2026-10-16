use serde::Serialize;
use ts_rs::TS;

/// Fixed simulation rate.
pub const TICK_HZ: u32 = 20;
/// Duration of one simulation tick in seconds.
pub const TICK_DT: f64 = 1.0 / TICK_HZ as f64;

/// Authoritative game state, advanced in fixed steps.
#[derive(Debug, Default)]
pub struct World {
    tick: u64,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the simulation by one fixed tick.
    pub fn step(&mut self) {
        self.tick += 1;
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            tick: self.tick,
            time_s: self.tick as f64 * TICK_DT,
        }
    }
}

/// State streamed to the UI every tick.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Snapshot {
    #[ts(type = "number")]
    pub tick: u64,
    pub time_s: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_advances_tick_and_time() {
        let mut world = World::new();
        for _ in 0..TICK_HZ {
            world.step();
        }
        let snap = world.snapshot();
        assert_eq!(snap.tick, u64::from(TICK_HZ));
        assert!((snap.time_s - 1.0).abs() < 1e-9);
    }
}
