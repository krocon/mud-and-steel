//! Teams (prompt 037): allies do not fire at each other, share their
//! vision if the match says so, win and lose together.

use sim::map::MapLayout;
use sim::scenario::{
    DEMO_SCENARIO, SKIRMISH_SCENARIO, SlotSetup, builtin_map, parse_scenario_for_slots,
    parse_scenario_with_factions,
};
use sim::{
    Catalog, CombatEvent, CombatRules, Command, FireMode, Map, PlayerCommand, PlayerId, Scale, Sim,
    SimConfig, Spawn, Teams, TerrainRules, UnitId, Vec2, VisionRules, unit_types_from_catalog,
};
use std::path::Path;
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const P3: PlayerId = PlayerId(3);
const FIRE: FireMode = FireMode::FireAtWill;
const HOLD: FireMode = FireMode::HoldFire;
const TANK: &str = "de_pz4_h";
const ENEMY: &str = "su_t34_76_1943";

/// 800 m × 800 m, 2 m samples; a 10 m hill along the x axis (hides a tank
/// south of it from a tank north of it).
fn hill_map() -> Arc<Map> {
    let n = 401;
    let heights = (0..n)
        .flat_map(|j| (0..n).map(move |i| (i, j)))
        .map(|(_, j): (i32, i32)| (10_000 - (j * 2 - 400).abs() * 100).max(0))
        .collect();
    let layout = MapLayout {
        name: "teams_test".into(),
        samples_x: n as u32,
        samples_y: n as u32,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    Arc::new(Map::from_rasters(layout, heights, vec![0; (n * n) as usize]).unwrap())
}

fn sim_with(
    spawns: &[(PlayerId, &str, i32, i32, FireMode)],
    teams: &[(PlayerId, u8)],
    shared_vision: bool,
) -> Sim {
    let map = hill_map();
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
        vision: VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: Teams::new(teams, shared_vision),
    };
    Sim::new(&config, map, 7).unwrap()
}

fn shots(sim: &mut Sim, ticks: u32, commands: &[PlayerCommand]) -> usize {
    let mut count = 0;
    for t in 0..ticks {
        sim.step(if t == 0 { commands } else { &[] });
        count += sim
            .events()
            .iter()
            .filter(|e| matches!(e, CombatEvent::Shot { .. }))
            .count();
    }
    count
}

/// Two tanks of different players 100 m apart on the same side of the
/// hill, and a third player's tank far behind it.
fn face_off(teams: &[(PlayerId, u8)]) -> Sim {
    sim_with(
        &[
            (P0, TANK, 0, 200, FIRE),
            (P1, ENEMY, 0, 300, FIRE),
            (P2, ENEMY, 300, -350, HOLD),
        ],
        teams,
        false,
    )
}

#[test]
fn allies_do_not_fire_at_each_other() {
    // Without teams the two fight.
    let mut ffa = face_off(&[]);
    assert!(shots(&mut ffa, 200, &[]) > 0);

    let allied = [(P0, 0), (P1, 0), (P2, 1)];
    let mut sim = face_off(&allied);
    assert!(sim.allied(P0, P1) && !sim.allied(P0, P2));
    // An attack order on an ally is ignored as well.
    let attack = PlayerCommand::new(
        P0,
        Command::Attack {
            units: vec![UnitId(0)],
            target: UnitId(1),
        },
    );
    assert_eq!(shots(&mut sim, 400, &[attack]), 0);
    assert_eq!(sim.unit_hp(UnitId(0)), face_off(&allied).unit_hp(UnitId(0)));
    assert_eq!(sim.unit_hp(UnitId(1)), face_off(&allied).unit_hp(UnitId(1)));
}

#[test]
fn allies_see_each_other_and_share_vision_if_asked() {
    // P0 south of the hill, its ally P1 and the enemy P2 north of it.
    let spawns = [
        (P0, TANK, 0, 200, HOLD),
        (P1, TANK, 300, -250, HOLD),
        (P2, ENEMY, 0, -150, HOLD),
    ];
    let teams = [(P0, 0), (P1, 0), (P2, 1)];
    let alone = sim_with(&spawns, &teams, false);
    assert!(alone.visible_to(P1, UnitId(2)), "P1 sees the enemy");
    assert!(
        !alone.visible_to(P0, UnitId(2)),
        "the hill hides it from P0"
    );
    // Allies are always visible to each other, even behind the hill.
    assert!(alone.visible_to(P0, UnitId(1)));
    assert!(alone.visible_to(P1, UnitId(0)));
    assert!(!alone.visible_to(P2, UnitId(0)));

    let shared = sim_with(&spawns, &teams, true);
    assert!(
        shared.visible_to(P0, UnitId(2)),
        "seen through the ally's eyes"
    );
    assert!(!shared.visible_to(P2, UnitId(0)));
    // The fog of war shows the ally's sight too.
    let cell_count = |sim: &Sim| {
        let mut states = Vec::new();
        sim.vision_states(P0, &mut states);
        states
            .iter()
            .filter(|&&s| s == sim::CellState::Visible)
            .count()
    };
    assert!(cell_count(&shared) > cell_count(&alone));
    // Different hashes: the teams and who sees what are state.
    assert_ne!(shared.state_hash(), alone.state_hash());
}

