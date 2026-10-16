use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use super::{BaitDef, Content, FishDef, LocationDef};

#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    Parse {
        path: PathBuf,
        source: Box<ron::error::SpannedError>,
    },
    #[error("duplicate {kind} id `{id}`")]
    Duplicate { kind: &'static str, id: String },
    #[error("{0}")]
    Invalid(String),
}

/// Load `fish.ron`, `baits.ron` and `locations/*.ron` from a content directory.
pub fn load_dir(dir: &Path) -> Result<Content, ContentError> {
    let fish: Vec<FishDef> = read_ron(&dir.join("fish.ron"))?;
    let baits: Vec<BaitDef> = read_ron(&dir.join("baits.ron"))?;

    let loc_dir = dir.join("locations");
    let mut paths: Vec<PathBuf> = fs::read_dir(&loc_dir)
        .map_err(|source| ContentError::Io {
            path: loc_dir.clone(),
            source,
        })?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "ron"))
        .collect();
    paths.sort();
    let locations = paths
        .iter()
        .map(|p| read_ron::<LocationDef>(p))
        .collect::<Result<Vec<_>, _>>()?;

    Content::new(fish, baits, locations)
}

fn read_ron<T: DeserializeOwned>(path: &Path) -> Result<T, ContentError> {
    let text = fs::read_to_string(path).map_err(|source| ContentError::Io {
        path: path.to_owned(),
        source,
    })?;
    ron::from_str(&text).map_err(|source| ContentError::Parse {
        path: path.to_owned(),
        source: Box::new(source),
    })
}

pub(super) fn validate(c: &Content) -> Result<(), ContentError> {
    let invalid = |msg: String| Err(ContentError::Invalid(msg));

    for f in &c.fish {
        let w = &f.weight;
        if !(0.0 < w.min && w.min <= w.typical && w.typical <= w.max) {
            return invalid(format!("fish `{}`: need 0 < min <= typical <= max", f.id));
        }
        if f.depth.min > f.depth.max {
            return invalid(format!("fish `{}`: depth.min > depth.max", f.id));
        }
        if f.activity.windows(2).any(|w| w[0].0 >= w[1].0)
            || f.activity
                .iter()
                .any(|(h, a)| !(0.0..24.0).contains(h) || *a < 0.0)
        {
            return invalid(format!(
                "fish `{}`: activity must be sorted by hour in 0..24 with values >= 0",
                f.id
            ));
        }
        if f.baits.is_empty() {
            return invalid(format!("fish `{}`: takes no baits", f.id));
        }
        for (bait, w) in &f.baits {
            if c.bait(bait).is_none() {
                return invalid(format!("fish `{}`: unknown bait `{bait}`", f.id));
            }
            if !(0.0..=1.0).contains(w) {
                return invalid(format!("fish `{}`: bait `{bait}` weight not in 0..1", f.id));
            }
        }
        if f.strength <= 0.0 {
            return invalid(format!("fish `{}`: strength must be > 0", f.id));
        }
    }

    for l in &c.locations {
        if l.spots.is_empty() {
            return invalid(format!("location `{}`: no spots", l.id));
        }
        for s in &l.spots {
            let v = &s.view;
            if !(0.0 <= v.horizon_y && v.horizon_y < v.shore_y && v.shore_y <= 1.0) {
                return invalid(format!(
                    "spot `{}/{}`: need 0 <= horizon_y < shore_y <= 1",
                    l.id, s.id
                ));
            }
            for z in &s.zones {
                for fish in z.fish.keys() {
                    if c.fish(fish).is_none() {
                        return invalid(format!(
                            "spot `{}/{}` zone {}: unknown fish `{fish}`",
                            l.id, s.id, z.id
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
