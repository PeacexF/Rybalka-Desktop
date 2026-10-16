use std::collections::BTreeMap;
use std::sync::Arc;

use super::rod::{RodKind, Tackle};
use super::*;
use crate::content::*;

fn content() -> Arc<Content> {
    let fish = vec![FishDef {
        id: "roach".into(),
        name: "Плотва".into(),
        weight: WeightDist {
            min: 0.05,
            typical: 0.2,
            max: 1.0,
        },
        trophy_kg: 0.6,
        horizon: Horizon::Any,
        depth: Range { min: 0.5, max: 4.0 },
        activity: vec![],
        baits: BTreeMap::from([("worm".to_string(), 1.0)]),
        strength: 2.0,
        bite: BiteStyle::Bold,
        price_per_kg: 50.0,
        xp_per_kg: 10.0,
        weather: BTreeMap::new(),
    }];
    let baits = vec![BaitDef {
        id: "worm".into(),
        name: "Червь".into(),
        kind: BaitKind::Natural,
        price: 20.0,
        pack: 20,
        rank: 0,
        lure: None,
    }];
    let locations = vec![LocationDef {
        id: "pond".into(),
        name: "Пруд".into(),
        kind: WaterKind::Pond,
        description: String::new(),
        rank: 0,
        price_per_day: 0.0,
        max_days: 7,
        spots: vec![SpotDef {
            id: "dam".into(),
            name: "Плотина".into(),
            background: "bg.png".into(),
            map: "map.png".into(),
            view: View {
                horizon_y: 0.4,
                shore_y: 0.9,
                near_m: 3.0,
                fov: 1.2,
            },
            zones: vec![ZoneDef {
                id: 1,
                name: "Вода".into(),
                fish: BTreeMap::from([("roach".to_string(), 1.0)]),
            }],
        }],
    }];
    Arc::new(Content::new(fish, baits, locations).unwrap())
}

/// Water (2 m deep, zone 1) below y = 0.45, land above.
fn map() -> WaterMap {
    WaterMap::from_fn(64, 64, |_, y| {
        if y > 0.45 {
            [40, 0, 1, 255]
        } else {
            [0, 0, 0, 0]
        }
    })
}

fn world() -> World {
    let mut w = World::new(content(), 1234);
    w.start_session("pond", "dam", map(), vec![Tackle::starter_float()])
        .unwrap();
    w
}

fn run_until(
    w: &mut World,
    max_ticks: u32,
    pred: impl Fn(&GameEvent) -> bool,
) -> Option<GameEvent> {
    for _ in 0..max_ticks {
        w.step();
        if let Some(e) = w.drain_events().into_iter().find(|e| pred(e)) {
            return Some(e);
        }
    }
    None
}

#[test]
fn full_catch_cycle() {
    let mut w = world();
    w.input(Input::Cast {
        rod: 0,
        x: 0.5,
        y: 0.7,
    })
    .unwrap();
    assert!(run_until(&mut w, 200, |e| matches!(e, GameEvent::CastLanded { .. })).is_some());

    // Keep striking during the hookable phase until a fish is on.
    let mut hooked = false;
    for _ in 0..20 {
        assert!(
            run_until(&mut w, 20 * 60 * 10, |e| matches!(
                e,
                GameEvent::Bite { .. }
            ))
            .is_some(),
            "no bite in 10 minutes"
        );
        // Wait for the decisive phase, then strike.
        for _ in 0..200 {
            let s = w.session().unwrap();
            match &s.rods[0].state {
                RodState::Bite { bite, .. } if bite.phase().hook_chance() > 0.5 => break,
                RodState::Bite { .. } => w.step(),
                _ => break,
            }
        }
        w.input(Input::Strike { rod: 0 }).unwrap();
        if matches!(
            w.session().unwrap().rods[0].state,
            RodState::Fighting { .. }
        ) {
            hooked = true;
            break;
        }
    }
    assert!(hooked, "never hooked a fish");

    w.input(Input::Reel { rod: 0, on: true }).unwrap();
    let landed = run_until(&mut w, 20 * 120, |e| {
        matches!(
            e,
            GameEvent::Landed { .. } | GameEvent::LineBroke { .. } | GameEvent::Escaped { .. }
        )
    });
    let Some(GameEvent::Landed { catch, .. }) = landed else {
        panic!("expected a landed fish, got {landed:?}");
    };
    assert_eq!(catch.fish_id, "roach");

    w.input(Input::KeepCatch { rod: 0 }).unwrap();
    let s = w.session().unwrap();
    assert_eq!(s.keepnet.len(), 1);
    assert!(matches!(s.rods[0].state, RodState::Idle));
}

