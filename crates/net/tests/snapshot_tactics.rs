//! Prompt 038 with 047: snapshots keep fire teams in the middle of a bound,
//! suppression, blind fire and jams.

use net::snapshot;
use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, Engage, FireMode, Formation, Map, PlayerCommand, PlayerId,
    Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const FIRE: FireMode = FireMode::FireAtWill;

fn flat_map() -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "snapshot_tactics".into(),
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
    let spawns = [
        (P0, "de_inf_rifle_squad", 0, 50, FIRE),
        (P0, "de_inf_rifle_squad", 30, 50, FIRE),
        (P1, "su_inf_rifle_squad", 0, -50, FIRE),
        (P1, "su_inf_mg_team", 40, -50, FIRE),
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
        mission: None,
        variant: None,
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
    };
    Sim::new(&config, map, 12).unwrap()
}

/// A snapshot taken while a squad bounds under fire continues tick for
/// tick like the original.
#[test]
fn snapshot_mid_bound_continues_identically() {
    let map = flat_map();
    let mut original = setup(map.clone());
    original.step(&[PlayerCommand::new(
        P0,
        Command::FormationMove {
            units: vec![UnitId(0), UnitId(1)],
            target: Vec2::from_meters(0, -30),
            formation: Formation::Line,
            facing: None,
            engage: Engage::All,
        },
    )]);
    let mut ticks = 0;
    loop {
        original.step(&[]);
        ticks += 1;
        let t = original.squad_tactics(UnitId(0)).unwrap();
        if t.bounding.is_some() && t.suppression > 0 {
            break;
        }
        assert!(ticks < 20 * 60, "the squad never bounded under fire");
    }
    let bytes = snapshot::encode(&original, &map);
    let mut restored = snapshot::decode(&bytes, map).expect("snapshot restores");
    assert_eq!(restored.state_hash(), original.state_hash());
    assert_eq!(restored.snapshot(), original.snapshot());
    for tick in 0..20 * 40 {
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
        restored.squad_tactics(UnitId(0)),
        original.squad_tactics(UnitId(0))
    );
}
