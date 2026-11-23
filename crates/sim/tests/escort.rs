//! Infantry behind tanks (prompt 198): a group order with squads and tanks
//! puts every squad behind a tank, its soldiers walk crouched in two files
//! in the tank's track, the hull stops machine-gun fire from the front, and
//! nobody gets under a hull on the way there and back. Hand-made map,
//! catalog units, fixed seeds; numbers with `--nocapture`.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, Engage, EscortRules, FireMode, Fixed, Formation, Map,
    PlayerCommand, PlayerId, Scale, Sim, SimConfig, Spawn, Stance, TerrainRules, UnitId, Vec2,
    unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const FIRE: FireMode = FireMode::FireAtWill;
const HOLD: FireMode = FireMode::HoldFire;
const TANK: &str = "de_pz4_h";
const SQUAD: &str = "de_inf_rifle_squad";
const HMG: &str = "su_inf_hmg_team";
const TRUCK: &str = "su_zis5";
/// Hull of the Panzer IV H (catalog: 5.92 m × 2.88 m).
const HALF_LENGTH_MM: i32 = 2960;
const HALF_WIDTH_MM: i32 = 1440;

/// 800 m × 800 m flat open map around the origin.
fn open_map() -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "escort_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let size = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; size], vec![0; size]).unwrap())
}

fn sim_with(seed: u64, spawns: &[(PlayerId, &str, i32, i32, FireMode)]) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    let map = open_map();
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
    Sim::new(&config, map, seed).unwrap()
}

fn group_move(units: &[u32], x: i32, y: i32, engage: Engage) -> PlayerCommand {
    PlayerCommand::new(
        P0,
        Command::FormationMove {
            units: units.iter().map(|&u| UnitId(u)).collect(),
            target: Vec2::from_meters(x, y),
            formation: Formation::Line,
            facing: None,
            engage,
        },
    )
}

fn unit(sim: &Sim, id: u32) -> sim::UnitSnapshot {
    sim.snapshot()
        .units
        .into_iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
}

fn mm(v: i32) -> Fixed {
    Fixed::from_num(v) / 1000
}

fn dot(a: Vec2, b: Vec2) -> Fixed {
    a.x * b.x + a.y * b.y
}

/// Two tanks and two squads beside them; an enemy truck (unarmed) in
/// sight north-east: the group knows the enemy is near.
fn platoon(seed: u64) -> Sim {
    let mut sim = sim_with(
        seed,
        &[
            (P0, TANK, -15, 0, HOLD),
            (P0, TANK, 15, 0, HOLD),
            (P0, SQUAD, -30, 0, HOLD),
            (P0, SQUAD, 30, 0, HOLD),
            (P1, TRUCK, 60, -40, HOLD),
        ],
    );
    sim.step(&[]);
    assert!(sim.visible_to(P0, UnitId(4)), "the truck is in sight");
    sim
}

#[test]
fn a_mixed_group_order_puts_the_squads_behind_the_tanks() {
    let mut sim = platoon(1);
    let ids: Vec<UnitId> = (0..4).map(UnitId).collect();
    let target = Vec2::from_meters(0, -150);
    let slots = sim.formation_preview(P0, &ids, target, Formation::Line, None);
    let slot = |id: u32| slots.iter().find(|s| s.0 == UnitId(id)).unwrap().1;
    // North is -y: the squads' slots 9 m behind (south of) the tanks',
    // the western squad behind the western tank.
    for (squad, tank) in [(2, 0), (3, 1)] {
        let d = slot(squad) - slot(tank);
        assert_eq!(d.x, Fixed::ZERO, "squad {squad} beside its tank");
        assert_eq!(d.y, mm(9_000), "squad {squad} {d:?} behind its tank");
    }
    sim.step(&[group_move(&[0, 1, 2, 3], 0, -150, Engage::Infantry)]);
    let north = Vec2::new(Fixed::ZERO, -Fixed::ONE);
    let mut kneeling = 0;
    let mut checked = 0;
    let mut last = [Vec2::ZERO; 2];
    // Positions of the tanks over the last second.
    let mut trail = [[Vec2::ZERO; 20]; 2];
    for tick in 0..20 * 200 {
        sim.step(&[]);
        let snapshot = sim.snapshot();
        for (squad, tank) in [(2u32, 0u32), (3, 1)] {
            let s = unit(&sim, squad);
            let t = unit(&sim, tank);
            let before = trail[tank as usize][(tick % 20) as usize];
            trail[tank as usize][(tick % 20) as usize] = t.position;
            // Formed up and on the way: the tank drove this tick.
            let driving = std::mem::replace(&mut last[tank as usize], t.position) != t.position;
            let moving = driving && sim.escorting(UnitId(squad)) == Some(UnitId(tank));
            // On the straight way (near the slot the tank turns in).
            let straight = (slot(tank) - t.position).length() > mm(30_000);
            if tick < 20 * 30 || !moving || !straight {
                continue;
            }
            // Once formed up: behind the tank in its track, 5–20 m back.
            let d = s.position - t.position;
            let back = -dot(d, north);
            assert!(
                back > mm(5_000) && back < mm(20_000),
                "tick {tick}: squad {squad} {back} m behind"
            );
            assert!(d.x.abs() < mm(3_000), "tick {tick}: squad {squad} {d:?}");
            // Its soldiers in the track of the hull (where it drove in the
            // last second), behind it.
            let to = t.position - before;
            let length = to.length();
            let track = Vec2::new(to.x / length, to.y / length);
            let right = Vec2::new(-track.y, track.x);
            for soldier in snapshot.soldiers.iter().filter(|x| x.unit == UnitId(squad)) {
                let d = soldier.position - t.position;
                assert!(
                    dot(d, right).abs() <= mm(HALF_WIDTH_MM),
                    "tick {tick}: soldier {} of {squad} {d:?} beside the track",
                    soldier.index
                );
                assert!(dot(d, track) < -mm(HALF_LENGTH_MM));
            }
            kneeling += usize::from(s.stance == Some(Stance::Kneel));
            checked += 1;
        }
    }
    println!("checked {checked} squad ticks, {kneeling} kneeling");
    assert!(checked > 20 * 50, "escorted only {checked} squad ticks");
    // Out of contact they walk upright, at the marching pace.
    assert_eq!(kneeling, 0);
    // Both arrive at their slots behind the tanks.
    for id in 0..4 {
        let u = unit(&sim, id);
        assert!(
            (u.position - slot(id)).length() < mm(3_000),
            "unit {id} at {:?}, slot {:?}",
            u.position,
            slot(id)
        );
    }
}

