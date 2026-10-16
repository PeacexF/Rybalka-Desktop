//! Arcade fight model: line tension, drag, rushes, fatigue, slack.

use serde::{Deserialize, Serialize};

use crate::rng::Rng;

/// Fraction of the overload above drag that still reaches the line (drag inertia).
const DRAG_LAG: f64 = 0.12;
/// Extra overload passed through when cranking against a slipping drag.
const CRANK_LAG: f64 = 0.28;
/// Chance that the hook shakes loose when a rush starts.
const SHAKE_OFF: f64 = 0.03;
/// How fast the fish strips line when the drag slips, m/s at full overload.
const SLIP_SPEED: f64 = 1.8;
/// Stamina drain per second per (tension / fish pull capacity).
const STAMINA_DRAIN: f64 = 0.07;
/// Seconds the line may stay overloaded before it snaps.
const BREAK_AFTER: f64 = 0.25;
/// Seconds of slack after which the hook may fall out.
const SLACK_GRACE: f64 = 1.5;
/// Distance at which the fish is netted.
pub const LAND_DISTANCE: f64 = 1.0;
/// Tension smoothing time constant, seconds.
const TENSION_TAU: f64 = 0.15;

/// Tackle parameters that matter during a fight.
#[derive(Debug, Clone, Copy)]
pub struct FightTackle {
    pub line_strength_kg: f64,
    pub drag_kg: f64,
    pub reel_speed_mps: f64,
    pub line_length_m: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    Landed,
    Broke,
    Escaped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fight {
    pub weight_kg: f64,
    pub strength: f64,
    pub distance_m: f64,
    pub tension_kg: f64,
    pub stamina: f64,
    pub reeling: bool,
    pub slipping: bool,
    /// Sideways offset of the fish from the cast line, metres.
    pub lateral_m: f64,
    lateral_vel: f64,
    rush_t: f64,
    rush_mult: f64,
    slack_t: f64,
    over_t: f64,
}

impl Fight {
    pub fn new(weight_kg: f64, strength: f64, distance_m: f64) -> Self {
        Self {
            weight_kg,
            strength,
            distance_m,
            tension_kg: 0.0,
            stamina: 1.0,
            reeling: false,
            slipping: false,
            lateral_m: 0.0,
            lateral_vel: 0.0,
            // Fish always bolts right after the hook-set.
            rush_t: 1.0,
            rush_mult: 1.4,
            slack_t: 0.0,
            over_t: 0.0,
        }
    }

    /// Current pull of the fish, kg.
    fn pull(&self) -> f64 {
        let fatigue = 0.2 + 0.8 * self.stamina;
        let rush = if self.rush_t > 0.0 {
            self.rush_mult
        } else {
            1.0
        };
        self.weight_kg * self.strength * fatigue * rush
    }

