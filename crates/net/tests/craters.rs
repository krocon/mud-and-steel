//! Prompt 021: two peers play the demo battle in lockstep over loopback;
//! the craters its ground hits leave are the same on both, sample by sample.

mod common;

use common::{pair, run};

const TICKS: u64 = 1000;

#[test]
fn craters_are_identical_on_two_peers() {
    let (mut a, mut b) = pair();
    run(&mut a, &mut b, TICKS);

    assert_eq!(a.hashes, b.hashes, "per-tick hashes differ");
    assert_eq!(a.session.desync(), None);
    assert_eq!(b.session.desync(), None);
    assert!(a.sim.crater_count() > 0, "the demo battle made no crater");
    assert_eq!(a.sim.crater_count(), b.sim.crater_count());
    let (ma, mb) = (a.sim.map(), b.sim.map());
    assert_eq!(ma.heights_mm(), mb.heights_mm());
    assert_eq!(ma.terrain_ids(), mb.terrain_ids());
    // The ground really changed, and the map identity did not.
    let loaded = sim::scenario::builtin_map();
    assert_ne!(ma.heights_mm(), loaded.heights_mm());
    assert_eq!(ma.hash(), loaded.hash());
}
