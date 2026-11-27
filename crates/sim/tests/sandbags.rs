//! Engineer-built sandbag walls and drag-line preview (prompt 235).

use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, FireMode, Fixed, LayKind, Map, PlayerCommand, PlayerId, Scale,
    Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2, WorkKind, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const ENGINEERS: &str = "de_inf_engineer";
const RIFLES: &str = "de_inf_rifle_squad";
const ENEMY_RIFLES: &str = "su_inf_rifle_squad";
const TRUCK: &str = "de_opel_blitz";
const TANK: &str = "su_t34_76_1943";

fn flat_map(water_x_from: Option<u32>) -> Arc<Map> {
    let n = 201;
    let layout = MapLayout {
        name: "sandbags_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    let mut terrain = vec![0u8; count];
    if let Some(wx) = water_x_from {
        // Terrain id 3 in builtin terrain.toml is deep water (impassable).
        let water_id = TerrainRules::builtin()
            .unwrap()
            .types
            .iter()
            .position(|t| !t.passable)
            .expect("impassable terrain type") as u8;
        for y in 0..n {
            for x in wx..n {
                terrain[(y * n + x) as usize] = water_id;
            }
        }
    }
    let map = Map::from_rasters(layout, vec![0; count], terrain).unwrap();
    Arc::new(map)
}

fn sim_on(map: Arc<Map>, spawns: &[(PlayerId, &str, i32, i32)], seed: u64) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    let config = SimConfig {
        tick_hz: 20,
        unit_types: unit_types_from_catalog(&catalog, &scale, &names).unwrap(),
        spawns: spawns
            .iter()
            .map(|&(owner, unit, x, y)| Spawn {
                owner,
                unit_type: unit.into(),
                position: Vec2::from_meters(x, y),
                fire_mode: FireMode::HoldFire,
            })
            .collect(),
        map: map.reference(),
        terrain: TerrainRules::builtin().unwrap(),
        combat: CombatRules::builtin().unwrap(),
        vision: sim::VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    };
    Sim::new(&config, map, seed).unwrap()
}

fn cmd(player: PlayerId, command: Command) -> PlayerCommand {
    PlayerCommand::new(player, command)
}

#[test]
fn sandbag_line_builds_gives_heavy_cover_and_passes_infantry() {
    let mut sim = sim_on(
        flat_map(None),
        &[(P0, ENGINEERS, 0, -4), (P0, RIFLES, 0, -10)],
        7,
    );
    let eng = UnitId(0);
    let rifle = UnitId(1);
    let before = sim.account(P0).unwrap();

    // Lay a 12 m sandbag wall along x = -6..6 at y = 0 (4 sections of 3 m).
    sim.step(&[cmd(
        P0,
        Command::Lay {
            units: vec![eng],
            kind: LayKind::Sandbags,
            from: Vec2::from_meters(-6, 0),
            to: Vec2::from_meters(6, 0),
        },
    )]);
    let after = sim.account(P0).unwrap();
    assert_eq!(
        before[2] - after[2],
        20,
        "4 sandbag sections cost 20 manpower"
    );

    // Step until all 4 sections finish (4 * 5 s = 20 s plus walking).
    for _ in 0..800 {
        sim.step(&[]);
        let built = sim
            .works_for(P0)
            .into_iter()
            .filter(|w| w.kind == WorkKind::Sandbags && w.built)
            .count();
        if built == 4 {
            break;
        }
    }
    let built: Vec<_> = sim
        .works_for(P0)
        .into_iter()
        .filter(|w| w.kind == WorkKind::Sandbags && w.built)
        .collect();
    assert_eq!(built.len(), 4, "all 4 sandbag sections should finish");

    // Infantry must be able to walk right across the sandbag wall.
    let rifle_profile = sim
        .snapshot()
        .units
        .iter()
        .find(|u| u.id == rifle)
        .unwrap()
        .unit_type as usize;
    assert!(
        sim.path_grid()
            .passable_for(rifle_profile, Vec2::from_meters(0, 0)),
        "infantry passes freely over sandbag walls"
    );

    // Cover preview & TakeCover on the south side facing north across the wall
    // must assign heavy cover (grade 2).
    let preview = sim.cover_preview(P0, &[rifle], Vec2::from_meters(0, -1), None);
    assert!(
        preview.iter().any(|s| s.grade == 2),
        "cover preview behind sandbag wall must show heavy cover (grade 2)"
    );
    sim.step(&[cmd(
        P0,
        Command::TakeCover {
            units: vec![rifle],
            target: Vec2::from_meters(0, -1),
            facing: None,
        },
    )]);
    for _ in 0..240 {
        sim.step(&[]);
    }
    assert_eq!(
        sim.unit_cover_grade(rifle),
        2,
        "squad settled behind sandbag wall must have heavy cover grade 2"
    );
}

