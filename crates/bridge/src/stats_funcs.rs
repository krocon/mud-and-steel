//! `SimHost` functions for the post-match statistics (prompt 045): the
//! result screen reads them, and the game writes them as JSON next to the
//! replays. The numbers come from `sim::stats::MatchStats`, the same
//! observer the headless tournaments use.

use crate::SimHost;
use crate::economy_funcs::display_name;
use godot::prelude::*;

#[godot_api(secondary)]
impl SimHost {
    /// Statistics of the match so far: an array with one dictionary per
    /// player: `id`, `faction`, `lost`, `kills`, `lost_value`,
    /// `kill_value`, `produced_value`, `points_captured`, `points_lost`,
    /// `types` (array of `id`, `name`, `produced`, `lost`, `kills`, sorted
    /// by kills, then losses) and `samples` (array of `seconds`, `account`
    /// and `income` as [fuel, ammunition, manpower], `points`, `units`).
    /// Prompt 179: `losses` (array of `seconds`, `type`, `name`, `x`, `z`,
    /// `cause`, `by` (display name of the shooter's type or ""), `face`
    /// ("front", "side", "rear", "top" or ""), `seen`) and `report` (the
    /// three costliest groups: `cause`, `by`, `count`, `value`, `unseen`,
    /// `flank`, `hint` (translation key)). Shooters the player never saw
    /// are named too: the report is meant for after the match.
    #[func]
    fn match_stats(&self) -> VarArray {
        let mut out = VarArray::new();
        let Some(game) = &self.game else {
            return out;
        };
        let stats = &game.stats;
        let hz = u64::from(game.sim.tick_hz().max(1));
        for p in stats.players() {
            let mut rows: Vec<(u32, sim::stats::TypeCounts)> =
                p.types.iter().map(|(&t, &c)| (t, c)).collect();
            rows.sort_by(|a, b| (b.1.kills, b.1.lost, a.0).cmp(&(a.1.kills, a.1.lost, b.0)));
            let mut types = VarArray::new();
            for (t, c) in rows {
                let id = stats.type_name(t);
                let row = vdict! {
                    "id" => id,
                    "name" => display_name(id),
                    "produced" => i64::from(c.produced),
                    "lost" => i64::from(c.lost),
                    "kills" => i64::from(c.kills),
                };
                types.push(&row.to_variant());
            }
            let mut samples = VarArray::new();
            for s in &p.samples {
                let mut row = vdict! {
                    "seconds" => i64::try_from(s.tick / hz).unwrap_or(i64::MAX),
                    "points" => i64::from(s.points),
                    "units" => i64::from(s.units),
                };
                row.set(
                    "account",
                    &PackedInt64Array::from(s.account.map(i64::from).as_slice()),
                );
                row.set(
                    "income",
                    &PackedInt64Array::from(s.income.map(i64::from).as_slice()),
                );
                samples.push(&row.to_variant());
            }
            let mut entry = vdict! {
                "id" => i64::from(p.player.0),
                "faction" => p.faction.unwrap_or(""),
                "lost" => i64::from(p.lost()),
                "kills" => i64::from(p.kills()),
                "lost_value" => i64::try_from(p.lost_value).unwrap_or(i64::MAX),
                "kill_value" => i64::try_from(p.kill_value()).unwrap_or(i64::MAX),
                "produced_value" => i64::try_from(p.produced_value).unwrap_or(i64::MAX),
                "points_captured" => i64::from(p.points_captured),
                "points_lost" => i64::from(p.points_lost),
            };
            entry.set("types", &types);
            entry.set("samples", &samples);
            let by_name = |t: Option<u32>| {
                t.map_or(String::new(), |t| {
                    display_name(stats.type_name(t)).to_string()
                })
            };
            let mut losses = VarArray::new();
            for l in &p.losses {
                let face = match l.face {
                    Some(sim::Face::Front) => "front",
                    Some(sim::Face::Side) => "side",
                    Some(sim::Face::Rear) => "rear",
                    Some(sim::Face::Top) => "top",
                    None => "",
                };
                let row = vdict! {
                    "seconds" => i64::try_from(l.tick / hz).unwrap_or(i64::MAX),
                    "type" => stats.type_name(l.unit_type),
                    "name" => display_name(stats.type_name(l.unit_type)),
                    "x" => crate::fixed_to_f32(l.position.x),
                    "z" => crate::fixed_to_f32(l.position.y),
                    "cause" => l.cause.name(),
                    "by" => by_name(l.by_type),
                    "face" => face,
                    "seen" => l.shooter_seen,
                };
                losses.push(&row.to_variant());
            }
            let mut report = VarArray::new();
            for line in sim::stats::report(&p.losses, 3) {
                let row = vdict! {
                    "cause" => line.cause.name(),
                    "by" => by_name(line.by_type),
                    "count" => i64::from(line.count),
                    "value" => i64::try_from(line.value).unwrap_or(i64::MAX),
                    "unseen" => i64::from(line.unseen),
                    "flank" => i64::from(line.flank),
                    "hint" => line.hint.as_str(),
                };
                report.push(&row.to_variant());
            }
            entry.set("losses", &losses);
            entry.set("report", &report);
            out.push(&entry.to_variant());
        }
        out
    }

    /// The statistics as JSON (with a last sample at the current tick);
    /// empty without a match.
    #[func]
    fn match_stats_json(&self) -> GString {
        let Some(game) = &self.game else {
            return GString::new();
        };
        let mut stats = game.stats.clone();
        stats.finish(&game.sim);
        GString::from(stats.to_json(&game.sim).as_str())
    }
}
