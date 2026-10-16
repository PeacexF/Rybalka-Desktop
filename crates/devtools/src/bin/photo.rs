//! Photo helpers for location backgrounds.
//!
//! ```text
//! photo prepare <in> <out.jpg> [--crop-top F] [--crop-bottom F] [--aspect A] [--shift-x F]
//!     Crop to 16:9 or a wider panorama aspect `A` (trimming the given
//!     fractions of height first, then centring, or shifting the crop by
//!     `F` of the spare width, -1..1), resize to 1080 px high and save as
//!     JPEG (q=88).
//! photo sheet <out.jpg> <images...>
//!     Tile images into a 5-column contact sheet (in argument order).
//! photo grid <in> <out.jpg>
//!     Overlay a coordinate grid (thin every 0.05, thick every 0.1) to help
//!     author water map specs in normalized coordinates.
//! ```

use std::path::Path;

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, RgbImage};

const OUT_H: u32 = 1080;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("prepare") if args.len() >= 3 => prepare(&args[1], &args[2], &args[3..]),
        Some("sheet") if args.len() >= 3 => sheet(&args[1], &args[2..]),
        Some("grid") if args.len() == 3 => grid(&args[1], &args[2]),
        _ => {
            eprintln!("usage: photo prepare <in> <out.jpg> [--crop-top F] [--crop-bottom F]");
            eprintln!("       photo sheet <out.jpg> <images...>");
            eprintln!("       photo grid <in> <out.jpg>");
            std::process::exit(2);
        }
    }
}

fn flag(opts: &[String], name: &str) -> f64 {
    opts.iter()
        .position(|o| o == name)
        .and_then(|i| opts.get(i + 1))
        .map_or(0.0, |v| v.parse().expect("flag value must be a number"))
}

fn prepare(input: &str, output: &str, opts: &[String]) {
    let img = image::open(input).expect("open input");
    let (w, h) = img.dimensions();
    let top = (f64::from(h) * flag(opts, "--crop-top")) as u32;
    let bottom = (f64::from(h) * flag(opts, "--crop-bottom")) as u32;
    let h2 = h - top - bottom;
    let aspect = match flag(opts, "--aspect") {
        a if a > 0.0 => a,
        _ => 16.0 / 9.0,
    };
    // Crop to the aspect inside the remaining band, centred vertically.
    let (cw, ch) = if f64::from(w) / f64::from(h2) > aspect {
        ((f64::from(h2) * aspect) as u32, h2)
    } else {
        (w, (f64::from(w) / aspect) as u32)
    };
    let shift = flag(opts, "--shift-x").clamp(-1.0, 1.0);
    let x = ((f64::from(w - cw) / 2.0) * (1.0 + shift)) as u32;
    let y = top + (h2 - ch) / 2;
    let out = img
        .crop_imm(x, y, cw, ch)
        .resize_exact(
            (f64::from(OUT_H) * aspect).round() as u32,
            OUT_H,
            FilterType::Lanczos3,
        )
        .to_rgb8();
    save_jpeg(&out, output);
    println!("{input}: {w}x{h} -> crop {cw}x{ch}+{x}+{y} -> {output}");
}

fn sheet(output: &str, inputs: &[String]) {
    const COLS: u32 = 5;
    const TW: u32 = 384;
    const TH: u32 = 216;
    const PAD: u32 = 6;
    let rows = (inputs.len() as u32).div_ceil(COLS);
    let mut canvas = RgbImage::from_pixel(
        COLS * (TW + PAD) + PAD,
        rows * (TH + PAD) + PAD,
        image::Rgb([20, 20, 20]),
    );
    for (i, path) in inputs.iter().enumerate() {
        let Ok(img) = image::open(path) else {
            eprintln!("skip {path}");
            continue;
        };
        let thumb: DynamicImage = img.resize_to_fill(TW, TH, FilterType::Triangle);
        let (c, r) = (i as u32 % COLS, i as u32 / COLS);
        image::imageops::overlay(
            &mut canvas,
            &thumb.to_rgb8(),
            i64::from(PAD + c * (TW + PAD)),
            i64::from(PAD + r * (TH + PAD)),
        );
    }
    save_jpeg(&canvas, output);
}

fn grid(input: &str, output: &str) {
    let mut img = image::open(input).expect("open input").to_rgb8();
    let (w, h) = img.dimensions();
    for i in 1..20 {
        let thick = i % 2 == 0;
        let color = if thick {
            image::Rgb([255, 40, 40])
        } else {
            image::Rgb([255, 255, 0])
        };
        let x = w * i / 20;
        let y = h * i / 20;
        let width = if thick { 2 } else { 1 };
        for d in 0..width {
            for yy in 0..h {
                img.put_pixel((x + d).min(w - 1), yy, color);
            }
            for xx in 0..w {
                img.put_pixel(xx, (y + d).min(h - 1), color);
            }
        }
    }
    save_jpeg(&img, output);
}

fn save_jpeg(img: &RgbImage, path: impl AsRef<Path>) {
    let path = path.as_ref();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).expect("create output dir");
    }
    let file = std::fs::File::create(path).expect("create output");
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 88);
    enc.encode_image(img).expect("encode jpeg");
}
