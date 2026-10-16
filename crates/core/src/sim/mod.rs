//! Game world: fixed-step simulation of a fishing session.

pub mod bite;
pub mod fight;
pub mod groundbait;
pub mod rod;
pub mod spin;
mod view;

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::clock::Clock;
use crate::content::{BaitKind, Content, SpotDef, TackleKind, View, ZoneDef};
use crate::map::WaterMap;
use crate::profile::{Place, Profile, SaleReport};
use crate::rng::Rng;
use crate::weather::{Weather, WeatherState};
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
    Cast {
        rod: usize,
        x: f64,
        y: f64,
    },
    Strike {
        rod: usize,
    },
    Reel {
        rod: usize,
        on: bool,
    },
    Retrieve {
        rod: usize,
    },
    SetDrag {
        rod: usize,
        kg: f64,
    },
    SetFloatDepth {
        rod: usize,
        m: f64,
    },
    SetRetrieveSpeed {
        rod: usize,
        mps: f64,
    },
    SetBait {
        rod: usize,
        bait: String,
    },
    SelectRod {
        rod: usize,
    },
    KeepCatch {
        rod: usize,
    },
    ReleaseCatch {
        rod: usize,
    },
    FastForward {
        on: bool,
    },
    /// Stop the clock and the rods (pause menu).
    Pause {
        on: bool,
    },
    /// Developer cheat (only with the debug overlay on): set hour and/or weather.
    Debug {
        hour: Option<f64>,
        weather: Option<Weather>,
    },
}

/// Something noteworthy that happened during a tick (sounds, toasts).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
#[ts(export)]
pub enum GameEvent {
    CastLanded {
        rod: usize,
    },
    CastOnLand {
        rod: usize,
    },
    Bite {
        rod: usize,
    },
    Hooked {
        rod: usize,
    },
    Missed {
        rod: usize,
    },
    FishLeft {
        rod: usize,
    },
    LineBroke {
        rod: usize,
    },
    Escaped {
        rod: usize,
    },
    Landed {
        rod: usize,
        catch: Catch,
    },
    Snagged {
        rod: usize,
    },
    Unsnagged {
        rod: usize,
    },
    /// A cast needs something the player has run out of (display name).
    OutOf {
        rod: usize,
        item: String,
    },
    WeatherChanged {
        weather: Weather,
    },
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
    #[error("`{0}` does not fit this rod")]
    WrongBait(String),
    #[error("debug commands need the debug overlay")]
    DebugOff,
    #[error("no game loaded")]
    NoProfile,
    #[error("no rod is assembled")]
    NoTackle,
    #[error("`{0}` is not in the backpack")]
    NotOwned(String),
}

/// A fishing session at one spot.
pub struct Session {
    pub location_id: String,
    pub spot: SpotDef,
    pub map: WaterMap,
    pub clock: Clock,
    pub rods: Vec<Rod>,
    /// Profile slot of each rod.
    pub slots: Vec<usize>,
    pub active: usize,
    pub keepnet: Vec<Catch>,
    pub groundbait: Vec<groundbait::Patch>,
    pub weather: WeatherState,
    pub paused: bool,
}

/// Authoritative game state, advanced in fixed steps.
pub struct World {
    content: Arc<Content>,
    rng: Rng,
    tick: u64,
    next_catch_id: u64,
    session: Option<Session>,
    /// Loaded game. Without one (tests, headless tools) consumables are unlimited.
    profile: Option<Profile>,
    /// The profile changed since the last [`World::take_save`].
    dirty: bool,
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
            profile: None,
            dirty: false,
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

    /// Start a session with explicit tackle (tests and headless tools).
    pub fn start_session(
        &mut self,
        location_id: &str,
        spot_id: &str,
        map: WaterMap,
        rods: Vec<Tackle>,
    ) -> Result<(), SimError> {
        let slots = (0..rods.len()).collect();
        self.open_session(location_id, spot_id, map, rods, slots, Clock::default())
    }

