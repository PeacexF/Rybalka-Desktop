//! Bite model: how often which fish takes the bait, and how the bite plays out.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::content::{BiteStyle, Content, FishDef, Horizon, ZoneDef};
use crate::map::Cell;
use crate::rng::Rng;
use crate::weather::Weather;

/// Bites per real second for a species with every factor at 1.0.
pub const BASE_RATE: f64 = 1.0 / 45.0;

/// Where the bait is in the water column.
#[derive(Debug, Clone, Copy)]
pub struct Presentation<'a> {
    pub cell: Cell,
    /// Depth of the bait below the surface.
    pub bait_depth_m: f64,
    pub bait: &'a str,
    pub hour: f64,
    pub weather: Weather,
    /// Extra multiplier (groundbait etc).
    pub bonus: f64,
}

impl Presentation<'_> {
    pub fn on_bottom(&self) -> bool {
        self.bait_depth_m >= self.cell.depth_m - 0.05
    }
}

/// Bite rate of one species at the given presentation, bites per real second.
pub fn species_rate(fish: &FishDef, zone_weight: f64, p: &Presentation) -> f64 {
    let bait = fish.baits.get(p.bait).copied().unwrap_or(0.0);
    if bait <= 0.0 || zone_weight <= 0.0 {
        return 0.0;
    }
    BASE_RATE
        * zone_weight
        * bait
        * depth_match(fish, p.cell.depth_m)
        * horizon_match(fish.horizon, p)
        * fish.activity_at(p.hour)
        * fish.weather_factor(p.weather)
        * p.bonus
}

/// Rates for all species present in the zone, in zone order.
pub fn rates<'c>(
    content: &'c Content,
    zone: &ZoneDef,
    p: &Presentation,
) -> Vec<(&'c FishDef, f64)> {
    zone.fish
        .iter()
        .filter_map(|(id, w)| content.fish(id).map(|f| (f, species_rate(f, *w, p))))
        .collect()
}

fn depth_match(fish: &FishDef, depth: f64) -> f64 {
    let outside = if depth < fish.depth.min {
        fish.depth.min - depth
    } else if depth > fish.depth.max {
        depth - fish.depth.max
    } else {
        0.0
    };
    (-outside / 1.0).exp()
}

fn horizon_match(h: Horizon, p: &Presentation) -> f64 {
    let off_bottom = (p.cell.depth_m - p.bait_depth_m).max(0.0);
    match h {
        Horizon::Any => 1.0,
        Horizon::Bottom if p.on_bottom() => 1.0,
        Horizon::Bottom if off_bottom < 0.3 => 0.6,
        Horizon::Bottom => 0.15,
        Horizon::Mid if p.on_bottom() => 0.35,
        Horizon::Mid => 1.0,
    }
}

/// Roll a weight for a freshly hooked fish: log-normal around `typical`.
pub fn roll_weight(fish: &FishDef, rng: &mut Rng) -> f64 {
    let w = fish.weight;
    let n = rng.normal();
    let sigma = if n >= 0.0 {
        (w.max / w.typical).ln() / 3.0
    } else {
        (w.typical / w.min).ln() / 2.5
    };
    let kg = (w.typical * (sigma * n).exp()).clamp(w.min, w.max);
    (kg * 1000.0).round() / 1000.0
}

/// Visible stage of a bite. The float/quivertip animates according to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum BitePhase {
    /// Small trembles — fish is testing the bait.
    Nibble,
    /// Short dip under the surface.
    Dip,
    /// Float goes under.
    Sink,
    /// Float slides away sideways.
    Lead,
    /// Float rises and lays flat.
    Lift,
    /// Hard grab (predators, feeder tip slams).
    Grab,
}

impl BitePhase {
    /// Chance that a strike during this phase hooks the fish.
    pub fn hook_chance(self) -> f64 {
        match self {
            Self::Nibble => 0.05,
            Self::Dip => 0.35,
            Self::Sink | Self::Lead | Self::Lift | Self::Grab => 0.9,
        }
    }
}

