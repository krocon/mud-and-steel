//! Gun crews (prompt 177): a Pak 40 against infantry from the front and
//! the flank, a Flak 36 against a tank. Numbers with `--nocapture`.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, FireMode, Map, PlayerCommand, PlayerId, Scale, Sim, SimConfig,
    Spawn, TerrainRules, UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const FIRE: FireMode = FireMode::FireAtWill;
const N: u32 = 401;

/// 800 m × 800 m flat map around the origin; `terrain` gives the terrain
/// id of a sample from its position in meters, `objects` are CSV lines.
fn map_with(terrain: impl Fn(i32, i32) -> u8, objects: &[String]) -> Arc<Map> {
    let layout = MapLayout {
        name: "crew_test".into(),
        samples_x: N,
        samples_y: N,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let half = (N as i32 - 1) / 2;
    let mut ids = Vec::with_capacity((N * N) as usize);
    for j in 0..N as i32 {
        for i in 0..N as i32 {
            ids.push(terrain((i - half) * 2, (j - half) * 2));
        }
    }
    let size = (N * N) as usize;
    let mut map = Map::from_rasters(layout, vec![0; size], ids).unwrap();
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

fn flat() -> Arc<Map> {
    map_with(|_, _| 0, &[])
}

fn sim_with(map: Arc<Map>, spawns: &[(PlayerId, &str, i32, i32, FireMode)]) -> Sim {
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
    Sim::new(&config, map, 7).unwrap()
}

/// Result of one fight: the gun (unit 0) is out
/// of action (destroyed or without crew) at this tick, and the attacker
/// (unit 1) lost at this tick.
#[derive(Debug, Default)]
struct Fight {
    gun_out: Option<u64>,
    attacker_lost: Option<u64>,
}

/// A gun of player 0 at the origin (it starts facing -y) against the
/// units of player 1 in `attackers` (unit, x, y), all told to attack it,
/// for `seconds`. The attacker counted as lost is the first one.
fn fight(gun: &str, attackers: &[(&str, i32, i32)], seconds: u32) -> Fight {
    let mut spawns = vec![(P0, gun, 0, 0, FIRE)];
    spawns.extend(attackers.iter().map(|&(u, x, y)| (P1, u, x, y, FIRE)));
    let mut sim = sim_with(flat(), &spawns);
    let units: Vec<UnitId> = (1..=attackers.len() as u32).map(UnitId).collect();
    sim.step(&[PlayerCommand::new(
        P1,
        Command::Attack {
            units,
            target: UnitId(0),
        },
    )]);
    let mut out = Fight::default();
    for _ in 0..20 * seconds {
        sim.step(&[]);
        let snap = sim.snapshot();
        let tick = sim.tick();
        let gun_out = !snap.units[0].alive()
            || sim
                .fire_status(UnitId(0))
                .is_some_and(|(s, _)| s.name() == "abandoned");
        if gun_out && out.gun_out.is_none() {
            out.gun_out = Some(tick);
        }
        if !snap.units[1].alive() && out.attacker_lost.is_none() {
            out.attacker_lost = Some(tick);
        }
        if out.gun_out.is_some() || out.attacker_lost.is_some() {
            break;
        }
    }
    out
}

#[test]
fn crews_under_fire() {
    let hmg = "us_inf_hmg_team";
    type Case<'a> = (&'a str, &'a str, Vec<(&'a str, i32, i32)>);
    let cases: [Case; 7] = [
        (
            "rifle squad, front",
            "de_pak40",
            vec![("su_inf_rifle_squad", 0, -60)],
        ),
        (
            "rifle squad, flank",
            "de_pak40",
            vec![("su_inf_rifle_squad", 60, 0)],
        ),
        ("HMG team, front", "de_pak40", vec![(hmg, 0, -60)]),
        ("HMG team, flank", "de_pak40", vec![(hmg, 60, 0)]),
        (
            "two HMG teams, both flanks",
            "de_pak40",
            vec![(hmg, 60, 0), (hmg, -60, 0)],
        ),
        (
            "mortar with spotter",
            "de_pak40",
            vec![("us_inf_mortar_medium", 0, -250), ("us_inf_recon", 0, -110)],
        ),
        (
            "Flak 36 vs T-34, front",
            "de_flak36",
            vec![("su_t34_76_1943", 0, -80)],
        ),
    ];
    for (name, gun, attackers) in &cases {
        println!("{name}: {:?}", fight(gun, attackers, 180));
    }
}
