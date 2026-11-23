//! Prepared ground of a map (prompt 124, `crate::prepared`): before the
//! first tick, the craters and field works `map.toml` lists are made with
//! the rules of the match: craters like ground hits (`plan_crater`), dug
//! works like a strongpoint's (finished, neutral, dug to full depth), wire
//! as a finished neutral obstacle. So they give the same cover, mud,
//! firing bays and look as their counterparts made during a match.
//!
//! Dug works go to `map_dig_pct` of the dig depth (`[positions]` of
//! `data/works.toml`): the full cut on sloped ground would close the trench
//! lines for everybody. Their cover is that of every dug sample.
//!
//! Everything is applied to the ground first and the path and vision grids
//! are brought up to date once for the whole area (a crater field of
//! thousands of craters would otherwise cost seconds); no crater events
//! are kept (the presentation reads the changed map at the start) and the
//! craters do not count as shell craters of the match. The craters' and
//! digs' samples enter the running crater checksum, the works the works
//! state: both are part of the state hash, like every crater and dig.

use super::Sim;
use crate::math::Vec2;
use crate::positions::NEUTRAL_OWNER;
use crate::prepared::PreparedWorkKind;
use crate::works::WorkKind;

const fn work_kind(kind: PreparedWorkKind) -> WorkKind {
    match kind {
        PreparedWorkKind::Trench => WorkKind::Trench,
        PreparedWorkKind::CommTrench => WorkKind::CommTrench,
        PreparedWorkKind::Wire => WorkKind::Wire,
        PreparedWorkKind::MgNest => WorkKind::MgNest,
        PreparedWorkKind::Dugout => WorkKind::Dugout,
    }
}

impl Sim {
    /// Makes the prepared ground of the map (see the module
    /// documentation). Called once by [`Sim::new`].
    pub(super) fn prepare_ground(&mut self) {
        if !self.map.prepared().has_ground() {
            return;
        }
        let prepared = self.map.prepared().clone();
        // Craters, planned on the loaded ground like the ground hits of one
        // tick.
        let plans: Vec<_> = prepared
            .craters
            .iter()
            .filter_map(|c| self.plan_crater(c.position, c.caliber_mm.saturating_mul(10)))
            .collect();
        let mut areas = self.deform_ground(plans, false);
        // Dug works: finished at once, dug one after the other (a plan is
        // relative to the ground the previous ones left).
        for w in prepared
            .works
            .iter()
            .filter(|w| w.kind != PreparedWorkKind::Wire)
        {
            self.add_prepared(work_kind(w.kind), &w.points, w.kind.is_point());
        }
        // Dug to `map_dig_pct` of the full depth (data/works.toml).
        let pct = self.works.rules.positions.map_dig_pct;
        let digs = std::mem::take(&mut self.works.pending_digs);
        for (id, steps) in digs {
            let step = (steps * pct).div_ceil(100).clamp(1, steps.max(1));
            let Some(k) = self.works.work_index(id) else {
                continue;
            };
            if let Some(plan) = self.dig_plan(k, step) {
                areas.extend(self.deform_ground(vec![plan], false));
            }
        }
        if let Some(&first) = areas.first() {
            let (low, high) = areas.iter().fold(first, |(lo, hi), &(a, b)| {
                (
                    Vec2::new(lo.x.min(a.x), lo.y.min(a.y)),
                    Vec2::new(hi.x.max(b.x), hi.y.max(b.y)),
                )
            });
            self.after_craters(&[(low, high)]);
        }
        // Wire last; its path cells are marked once over all of it.
        let mut wire = Vec::new();
        for w in prepared
            .works
            .iter()
            .filter(|w| w.kind == PreparedWorkKind::Wire)
        {
            for s in w.points.windows(2) {
                for (a, b, _) in self.pieces(NEUTRAL_OWNER, WorkKind::Wire, s[0], s[1], s[0]) {
                    wire.push(self.add_finished_work_with(
                        WorkKind::Wire,
                        NEUTRAL_OWNER,
                        a,
                        b,
                        false,
                    ));
                }
            }
        }
        self.refresh_works(&wire);
        self.works.positions.used |= !prepared.works.is_empty();
        self.craters.events.clear();
    }

    /// Finished neutral works of `kind` along `points` (each segment cut
    /// into pieces like an engineer's line) or at each point.
    fn add_prepared(&mut self, kind: WorkKind, points: &[Vec2], at_points: bool) {
        if at_points {
            for &p in points {
                for (a, b, _) in self.pieces(NEUTRAL_OWNER, kind, p, p, p) {
                    self.add_finished_work(kind, NEUTRAL_OWNER, a, b);
                }
            }
            return;
        }
        for s in points.windows(2) {
            for (a, b, _) in self.pieces(NEUTRAL_OWNER, kind, s[0], s[1], s[0]) {
                self.add_finished_work(kind, NEUTRAL_OWNER, a, b);
            }
        }
    }
}