#[test]
fn sandbags_block_light_vehicles_and_are_crushed_by_tanks() {
    let mut sim = sim_on(
        flat_map(None),
        &[
            (P0, ENGINEERS, 0, -4),
            (P0, TRUCK, -20, -15),
            (P1, TANK, 0, -14),
        ],
        11,
    );
    let truck = UnitId(1);
    let tank = UnitId(2);

    // Wide 90 m sandbag line (-45..45) so driving straight through is shorter
    // than detouring around the ends (detour_m = 40 m).
    let placed = sim.place_finished_work(
        WorkKind::Sandbags,
        P0,
        Vec2::from_meters(-45, 0),
        Vec2::from_meters(45, 0),
    );
    assert_eq!(placed, 30);

    let snap = sim.snapshot();
    let truck_profile = snap.units.iter().find(|u| u.id == truck).unwrap().unit_type as usize;
    let tank_profile = snap.units.iter().find(|u| u.id == tank).unwrap().unit_type as usize;

    // Light truck (3.1 t < 12 t) is blocked along the whole wall; T-34
    // (30.9 t >= 12 t) can path right through.
    let before_works: Vec<_> = sim
        .works_for(P0)
        .into_iter()
        .filter(|w| w.kind == WorkKind::Sandbags && w.built)
        .collect();
    assert!(
        !sim.path_grid()
            .passable_for(truck_profile, Vec2::from_meters(0, 0)),
        "light truck must be blocked by sandbag wall"
    );
    assert!(
        sim.path_grid()
            .passable_for(tank_profile, Vec2::from_meters(0, 0)),
        "medium tank must be able to path through crushable sandbag wall"
    );

    // Drive the T-34 straight through the sandbag wall and verify it crushes pieces.
    sim.step(&[cmd(
        P1,
        Command::MoveTo {
            units: vec![tank],
            target: Vec2::from_meters(0, 14),
        },
    )]);
    for _ in 0..300 {
        sim.step(&[]);
    }
    let after_ids: Vec<u32> = sim
        .works_for(P0)
        .into_iter()
        .filter(|w| w.kind == WorkKind::Sandbags && w.built)
        .map(|w| w.id)
        .collect();
    let crushed: Vec<_> = before_works
        .iter()
        .filter(|w| !after_ids.contains(&w.id))
        .collect();
    assert!(
        !crushed.is_empty(),
        "tank rolling over sandbag line must crush sections (remaining: {})",
        after_ids.len()
    );
    assert!(
        crushed.iter().any(|w| {
            let mid = Vec2::new((w.from.x + w.to.x) / 2, (w.from.y + w.to.y) / 2);
            sim.path_grid().passable_for(truck_profile, mid)
        }),
        "gap crushed by tank must become passable for light vehicles again"
    );
}

#[test]
fn sandbags_are_cleared_and_previewed_without_mutating_state() {
    // Impassable water starts at x_sample = 105 (+10 m in world coords).
    let mut sim = sim_on(
        flat_map(Some(105)),
        &[(P0, ENGINEERS, -6, -4), (P1, ENEMY_RIFLES, 60, 0)],
        19,
    );
    let eng = UnitId(0);
    let hash_before = sim.state_hash();

    // Preview from x = -6 m to x = 24 m crosses into water: first sections valid (0),
    // water sections invalid (1).
    let preview = sim.work_line_preview(
        P0,
        WorkKind::Sandbags,
        Vec2::from_meters(-6, 0),
        Vec2::from_meters(24, 0),
    );
    assert!(!preview.is_empty());
    assert!(
        preview.iter().any(|&(_, _, s)| s == 0),
        "valid dry sections"
    );
    assert!(
        preview.iter().any(|&(_, _, s)| s == 1),
        "impassable water sections must report status 1"
    );
    assert_eq!(
        sim.state_hash(),
        hash_before,
        "work_line_preview must be purely read-only"
    );

    // Place and clear sandbags with engineers.
    let placed = sim.place_finished_work(
        WorkKind::Sandbags,
        P0,
        Vec2::from_meters(-3, 0),
        Vec2::from_meters(3, 0),
    );
    assert_eq!(placed, 2);
    sim.step(&[cmd(
        P0,
        Command::Clear {
            units: vec![eng],
            target: Vec2::from_meters(0, 0),
        },
    )]);
    for _ in 0..400 {
        sim.step(&[]);
    }
    let remaining = sim
        .works_for(P0)
        .into_iter()
        .filter(|w| w.kind == WorkKind::Sandbags)
        .count();
    assert_eq!(remaining, 0, "engineers should clear sandbag sections");
}

#[test]
fn sandbags_are_deterministic() {
    let run = || {
        let mut sim = sim_on(
            flat_map(None),
            &[(P0, ENGINEERS, 0, -4), (P1, TANK, 0, -18)],
            42,
        );
        sim.step(&[cmd(
            P0,
            Command::Lay {
                units: vec![UnitId(0)],
                kind: LayKind::Sandbags,
                from: Vec2::from_meters(-6, 0),
                to: Vec2::from_meters(6, 0),
            },
        )]);
        for _ in 0..400 {
            sim.step(&[]);
        }
        sim.step(&[cmd(
            P1,
            Command::MoveTo {
                units: vec![UnitId(1)],
                target: Vec2::from_meters(0, 12),
            },
        )]);
        for _ in 0..250 {
            sim.step(&[]);
        }
        (
            sim.state_hash(),
            Fixed::from_num(sim.works_for(P0).len() as i32),
        )
    };
    assert_eq!(
        run(),
        run(),
        "sandbag building and crushing must be deterministic"
    );
}
