//! Player profile: money, experience, owned items and rod slots.
//!
//! The profile is what gets saved. While fishing, the live state (keepnet,
//! clock, rod settings) lives in the session and is synced back with
//! [`crate::World::sync_profile`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::clock::Clock;
use crate::content::{BaitKind, Content, Item, ItemCategory, TackleKind};
use crate::sim::Catch;
use crate::sim::rod::{RodKind, Tackle};

/// Rod slots on the bank, as in RR.
pub const SLOTS: usize = 3;
/// Line on every reel, metres (not a shop stat in the arcade model).
const LINE_LENGTH_M: f64 = 150.0;
/// Sale price multiplier for trophies.
pub const TROPHY_BONUS: f64 = 1.5;

/// Ranks by experience: `(xp needed, name)`.
pub const RANKS: [(f64, &str); 8] = [
    (0.0, "Новичок"),
    (150.0, "Любитель"),
    (500.0, "Рыболов"),
    (1200.0, "Опытный рыболов"),
    (2500.0, "Мастер"),
    (5000.0, "Мастер спорта"),
    (9000.0, "Профи"),
    (15000.0, "Гуру"),
];

pub fn rank_of(xp: f64) -> u32 {
    RANKS.iter().rposition(|(need, _)| xp >= *need).unwrap_or(0) as u32
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    pub money: f64,
    pub xp: f64,
    /// Item id → units owned (rods/reels/lines: pieces; consumables: units).
    pub items: BTreeMap<String, u32>,
    pub slots: Vec<Slot>,
    /// Fish caught but not sold yet.
    pub keepnet: Vec<Catch>,
    /// Game clock, minutes since day 1 00:00.
    pub clock_min: f64,
    pub next_catch_id: u64,
    /// Where the player is fishing; restored on load.
    pub at: Option<Place>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            money: 0.0,
            xp: 0.0,
            items: BTreeMap::new(),
            slots: vec![Slot::default(); SLOTS],
            keepnet: Vec::new(),
            clock_min: Clock::default().minutes,
            next_catch_id: 1,
            at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Place {
    pub location: String,
    pub spot: String,
}

/// A rod slot: which gear is assembled and its settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Slot {
    pub rod: Option<String>,
    pub reel: Option<String>,
    pub line: Option<String>,
    pub bait: Option<String>,
    pub float_depth_m: f64,
    pub drag_kg: f64,
    pub retrieve_mps: f64,
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            rod: None,
            reel: None,
            line: None,
            bait: None,
            float_depth_m: 1.5,
            drag_kg: 1.5,
            retrieve_mps: 0.7,
        }
    }
}

