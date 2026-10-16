//! Serializable views of the world streamed to the UI.

use std::collections::BTreeMap;

use serde::Serialize;
use ts_rs::TS;

use super::bite::{self, BitePhase, Presentation};
use super::rod::{Point, Rod, RodKind, RodState};
use super::{Catch, GameEvent, Session, World, fight_point};
use crate::content::View;
use crate::profile::{self, Profile, Slot};

/// State streamed to the UI every tick.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Snapshot {
    #[ts(type = "number")]
    pub tick: u64,
    pub session: Option<SessionView>,
    /// The loaded game; `None` in the main menu.
    pub profile: Option<ProfileView>,
    pub events: Vec<GameEvent>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileView {
    pub money: f64,
    pub xp: f64,
    pub rank: u32,
    pub rank_name: String,
    /// Experience at which the current rank starts.
    pub rank_xp: f64,
    /// Experience needed for the next rank (`None` at the top).
    pub next_rank_xp: Option<f64>,
    pub items: BTreeMap<String, u32>,
    /// Slot setup as last synced (stale while fishing).
    pub slots: Vec<Slot>,
    /// Unsold fish.
    pub keepnet: usize,
    pub keepnet_kg: f64,
    pub day: u32,
    /// Fractional hour `0..24`.
    pub hour: f64,
    pub trip: Option<TripView>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TripView {
    pub location: String,
    pub location_name: String,
    /// Day and hour when the pass runs out.
    pub ends_day: u32,
    pub ends_hour: f64,
    pub over: bool,
    /// Sleeping until morning is possible.
    pub can_sleep: bool,
}

impl ProfileView {
    pub fn new(p: &Profile, content: &crate::content::Content) -> Self {
        let rank = p.rank();
        let clock = crate::clock::Clock {
            minutes: p.clock_min,
            ..Default::default()
        };
        Self {
            money: p.money,
            xp: p.xp,
            rank,
            rank_name: profile::rank_name(rank).into(),
            rank_xp: profile::RANKS[rank as usize].0,
            next_rank_xp: profile::RANKS.get(rank as usize + 1).map(|r| r.0),
            items: p.items.clone(),
            slots: p.slots.clone(),
            keepnet: p.keepnet.len(),
            keepnet_kg: p.keepnet.iter().map(|c| c.weight_kg).sum(),
            day: clock.day(),
            hour: clock.hour(),
            trip: p.trip.as_ref().map(|t| {
                let ends = crate::clock::Clock {
                    minutes: t.ends_min,
                    ..Default::default()
                };
                TripView {
                    location: t.location.clone(),
                    location_name: content
                        .location(&t.location)
                        .map(|l| l.name.clone())
                        .unwrap_or_default(),
                    ends_day: ends.day(),
                    ends_hour: ends.hour(),
                    over: p.trip_over(),
                    can_sleep: p.next_morning().is_ok(),
                }
            }),
        }
    }
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
    pub weather: crate::weather::Weather,
    pub paused: bool,
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
    pub retrieve_mps: f64,
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
    Lure {
        at: Point,
        distance_m: f64,
        depth_m: f64,
        /// Water depth under the lure.
        bottom_m: f64,
        reeling: bool,
    },
    Snagged {
        at: Point,
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
            profile: self.profile().map(|p| ProfileView::new(p, &self.content)),
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
            weather: s.weather.current,
            paused: s.paused,
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
            RodState::Bite { at, cell, bite, .. } => {
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
            RodState::Lure { run, .. } => {
                let at = run.point(&s.spot.view);
                RodStateView::Lure {
                    at,
                    distance_m: run.distance_m,
                    depth_m: run.depth_m,
                    bottom_m: s.map.sample(at.x, at.y).map_or(0.0, |c| c.depth_m),
                    reeling: run.reeling,
                }
            }
            RodState::Snagged { run } => RodStateView::Snagged {
                at: run.point(&s.spot.view),
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
            retrieve_mps: t.retrieve_mps,
            state,
        }
    }

    fn debug_rates(&self, s: &Session, r: &Rod) -> Vec<(String, f64)> {
        if let RodState::Lure { run, .. } = &r.state {
            let at = run.point(&s.spot.view);
            let lure = self.content.bait(&r.tackle.bait).and_then(|b| b.lure);
            let (Some(cell), Some(lure)) = (s.map.sample(at.x, at.y), lure) else {
                return Vec::new();
            };
            let Some(zone) = s.spot.zone(cell.zone) else {
                return Vec::new();
            };
            let p = Presentation {
                cell,
                bait_depth_m: run.depth_m,
                bait: &r.tackle.bait,
                hour: s.clock.hour(),
                weather: s.weather.current,
                bonus: run.attraction(&lure, r.tackle.retrieve_mps),
            };
            return bite::rates(&self.content, zone, &p)
                .into_iter()
                .map(|(f, rate)| (f.name.clone(), rate * 60.0))
                .collect();
        }
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
            weather: s.weather.current,
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

/// A shop listing.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ShopItem {
    pub id: String,
    pub name: String,
    pub category: crate::content::ItemCategory,
    pub price: f64,
    /// Units per purchase.
    pub pack: u32,
    pub rank: u32,
    pub rank_name: String,
    pub stats: String,
    /// Rods only: which kind.
    pub rod_kind: Option<RodKind>,
    /// Terminal tackle only: which kind.
    pub tackle_kind: Option<crate::content::TackleKind>,
    /// Price at a fishing base, if it sells this at all.
    pub local_price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LocationInfo {
    pub id: String,
    pub name: String,
    pub kind: crate::content::WaterKind,
    pub description: String,
    pub rank: u32,
    pub rank_name: String,
    pub price_per_day: f64,
    pub max_days: u32,
    /// Species that live here, by display name.
    pub fish: Vec<String>,
    pub spots: Vec<SpotInfo>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpotInfo {
    pub id: String,
    pub name: String,
    pub background: String,
}

impl World {
    pub fn catalog(&self) -> Vec<ShopItem> {
        self.content
            .items()
            .map(|i| ShopItem {
                id: i.id().into(),
                name: i.name().into(),
                category: i.category(),
                price: i.price(),
                pack: i.pack(),
                rank: i.rank(),
                rank_name: profile::rank_name(i.rank()).into(),
                stats: i.stats(),
                rod_kind: match i {
                    crate::content::Item::Rod(r) => Some(r.kind),
                    _ => None,
                },
                tackle_kind: match i {
                    crate::content::Item::Tackle(t) => Some(t.kind),
                    _ => None,
                },
                local_price: profile::local_price(&i),
            })
            .collect()
    }

    /// Waters in order of progression (rank, then price).
    pub fn locations(&self) -> Vec<LocationInfo> {
        let mut locs: Vec<_> = self.content.locations.iter().collect();
        locs.sort_by(|a, b| {
            a.rank
                .cmp(&b.rank)
                .then(a.price_per_day.total_cmp(&b.price_per_day))
        });
        locs.into_iter()
            .map(|l| LocationInfo {
                id: l.id.clone(),
                name: l.name.clone(),
                kind: l.kind,
                description: l.description.clone(),
                rank: l.rank,
                rank_name: profile::rank_name(l.rank).into(),
                price_per_day: l.price_per_day,
                max_days: l.max_days,
                fish: {
                    let mut ids: Vec<&String> = l
                        .spots
                        .iter()
                        .flat_map(|s| &s.zones)
                        .flat_map(|z| z.fish.keys())
                        .collect();
                    ids.sort();
                    ids.dedup();
                    // Catalogue order (fish.ron), not alphabetical.
                    self.content
                        .fish
                        .iter()
                        .filter(|f| ids.contains(&&f.id))
                        .map(|f| f.name.clone())
                        .collect()
                },
                spots: l
                    .spots
                    .iter()
                    .map(|s| SpotInfo {
                        id: s.id.clone(),
                        name: s.name.clone(),
                        background: s.background.clone(),
                    })
                    .collect(),
            })
            .collect()
    }
}

/// What the main menu can offer.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MenuState {
    pub has_save: bool,
    /// Why saves don't work, if they don't.
    pub save_error: Option<String>,
}