#[test]
fn cast_on_land_returns_to_idle() {
    let mut w = world();
    // Max cast 25 m clamps y towards the horizon; aim at land near the top.
    w.rods_mut()[0].tackle.max_cast_m = 1000.0;
    w.input(Input::Cast {
        rod: 0,
        x: 0.5,
        y: 0.42,
    })
    .unwrap();
    assert!(run_until(&mut w, 400, |e| matches!(e, GameEvent::CastOnLand { .. })).is_some());
    assert!(matches!(w.session().unwrap().rods[0].state, RodState::Idle));
}

#[test]
fn cast_is_clamped_to_max_distance() {
    let mut w = world();
    w.input(Input::Cast {
        rod: 0,
        x: 0.5,
        y: 0.46,
    })
    .unwrap();
    let s = w.session().unwrap();
    let RodState::Casting { target, .. } = s.rods[0].state else {
        panic!("not casting");
    };
    let d = s.spot.view.distance_at(target.y);
    assert!(d < 25.0 * 1.15, "cast {d} m");
}

#[test]
fn float_deeper_than_water_lies_flat() {
    let mut t = Tackle::starter_float();
    t.float_depth_m = 3.0;
    let cell = crate::map::Cell {
        depth_m: 2.0,
        bottom: crate::map::Bottom::Silt,
        zone: 1,
    };
    assert_eq!(t.bait_depth(&cell), (2.0, true));
    assert_eq!(t.kind, RodKind::Float);
}

#[test]
fn snapshot_serializes() {
    let mut w = world();
    w.debug = true;
    w.input(Input::Cast {
        rod: 0,
        x: 0.5,
        y: 0.7,
    })
    .unwrap();
    for _ in 0..100 {
        w.step();
    }
    let json = serde_json::to_string(&w.snapshot()).unwrap();
    assert!(json.contains("\"state\":\"Waiting\""), "{json}");
}

#[test]
fn bad_rod_index_is_an_error() {
    let mut w = world();
    assert_eq!(w.input(Input::Strike { rod: 5 }), Err(SimError::NoRod(5)));
}

impl World {
    fn rods_mut(&mut self) -> &mut Vec<Rod> {
        &mut self.session.as_mut().unwrap().rods
    }
}

#[test]
fn feeder_cast_lays_groundbait() {
    let mut w = World::new(content(), 99);
    w.start_session(
        "pond",
        "dam",
        map(),
        vec![Tackle::starter_float(), Tackle::starter_feeder()],
    )
    .unwrap();
    w.input(Input::Cast {
        rod: 1,
        x: 0.5,
        y: 0.6,
    })
    .unwrap();
    assert!(
        run_until(&mut w, 400, |e| matches!(
            e,
            GameEvent::CastLanded { rod: 1 }
        ))
        .is_some()
    );
    let s = w.session().unwrap();
    assert_eq!(s.groundbait.len(), 1);
    assert_eq!(s.active, 1, "casting selects the rod");
}