#[test]
fn infantry_alone_or_a_plain_move_keeps_its_slots() {
    let sim = sim_with(1, &[(P0, SQUAD, -15, 0, HOLD), (P0, SQUAD, 15, 0, HOLD)]);
    let ids = [UnitId(0), UnitId(1)];
    let slots = sim.formation_preview(P0, &ids, Vec2::from_meters(0, -300), Formation::Line, None);
    assert_eq!(slots[0].1.y, slots[1].1.y, "a line of squads");
    // No enemy in sight: a march, every unit keeps its own slot.
    let alone = sim_with(
        1,
        &[
            (P0, TANK, -15, 0, HOLD),
            (P0, TANK, 15, 0, HOLD),
            (P0, SQUAD, -30, 0, HOLD),
            (P0, SQUAD, 30, 0, HOLD),
        ],
    );
    let ids: Vec<UnitId> = (0..4).map(UnitId).collect();
    let target = Vec2::from_meters(0, -150);
    let slots = alone.formation_preview(P0, &ids, target, Formation::Line, None);
    let ys: Vec<Fixed> = slots.iter().map(|s| s.1.y).collect();
    assert!(ys.iter().all(|&y| y == ys[0]), "one line: {slots:?}");
    // A plain move (M) drives no march: nobody follows a tank.
    let mut sim = platoon(1);
    sim.step(&[group_move(&[0, 1, 2, 3], 0, -300, Engage::Never)]);
    for _ in 0..100 {
        sim.step(&[]);
        assert_eq!(sim.escorting(UnitId(2)), None);
    }
}

/// One tank and one squad attack-move north towards an enemy heavy
/// machine gun to 70 m from it (its range is 125 m), all with hold fire so
/// that nobody stops it, the order repeated once the gun is spotted; returns the hit points the squad lost on the way
/// until it is within 15 m of its slot (`escort` off: the old slots beside
/// the tank).
fn under_mg(seed: u64, escort: bool) -> u32 {
    let (lost, _, _, (kneeling, escorted)) = under_mg_firing(seed, escort, HOLD);
    if escort {
        // Under fire behind the tank they move crouched.
        println!("under fire behind the tank {escorted} ticks, kneeling {kneeling}");
        assert!(escorted > 20, "escorted {escorted} ticks under fire");
        assert!(
            kneeling * 2 >= escorted,
            "kneeling {kneeling} of {escorted}"
        );
    }
    lost
}

