//! Headless balance report: bite rates across a spot and fight outcomes.
//!
//! ```text
//! cargo run -p rybalka-devtools --release --bin balance -- [location] [spot]
//! ```
//!
//! Part 1 samples castable water on a grid and prints, per rod setup and bait,
//! the expected time between bites by hour (cloudy weather, no groundbait),
//! plus the species mix at the best hour. Part 2 plays fights with the
//! starter tackle (always reeling, default drag) and reports outcomes.

use std::collections::BTreeMap;
use std::path::Path;

use rybalka_core::content::{BaitKind, Content, SpotDef, load_dir};
use rybalka_core::map::{Cell, WaterMap};
use rybalka_core::rng::Rng;
use rybalka_core::sim::bite::{self, Presentation};
use rybalka_core::sim::fight::{Fight, Outcome};
use rybalka_core::sim::rod::{RodKind, Tackle};
use rybalka_core::sim::spin::LURE_RATE_MULT;
use rybalka_core::weather::Weather;

const HOURS: [f64; 8] = [5.0, 7.0, 9.0, 12.0, 15.0, 18.0, 20.0, 22.0];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let location = args.first().map_or("pond", String::as_str);
    let spot_id = args.get(1).map_or("willow", String::as_str);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let content = load_dir(&root.join("content")).unwrap_or_else(|e| panic!("{e}"));
    let spot = content
        .location(location)
        .and_then(|l| l.spot(spot_id))
        .unwrap_or_else(|| panic!("no spot {location}/{spot_id}"));
    let map_bytes = std::fs::read(root.join("assets").join(&spot.map)).expect("read map");
    let map = WaterMap::from_png(&map_bytes).expect("decode map");

    println!("# Balance: {location}/{spot_id} ({})\n", spot.name);
    bite_report(&content, spot, &map);
    fight_report(&content);
}

/// Water cells reachable with `max_cast_m`, sampled on a grid.
fn castable_cells(spot: &SpotDef, map: &WaterMap, max_cast_m: f64) -> Vec<Cell> {
    let mut cells = Vec::new();
    for yi in 0..60 {
        for xi in 0..60 {
            let (x, y) = ((f64::from(xi) + 0.5) / 60.0, (f64::from(yi) + 0.5) / 60.0);
            if spot.view.distance_at(y) > max_cast_m {
                continue;
            }
            if let Some(c) = map.sample(x, y) {
                cells.push(c);
            }
        }
    }
    cells
}

fn bite_report(content: &Content, spot: &SpotDef, map: &WaterMap) {
    println!("## Seconds between bites (median over castable water / best spot)\n");
    let setups = [
        ("Поплавок 1.5 м", Tackle::starter_float()),
        ("Фидер", Tackle::starter_feeder()),
        ("Спиннинг", Tackle::starter_spinning()),
    ];
    print!("| Снасть | Наживка |");
    for h in HOURS {
        print!(" {h:02}ч |");
    }
    println!("\n|---|---|{}", "---|".repeat(HOURS.len()));

    for (name, tackle) in &setups {
        let cells = castable_cells(spot, map, tackle.max_cast_m);
        let lure = tackle.kind == RodKind::Spinning;
        for b in content
            .baits
            .iter()
            .filter(|b| (b.kind == BaitKind::Lure) == lure)
        {
            print!("| {name} | {} |", b.name);
            let mut best_mix: Option<(f64, BTreeMap<String, f64>)> = None;
            for hour in HOURS {
                let mut per_cell: Vec<f64> = Vec::with_capacity(cells.len());
                for cell in &cells {
                    let Some(zone) = spot.zone(cell.zone) else {
                        continue;
                    };
                    let bait_depth_m = match tackle.kind {
                        RodKind::Float => tackle.float_depth_m.min(cell.depth_m),
                        // A lure averages its running depth and the bottom.
                        RodKind::Spinning => b.lure.map_or(1.0, |l| l.depth).min(cell.depth_m),
                        RodKind::Feeder => cell.depth_m,
                    };
                    let p = Presentation {
                        cell: *cell,
                        bait_depth_m,
                        bait: &b.id,
                        hour,
                        weather: Weather::Cloudy,
                        bonus: if lure { LURE_RATE_MULT } else { 1.0 },
                    };
                    let rates = bite::rates(content, zone, &p);
                    let total: f64 = rates.iter().map(|(_, r)| r).sum();
                    if best_mix.as_ref().is_none_or(|(t, _)| total > *t) {
                        let mix = rates
                            .iter()
                            .map(|(f, r)| (f.name.clone(), r / total.max(1e-12)))
                            .collect();
                        best_mix = Some((total, mix));
                    }
                    per_cell.push(total);
                }
                per_cell.sort_by(f64::total_cmp);
                let median = per_cell.get(per_cell.len() / 2).copied().unwrap_or(0.0);
                let best = per_cell.last().copied().unwrap_or(0.0);
                print!(" {} / {} |", secs(median), secs(best));
            }
            if let Some((_, mix)) = best_mix {
                let mut mix: Vec<_> = mix.into_iter().filter(|(_, s)| *s > 0.02).collect();
                mix.sort_by(|a, b| b.1.total_cmp(&a.1));
                let mix: Vec<String> = mix
                    .iter()
                    .map(|(n, s)| format!("{n} {:.0}%", s * 100.0))
                    .collect();
                print!(" {}", mix.join(", "));
            }
            println!();
        }
    }
    println!();
}

