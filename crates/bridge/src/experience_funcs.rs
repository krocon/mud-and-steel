//! `SimHost` functions for experience and ranks (prompt 050): rank and
//! kills of every unit for the unit panel, the soldiers' kit
//! (`scenes/world/infantry_view.gd`) and the kill rings on gun barrels
//! (`scenes/world/veterancy_view.gd`). The simulation decides everything;
//! Godot shows it.

use crate::SimHost;
use godot::prelude::*;
use sim::UnitId;

#[godot_api(secondary)]
impl SimHost {
    /// Rank index of every unit (order of `unit_ids`; 0 = recruit).
    #[func]
    fn unit_ranks(&self) -> PackedInt32Array {
        self.current
            .units
            .iter()
            .map(|u| i32::from(u.experience.rank))
            .collect()
    }

    /// Enemy vehicles and guns every unit destroyed (order of `unit_ids`).
    #[func]
    fn unit_vehicle_kills(&self) -> PackedInt32Array {
        self.current
            .units
            .iter()
            .map(|u| i32::from(u.experience.vehicle_kills))
            .collect()
    }

    /// Names of the ranks, lowest first (`data/experience.toml`).
    #[func]
    fn rank_names(&self) -> PackedStringArray {
        let Some(game) = &self.game else {
            return PackedStringArray::new();
        };
        game.sim
            .experience_rules()
            .ranks
            .iter()
            .map(|r| {
                GString::from(crate::language_funcs::translated("RANK_", &r.name, &r.name).as_str())
            })
            .collect()
    }

    /// Experience of unit `id` for the unit panel (empty if it is not in
    /// the snapshot): `rank`, `rank_name`, `xp`, `rank_xp` (points where the
    /// rank began), `next_xp` (points for the next rank, -1 at the
    /// highest), `kills`, `vehicle_kills`, and the bonuses of the rank in
    /// percent of a recruit's value (`dispersion_pct`, `reload_pct`,
    /// `suppression_pct`).
    #[func]
    fn unit_experience(&self, id: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(id)) else {
            return d;
        };
        let Some(unit) = self.current.units.iter().find(|u| u.id == UnitId(id)) else {
            return d;
        };
        let rules = game.sim.experience_rules();
        let e = unit.experience;
        let rank = rules.rank(e.rank);
        d.set("rank", i64::from(e.rank));
        d.set(
            "rank_name",
            crate::language_funcs::translated("RANK_", &rank.name, &rank.name).as_str(),
        );
        d.set("xp", i64::from(e.xp));
        d.set("rank_xp", i64::from(rank.xp));
        d.set("next_xp", rules.next_xp(e.rank).map_or(-1, i64::from));
        d.set("kills", i64::from(e.kills));
        d.set("vehicle_kills", i64::from(e.vehicle_kills));
        d.set("dispersion_pct", i64::from(rank.dispersion_pct));
        d.set("reload_pct", i64::from(rank.reload_pct));
        d.set("suppression_pct", i64::from(rank.suppression_pct));
        d
    }

    /// Debug and self-test: sets the experience points of unit `id` (a
    /// veteran without a battle; shown from the next tick). Only in
    /// matches without other human players (it does not go over the
    /// network); the replay of such a match no longer verifies.
    #[func]
    fn debug_set_experience(&mut self, id: i64, xp: i64) {
        let (Some(game), Ok(id), Ok(xp)) =
            (self.game.as_mut(), u32::try_from(id), u32::try_from(xp))
        else {
            return;
        };
        if game.networked {
            godot_warn!("debug experience is not possible in network matches");
            return;
        }
        game.sim.set_unit_experience(UnitId(id), xp);
    }
}
