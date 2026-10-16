//! Shared helpers for dev tools.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub fn write_rgba_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let w = BufWriter::new(File::create(path)?);
    let mut enc = png::Encoder::new(w, width, height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::High);
    let mut writer = enc.write_header().map_err(std::io::Error::other)?;
    writer
        .write_image_data(rgba)
        .map_err(std::io::Error::other)?;
    Ok(())
}

/// Deterministic 2D value noise with smooth interpolation, roughly in `0..1`.
pub struct Noise {
    seed: u32,
}

impl Noise {
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }

    fn hash(&self, x: i32, y: i32) -> f64 {
        let mut h = (x as u32).wrapping_mul(0x27d4_eb2d)
            ^ (y as u32).wrapping_mul(0x1656_67b1)
            ^ self.seed.wrapping_mul(0x9e37_79b9);
        h ^= h >> 15;
        h = h.wrapping_mul(0x2c1b_3c6d);
        h ^= h >> 12;
        h = h.wrapping_mul(0x297a_2d39);
        h ^= h >> 15;
        f64::from(h) / f64::from(u32::MAX)
    }

    pub fn get(&self, x: f64, y: f64) -> f64 {
        let (xi, yi) = (x.floor() as i32, y.floor() as i32);
        let (xf, yf) = (x - x.floor(), y - y.floor());
        let s = |t: f64| t * t * (3.0 - 2.0 * t);
        let (sx, sy) = (s(xf), s(yf));
        let a = self.hash(xi, yi);
        let b = self.hash(xi + 1, yi);
        let c = self.hash(xi, yi + 1);
        let d = self.hash(xi + 1, yi + 1);
        let top = a + (b - a) * sx;
        let bot = c + (d - c) * sx;
        top + (bot - top) * sy
    }

    /// Fractal sum of `octaves` layers.
    pub fn fbm(&self, x: f64, y: f64, octaves: u32) -> f64 {
        let (mut sum, mut amp, mut freq, mut norm) = (0.0, 0.5, 1.0, 0.0);
        for _ in 0..octaves {
            sum += amp * self.get(x * freq, y * freq);
            norm += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        sum / norm
    }

    /// Hash of an integer cell, `0..1`.
    pub fn cell(&self, x: i32, y: i32) -> f64 {
        self.hash(x, y)
    }
}

pub type Rgb = [f64; 3];

pub fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

pub fn hex(c: u32) -> Rgb {
    [
        f64::from((c >> 16) & 0xff),
        f64::from((c >> 8) & 0xff),
        f64::from(c & 0xff),
    ]
}

pub fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
