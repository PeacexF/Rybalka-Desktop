//! Game world: fixed-step simulation of a fishing session.

pub mod bite;
pub mod fight;
pub mod groundbait;
pub mod rod;
mod view;

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::clock::Clock;
use crate::content::{Content, SpotDef};
use crate::map::WaterMap;
use crate::rng::Rng;
use bite::Presentation;
use fight::Outcome;
use rod::{BiteState, Hooked, Point, Rod, RodKind, RodState, Tackle};

pub use view::*;

/// Fixed simulation rate.
pub const TICK_HZ: u32 = 20;
/// Duration of one simulation tick in seconds.
pub const TICK_DT: f64 = 1.0 / TICK_HZ as f64;

/// Horizontal field of view: at distance `d` the image is `FOV_K * d` metres wide.
pub(crate) const FOV_K: f64 = 1.2;
/// Quiet period after a cast before fish may bite, seconds.
const CALM_AFTER_CAST: f64 = 3.0;

/// A landed fish.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Catch {
    #[ts(type = "number")]
    pub id: u64,
    pub fish_id: String,
    pub name: String,
    pub weight_kg: f64,
    pub trophy: bool,
    pub bait: String,
    /// Game minutes when landed.
    pub at_min: f64,
}

/// Player commands. Rod indexes refer to [`SessionView::rods`].
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
#[ts(export)]
pub enum Input {
    Cast { rod: usize, x: f64, y: f64 },
    Strike { rod: usize },
    Reel { rod: usize, on: bool },
    Retrieve { rod: usize },
    SetDrag { rod: usize, kg: f64 },
    SetFloatDepth { rod: usize, m: f64 },
    SetBait { rod: usize, bait: String },
    SelectRod { rod: usize },
    KeepCatch { rod: usize },
    ReleaseCatch { rod: usize },
    FastForward { on: bool },
}

/// Something noteworthy that happened during a tick (sounds, toasts).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
#[ts(export)]
pub enum GameEvent {
    CastLanded { rod: usize },
    CastOnLand { rod: usize },
    Bite { rod: usize },
    Hooked { rod: usize },
    Missed { rod: usize },
    FishLeft { rod: usize },
    LineBroke { rod: usize },
    Escaped { rod: usize },
    Landed { rod: usize, catch: Catch },
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SimError {
    #[error("no active session")]
    NoSession,
    #[error("unknown location `{0}`")]
    UnknownLocation(String),
    #[error("unknown spot `{0}`")]
    UnknownSpot(String),
    #[error("no rod in slot {0}")]
    NoRod(usize),
    #[error("unknown bait `{0}`")]
    UnknownBait(String),
}

/// A fishing session at one spot.
pub struct Session {
    pub location_id: String,
    pub spot: SpotDef,
    pub map: WaterMap,
    pub clock: Clock,
    pub rods: Vec<Rod>,
    pub active: usize,
    pub keepnet: Vec<Catch>,
    pub groundbait: Vec<groundbait::Patch>,
}

/// Authoritative game state, advanced in fixed steps.
pub struct World {
    content: Arc<Content>,
    rng: Rng,
    tick: u64,
    next_catch_id: u64,
    session: Option<Session>,
    events: Vec<GameEvent>,
    pub debug: bool,
}

impl World {
    pub fn new(content: Arc<Content>, seed: u64) -> Self {
        Self {
            content,
            rng: Rng::new(seed),
            tick: 0,
            next_catch_id: 1,
            session: None,
            events: Vec::new(),
            debug: false,
        }
    }

    pub fn content(&self) -> &Content {
        &self.content
    }

    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    pub fn start_session(
        &mut self,
        location_id: &str,
        spot_id: &str,
        map: WaterMap,
        rods: Vec<Tackle>,
    ) -> Result<(), SimError> {
        let loc = self
            .content
            .location(location_id)
            .ok_or_else(|| SimError::UnknownLocation(location_id.into()))?;
        let spot = loc
            .spot(spot_id)
            .ok_or_else(|| SimError::UnknownSpot(spot_id.into()))?
            .clone();
        self.session = Some(Session {
            location_id: location_id.into(),
            spot,
            map,
            clock: Clock::default(),
            rods: rods.into_iter().map(Rod::new).collect(),
            active: 0,
            keepnet: Vec::new(),
            groundbait: Vec::new(),
        });
        Ok(())
    }

    pub fn end_session(&mut self) -> Option<Session> {
        self.session.take()
    }

