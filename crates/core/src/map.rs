//! Water map of a fishing spot: an RGBA image aligned with the background.
//!
//! - `A` — water mask (0 = land)
//! - `R` — depth in 5 cm steps (0..12.75 m)
//! - `G` — bottom type, see [`Bottom`]
//! - `B` — zone id

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const DEPTH_STEP_M: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Bottom {
    Silt,
    Sand,
    Stone,
    Weed,
    Snag,
}

impl Bottom {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Sand,
            2 => Self::Stone,
            3 => Self::Weed,
            4 => Self::Snag,
            _ => Self::Silt,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Cell {
    pub depth_m: f64,
    pub bottom: Bottom,
    pub zone: u8,
}

#[derive(Debug, thiserror::Error)]
pub enum MapError {
    #[error("png decode: {0}")]
    Png(#[from] png::DecodingError),
    #[error("water map must be 8-bit RGBA, got {0:?} {1:?}")]
    Format(png::ColorType, png::BitDepth),
}

#[derive(Debug, Clone)]
pub struct WaterMap {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl WaterMap {
    pub fn from_png(bytes: &[u8]) -> Result<Self, MapError> {
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info()?;
        let mut buf = vec![0; reader.output_buffer_size().unwrap_or(0)];
        let info = reader.next_frame(&mut buf)?;
        if info.bit_depth != png::BitDepth::Eight {
            return Err(MapError::Format(info.color_type, info.bit_depth));
        }
        buf.truncate(info.buffer_size());
        match info.color_type {
            png::ColorType::Rgba => {}
            // Encoders may drop a fully opaque alpha channel (all water).
            png::ColorType::Rgb => {
                buf = buf
                    .chunks_exact(3)
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect();
            }
            other => return Err(MapError::Format(other, info.bit_depth)),
        }
        Ok(Self {
            width: info.width,
            height: info.height,
            rgba: buf,
        })
    }

    /// Build a map from a function of normalized coordinates (for tests/tools).
    pub fn from_fn(width: u32, height: u32, f: impl Fn(f64, f64) -> [u8; 4]) -> Self {
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let u = (f64::from(x) + 0.5) / f64::from(width);
                let v = (f64::from(y) + 0.5) / f64::from(height);
                rgba.extend_from_slice(&f(u, v));
            }
        }
        Self {
            width,
            height,
            rgba,
        }
    }

    /// Sample at normalized image coordinates. `None` on land or out of bounds.
    pub fn sample(&self, x: f64, y: f64) -> Option<Cell> {
        if !(0.0..1.0).contains(&x) || !(0.0..1.0).contains(&y) {
            return None;
        }
        let px = (x * f64::from(self.width)) as usize;
        let py = (y * f64::from(self.height)) as usize;
        let i = (py * self.width as usize + px) * 4;
        let [r, g, b, a] = self.rgba[i..i + 4].try_into().ok()?;
        (a >= 128).then(|| Cell {
            depth_m: f64::from(r) * DEPTH_STEP_M,
            bottom: Bottom::from_u8(g),
            zone: b,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_water_and_land() {
        let map = WaterMap::from_fn(10, 10, |_, y| {
            if y < 0.5 {
                [0, 0, 0, 0]
            } else {
                [40, 3, 2, 255]
            }
        });
        assert_eq!(map.sample(0.5, 0.2), None);
        let c = map.sample(0.5, 0.8).unwrap();
        assert!((c.depth_m - 2.0).abs() < 1e-9);
        assert_eq!(c.bottom, Bottom::Weed);
        assert_eq!(c.zone, 2);
        assert_eq!(map.sample(1.2, 0.8), None);
    }
}

#[cfg(test)]
mod asset_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn shipped_pond_map_has_water_and_zones() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/locations/pond/willow/map.png");
        let map = WaterMap::from_png(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(map.sample(0.5, 0.3), None, "sky must be land");
        assert_eq!(map.sample(0.97, 0.8), None, "birch branches are not water");
        let near = map.sample(0.3, 0.9).expect("foreground should be water");
        assert!(near.depth_m > 0.5);
        assert_eq!(map.sample(0.2, 0.6).map(|c| c.zone), Some(4), "old channel");
        assert_eq!(
            map.sample(0.8, 0.75).map(|c| c.zone),
            Some(3),
            "under the birch"
        );
    }
}