/// Part of a slot that can be (un)equipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub enum Part {
    Rod,
    Reel,
    Line,
    Bait,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProfileError {
    #[error("нет такого товара: `{0}`")]
    UnknownItem(String),
    #[error("нужен разряд «{0}»")]
    Rank(&'static str),
    #[error("не хватает денег")]
    Money,
    #[error("нет слота {0}")]
    NoSlot(usize),
    #[error("это не подходит сюда")]
    WrongPart,
    #[error("нет в рюкзаке")]
    NotOwned,
    #[error("все такие уже стоят на других удочках")]
    InUse,
    #[error("сначала поставьте удилище")]
    NoRod,
}

/// Result of selling the keepnet.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SaleReport {
    pub lines: Vec<SaleLine>,
    pub money: f64,
    pub xp: f64,
    pub rank_before: u32,
    pub rank_after: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SaleLine {
    pub name: String,
    pub count: u32,
    pub weight_kg: f64,
    pub money: f64,
}

impl Profile {
    /// Fresh profile from the content's starter kit.
    pub fn starter(c: &Content) -> Self {
        let st = &c.gear.starter;
        let mut p = Self {
            money: st.money,
            items: st.items.clone(),
            ..Self::default()
        };
        for (i, s) in st.slots.iter().take(SLOTS).enumerate() {
            let slot = &mut p.slots[i];
            slot.rod.clone_from(&s.rod);
            slot.reel.clone_from(&s.reel);
            slot.line.clone_from(&s.line);
            slot.bait.clone_from(&s.bait);
            slot.drag_kg = default_drag(c, slot);
        }
        p
    }

    /// Drop references to items the content no longer has (after content updates).
    pub fn sanitize(&mut self, c: &Content) {
        self.items.retain(|id, n| *n > 0 && c.item(id).is_some());
        self.slots.resize_with(SLOTS, Slot::default);
        for slot in &mut self.slots {
            for part in [
                &mut slot.rod,
                &mut slot.reel,
                &mut slot.line,
                &mut slot.bait,
            ] {
                if part.as_ref().is_some_and(|id| c.item(id).is_none()) {
                    *part = None;
                }
            }
        }
        if let Some(at) = &self.at
            && c.location(&at.location)
                .and_then(|l| l.spot(&at.spot))
                .is_none()
        {
            self.at = None;
        }
    }

    pub fn rank(&self) -> u32 {
        rank_of(self.xp)
    }

    pub fn count(&self, id: &str) -> u32 {
        self.items.get(id).copied().unwrap_or(0)
    }

    /// Remove one unit; `false` if there was none.
    pub fn take(&mut self, id: &str) -> bool {
        match self.items.get_mut(id) {
            Some(n) if *n > 0 => {
                *n -= 1;
                if *n == 0 {
                    self.items.remove(id);
                }
                true
            }
            _ => false,
        }
    }

    /// First owned consumable of a kind, in catalogue order.
    fn owned_of_kind<'c>(&self, c: &'c Content, kind: TackleKind) -> Option<&'c str> {
        c.gear
            .tackle
            .iter()
            .find(|t| t.kind == kind && self.count(&t.id) > 0)
            .map(|t| t.id.as_str())
    }

    pub fn take_kind(&mut self, c: &Content, kind: TackleKind) -> bool {
        match self.owned_of_kind(c, kind) {
            Some(id) => self.take(id),
            None => false,
        }
    }

    pub fn buy(&mut self, c: &Content, id: &str, packs: u32) -> Result<(), ProfileError> {
        let item = c
            .item(id)
            .ok_or_else(|| ProfileError::UnknownItem(id.into()))?;
        if self.rank() < item.rank() {
            return Err(ProfileError::Rank(rank_name(item.rank())));
        }
        let cost = item.price() * f64::from(packs.max(1));
        if cost > self.money + 1e-9 {
            return Err(ProfileError::Money);
        }
        self.money -= cost;
        *self.items.entry(id.into()).or_default() += item.pack() * packs.max(1);
        Ok(())
    }

    /// Put an owned item into a slot, or clear that part with `None`.
    pub fn equip(
        &mut self,
        c: &Content,
        slot: usize,
        part: Part,
        id: Option<&str>,
    ) -> Result<(), ProfileError> {
        if slot >= self.slots.len() {
            return Err(ProfileError::NoSlot(slot));
        }
        if let Some(id) = id {
            let item = c
                .item(id)
                .ok_or_else(|| ProfileError::UnknownItem(id.into()))?;
            let fits = matches!(
                (part, item.category()),
                (Part::Rod, ItemCategory::Rod)
                    | (Part::Reel, ItemCategory::Reel)
                    | (Part::Line, ItemCategory::Line)
                    | (Part::Bait, ItemCategory::Bait | ItemCategory::Lure)
            );
            if !fits {
                return Err(ProfileError::WrongPart);
            }
            let owned = self.count(id);
            if owned == 0 {
                return Err(ProfileError::NotOwned);
            }
            if part == Part::Bait {
                let kind = self.slots[slot]
                    .rod
                    .as_deref()
                    .and_then(|r| rod_kind(c, r))
                    .ok_or(ProfileError::NoRod)?;
                if !bait_fits(c, kind, id) {
                    return Err(ProfileError::WrongPart);
                }
            } else {
                // Durable gear: each piece goes on one rod only.
                let used = self
                    .slots
                    .iter()
                    .enumerate()
                    .filter(|(i, s)| *i != slot && part_of(s, part) == Some(id))
                    .count() as u32;
                if used >= owned {
                    return Err(ProfileError::InUse);
                }
            }
        }

        *part_mut(&mut self.slots[slot], part) = id.map(Into::into);
        let s = &self.slots[slot];
        match part {
            Part::Rod => {
                // A new rod may need a different kind of bait.
                let kind = s.rod.as_deref().and_then(|r| rod_kind(c, r));
                let ok = match (kind, s.bait.as_deref()) {
                    (Some(k), Some(b)) => bait_fits(c, k, b),
                    _ => false,
                };
                if !ok {
                    let bait = kind.and_then(|k| {
                        c.baits
                            .iter()
                            .find(|b| bait_fits(c, k, &b.id) && self.count(&b.id) > 0)
                            .map(|b| b.id.clone())
                    });
                    self.slots[slot].bait = bait;
                }
            }
            Part::Reel | Part::Line => self.slots[slot].drag_kg = default_drag(c, s),
            Part::Bait => {}
        }
        Ok(())
    }

    /// Effective tackle of a slot, if it has a rod, reel and line.
    pub fn tackle_for(&self, c: &Content, slot: usize) -> Option<Tackle> {
        let s = self.slots.get(slot)?;
        let rod = match c.item(s.rod.as_deref()?)? {
            Item::Rod(r) => r,
            _ => return None,
        };
        let reel = match c.item(s.reel.as_deref()?)? {
            Item::Reel(r) => r,
            _ => return None,
        };
        let line = match c.item(s.line.as_deref()?)? {
            Item::Line(l) => l,
            _ => return None,
        };
        let max_drag_kg = reel.max_drag_kg.min(line.strength_kg * 1.3);
        Some(Tackle {
            kind: rod.kind,
            max_cast_m: rod.cast_m,
            line_strength_kg: line.strength_kg,
            line_length_m: LINE_LENGTH_M,
            reel_speed_mps: reel.speed_mps,
            drag_kg: s.drag_kg.clamp(0.1, max_drag_kg),
            max_drag_kg,
            float_depth_m: s.float_depth_m,
            retrieve_mps: s.retrieve_mps,
            bait: s.bait.clone().unwrap_or_default(),
        })
    }

    /// Name of the first thing missing for a cast with this tackle, if any.
    pub fn missing_for_cast(&self, c: &Content, t: &Tackle) -> Option<String> {
        for kind in rig_parts(t.kind) {
            if self.owned_of_kind(c, *kind).is_none() {
                return Some(kind_name(c, *kind));
            }
        }
        if t.bait.is_empty() || self.count(&t.bait) == 0 {
            return Some(match c.bait(&t.bait) {
                Some(b) => b.name.clone(),
                None if t.kind == RodKind::Spinning => "приманка".into(),
                None => "наживка".into(),
            });
        }
        None
    }

    /// The rig is gone (line broke): hook, float/feeder and the bait or lure.
    pub fn lose_rig(&mut self, c: &Content, t: &Tackle) {
        for kind in rig_parts(t.kind) {
            self.take_kind(c, *kind);
        }
        self.take(&t.bait);
    }

    /// A natural bait got eaten or fell off. Lures are not used up this way.
    pub fn use_bait(&mut self, c: &Content, t: &Tackle) {
        if c.bait(&t.bait).is_some_and(|b| b.kind == BaitKind::Natural) {
            self.take(&t.bait);
        }
    }

    /// Sell everything in the keepnet: money and experience.
    pub fn sell_keepnet(&mut self, c: &Content) -> SaleReport {
        let rank_before = self.rank();
        let mut lines: Vec<SaleLine> = Vec::new();
        let (mut money, mut xp) = (0.0, 0.0);
        for catch in self.keepnet.drain(..) {
            let Some(f) = c.fish(&catch.fish_id) else {
                continue;
            };
            let bonus = if catch.trophy { TROPHY_BONUS } else { 1.0 };
            let m = (f.price_per_kg * catch.weight_kg * bonus).round();
            money += m;
            xp += f.xp_per_kg * catch.weight_kg * bonus;
            match lines.iter_mut().find(|l| l.name == f.name) {
                Some(l) => {
                    l.count += 1;
                    l.weight_kg += catch.weight_kg;
                    l.money += m;
                }
                None => lines.push(SaleLine {
                    name: f.name.clone(),
                    count: 1,
                    weight_kg: catch.weight_kg,
                    money: m,
                }),
            }
        }
        self.money += money;
        self.xp += xp.round();
        SaleReport {
            lines,
            money,
            xp: xp.round(),
            rank_before,
            rank_after: self.rank(),
        }
    }
}

