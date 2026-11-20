//! What the ground at a point does (prompt 100: the ground tooltip). Read
//! only: every value comes from the rules the simulation uses, nothing is
//! computed a second way.
//!
//! - Movement: the terrain type (`data/terrain.toml`: speed basis and
//!   percent), the soft-ground factor per running gear of the weather now
//!   (`data/weather.toml`), passability per size class from the path grid
//!   (terrain, objects, slope, works), and the steepest uphill grade with
//!   its slope factor.
//! - Concealment: the vision cell (solid obstacle, foliage, smoke).
//! - Cover: the map object under the point (objects that block tanks stop
//!   bullets; soft cover from `data/infantry.toml`), a dug position or a
//!   crater (`data/craters.toml`, `data/works.toml`).
//!
//! Fog of war (prompt 098: the landscape is always known): field works
//! count only if `player` sees them ([`Sim::works_for`]); mines never.

use super::Sim;
use super::craters::nearest_sample;
use crate::command::PlayerId;
use crate::infantry::Stance;
use crate::math::{Fixed, Vec2};
use crate::path::SizeClass;
use crate::terrain::SpeedBasis;
use crate::vision::Sight;
use crate::works::WorkKind;

/// How far around the point map objects and works count (m).
const REACH_MM: i32 = 1500;

/// The ground at one point, for presentation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundInfo {
    /// Terrain id (index in `data/terrain.toml`) and its name.
    pub terrain: u8,
    pub terrain_name: String,
    /// Units use their road speed here (else cross-country).
    pub road: bool,
    /// Percent of the basis speed (`speed_pct`).
    pub speed_pct: u32,
    /// Soft ground now, per running gear of `data/weather.toml` in file
    /// order: gear name and percent (100: firm).
    pub gears: Vec<(String, u32)>,
    /// Passable for infantry and small vehicles / for tanks and trucks.
    pub passable_small: bool,
    pub passable_large: bool,
    /// Steepest uphill grade from here in percent, and the slope factor in
    /// percent (`None`: too steep to climb).
    pub grade_pct: u32,
    pub slope_pct: Option<u32>,
    /// What the vision cell does to sight.
    pub sight: Sight,
    pub smoke: bool,
    /// Map object under the point: catalog id, and whether it is a ruin.
    pub object: Option<String>,
    pub ruin: bool,
    /// The object stops bullets (it blocks tanks and has a height).
    pub hard_cover: bool,
    /// Soft cover of the object in percent (`data/infantry.toml`).
    pub object_cover_pct: u32,
    /// Infantry can occupy the object.
    pub building: bool,
    /// A field work `player` sees here.
    pub work: Option<WorkKind>,
    /// Cover of a prone soldier in a dug position or crater here (percent).
    pub ground_cover_pct: u32,
}

