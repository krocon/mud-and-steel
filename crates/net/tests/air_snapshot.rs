//! Prompt 033: aircraft, bombs, cooldowns and reconnaissance viewsheds are
//! part of the snapshot (prompt 038): a game saved during an air attack
//! continues exactly like the original.

use net::snapshot;
use sim::scenario::{builtin_map, demo_config};
use sim::{AirMission, Command, PlayerCommand, PlayerId, Sim, Vec2};

#[test]
fn saved_during_an_air_attack_continues_identically() {
    let config = demo_config();
    let map = builtin_map();
    let mut original = Sim::new(&config, map.clone(), 7).unwrap();
    // Player 0 calls bombers and a reconnaissance flight on player 1's
    // start, player 1 fighters on player 0's.
    let center = |p: PlayerId| {
        let own: Vec<Vec2> = config
            .spawns
            .iter()
            .filter(|s| s.owner == p)
            .map(|s| s.position)
            .collect();
        let n = sim::Fixed::from_num(own.len());
        own.iter()
            .fold(Vec2::ZERO, |sum, &q| sum + Vec2::new(q.x / n, q.y / n))
    };
    let calls = [
        PlayerCommand::new(
            PlayerId(0),
            Command::CallAir {
                mission: AirMission::Bomb,
                target: center(PlayerId(1)),
            },
        ),
        PlayerCommand::new(
            PlayerId(0),
            Command::CallAir {
                mission: AirMission::Recon,
                target: center(PlayerId(1)),
            },
        ),
        PlayerCommand::new(
            PlayerId(1),
            Command::CallAir {
                mission: AirMission::Strafe,
                target: center(PlayerId(0)),
            },
        ),
    ];
    original.step(&calls);
    let mut saved_with_bombs = false;
    for at in [60u64, 200, 320] {
        while original.tick() < at {
            original.step(&[]);
        }
        let snap = original.snapshot();
        assert!(!snap.aircraft.is_empty(), "no aircraft at tick {at}");
        saved_with_bombs |= !snap.bombs.is_empty();
        let bytes = snapshot::encode(&original, &map);
        let mut restored = snapshot::decode(&bytes, map.clone()).expect("snapshot restores");
        assert_eq!(restored.state_hash(), original.state_hash());
        let mut copy = original.clone();
        for _ in 0..600 {
            copy.step(&[]);
            restored.step(&[]);
            assert_eq!(restored.state_hash(), copy.state_hash(), "from tick {at}");
            assert_eq!(restored.air_events(), copy.air_events());
        }
    }
    assert!(saved_with_bombs, "no save while bombs fell");
}
