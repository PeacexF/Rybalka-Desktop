//! A rod in the water: tackle settings and its state machine.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::bite::BitePhase;
use super::fight::{Fight, FightTackle};
use crate::map::Cell;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum RodKind {
    Float,
    Feeder,
    Spinning,
}

/// Point in normalized image coordinates of the spot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Effective tackle of a rod slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tackle {
    pub kind: RodKind,
    pub max_cast_m: f64,
    pub line_strength_kg: f64,
    pub line_length_m: f64,
    pub reel_speed_mps: f64,
    pub drag_kg: f64,
    /// Float rods: distance from float to hook.
    pub float_depth_m: f64,
    pub bait: String,
}

impl Tackle {
    pub fn starter_float() -> Self {
        Self {
            kind: RodKind::Float,
            max_cast_m: 25.0,
            line_strength_kg: 3.0,
            line_length_m: 100.0,
            reel_speed_mps: 1.0,
            drag_kg: 1.8,
            float_depth_m: 1.5,
            bait: "worm".into(),
        }
    }

    pub fn starter_feeder() -> Self {
        Self {
            kind: RodKind::Feeder,
            max_cast_m: 45.0,
            line_strength_kg: 4.0,
            line_length_m: 150.0,
            reel_speed_mps: 1.2,
            drag_kg: 2.4,
            float_depth_m: 0.0,
            bait: "maggot".into(),
        }
    }

    pub fn fight(&self) -> FightTackle {
        FightTackle {
            line_strength_kg: self.line_strength_kg,
            drag_kg: self.drag_kg,
            reel_speed_mps: self.reel_speed_mps,
            line_length_m: self.line_length_m,
        }
    }

    /// Depth of the bait at a cell, and whether the float lies flat (rig longer than depth).
    pub fn bait_depth(&self, cell: &Cell) -> (f64, bool) {
        match self.kind {
            RodKind::Float => (
                self.float_depth_m.min(cell.depth_m),
                self.float_depth_m > cell.depth_m + 0.15,
            ),
            RodKind::Feeder | RodKind::Spinning => (cell.depth_m, false),
        }
    }
}

/// A fish on the line (or about to be).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hooked {
    pub fish_id: String,
    pub weight_kg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiteState {
    pub fish: Hooked,
    pub script: Vec<(BitePhase, f64)>,
    pub idx: usize,
    pub t: f64,
}

impl BiteState {
    pub fn phase(&self) -> BitePhase {
        self.script[self.idx.min(self.script.len() - 1)].0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RodState {
    Idle,
    Casting {
        target: Point,
        t: f64,
        dur: f64,
    },
    Waiting {
        at: Point,
        cell: Cell,
        /// Seconds before fish may bite (after cast / missed strike).
        calm: f64,
    },
    Bite {
        at: Point,
        cell: Cell,
        bite: BiteState,
    },
    Fighting {
        origin: Point,
        fish: Hooked,
        fight: Fight,
    },
    Retrieving {
        from: Point,
        t: f64,
        dur: f64,
    },
    Landed {
        catch: super::Catch,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rod {
    pub tackle: Tackle,
    pub state: RodState,
}

impl Rod {
    pub fn new(tackle: Tackle) -> Self {
        Self {
            tackle,
            state: RodState::Idle,
        }
    }
}
