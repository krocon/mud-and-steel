//! Weather, soft ground and time of day (prompt 040): mud slows wheels more
//! than tracks, roads stay fast, rain softens the ground over time, fog and
//! night shorten sight, muzzle flashes reveal the shooter at night, and the
//! same inputs give the same hash.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, Daytime, FireMode, Ground, Map, PlayerCommand, PlayerId, Scale,
    Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2, VisionRules, Weather, WeatherSetup,
    unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const FIRE: FireMode = FireMode::FireAtWill;
const HOLD: FireMode = FireMode::HoldFire;

const TANK: &str = "de_pz4_h";
const TRUCK: &str = "de_opel_blitz";
const ENEMY: &str = "su_t34_76_1943";
const SQUAD: &str = "su_inf_rifle_squad";

const GRASS: u8 = 0;
const DIRT_TRACK: u8 = 1;
const ROAD: u8 = 2;

/// 800 m × 800 m of flat ground of one terrain type, 2 m samples.
fn flat_map(terrain: u8) -> Arc<Map> {
    let n = 401u32;
    let layout = MapLayout {
        name: format!("weather_test_{terrain}"),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; count], vec![terrain; count]).unwrap())
}

fn setup(weather: Weather, daytime: Daytime) -> WeatherSetup {
    WeatherSetup { weather, daytime }
}

fn sim_with(
    map: Arc<Map>,
    weather: WeatherSetup,
    spawns: &[(PlayerId, &str, i32, i32, FireMode)],
) -> Sim {
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
        teams: sim::Teams::default(),
        weather,
        mission: None,
    };
    Sim::new(&config, map, 7).unwrap()
}

/// Meters a lone `unit` drives north in `ticks` on flat `terrain`.
fn distance(unit: &str, terrain: u8, weather: Weather, ticks: u32) -> i32 {
    let mut sim = sim_with(
        flat_map(terrain),
        setup(weather, Daytime::Day),
        &[(P0, unit, 0, -300, HOLD)],
    );
    let order = PlayerCommand {
        player: P0,
        command: Command::MoveTo {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(0, 300),
        },
    };
    sim.step(&[order]);
    for _ in 1..ticks {
        sim.step(&[]);
    }
    let y = sim.snapshot().units[0].position.y;
    y.to_num::<i32>() + 300
}

#[test]
fn mud_slows_wheels_more_than_tracks() {
    let ticks = 400;
    let tank_dry = distance(TANK, DIRT_TRACK, Weather::Clear, ticks);
    let tank_wet = distance(TANK, DIRT_TRACK, Weather::Rain, ticks);
    let truck_dry = distance(TRUCK, DIRT_TRACK, Weather::Clear, ticks);
    let truck_wet = distance(TRUCK, DIRT_TRACK, Weather::Rain, ticks);
    assert!(tank_dry > 50 && truck_dry > 50, "{tank_dry} {truck_dry}");
    assert!(tank_wet < tank_dry, "tank {tank_wet} vs {tank_dry}");
    // Share of the dry distance (in percent): wheels lose more.
    let tank_pct = tank_wet * 100 / tank_dry;
    let truck_pct = truck_wet * 100 / truck_dry;
    assert!(
        truck_pct + 10 < tank_pct,
        "truck {truck_pct} %, tank {tank_pct} %"
    );
}

#[test]
fn roads_stay_fast_in_the_rain() {
    let ticks = 300;
    assert_eq!(
        distance(TRUCK, ROAD, Weather::Rain, ticks),
        distance(TRUCK, ROAD, Weather::Clear, ticks)
    );
    // Snow lies on roads too, but less than on grass.
    let road = distance(TRUCK, ROAD, Weather::Snow, ticks);
    let grass = distance(TRUCK, GRASS, Weather::Snow, ticks);
    let grass_dry = distance(TRUCK, GRASS, Weather::Clear, ticks);
    assert!(grass * 100 / grass_dry < road * 100 / distance(TRUCK, ROAD, Weather::Clear, ticks));
}

