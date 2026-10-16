//! Static game data loaded from RON files in `content/`.

mod defs;
pub mod gear;
mod load;

pub use defs::*;
pub use gear::{GearCatalog, Item, ItemCategory};
pub use load::{ContentError, load_dir};

use std::collections::HashMap;

/// All loaded and validated content, indexed by id.
#[derive(Debug, Default)]
pub struct Content {
    pub fish: Vec<FishDef>,
    pub baits: Vec<BaitDef>,
    pub locations: Vec<LocationDef>,
    pub gear: GearCatalog,
    fish_idx: HashMap<String, usize>,
    bait_idx: HashMap<String, usize>,
    location_idx: HashMap<String, usize>,
    /// Every purchasable id (gear and baits) → (category index, position).
    item_idx: HashMap<String, (u8, usize)>,
}

impl Content {
    /// Build indexes and validate cross references.
    pub fn new(
        fish: Vec<FishDef>,
        baits: Vec<BaitDef>,
        locations: Vec<LocationDef>,
    ) -> Result<Self, ContentError> {
        let mut c = Self {
            fish,
            baits,
            locations,
            ..Default::default()
        };
        c.fish_idx = index(&c.fish, |f| &f.id, "fish")?;
        c.bait_idx = index(&c.baits, |b| &b.id, "bait")?;
        c.location_idx = index(&c.locations, |l| &l.id, "location")?;
        c.index_items()?;
        load::validate(&c)?;
        Ok(c)
    }

    /// Attach the shop catalogue and validate it.
    pub fn with_gear(mut self, gear: GearCatalog) -> Result<Self, ContentError> {
        self.gear = gear;
        self.index_items()?;
        load::validate(&self)?;
        load::validate_gear(&self)?;
        Ok(self)
    }

    fn index_items(&mut self) -> Result<(), ContentError> {
        let mut idx = HashMap::new();
        let mut add = |id: &String, key: (u8, usize)| {
            if idx.insert(id.clone(), key).is_some() {
                return Err(ContentError::Duplicate {
                    kind: "item",
                    id: id.clone(),
                });
            }
            Ok(())
        };
        let g = &self.gear;
        for (i, d) in g.rods.iter().enumerate() {
            add(&d.id, (0, i))?;
        }
        for (i, d) in g.reels.iter().enumerate() {
            add(&d.id, (1, i))?;
        }
        for (i, d) in g.lines.iter().enumerate() {
            add(&d.id, (2, i))?;
        }
        for (i, d) in g.tackle.iter().enumerate() {
            add(&d.id, (3, i))?;
        }
        for (i, d) in self.baits.iter().enumerate() {
            add(&d.id, (4, i))?;
        }
        self.item_idx = idx;
        Ok(())
    }

    pub fn item(&self, id: &str) -> Option<Item<'_>> {
        let &(cat, i) = self.item_idx.get(id)?;
        Some(match cat {
            0 => Item::Rod(&self.gear.rods[i]),
            1 => Item::Reel(&self.gear.reels[i]),
            2 => Item::Line(&self.gear.lines[i]),
            3 => Item::Tackle(&self.gear.tackle[i]),
            _ => Item::Bait(&self.baits[i]),
        })
    }

    /// All purchasable items in catalogue order.
    pub fn items(&self) -> impl Iterator<Item = Item<'_>> {
        let g = &self.gear;
        g.rods
            .iter()
            .map(Item::Rod)
            .chain(g.reels.iter().map(Item::Reel))
            .chain(g.lines.iter().map(Item::Line))
            .chain(g.tackle.iter().map(Item::Tackle))
            .chain(self.baits.iter().map(Item::Bait))
    }

    pub fn fish(&self, id: &str) -> Option<&FishDef> {
        self.fish_idx.get(id).map(|&i| &self.fish[i])
    }

    pub fn bait(&self, id: &str) -> Option<&BaitDef> {
        self.bait_idx.get(id).map(|&i| &self.baits[i])
    }

    pub fn location(&self, id: &str) -> Option<&LocationDef> {
        self.location_idx.get(id).map(|&i| &self.locations[i])
    }
}

fn index<T>(
    items: &[T],
    id: impl Fn(&T) -> &String,
    kind: &'static str,
) -> Result<HashMap<String, usize>, ContentError> {
    let mut map = HashMap::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        if map.insert(id(item).clone(), i).is_some() {
            return Err(ContentError::Duplicate {
                kind,
                id: id(item).clone(),
            });
        }
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// The shipped content directory must always load and validate.
    #[test]
    fn shipped_content_is_valid() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let c = super::load_dir(&dir).unwrap_or_else(|e| panic!("{e}"));
        assert!(!c.fish.is_empty());
        assert!(c.location("pond").is_some());
        // Shop catalogue: every starter item exists and baits are purchasable too.
        assert!(c.item("rod_float_4m").is_some());
        assert!(c.item("worm").is_some_and(|i| i.pack() > 1));
        assert!(c.items().count() > c.baits.len());
    }
}
