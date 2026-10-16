//! Weather: changes twice a day and nudges fish activity.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::rng::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Weather {
    Clear,
    Cloudy,
    Overcast,
    Rain,
    Wind,
    Fog,
}

impl Weather {
    pub const ALL: [Weather; 6] = [
        Weather::Clear,
        Weather::Cloudy,
        Weather::Overcast,
        Weather::Rain,
        Weather::Wind,
        Weather::Fog,
    ];

    /// Default activity multiplier when a species doesn't override it.
    pub fn default_activity(self) -> f64 {
        match self {
            Weather::Clear => 0.85,
            Weather::Cloudy => 1.0,
            Weather::Overcast => 1.1,
            Weather::Rain => 1.2,
            Weather::Wind => 1.05,
            Weather::Fog => 0.9,
        }
    }

    /// Likely next weather (a small Markov chain): mostly stays, drifts to neighbours.
    fn next(self, rng: &mut Rng) -> Weather {
        use Weather::*;
        let options: &[(Weather, f64)] = match self {
            Clear => &[(Clear, 5.0), (Cloudy, 3.0), (Wind, 1.0), (Fog, 0.5)],
            Cloudy => &[(Cloudy, 4.0), (Clear, 2.0), (Overcast, 2.0), (Wind, 1.0)],
            Overcast => &[(Overcast, 3.0), (Rain, 2.5), (Cloudy, 2.0), (Wind, 1.0)],
            Rain => &[(Rain, 3.0), (Overcast, 3.0), (Cloudy, 1.0)],
            Wind => &[(Wind, 2.0), (Cloudy, 2.0), (Clear, 1.5), (Overcast, 1.0)],
            Fog => &[(Fog, 1.0), (Clear, 2.0), (Cloudy, 2.0)],
        };
        let weights: Vec<f64> = options.iter().map(|(_, w)| *w).collect();
        rng.weighted(&weights).map_or(self, |i| options[i].0)
    }
}

/// Hours at which the weather may change.
const CHANGE_HOURS: [u32; 2] = [4, 14];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherState {
    pub current: Weather,
    /// Last (day, change slot) at which the weather was rolled.
    last_roll: (u32, usize),
}

impl WeatherState {
    pub fn new(day: u32, hour: f64, rng: &mut Rng) -> Self {
        let mut s = Self {
            current: Weather::Cloudy,
            last_roll: (0, 0),
        };
        s.current = s.current.next(rng).next(rng);
        s.last_roll = slot(day, hour);
        s
    }

    /// Force the weather (debug); it holds until the next change hour.
    pub fn set(&mut self, w: Weather, day: u32, hour: f64) {
        self.current = w;
        self.last_roll = slot(day, hour);
    }

    /// Roll new weather when a change hour has passed. Returns `true` if it changed.
    pub fn update(&mut self, day: u32, hour: f64, rng: &mut Rng) -> bool {
        let now = slot(day, hour);
        if now == self.last_roll {
            return false;
        }
        self.last_roll = now;
        let before = self.current;
        self.current = self.current.next(rng);
        before != self.current
    }
}

fn slot(day: u32, hour: f64) -> (u32, usize) {
    let idx = CHANGE_HOURS.iter().rposition(|&h| hour >= f64::from(h));
    match idx {
        Some(i) => (day, i + 1),
        // Before the first change hour: still the previous day's last slot.
        None => (day.saturating_sub(1), CHANGE_HOURS.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_only_at_change_hours() {
        let mut rng = Rng::new(1);
        let mut w = WeatherState::new(1, 6.0, &mut rng);
        let mut rolls = 0;
        let mut h = 6.0;
        let mut day = 1;
        for _ in 0..(48 * 6) {
            h += 1.0 / 6.0;
            if h >= 24.0 {
                h -= 24.0;
                day += 1;
            }
            let before = w.last_roll;
            w.update(day, h, &mut rng);
            if w.last_roll != before {
                rolls += 1;
            }
        }
        // Two days → 4 change points.
        assert_eq!(rolls, 4);
    }

    #[test]
    fn all_weathers_reachable() {
        let mut rng = Rng::new(2);
        let mut seen = std::collections::BTreeSet::new();
        let mut w = Weather::Clear;
        for _ in 0..2000 {
            w = w.next(&mut rng);
            seen.insert(w);
        }
        assert_eq!(seen.len(), Weather::ALL.len());
    }
}