fn secs(rate_per_s: f64) -> String {
    if rate_per_s <= 1e-9 {
        "—".into()
    } else {
        format!("{:.0}", 1.0 / rate_per_s)
    }
}

fn fight_report(content: &Content) {
    println!("## Fights with starter tackle, default drag\n");
    println!("«Тупо» — подмотка без остановки; «умно» — отпускать при пробуксовке");
    println!("фрикциона или натяжении > 85%.\n");
    println!(
        "| Рыба | Вес | Снасть | Тупо: вытащено / обрыв / сход | Умно: вытащено / обрыв / сход | Время умно, с |"
    );
    println!("|---|---|---|---|---|---|");
    for f in &content.fish {
        let tackle = if f.baits.keys().any(|b| {
            content
                .bait(b)
                .is_some_and(|d| d.kind == BaitKind::Lure && f.baits[b] >= 0.8)
        }) {
            Tackle::starter_spinning()
        } else if f.weight.typical > 0.5 {
            Tackle::starter_feeder()
        } else {
            Tackle::starter_float()
        };
        for (label, kg) in [
            ("типичный", f.weight.typical),
            ("трофей", f.trophy_kg),
            ("макс", f.weight.max),
        ] {
            let dumb = simulate(f.strength, kg, &tackle, false);
            let smart = simulate(f.strength, kg, &tackle, true);
            println!(
                "| {} | {label} {kg:.2} кг | {:?} | {} | {} | {} |",
                f.name,
                tackle.kind,
                dumb.summary(),
                smart.summary(),
                smart.avg_time()
            );
        }
    }
}

struct Stats {
    landed: u32,
    broke: u32,
    escaped: u32,
    time: f64,
    n: u32,
}

impl Stats {
    fn summary(&self) -> String {
        let pct = |k: u32| f64::from(k) * 100.0 / f64::from(self.n);
        format!(
            "{:.0}% / {:.0}% / {:.0}%",
            pct(self.landed),
            pct(self.broke),
            pct(self.escaped)
        )
    }

    fn avg_time(&self) -> String {
        if self.landed == 0 {
            "—".into()
        } else {
            format!("{:.0}", self.time / f64::from(self.landed))
        }
    }
}

fn simulate(strength: f64, kg: f64, tackle: &Tackle, smart: bool) -> Stats {
    const N: u32 = 300;
    let mut st = Stats {
        landed: 0,
        broke: 0,
        escaped: 0,
        time: 0.0,
        n: N,
    };
    let t = tackle.fight();
    for seed in 0..u64::from(N) {
        let mut rng = Rng::new(seed);
        let mut fight = Fight::new(kg, strength, 20.0);
        let mut secs = 0.0;
        let outcome = loop {
            fight.reeling =
                !smart || (!fight.slipping && fight.tension_kg < 0.85 * t.line_strength_kg);
            secs += 0.05;
            match fight.step(0.05, &t, &mut rng) {
                Outcome::Continue if secs < 600.0 => {}
                Outcome::Continue => break Outcome::Escaped,
                o => break o,
            }
        };
        match outcome {
            Outcome::Landed => {
                st.landed += 1;
                st.time += secs;
            }
            Outcome::Broke => st.broke += 1,
            _ => st.escaped += 1,
        }
    }
    st
}