pub fn rank_name(rank: u32) -> &'static str {
    RANKS[(rank as usize).min(RANKS.len() - 1)].1
}

/// Consumables a cast takes besides the bait.
fn rig_parts(kind: RodKind) -> &'static [TackleKind] {
    match kind {
        RodKind::Float => &[TackleKind::Hook, TackleKind::Float],
        RodKind::Feeder => &[TackleKind::Hook, TackleKind::Feeder],
        RodKind::Spinning => &[],
    }
}

fn kind_name(c: &Content, kind: TackleKind) -> String {
    c.gear
        .tackle
        .iter()
        .find(|t| t.kind == kind)
        .map_or_else(|| format!("{kind:?}"), |t| t.name.clone())
}

fn rod_kind(c: &Content, rod: &str) -> Option<RodKind> {
    match c.item(rod)? {
        Item::Rod(r) => Some(r.kind),
        _ => None,
    }
}

fn bait_fits(c: &Content, kind: RodKind, bait: &str) -> bool {
    c.bait(bait)
        .is_some_and(|b| (b.kind == BaitKind::Lure) == (kind == RodKind::Spinning))
}

/// Drag that suits the line: 60% of its strength, within the reel's limit.
fn default_drag(c: &Content, s: &Slot) -> f64 {
    let strength = match s.line.as_deref().and_then(|l| c.item(l)) {
        Some(Item::Line(l)) => l.strength_kg,
        _ => return Slot::default().drag_kg,
    };
    let max = match s.reel.as_deref().and_then(|r| c.item(r)) {
        Some(Item::Reel(r)) => r.max_drag_kg,
        _ => f64::INFINITY,
    };
    ((strength * 0.6).min(max) * 10.0).round() / 10.0
}