/// Generate the sequence of bite phases with durations (seconds).
pub fn bite_script(style: BiteStyle, rng: &mut Rng) -> Vec<(BitePhase, f64)> {
    use BitePhase::*;
    let mut s = Vec::new();
    match style {
        BiteStyle::Twitchy => {
            for _ in 0..(1 + (rng.f64() * 3.0) as usize) {
                s.push((Nibble, rng.range(0.4, 1.2)));
                if rng.chance(0.4) {
                    s.push((Dip, rng.range(0.25, 0.45)));
                }
            }
            s.push((Sink, rng.range(0.9, 1.6)));
        }
        BiteStyle::Bold => {
            if rng.chance(0.5) {
                s.push((Nibble, rng.range(0.3, 0.7)));
            }
            s.push((Dip, rng.range(0.3, 0.5)));
            s.push((Sink, rng.range(1.0, 1.8)));
        }
        BiteStyle::Slow => {
            s.push((Nibble, rng.range(1.5, 4.0)));
            if rng.chance(0.5) {
                s.push((Dip, rng.range(0.3, 0.5)));
                s.push((Nibble, rng.range(0.8, 2.0)));
            }
            s.push((Lead, rng.range(1.5, 2.5)));
        }
        BiteStyle::Lift => {
            s.push((Nibble, rng.range(1.0, 2.5)));
            s.push((Lift, rng.range(1.8, 3.0)));
        }
        BiteStyle::Predator => {
            s.push((Grab, rng.range(0.8, 1.4)));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Range, WeightDist};
    use crate::map::Bottom;
    use std::collections::BTreeMap;

    fn fish(horizon: Horizon) -> FishDef {
        FishDef {
            id: "roach".into(),
            name: "Плотва".into(),
            weight: WeightDist {
                min: 0.05,
                typical: 0.2,
                max: 1.0,
            },
            trophy_kg: 0.6,
            horizon,
            depth: Range { min: 1.0, max: 3.0 },
            activity: vec![],
            baits: BTreeMap::from([("worm".to_string(), 1.0)]),
            strength: 2.0,
            bite: BiteStyle::Twitchy,
            price_per_kg: 50.0,
            xp_per_kg: 10.0,
            weather: BTreeMap::new(),
        }
    }

    fn pres(depth: f64, bait_depth: f64, bait: &str) -> Presentation<'_> {
        Presentation {
            cell: Cell {
                depth_m: depth,
                bottom: Bottom::Silt,
                zone: 1,
            },
            bait_depth_m: bait_depth,
            bait,
            hour: 8.0,
            weather: Weather::Cloudy,
            bonus: 1.0,
        }
    }

    #[test]
    fn wrong_bait_never_bites() {
        assert_eq!(
            species_rate(&fish(Horizon::Any), 1.0, &pres(2.0, 1.0, "bread")),
            0.0
        );
    }

    #[test]
    fn ideal_conditions_hit_base_rate() {
        let r = species_rate(&fish(Horizon::Any), 1.0, &pres(2.0, 1.0, "worm"));
        assert!((r - BASE_RATE).abs() < 1e-12);
    }

    #[test]
    fn bottom_feeder_prefers_bait_on_bottom() {
        let f = fish(Horizon::Bottom);
        let on = species_rate(&f, 1.0, &pres(2.0, 2.0, "worm"));
        let off = species_rate(&f, 1.0, &pres(2.0, 0.5, "worm"));
        assert!(on > off * 3.0);
    }

    #[test]
    fn depth_outside_range_falls_off() {
        let f = fish(Horizon::Any);
        let ok = species_rate(&f, 1.0, &pres(2.0, 1.0, "worm"));
        let deep = species_rate(&f, 1.0, &pres(6.0, 1.0, "worm"));
        assert!(deep < ok * 0.1);
    }

    #[test]
    fn weights_stay_in_range_and_centre_on_typical() {
        let f = fish(Horizon::Any);
        let mut rng = Rng::new(9);
        let mut ws: Vec<f64> = (0..5000).map(|_| roll_weight(&f, &mut rng)).collect();
        assert!(ws.iter().all(|w| (0.05..=1.0).contains(w)));
        ws.sort_by(f64::total_cmp);
        let median = ws[ws.len() / 2];
        assert!((0.17..0.23).contains(&median), "median {median}");
    }

    #[test]
    fn every_script_ends_in_a_hookable_phase() {
        let mut rng = Rng::new(5);
        for style in [
            BiteStyle::Twitchy,
            BiteStyle::Bold,
            BiteStyle::Slow,
            BiteStyle::Lift,
            BiteStyle::Predator,
        ] {
            for _ in 0..50 {
                let s = bite_script(style, &mut rng);
                assert!(s.last().unwrap().0.hook_chance() > 0.5);
            }
        }
    }
}
