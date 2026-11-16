//! A base match is lost with the base (decision 6 of prompt 045): a
//! player who has had a completed headquarters or production building is
//! defeated once none of them is left, even with units on the field. Construction sites do not count; battle mode is unaffected.

use sim::scenario::{SKIRMISH_SCENARIO, SlotSetup, builtin_map, parse_scenario_for_slots};
use sim::{
    Catalog, Command, DefeatReason, FireMode, Map, PlayerCommand, PlayerId, Scale, Sim, SimConfig,
    TerrainRules, UnitId, Vec2,
};
use std::path::Path;
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const P3: PlayerId = PlayerId(3);

/// The skirmish with every unit holding fire.
fn peaceful() -> SimConfig {
    let mut config = sim::scenario::skirmish_config(&[]);
    for spawn in &mut config.spawns {
        spawn.fire_mode = FireMode::HoldFire;
    }
    config
}

fn t(sim: &Sim, name: &str) -> u32 {
    sim.unit_type_index(name).unwrap()
}

fn building(sim: &Sim, player: PlayerId, name: &str) -> UnitId {
    sim.structures()
        .into_iter()
        .find(|s| s.owner == player && s.unit_type == t(sim, name))
        .unwrap()
        .unit
}

fn step(sim: &mut Sim, player: PlayerId, command: Command) {
    sim.step(&[PlayerCommand::new(player, command)]);
}

fn run(sim: &mut Sim, ticks: u64) {
    for _ in 0..ticks {
        sim.step(&[]);
    }
}

fn defeat(sim: &Sim, player: PlayerId) -> Option<DefeatReason> {
    sim.standings()
        .iter()
        .find(|s| s.player == player)
        .and_then(|s| s.defeat.map(|d| d.0))
}

/// Places a barracks of `player` next to its headquarters.
fn build_barracks(sim: &mut Sim, player: PlayerId) -> UnitId {
    let barracks = t(sim, "base_barracks");
    let home = sim.unit_position(building(sim, player, "base_hq")).unwrap();
    let spot = sim.find_site(player, barracks, home).unwrap();
    step(
        sim,
        player,
        Command::Build {
            unit_type: barracks,
            position: spot,
        },
    );
    building(sim, player, "base_barracks")
}

#[test]
fn losing_the_headquarters_defeats_a_player_with_units_left() {
    let mut config = peaceful();
    // Three T-34 of player 1 in front of player 0's headquarters.
    let home = config
        .spawns
        .iter()
        .find(|s| s.owner == P0 && s.unit_type == "base_hq")
        .unwrap()
        .position;
    for dx in [-30, 30, 42] {
        config.spawns.push(sim::Spawn {
            owner: P1,
            unit_type: "su_t34_76_1943".into(),
            position: home + Vec2::from_meters(dx, -22),
            fire_mode: FireMode::FireAtWill,
        });
    }
    let mut sim = Sim::new(&config, builtin_map(), 7).unwrap();
    let hq0 = building(&sim, P0, "base_hq");
    let tanks: Vec<UnitId> = sim
        .snapshot()
        .units
        .iter()
        .filter(|u| u.owner == P1 && u.unit_type == t(&sim, "su_t34_76_1943"))
        .map(|u| u.id)
        .collect();
    step(
        &mut sim,
        P1,
        Command::Attack {
            units: tanks,
            target: hq0,
        },
    );
    for _ in 0..4000 {
        sim.step(&[]);
        if sim.unit_hp(hq0) == Some(0) {
            break;
        }
    }
    assert_eq!(sim.unit_hp(hq0), Some(0), "the headquarters fell");
    // Its truck and squads still live (they hold fire, the tanks only
    // attacked the building), but the player is defeated in the same tick.
    assert!(sim.alive_units(P0) > 0);
    assert_eq!(defeat(&sim, P0), Some(DefeatReason::BaseLost));
    let outcome = sim.outcome().expect("decided");
    assert_eq!(outcome.winner, Some(P1));
}

