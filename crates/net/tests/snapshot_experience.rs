//! Prompt 050: experience points, ranks and kills are part of the saved
//! game snapshot (prompt 038), and a restored match continues identically.

use net::snapshot;
use sim::scenario::{builtin_map, run_demo};

#[test]
fn experience_survives_save_and_restore() {
    // The demo battle is under way: tanks have kills and ranks.
    let mut original = run_demo(42, 380);
    let ranked: Vec<_> = original
        .snapshot()
        .units
        .iter()
        .filter(|u| u.experience.xp > 0)
        .map(|u| (u.id, u.experience))
        .collect();
    assert!(
        ranked.iter().any(|(_, e)| e.kills > 0 && e.rank > 0),
        "no ranked unit in the demo"
    );
    let bytes = snapshot::encode(&original, &builtin_map());
    let mut restored = snapshot::decode(&bytes, builtin_map()).expect("snapshot restores");
    for &(id, view) in &ranked {
        assert_eq!(restored.unit_experience(id), Some(view));
    }
    for _ in 0..200 {
        original.step(&[]);
        restored.step(&[]);
    }
    assert_eq!(original.state_hash(), restored.state_hash());
}
