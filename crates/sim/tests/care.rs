//! Wounds, buddy aid, medics and bailed-out crews (prompt 049), on
//! hand-made maps with catalog units and fixed seeds.

use sim::map::MapLayout;
use sim::{
    AidView, CareEvent, Catalog, CombatRules, Command, FireMode, Map, PlayerCommand, PlayerId,
    Scale, Sim, SimConfig, SoldierSnapshot, Spawn, Stance, StanceOrder, TerrainRules, UnitId, Vec2,
    WoundView, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const FIRE: FireMode = FireMode::FireAtWill;
const HOLD: FireMode = FireMode::HoldFire;
const SQUAD: &str = "de_inf_rifle_squad";

/// Ticks of `ms` real milliseconds (20 Hz, infantry time factor 2).
const fn ticks(ms: u32) -> u32 {
    ms * 20 / 2000
}

fn map_with(objects: &[String]) -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "care_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let size = (n * n) as usize;
    let mut map = Map::from_rasters(layout, vec![0; size], vec![0; size]).unwrap();
    if !objects.is_empty() {
        map.add_objects(&format!(
            "{}\n{}\n",
            sim::objects::CSV_HEADER,
            objects.join("\n")
        ))
        .unwrap();
    }
    Arc::new(map)
}

/// Units start facing the map center. The match includes the crew squads
/// the vehicles need (as scenarios do).
fn sim_with(map: Arc<Map>, seed: u64, spawns: &[(PlayerId, &str, i32, i32, FireMode)]) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    let mut unit_types = unit_types_from_catalog(&catalog, &scale, &names).unwrap();
    let crews = sim::care::crew_types(&catalog, &scale, &unit_types).unwrap();
    unit_types.extend(crews);
    let config = SimConfig {
        tick_hz: 20,
        unit_types,
        spawns: spawns
            .iter()
            .map(|&(owner, unit, x, y, fire_mode)| Spawn {
                owner,
                unit_type: unit.into(),
                position: Vec2::from_meters(x, y),
                fire_mode,
            })
            .collect(),
        map: map.reference(),
        terrain: TerrainRules::builtin().unwrap(),
        combat: CombatRules::builtin().unwrap(),
        vision: sim::VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
    };
    Sim::new(&config, map, seed).unwrap()
}

fn order(player: PlayerId, command: Command) -> PlayerCommand {
    PlayerCommand::new(player, command)
}

fn move_to(player: PlayerId, unit: u32, x: i32, y: i32) -> PlayerCommand {
    order(
        player,
        Command::MoveTo {
            units: vec![UnitId(unit)],
            target: Vec2::from_meters(x, y),
        },
    )
}

fn run(sim: &mut Sim, ticks: u32) {
    for _ in 0..ticks {
        sim.step(&[]);
    }
}

fn soldier(sim: &Sim, unit: u32, index: u16) -> SoldierSnapshot {
    *sim.snapshot()
        .soldiers
        .iter()
        .find(|s| s.unit == UnitId(unit) && s.index == index)
        .expect("soldier in the snapshot")
}

fn unit_position(sim: &Sim, unit: u32) -> Vec2 {
    sim.snapshot()
        .units
        .iter()
        .find(|u| u.id == UnitId(unit))
        .unwrap()
        .position
}

#[test]
fn lightly_wounded_bandage_themselves_and_limp() {
    let mut sim = sim_with(map_with(&[]), 1, &[(P0, SQUAD, 0, 0, HOLD)]);
    run(&mut sim, 5);
    sim.wound_soldier(UnitId(0), 2, false);
    sim.step(&[]);
    assert_eq!(soldier(&sim, 0, 2).wound, WoundView::Bandaging);
    assert_eq!(sim.unit_wounded(UnitId(0)), (1, 0));
    let rules = *sim.care_rules();
    run(&mut sim, ticks(rules.wound.bandage_ms));
    assert_eq!(soldier(&sim, 0, 2).wound, WoundView::Limping);
    run(&mut sim, ticks(rules.wound.limp_ms));
    assert_eq!(soldier(&sim, 0, 2).wound, WoundView::Bandaged);
    assert_eq!(soldier(&sim, 0, 2).hp, 100);
}