    /// Apply a player command immediately.
    pub fn input(&mut self, input: Input) -> Result<(), SimError> {
        let s = self.session.as_mut().ok_or(SimError::NoSession)?;
        let rod_idx = match &input {
            Input::Cast { rod, .. }
            | Input::Strike { rod }
            | Input::Reel { rod, .. }
            | Input::Retrieve { rod }
            | Input::SetDrag { rod, .. }
            | Input::SetFloatDepth { rod, .. }
            | Input::SetBait { rod, .. }
            | Input::SelectRod { rod }
            | Input::KeepCatch { rod }
            | Input::ReleaseCatch { rod } => Some(*rod),
            Input::FastForward { .. } => None,
        };
        if let Some(i) = rod_idx
            && i >= s.rods.len()
        {
            return Err(SimError::NoRod(i));
        }

        match input {
            Input::Cast { rod, x, y } => cast(s, rod, Point { x, y }, &mut self.rng),
            Input::Strike { rod } => strike(&self.content, s, rod, &mut self.rng, &mut self.events),
            Input::Reel { rod, on } => {
                if let RodState::Fighting { fight, .. } = &mut s.rods[rod].state {
                    fight.reeling = on;
                }
            }
            Input::Retrieve { rod } => retrieve(&mut s.rods[rod]),
            Input::SetDrag { rod, kg } => {
                let t = &mut s.rods[rod].tackle;
                t.drag_kg = kg.clamp(0.1, t.line_strength_kg * 1.3);
            }
            Input::SetFloatDepth { rod, m } => {
                let r = &mut s.rods[rod];
                r.tackle.float_depth_m = (m / 0.05).round() * 0.05;
                r.tackle.float_depth_m = r.tackle.float_depth_m.clamp(0.1, 12.0);
                // Changing depth means pulling the rig out.
                retrieve(r);
            }
            Input::SetBait { rod, bait } => {
                if self.content.bait(&bait).is_none() {
                    return Err(SimError::UnknownBait(bait));
                }
                let r = &mut s.rods[rod];
                r.tackle.bait = bait;
                retrieve(r);
            }
            Input::SelectRod { rod } => s.active = rod,
            Input::KeepCatch { rod } | Input::ReleaseCatch { rod } => {
                let keep = matches!(input, Input::KeepCatch { .. });
                let r = &mut s.rods[rod];
                if let RodState::Landed { catch } = &r.state {
                    if keep {
                        s.keepnet.push(catch.clone());
                    }
                    r.state = RodState::Idle;
                }
            }
            Input::FastForward { on } => s.clock.fast_forward = on,
        }
        Ok(())
    }

    /// Advance the simulation by one fixed tick.
    pub fn step(&mut self) {
        self.tick += 1;
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let dt = TICK_DT;
        s.clock.advance(dt);
        groundbait::decay(&mut s.groundbait, dt);
        let hour = s.clock.hour();
        for i in 0..s.rods.len() {
            step_rod(
                &self.content,
                s,
                i,
                dt,
                hour,
                &mut self.rng,
                &mut self.events,
                &mut self.next_catch_id,
            );
        }
    }

    /// Take events produced since the last call.
    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }
}

fn cast(s: &mut Session, rod: usize, target: Point, rng: &mut Rng) {
    let r = &mut s.rods[rod];
    if !matches!(r.state, RodState::Idle) {
        return;
    }
    let view = s.spot.view;
    let mut p = target;
    // Clamp to max cast distance.
    if view.distance_at(p.y) > r.tackle.max_cast_m {
        p.y = view.y_at(r.tackle.max_cast_m);
    }
    // A little scatter, proportional to distance.
    let d = view.distance_at(p.y);
    let spread_m = 0.03 * d;
    p.x += rng.normal() * spread_m / (FOV_K * d);
    p.y = view.y_at((d + rng.normal() * spread_m).max(view.near_m));
    s.active = rod;
    r.state = RodState::Casting {
        target: p,
        t: 0.0,
        dur: 0.5 + d / 40.0,
    };
}

fn strike(
    content: &Content,
    s: &mut Session,
    rod: usize,
    rng: &mut Rng,
    events: &mut Vec<GameEvent>,
) {
    let view = s.spot.view;
    let r = &mut s.rods[rod];
    match &r.state {
        RodState::Bite { at, cell, bite } => {
            if rng.chance(bite.phase().hook_chance()) {
                let distance = view.distance_at(at.y);
                let fish = bite.fish.clone();
                let strength = content.fish(&fish.fish_id).map_or(1.0, |f| f.strength);
                r.state = RodState::Fighting {
                    origin: *at,
                    fight: fight::Fight::new(fish.weight_kg, strength, distance),
                    fish,
                };
                events.push(GameEvent::Hooked { rod });
            } else {
                r.state = RodState::Waiting {
                    at: *at,
                    cell: *cell,
                    calm: 4.0,
                };
                events.push(GameEvent::Missed { rod });
            }
        }
        RodState::Waiting { .. } => retrieve(r),
        _ => {}
    }
}

