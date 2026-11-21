//! Experience and ranks (prompt 050): points for damage, kills and
//! surviving fire, promotion, and the rank bonuses on reload time and
//! dispersion.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatEvent, CombatRules, Command, ExperienceRules, ExperienceView, FireMode, HitKind,
    Map, PlayerCommand, PlayerId, Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2,
    unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const PZ4: &str = "de_pz4_h";
const TRUCK: &str = "su_zis5";
const SQUAD: &str = "de_inf_rifle_squad";

/// Flat 800 m × 800 m around the origin.
fn flat_map() -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "experience_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap())
}

fn sim_with(spawns: &[(PlayerId, &str, i32, i32)], seed: u64) -> Sim {
    let map = flat_map();
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

fn xp(sim: &Sim, id: u32) -> ExperienceView {
    sim.unit_experience(UnitId(id)).unwrap()
}

#[test]
fn units_start_as_recruits() {
    let sim = sim_with(&[(P0, PZ4, 0, 0), (P1, SQUAD, 100, 0)], 1);
    for u in sim.snapshot().units {
        assert_eq!(u.experience, ExperienceView::default());
    }
}

#[test]
fn destroying_a_vehicle_promotes_the_shooter() {
    let rules = ExperienceRules::builtin().unwrap();
    let mut sim = sim_with(&[(P0, PZ4, 0, 0), (P1, TRUCK, 120, 0)], 2);
    sim.step(&[cmd(
        P0,
        Command::Attack {
            units: vec![UnitId(0)],
            target: UnitId(1),
        },
    )]);
    let mut promoted = None;
    for _ in 0..1200 {
        sim.step(&[]);
        if let Some(&(id, rank)) = sim.promotions().first() {
            promoted = Some((id, rank, sim.tick()));
        }
        if sim.unit_hp(UnitId(1)) == Some(0) {
            break;
        }
    }
    assert_eq!(sim.unit_hp(UnitId(1)), Some(0), "the truck survived");
    let tank = xp(&sim, 0);
    // All the damage to a whole unit, plus the kill.
    assert_eq!(tank.xp, rules.gain.damage_xp + rules.gain.kill_xp);
    assert_eq!((tank.kills, tank.vehicle_kills), (1, 1));
    assert_eq!(tank.rank, 1);
    let (id, rank, _) = promoted.expect("a promotion event");
    assert_eq!((id, rank), (UnitId(0), 1));
    // The truck survived the first hit (two hits destroy it) and gets
    // nothing for dying.
    assert_eq!(xp(&sim, 1).xp, rules.gain.survived_hit_xp);
    // The snapshot carries the same view.
    let snapshot = sim.snapshot();
    let unit = snapshot.units.iter().find(|u| u.id == UnitId(0)).unwrap();
    assert_eq!(unit.experience, tank);
}

#[test]
fn squads_earn_points_for_damage_and_for_surviving_fire() {
    let rules = ExperienceRules::builtin().unwrap();
    // 45 m: inside the 50 m of the squad's machine gun (400 m real x 1/8).
    let mut sim = sim_with(&[(P0, SQUAD, 0, 0), (P1, SQUAD, 45, 0)], 3);
    sim.step(&[cmd(
        P0,
        Command::Attack {
            units: vec![UnitId(0)],
            target: UnitId(1),
        },
    )]);
    let mut hits = 0;
    for _ in 0..400 {
        sim.step(&[]);
        hits += sim
            .events()
            .iter()
            .filter(|e| {
                matches!(e, CombatEvent::Hit { kind: HitKind::Unit(UnitId(1)), damage, .. } if *damage > 0)
            })
            .count();
    }
    assert!(hits > 0, "no hits");
    let shooter = xp(&sim, 0);
    let target = xp(&sim, 1);
    assert!(shooter.xp > 0);
    // The target held fire: it earns only by surviving hits and being
    // pinned down, never by damage.
    assert!(target.xp > 0, "no points for surviving fire");
    assert_eq!(target.kills, 0);
    let survived_max = u32::try_from(hits).unwrap() * rules.gain.survived_hit_xp
        + 400 / 20 * rules.gain.suppressed_xp_per_s;
    assert!(target.xp <= survived_max);
}

/// Salvos of unit 0 firing at the ground for `ticks` ticks, 75 m away:
/// inside the 87.5 m of the 75 mm gun (700 m real x 1/8).
fn salvos(sim: &mut Sim, ticks: u32) -> (u32, Vec<Vec2>) {
    sim.step(&[cmd(
        P0,
        Command::AttackGround {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(75, 0),
        },
    )]);
    let mut shots = 0;
    let mut impacts = Vec::new();
    for _ in 0..ticks {
        sim.step(&[]);
        for e in sim.events() {
            match e {
                CombatEvent::Shot { weapon: 0, .. } => shots += 1,
                CombatEvent::Hit { at, .. } => impacts.push(at.ground()),
                _ => {}
            }
        }
    }
    (shots, impacts)
}

#[test]
fn veterans_reload_faster_and_shoot_closer() {
    let rules = ExperienceRules::builtin().unwrap();
    let mut recruit = sim_with(&[(P0, PZ4, 0, 0)], 4);
    let mut elite = sim_with(&[(P0, PZ4, 0, 0)], 4);
    elite.set_unit_experience(UnitId(0), rules.ranks[3].xp);
    assert_eq!(xp(&elite, 0).rank, 3);
    let (recruit_shots, recruit_hits) = salvos(&mut recruit, 1200);
    let (elite_shots, elite_hits) = salvos(&mut elite, 1200);
    // 60 ticks between rounds, 88 % of that for the elite crew.
    assert!(recruit_shots >= 18);
    assert!(
        elite_shots > recruit_shots,
        "elite {elite_shots} vs recruit {recruit_shots}"
    );
    assert!(elite_shots <= recruit_shots * 100 / 85 + 1);
    // Same random numbers: the elite rounds land closer to the aim point.
    let target = Vec2::from_meters(75, 0);
    let spread = |hits: &[Vec2]| {
        hits.iter()
            .take(15)
            .map(|p| (p.y - target.y).abs())
            .fold(sim::Fixed::ZERO, |a, b| a + b)
    };
    assert!(recruit_hits.len() >= 15 && elite_hits.len() >= 15);
    assert!(spread(&elite_hits) < spread(&recruit_hits));
}

#[test]
fn experience_is_part_of_the_state_hash() {
    let mut a = sim_with(&[(P0, PZ4, 0, 0)], 5);
    let b = a.clone();
    assert_eq!(a.state_hash(), b.state_hash());
    a.set_unit_experience(UnitId(0), 1);
    assert_ne!(a.state_hash(), b.state_hash());
}
