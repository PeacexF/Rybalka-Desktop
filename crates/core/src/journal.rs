//! Records and the angler's diary: personal bests per species, a log of every
//! landed fish and a summary of every trip.
//!
//! Records live in the profile (they're small and needed at catch time).
//! Catch and trip logs grow without bound, so the world only queues them as
//! [`JournalEntry`]s; the app writes them to the save database.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::sim::rod::RodKind;
use crate::weather::Weather;

/// Personal best for one species.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Record {
    pub weight_kg: f64,
    /// Game minute of the catch.
    pub at_min: f64,
    pub location: String,
    pub spot: String,
    pub rod: RodKind,
    pub bait: String,
    /// All landed fish of this species, and how many were trophies.
    pub count: u32,
    pub trophies: u32,
}

/// One landed fish.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CatchLog {
    pub fish_id: String,
    pub fish_name: String,
    pub weight_kg: f64,
    pub trophy: bool,
    /// It beat the previous personal best.
    pub record: bool,
    pub bait: String,
    pub rod: RodKind,
    pub location: String,
    pub spot: String,
    pub at_min: f64,
    pub weather: Weather,
}

/// Running totals of the current trip.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TripStats {
    pub started_min: f64,
    pub fish: u32,
    pub kg: f64,
    pub earned: f64,
    pub best: Option<(String, f64)>,
    pub weathers: Vec<Weather>,
}

impl TripStats {
    pub fn add_catch(&mut self, name: &str, kg: f64) {
        self.fish += 1;
        self.kg += kg;
        if self.best.as_ref().is_none_or(|(_, b)| kg > *b) {
            self.best = Some((name.into(), kg));
        }
    }

    pub fn saw_weather(&mut self, w: Weather) {
        if !self.weathers.contains(&w) {
            self.weathers.push(w);
        }
    }
}

/// A finished trip, for the diary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TripLog {
    pub location: String,
    pub started_min: f64,
    pub ended_min: f64,
    pub fish: u32,
    pub kg: f64,
    pub earned: f64,
    pub best_name: Option<String>,
    pub best_kg: f64,
    pub weathers: Vec<Weather>,
}

/// Something to append to the diary database.
#[derive(Debug, Clone, PartialEq)]
pub enum JournalEntry {
    Catch(CatchLog),
    Trip(TripLog),
}

/// A species in the records table: its best catch so far, if any.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpeciesRecord {
    pub fish_id: String,
    pub name: String,
    pub trophy_kg: f64,
    pub record: Option<Record>,
}

/// Everything the diary screen shows.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Diary {
    pub records: Vec<SpeciesRecord>,
    /// Newest first.
    pub trips: Vec<TripLog>,
    /// Newest first.
    pub catches: Vec<CatchLog>,
}