    fn open_session(
        &mut self,
        location_id: &str,
        spot_id: &str,
        map: WaterMap,
        rods: Vec<Tackle>,
        slots: Vec<usize>,
        clock: Clock,
    ) -> Result<(), SimError> {
        let loc = self
            .content
            .location(location_id)
            .ok_or_else(|| SimError::UnknownLocation(location_id.into()))?;
        let spot = loc
            .spot(spot_id)
            .ok_or_else(|| SimError::UnknownSpot(spot_id.into()))?
            .clone();
        let weather = WeatherState::new(clock.day(), clock.hour(), &mut self.rng);
        self.session = Some(Session {
            location_id: location_id.into(),
            spot,
            map,
            clock,
            weather,
            rods: rods.into_iter().map(Rod::new).collect(),
            slots,
            active: 0,
            keepnet: Vec::new(),
            groundbait: Vec::new(),
            paused: false,
        });
        Ok(())
    }

    pub fn profile(&self) -> Option<&Profile> {
        self.profile.as_ref()
    }

    /// Load (or unload) a game. Any running session is dropped.
    pub fn set_profile(&mut self, profile: Option<Profile>) {
        self.session = None;
        self.profile = profile;
        self.dirty = self.profile.is_some();
    }

    /// Change the profile between sessions (shop, tackle).
    pub fn update_profile<R>(
        &mut self,
        f: impl FnOnce(&mut Profile, &Content) -> R,
    ) -> Result<R, SimError> {
        let p = self.profile.as_mut().ok_or(SimError::NoProfile)?;
        self.dirty = true;
        Ok(f(p, &self.content))
    }

    /// Go fishing with the rods assembled in the profile's slots.
    pub fn go_fishing(
        &mut self,
        location_id: &str,
        spot_id: &str,
        map: WaterMap,
    ) -> Result<(), SimError> {
        let p = self.profile.as_ref().ok_or(SimError::NoProfile)?;
        let (slots, rods): (Vec<usize>, Vec<Tackle>) = (0..p.slots.len())
            .filter_map(|i| p.tackle_for(&self.content, i).map(|t| (i, t)))
            .unzip();
        if rods.is_empty() {
            return Err(SimError::NoTackle);
        }
        let clock = Clock {
            minutes: p.clock_min,
            ..Clock::default()
        };
        let (keepnet, next_id) = (p.keepnet.clone(), p.next_catch_id);
        self.open_session(location_id, spot_id, map, rods, slots, clock)?;
        self.session.as_mut().expect("just opened").keepnet = keepnet;
        self.next_catch_id = next_id;
        if let Some(p) = &mut self.profile {
            p.at = Some(Place {
                location: location_id.into(),
                spot: spot_id.into(),
            });
        }
        self.dirty = true;
        Ok(())
    }

    /// Copy the live session state (keepnet, clock, rod settings) into the profile.
    pub fn sync_profile(&mut self) {
        let (Some(s), Some(p)) = (&self.session, &mut self.profile) else {
            return;
        };
        p.keepnet.clone_from(&s.keepnet);
        p.clock_min = s.clock.minutes;
        p.next_catch_id = self.next_catch_id;
        for (r, &i) in s.rods.iter().zip(&s.slots) {
            let Some(slot) = p.slots.get_mut(i) else {
                continue;
            };
            let t = &r.tackle;
            slot.bait = (!t.bait.is_empty()).then(|| t.bait.clone());
            slot.float_depth_m = t.float_depth_m;
            slot.drag_kg = t.drag_kg;
            slot.retrieve_mps = t.retrieve_mps;
        }
    }

    /// Leave the water: fish still in the landing net go to the keepnet,
    /// and the keepnet is sold.
    pub fn leave_water(&mut self) -> Result<SaleReport, SimError> {
        if self.profile.is_none() {
            return Err(SimError::NoProfile);
        }
        let s = self.session.as_mut().ok_or(SimError::NoSession)?;
        for r in &mut s.rods {
            if let RodState::Landed { catch } = &r.state {
                s.keepnet.push(catch.clone());
                r.state = RodState::Idle;
            }
        }
        self.sync_profile();
        self.session = None;
        let p = self.profile.as_mut().expect("checked above");
        p.at = None;
        self.dirty = true;
        Ok(p.sell_keepnet(&self.content))
    }

    /// The profile to save, if it changed since the last call.
    pub fn take_save(&mut self) -> Option<Profile> {
        if !self.dirty {
            return None;
        }
        self.sync_profile();
        self.dirty = false;
        self.profile.clone()
    }

    pub fn end_session(&mut self) -> Option<Session> {
        self.session.take()
    }

