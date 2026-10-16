//! Rasterize a water map spec (`map.ron`) into `map.png`.
//!
//! ```text
//! gen_map <map.ron> <map.png> [--preview <bg.jpg> <preview.jpg>]
//! ```
//!
//! The spec describes the water area as polygons in normalized image
//! coordinates, a depth profile by distance (using the spot's perspective
//! calibration) and soft-edged features that set zones, bottom types and
//! adjust depth. `--preview` writes the background with depth/zones tinted
//! over it, to check alignment by eye.

use image::RgbImage;
use rybalka_core::content::View;
use rybalka_core::map::DEPTH_STEP_M;
use rybalka_devtools::{Noise, smoothstep, write_rgba_png};
use serde::Deserialize;

type Pt = (f64, f64);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Spec {
    /// Output size in pixels (the map may be coarser than the background).
    size: (u32, u32),
    view: View,
    water: Vec<Vec<Pt>>,
    #[serde(default)]
    land: Vec<Vec<Pt>>,
    depth: DepthProfile,
    #[serde(default = "one")]
    default_zone: u8,
    #[serde(default)]
    features: Vec<Feature>,
}

fn one() -> u8 {
    1
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DepthProfile {
    /// Depth at the near edge of the frame.
    near: f64,
    /// Depth reached at `far_at_m` metres and beyond.
    far: f64,
    far_at_m: f64,
    /// Amplitude of bottom relief noise, metres.
    #[serde(default)]
    noise: f64,
    /// Closer than this the bottom is sand.
    #[serde(default)]
    sand_within_m: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Feature {
    shape: Shape,
    /// Width of the soft edge, normalized units.
    #[serde(default)]
    soft: f64,
    #[serde(default)]
    depth: Option<DepthOp>,
    #[serde(default)]
    zone: Option<u8>,
    #[serde(default)]
    bottom: Option<Bottom>,
}

#[derive(Deserialize)]
enum Shape {
    Polygon(Vec<Pt>),
    Ellipse { cx: f64, cy: f64, rx: f64, ry: f64 },
}

#[derive(Deserialize, Clone, Copy)]
enum DepthOp {
    Set(f64),
    Add(f64),
    Mul(f64),
}

#[derive(Deserialize, Clone, Copy)]
enum Bottom {
    Silt = 0,
    Sand = 1,
    Stone = 2,
    Weed = 3,
    Snag = 4,
}

fn inside(poly: &[Pt], p: Pt) -> bool {
    let mut c = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            c = !c;
        }
    }
    c
}

fn edge_distance(poly: &[Pt], p: Pt) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let len2 = (dx * dx + dy * dy).max(1e-12);
            let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
            let (qx, qy) = (a.0 + t * dx - p.0, a.1 + t * dy - p.1);
            (qx * qx + qy * qy).sqrt()
        })
        .fold(f64::INFINITY, f64::min)
}

impl Shape {
    /// Weight `0..1`: 1 well inside, fading to 0 over `soft` outside the edge.
    fn weight(&self, p: Pt, soft: f64) -> f64 {
        match self {
            Shape::Polygon(poly) => {
                if inside(poly, p) {
                    1.0
                } else if soft > 0.0 {
                    1.0 - smoothstep(0.0, soft, edge_distance(poly, p))
                } else {
                    0.0
                }
            }
            Shape::Ellipse { cx, cy, rx, ry } => {
                let r = (((p.0 - cx) / rx).powi(2) + ((p.1 - cy) / ry).powi(2)).sqrt();
                let soft_r = if soft > 0.0 { soft / rx.min(*ry) } else { 1e-6 };
                1.0 - smoothstep(1.0, 1.0 + soft_r, r)
            }
        }
    }
}

struct Texel {
    water: bool,
    depth: f64,
    bottom: u8,
    zone: u8,
}

