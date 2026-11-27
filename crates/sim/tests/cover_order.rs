//! Targeted cover command, cover preview and squad cover grades (prompt 233):
//! `Command::TakeCover`, `Sim::cover_preview`, `Sim::unit_cover_grade`.

use sim::{Command, FireMode, Fixed, PlayerCommand, PlayerId, Sim, Spawn, UnitId, Vec2};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn make_sim(spawns: Vec<Spawn>, seed: u64) -> Sim {
    let mut config = sim::scenario::skirmish_config(&[]);
    config.spawns = spawns;
    Sim::new(&config, sim::scenario::builtin_map(), seed).unwrap()
}

fn find_object(prefix: &str) -> (u32, Vec2) {
    let map = sim::scenario::builtin_map();
    let kinds = map.object_kinds();
    (0u32..)
        .zip(map.objects())
        .find(|(_, o)| {
            kinds
                .get(usize::from(o.kind))
                .is_some_and(|k| k.id.contains(prefix))
        })
        .map(|(i, o)| (i, o.position))
        .unwrap_or_else(|| panic!("builtin map has an object containing {prefix}"))
}

#[test]
fn cover_preview_flips_sides_and_classifies_heavy_vs_light() {
    let (_, wall_pos) = find_object("wall_stone");
    let (_, light_pos) = find_object("hedge");
    let squad_pos = wall_pos + Vec2::from_meters(0, -14);
    let sim = make_sim(
        vec![
            Spawn {
                owner: P0,
                unit_type: "de_inf_rifle_squad".into(),
                position: squad_pos,
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P1,
                unit_type: "su_inf_rifle_squad".into(),
                position: Vec2::from_meters(180, 180),
                fire_mode: FireMode::HoldFire,
            },
        ],
        13,
    );
    let squad = UnitId(0);

    // Preview on both sides of the stone wall: spots flip sides and classify as heavy (grade 2).
    let side_a = sim.cover_preview(P0, &[squad], wall_pos + Vec2::from_meters(0, -2), None);
    let side_b = sim.cover_preview(P0, &[squad], wall_pos + Vec2::from_meters(0, 2), None);
    assert!(!side_a.is_empty());
    assert!(!side_b.is_empty());
    assert!(
        side_a.iter().any(|s| s.grade == 2),
        "stone wall should yield heavy cover (grade 2): {side_a:?}"
    );
    assert!(
        side_b.iter().any(|s| s.grade == 2),
        "opposite side of stone wall should yield heavy cover (grade 2): {side_b:?}"
    );
    // Facings on opposite sides point in opposite directions.
    let dot = side_a[0].facing.x * side_b[0].facing.x + side_a[0].facing.y * side_b[0].facing.y;
    assert!(
        dot < Fixed::ZERO,
        "opposite sides of wall must face opposite ways: {:?} vs {:?}",
        side_a[0].facing,
        side_b[0].facing
    );

    // Hedge classifies as light cover (grade 1).
    let light_spots = sim.cover_preview(P0, &[squad], light_pos + Vec2::from_meters(0, -2), None);
    assert!(
        light_spots.iter().any(|s| s.grade == 1),
        "hedge should yield light cover (grade 1): {light_spots:?}"
    );
}

#[test]
fn take_cover_matches_preview_and_updates_unit_cover_grade() {
    let (_, wall_pos) = find_object("wall_stone");
    let start = wall_pos + Vec2::from_meters(0, -18);
    let target = wall_pos + Vec2::from_meters(0, -2);
    let mut sim = make_sim(
        vec![
            Spawn {
                owner: P0,
                unit_type: "de_inf_rifle_squad".into(),
                position: start,
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P1,
                unit_type: "su_inf_rifle_squad".into(),
                position: Vec2::from_meters(180, 180),
                fire_mode: FireMode::HoldFire,
            },
        ],
        29,
    );
    let squad = UnitId(0);

    // Open ground at start -> cover grade 0 (after settling).
    sim.step(&[]);
    let preview = sim.cover_preview(P0, &[squad], target, None);
    let heavy_in_preview = preview.iter().filter(|s| s.grade == 2).count();
    assert!(heavy_in_preview > 0, "preview should find heavy cover");

    // Issue TakeCover to target and let squad arrive and settle.
    sim.step(&[PlayerCommand::new(
        P0,
        Command::TakeCover {
            units: vec![squad],
            target,
            facing: None,
        },
    )]);
    for _ in 0..20 * 15 {
        sim.step(&[]);
    }
    assert_eq!(
        sim.unit_cover_grade(squad),
        2,
        "squad settled behind stone wall must report heavy cover grade 2"
    );
    // Soldiers should occupy the previewed spots.
    let snap = sim.snapshot();
    let soldiers: Vec<Vec2> = snap
        .soldiers
        .iter()
        .filter(|s| s.unit == squad && s.hp > 0)
        .map(|s| s.position)
        .collect();
    let matched = soldiers
        .iter()
        .filter(|&&p| {
            preview
                .iter()
                .any(|s| s.grade == 2 && (s.position - p).length() <= Fixed::from_num(2))
        })
        .count();
    assert!(
        matched >= heavy_in_preview.min(soldiers.len()) / 2,
        "soldiers should settle onto previewed cover spots (matched {matched}/{heavy_in_preview})"
    );
}

#[test]
fn nearest_cover_quick_command_moves_open_squad_into_cover() {
    let (_, wall_pos) = find_object("wall_stone");
    let start = wall_pos + Vec2::from_meters(0, -14);
    let mut a = make_sim(
        vec![
            Spawn {
                owner: P0,
                unit_type: "de_inf_rifle_squad".into(),
                position: start,
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P1,
                unit_type: "su_inf_rifle_squad".into(),
                position: Vec2::from_meters(180, 180),
                fire_mode: FireMode::HoldFire,
            },
        ],
        41,
    );
    let mut b = make_sim(
        vec![
            Spawn {
                owner: P0,
                unit_type: "de_inf_rifle_squad".into(),
                position: start,
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P1,
                unit_type: "su_inf_rifle_squad".into(),
                position: Vec2::from_meters(180, 180),
                fire_mode: FireMode::HoldFire,
            },
        ],
        41,
    );
    let squad = UnitId(0);
    // Clicking own position (within 2 m) with facing: None triggers deterministic nearest-cover search.
    let cmd = [PlayerCommand::new(
        P0,
        Command::TakeCover {
            units: vec![squad],
            target: start,
            facing: None,
        },
    )];
    a.step(&cmd);
    b.step(&cmd);
    for _ in 0..20 * 15 {
        a.step(&[]);
        b.step(&[]);
    }
    assert_eq!(
        a.state_hash(),
        b.state_hash(),
        "TakeCover must be deterministic"
    );
    assert!(
        a.unit_cover_grade(squad) >= 1,
        "nearest-cover quick command should move squad into nearby cover (grade={})",
        a.unit_cover_grade(squad)
    );
}