#[test]
fn limping_men_slow_their_squad() {
    let distance = |wounded: bool| {
        let mut sim = sim_with(map_with(&[]), 1, &[(P0, SQUAD, 0, 0, HOLD)]);
        run(&mut sim, 5);
        if wounded {
            sim.wound_soldier(UnitId(0), 4, false);
        }
        sim.step(&[move_to(P0, 0, 0, -150)]);
        run(&mut sim, 200);
        (unit_position(&sim, 0) - Vec2::ZERO).length()
    };
    let (fit, limping) = (distance(false), distance(true));
    println!("fit {fit}, limping {limping}");
    assert!(limping * 10 < fit * 8, "fit {fit}, limping {limping}");
}

#[test]
fn heavily_wounded_die_without_help() {
    let mut sim = sim_with(map_with(&[]), 1, &[(P0, SQUAD, 0, 0, HOLD)]);
    run(&mut sim, 5);
    sim.wound_soldier(UnitId(0), 1, true);
    sim.step(&[]);
    assert_eq!(soldier(&sim, 0, 1).wound, WoundView::Down { stable: false });
    assert_eq!(sim.unit_wounded(UnitId(0)), (0, 1));
    let bleed = ticks(sim.care_rules().wound.bleed_ms);
    run(&mut sim, bleed - 5);
    assert!(soldier(&sim, 0, 1).hp > 0);
    let mut died = false;
    for _ in 0..10 {
        sim.step(&[]);
        died |= sim.care_events().iter().any(
            |e| matches!(e, CareEvent::DiedOfWounds { unit, soldier: 1 } if *unit == UnitId(0)),
        );
    }
    assert!(died);
    assert_eq!(soldier(&sim, 0, 1).hp, 0);
    assert_eq!(sim.unit_hp(UnitId(0)), Some(800));
}

#[test]
fn hits_wound_soldiers_light_and_heavy() {
    let (mut light, mut heavy) = (0, 0);
    for seed in 1..=4 {
        let mut sim = sim_with(
            map_with(&[]),
            seed,
            &[
                (P0, "su_inf_mg_team", 0, 120, FIRE),
                (P1, SQUAD, 0, 0, HOLD),
            ],
        );
        sim.step(&[order(
            P1,
            Command::SetStance {
                units: vec![UnitId(1)],
                stance: StanceOrder::Fixed(Stance::Stand),
            },
        )]);
        for _ in 0..600 {
            sim.step(&[]);
            for e in sim.care_events() {
                if let CareEvent::Wounded { heavy: h, .. } = e {
                    if *h {
                        heavy += 1;
                    } else {
                        light += 1;
                    }
                }
            }
        }
    }
    println!("light {light}, heavy {heavy}");
    assert!(light > 0 && heavy > 0, "light {light}, heavy {heavy}");
}

/// A row of sandbag positions 4 m north of the origin.
fn sandbags() -> Vec<String> {
    (-4..=4)
        .map(|k| format!("fort_sandbag_position,Position,{},-4000,90", k * 3000))
        .collect()
}

#[test]
fn comrades_drag_the_wounded_into_cover() {
    let mut sim = sim_with(map_with(&sandbags()), 1, &[(P0, SQUAD, 0, 0, HOLD)]);
    sim.step(&[]);
    // Down before he reached his cover spot.
    sim.wound_soldier(UnitId(0), 6, true);
    assert!(!soldier(&sim, 0, 6).in_cover);
    let mut dragged = false;
    for _ in 0..300 {
        sim.step(&[]);
        let snap = sim.snapshot();
        dragged |= snap.soldiers.iter().any(|s| {
            matches!(
                s.aid,
                AidView::Drag {
                    soldier: 6,
                    pulling: true
                }
            )
        });
    }
    assert!(dragged, "nobody dragged him");
    let s = soldier(&sim, 0, 6);
    assert!(s.in_cover, "{:?}", s.position);
    assert!(matches!(s.wound, WoundView::Down { .. }));
}

