//! Spinning: lure retrieve, running depth, snags and predator strikes.

use serde::{Deserialize, Serialize};

use super::rod::Point;
use crate::content::{LureDef, View};
use crate::map::{Bottom, Cell};

/// Lures get more attention than baits sitting still: they cover water.
pub const LURE_RATE_MULT: f64 = 1.5;
/// The lure is out of the water once it is this close.
pub const LURE_OUT_M: f64 = 2.5;
/// Chance per second (while reeling along the bottom) of a snag.
const SNAG_RATE: f64 = 0.6;
const WEED_RATE: f64 = 0.15;
/// Chance that a pull frees a snagged lure.
pub const FREE_CHANCE: f64 = 0.55;

/// A lure on its way back to the angler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LureRun {
    /// Where the lure landed; it comes back along this line.
    pub origin: Point,
    pub distance_m: f64,
    pub depth_m: f64,
    pub reeling: bool,
}

impl LureRun {
    pub fn new(origin: Point, view: &View) -> Self {
        Self {
            origin,
            distance_m: view.distance_at(origin.y),
            depth_m: 0.0,
            reeling: false,
        }
    }

    pub fn point(&self, view: &View) -> Point {
        Point {
            x: self.origin.x,
            y: view.y_at(self.distance_m.max(view.near_m * 0.5)),
        }
    }

    /// Move the lure for one tick. Returns `false` once it is back at the angler.
    pub fn advance(&mut self, lure: &LureDef, speed_mps: f64, dt: f64) -> bool {
        if self.reeling {
            self.distance_m -= speed_mps * dt;
            // Towards the running depth; faster retrieve pulls it a bit higher.
            let target = lure.depth * (1.0 - 0.3 * (speed_mps - lure.speed.max).max(0.0));
            let step = 0.6 * dt;
            self.depth_m += (target - self.depth_m).clamp(-step, step);
        } else {
            self.depth_m += lure.sink_mps * dt;
        }
        self.depth_m = self.depth_m.max(0.0);
        self.distance_m > LURE_OUT_M
    }

    /// Keep the lure above the bottom of `cell`.
    pub fn clamp_to(&mut self, cell: &Cell) {
        self.depth_m = self.depth_m.min(cell.depth_m);
    }

    pub fn on_bottom(&self, cell: &Cell) -> bool {
        self.depth_m >= cell.depth_m - 0.05
    }

    /// Per-second chance of a snag at this cell.
    pub fn snag_rate(&self, cell: &Cell) -> f64 {
        if !self.reeling || !self.on_bottom(cell) {
            return 0.0;
        }
        match cell.bottom {
            Bottom::Snag => SNAG_RATE,
            Bottom::Weed => WEED_RATE,
            _ => 0.0,
        }
    }

    /// How attractive the presentation is (multiplies the bite rate).
    pub fn attraction(&self, lure: &LureDef, speed_mps: f64) -> f64 {
        let base = if self.reeling {
            let outside = if speed_mps < lure.speed.min {
                lure.speed.min - speed_mps
            } else if speed_mps > lure.speed.max {
                speed_mps - lure.speed.max
            } else {
                0.0
            };
            (-outside / 0.25).exp()
        } else {
            lure.pause
        };
        base * LURE_RATE_MULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Range;

    const VIEW: View = View {
        horizon_y: 0.5,
        shore_y: 1.0,
        near_m: 5.0,
    };

    fn lure(sink: f64) -> LureDef {
        LureDef {
            speed: Range { min: 0.5, max: 0.9 },
            depth: 1.0,
            sink_mps: sink,
            pause: 0.3,
        }
    }

    fn cell(depth: f64, bottom: Bottom) -> Cell {
        Cell {
            depth_m: depth,
            bottom,
            zone: 1,
        }
    }

    #[test]
    fn retrieve_brings_lure_home() {
        let mut run = LureRun::new(Point { x: 0.5, y: 0.6 }, &VIEW);
        assert!((run.distance_m - 25.0).abs() < 1e-9);
        run.reeling = true;
        let mut ticks = 0;
        while run.advance(&lure(0.5), 0.7, 0.05) {
            ticks += 1;
            assert!(ticks < 10_000);
        }
        // 22.5 m at 0.7 m/s ≈ 32 s.
        assert!((600..700).contains(&ticks), "{ticks}");
    }

    #[test]
    fn paused_jig_sinks_to_bottom_and_floater_rises() {
        let c = cell(3.0, Bottom::Silt);
        let mut jig = LureRun::new(Point { x: 0.5, y: 0.6 }, &VIEW);
        for _ in 0..200 {
            jig.advance(&lure(0.8), 0.5, 0.05);
            jig.clamp_to(&c);
        }
        assert!(jig.on_bottom(&c));

        // 3 m at 0.1 m/s takes 30 s.
        let mut wobbler = jig.clone();
        for _ in 0..700 {
            wobbler.advance(&lure(-0.1), 0.5, 0.05);
        }
        assert_eq!(wobbler.depth_m, 0.0);
    }

    #[test]
    fn speed_outside_working_range_is_less_attractive() {
        let mut run = LureRun::new(Point { x: 0.5, y: 0.6 }, &VIEW);
        run.reeling = true;
        let l = lure(0.5);
        let good = run.attraction(&l, 0.7);
        let fast = run.attraction(&l, 1.6);
        assert!(good > fast * 5.0);
    }

    #[test]
    fn snags_only_on_bottom_over_snags() {
        let mut run = LureRun::new(Point { x: 0.5, y: 0.6 }, &VIEW);
        run.reeling = true;
        run.depth_m = 2.0;
        assert_eq!(run.snag_rate(&cell(2.0, Bottom::Silt)), 0.0);
        assert!(run.snag_rate(&cell(2.0, Bottom::Snag)) > 0.0);
        assert_eq!(run.snag_rate(&cell(4.0, Bottom::Snag)), 0.0);
    }
}
