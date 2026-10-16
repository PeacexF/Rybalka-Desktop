//! Groundbait patches laid by feeder casts: a local, fading bite bonus.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::FOV_K;
use super::rod::Point;
use crate::content::View;

/// Real seconds a fresh patch keeps working.
pub const LIFETIME_S: f64 = 240.0;
/// Radius of effect, metres.
pub const RADIUS_M: f64 = 3.0;
/// Patch strength added per feeder cast.
pub const PER_CAST: f64 = 0.6;
/// Cap on the strength of one patch.
pub const MAX_STRENGTH: f64 = 2.5;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Patch {
    pub at: Point,
    pub strength: f64,
    pub left_s: f64,
}

impl Patch {
    fn effective(&self) -> f64 {
        self.strength * (self.left_s / LIFETIME_S).clamp(0.0, 1.0)
    }
}

/// Ground-plane position in metres: (lateral, distance).
fn ground(view: &View, p: Point) -> (f64, f64) {
    let d = view.distance_at(p.y).min(500.0);
    ((p.x - 0.5) * FOV_K * d, d)
}

pub fn distance_m(view: &View, a: Point, b: Point) -> f64 {
    let (ax, ay) = ground(view, a);
    let (bx, by) = ground(view, b);
    (ax - bx).hypot(ay - by)
}

/// Feed a spot: strengthen a nearby patch or start a new one.
pub fn add(patches: &mut Vec<Patch>, view: &View, at: Point, strength: f64) {
    if let Some(p) = patches
        .iter_mut()
        .find(|p| distance_m(view, p.at, at) < RADIUS_M * 0.7)
    {
        p.strength = (p.effective() + strength).min(MAX_STRENGTH);
        p.left_s = LIFETIME_S;
        return;
    }
    patches.push(Patch {
        at,
        strength: strength.min(MAX_STRENGTH),
        left_s: LIFETIME_S,
    });
}

/// Bite multiplier at a point (1.0 = no groundbait).
pub fn bonus(patches: &[Patch], view: &View, at: Point) -> f64 {
    1.0 + patches
        .iter()
        .map(|p| {
            let d = distance_m(view, p.at, at) / RADIUS_M;
            p.effective() * (-d * d).exp()
        })
        .sum::<f64>()
}

pub fn decay(patches: &mut Vec<Patch>, dt: f64) {
    for p in patches.iter_mut() {
        p.left_s -= dt;
    }
    patches.retain(|p| p.left_s > 0.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: View = View {
        horizon_y: 0.5,
        shore_y: 1.0,
        near_m: 5.0,
    };

    #[test]
    fn bonus_is_local_and_fades() {
        let mut ps = Vec::new();
        let at = Point { x: 0.5, y: 0.6 };
        add(&mut ps, &VIEW, at, PER_CAST);
        let here = bonus(&ps, &VIEW, at);
        let far = bonus(&ps, &VIEW, Point { x: 0.1, y: 0.6 });
        assert!(here > 1.5, "{here}");
        assert!(far < 1.01, "{far}");
        decay(&mut ps, LIFETIME_S / 2.0);
        assert!(bonus(&ps, &VIEW, at) < here);
        decay(&mut ps, LIFETIME_S);
        assert!(ps.is_empty());
    }

    #[test]
    fn repeated_casts_stack_up_to_cap() {
        let mut ps = Vec::new();
        let at = Point { x: 0.5, y: 0.6 };
        for _ in 0..10 {
            add(&mut ps, &VIEW, at, PER_CAST);
        }
        assert_eq!(ps.len(), 1);
        assert!((ps[0].strength - MAX_STRENGTH).abs() < 1e-9);
    }
}