#[test]
fn squads_carry_their_wounded_along() {
    let mut sim = sim_with(map_with(&[]), 1, &[(P0, SQUAD, 0, 0, HOLD)]);
    run(&mut sim, 40);
    sim.wound_soldier(UnitId(0), 3, true);
    sim.step(&[move_to(P0, 0, 0, -60)]);
    let mut carried = false;
    for _ in 0..1500 {
        sim.step(&[]);
        carried |= matches!(soldier(&sim, 0, 3).aid, AidView::Dragged);
    }
    assert!(carried);
    let squad = unit_position(&sim, 0);
    assert!(
        (squad - Vec2::from_meters(0, -60)).length() < sim::Fixed::from_num(3),
        "{squad:?}"
    );
    let s = soldier(&sim, 0, 3);
    assert!((s.position - squad).length() < sim::Fixed::from_num(12));
}

#[test]
fn comrades_go_to_the_knocked_down() {
    // An 8 cm mortar shell on a standing squad (as in tests/knockdown.rs).
    let mut helped = 0;
    for seed in 1..=6 {
        let mut sim = sim_with(
            map_with(&[]),
            seed,
            &[
                (P0, "de_inf_mortar_medium", 0, 150, HOLD),
                (P1, "su_inf_rifle_squad", 0, 0, HOLD),
            ],
        );
        sim.step(&[order(
            P1,
            Command::SetStance {
                units: vec![UnitId(1)],
                stance: StanceOrder::Fixed(Stance::Stand),
            },
        )]);
        run(&mut sim, 60);
        sim.step(&[order(
            P0,
            Command::Barrage {
                units: vec![UnitId(0)],
                target: Vec2::ZERO,
                radius_m: 0,
                salvos: 1,
                smoke: false,
            },
        )]);
        for _ in 0..1200u32 {
            sim.step(&[]);
            let snap = sim.snapshot();
            for s in snap.soldiers.iter().filter(|s| s.unit == UnitId(1)) {
                if let AidView::HelpUp { soldier } = s.aid {
                    let other = snap
                        .soldiers
                        .iter()
                        .find(|o| o.unit == UnitId(1) && o.index == soldier)
                        .unwrap();
                    assert!(other.knocked > 0 && other.hp > 0);
                    helped += 1;
                }
            }
        }
    }
    assert!(helped > 0, "nobody helped");
}

#[test]
fn an_aid_man_treats_the_wounded_where_they_lie() {
    let mut sim = sim_with(
        map_with(&[]),
        1,
        &[
            (P0, "us_inf_rifle_squad", 0, 0, HOLD),
            (P0, "us_inf_medic", 30, 0, HOLD),
        ],
    );
    run(&mut sim, 40);
    sim.wound_soldier(UnitId(0), 5, true);
    let mut treated = false;
    let mut treating = false;
    for _ in 0..1200 {
        sim.step(&[]);
        treating |= sim
            .snapshot()
            .soldiers
            .iter()
            .any(|s| s.unit == UnitId(1) && matches!(s.aid, AidView::Treat { soldier: 5, .. }));
        treated |= sim.care_events().iter().any(|e| {
            matches!(
                e,
                CareEvent::Treated {
                    medic: UnitId(1),
                    soldier: 5,
                    ..
                }
            )
        });
        if treated {
            break;
        }
    }
    assert!(treating && treated);
    let s = soldier(&sim, 0, 5);
    assert_eq!(s.wound, WoundView::Limping);
    assert!(s.hp > 0);
}