fn texel(spec: &Spec, noise: &Noise, p: Pt) -> Texel {
    let water = spec.water.iter().any(|poly| inside(poly, p))
        && !spec.land.iter().any(|poly| inside(poly, p));
    let d = spec.view.distance_at(p.1).min(spec.depth.far_at_m * 4.0);
    let prof = &spec.depth;
    let mut depth = prof.near + (prof.far - prof.near) * smoothstep(0.0, prof.far_at_m, d);
    depth += prof.noise * (noise.fbm(p.0 * 8.0, p.1 * 14.0, 3) - 0.5) * 2.0;
    let mut bottom = if d < prof.sand_within_m {
        Bottom::Sand as u8
    } else {
        Bottom::Silt as u8
    };
    let mut zone = spec.default_zone;
    for f in &spec.features {
        let w = f.shape.weight(p, f.soft);
        if w <= 0.0 {
            continue;
        }
        if let Some(op) = f.depth {
            depth = match op {
                DepthOp::Set(v) => depth + (v - depth) * w,
                DepthOp::Add(v) => depth + v * w,
                DepthOp::Mul(v) => depth * (1.0 + (v - 1.0) * w),
            };
        }
        if w >= 0.5 {
            if let Some(z) = f.zone {
                zone = z;
            }
            if let Some(b) = f.bottom {
                bottom = b as u8;
            }
        }
    }
    Texel {
        water,
        depth: depth.clamp(0.1, 255.0 * DEPTH_STEP_M),
        bottom,
        zone,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 2 && !(args.len() == 5 && args[2] == "--preview") {
        eprintln!("usage: gen_map <map.ron> <map.png> [--preview <bg.jpg> <preview.jpg>]");
        std::process::exit(2);
    }
    let text = std::fs::read_to_string(&args[0]).expect("read spec");
    let spec: Spec = ron::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", args[0]));
    let noise = Noise::new(7);

    let (w, h) = spec.size;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    let (mut min_d, mut max_d) = (f64::INFINITY, 0.0f64);
    for y in 0..h {
        for x in 0..w {
            let p = (
                (f64::from(x) + 0.5) / f64::from(w),
                (f64::from(y) + 0.5) / f64::from(h),
            );
            let t = texel(&spec, &noise, p);
            if t.water {
                min_d = min_d.min(t.depth);
                max_d = max_d.max(t.depth);
                rgba.extend([
                    (t.depth / DEPTH_STEP_M).round() as u8,
                    t.bottom,
                    t.zone,
                    255,
                ]);
            } else {
                rgba.extend([0, 0, 0, 0]);
            }
        }
    }
    write_rgba_png(std::path::Path::new(&args[1]), w, h, &rgba).expect("write map");
    println!("{}: {w}x{h}, depth {min_d:.2}..{max_d:.2} m", args[1]);

    if args.len() == 5 {
        preview(&spec, &noise, &args[3], &args[4]);
    }
}

/// Tint water by depth (light = shallow, dark blue = deep) and outline zones.
fn preview(spec: &Spec, noise: &Noise, bg: &str, out: &str) {
    let mut img: RgbImage = image::open(bg).expect("open bg").to_rgb8();
    let (w, h) = img.dimensions();
    let zone_colors = [
        [255, 255, 255],
        [80, 200, 255],
        [255, 200, 40],
        [255, 80, 200],
        [120, 255, 120],
        [255, 120, 60],
    ];
    let at = |x: u32, y: u32| {
        texel(
            spec,
            noise,
            (
                (f64::from(x) + 0.5) / f64::from(w),
                (f64::from(y) + 0.5) / f64::from(h),
            ),
        )
    };
    for by in (0..h).step_by(2) {
        for bx in (0..w).step_by(2) {
            let t = at(bx, by);
            if !t.water {
                continue;
            }
            let edge = bx + 4 < w && at(bx + 4, by).zone != t.zone
                || by + 4 < h && at(bx, by + 4).zone != t.zone;
            let k = (t.depth / 6.0).min(1.0);
            let tint = [255.0 * (1.0 - k), 220.0 - 140.0 * k, 120.0 + 135.0 * k];
            for (x, y) in [(bx, by), (bx + 1, by), (bx, by + 1), (bx + 1, by + 1)] {
                if x >= w || y >= h {
                    continue;
                }
                let px = img.get_pixel_mut(x, y);
                if edge {
                    *px = image::Rgb(zone_colors[usize::from(t.zone) % zone_colors.len()]);
                } else {
                    for c in 0..3 {
                        px[c] = (f64::from(px[c]) * 0.45 + tint[c] * 0.55) as u8;
                    }
                }
            }
        }
    }
    let file = std::fs::File::create(out).expect("create preview");
    image::codecs::jpeg::JpegEncoder::new_with_quality(file, 85)
        .encode_image(&img)
        .expect("encode preview");
    println!("preview: {out}");
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Spec;

    /// Every shipped spot: the map spec's calibration matches the content,
    /// the map has the background's aspect ratio and only uses known zones.
    #[test]
    fn shipped_maps_match_content() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let content = rybalka_core::content::load_dir(&root.join("content")).unwrap();
        for loc in &content.locations {
            for spot in &loc.spots {
                let name = format!("{}/{}", loc.id, spot.id);
                let png = root.join("assets").join(&spot.map);
                let spec_path = png.with_extension("ron");
                let spec: Spec = ron::from_str(&std::fs::read_to_string(&spec_path).unwrap())
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(spec.view, spot.view, "{name}: view differs from map.ron");

                let (bw, bh) =
                    image::image_dimensions(root.join("assets").join(&spot.background)).unwrap();
                let (mw, mh) = spec.size;
                let (ab, am) = (f64::from(bw) / f64::from(bh), f64::from(mw) / f64::from(mh));
                assert!(
                    (ab - am).abs() < 0.02,
                    "{name}: map {am:.3} vs background {ab:.3}"
                );

                let map =
                    rybalka_core::map::WaterMap::from_png(&std::fs::read(&png).unwrap()).unwrap();
                for yi in 0..50 {
                    for xi in 0..50 {
                        let (x, y) = (f64::from(xi) / 50.0, f64::from(yi) / 50.0);
                        if let Some(c) = map.sample(x, y) {
                            assert!(
                                spot.zone(c.zone).is_some(),
                                "{name}: zone {} at {x},{y}",
                                c.zone
                            );
                        }
                    }
                }
            }
        }
    }
}
