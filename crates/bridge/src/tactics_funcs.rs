//! `SimHost` functions for prompt 047 (infantry tactics): fire teams,
//! suppression, blind fire and jams. The simulation decides everything;
//! `scenes/world/infantry_view.gd` only picks clips from it.

use crate::SimHost;
use godot::prelude::*;
use sim::UnitId;

#[godot_api(secondary)]
impl SimHost {
    /// Tactical flags of every soldier, in the order of `soldier_units`:
    /// 1 = clearing a jammed weapon, 2 = rushing with his fire team (not
    /// firing), 4 = member of the maneuver team (`sim::soldier_flags`).
    #[func]
    fn soldier_tactics(&self) -> PackedInt32Array {
        let Some(game) = self.game.as_ref() else {
            return PackedInt32Array::new();
        };
        let mut unit = None;
        let mut flags: Vec<u8> = Vec::new();
        self.current
            .soldiers
            .iter()
            .map(|s| {
                if unit != Some(s.unit) {
                    unit = Some(s.unit);
                    flags = game.sim.soldier_tactics(s.unit);
                }
                i32::from(flags.get(usize::from(s.index)).copied().unwrap_or(0))
            })
            .collect()
    }

    /// Tactical state of a squad: `{suppression (0..100), pinned, bounding
    /// (-1: no, else the moving team: 0 base of fire, 1 maneuver),
    /// blind_fire, close}`; empty for vehicles and unknown ids.
    #[func]
    fn squad_tactics(&self, id: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(game), Ok(id)) = (self.game.as_ref(), u32::try_from(id)) else {
            return d;
        };
        let Some(t) = game.sim.squad_tactics(UnitId(id)) else {
            return d;
        };
        d.set("suppression", i64::from(t.suppression));
        d.set("pinned", t.pinned);
        d.set("bounding", t.bounding.map_or(-1, i64::from));
        d.set("blind_fire", t.blind_fire);
        d.set("close", t.close);
        d
    }
}
