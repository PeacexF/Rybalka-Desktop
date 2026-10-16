//! Shop catalogue: durable gear (rods, reels, lines), consumables and the starter kit.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::sim::rod::RodKind;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GearCatalog {
    pub rods: Vec<RodDef>,
    pub reels: Vec<ReelDef>,
    pub lines: Vec<LineDef>,
    pub tackle: Vec<TackleDef>,
    pub starter: StarterDef,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RodDef {
    pub id: String,
    pub name: String,
    pub kind: RodKind,
    pub cast_m: f64,
    pub price: f64,
    #[serde(default)]
    pub rank: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReelDef {
    pub id: String,
    pub name: String,
    pub speed_mps: f64,
    pub max_drag_kg: f64,
    pub price: f64,
    #[serde(default)]
    pub rank: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineDef {
    pub id: String,
    pub name: String,
    pub strength_kg: f64,
    pub price: f64,
    #[serde(default)]
    pub rank: u32,
}

/// Terminal tackle and groundbait, sold in packs and used up.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TackleDef {
    pub id: String,
    pub name: String,
    pub kind: TackleKind,
    pub pack: u32,
    pub price: f64,
    #[serde(default)]
    pub rank: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum TackleKind {
    Hook,
    Float,
    Feeder,
    Groundbait,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarterDef {
    pub money: f64,
    pub items: BTreeMap<String, u32>,
    pub slots: Vec<StarterSlot>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarterSlot {
    pub rod: Option<String>,
    pub reel: Option<String>,
    pub line: Option<String>,
    pub bait: Option<String>,
}

/// Shop category of any purchasable item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum ItemCategory {
    Rod,
    Reel,
    Line,
    Tackle,
    Bait,
    Lure,
}

/// Uniform view of any item in the catalogue.
#[derive(Debug, Clone, Copy)]
pub enum Item<'a> {
    Rod(&'a RodDef),
    Reel(&'a ReelDef),
    Line(&'a LineDef),
    Tackle(&'a TackleDef),
    Bait(&'a super::BaitDef),
}

impl Item<'_> {
    pub fn id(&self) -> &str {
        match self {
            Item::Rod(d) => &d.id,
            Item::Reel(d) => &d.id,
            Item::Line(d) => &d.id,
            Item::Tackle(d) => &d.id,
            Item::Bait(d) => &d.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Item::Rod(d) => &d.name,
            Item::Reel(d) => &d.name,
            Item::Line(d) => &d.name,
            Item::Tackle(d) => &d.name,
            Item::Bait(d) => &d.name,
        }
    }

    pub fn price(&self) -> f64 {
        match self {
            Item::Rod(d) => d.price,
            Item::Reel(d) => d.price,
            Item::Line(d) => d.price,
            Item::Tackle(d) => d.price,
            Item::Bait(d) => d.price,
        }
    }

    pub fn rank(&self) -> u32 {
        match self {
            Item::Rod(d) => d.rank,
            Item::Reel(d) => d.rank,
            Item::Line(d) => d.rank,
            Item::Tackle(d) => d.rank,
            Item::Bait(d) => d.rank,
        }
    }

    /// Units per purchase.
    pub fn pack(&self) -> u32 {
        match self {
            Item::Tackle(d) => d.pack,
            Item::Bait(d) => d.pack,
            _ => 1,
        }
    }

    pub fn category(&self) -> ItemCategory {
        match self {
            Item::Rod(_) => ItemCategory::Rod,
            Item::Reel(_) => ItemCategory::Reel,
            Item::Line(_) => ItemCategory::Line,
            Item::Tackle(_) => ItemCategory::Tackle,
            Item::Bait(b) if b.kind == super::BaitKind::Lure => ItemCategory::Lure,
            Item::Bait(_) => ItemCategory::Bait,
        }
    }

    /// Short human-readable stats for the shop.
    pub fn stats(&self) -> String {
        match self {
            Item::Rod(d) => format!("заброс до {} м", d.cast_m),
            Item::Reel(d) => format!(
                "подмотка {:.1} м/с · фрикцион до {} кг",
                d.speed_mps, d.max_drag_kg
            ),
            Item::Line(d) => format!("разрывная нагрузка {} кг", d.strength_kg),
            Item::Tackle(d) => format!("{} шт. в упаковке", d.pack),
            Item::Bait(b) => match b.lure {
                Some(l) => format!(
                    "проводка {:.1}–{:.1} м/с · ход {:.1} м",
                    l.speed.min, l.speed.max, l.depth
                ),
                None => format!("{} порций", b.pack),
            },
        }
    }
}