#[test]
fn rain_softens_the_ground_and_the_path_costs_follow() {
    let mut sim = sim_with(
        flat_map(GRASS),
        setup(Weather::Rain, Daytime::Day),
        &[(P0, TRUCK, 0, 0, HOLD)],
    );
    let (ground, start) = sim.ground_state();
    assert_eq!(ground, Ground::Wet);
    let cell = |sim: &Sim| sim.path_grid().costs(0)[100 * 200 + 100];
    let cost_start = cell(&sim);
    // 120 game seconds per step, then 8 path rows per tick.
    for _ in 0..(120 * 20 + 20) {
        sim.step(&[]);
    }
    let (_, later) = sim.ground_state();
    assert!(later > start, "{start} -> {later}");
    assert!(
        cell(&sim) > cost_start,
        "cost {} -> {}",
        cost_start,
        cell(&sim)
    );

    // Clear weather: firm ground, costs as without weather.
    let clear = sim_with(
        flat_map(GRASS),
        WeatherSetup::default(),
        &[(P0, TRUCK, 0, 0, HOLD)],
    );
    assert_eq!(clear.ground_state(), (Ground::Dry, 0));
    assert!(cell(&clear) < cost_start);
}

/// Whether P0 (a tank at `(0, 200)`) sees a P1 tank at `(0, y)`.
fn p0_sees(weather: WeatherSetup, y: i32) -> bool {
    let sim = sim_with(
        flat_map(GRASS),
        weather,
        &[(P0, TANK, 0, 200, HOLD), (P1, ENEMY, 0, y, HOLD)],
    );
    sim.visible_to(P0, UnitId(1))
}

#[test]
fn fog_and_night_shorten_sight() {
    // 150 m apart: seen by day (250 m, 2 km real × 1/8, prompt 097), not in
    // fog (35 %: 87.5 m) or at night (45 %: 112.5 m).
    assert!(p0_sees(WeatherSetup::default(), 50));
    assert!(!p0_sees(setup(Weather::Fog, Daytime::Day), 50));
    assert!(!p0_sees(setup(Weather::Clear, Daytime::Night), 50));
    // Close by (40 m), fog hides nothing.
    assert!(p0_sees(setup(Weather::Fog, Daytime::Day), 160));
    let day = VisionRules::builtin().unwrap();
    let mut config_vision = SimConfig {
        tick_hz: 20,
        unit_types: Vec::new(),
        spawns: Vec::new(),
        map: flat_map(GRASS).reference(),
        terrain: TerrainRules::builtin().unwrap(),
        combat: CombatRules::builtin().unwrap(),
        vision: day.clone(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: setup(Weather::Fog, Daytime::Day),
        mission: None,
    };
    let fog = config_vision.effective_vision().range_m(None);
    config_vision.weather = WeatherSetup::default();
    assert_eq!(config_vision.effective_vision(), day);
    assert!(fog < day.range_m(None) / 2);
}

#[test]
fn muzzle_flashes_reveal_the_shooter_at_night() {
    // Two tanks fight 75 m apart (their guns reach 87.5 m, prompt 097); an
    // observer 150 m behind the P1 tank cannot see it at night (112.5 m
    // sight) until it fires (flashes are seen within 250 m).
    let night = setup(Weather::Clear, Daytime::Night);
    let mut sim = sim_with(
        flat_map(GRASS),
        night,
        &[
            (P0, TANK, 0, -75, FIRE),
            (P1, ENEMY, 0, 0, FIRE),
            (P2, SQUAD, 0, 150, HOLD),
        ],
    );
    assert!(sim.visible_to(P0, UnitId(1)), "the tanks see each other");
    assert!(
        !sim.visible_to(P2, UnitId(1)),
        "the observer sees nothing yet"
    );
    let mut revealed = false;
    for _ in 0..600 {
        sim.step(&[]);
        if sim.visible_to(P2, UnitId(1)) {
            revealed = true;
            break;
        }
    }
    assert!(revealed, "a shot of the P1 tank is seen by the observer");
}

#[test]
fn same_weather_and_seed_give_the_same_hash() {
    let run = |weather| {
        let mut sim = sim_with(
            flat_map(DIRT_TRACK),
            weather,
            &[(P0, TANK, 0, -75, FIRE), (P1, ENEMY, 0, 0, FIRE)],
        );
        for _ in 0..300 {
            sim.step(&[]);
        }
        sim.state_hash()
    };
    let rain_night = setup(Weather::Rain, Daytime::Night);
    assert_eq!(run(rain_night), run(rain_night));
    assert_ne!(run(rain_night), run(WeatherSetup::default()));
}
