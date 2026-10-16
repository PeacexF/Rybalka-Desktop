//! Static game data loaded from RON files in `content/`.

mod defs;
mod load;

pub use defs::*;
pub use load::{ContentError, load_dir};

use std::collections::HashMap;

/// All loaded and validated content, indexed by id.
#[derive(Debug, Default)]
pub struct Content {
    pub fish: Vec<FishDef>,
    pub baits: Vec<BaitDef>,
    pub locations: Vec<LocationDef>,
    fish_idx: HashMap<String, usize>,
    bait_idx: HashMap<String, usize>,
    location_idx: HashMap<String, usize>,
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
        load::validate(&c)?;
        Ok(c)
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
    }
}