#[test]
fn a_construction_site_does_not_keep_a_player_in_the_match() {
    let mut sim = Sim::new(&peaceful(), builtin_map(), 7).unwrap();
    let site = build_barracks(&mut sim, P0);
    run(&mut sim, 20);
    assert!(
        sim.structures()
            .iter()
            .any(|s| s.unit == site && !s.completed())
    );
    let hq0 = building(&sim, P0, "base_hq");
    step(&mut sim, P0, Command::Sell { building: hq0 });
    assert_eq!(defeat(&sim, P0), Some(DefeatReason::BaseLost));
    assert_eq!(sim.outcome().and_then(|o| o.winner), Some(P1));
}

#[test]
fn a_barracks_keeps_a_player_in_the_match_until_it_falls() {
    let mut sim = Sim::new(&peaceful(), builtin_map(), 7).unwrap();
    let barracks = build_barracks(&mut sim, P0);
    // Completed after 25 s.
    run(&mut sim, 25 * 20 + 1);
    assert!(
        sim.structures()
            .iter()
            .any(|s| s.unit == barracks && s.completed())
    );
    let hq0 = building(&sim, P0, "base_hq");
    step(&mut sim, P0, Command::Sell { building: hq0 });
    run(&mut sim, 100);
    assert_eq!(defeat(&sim, P0), None, "the barracks still stands");
    assert!(sim.outcome().is_none());
    step(&mut sim, P0, Command::Sell { building: barracks });
    assert_eq!(defeat(&sim, P0), Some(DefeatReason::BaseLost));
    assert_eq!(defeat(&sim, P1), None);
    assert_eq!(sim.outcome().and_then(|o| o.winner), Some(P1));
}

#[test]
fn battle_mode_has_no_base_to_lose() {
    // The demo: tanks and squads, no building from the start.
    let mut sim = Sim::new(&sim::scenario::demo_config(), builtin_map(), 42).unwrap();
    assert!(sim.structures().is_empty());
    run(&mut sim, 200);
    assert!(sim.standings().iter().all(|s| s.defeat.is_none()));
    assert!(sim.outcome().is_none());
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
fn a_team_loses_when_every_member_lost_the_base() {
    let map = twin_meadow();
    let slot = |player: u8, team: u8, faction: &str| SlotSetup {
        player: PlayerId(player),
        team,
        faction: Some(faction.into()),
    };
    let slots = [
        slot(0, 0, "de"),
        slot(1, 1, "su"),
        slot(2, 0, "us"),
        slot(3, 1, "de"),
    ];
    let mut config = parse_scenario_for_slots(
        SKIRMISH_SCENARIO,
        &slots,
        true,
        &Catalog::builtin().unwrap(),
        &Scale::builtin().unwrap(),
        &TerrainRules::builtin().unwrap(),
        &map,
    )
    .unwrap();
    for spawn in &mut config.spawns {
        spawn.fire_mode = FireMode::HoldFire;
    }
    let mut sim = Sim::new(&config, map, 3).unwrap();
    run(&mut sim, 5);
    let hq0 = building(&sim, P0, "base_hq");
    step(&mut sim, P0, Command::Sell { building: hq0 });
    assert_eq!(defeat(&sim, P0), Some(DefeatReason::BaseLost));
    assert!(sim.outcome().is_none(), "P2 still has its base");
    run(&mut sim, 20);
    assert!(sim.outcome().is_none());
    let hq2 = building(&sim, P2, "base_hq");
    step(&mut sim, P2, Command::Sell { building: hq2 });
    assert_eq!(defeat(&sim, P2), Some(DefeatReason::BaseLost));
    let outcome = sim.outcome().expect("decided");
    assert_eq!(outcome.winner, Some(P1));
    assert_eq!(sim.winners(), vec![P1, P3]);
}