/// [`under_mg`] with the fire mode `own` for the tank and the squad;
/// also returns the ticks until the squad stands near its slot, whether
/// the machine gun still lives and the ticks the squad escorted the tank
/// under fire (the tank or the squad remembers where the fire came from)
/// kneeling and in all.
fn under_mg_firing(seed: u64, escort: bool, own: FireMode) -> (u32, u32, bool, (u32, u32)) {
    let mut sim = sim_with(
        seed,
        &[
            (P0, TANK, 0, 0, own),
            (P0, SQUAD, 0, 12, own),
            (P1, HMG, 0, -200, FIRE),
        ],
    );
    if !escort {
        sim.set_escort_rules(EscortRules::OFF);
    }
    let start = unit(&sim, 1).hp;
    // Attack-move; with hold fire the squad does not halt.
    sim.step(&[group_move(&[0, 1], 0, -130, Engage::Infantry)]);
    let mut ticks = 0;
    let (mut kneeling, mut escorted) = (0, 0);
    let mut spotted = false;
    for tick in 0..20 * 150 {
        ticks = tick;
        // Once the machine gun is spotted the order is given again (the
        // squad then falls in behind the tank).
        if !spotted && sim.visible_to(P0, UnitId(2)) {
            spotted = true;
            sim.step(&[group_move(&[0, 1], 0, -130, Engage::Infantry)]);
        } else {
            sim.step(&[]);
        }
        // Until the squad stands near its slot.
        let squad = unit(&sim, 1);
        // The machine gun fires at the squad (its rounds hit the tank).
        let fired_at =
            sim.alert_direction(UnitId(0)).is_some() || sim.alert_direction(UnitId(1)).is_some();
        if sim.escorting(UnitId(1)).is_some() && fired_at {
            escorted += 1;
            kneeling += u32::from(squad.stance == Some(Stance::Kneel));
        }
        if tick > 20 && (squad.position.y + Fixed::from_num(130)).abs() < mm(15_000) {
            break;
        }
    }
    (
        start - unit(&sim, 1).hp,
        ticks,
        unit(&sim, 2).alive(),
        (kneeling, escorted),
    )
}

#[test]
fn the_hull_stops_machine_gun_fire_from_the_front() {
    let (mut with, mut without) = (0, 0);
    for seed in 1..=3 {
        let (a, b) = (under_mg(seed, true), under_mg(seed, false));
        println!("seed {seed}: hit points lost behind the tank {a}, beside it {b}");
        with += a;
        without += b;
    }
    assert!(without > 0, "the machine gun never hit");
    assert!(with * 2 <= without, "behind {with}, beside {without}");
}

/// The same approach with both sides firing (the scenario "tank and squad
/// against a machine-gun nest" before and after prompt 198): numbers only,
/// the squad must not lose more behind the tank.
#[test]
fn approach_against_a_firing_machine_gun() {
    let (mut with, mut without) = (0, 0);
    for seed in 1..=4 {
        let a = under_mg_firing(seed, true, FIRE);
        let b = under_mg_firing(seed, false, FIRE);
        println!(
            "seed {seed}: behind the tank lost {} hp in {} ticks (MG alive {}), \
             beside it {} hp in {} ticks (MG alive {})",
            a.0, a.1, a.2, b.0, b.1, b.2
        );
        with += a.0;
        without += b.0;
    }
    assert!(with <= without, "behind {with}, beside {without}");
}

/// Whether a living soldier of player 0 stands under the hull of a tank.
fn under_a_hull(sim: &Sim, tanks: &[u32]) -> Option<String> {
    let snapshot = sim.snapshot();
    for &t in tanks {
        let tank = unit(sim, t);
        let forward = tank.facing;
        let right = Vec2::new(-forward.y, forward.x);
        for s in snapshot.soldiers.iter().filter(|s| s.hp > 0) {
            let d = s.position - tank.position;
            if dot(d, forward).abs() < mm(HALF_LENGTH_MM - 200)
                && dot(d, right).abs() < mm(HALF_WIDTH_MM - 200)
            {
                return Some(format!(
                    "soldier {} of {:?} under tank {t} at {:?}",
                    s.index, s.unit, tank.position
                ));
            }
        }
    }
    None
}

#[test]
fn nobody_gets_under_a_hull_there_and_back() {
    for seed in 1..=2 {
        let mut sim = platoon(seed);
        sim.step(&[group_move(&[0, 1, 2, 3], 0, -150, Engage::Infantry)]);
        for tick in 0..20 * 50 {
            sim.step(&[]);
            if let Some(e) = under_a_hull(&sim, &[0, 1]) {
                panic!("seed {seed} tick {tick} on the way there: {e}");
            }
        }
        // The tanks drive back alone, through their squads.
        sim.step(&[PlayerCommand::new(
            P0,
            Command::MoveTo {
                units: vec![UnitId(0), UnitId(1)],
                target: Vec2::from_meters(0, 50),
            },
        )]);
        let before = unit(&sim, 0).position;
        for tick in 0..20 * 60 {
            sim.step(&[]);
            if let Some(e) = under_a_hull(&sim, &[0, 1]) {
                panic!("seed {seed} tick {tick} on the way back: {e}");
            }
        }
        let after = unit(&sim, 0).position;
        assert!(after.y > before.y + Fixed::from_num(50), "drove back");
        // And the whole group drives back the other way.
        sim.step(&[group_move(&[0, 1, 2, 3], 0, -100, Engage::Infantry)]);
        for tick in 0..20 * 60 {
            sim.step(&[]);
            if let Some(e) = under_a_hull(&sim, &[0, 1]) {
                panic!("seed {seed} tick {tick} on the second way: {e}");
            }
        }
    }
}
