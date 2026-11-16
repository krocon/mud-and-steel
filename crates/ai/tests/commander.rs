//! The commander (prompt 051): staged flank attacks and surrendering when
//! hopeless, on a flat open map.

use ai::Difficulty;
use ai::game::Game;
use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, DefeatReason, FireMode, Fixed, Map, PlayerId, Scale, SimConfig,
    Spawn, TerrainRules, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

/// 1200 m × 1200 m of open meadow.
fn meadow() -> Arc<Map> {
    let n = 601u32;
    let layout = MapLayout {
        name: "ai_commander".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let size = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; size], vec![0; size]).unwrap())
}

fn config(map: &Map, spawns: &[(PlayerId, &str, i32, i32)]) -> SimConfig {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    SimConfig {
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
    }
}

/// Rifle squads that see only tanks (which hold their fire): Hard gives up
/// after a minute, the old Hard never does.
#[test]
fn hard_surrenders_when_nothing_it_has_can_hurt_the_enemy() {
    let map = meadow();
    let config = config(
        &map,
        &[
            (PlayerId(0), "de_pz4_h", -200, 0),
            (PlayerId(0), "de_pz4_h", -200, 20),
            (PlayerId(1), "su_inf_rifle_squad", 200, 0),
        ],
    );
    for (level, gives_up) in [(Difficulty::Hard, true), (Difficulty::HardOld, false)] {
        let mut game = Game::new(
            &config,
            map.clone(),
            4,
            &[(PlayerId(0), Difficulty::Passive), (PlayerId(1), level)],
        )
        .unwrap();
        let mut surrendered = false;
        while game.sim().outcome().is_none() && game.sim().tick() < 20 * 150 {
            let d = game.think();
            surrendered |= d[1].contains(&Command::Surrender);
            game.step(d);
        }
        assert_eq!(surrendered, gives_up, "{level}");
        if gives_up {
            let standing = game.sim().standings()[1];
            assert!(
                matches!(standing.defeat, Some((DefeatReason::Surrendered, t)) if t > 1200),
                "{standing:?}"
            );
        }
    }
}

/// Tanks against a passive enemy: Hard stages its army beside the enemy's
/// position and attacks from the flank, not along the direct line.
#[test]
fn hard_attacks_from_the_flank() {
    let map = meadow();
    let mut spawns = Vec::new();
    for k in 0..4 {
        spawns.push((PlayerId(0), "de_pz4_h", 300, -18 + 12 * k));
        spawns.push((PlayerId(1), "su_t34_76_1943", -450, -18 + 12 * k));
    }
    let config = config(&map, &spawns);
    let mut game = Game::new(
        &config,
        map,
        6,
        &[
            (PlayerId(0), Difficulty::Passive),
            (PlayerId(1), Difficulty::Hard),
        ],
    )
    .unwrap();
    let enemy = Vec2::from_meters(300, 0);
    loop {
        assert!(game.sim().tick() < 20 * 300, "no attack");
        let d = game.think();
        if d[1].iter().any(|c| matches!(c, Command::Attack { .. })) {
            break;
        }
        game.step(d);
    }
    let snapshot = game.sim().snapshot();
    let tanks: Vec<Vec2> = snapshot
        .units
        .iter()
        .filter(|u| u.owner == PlayerId(1) && u.hp > 0)
        .map(|u| u.position)
        .collect();
    let n = i64::try_from(tanks.len()).unwrap();
    let sum = tanks.iter().fold(Vec2::ZERO, |a, &p| a + p);
    let center = Vec2::new(sum.x / n, sum.y / n);
    let off = center - enemy;
    // At least 30 degrees off the direct line (the -x axis).
    assert!(
        off.y.abs() * 2 > off.length(),
        "attacked along the direct line from {center:?}"
    );
    assert!(off.length() > Fixed::from_num(150), "{center:?}");
}
