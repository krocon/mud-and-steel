//! The computer player and the heavy rocket strike (prompt 054).

use ai::{Ai, Difficulty, PlayerView};
use sim::scenario::{builtin_map, skirmish_config};
use sim::{FireMode, PlayerCommand, PlayerId, Sim, Spawn, StrikeEvent, Vec2};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

/// The skirmish, everybody holding fire, player 0 with a completed battery
/// position, run peacefully until the strike unlocks.
fn unlocked() -> (Sim, sim::SimConfig) {
    let mut config = skirmish_config(&[]);
    for spawn in &mut config.spawns {
        spawn.fire_mode = FireMode::HoldFire;
    }
    let hq = config
        .spawns
        .iter()
        .find(|s| s.owner == P0 && s.unit_type == "base_hq")
        .unwrap()
        .position;
    config.spawns.push(Spawn {
        owner: P0,
        unit_type: "base_rocket_battery".to_owned(),
        position: hq + Vec2::from_meters(30, 0),
        fire_mode: FireMode::HoldFire,
    });
    let mut sim = Sim::new(&config, builtin_map(), 3).unwrap();
    let left = sim.strike_option(P0).unlock_left;
    for _ in 0..left {
        sim.step(&[]);
    }
    (sim, config)
}

#[test]
fn computer_player_calls_the_strike_and_the_enemy_leaves_the_area() {
    let (mut sim, config) = unlocked();
    let map = builtin_map();
    let mut ais = [
        Ai::new(P0, Difficulty::Normal, &config, &map, 3),
        Ai::new(P1, Difficulty::Hard, &config, &map, 3),
    ];
    let mut called = None;
    let mut inside_at_call = 0;
    let mut inside_at_launch = None;
    for _ in 0..4000 {
        let mut commands = Vec::new();
        for ai in &mut ais {
            let view = PlayerView::new(&sim, ai.player());
            for c in ai.think(&view) {
                commands.push(PlayerCommand::new(ai.player(), c));
            }
        }
        sim.step(&commands);
        // Mobile units of player 1 within the strike's radius.
        let inside = |sim: &Sim, target: Vec2| {
            sim.snapshot()
                .units
                .iter()
                .filter(|u| u.owner == P1 && u.alive() && !sim.is_structure(u.unit_type))
                .filter(|u| (u.position - target).length() <= sim::Fixed::from_num(60))
                .count()
        };
        for e in sim.strike_events().to_vec() {
            match e {
                StrikeEvent::Called { owner, target, .. } => {
                    assert_eq!(owner, P0);
                    called = Some(target);
                    inside_at_call = inside(&sim, target);
                }
                StrikeEvent::Launched { target, .. } => {
                    inside_at_launch = Some(inside(&sim, target));
                }
                _ => {}
            }
        }
        if inside_at_launch.is_some() {
            break;
        }
    }
    assert!(called.is_some(), "no strike called");
    let at_launch = inside_at_launch.expect("the strike was fired");
    // The strike falls on the enemy's base, where its squads stand; they
    // leave the marked area before the rockets come.
    assert!(inside_at_call > 0);
    assert_eq!(at_launch, 0, "{inside_at_call} -> {at_launch}");
}
