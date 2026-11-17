//! Heavy rocket strike (prompt 054): unlock, warning, cancelling by
//! destroying the battery position, the salvo, determinism.

use sim::scenario::{builtin_map, skirmish_config};
use sim::{
    Command, FireMode, PlayerCommand, PlayerId, Sim, SimConfig, Spawn, StrikeBlock, StrikeEvent,
    UnitId, Vec2,
};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

/// The skirmish with every unit holding fire; with `battery`, player 0
/// starts with a completed rocket battery position next to its
/// headquarters.
fn config(battery: bool) -> SimConfig {
    let mut config = skirmish_config(&[]);
    for spawn in &mut config.spawns {
        spawn.fire_mode = FireMode::HoldFire;
    }
    if battery {
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
    }
    config
}

fn start(battery: bool) -> Sim {
    Sim::new(&config(battery), builtin_map(), 11).unwrap()
}

fn run(sim: &mut Sim, ticks: u64) -> Vec<StrikeEvent> {
    let mut events = Vec::new();
    for _ in 0..ticks {
        sim.step(&[]);
        events.extend_from_slice(sim.strike_events());
    }
    events
}

fn step(sim: &mut Sim, player: PlayerId, command: Command) -> Vec<StrikeEvent> {
    sim.step(&[PlayerCommand::new(player, command)]);
    sim.strike_events().to_vec()
}

fn enemy_hq(sim: &Sim) -> (UnitId, Vec2) {
    let t = sim.unit_type_index("base_hq").unwrap();
    let s = sim
        .structures()
        .into_iter()
        .find(|s| s.owner == P1 && s.unit_type == t)
        .unwrap();
    (s.unit, sim.unit_position(s.unit).unwrap())
}

fn hp(sim: &Sim, id: UnitId) -> u32 {
    sim.snapshot()
        .units
        .iter()
        .find(|u| u.id == id)
        .map_or(0, |u| u.hp)
}

/// Runs until the strike is unlocked.
fn unlock(sim: &mut Sim) {
    let left = sim.strike_option(P0).unlock_left;
    run(sim, u64::from(left));
    assert_eq!(sim.strike_option(P0).unlock_left, 0);
}

#[test]
fn strike_needs_a_battery_and_the_late_game() {
    let mut sim = start(false);
    assert_eq!(sim.strike_option(P0).blocked, Some(StrikeBlock::NoBattery));
    let mut sim_b = start(true);
    let option = sim_b.strike_option(P0);
    assert_eq!(option.blocked, Some(StrikeBlock::Locked));
    assert!(option.battery.is_some());
    assert_eq!(
        u64::from(option.unlock_left),
        u64::from(sim_b.strike_rules().unlock_s) * 20
    );
    // A call before the unlock is ignored and costs nothing.
    let (_, target) = enemy_hq(&sim_b);
    let account = sim_b.account(P0);
    let events = step(&mut sim_b, P0, Command::HeavyStrike { target });
    assert!(events.is_empty());
    assert_eq!(sim_b.account(P0), account);
    assert!(sim_b.strikes().is_empty());
    // Without a battery, never.
    unlock(&mut sim);
    assert!(step(&mut sim, P0, Command::HeavyStrike { target }).is_empty());
}

#[test]
fn warning_then_salvo_hits_the_target_area() {
    let mut sim = start(true);
    unlock(&mut sim);
    assert_eq!(sim.strike_option(P0).blocked, None);
    // On the enemy's rifle squads.
    let squads: Vec<UnitId> = sim
        .snapshot()
        .units
        .iter()
        .filter(|u| u.owner == P1 && u.stance.is_some())
        .map(|u| u.id)
        .collect();
    let target = sim.unit_position(squads[0]).unwrap();
    let battery = sim.strike_option(P0).battery.unwrap();
    let hp_before: u32 = squads.iter().map(|&s| hp(&sim, s)).sum();
    let account = sim.account(P0).unwrap();
    let events = step(&mut sim, P0, Command::HeavyStrike { target });
    assert!(matches!(
        events[..],
        [StrikeEvent::Called { owner: P0, battery: b, .. }] if b == battery
    ));
    let rules = sim.strike_rules().clone();
    let paid = sim.account(P0).unwrap();
    for k in 0..3 {
        assert!(paid[k] + rules.cost[k] >= account[k]);
    }
    // Public: the strike, its target and the position; the enemy sees the
    // battery position during the warning; no second call meanwhile.
    let views = sim.strikes();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].target, target);
    assert!(sim.visible_to(P1, battery));
    assert_eq!(sim.strike_option(P0).blocked, Some(StrikeBlock::Busy));
    // Nothing flies during the warning.
    let warning = u64::from(rules.warning_s) * 20;
    let events = run(&mut sim, warning - 1);
    assert!(events.is_empty());
    assert!(sim.snapshot().shells.is_empty());
    assert!(sim.visible_to(P1, battery));
    // Then the rockets leave one after the other.
    let events = run(&mut sim, 2);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, StrikeEvent::Launched { owner: P0, .. }))
    );
    assert!(!sim.snapshot().shells.is_empty());
    // All of them land within the radius (plus dispersion of nothing) and
    // hurt the headquarters.
    let mut craters = 0;
    let mut hits = 0;
    for _ in 0..600 {
        sim.step(&[]);
        craters += sim.crater_events().len();
        for e in &sim.snapshot().events {
            if let sim::CombatEvent::Hit { at, shooter, .. } = e
                && *shooter == battery
            {
                hits += 1;
                let d = (at.ground() - target).length();
                assert!(d <= sim::Fixed::from_num(rules.radius_m + 1), "{d}");
            }
        }
    }
    assert!(hits >= rules.rockets as usize, "{hits} hits");
    assert!(craters > 0);
    assert!(sim.strikes().is_empty());
    let hp_after: u32 = squads.iter().map(|&s| hp(&sim, s)).sum();
    assert!(hp_after < hp_before, "{hp_after} of {hp_before}");
    // Cooling down afterwards.
    assert_eq!(sim.strike_option(P0).blocked, Some(StrikeBlock::Cooldown));
}

