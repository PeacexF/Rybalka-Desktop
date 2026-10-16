//! Headless balance report: bite rates across a spot and fight outcomes.
//!
//! ```text
//! cargo run -p rybalka-devtools --release --bin balance -- [location] [spot]
//! cargo run -p rybalka-devtools --release --bin balance -- economy
//! ```
//!
//! `economy` estimates income per real hour on every water and how long the
//! ranks take, for the economy balance pass.
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
    if args.first().is_some_and(|a| a == "economy") {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let content = load_dir(&root.join("content")).unwrap_or_else(|e| panic!("{e}"));
        economy_report(&content, &root);
        return;
    }
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

/// Water cells reachable with `max_cast_m`, sampled on an `n`×`n` grid.
fn castable_cells(spot: &SpotDef, map: &WaterMap, max_cast_m: f64, n: u32) -> Vec<Cell> {
    let mut cells = Vec::new();
    let g = f64::from(n);
    for yi in 0..n {
        for xi in 0..n {
            let (x, y) = ((f64::from(xi) + 0.5) / g, (f64::from(yi) + 0.5) / g);
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
        let cells = castable_cells(spot, map, tackle.max_cast_m, 60);
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

/// Share of bites that end up in the keepnet (missed strikes, fish that
/// leave, escapes and breaks), for a reasonably careful player.
const LANDED_PER_BITE: f64 = 0.55;
/// Real seconds per landed fish spent striking, playing and recasting.
const HANDLING_S: f64 = 30.0;

/// Mean sale price and experience per fish of a species (trophies included).
fn fish_value(f: &rybalka_core::content::FishDef) -> (f64, f64) {
    let mut rng = Rng::new(42);
    let (mut money, mut xp) = (0.0, 0.0);
    const N: u32 = 2000;
    for _ in 0..N {
        let kg = bite::roll_weight(f, &mut rng);
        let bonus = if kg >= f.trophy_kg {
            rybalka_core::profile::TROPHY_BONUS
        } else {
            1.0
        };
        money += f.price_per_kg * kg * bonus;
        xp += f.xp_per_kg * kg * bonus;
    }
    (money / f64::from(N), xp / f64::from(N))
}

struct Income {
    setup: String,
    rub_h: f64,
    xp_h: f64,
    /// Income with three rods of this kind out (same as `rub_h` for spinning).
    rub_h3: f64,
}

/// Hours sampled for the daytime average.
const ECONOMY_HOURS: [f64; 5] = [5.0, 9.0, 13.0, 17.0, 21.0];

/// Best single-rod income per real hour at a spot: averaged over daytime
/// hours and castable water, best bait (open at `rank`) per rod type.
fn spot_income(
    content: &Content,
    rank: u32,
    values: &BTreeMap<&str, (f64, f64)>,
    spot: &SpotDef,
    map: &WaterMap,
    tackles: &[(&str, Tackle)],
) -> Vec<Income> {
    let mut out = Vec::new();
    for (name, tackle) in tackles {
        let cells = castable_cells(spot, map, tackle.max_cast_m, 20);
        let lure = tackle.kind == RodKind::Spinning;
        let mut best: Option<Income> = None;
        for b in content
            .baits
            .iter()
            .filter(|b| (b.kind == BaitKind::Lure) == lure && b.rank <= rank)
        {
            let (mut rate, mut rub, mut xp, mut n) = (0.0, 0.0, 0.0, 0.0);
            for hour in ECONOMY_HOURS {
                for cell in &cells {
                    let Some(zone) = spot.zone(cell.zone) else {
                        continue;
                    };
                    let bait_depth_m = match tackle.kind {
                        RodKind::Float => tackle.float_depth_m.min(cell.depth_m),
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
                    for (f, r) in bite::rates(content, zone, &p) {
                        let (m, x) = values[f.id.as_str()];
                        rate += r;
                        rub += r * m;
                        xp += r * x;
                    }
                    n += 1.0;
                }
            }
            if n == 0.0 || rate <= 0.0 {
                continue;
            }
            // Per landed fish, then per real hour including handling time.
            let bites_s = rate / n;
            let fish_s = |rods: f64| {
                1.0 / (1.0 / (rods * bites_s * LANDED_PER_BITE) + HANDLING_S) * 3600.0 / rate
            };
            let rods = if lure { 1.0 } else { 3.0 };
            let inc = Income {
                setup: format!("{name}, {}", b.name),
                rub_h: fish_s(1.0) * rub,
                xp_h: fish_s(1.0) * xp,
                rub_h3: fish_s(rods) * rub,
            };
            if best.as_ref().is_none_or(|bst| inc.rub_h > bst.rub_h) {
                best = Some(inc);
            }
        }
        out.extend(best);
    }
    out
}

fn economy_report(content: &Content, root: &Path) {
    // Tackle a player plausibly has when they reach each water.
    let gear = |rod: &str, reel: &str, line: &str| {
        let mut p = rybalka_core::profile::Profile::starter(content);
        p.items.insert(rod.into(), 1);
        p.items.insert(reel.into(), 1);
        p.items.insert(line.into(), 1);
        p.xp = 1e9;
        p.equip(content, 0, rybalka_core::profile::Part::Rod, Some(rod))
            .unwrap();
        p.equip(content, 0, rybalka_core::profile::Part::Reel, Some(reel))
            .unwrap();
        p.equip(content, 0, rybalka_core::profile::Part::Line, Some(line))
            .unwrap();
        p.tackle_for(content, 0).unwrap()
    };
    let starter = [("Маховая", gear("rod_float_4m", "reel_1000", "line_016"))];
    let mid = [
        ("Маховая", gear("rod_float_4m", "reel_1000", "line_016")),
        (
            "Фидер лёгкий",
            gear("rod_feeder_light", "reel_2500", "line_020"),
        ),
        ("Спиннинг UL", gear("rod_spin_ul", "reel_2500", "line_020")),
    ];
    let top = [
        ("Болонская", gear("rod_bolo_5m", "reel_2500", "line_020")),
        (
            "Фидер тяжёлый",
            gear("rod_feeder_heavy", "reel_4000", "line_025"),
        ),
        ("Спиннинг M", gear("rod_spin_m", "reel_4000", "braid_010")),
    ];

    let values: BTreeMap<&str, (f64, f64)> = content
        .fish
        .iter()
        .map(|f| (f.id.as_str(), fish_value(f)))
        .collect();

    println!("# Economy\n");
    println!("One rod, careful play: {LANDED_PER_BITE} landed per bite, {HANDLING_S} s per fish,");
    println!("daytime average, cloudy. Real time; a fishing day (05–23) is ~108 real minutes.\n");
    println!("| Рыба | ₽ за рыбу | опыт за рыбу |");
    println!("|---|---|---|");
    for f in &content.fish {
        let (m, x) = values[f.id.as_str()];
        println!("| {} | {m:.0} | {x:.0} |", f.name);
    }
    println!();
    println!("| Водоём | Место | Снасть | ₽/ч | опыт/ч | ₽/ч, 3 удочки |");
    println!("|---|---|---|---|---|---|");
    let mut by_loc: Vec<(String, u32, f64, f64, f64)> = Vec::new();
    for loc in &content.locations {
        let tackles: &[(&str, Tackle)] = match loc.rank {
            0 => &starter,
            1 | 2 => &mid,
            _ => &top,
        };
        let (mut rub, mut xp, mut spots) = (0.0, 0.0, 0.0);
        for spot in &loc.spots {
            let bytes = std::fs::read(root.join("assets").join(&spot.map)).expect("read map");
            let map = WaterMap::from_png(&bytes).expect("decode map");
            let inc = spot_income(content, loc.rank, &values, spot, &map, tackles);
            for i in &inc {
                println!(
                    "| {} | {} | {} | {:.0} | {:.0} | {:.0} |",
                    loc.name, spot.name, i.setup, i.rub_h, i.xp_h, i.rub_h3
                );
            }
            if let Some(b) = inc.iter().max_by(|a, b| a.rub_h.total_cmp(&b.rub_h)) {
                rub += b.rub_h;
                xp += b.xp_h;
                spots += 1.0;
            }
        }
        if spots > 0.0 {
            by_loc.push((
                loc.name.clone(),
                loc.rank,
                loc.price_per_day,
                rub / spots,
                xp / spots,
            ));
        }
    }
    by_loc.sort_by_key(|l| l.1);

    println!("\n## Per water (best setup, averaged over spots)\n");
    println!("| Водоём | Разряд | Путёвка/день | ₽/ч | опыт/ч | Путёвка окупается, мин |");
    println!("|---|---|---|---|---|---|");
    for (name, rank, pass, rub, xp) in &by_loc {
        println!(
            "| {name} | {} | {pass:.0} | {rub:.0} | {xp:.0} | {:.0} |",
            rybalka_core::profile::rank_name(*rank),
            pass / rub * 60.0
        );
    }

    println!("\n## Time to each rank, fishing the best water you may enter\n");
    println!("| Разряд | Опыт | Где ловим | Минут (с начала) | Заработано ₽ |");
    println!("|---|---|---|---|---|");
    let (mut minutes, mut money) = (0.0, 0.0);
    for (i, (need, name)) in rybalka_core::profile::RANKS.iter().enumerate().skip(1) {
        let prev = rybalka_core::profile::RANKS[i - 1].0;
        let here = by_loc
            .iter()
            .filter(|l| (l.1 as usize) < i)
            .max_by(|a, b| a.4.total_cmp(&b.4))
            .expect("some water for every rank");
        let hours = (need - prev) / here.4;
        minutes += hours * 60.0;
        money += hours * here.3;
        println!(
            "| {name} | {need:.0} | {} | {minutes:.0} | {money:.0} |",
            here.0
        );
    }
}