    pub fn step(&mut self, dt: f64, t: &FightTackle, rng: &mut Rng) -> Outcome {
        // Rushes: more frequent while the fish is fresh.
        if self.rush_t > 0.0 {
            self.rush_t -= dt;
        } else if rng.chance(dt * 0.35 * self.stamina) {
            self.rush_t = rng.range(0.8, 2.5);
            self.rush_mult = rng.range(1.3, 1.9);
            self.lateral_vel = rng.range(-1.5, 1.5);
            if rng.chance(SHAKE_OFF) {
                return Outcome::Escaped;
            }
        }

        let pull = self.pull();
        let reel_load = if self.reeling {
            0.1 * t.line_strength_kg + 0.15 * self.weight_kg
        } else {
            0.0
        };
        let load = pull + reel_load;

        // The drag gives line when the load exceeds it, but only the fish's
        // own pull can take line back out.
        self.slipping = load > t.drag_kg;
        let target = if self.slipping {
            // Cranking while the drag gives line overloads it: ease off on rushes.
            let lag = if self.reeling {
                DRAG_LAG + CRANK_LAG
            } else {
                DRAG_LAG
            };
            t.drag_kg + (load - t.drag_kg) * lag
        } else {
            load
        };
        self.tension_kg += (target - self.tension_kg) * (1.0 - (-dt / TENSION_TAU).exp());

        if pull > t.drag_kg {
            self.distance_m += SLIP_SPEED * (pull - t.drag_kg) / pull * dt;
        } else if self.reeling {
            let effective = ((t.drag_kg - pull) / reel_load).min(1.0);
            self.distance_m -= t.reel_speed_mps * effective * dt;
        }
        self.distance_m = self.distance_m.max(0.0);

        // Sideways wandering, bounded by distance.
        self.lateral_vel += rng.normal() * 0.6 * dt - self.lateral_vel * 0.5 * dt;
        self.lateral_m += self.lateral_vel * dt * (0.3 + self.stamina);
        let max_lat = self.distance_m * 0.35;
        self.lateral_m = self.lateral_m.clamp(-max_lat, max_lat);

        // Fatigue.
        let capacity = (self.weight_kg * self.strength).max(0.05);
        self.stamina -= STAMINA_DRAIN * (self.tension_kg / capacity).min(2.0) * dt;
        if self.tension_kg < 0.2 * capacity {
            self.stamina += 0.02 * dt;
        }
        self.stamina = self.stamina.clamp(0.0, 1.0);

        // Failure conditions.
        if self.tension_kg > t.line_strength_kg {
            self.over_t += dt;
            if self.over_t > BREAK_AFTER {
                return Outcome::Broke;
            }
        } else {
            self.over_t = (self.over_t - dt).max(0.0);
        }
        if self.distance_m > t.line_length_m {
            return Outcome::Broke;
        }
        if self.tension_kg < 0.03 * t.line_strength_kg + 0.02 {
            self.slack_t += dt;
            if self.slack_t > SLACK_GRACE && rng.chance(0.6 * dt) {
                return Outcome::Escaped;
            }
        } else {
            self.slack_t = 0.0;
        }

        if self.distance_m <= LAND_DISTANCE {
            return Outcome::Landed;
        }
        Outcome::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 0.05;

    fn tackle(drag: f64) -> FightTackle {
        FightTackle {
            line_strength_kg: 3.0,
            drag_kg: drag,
            reel_speed_mps: 1.0,
            line_length_m: 100.0,
        }
    }

    /// Play the fish; return outcome and elapsed seconds. `smart` eases off
    /// while the drag slips or the line is near its limit.
    fn play(fight: &mut Fight, t: &FightTackle, seed: u64, smart: bool) -> (Outcome, f64) {
        let mut rng = Rng::new(seed);
        for i in 0..20_000 {
            fight.reeling =
                !smart || (!fight.slipping && fight.tension_kg < 0.85 * t.line_strength_kg);
            let o = fight.step(DT, t, &mut rng);
            if o != Outcome::Continue {
                return (o, f64::from(i) * DT);
            }
        }
        (Outcome::Continue, f64::INFINITY)
    }

    fn run(fight: &mut Fight, t: &FightTackle, seed: u64) -> (Outcome, f64) {
        play(fight, t, seed, false)
    }

    #[test]
    fn small_fish_is_landed_quickly() {
        let mut landed = 0;
        for seed in 0..40 {
            let mut f = Fight::new(0.2, 2.0, 15.0);
            let (o, secs) = run(&mut f, &tackle(1.8), seed);
            // A few shake off at the start of a rush; that's intended.
            if o == Outcome::Landed {
                landed += 1;
                assert!(secs < 40.0, "seed {seed}: {secs}s");
            } else {
                assert_eq!(o, Outcome::Escaped, "seed {seed}");
            }
        }
        assert!(landed >= 34, "{landed}/40");
    }

    #[test]
    fn big_fish_on_tight_drag_breaks_line() {
        let broke = (0..20)
            .filter(|&seed| {
                let mut f = Fight::new(4.0, 1.8, 20.0);
                run(&mut f, &tackle(3.5), seed).0 == Outcome::Broke
            })
            .count();
        assert!(broke >= 18, "only {broke}/20 broke");
    }

    #[test]
    fn big_fish_on_loose_drag_can_be_landed_with_care() {
        let landed = (0..20)
            .filter(|&seed| {
                let mut f = Fight::new(2.0, 1.8, 20.0);
                play(&mut f, &tackle(1.4), seed, true).0 == Outcome::Landed
            })
            .count();
        assert!(landed >= 15, "only {landed}/20 landed");
    }

    #[test]
    fn cranking_through_rushes_breaks_more_lines() {
        // A typical carp on the starter feeder (4 kg line, 2.4 kg drag).
        let feeder = FightTackle {
            line_strength_kg: 4.0,
            drag_kg: 2.4,
            reel_speed_mps: 1.2,
            line_length_m: 150.0,
        };
        let broke = |smart: bool| {
            (0..40)
                .filter(|&seed| {
                    let mut f = Fight::new(2.2, 2.2, 20.0);
                    play(&mut f, &feeder, seed, smart).0 == Outcome::Broke
                })
                .count()
        };
        let (dumb, smart) = (broke(false), broke(true));
        assert!(dumb >= 30 && smart <= 4, "dumb {dumb}/40, smart {smart}/40");
    }

    #[test]
    fn slack_line_eventually_loses_fish() {
        let mut rng = Rng::new(1);
        let mut f = Fight::new(0.2, 2.0, 15.0);
        f.stamina = 0.0;
        f.rush_t = 0.0;
        let t = tackle(1.8);
        let escaped = (0..2000).any(|_| f.step(DT, &t, &mut rng) == Outcome::Escaped);
        assert!(escaped);
    }
}