/// Content with a lure and a predator that takes it, for spinning tests.
fn spinning_world(seed: u64) -> World {
    let mut c = Arc::try_unwrap(content()).unwrap();
    let mut fish = c.fish.clone();
    fish.push(FishDef {
        id: "pike".into(),
        name: "Щука".into(),
        weight: WeightDist {
            min: 0.5,
            typical: 1.0,
            max: 3.0,
        },
        trophy_kg: 2.0,
        horizon: Horizon::Any,
        depth: Range { min: 0.5, max: 4.0 },
        activity: vec![],
        baits: BTreeMap::from([("spinner".to_string(), 1.0)]),
        strength: 1.5,
        bite: BiteStyle::Predator,
        price_per_kg: 100.0,
        xp_per_kg: 10.0,
        weather: BTreeMap::new(),
    });
    let mut baits = c.baits.clone();
    baits.push(BaitDef {
        id: "spinner".into(),
        name: "Вертушка".into(),
        kind: BaitKind::Lure,
        price: 90.0,
        pack: 1,
        rank: 0,
        lure: Some(LureDef {
            speed: Range { min: 0.5, max: 0.9 },
            depth: 0.8,
            sink_mps: 0.3,
            pause: 0.2,
        }),
    });
    let mut locations = std::mem::take(&mut c.locations);
    locations[0].spots[0].zones[0]
        .fish
        .insert("pike".to_string(), 1.0);
    let content = Arc::new(Content::new(fish, baits, locations).unwrap());
    let mut w = World::new(content, seed);
    w.start_session("pond", "dam", map(), vec![Tackle::starter_spinning()])
        .unwrap();
    w
}

#[test]
fn spinning_retrieve_gets_strikes() {
    let mut w = spinning_world(5);
    let mut strikes = 0;
    for _ in 0..30 {
        w.input(Input::Cast {
            rod: 0,
            x: 0.5,
            y: 0.5,
        })
        .unwrap();
        run_until(&mut w, 400, |e| matches!(e, GameEvent::CastLanded { .. }));
        w.input(Input::Reel { rod: 0, on: true }).unwrap();
        // Reel until the lure is back, a fish is on, or the bite times out.
        for _ in 0..20 * 120 {
            w.step();
            if w.drain_events()
                .iter()
                .any(|e| matches!(e, GameEvent::Bite { .. }))
            {
                strikes += 1;
            }
            let st = &w.session().unwrap().rods[0].state;
            if matches!(st, RodState::Idle) {
                break;
            }
            if matches!(st, RodState::Fighting { .. } | RodState::Landed { .. }) {
                // Reset the rod for the next cast.
                w.rods_mut()[0].state = RodState::Idle;
                break;
            }
        }
    }
    assert!(strikes >= 5, "only {strikes} strikes in 30 retrieves");
}

#[test]
fn spinning_rejects_natural_bait() {
    let mut w = spinning_world(1);
    assert!(matches!(
        w.input(Input::SetBait {
            rod: 0,
            bait: "worm".into()
        }),
        Err(SimError::WrongBait(_))
    ));
}

#[test]
fn snagged_lure_is_freed_or_lost_on_pull() {
    let mut w = spinning_world(3);
    let view = w.session().unwrap().spot.view;
    let run = spin::LureRun::new(Point { x: 0.5, y: 0.7 }, &view);
    w.rods_mut()[0].state = RodState::Snagged { run };
    w.input(Input::Retrieve { rod: 0 }).unwrap();
    let ev = w.drain_events();
    assert!(
        ev.iter()
            .any(|e| matches!(e, GameEvent::Unsnagged { .. } | GameEvent::LineBroke { .. }))
    );
}

/// World with the shipped content and the starter profile.
fn economy_world() -> World {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let c = Arc::new(crate::content::load_dir(&dir).unwrap());
    let mut w = World::new(c.clone(), 77);
    let mut p = crate::profile::Profile::starter(&c);
    p.buy_trip(&c, "pond", 3).unwrap();
    w.set_profile(Some(p));
    w
}

fn cast_and_land(w: &mut World, rod: usize) {
    w.input(Input::Cast {
        rod,
        x: 0.5,
        y: 0.7,
    })
    .unwrap();
    run_until(w, 400, |e| matches!(e, GameEvent::CastLanded { .. }));
}

#[test]
fn go_fishing_uses_assembled_slots() {
    let mut w = economy_world();
    w.go_fishing("pond", "willow", map()).unwrap();
    let s = w.session().unwrap();
    assert_eq!(s.rods.len(), 1, "starter kit has one rod");
    assert_eq!(s.slots, vec![0]);
    assert_eq!(s.rods[0].tackle.max_cast_m, 18.0);
    assert!(w.profile().unwrap().at.is_some());
}

#[test]
fn no_tackle_no_fishing() {
    let mut w = economy_world();
    w.update_profile(|p, _| p.slots[0].rod = None).unwrap();
    assert_eq!(
        w.go_fishing("pond", "willow", map()),
        Err(SimError::NoTackle)
    );
}

