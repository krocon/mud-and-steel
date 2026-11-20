//! The tank orders and rules of prompt 093 in lockstep: two peers shell a
//! building on order, their tanks fire machine guns and run into each
//! other's infantry; every tick has the same hash on both, and the replay
//! plays back to the same end.

mod common;

use common::{pair, run};
use net::Replay;
use sim::scenario::COMMAND_INTERVAL;
use sim::{Command, ObjectStage, Sim, UnitId, Vec2};

const TICKS: u64 = 700;

/// The building nearest to `(x, y)` that can be shelled.
fn building_near(sim: &Sim, x: i32, y: i32) -> u32 {
    let at = Vec2::from_meters(x, y);
    let map = sim.map();
    (0u32..)
        .zip(map.objects())
        .filter(|&(i, o)| {
            sim.shellable(i)
                && map.object_kinds()[usize::from(o.kind)]
                    .id
                    .starts_with("env_house")
        })
        .min_by_key(|&(i, o)| ((o.position - at).length(), i))
        .map(|(i, _)| i)
        .expect("test_small has houses")
}

#[test]
fn shelling_orders_stay_in_sync_and_replay() {
    let (mut a, mut b) = pair();
    let house = building_near(&a.sim, 0, 60);
    // Player 0 shells the house with four tanks; player 1 sends the same
    // order for its own tanks a little later, and a forged one for tanks
    // that are not its own.
    //
    // Since prompt 097 (direct fire 1/8, demo starts at y = ±30 m) the
    // house, about 150 m from player 0's tanks, is beyond the 87.5 m of
    // their guns: they first drive some 70 m at real vehicle speed. The demo
    // scripts send Stop / MoveTo to random units every COMMAND_INTERVAL
    // ticks and would cancel a single order on the way (before, the house
    // was in range from the start and the first shots fell at once), so each
    // peer repeats its order one tick after every script command.
    let order = |units: std::ops::Range<u32>| Command::AttackObject {
        units: units.map(UnitId).collect(),
        object: house,
    };
    let repeated = |first: u64, units: std::ops::Range<u32>| {
        std::iter::once(first)
            .chain(
                (first / COMMAND_INTERVAL + 1..TICKS / COMMAND_INTERVAL)
                    .map(|k| k * COMMAND_INTERVAL + 1),
            )
            .map(move |t| (t, order(units.clone())))
            .collect::<Vec<_>>()
    };
    a.extra = repeated(20, 0..4);
    b.extra = repeated(40, 8..12);
    b.extra.push((41, order(0..8)));
    let before = a.sim.object_hp(house).unwrap().0;
    run(&mut a, &mut b, TICKS);
    assert_eq!(a.hashes, b.hashes);
    assert_eq!(a.sim.state_hash(), b.sim.state_hash());
    // The order did something on both peers.
    let after = a.sim.object_hp(house).unwrap().0;
    assert!(
        after < before || a.sim.object_stage(house) == Some(ObjectStage::Destroyed),
        "the house was not shelled: {after} of {before} hit points"
    );

    let replay = a.recorder.finish(&a.sim);
    assert_eq!(replay, b.recorder.finish(&b.sim));
    assert!(
        replay
            .frames
            .iter()
            .flat_map(|f| &f.commands)
            .any(|c| matches!(c.command, Command::AttackObject { .. }))
    );
    let decoded = Replay::decode(&replay.encode()).unwrap();
    assert_eq!(decoded, replay);
    let report = decoded.verify(sim::scenario::builtin_map()).unwrap();
    assert!(report.matches(), "{report:?}");
    assert_eq!(report.actual_hash, a.sim.state_hash());
}
