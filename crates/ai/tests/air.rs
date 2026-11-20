//! The computer player and air support (prompt 033).

use ai::game::Game;
use ai::{Ai, Difficulty, PlayerView, SeenUnit};
use sim::scenario::{builtin_map, demo_config, skirmish_config};
use sim::{AirMission, Command, Fixed, PlayerId, Sim, UnitId, Vec2};

/// Normal calls bombers on a large enemy group far from its own units;
/// Easy never calls aircraft.
#[test]
fn normal_calls_bombers_on_a_large_group() {
    let config = demo_config();
    let map = builtin_map();
    let sim = Sim::new(&config, map.clone(), 1).unwrap();
    let mut view = PlayerView::new(&sim, PlayerId(0));
    let own: Vec<Vec2> = view.own.iter().map(|u| u.position).collect();
    // More than 250 m from every own unit (300 m before prompt 097 moved
    // the starts of test_small to 60 m apart; the bombers' safety margin
    // is 150 m).
    let far = (-8..=8)
        .flat_map(|x| (-8..=8).map(move |y| Vec2::from_meters(x * 100, y * 100)))
        .find(|&p| map.contains(p) && own.iter().all(|&q| (p - q).length() > Fixed::from_num(250)))
        .expect("a point far from player 0");
    let t34 = config
        .unit_types
        .iter()
        .position(|t| t.name == "su_t34_76_1943")
        .and_then(|i| u32::try_from(i).ok())
        .unwrap();
    view.enemies = (0..5)
        .map(|k| SeenUnit {
            id: UnitId(100 + k),
            owner: PlayerId(1),
            unit_type: t34,
            position: far + Vec2::from_meters(i32::try_from(k).unwrap() * 8, 0),
            facing: Vec2::from_meters(0, 1),
            aim: Vec2::from_meters(0, 1),
            hp: 1000,
            max_hp: 1000,
        })
        .collect();
    view.tick = 20;
    let mut normal = Ai::new(PlayerId(0), Difficulty::Normal, &config, &map, 1);
    let commands = normal.think(&view);
    assert!(
        commands.iter().any(|c| matches!(c,
            Command::CallAir { mission: AirMission::Bomb, target }
                if (*target - far).length() < Fixed::from_num(40))),
        "{commands:?}"
    );
    let mut easy = Ai::new(PlayerId(0), Difficulty::Easy, &config, &map, 1);
    view.tick = 80;
    assert!(
        !easy
            .think(&view)
            .iter()
            .any(|c| matches!(c, Command::CallAir { .. }))
    );
}

/// With barracks, Hard builds light flak once it saw enemy aircraft.
#[test]
fn hard_builds_flak_when_attacked_from_the_air() {
    let skirmish = skirmish_config(&[]);
    let map = builtin_map();
    let mut game = Game::new(
        &skirmish,
        map.clone(),
        5,
        &[
            (PlayerId(0), Difficulty::Hard),
            (PlayerId(1), Difficulty::Passive),
        ],
    )
    .unwrap();
    let barracks = game.sim().unit_type_index("base_barracks").unwrap();
    while !game
        .sim()
        .structures()
        .iter()
        .any(|s| s.owner == PlayerId(0) && s.unit_type == barracks && s.completed())
    {
        assert!(game.sim().tick() < 6000, "no barracks");
        let d = game.think();
        game.step(d);
    }
    let flak = game.sim().unit_type_index("de_flak38").unwrap();
    let mut view = PlayerView::new(game.sim(), PlayerId(0));
    view.tick = view.tick / 10 * 10 + 10;
    let builds_flak = |commands: &[Command]| {
        commands
            .iter()
            .any(|c| matches!(c, Command::Build { unit_type, .. } if *unit_type == flak))
    };
    let mut ai = Ai::new(PlayerId(0), Difficulty::Hard, &skirmish, &map, 5);
    let calm = ai.think(&view);
    assert!(!builds_flak(&calm), "{calm:?}");
    view.enemy_aircraft = 1;
    view.tick += 10;
    let alarmed = ai.think(&view);
    assert!(builds_flak(&alarmed), "{alarmed:?}");
}