#[test]
fn cast_without_hooks_is_refused() {
    let mut w = economy_world();
    w.update_profile(|p, _| p.items.remove("hooks")).unwrap();
    w.go_fishing("pond", "willow", map()).unwrap();
    w.input(Input::Cast {
        rod: 0,
        x: 0.5,
        y: 0.7,
    })
    .unwrap();
    let ev = w.drain_events();
    assert!(
        matches!(&ev[..], [GameEvent::OutOf { rod: 0, item }] if item == "Крючки"),
        "{ev:?}"
    );
    assert!(matches!(w.session().unwrap().rods[0].state, RodState::Idle));
}

#[test]
fn broken_line_loses_the_rig() {
    let mut w = economy_world();
    w.go_fishing("pond", "willow", map()).unwrap();
    let p = w.profile().unwrap();
    let (hooks, floats) = (p.count("hooks"), p.count("floats"));
    let view = w.session().unwrap().spot.view;
    let snag = || RodState::Snagged {
        run: spin::LureRun::new(Point { x: 0.5, y: 0.7 }, &view),
    };
    // Pull until the line snaps (a lucky pull just frees the rig).
    for _ in 0..50 {
        w.rods_mut()[0].state = snag();
        w.input(Input::Retrieve { rod: 0 }).unwrap();
        if w.drain_events()
            .iter()
            .any(|e| matches!(e, GameEvent::LineBroke { .. }))
        {
            break;
        }
    }
    let p = w.profile().unwrap();
    assert_eq!(p.count("hooks"), hooks - 1);
    assert_eq!(p.count("floats"), floats - 1);
}

#[test]
fn feeder_without_groundbait_lays_no_patch() {
    let mut w = economy_world();
    w.update_profile(|p, c| {
        p.items.insert("rod_feeder_light".into(), 1);
        p.items.insert("feeders".into(), 2);
        p.xp = 1000.0;
        p.equip(c, 0, crate::profile::Part::Rod, Some("rod_feeder_light"))
            .unwrap();
    })
    .unwrap();
    w.go_fishing("pond", "willow", map()).unwrap();
    cast_and_land(&mut w, 0);
    assert!(w.session().unwrap().groundbait.is_empty());

    w.input(Input::Retrieve { rod: 0 }).unwrap();
    run_until(&mut w, 100, |_| false);
    w.update_profile(|p, _| p.items.insert("groundbait".into(), 1))
        .unwrap();
    cast_and_land(&mut w, 0);
    assert_eq!(w.session().unwrap().groundbait.len(), 1);
    assert_eq!(w.profile().unwrap().count("groundbait"), 0);
}

#[test]
fn leaving_water_keeps_catch_and_settings() {
    let mut w = economy_world();
    w.go_fishing("pond", "willow", map()).unwrap();
    w.input(Input::SetFloatDepth { rod: 0, m: 2.2 }).unwrap();
    let catch = Catch {
        id: 1,
        fish_id: "roach".into(),
        name: "Плотва".into(),
        weight_kg: 0.4,
        trophy: false,
        bait: "worm".into(),
        at_min: 0.0,
        record: false,
    };
    w.rods_mut()[0].state = RodState::Landed { catch };
    for _ in 0..200 {
        w.step();
    }
    w.leave_water().unwrap();
    let p = w.profile().unwrap();
    assert_eq!(p.keepnet.len(), 1, "fish in the landing net is kept");
    assert!((p.slots[0].float_depth_m - 2.2).abs() < 1e-9);
    assert!(p.clock_min > crate::clock::Clock::default().minutes);
    assert!(p.at.is_none() && w.session().is_none());

    let money = w.profile().unwrap().money;
    let report = w.sell_catch().unwrap();
    assert_eq!(report.lines.len(), 1);
    assert!((w.profile().unwrap().money - money - report.money).abs() < 1e-9);
    assert!(w.profile().unwrap().keepnet.is_empty());
}

#[test]
fn no_pass_no_fishing() {
    let mut w = economy_world();
    w.end_trip().unwrap();
    assert_eq!(
        w.go_fishing("pond", "willow", map()),
        Err(SimError::NoPass("pond".into()))
    );
}