    /// Apply a player command immediately.
    pub fn input(&mut self, input: Input) -> Result<(), SimError> {
        let from = self.events.len();
        let res = self.apply(input);
        self.settle(from);
        res
    }

    fn apply(&mut self, input: Input) -> Result<(), SimError> {
        let s = self.session.as_mut().ok_or(SimError::NoSession)?;
        let rod_idx = match &input {
            Input::Cast { rod, .. }
            | Input::Strike { rod }
            | Input::Reel { rod, .. }
            | Input::Retrieve { rod }
            | Input::SetDrag { rod, .. }
            | Input::SetFloatDepth { rod, .. }
            | Input::SetRetrieveSpeed { rod, .. }
            | Input::SetBait { rod, .. }
            | Input::SelectRod { rod }
            | Input::KeepCatch { rod }
            | Input::ReleaseCatch { rod } => Some(*rod),
            Input::FastForward { .. } | Input::Pause { .. } | Input::Debug { .. } => None,
        };
        if let Some(i) = rod_idx
            && i >= s.rods.len()
        {
            return Err(SimError::NoRod(i));
        }

        match input {
            Input::Cast { rod, x, y } => {
                if let Some(p) = &self.profile
                    && let Some(item) = p.missing_for_cast(&self.content, &s.rods[rod].tackle)
                {
                    self.events.push(GameEvent::OutOf { rod, item });
                    return Ok(());
                }
                cast(s, rod, Point { x, y }, &mut self.rng);
            }
            Input::Strike { rod } => strike(&self.content, s, rod, &mut self.rng, &mut self.events),
            Input::Reel { rod, on } => match &mut s.rods[rod].state {
                RodState::Fighting { fight, .. } => fight.reeling = on,
                RodState::Lure { run, .. } => run.reeling = on,
                _ => {}
            },
            Input::Retrieve { rod } => {
                let view = s.spot.view;
                let r = &mut s.rods[rod];
                if let RodState::Snagged { run } = &r.state {
                    if self.rng.chance(spin::FREE_CHANCE) {
                        let mut run = run.clone();
                        run.reeling = false;
                        r.state = RodState::Lure { run, calm: 2.0 };
                        self.events.push(GameEvent::Unsnagged { rod });
                    } else {
                        r.state = RodState::Idle;
                        self.events.push(GameEvent::LineBroke { rod });
                    }
                } else {
                    retrieve(r, &view);
                }
            }
            Input::SetRetrieveSpeed { rod, mps } => {
                s.rods[rod].tackle.retrieve_mps = mps.clamp(0.1, 1.8);
            }
            Input::SetDrag { rod, kg } => {
                let t = &mut s.rods[rod].tackle;
                t.drag_kg = kg.clamp(0.1, t.max_drag_kg);
            }
            Input::SetFloatDepth { rod, m } => {
                let r = &mut s.rods[rod];
                r.tackle.float_depth_m = (m / 0.05).round() * 0.05;
                r.tackle.float_depth_m = r.tackle.float_depth_m.clamp(0.1, 12.0);
                // Changing depth means pulling the rig out.
                retrieve(r, &s.spot.view);
            }
            Input::SetBait { rod, bait } => {
                let Some(def) = self.content.bait(&bait) else {
                    return Err(SimError::UnknownBait(bait));
                };
                let r = &mut s.rods[rod];
                if (def.kind == BaitKind::Lure) != (r.tackle.kind == RodKind::Spinning) {
                    return Err(SimError::WrongBait(bait));
                }
                if self.profile.as_ref().is_some_and(|p| p.count(&bait) == 0) {
                    return Err(SimError::NotOwned(bait));
                }
                r.tackle.bait = bait;
                retrieve(r, &s.spot.view);
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
            Input::Pause { on } => {
                s.paused = on;
                for r in &mut s.rods {
                    match &mut r.state {
                        RodState::Fighting { fight, .. } => fight.reeling = false,
                        RodState::Lure { run, .. } => run.reeling = false,
                        _ => {}
                    }
                }
            }
            Input::Debug { hour, weather } => {
                if !self.debug {
                    return Err(SimError::DebugOff);
                }
                if let Some(h) = hour {
                    let day_start = f64::from(s.clock.day() - 1) * 24.0 * 60.0;
                    s.clock.minutes = day_start + h.rem_euclid(24.0) * 60.0;
                }
                // Re-anchor the weather to the new time so it doesn't re-roll at once.
                let w = weather.unwrap_or(s.weather.current);
                s.weather.set(w, s.clock.day(), s.clock.hour());
                if weather.is_some() {
                    self.events.push(GameEvent::WeatherChanged { weather: w });
                }
            }
        }
        Ok(())
    }

    /// Advance the simulation by one fixed tick.
    pub fn step(&mut self) {
        self.tick += 1;
        let from = self.events.len();
        self.step_session();
        self.settle(from);
    }

    fn step_session(&mut self) {
        let Some(s) = self.session.as_mut().filter(|s| !s.paused) else {
            return;
        };
        // Clock, keepnet and rods change every tick on the water.
        self.dirty |= self.profile.is_some();
        let dt = TICK_DT;
        s.clock.advance(dt);
        groundbait::decay(&mut s.groundbait, dt);
        let hour = s.clock.hour();
        if s.weather.update(s.clock.day(), hour, &mut self.rng) {
            self.events.push(GameEvent::WeatherChanged {
                weather: s.weather.current,
            });
        }
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

    /// Side effects of events from `from` on: groundbait from feeder casts and
    /// consumables used up (only with a profile; otherwise they are unlimited).
    fn settle(&mut self, from: usize) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let content = &*self.content;
        let mut profile = self.profile.as_mut();
        for e in &self.events[from..] {
            let (rod, chance) = match e {
                GameEvent::CastLanded { rod } => {
                    let r = &s.rods[*rod];
                    if let (RodKind::Feeder, RodState::Waiting { at, .. }) =
                        (r.tackle.kind, &r.state)
                        && profile
                            .as_mut()
                            .is_none_or(|p| p.take_kind(content, TackleKind::Groundbait))
                    {
                        groundbait::add(&mut s.groundbait, &s.spot.view, *at, groundbait::PER_CAST);
                    }
                    continue;
                }
                GameEvent::LineBroke { rod } => {
                    if let Some(p) = profile.as_mut() {
                        p.lose_rig(content, &s.rods[*rod].tackle);
                    }
                    continue;
                }
                GameEvent::Landed { rod, .. } | GameEvent::Escaped { rod } => (*rod, 1.0),
                GameEvent::FishLeft { rod } => (*rod, BAIT_LOST_FISH_LEFT),
                GameEvent::Missed { rod } => (*rod, BAIT_LOST_MISSED),
                _ => continue,
            };
            if let Some(p) = profile.as_mut()
                && self.rng.chance(chance)
            {
                p.use_bait(content, &s.rods[rod].tackle);
            }
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
        RodState::Bite {
            at,
            cell,
            bite,
            lure,
        } => {
            if rng.chance(bite.phase().hook_chance()) {
                r.state = hooked_state(content, &view, *at, bite.fish.clone(), false);
                events.push(GameEvent::Hooked { rod });
            } else {
                r.state = match lure {
                    Some(run) => RodState::Lure {
                        run: run.clone(),
                        calm: 2.0,
                    },
                    None => RodState::Waiting {
                        at: *at,
                        cell: *cell,
                        calm: 4.0,
                    },
                };
                events.push(GameEvent::Missed { rod });
            }
        }
        RodState::Waiting { .. } => retrieve(r, &view),
        _ => {}
    }
}

/// Start fighting a fish that took the bait at `at`.
fn hooked_state(
    content: &Content,
    view: &View,
    at: Point,
    fish: Hooked,
    reeling: bool,
) -> RodState {
    let strength = content.fish(&fish.fish_id).map_or(1.0, |f| f.strength);
    let mut fight = fight::Fight::new(fish.weight_kg, strength, view.distance_at(at.y));
    fight.reeling = reeling;
    RodState::Fighting {
        origin: at,
        fight,
        fish,
    }
}

/// Roll for a bite this tick; on success pick the species, weight and bite script.
fn roll_bite(
    content: &Content,
    zone: &ZoneDef,
    p: &Presentation,
    dt: f64,
    rng: &mut Rng,
) -> Option<BiteState> {
    let rates = bite::rates(content, zone, p);
    let total: f64 = rates.iter().map(|(_, r)| r).sum();
    if total <= 0.0 || !rng.chance(total * dt) {
        return None;
    }
    let weights: Vec<f64> = rates.iter().map(|(_, r)| *r).collect();
    let fish = rates[rng.weighted(&weights)?].0;
    Some(BiteState {
        fish: Hooked {
            fish_id: fish.id.clone(),
            weight_kg: bite::roll_weight(fish, rng),
        },
        script: bite::bite_script(fish.bite, rng),
        idx: 0,
        t: 0.0,
    })
}

/// Chance that a fish that nibbled and left took the bait with it.
const BAIT_LOST_FISH_LEFT: f64 = 0.5;
/// Chance that a missed strike tore the bait off the hook.
const BAIT_LOST_MISSED: f64 = 0.3;

/// Chance that a predator hitting a lure on the retrieve hooks itself.
const SELF_HOOK: f64 = 0.6;

fn retrieve(r: &mut Rod, view: &View) {
    let from = match &r.state {
        RodState::Waiting { at, .. } | RodState::Bite { at, .. } => *at,
        RodState::Casting { target, .. } => *target,
        RodState::Lure { run, .. } => run.point(view),
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
                    Some(_) if r.tackle.kind == RodKind::Spinning => {
                        r.state = RodState::Lure {
                            run: spin::LureRun::new(at, &s.spot.view),
                            calm: 1.0,
                        };
                        events.push(GameEvent::CastLanded { rod: i });
                    }
                    Some(cell) => {
                        r.state = RodState::Waiting {
                            at,
                            cell,
                            calm: CALM_AFTER_CAST,
                        };
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
                weather: s.weather.current,
                bonus: groundbait::bonus(&s.groundbait, &s.spot.view, *at),
            };
            let Some(bite) = roll_bite(content, zone, &p, dt, rng) else {
                return;
            };
            r.state = RodState::Bite {
                at: *at,
                cell: *cell,
                bite,
                lure: None,
            };
            events.push(GameEvent::Bite { rod: i });
        }
        RodState::Bite {
            at,
            cell,
            bite,
            lure,
        } => {
            bite.t += dt;
            if bite.t >= bite.script[bite.idx].1 {
                bite.t = 0.0;
                bite.idx += 1;
                if bite.idx >= bite.script.len() {
                    r.state = match lure.take() {
                        Some(run) => RodState::Lure { run, calm: 1.5 },
                        None => RodState::Waiting {
                            at: *at,
                            cell: *cell,
                            calm: 2.0,
                        },
                    };
                    events.push(GameEvent::FishLeft { rod: i });
                }
            }
        }
        RodState::Snagged { .. } => {}
        RodState::Lure { run, calm } => {
            let view = s.spot.view;
            let lure = content.bait(&r.tackle.bait).and_then(|b| b.lure);
            let Some(lure) = lure else {
                r.state = RodState::Idle;
                return;
            };
            if !run.advance(&lure, r.tackle.retrieve_mps, dt) {
                r.state = RodState::Idle;
                return;
            }
            let at = run.point(&view);
            let Some(cell) = s.map.sample(at.x, at.y) else {
                r.state = RodState::Idle;
                return;
            };
            run.clamp_to(&cell);
            if rng.chance(run.snag_rate(&cell) * dt) {
                r.state = RodState::Snagged { run: run.clone() };
                events.push(GameEvent::Snagged { rod: i });
                return;
            }
            if *calm > 0.0 {
                *calm -= dt;
                return;
            }
            let Some(zone) = s.spot.zone(cell.zone) else {
                return;
            };
            let p = Presentation {
                cell,
                bait_depth_m: run.depth_m,
                bait: &r.tackle.bait,
                hour,
                weather: s.weather.current,
                bonus: run.attraction(&lure, r.tackle.retrieve_mps),
            };
            let Some(bite) = roll_bite(content, zone, &p, dt, rng) else {
                return;
            };
            events.push(GameEvent::Bite { rod: i });
            // A predator hitting a moving lure often hooks itself.
            if run.reeling && rng.chance(SELF_HOOK) {
                r.state = hooked_state(content, &view, at, bite.fish, true);
                events.push(GameEvent::Hooked { rod: i });
            } else {
                r.state = RodState::Bite {
                    at,
                    cell,
                    bite,
                    lure: Some(run.clone()),
                };
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
