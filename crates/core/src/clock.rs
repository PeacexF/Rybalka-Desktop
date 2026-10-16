//! In-game clock. Time is tracked in game minutes since 00:00 of day 1.

use serde::{Deserialize, Serialize};

/// Default speed: 1 real minute = 10 game minutes.
pub const DEFAULT_SPEED: f64 = 10.0;
/// Multiplier applied while the player holds "fast forward".
pub const FAST_FORWARD: f64 = 4.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Clock {
    /// Game minutes since day 1, 00:00.
    pub minutes: f64,
    /// Game minutes per real minute.
    pub speed: f64,
    pub fast_forward: bool,
}

impl Clock {
    pub fn at(day: u32, hour: u32, minute: u32) -> Self {
        Self {
            minutes: f64::from((day.max(1) - 1) * 24 * 60 + hour * 60 + minute),
            speed: DEFAULT_SPEED,
            fast_forward: false,
        }
    }

    /// Advance by `real_dt` real seconds.
    pub fn advance(&mut self, real_dt: f64) {
        let mult = if self.fast_forward { FAST_FORWARD } else { 1.0 };
        self.minutes += real_dt / 60.0 * self.speed * mult;
    }

    /// Day number, starting at 1.
    pub fn day(&self) -> u32 {
        (self.minutes / (24.0 * 60.0)) as u32 + 1
    }

    /// Fractional hour of the current day, `0.0..24.0`.
    pub fn hour(&self) -> f64 {
        (self.minutes / 60.0).rem_euclid(24.0)
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::at(1, 5, 30)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_at_default_speed() {
        let mut c = Clock::at(1, 6, 0);
        c.advance(60.0);
        assert!((c.hour() - (6.0 + 10.0 / 60.0)).abs() < 1e-9);
    }

    #[test]
    fn wraps_to_next_day() {
        let mut c = Clock::at(1, 23, 50);
        c.speed = 60.0;
        c.advance(30.0);
        assert_eq!(c.day(), 2);
        assert!(c.hour() < 1.0);
    }
}