fn retrieve(r: &mut Rod) {
    let from = match &r.state {
        RodState::Waiting { at, .. } | RodState::Bite { at, .. } => *at,
        RodState::Casting { target, .. } => *target,
        _ => return,
    };
    r.state = RodState::Retrieving {
        from,
        t: 0.0,
        dur: 0.8,
    };
}

#[allow(clippy::too_many_arguments)]
fn step_rod(
    content: &Content,
    s: &mut Session,
    i: usize,
    dt: f64,
    hour: f64,
    rng: &mut Rng,
    events: &mut Vec<GameEvent>,
    next_catch_id: &mut u64,
) {
    let r = &mut s.rods[i];
    match &mut r.state {
        RodState::Idle | RodState::Landed { .. } => {}
        RodState::Casting { target, t, dur } => {
            *t += dt;
            if *t >= *dur {
                let at = *target;
                match s.map.sample(at.x, at.y) {
                    Some(cell) => {
                        r.state = RodState::Waiting {
                            at,
                            cell,
                            calm: CALM_AFTER_CAST,
                        };
                        if r.tackle.kind == RodKind::Feeder {
                            groundbait::add(
                                &mut s.groundbait,
                                &s.spot.view,
                                at,
                                groundbait::PER_CAST,
                            );
                        }
                        events.push(GameEvent::CastLanded { rod: i });
                    }
                    None => {
                        r.state = RodState::Idle;
                        events.push(GameEvent::CastOnLand { rod: i });
                    }
                }
            }
        }
        RodState::Retrieving { t, dur, .. } => {
            *t += dt;
            if *t >= *dur {
                r.state = RodState::Idle;
            }
        }
        RodState::Waiting { at, cell, calm } => {
            if *calm > 0.0 {
                *calm -= dt;
                return;
            }
            let Some(zone) = s.spot.zone(cell.zone) else {
                return;
            };
            let (bait_depth_m, _) = r.tackle.bait_depth(cell);
            let p = Presentation {
                cell: *cell,
                bait_depth_m,
                bait: &r.tackle.bait,
                hour,
                bonus: groundbait::bonus(&s.groundbait, &s.spot.view, *at),
            };
            let rates = bite::rates(content, zone, &p);
            let total: f64 = rates.iter().map(|(_, r)| r).sum();
            if total <= 0.0 || !rng.chance(total * dt) {
                return;
            }
            let weights: Vec<f64> = rates.iter().map(|(_, r)| *r).collect();
            let Some(k) = rng.weighted(&weights) else {
                return;
            };
            let fish = rates[k].0;
            let bite = BiteState {
                fish: Hooked {
                    fish_id: fish.id.clone(),
                    weight_kg: bite::roll_weight(fish, rng),
                },
                script: bite::bite_script(fish.bite, rng),
                idx: 0,
                t: 0.0,
            };
            r.state = RodState::Bite {
                at: *at,
                cell: *cell,
                bite,
            };
            events.push(GameEvent::Bite { rod: i });
        }
        RodState::Bite { at, cell, bite } => {
            bite.t += dt;
            if bite.t >= bite.script[bite.idx].1 {
                bite.t = 0.0;
                bite.idx += 1;
                if bite.idx >= bite.script.len() {
                    r.state = RodState::Waiting {
                        at: *at,
                        cell: *cell,
                        calm: 2.0,
                    };
                    events.push(GameEvent::FishLeft { rod: i });
                }
            }
        }
        RodState::Fighting { fish, fight, .. } => match fight.step(dt, &r.tackle.fight(), rng) {
            Outcome::Continue => {}
            Outcome::Broke => {
                r.state = RodState::Idle;
                events.push(GameEvent::LineBroke { rod: i });
            }
            Outcome::Escaped => {
                r.state = RodState::Idle;
                events.push(GameEvent::Escaped { rod: i });
            }
            Outcome::Landed => {
                let def = content.fish(&fish.fish_id);
                let catch = Catch {
                    id: *next_catch_id,
                    fish_id: fish.fish_id.clone(),
                    name: def.map(|d| d.name.clone()).unwrap_or_default(),
                    weight_kg: fish.weight_kg,
                    trophy: def.is_some_and(|d| fish.weight_kg >= d.trophy_kg),
                    bait: r.tackle.bait.clone(),
                    at_min: s.clock.minutes,
                };
                *next_catch_id += 1;
                events.push(GameEvent::Landed {
                    rod: i,
                    catch: catch.clone(),
                });
                r.state = RodState::Landed { catch };
            }
        },
    }
}

/// Screen position of a fish being played: along the cast line, offset sideways.
pub(crate) fn fight_point(
    view: &crate::content::View,
    origin: Point,
    fight: &fight::Fight,
) -> Point {
    let d = fight.distance_m.max(view.near_m * 0.6);
    Point {
        x: origin.x + fight.lateral_m / (FOV_K * d),
        y: view.y_at(d),
    }
}

#[cfg(test)]
mod tests;
