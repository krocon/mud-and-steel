//! Prompt 049: wounds, buddy aid, medic jobs and bailed-out crews are part
//! of the saved game snapshot (prompt 038), and a restored match continues
//! identically.

use net::snapshot;
use sim::scenario::{builtin_map, run_demo};
use sim::{UnitId, WoundView};

#[test]
fn wounds_and_crews_survive_save_and_restore() {
    // The demo battle is under way: tanks burn, crews bailed out, men of
    // the rifle squads are wounded.
    let mut original = run_demo(42, 380);
    // A heavily wounded man whose comrades are about to drag him.
    let squad = original
        .snapshot()
        .units
        .iter()
        .find(|u| u.stance.is_some() && u.alive() && !original.is_bailed_crew(u.id))
        .map(|u| u.id)
        .expect("a squad in the demo");
    original.wound_soldier(squad, 0, true);
    original.step(&[]);
    let before = original.snapshot();
    assert!(
        before
            .units
            .iter()
            .any(|u| original.is_bailed_crew(u.id) && u.alive()),
        "no crew bailed out in the demo"
    );
    assert!(
        before
            .soldiers
            .iter()
            .any(|s| matches!(s.wound, WoundView::Down { .. })),
        "nobody is down"
    );
    let bytes = snapshot::encode(&original, &builtin_map());
    let mut restored = snapshot::decode(&bytes, builtin_map()).expect("snapshot restores");
    assert_eq!(restored.snapshot().soldiers, before.soldiers);
    assert_eq!(original.state_hash(), restored.state_hash());
    for _ in 0..300 {
        original.step(&[]);
        restored.step(&[]);
    }
    assert_eq!(original.state_hash(), restored.state_hash());
    assert_eq!(
        original.is_bailed_crew(UnitId(20)),
        restored.is_bailed_crew(UnitId(20))
    );
}
