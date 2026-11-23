//! Bridges: map objects with a drivable deck (prompt 014).
//!
//! A bridge is a placed object whose kind has a [`Deck`] rule
//! (`data/objects.toml`). Its deck is a box along the object's heading,
//! as long as the object and as wide as the roadway. The deck height runs
//! linearly between the ground heights at both ends of the axis, so it
//! always meets the banks; the map generator builds ramps up to it.
//!
//! [`crate::Map::height_at`] and [`crate::Map::terrain_at`] report the deck
//! where a point lies on an intact one, so movement and speed need no
//! bridge logic of their own; the path grid opens the cells along the deck
//! (`crate::path`). Design and reasons: `docs/MAPS.md` ("Bridges").

use crate::command::UnitId;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::objects::{Deck, MapObject, Shape};
use serde::{Deserialize, Serialize};

/// A bridge laid by a bridge-laying tank (prompt 055): no map object, the
/// deck alone. Saved in the [`crate::snapshot::MapDelta`] of a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaidBridge {
    /// The bridge-laying tank that carries it.
    pub layer: UnitId,
    pub center: Vec2,
    /// Unit vector along the deck.
    pub forward: Vec2,
    pub half_length: Fixed,
    pub half_width: Fixed,
    pub height_start: Fixed,
    pub height_end: Fixed,
    pub max_load_kg: u32,
    pub terrain: u8,
}

/// A bridge on the map, derived from a placed object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bridge {
    /// Index into [`crate::Map::objects`].
    pub object: u32,
    pub center: Vec2,
    /// Unit vector along the deck (the object's heading).
    pub forward: Vec2,
    /// Unit vector across the deck.
    pub right: Vec2,
    pub half_length: Fixed,
    /// Half of the roadway width.
    pub half_width: Fixed,
    /// Ground height (m) at the start of the axis (`center - forward *
    /// half_length`) and at its end.
    pub height_start: Fixed,
    pub height_end: Fixed,
    pub max_load_kg: u32,
    /// Terrain id whose speed applies on the deck.
    pub terrain: u8,
    /// Destroyed bridges (`intact == false`) have no deck.
    pub intact: bool,
    /// The bridge-laying tank of a laid bridge (prompt 055); `object` is
    /// `u32::MAX` then.
    pub laid_by: Option<UnitId>,
    /// See [`Deck::see_through`] (prompt 115).
    pub see_through: bool,
    /// See [`Deck::railing_cover_pct`] (prompt 115).
    pub railing_cover_pct: u32,
    /// The start (`hangs[0]`) or end (`hangs[1]`) of the deck does not meet
    /// the ground (more than [`crate::map::DECK_SIDE_STEP_MM`] apart): the
    /// inner joints of a long bridge of several pieces (prompt 115). Units
    /// leave the deck there only onto the next piece.
    pub hangs: [bool; 2],
    /// Axis-aligned bounds of the deck.
    lo: Vec2,
    hi: Vec2,
}

impl Bridge {
    /// The bridge of `object` with `deck`; `ground` gives the ground height
    /// at a point.
    pub(crate) fn new(
        index: u32,
        object: &MapObject,
        shape: Shape,
        deck: Deck,
        ground: impl Fn(Vec2) -> Fixed,
    ) -> Option<Self> {
        let Shape::Box { half_length, .. } = shape else {
            return None;
        };
        let (forward, right) = object.axes();
        let start = object.position - forward.scale(half_length);
        let end = object.position + forward.scale(half_length);
        let extent = Vec2::new(
            half_length * forward.x.abs() + deck.half_width * right.x.abs(),
            half_length * forward.y.abs() + deck.half_width * right.y.abs(),
        );
        Some(Self {
            object: index,
            center: object.position,
            forward,
            right,
            half_length,
            half_width: deck.half_width,
            height_start: ground(start),
            height_end: ground(end),
            max_load_kg: deck.max_load_kg,
            terrain: deck.terrain,
            intact: true,
            laid_by: None,
            see_through: deck.see_through,
            railing_cover_pct: deck.railing_cover_pct,
            hangs: [false; 2],
            lo: object.position - extent,
            hi: object.position + extent,
        })
    }

    /// The deck of a laid bridge (intact).
    pub fn laid(l: &LaidBridge) -> Self {
        let forward = l.forward;
        let right = Vec2::new(-forward.y, forward.x);
        let extent = Vec2::new(
            l.half_length * forward.x.abs() + l.half_width * right.x.abs(),
            l.half_length * forward.y.abs() + l.half_width * right.y.abs(),
        );
        Self {
            object: u32::MAX,
            center: l.center,
            forward,
            right,
            half_length: l.half_length,
            half_width: l.half_width,
            height_start: l.height_start,
            height_end: l.height_end,
            max_load_kg: l.max_load_kg,
            terrain: l.terrain,
            intact: true,
            laid_by: Some(l.layer),
            see_through: false,
            railing_cover_pct: 0,
            hangs: [false; 2],
            lo: l.center - extent,
            hi: l.center + extent,
        }
    }

    /// What describes a laid bridge (`None` for bridges of the map).
    pub fn laid_spec(&self) -> Option<LaidBridge> {
        Some(LaidBridge {
            layer: self.laid_by?,
            center: self.center,
            forward: self.forward,
            half_length: self.half_length,
            half_width: self.half_width,
            height_start: self.height_start,
            height_end: self.height_end,
            max_load_kg: self.max_load_kg,
            terrain: self.terrain,
        })
    }

    /// Both ends of the axis: `(start, end)`.
    pub fn ends(&self) -> (Vec2, Vec2) {
        let along = self.forward.scale(self.half_length);
        (self.center - along, self.center + along)
    }