#[test]
fn a_team_wins_when_all_enemies_are_defeated() {
    let spawns = [
        (P0, TANK, -300, 300, HOLD),
        (P1, ENEMY, 300, -300, HOLD),
        (P2, TANK, 300, 300, HOLD),
        (P3, ENEMY, -300, -300, HOLD),
    ];
    let mut sim = sim_with(&spawns, &[(P0, 0), (P2, 0), (P1, 1), (P3, 1)], false);
    let surrender = |p| [PlayerCommand::new(p, Command::Surrender)];
    sim.step(&surrender(P1));
    assert!(sim.outcome().is_none(), "P3 still fights");
    sim.step(&surrender(P0));
    assert!(sim.outcome().is_none(), "P2 still fights");
    sim.step(&surrender(P3));
    let outcome = sim.outcome().expect("decided");
    // P0 surrendered: P2 is the winner named, the whole team wins.
    assert_eq!(outcome.winner, Some(P2));
    assert_eq!(sim.winners(), vec![P0, P2]);
    // P0's units stay where they are; nothing was handed to P2.
    assert_eq!(sim.alive_units(P0), 1);
    assert_eq!(sim.alive_units(P2), 1);
}

fn demo_for(slots: &[SlotSetup], shared: bool, scenario: &str, map: &Map) -> SimConfig {
    parse_scenario_for_slots(
        scenario,
        slots,
        shared,
        &Catalog::builtin().unwrap(),
        &Scale::builtin().unwrap(),
        &TerrainRules::builtin().unwrap(),
        map,
    )
    .unwrap()
}

fn slot(player: u8, team: u8, faction: Option<&str>) -> SlotSetup {
    SlotSetup {
        player: PlayerId(player),
        team,
        faction: faction.map(String::from),
    }
}

#[test]
fn one_against_one_from_slots_is_the_plain_scenario() {
    let map = builtin_map();
    let slots = [slot(0, 0, Some("us")), slot(1, 1, None)];
    let config = demo_for(&slots, true, DEMO_SCENARIO, &map);
    let plain = parse_scenario_with_factions(
        DEMO_SCENARIO,
        &[(P0, "us")],
        &Catalog::builtin().unwrap(),
        &Scale::builtin().unwrap(),
        &TerrainRules::builtin().unwrap(),
        &map,
    )
    .unwrap();
    assert!(config.teams.is_free_for_all());
    assert_eq!(config, plain);
}

fn twin_meadow() -> Arc<Map> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/maps/twin_meadow");
    let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
    let mut map = Map::load(
        &String::from_utf8(read("map.toml")).unwrap(),
        &read("height.png"),
        &read("terrain.png"),
    )
    .unwrap();
    map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
        .unwrap();
    Arc::new(map)
}

#[test]
fn two_against_two_places_every_player_on_its_start() {
    let map = twin_meadow();
    let slots = [
        slot(0, 0, Some("de")),
        slot(1, 1, Some("su")),
        slot(2, 0, Some("us")),
        slot(3, 1, Some("de")),
    ];
    for scenario in [DEMO_SCENARIO, SKIRMISH_SCENARIO] {
        let config = demo_for(&slots, true, scenario, &map);
        assert_eq!(config.players(), vec![P0, P1, P2, P3]);
        assert!(config.teams.allied(P0, P2) && config.teams.allied(P1, P3));
        assert!(config.teams.hostile(P0, P1));
        for start in map.starts() {
            let player = PlayerId(start.player);
            let own: Vec<&Spawn> = config.spawns.iter().filter(|s| s.owner == player).collect();
            assert!(!own.is_empty(), "player {} has units", start.player);
            // Player 3 plays Germany with the template of player 1.
            if player == P3 {
                assert!(own.iter().any(|s| s.unit_type.starts_with("de_")));
            }
        }
        let mut sim = Sim::new(&config, map.clone(), 3).unwrap();
        for _ in 0..20 {
            sim.step(&[]);
        }
        assert!(sim.outcome().is_none());
        // The battle armies stand near their own start.
        if scenario == DEMO_SCENARIO {
            let first = |p: PlayerId| {
                config
                    .spawns
                    .iter()
                    .find(|s| s.owner == p)
                    .map(|s| s.position)
                    .unwrap()
            };
            for start in map.starts() {
                let d = (first(PlayerId(start.player)) - start.position).length();
                assert!(
                    d < sim::Fixed::from_num(40),
                    "player {}: {d} m",
                    start.player
                );
            }
        }
    }
}

#[test]
fn a_player_without_a_start_is_refused() {
    let map = builtin_map();
    let slots = [slot(0, 0, None), slot(1, 1, None), slot(2, 0, None)];
    let error = parse_scenario_for_slots(
        DEMO_SCENARIO,
        &slots,
        true,
        &Catalog::builtin().unwrap(),
        &Scale::builtin().unwrap(),
        &TerrainRules::builtin().unwrap(),
        &map,
    )
    .unwrap_err();
    assert!(
        error.0.contains("no start position for player 2"),
        "{error}"
    );
}
