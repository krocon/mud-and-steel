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

use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::objects::{Deck, MapObject, Shape};

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
            lo: object.position - extent,
            hi: object.position + extent,
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

    /// Axis-aligned bounds of the deck: `(min, max)`.
    pub const fn bounds(&self) -> (Vec2, Vec2) {
        (self.lo, self.hi)
    }

    pub(crate) fn hash_state(bridges: &[Self], h: &mut StableHasher) {
        // Nothing for maps without bridges, so their state hash is unchanged.
        for b in bridges {
            h.write_u8(u8::from(b.intact));
        }
    }
}