    /// Position of `p` along and across the deck, relative to its center.
    pub fn local(&self, p: Vec2) -> (Fixed, Fixed) {
        let d = p - self.center;
        (
            d.x * self.forward.x + d.y * self.forward.y,
            d.x * self.right.x + d.y * self.right.y,
        )
    }

    /// Whether `p` lies on the deck (edges included), intact or not.
    pub fn covers(&self, p: Vec2) -> bool {
        if p.x < self.lo.x || p.x > self.hi.x || p.y < self.lo.y || p.y > self.hi.y {
            return false;
        }
        let (along, across) = self.local(p);
        along.abs() <= self.half_length && across.abs() <= self.half_width
    }

    /// Deck height (m) at `p`, from its position along the axis.
    pub fn height_at(&self, p: Vec2) -> Fixed {
        let (along, _) = self.local(p);
        let along = along.clamp(-self.half_length, self.half_length);
        let t = (along + self.half_length) / (self.half_length * 2);
        self.height_start + (self.height_end - self.height_start) * t
    }

    /// Whether the end of the deck beyond which `p` lies (seen along the
    /// axis) hangs in the air (see [`Bridge::hangs`]).
    pub fn hangs_towards(&self, p: Vec2) -> bool {
        let (along, _) = self.local(p);
        self.hangs[usize::from(along > Fixed::ZERO)]
    }

    /// Axis-aligned bounds of the deck: `(min, max)`.
    pub const fn bounds(&self) -> (Vec2, Vec2) {
        (self.lo, self.hi)
    }

    /// Long bridges (prompt 115): pieces of the map whose ends meet end to
    /// end on the same axis form one roadway. Its height runs linearly
    /// between the ground at the two outer ends of the whole chain, so the
    /// joints over the river lie as high as the ramps on land; joints that
    /// do not meet the ground are marked in [`Bridge::hangs`]. `ground`
    /// gives the ground height at a point. A single bridge keeps its own
    /// ends (as before prompt 115).
    pub(crate) fn link_chains(bridges: &mut [Self], ground: impl Fn(Vec2) -> Fixed) {
        let n = bridges.len();
        if n < 2 {
            return;
        }
        let tol = Fixed::ONE / 4;
        let meet = |a: Vec2, b: Vec2| (a.x - b.x).abs() <= tol && (a.y - b.y).abs() <= tol;
        let parallel = |a: &Self, b: &Self| {
            (a.forward.x * b.forward.y - a.forward.y * b.forward.x).abs() <= Fixed::ONE / 64
        };
        // Union-find over touching pieces, smallest index as the root.
        let mut parent: Vec<usize> = (0..n).collect();
        fn root(parent: &mut [usize], mut a: usize) -> usize {
            while parent[a] != a {
                parent[a] = parent[parent[a]];
                a = parent[a];
            }
            a
        }
        for a in 0..n {
            for b in a + 1..n {
                let (x, y) = (&bridges[a], &bridges[b]);
                if x.laid_by.is_some() || y.laid_by.is_some() || !parallel(x, y) {
                    continue;
                }
                let (xs, xe) = x.ends();
                let (ys, ye) = y.ends();
                if meet(xs, ys) || meet(xs, ye) || meet(xe, ys) || meet(xe, ye) {
                    let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
                    parent[ra.max(rb)] = ra.min(rb);
                }
            }
        }
        for first in 0..n {
            if root(&mut parent, first) != first {
                continue;
            }
            let members: Vec<usize> = (first..n)
                .filter(|&k| root(&mut parent, k) == first)
                .collect();
            if members.len() < 2 {
                continue;
            }
            // Positions along the axis of the first piece.
            let dir = bridges[first].forward;
            let origin = bridges[first].center;
            let along = |p: Vec2| (p.x - origin.x) * dir.x + (p.y - origin.y) * dir.y;
            let (mut lo, mut hi) = (None::<(Fixed, Vec2)>, None::<(Fixed, Vec2)>);
            for &k in &members {
                let (s, e) = bridges[k].ends();
                for p in [s, e] {
                    let t = along(p);
                    if lo.is_none_or(|(v, _)| t < v) {
                        lo = Some((t, p));
                    }
                    if hi.is_none_or(|(v, _)| t > v) {
                        hi = Some((t, p));
                    }
                }
            }
            let (Some((t0, p0)), Some((t1, p1))) = (lo, hi) else {
                continue;
            };
            if t1 <= t0 {
                continue;
            }
            let (g0, g1) = (ground(p0), ground(p1));
            let height = |p: Vec2| g0 + (g1 - g0) * ((along(p) - t0) / (t1 - t0));
            let step = Fixed::from_num(crate::map::DECK_SIDE_STEP_MM) / Fixed::from_num(1000);
            for &k in &members {
                let (s, e) = bridges[k].ends();
                let b = &mut bridges[k];
                b.height_start = height(s);
                b.height_end = height(e);
                b.hangs = [
                    (b.height_start - ground(s)).abs() > step,
                    (b.height_end - ground(e)).abs() > step,
                ];
            }
        }
    }

    pub(crate) fn hash_state(bridges: &[Self], h: &mut StableHasher) {
        // Nothing for maps without bridges, so their state hash is unchanged.
        for b in bridges {
            h.write_u8(u8::from(b.intact));
            // Laid bridges (prompt 055): where they lie.
            if let Some(l) = b.laid_spec() {
                h.write_u32(l.layer.0);
                for v in [
                    l.center.x,
                    l.center.y,
                    l.forward.x,
                    l.forward.y,
                    l.half_length,
                    l.half_width,
                    l.height_start,
                    l.height_end,
                ] {
                    h.write_i64(v.to_bits());
                }
                h.write_u32(l.max_load_kg);
            }
        }
    }
}