impl Sim {
    /// The ground at `p` as `player` may know it (prompt 100). `None`
    /// outside the map.
    pub fn ground_info(&self, player: PlayerId, p: Vec2) -> Option<GroundInfo> {
        let map = &self.map;
        if !map.contains(p) {
            return None;
        }
        let terrain = map.terrain_at(p);
        let t = self.terrain.types.get(usize::from(terrain))?;
        let grid = &self.paths.grid;
        let (x, y) = grid.cell_of(p);
        let reach = Fixed::from_num(REACH_MM) / 1000;
        // Field works the player sees near the point.
        let work = self
            .works_for(player)
            .into_iter()
            .filter(|w| w.built && !matches!(w.kind, WorkKind::MinesAt | WorkKind::MinesAp))
            .find(|w| (nearest_on(w.from, w.to, p) - p).length() <= reach)
            .map(|w| w.kind);
        let passable = |class: SizeClass| {
            if grid.passable(class, x, y) {
                return true;
            }
            // Wire or hedgehogs the player does not see do not show.
            work.is_none()
                && grid.work_blocked(class, x, y)
                && t.passable
                && !grid.object_blocked(class, x, y)
        };
        let (grade_pct, slope_pct) = self.steepest(p);
        let vision = &self.vision.grid;
        let cell = vision.index_of(p);
        let object = self.object_near(p, reach);
        let kinds = map.object_kinds();
        let (object_id, ruin, hard, soft, building) =
            object.map_or((None, false, false, 0, false), |i| {
                let kind = &kinds[usize::from(map.objects()[i as usize].kind)];
                let ruin = self.object_is_ruin(i);
                (
                    Some(kind.id.clone()),
                    ruin,
                    !ruin && kind.blocks.large() && kind.height > Fixed::ZERO,
                    self.infantry.rules.cover_pct(&kind.id),
                    !ruin && self.infantry.rules.building(&kind.id).is_some(),
                )
            });
        let ground_cover_pct = nearest_sample(map, p).map_or(0, |sample| {
            let dug = work
                .and_then(|_| self.dug_cover_pct(sample, Stance::Prone))
                .unwrap_or(0);
            let crater = self.crater_cover_pct(sample, Stance::Prone);
            dug.max(crater)
        });
        Some(GroundInfo {
            terrain,
            terrain_name: t.name.clone(),
            road: t.speed == SpeedBasis::Road,
            speed_pct: t.speed_pct,
            gears: self.weather.gear_speeds(usize::from(terrain)),
            passable_small: passable(SizeClass::Small),
            passable_large: passable(SizeClass::Large),
            grade_pct,
            slope_pct,
            sight: vision.blocker_at(cell as usize),
            smoke: vision.smoke_at(cell),
            object: object_id,
            ruin,
            hard_cover: hard,
            object_cover_pct: soft,
            building,
            work,
            ground_cover_pct,
        })
    }

    /// Steepest uphill grade from `p` over one meter in eight directions
    /// (percent) and the slope factor there (percent, `None`: too steep).
    fn steepest(&self, p: Vec2) -> (u32, Option<u32>) {
        let here = self.map.height_at(p);
        let mut steepest = Fixed::ZERO;
        for (dx, dy) in [
            (1000, 0),
            (-1000, 0),
            (0, 1000),
            (0, -1000),
            (707, 707),
            (-707, 707),
            (707, -707),
            (-707, -707),
        ] {
            let q = p + Vec2::from_millimeters(dx, dy);
            if self.map.contains(q) {
                steepest = steepest.max(self.map.height_at(q) - here);
            }
        }
        let pct = (steepest * 100).to_num::<i64>().max(0);
        let factor = self
            .terrain
            .slope
            .factor(steepest)
            .map(|f| (f * 100).to_num::<u32>());
        (u32::try_from(pct).unwrap_or(u32::MAX), factor)
    }

    /// The map object whose ground shape lies within `reach` of `p`, the
    /// nearest first (lowest index on ties).
    fn object_near(&self, p: Vec2, reach: Fixed) -> Option<u32> {
        let map = &self.map;
        let kinds = map.object_kinds();
        (0u32..)
            .zip(map.objects())
            .filter(|(_, o)| {
                let shape = kinds[usize::from(o.kind)].shape;
                o.overlaps_square(shape, p, reach)
            })
            .min_by_key(|(i, o)| ((o.position - p).length(), *i))
            .map(|(i, _)| i)
    }
}

/// The point of the segment `a`–`b` nearest to `p`.
fn nearest_on(a: Vec2, b: Vec2, p: Vec2) -> Vec2 {
    let ab = b - a;
    let len2 = ab.x * ab.x + ab.y * ab.y;
    if len2 == Fixed::ZERO {
        return a;
    }
    let ap = p - a;
    let t = ((ap.x * ab.x + ap.y * ab.y) / len2).clamp(Fixed::ZERO, Fixed::ONE);
    a + Vec2::new(ab.x * t, ab.y * t)
}