fn part_of(s: &Slot, part: Part) -> Option<&str> {
    match part {
        Part::Rod => s.rod.as_deref(),
        Part::Reel => s.reel.as_deref(),
        Part::Line => s.line.as_deref(),
        Part::Bait => s.bait.as_deref(),
    }
}

fn part_mut(s: &mut Slot, part: Part) -> &mut Option<String> {
    match part {
        Part::Rod => &mut s.rod,
        Part::Reel => &mut s.reel,
        Part::Line => &mut s.line,
        Part::Bait => &mut s.bait,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn content() -> Content {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        crate::content::load_dir(&dir).unwrap()
    }

    #[test]
    fn ranks_by_xp() {
        assert_eq!(rank_of(0.0), 0);
        assert_eq!(rank_of(149.0), 0);
        assert_eq!(rank_of(150.0), 1);
        assert_eq!(rank_of(1e9), RANKS.len() as u32 - 1);
    }

    #[test]
    fn starter_has_a_working_float_rod() {
        let c = content();
        let p = Profile::starter(&c);
        assert_eq!(p.slots.len(), SLOTS);
        let t = p.tackle_for(&c, 0).expect("slot 1 assembled");
        assert_eq!(t.kind, RodKind::Float);
        assert_eq!(t.bait, "worm");
        assert!(t.drag_kg < t.line_strength_kg);
        assert_eq!(p.missing_for_cast(&c, &t), None);
        assert!(p.tackle_for(&c, 1).is_none());
    }

    #[test]
    fn buying_needs_money_and_rank() {
        let c = content();
        let mut p = Profile::starter(&c);
        p.money = 100.0;
        assert_eq!(p.buy(&c, "worm", 2), Ok(()));
        assert_eq!(p.count("worm"), 40 + 40);
        assert!((p.money - 60.0).abs() < 1e-9);
        assert_eq!(
            p.buy(&c, "rod_bolo_5m", 1),
            Err(ProfileError::Rank("Рыболов"))
        );
        p.xp = 1e6;
        assert_eq!(p.buy(&c, "rod_bolo_5m", 1), Err(ProfileError::Money));
        assert!(matches!(
            p.buy(&c, "nope", 1),
            Err(ProfileError::UnknownItem(_))
        ));
    }

    #[test]
    fn one_rod_cannot_go_on_two_slots() {
        let c = content();
        let mut p = Profile::starter(&c);
        assert_eq!(
            p.equip(&c, 1, Part::Rod, Some("rod_float_4m")),
            Err(ProfileError::InUse)
        );
        p.items.insert("rod_float_4m".into(), 2);
        assert_eq!(p.equip(&c, 1, Part::Rod, Some("rod_float_4m")), Ok(()));
        assert_eq!(p.slots[1].bait.as_deref(), Some("worm"), "bait auto-picked");
        assert_eq!(
            p.equip(&c, 1, Part::Reel, Some("reel_2500")),
            Err(ProfileError::NotOwned)
        );
        assert_eq!(
            p.equip(&c, 1, Part::Line, Some("reel_1000")),
            Err(ProfileError::WrongPart)
        );
    }

    #[test]
    fn spinning_rod_takes_lures_only() {
        let c = content();
        let mut p = Profile::starter(&c);
        p.items.insert("rod_spin_ul".into(), 1);
        p.items.insert("spinner".into(), 1);
        p.equip(&c, 0, Part::Rod, Some("rod_spin_ul")).unwrap();
        assert_eq!(p.slots[0].bait.as_deref(), Some("spinner"));
        assert_eq!(
            p.equip(&c, 0, Part::Bait, Some("worm")),
            Err(ProfileError::WrongPart)
        );
        let t = p.tackle_for(&c, 0).unwrap();
        assert_eq!(p.missing_for_cast(&c, &t), None, "lures need no hooks");
        p.lose_rig(&c, &t);
        assert_eq!(p.missing_for_cast(&c, &t).as_deref(), Some("Вертушка №2"));
    }

    #[test]
    fn broken_rig_and_eaten_bait_are_used_up() {
        let c = content();
        let mut p = Profile::starter(&c);
        let t = p.tackle_for(&c, 0).unwrap();
        let (hooks, floats, worms) = (p.count("hooks"), p.count("floats"), p.count("worm"));
        p.lose_rig(&c, &t);
        assert_eq!(p.count("hooks"), hooks - 1);
        assert_eq!(p.count("floats"), floats - 1);
        assert_eq!(p.count("worm"), worms - 1);
        p.use_bait(&c, &t);
        assert_eq!(p.count("worm"), worms - 2);
        p.items.remove("floats");
        assert_eq!(p.missing_for_cast(&c, &t).as_deref(), Some("Поплавки"));
    }

    #[test]
    fn selling_pays_by_weight_with_trophy_bonus() {
        let c = content();
        let mut p = Profile::starter(&c);
        let roach = c.fish("roach").unwrap();
        let catch = |id, kg, trophy| Catch {
            id,
            fish_id: "roach".into(),
            name: roach.name.clone(),
            weight_kg: kg,
            trophy,
            bait: "worm".into(),
            at_min: 0.0,
        };
        p.keepnet = vec![catch(1, 0.5, false), catch(2, 1.0, true)];
        let money = p.money;
        let r = p.sell_keepnet(&c);
        let expect = (roach.price_per_kg * 0.5).round() + (roach.price_per_kg * 1.5).round();
        assert!((r.money - expect).abs() < 1e-9, "{r:?}");
        assert!((p.money - money - expect).abs() < 1e-9);
        assert_eq!(r.lines.len(), 1);
        assert_eq!(r.lines[0].count, 2);
        assert!(p.keepnet.is_empty());
        assert!(p.xp > 0.0);
    }

    #[test]
    fn sanitize_drops_unknown_items() {
        let c = content();
        let mut p = Profile::starter(&c);
        p.items.insert("gone".into(), 3);
        p.slots[2].rod = Some("gone".into());
        p.slots.truncate(1);
        p.sanitize(&c);
        assert!(!p.items.contains_key("gone"));
        assert_eq!(p.slots.len(), SLOTS);
        assert_eq!(p.slots[2].rod, None);
    }

    #[test]
    fn profile_round_trips_through_json() {
        let c = content();
        let p = Profile::starter(&c);
        let json = serde_json::to_string(&p).unwrap();
        let back: Profile = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
        // Fields added later get defaults.
        let old: Profile = serde_json::from_str(r#"{"money": 5}"#).unwrap();
        assert_eq!(old.money, 5.0);
        assert_eq!(old.slots.len(), SLOTS);
    }
}
