//! Serializable views of the world streamed to the UI.

use serde::Serialize;
use ts_rs::TS;

use super::bite::{self, BitePhase, Presentation};
use super::rod::{Point, Rod, RodKind, RodState};
use super::{Catch, GameEvent, Session, World, fight_point};
use crate::content::View;

/// State streamed to the UI every tick.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Snapshot {
    #[ts(type = "number")]
    pub tick: u64,
    pub session: Option<SessionView>,
    pub events: Vec<GameEvent>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionView {
    pub location_id: String,
    pub spot_id: String,
    pub spot_name: String,
    pub background: String,
    pub map: String,
    pub view: View,
    pub clock: ClockView,
    pub active_rod: usize,
    pub rods: Vec<RodView>,
    pub keepnet: Vec<Catch>,
    pub groundbait: Vec<super::groundbait::Patch>,
    pub debug: Option<DebugView>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ClockView {
    pub day: u32,
    /// Fractional hour `0..24`.
    pub hour: f64,
    pub fast_forward: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RodView {
    pub kind: RodKind,
    pub bait: String,
    pub bait_name: String,
    pub float_depth_m: f64,
    pub drag_kg: f64,
    pub line_strength_kg: f64,
    pub max_cast_m: f64,
    pub state: RodStateView,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "state", rename_all_fields = "camelCase")]
#[ts(export)]
pub enum RodStateView {
    Idle,
    Casting {
        to: Point,
        progress: f64,
    },
    Waiting {
        at: Point,
        depth_m: f64,
        lying: bool,
    },
    Bite {
        at: Point,
        depth_m: f64,
        lying: bool,
        phase: BitePhase,
        /// Seconds into the current phase.
        phase_t: f64,
    },
    Retrieving {
        from: Point,
        progress: f64,
    },
    Fighting {
        at: Point,
        distance_m: f64,
        /// Tension relative to line strength (1.0 = breaking point).
        tension: f64,
        slipping: bool,
        reeling: bool,
    },
    Landed {
        catch: Catch,
    },
}

/// Developer overlay data (F3).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DebugView {
    /// Per rod: `(fish name, bites per minute)`.
    pub bite_rates: Vec<Vec<(String, f64)>>,
}

impl World {
    /// Build the snapshot for this tick and drain pending events.
    pub fn snapshot(&mut self) -> Snapshot {
        let events = self.drain_events();
        Snapshot {
            tick: self.tick,
            session: self.session.as_ref().map(|s| self.session_view(s)),
            events,
        }
    }

    fn session_view(&self, s: &Session) -> SessionView {
        SessionView {
            location_id: s.location_id.clone(),
            spot_id: s.spot.id.clone(),
            spot_name: s.spot.name.clone(),
            background: s.spot.background.clone(),
            map: s.spot.map.clone(),
            view: s.spot.view,
            clock: ClockView {
                day: s.clock.day(),
                hour: s.clock.hour(),
                fast_forward: s.clock.fast_forward,
            },
            active_rod: s.active,
            rods: s.rods.iter().map(|r| self.rod_view(s, r)).collect(),
            keepnet: s.keepnet.clone(),
            groundbait: s.groundbait.clone(),
            debug: self.debug.then(|| DebugView {
                bite_rates: s.rods.iter().map(|r| self.debug_rates(s, r)).collect(),
            }),
        }
    }

    fn rod_view(&self, s: &Session, r: &Rod) -> RodView {
        let t = &r.tackle;
        let state = match &r.state {
            RodState::Idle => RodStateView::Idle,
            RodState::Casting { target, t, dur } => RodStateView::Casting {
                to: *target,
                progress: (t / dur).min(1.0),
            },
            RodState::Waiting { at, cell, .. } => {
                let (_, lying) = r.tackle.bait_depth(cell);
                RodStateView::Waiting {
                    at: *at,
                    depth_m: cell.depth_m,
                    lying,
                }
            }
            RodState::Bite { at, cell, bite } => {
                let (_, lying) = r.tackle.bait_depth(cell);
                RodStateView::Bite {
                    at: *at,
                    depth_m: cell.depth_m,
                    lying,
                    phase: bite.phase(),
                    phase_t: bite.t,
                }
            }
            RodState::Retrieving { from, t, dur } => RodStateView::Retrieving {
                from: *from,
                progress: (t / dur).min(1.0),
            },
            RodState::Fighting { origin, fight, .. } => RodStateView::Fighting {
                at: fight_point(&s.spot.view, *origin, fight),
                distance_m: fight.distance_m,
                tension: fight.tension_kg / t.line_strength_kg,
                slipping: fight.slipping,
                reeling: fight.reeling,
            },
            RodState::Landed { catch } => RodStateView::Landed {
                catch: catch.clone(),
            },
        };
        RodView {
            kind: t.kind,
            bait: t.bait.clone(),
            bait_name: self
                .content
                .bait(&t.bait)
                .map(|b| b.name.clone())
                .unwrap_or_default(),
            float_depth_m: t.float_depth_m,
            drag_kg: t.drag_kg,
            line_strength_kg: t.line_strength_kg,
            max_cast_m: t.max_cast_m,
            state,
        }
    }

    fn debug_rates(&self, s: &Session, r: &Rod) -> Vec<(String, f64)> {
        let (RodState::Waiting { at, cell, .. } | RodState::Bite { at, cell, .. }) = &r.state
        else {
            return Vec::new();
        };
        let Some(zone) = s.spot.zone(cell.zone) else {
            return Vec::new();
        };
        let (bait_depth_m, _) = r.tackle.bait_depth(cell);
        let p = Presentation {
            cell: *cell,
            bait_depth_m,
            bait: &r.tackle.bait,
            hour: s.clock.hour(),
            bonus: super::groundbait::bonus(&s.groundbait, &s.spot.view, *at),
        };
        bite::rates(&self.content, zone, &p)
            .into_iter()
            .map(|(f, rate)| (f.name.clone(), rate * 60.0))
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BaitInfo {
    pub id: String,
    pub name: String,
    pub kind: crate::content::BaitKind,
}

impl World {
    pub fn baits(&self) -> Vec<BaitInfo> {
        self.content
            .baits
            .iter()
            .map(|b| BaitInfo {
                id: b.id.clone(),
                name: b.name.clone(),
                kind: b.kind,
            })
            .collect()
    }
}