#[test]
fn session_ends_when_the_pass_runs_out() {
    let mut w = economy_world();
    let ends = w.profile().unwrap().trip.as_ref().unwrap().ends_min;
    w.go_fishing("pond", "willow", map()).unwrap();
    w.session.as_mut().unwrap().clock.minutes = ends - 0.5;
    let over = run_until(&mut w, 20 * 60, |e| matches!(e, GameEvent::TripOver));
    assert!(over.is_some());
    assert!(w.session().is_none());
    assert!(w.profile().unwrap().trip_over());
}

#[test]
fn unowned_bait_cannot_be_put_on() {
    let mut w = economy_world();
    w.go_fishing("pond", "willow", map()).unwrap();
    assert_eq!(
        w.input(Input::SetBait {
            rod: 0,
            bait: "bread".into()
        }),
        Err(SimError::NotOwned("bread".into()))
    );
}

#[test]
fn save_is_taken_only_when_dirty() {
    let mut w = economy_world();
    assert!(w.take_save().is_some());
    assert!(w.take_save().is_none());
    w.go_fishing("pond", "willow", map()).unwrap();
    w.step();
    assert!(w.take_save().is_some_and(|p| p.at.is_some()));
}

#[test]
fn pause_stops_the_clock() {
    let mut w = world();
    w.input(Input::Pause { on: true }).unwrap();
    let before = w.session().unwrap().clock.minutes;
    for _ in 0..100 {
        w.step();
    }
    assert_eq!(w.session().unwrap().clock.minutes, before);
    w.input(Input::Pause { on: false }).unwrap();
    w.step();
    assert!(w.session().unwrap().clock.minutes > before);
}

#[test]
fn landed_fish_is_logged_and_sets_records() {
    let mut w = economy_world();
    w.go_fishing("pond", "willow", map()).unwrap();
    let land = |w: &mut World, kg: f64| {
        let catch = Catch {
            id: 9,
            fish_id: "perch".into(),
            name: "Окунь".into(),
            weight_kg: kg,
            trophy: false,
            bait: "worm".into(),
            at_min: 400.0,
            record: false,
        };
        w.rods_mut()[0].state = RodState::Landed {
            catch: catch.clone(),
        };
        let from = w.events.len();
        w.events.push(GameEvent::Landed { rod: 0, catch });
        w.settle(from);
        w.drain_events()
    };
    let ev = land(&mut w, 0.3);
    assert!(
        ev.iter()
            .any(|e| matches!(e, GameEvent::NewRecord { first: true, .. }))
    );
    assert!(
        matches!(&w.session().unwrap().rods[0].state, RodState::Landed { catch } if catch.record)
    );
    let ev = land(&mut w, 0.2);
    assert!(!ev.iter().any(|e| matches!(e, GameEvent::NewRecord { .. })));

    let journal = w.take_journal();
    assert_eq!(journal.len(), 2);
    let JournalEntry::Catch(log) = &journal[0] else {
        panic!("expected a catch log");
    };
    assert_eq!(log.bait, "Червь");
    assert_eq!(log.location, "Пруд у деревни");
    assert_eq!(w.profile().unwrap().records["perch"].count, 2);

    w.end_trip().unwrap();
    let journal = w.take_journal();
    assert!(matches!(&journal[..], [JournalEntry::Trip(t)] if t.fish == 2));
}

#[test]
fn time_speed_applies_to_running_and_new_sessions() {
    let mut w = world();
    let before = w.session().unwrap().clock.minutes;
    w.set_time_speed(30.0);
    // One real second at 30 game minutes per real minute.
    for _ in 0..20 {
        w.step();
    }
    let passed = w.session().unwrap().clock.minutes - before;
    assert!((passed - 0.5).abs() < 1e-6, "{passed}");

    w.start_session("pond", "dam", map(), vec![Tackle::starter_float()])
        .unwrap();
    assert_eq!(w.session().unwrap().clock.speed, 30.0);
    w.set_time_speed(1e6);
    assert_eq!(w.session().unwrap().clock.speed, clock::MAX_SPEED);
}
