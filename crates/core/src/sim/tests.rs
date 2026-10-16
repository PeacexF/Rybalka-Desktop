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
    }];
    let baits = vec![BaitDef {
        id: "worm".into(),
        name: "Червь".into(),
        kind: BaitKind::Natural,
        lure: None,
    }];
    let locations = vec![LocationDef {
        id: "pond".into(),
        name: "Пруд".into(),
        spots: vec![SpotDef {
            id: "dam".into(),
            name: "Плотина".into(),
            background: "bg.png".into(),
            map: "map.png".into(),
            view: View {
                horizon_y: 0.4,
                shore_y: 0.9,
                near_m: 3.0,
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
    });
    let mut baits = c.baits.clone();
    baits.push(BaitDef {
        id: "spinner".into(),
        name: "Вертушка".into(),
        kind: BaitKind::Lure,
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