#[test]
fn losing_the_battery_in_the_warning_time_calls_the_strike_off() {
    let mut sim = start(true);
    unlock(&mut sim);
    let (hq, target) = enemy_hq(&sim);
    let battery = sim.strike_option(P0).battery.unwrap();
    let hp_before = hp(&sim, hq);
    step(&mut sim, P0, Command::HeavyStrike { target });
    run(&mut sim, 100);
    // Selling stands in for the enemy destroying it: the position is gone.
    let events = step(&mut sim, P0, Command::Sell { building: battery });
    let mut events = events;
    events.extend(run(&mut sim, 2));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, StrikeEvent::Cancelled { owner: P0, .. })),
        "{events:?}"
    );
    assert!(sim.strikes().is_empty());
    let events = run(&mut sim, 1200);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, StrikeEvent::Launched { .. }))
    );
    assert!(sim.snapshot().shells.is_empty());
    assert_eq!(hp(&sim, hq), hp_before);
}

#[test]
fn strike_is_deterministic() {
    let play = || {
        let mut sim = start(true);
        unlock(&mut sim);
        let (_, target) = enemy_hq(&sim);
        step(&mut sim, P0, Command::HeavyStrike { target });
        // A copy taken in the warning time (as a saved game) continues
        // exactly like the original.
        run(&mut sim, 300);
        let mut copy = sim.clone();
        run(&mut sim, 900);
        run(&mut copy, 900);
        assert_eq!(sim.state_hash(), copy.state_hash());
        sim.state_hash()
    };
    assert_eq!(play(), play());
}

#[test]
fn ammunition_explosions_are_reported() {
    let config = sim::scenario::demo_config();
    let (mut destroyed, mut exploded) = (Vec::new(), Vec::new());
    for seed in 1..=4 {
        let mut sim = Sim::new(&config, builtin_map(), seed).unwrap();
        for _ in 0..1500 {
            sim.step(&[]);
            for e in &sim.snapshot().events {
                if let sim::CombatEvent::Destroyed { unit, .. } = e {
                    destroyed.push((seed, *unit));
                }
            }
            for e in sim.strike_events() {
                if let StrikeEvent::AmmoExplosion { unit, .. } = e {
                    exploded.push((seed, *unit));
                }
            }
        }
    }
    // Some of the destroyed tanks blew up (40 %, data/infantry.toml), and
    // only destroyed ones.
    assert!(!exploded.is_empty(), "{} destroyed", destroyed.len());
    assert!(exploded.len() < destroyed.len());
    assert!(exploded.iter().all(|u| destroyed.contains(u)));
}

/// Tools only (prompt 076): a debug rocket leaves the given unit as a shell
/// with a shot event, like a strike rocket, and lands within the minimum
/// flight time plus a little.
#[test]
fn debug_rocket_flies_like_a_strike_rocket() {
    let mut sim = start(true);
    let battery = sim.strike_option(P0).battery.unwrap();
    let (_, target) = enemy_hq(&sim);
    assert!(!sim.debug_rocket(UnitId(u32::MAX), target, 1320));
    assert!(sim.debug_rocket(battery, target, 1320));
    // The shot event is queued with the launch and reported by the next step.
    let shot = sim.events().iter().any(|e| {
        matches!(e, sim::CombatEvent::Shot { unit, caliber_mm_x10: 1320, .. } if *unit == battery)
    });
    sim.step(&[]);
    let shot = shot
        || sim.events().iter().any(|e| {
            matches!(e, sim::CombatEvent::Shot { unit, caliber_mm_x10: 1320, .. } if *unit == battery)
        });
    assert!(shot);
    assert_eq!(sim.snapshot().shells.len(), 1);
    run(&mut sim, 20 * 20);
    assert!(sim.snapshot().shells.is_empty());
}
