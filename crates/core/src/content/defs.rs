use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A fish species.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishDef {
    pub id: String,
    pub name: String,
    pub weight: WeightDist,
    /// Weight from which a catch counts as a trophy.
    pub trophy_kg: f64,
    pub horizon: Horizon,
    /// Preferred water depth range, metres.
    pub depth: Range,
    /// Activity keypoints `(hour, 0..1)`, linearly interpolated and wrapped.
    pub activity: Vec<(f64, f64)>,
    /// Bait/lure id → attractiveness `0..1`. Missing = not taken.
    pub baits: BTreeMap<String, f64>,
    /// Pull per kg of body weight during the fight.
    pub strength: f64,
    pub bite: BiteStyle,
    pub price_per_kg: f64,
    pub xp_per_kg: f64,
    /// Overrides of the default weather multipliers for this species.
    #[serde(default)]
    pub weather: BTreeMap<crate::weather::Weather, f64>,
}

impl FishDef {
    pub fn weather_factor(&self, w: crate::weather::Weather) -> f64 {
        self.weather
            .get(&w)
            .copied()
            .unwrap_or_else(|| w.default_activity())
    }
}

impl FishDef {
    /// Activity at a fractional hour `0..24`.
    pub fn activity_at(&self, hour: f64) -> f64 {
        let pts = &self.activity;
        match pts.len() {
            0 => return 1.0,
            1 => return pts[0].1,
            _ => {}
        }
        let h = hour.rem_euclid(24.0);
        // Find surrounding keypoints, wrapping around midnight.
        let (mut a, mut b) = (pts[pts.len() - 1], pts[0]);
        a.0 -= 24.0;
        for w in pts.windows(2) {
            if w[0].0 <= h && h < w[1].0 {
                (a, b) = (w[0], w[1]);
                break;
            }
        }
        if h >= pts[pts.len() - 1].0 {
            a = pts[pts.len() - 1];
            b = (pts[0].0 + 24.0, pts[0].1);
        }
        let t = if b.0 > a.0 {
            (h - a.0) / (b.0 - a.0)
        } else {
            0.0
        };
        a.1 + (b.1 - a.1) * t
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeightDist {
    pub min: f64,
    /// Most common weight (median of the log-normal).
    pub typical: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Range {
    pub min: f64,
    pub max: f64,
}

impl Range {
    pub fn contains(&self, x: f64) -> bool {
        (self.min..=self.max).contains(&x)
    }
}

/// Where in the water column the species feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Horizon {
    Bottom,
    Mid,
    Any,
}

/// How the species takes the bait; drives the bite animation phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum BiteStyle {
    /// Quick nibbles, then a sharp sink (roach, rudd).
    Twitchy,
    /// Short and decisive (perch, ruffe).
    Bold,
    /// Long fidgeting, then leads away (crucian, tench).
    Slow,
    /// Lifts the float and lays it flat (bream).
    Lift,
    /// Hard grab on a moving lure (pike, zander).
    Predator,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaitDef {
    pub id: String,
    pub name: String,
    pub kind: BaitKind,
    /// Price of one pack.
    #[serde(default)]
    pub price: f64,
    /// Units per pack (portions of bait, lures come singly).
    #[serde(default = "one")]
    pub pack: u32,
    #[serde(default)]
    pub rank: u32,
    /// Required for `Lure`s.
    #[serde(default)]
    pub lure: Option<LureDef>,
}

/// How an artificial lure behaves on the retrieve.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LureDef {
    /// Retrieve speed (m/s) at which it works best.
    pub speed: Range,
    /// Running depth while reeled, metres (clamped by the bottom).
    pub depth: f64,
    /// Vertical speed while paused, m/s (+ sinks, − floats up).
    pub sink_mps: f64,
    /// Attractiveness while paused (falling/hanging), relative to a good retrieve.
    pub pause: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum BaitKind {
    Natural,
    Lure,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocationDef {
    pub id: String,
    pub name: String,
    pub kind: WaterKind,
    #[serde(default)]
    pub description: String,
    /// Rank needed to buy a pass.
    #[serde(default)]
    pub rank: u32,
    /// Pass price per day.
    #[serde(default)]
    pub price_per_day: f64,
    /// Longest pass, days.
    #[serde(default = "week")]
    pub max_days: u32,
    pub spots: Vec<SpotDef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum WaterKind {
    Pond,
    Lake,
    River,
    Reservoir,
}

impl LocationDef {
    pub fn spot(&self, id: &str) -> Option<&SpotDef> {
        self.spots.iter().find(|s| s.id == id)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpotDef {
    pub id: String,
    pub name: String,
    /// Background image, relative to `assets/`.
    pub background: String,
    /// Water map image (RGBA), relative to `assets/`.
    pub map: String,
    pub view: View,
    pub zones: Vec<ZoneDef>,
}

impl SpotDef {
    pub fn zone(&self, id: u8) -> Option<&ZoneDef> {
        self.zones.iter().find(|z| z.id == id)
    }
}

/// Perspective calibration of a spot, in normalized image coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct View {
    /// `y` of the horizon (infinitely far).
    pub horizon_y: f64,
    /// `y` of the near shoreline.
    pub shore_y: f64,
    /// Distance from the angler to the water at `shore_y`, metres.
    pub near_m: f64,
    /// Horizontal field of view: at distance `d` the image is `fov * d` metres
    /// wide (1.2 for a 16:9 photo; wider panoramas need more).
    #[serde(default = "default_fov")]
    pub fov: f64,
}

impl View {
    /// Distance from the angler to a point at image `y`.
    pub fn distance_at(&self, y: f64) -> f64 {
        if y <= self.horizon_y + 1e-6 {
            return f64::INFINITY;
        }
        self.near_m * (self.shore_y - self.horizon_y) / (y - self.horizon_y)
    }

    /// Image `y` of a point at distance `d` metres.
    pub fn y_at(&self, d: f64) -> f64 {
        self.horizon_y + self.near_m * (self.shore_y - self.horizon_y) / d.max(1e-3)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneDef {
    pub id: u8,
    pub name: String,
    /// Fish id → population weight in this zone.
    pub fish: BTreeMap<String, f64>,
}

fn one() -> u32 {
    1
}

fn default_fov() -> f64 {
    1.2
}

fn week() -> u32 {
    7
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fish_with_activity(activity: Vec<(f64, f64)>) -> FishDef {
        FishDef {
            id: "f".into(),
            name: "F".into(),
            weight: WeightDist {
                min: 0.1,
                typical: 0.2,
                max: 1.0,
            },
            trophy_kg: 0.5,
            horizon: Horizon::Any,
            depth: Range { min: 0.0, max: 5.0 },
            activity,
            baits: BTreeMap::new(),
            strength: 1.0,
            bite: BiteStyle::Bold,
            price_per_kg: 1.0,
            xp_per_kg: 1.0,
            weather: BTreeMap::new(),
        }
    }

    #[test]
    fn activity_interpolates_and_wraps() {
        let f = fish_with_activity(vec![(6.0, 1.0), (12.0, 0.0), (18.0, 1.0)]);
        assert!((f.activity_at(6.0) - 1.0).abs() < 1e-9);
        assert!((f.activity_at(9.0) - 0.5).abs() < 1e-9);
        assert!((f.activity_at(15.0) - 0.5).abs() < 1e-9);
        // 18 → 6 (next day) stays at 1.0
        assert!((f.activity_at(0.0) - 1.0).abs() < 1e-9);
        assert!((f.activity_at(23.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn view_round_trips() {
        let v = View {
            horizon_y: 0.4,
            shore_y: 0.9,
            near_m: 3.0,
            fov: 1.2,
        };
        assert!((v.distance_at(0.9) - 3.0).abs() < 1e-9);
        for d in [3.0, 10.0, 25.0, 60.0] {
            assert!((v.distance_at(v.y_at(d)) - d).abs() < 1e-6);
        }
        assert!(v.distance_at(0.5) > v.distance_at(0.7));
    }
}
