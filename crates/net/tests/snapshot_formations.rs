//! Prompt 038 with 046: snapshots keep attack-move, secured areas and
//! facings.

use net::snapshot;
use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, DrillView, Engage, FireMode, Formation, Map, PlayerCommand,
    PlayerId, Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn flat_map() -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "snapshot_formations".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap())
}

fn setup(map: Arc<Map>) -> Sim {
    // Prompt 097 halved direct-fire range (75 mm gun 87.5 m) and vehicle
    // speed: the truck the tanks halt for stands 50 m beside their path
    // (was 100 m), and the tanks start 150 m short of it (was 300 m), so
    // they halt after about 350 ticks, well inside the limit below.
    let spawns = [
        (P0, "de_pz4_h", 0, -150, FireMode::FireAtWill),
        (P0, "de_pz4_h", 20, -150, FireMode::FireAtWill),
        (P0, "de_inf_rifle_squad", -100, 0, FireMode::FireAtWill),
        (P1, "de_opel_blitz", 50, 0, FireMode::HoldFire),
        (P1, "de_opel_blitz", -100, 270, FireMode::HoldFire),
    ];
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
        mission: None,
        variant: None,
    };
    Sim::new(&config, map, 11).unwrap()
}

/// Prompt 046: a snapshot taken while two tanks attack-move (one halted to
/// fight) and a squad secures an area continues tick for tick like the
/// original.
#[test]
fn snapshot_during_attack_move_and_secure() {
    let map = flat_map();
    let mut original = setup(map.clone());
    original.step(&[
        PlayerCommand::new(
            P0,
            Command::FormationMove {
                units: vec![UnitId(0), UnitId(1)],
                target: Vec2::from_meters(0, 300),
                formation: Formation::Wedge,
                facing: Some(Vec2::from_meters(0, 1)),
                engage: Engage::All,
            },
        ),
        PlayerCommand::new(
            P0,
            Command::Secure {
                units: vec![UnitId(2)],
                center: Vec2::from_meters(-100, 0),
                radius_m: 300,
                formation: Formation::Auto,
                facing: None,
            },
        ),
    ]);
    let mut ticks = 0;
    while !matches!(
        original.drill_view(UnitId(0)),
        Some(DrillView::Advance(_, true))
    ) {
        original.step(&[]);
        ticks += 1;
        assert!(ticks < 20 * 60, "the tank never halted");
    }
    assert!(matches!(
        original.drill_view(UnitId(2)),
        Some(DrillView::Secure(..))
    ));
    let bytes = snapshot::encode(&original, &map);
    let mut restored = snapshot::decode(&bytes, map).expect("snapshot restores");
    assert_eq!(restored.state_hash(), original.state_hash());
    assert_eq!(restored.snapshot(), original.snapshot());
    for tick in 0..20 * 60 {
        original.step(&[]);
        restored.step(&[]);
        assert_eq!(
            restored.state_hash(),
            original.state_hash(),
            "diverged {} ticks after the restore",
            tick + 1
        );
    }
    assert_eq!(
        restored.drill_view(UnitId(2)),
        original.drill_view(UnitId(2))
    );
}