#[test]
fn a_medic_team_carries_the_wounded_on_a_stretcher() {
    let mut sim = sim_with(
        map_with(&[]),
        1,
        &[(P0, SQUAD, 0, 0, HOLD), (P0, "de_inf_medic", 0, -40, HOLD)],
    );
    run(&mut sim, 40);
    let post = unit_position(&sim, 1);
    sim.wound_soldier(UnitId(0), 2, true);
    let (mut on_stretcher, mut bearers, mut treated) = (false, 0, false);
    for _ in 0..3000 {
        sim.step(&[]);
        let snap = sim.snapshot();
        on_stretcher |= snap
            .soldiers
            .iter()
            .any(|s| s.unit == UnitId(0) && s.index == 2 && s.aid == AidView::OnStretcher);
        bearers = bearers.max(
            snap.soldiers
                .iter()
                .filter(|s| matches!(s.aid, AidView::Stretcher { soldier: 2, .. }))
                .count(),
        );
        if sim
            .care_events()
            .iter()
            .any(|e| matches!(e, CareEvent::Treated { soldier: 2, .. }))
        {
            treated = true;
            break;
        }
    }
    assert!(on_stretcher && treated);
    assert_eq!(bearers, 2);
    // Treated at the team's post, not where he fell.
    let s = soldier(&sim, 0, 2);
    assert!(
        (s.position - post).length() < sim::Fixed::from_num(8),
        "{:?}",
        s.position
    );
    assert_eq!(s.wound, WoundView::Limping);
    // He walks back to his squad.
    run(&mut sim, 600);
    let s = soldier(&sim, 0, 2);
    assert!((s.position - unit_position(&sim, 0)).length() < sim::Fixed::from_num(10));
}

#[test]
fn crews_bail_out_of_destroyed_tanks() {
    let mut seen = Vec::new();
    for seed in 1..=8 {
        let mut sim = sim_with(
            map_with(&[]),
            seed,
            &[
                (P0, "de_pz4_h", 0, 300, FIRE),
                (P0, "de_pz4_h", 20, 300, FIRE),
                (P1, "us_m4a3_75", 0, 0, FIRE),
            ],
        );
        for _ in 0..2000 {
            sim.step(&[]);
            for e in sim.care_events() {
                if let CareEvent::BailedOut { vehicle, crew, men } = *e {
                    seen.push(men);
                    assert!((1..=5).contains(&men));
                    assert!(sim.is_bailed_crew(crew));
                    let snap = sim.snapshot();
                    let crew_unit = snap.units.iter().find(|u| u.id == crew).unwrap();
                    let vehicle_unit = snap.units.iter().find(|u| u.id == vehicle).unwrap();
                    assert_eq!(crew_unit.owner, vehicle_unit.owner);
                    assert_eq!(crew_unit.hp, u32::from(men) * 100);
                    assert!(
                        (crew_unit.position - vehicle_unit.position).length()
                            < sim::Fixed::from_num(15)
                    );
                    let gone = snap
                        .soldiers
                        .iter()
                        .filter(|s| s.unit == crew && s.wound == WoundView::Gone)
                        .count();
                    assert_eq!(gone, 5 - usize::from(men));
                }
            }
            if sim.outcome().is_some() {
                break;
            }
        }
        // Crews do not keep a player in the match.
        if let Some(o) = sim.outcome() {
            let loser = if o.winner == Some(P0) { P1 } else { P0 };
            assert_eq!(sim.alive_units(loser), 0);
        }
    }
    println!("crews: {seen:?}");
    assert!(!seen.is_empty(), "no crew bailed out");
}

#[test]
fn care_runs_are_deterministic() {
    let run_once = || {
        let mut sim = sim_with(
            map_with(&sandbags()),
            7,
            &[
                (P0, "su_inf_mg_team", 0, 120, FIRE),
                (P1, SQUAD, 0, 0, FIRE),
                (P1, "de_inf_medic", 0, -40, HOLD),
            ],
        );
        run(&mut sim, 1500);
        sim.state_hash()
    };
    assert_eq!(run_once(), run_once());
}
