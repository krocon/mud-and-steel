//! `SimHost` functions for the gun crews drawn at the guns (prompt 193):
//! men left, pinned down and the time to the next round of every crewed
//! gun, read only (`scenes/world/gun_crew_view.gd`). The simulation decides
//! who falls; Godot only draws the men.

use crate::SimHost;
use godot::prelude::*;
use sim::UnitId;

/// Values per unit in [`SimHost::unit_crews`].
const CREW_STRIDE: usize = 4;

#[godot_api(secondary)]
impl SimHost {
    /// The crews of every unit (order of `unit_ids`), four values each:
    /// men left (-1: no crew), men at full strength, flags (1: pinned
    /// down) and ticks until the next round (0: loaded).
    #[func]
    fn unit_crews(&self) -> PackedInt32Array {
        let Some(game) = self.game.as_ref() else {
            return PackedInt32Array::new();
        };
        let mut out = Vec::with_capacity(self.current.units.len() * CREW_STRIDE);
        for u in &self.current.units {
            match game.sim.crew_view(u.id) {
                Some(c) => out.extend([
                    i32::from(c.men),
                    i32::from(c.full),
                    i32::from(c.pinned),
                    i32::try_from(c.reload_ticks).unwrap_or(i32::MAX),
                ]),
                None => out.extend([-1, 0, 0, 0]),
            }
        }
        PackedInt32Array::from(out.as_slice())
    }

    /// Debug and self-test: gun `id` loses `lose` men and is pinned down if
    /// `pin`. Only in matches without other human players (it does not go
    /// over the network).
    #[func]
    fn debug_gun_crew(&mut self, id: i64, lose: i64, pin: bool) {
        let (Some(game), Ok(id)) = (self.game.as_mut(), u32::try_from(id)) else {
            return;
        };
        if game.networked {
            godot_warn!("debug crew losses are not possible in network matches");
            return;
        }
        let lose = u16::try_from(lose.max(0)).unwrap_or(u16::MAX);
        game.sim.debug_gun_crew(UnitId(id), lose, pin);
    }
}
