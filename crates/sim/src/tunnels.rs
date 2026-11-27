//! Tunnels through rock massifs (prompt 148): a line of cells that stays open
//! beneath impassable mountain rock, the subterranean counterpart to bridges.
//!
//! One layer per cell: the mountain above an interior tunnel cell must be
//! impassable to every movement class (`map-check` and `gen_sketch` enforce
//! this), so no two units ever occupy different elevations at the same `(x, y)`.
//! Maps without tunnels hash and simulate bit-for-bit identically to before.

use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// Maximum supported tunnel length (meters).
pub const MAX_TUNNEL_LEN_M: i32 = 1500;
/// Distance from either portal (meters) that counts as the open portal mouth
/// rather than underground interior.
pub const PORTAL_APRON_M: i32 = 4;
/// Radius around a collapsed portal (meters) whose path cells are blocked.
pub const PORTAL_COLLAPSE_M: i32 = 6;
/// How far an observer outside an open portal (aligned with its axis) can see
/// into the bore, and how far a unit near an open portal can see out (meters).
pub const PORTAL_LOOK_M: i32 = 55;

/// Road or railway tunnel (`[[tunnel]]` `kind` in `map.toml`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TunnelKind {
    #[default]
    Road,
    Rail,
}

impl TunnelKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Road => "road",
            Self::Rail => "rail",
        }
    }

    pub const fn default_width_mm(self) -> u32 {
        match self {
            Self::Road => 6000,
            Self::Rail => 8000,
        }
    }

    pub const fn default_clearance_height_mm(self) -> u32 {
        match self {
            Self::Road => 4500,
            Self::Rail => 5500,
        }
    }

    pub const fn default_clearance_width_mm(self) -> u32 {
        match self {
            Self::Road => 4500,
            Self::Rail => 5500,
        }
    }

    /// Terrain id inside the bore (`2` = road, `1` = dirt_track/ballast).
    pub const fn terrain(self) -> u8 {
        match self {
            Self::Road => 2,
            Self::Rail => 1,
        }
    }
}

/// Raw `[[tunnel]]` entry in `map.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TunnelEntry {
    #[serde(default)]
    pub kind: TunnelKind,
    /// Polyline in whole meters: first point is portal 0, last point is portal 1.
    pub points_m: Vec<[i32; 2]>,
    #[serde(default)]
    pub width_mm: Option<u32>,
    #[serde(default)]
    pub clearance_height_mm: Option<u32>,
    #[serde(default)]
    pub clearance_width_mm: Option<u32>,
    /// Optional explicit sole elevation at `[portal 0, portal 1]` in millimeters.
    /// Defaults to ground height at each portal when omitted.
    #[serde(default)]
    pub portal_heights_mm: Option<[i32; 2]>,
}

/// One straight segment of a tunnel bore.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Segment {
    a: Vec2,
    b: Vec2,
    dir: Vec2,
    right: Vec2,
    len: Fixed,
    cum_start: Fixed,
}

/// A resolved tunnel on the map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tunnel {
    pub kind: TunnelKind,
    pub points: Vec<Vec2>,
    segments: Vec<Segment>,
    pub total_len: Fixed,
    pub half_width: Fixed,
    pub width_mm: u32,
    pub clearance_height_mm: u32,
    pub clearance_width_mm: u32,
    pub terrain: u8,
    /// Sole elevation (m) at portal 0 and portal 1.
    pub portal_heights: [Fixed; 2],
    /// Whether portal 0 and portal 1 are open (collapsed portals block all movement).
    pub portals_open: [bool; 2],
    lo: Vec2,
    hi: Vec2,
}

impl Tunnel {
    /// Builds and validates a tunnel from a `map.toml` entry.
    pub(crate) fn from_entry(
        entry: &TunnelEntry,
        ground: impl Fn(Vec2) -> Fixed,
    ) -> Result<Self, String> {
        if entry.points_m.len() < 2 {
            return Err("tunnel needs at least 2 points_m".into());
        }
        let points: Vec<Vec2> = entry
            .points_m
            .iter()
            .map(|p| Vec2::from_meters(p[0], p[1]))
            .collect();
        let width_mm = entry
            .width_mm
            .unwrap_or_else(|| entry.kind.default_width_mm());
        if !(4000..=16_000).contains(&width_mm) {
            return Err(format!(
                "tunnel width_mm must be 4000..=16000, got {width_mm}"
            ));
        }
        let half_width = Fixed::from_num(width_mm) / Fixed::from_num(2000);
        let clearance_height_mm = entry
            .clearance_height_mm
            .unwrap_or_else(|| entry.kind.default_clearance_height_mm());
        let clearance_width_mm = entry
            .clearance_width_mm
            .unwrap_or_else(|| entry.kind.default_clearance_width_mm());
        let mut segments = Vec::with_capacity(points.len() - 1);
        let mut cum = Fixed::ZERO;
        let mut lo = points[0];
        let mut hi = points[0];
        for w in points.windows(2) {
            let (a, b) = (w[0], w[1]);
            let d = b - a;
            let len = d.length();
            if len < Fixed::from_num(4) {
                return Err("tunnel segment must be at least 4 m long".into());
            }
            let dir = Vec2::new(d.x / len, d.y / len);
            let right = Vec2::new(-dir.y, dir.x);
            segments.push(Segment {
                a,
                b,
                dir,
                right,
                len,
                cum_start: cum,
            });
            cum += len;
            lo = Vec2::new(lo.x.min(b.x), lo.y.min(b.y));
            hi = Vec2::new(hi.x.max(b.x), hi.y.max(b.y));
        }
        if cum > Fixed::from_num(MAX_TUNNEL_LEN_M) {
            return Err(format!(
                "tunnel length {} m exceeds maximum {MAX_TUNNEL_LEN_M} m",
                cum.round()
            ));
        }
        let portal_heights = match entry.portal_heights_mm {
            Some([h0, h1]) => [
                Fixed::from_num(h0) / Fixed::from_num(1000),
                Fixed::from_num(h1) / Fixed::from_num(1000),
            ],
            None => [ground(points[0]), ground(points[points.len() - 1])],
        };
        let pad = half_width + Fixed::from_num(2);
        Ok(Self {
            kind: entry.kind,
            points,
            segments,
            total_len: cum,
            half_width,
            width_mm,
            clearance_height_mm,
            clearance_width_mm,
            terrain: entry.kind.terrain(),
            portal_heights,
            portals_open: [true, true],
            lo: lo - Vec2::new(pad, pad),
            hi: hi + Vec2::new(pad, pad),
        })
    }

    /// Converts back to a `map.toml` entry (for map editor saving).
    pub fn to_entry(&self) -> TunnelEntry {
        let to_mm = |v: Fixed| (v * Fixed::from_num(1000)).round().to_num::<i32>();
        TunnelEntry {
            kind: self.kind,
            points_m: self
                .points
                .iter()
                .map(|p| [p.x.round().to_num::<i32>(), p.y.round().to_num::<i32>()])
                .collect(),
            width_mm: Some(self.width_mm),
            clearance_height_mm: Some(self.clearance_height_mm),
            clearance_width_mm: Some(self.clearance_width_mm),
            portal_heights_mm: Some([to_mm(self.portal_heights[0]), to_mm(self.portal_heights[1])]),
        }
    }

    /// Axis-aligned bounds of the tunnel bore: `(min, max)`.
    pub const fn bounds(&self) -> (Vec2, Vec2) {
        (self.lo, self.hi)
    }

    /// World position of portal `portal` (`0` = start, `1` = end).
    pub fn portal_pos(&self, portal: usize) -> Vec2 {
        if portal == 0 {
            self.points[0]
        } else {
            self.points[self.points.len() - 1]
        }
    }

    /// Unit direction pointing OUT of the mountain at `portal`.
    pub fn portal_outward(&self, portal: usize) -> Vec2 {
        if portal == 0 {
            let d = self.segments[0].dir;
            Vec2::new(-d.x, -d.y)
        } else {
            self.segments[self.segments.len() - 1].dir
        }
    }

    /// Whether portal `portal` is open.
    pub const fn portal_open(&self, portal: usize) -> bool {
        if portal == 0 {
            self.portals_open[0]
        } else {
            self.portals_open[1]
        }
    }

    /// Projects `p` onto the nearest point on the tunnel polyline, returning
    /// `(arc_s, signed_across, seg_dir, dist_to_centerline)` when within the
    /// tunnel's bounding box.
    pub fn project(&self, p: Vec2) -> Option<(Fixed, Fixed, Vec2, Fixed)> {
        if p.x < self.lo.x || p.x > self.hi.x || p.y < self.lo.y || p.y > self.hi.y {
            return None;
        }
        let mut best: Option<(Fixed, Fixed, Vec2, Fixed)> = None;
        for s in &self.segments {
            let rel = p - s.a;
            let along = (rel.x * s.dir.x + rel.y * s.dir.y).clamp(Fixed::ZERO, s.len);
            let q = s.a + s.dir.scale(along);
            let d = (p - q).length();
            if best.is_none_or(|b| d < b.3) {
                let across = rel.x * s.right.x + rel.y * s.right.y;
                best = Some((s.cum_start + along, across, s.dir, d));
            }
        }
        best
    }

    /// Whether `p` lies inside the bore footprint (between the two portals,
    /// within `half_width` of the centerline).
    pub fn covers(&self, p: Vec2) -> bool {
        let Some((arc_s, _, _, d)) = self.project(p) else {
            return false;
        };
        d <= self.half_width && arc_s >= Fixed::ZERO && arc_s <= self.total_len
    }

    /// Whether `p` lies strictly underground inside the tunnel (beyond the open
    /// portal mouths by [`PORTAL_APRON_M`]).
    pub fn inside(&self, p: Vec2) -> bool {
        let Some((arc_s, _, _, d)) = self.project(p) else {
            return false;
        };
        let apron = Fixed::from_num(PORTAL_APRON_M);
        d <= self.half_width && arc_s >= apron && self.total_len - arc_s >= apron
    }

    /// Whether `p` is blocked by a collapsed portal of this tunnel.
    pub fn blocked_by_collapsed_portal(&self, p: Vec2) -> bool {
        let r = Fixed::from_num(PORTAL_COLLAPSE_M);
        for portal in 0..2 {
            if !self.portals_open[portal] && (p - self.portal_pos(portal)).length() <= r {
                return true;
            }
        }
        false
    }

    /// Interpolated sole elevation (m) at `p`.
    pub fn height_at(&self, p: Vec2) -> Fixed {
        let arc_s = self.project(p).map_or(Fixed::ZERO, |(s, _, _, _)| {
            s.clamp(Fixed::ZERO, self.total_len)
        });
        if self.total_len <= Fixed::ZERO {
            return self.portal_heights[0];
        }
        let t = arc_s / self.total_len;
        self.portal_heights[0] + (self.portal_heights[1] - self.portal_heights[0]) * t
    }

    /// Constant longitudinal grade between the two portals.
    pub fn grade(&self) -> Fixed {
        if self.total_len <= Fixed::ZERO {
            Fixed::ZERO
        } else {
            (self.portal_heights[1] - self.portal_heights[0]).abs() / self.total_len
        }
    }

    /// Whether a unit with catalog dimensions `(width_mm, height_mm)` fits
    /// through this tunnel's clearance gauge.
    pub const fn fits(&self, width_mm: u32, height_mm: u32) -> bool {
        width_mm <= self.clearance_width_mm && height_mm <= self.clearance_height_mm
    }

    /// Whether an observer at `viewer` outside open `portal` can look straight
    /// into the portal mouth (within `PORTAL_LOOK_M` and ~25° of the axis).
    pub fn portal_look_in(&self, portal: usize, viewer: Vec2, target: Vec2) -> bool {
        if !self.portals_open[portal] {
            return false;
        }
        let ppos = self.portal_pos(portal);
        let out = self.portal_outward(portal);
        let dv = viewer - ppos;
        let dist_v = dv.length();
        let look = Fixed::from_num(PORTAL_LOOK_M);
        if dist_v > look {
            return false;
        }
        // Observer must be on the outside of the portal, roughly aligned with its axis.
        if dist_v > Fixed::from_num(3) {
            let dot = (dv.x * out.x + dv.y * out.y) / dist_v;
            if dot < Fixed::from_num(9) / 10 {
                return false;
            }
        }
        // Target must be within PORTAL_LOOK_M inside that portal.
        let Some((arc_s, _, _, d)) = self.project(target) else {
            return false;
        };
        if d > self.half_width + Fixed::ONE {
            return false;
        }
        let depth = if portal == 0 {
            arc_s
        } else {
            self.total_len - arc_s
        };
        depth <= look && (viewer - target).length() <= look
    }

    /// Whether an observer at `a` and a target at `b` inside this same tunnel
    /// have an unobstructed bore line of sight (no sharp corner blocking them).
    pub fn bore_los(&self, a: Vec2, b: Vec2) -> bool {
        let d = b - a;
        let len = d.length();
        if len <= Fixed::from_num(4) {
            return true;
        }
        let steps = (len / Fixed::from_num(4)).ceil().to_num::<i32>().max(1);
        for k in 1..steps {
            let p = a + d.scale(Fixed::from_num(k) / Fixed::from_num(steps));
            if !self.covers(p) || self.blocked_by_collapsed_portal(p) {
                return false;
            }
        }
        true
    }

    /// Feeds static tunnel definition into the map content hash.
    pub(crate) fn hash_definition(&self, h: &mut StableHasher) {
        h.write_u8(self.kind as u8);
        h.write_u32(self.width_mm);
        h.write_u32(self.clearance_height_mm);
        h.write_u32(self.clearance_width_mm);
        h.write_u64(self.points.len() as u64);
        for p in &self.points {
            h.write_i64(p.x.to_bits());
            h.write_i64(p.y.to_bits());
        }
        for z in self.portal_heights {
            h.write_i64(z.to_bits());
        }
    }

    /// Feeds dynamic tunnel portal states into the simulation state hash.
    /// Writes NOTHING when `tunnels` is empty so maps without tunnels keep
    /// their exact bit-for-bit state hash.
    pub(crate) fn hash_state(tunnels: &[Self], h: &mut StableHasher) {
        for t in tunnels {
            h.write_u8(u8::from(t.portals_open[0]) | (u8::from(t.portals_open[1]) << 1));
        }
    }

    /// Formats `[[tunnel]]` sections for `map.toml`.
    pub fn to_toml(tunnels: &[Self]) -> String {
        let mut out = String::new();
        for t in tunnels {
            let e = t.to_entry();
            let pts: Vec<String> = e
                .points_m
                .iter()
                .map(|p| format!("[{}, {}]", p[0], p[1]))
                .collect();
            let _ = write!(
                out,
                "\n[[tunnel]]\nkind = \"{}\"\npoints_m = [{}]\n",
                e.kind.name(),
                pts.join(", ")
            );
            if let Some(w) = e.width_mm
                && w != e.kind.default_width_mm()
            {
                let _ = writeln!(out, "width_mm = {w}");
            }
            if let Some(ch) = e.clearance_height_mm
                && ch != e.kind.default_clearance_height_mm()
            {
                let _ = writeln!(out, "clearance_height_mm = {ch}");
            }
            if let Some(cw) = e.clearance_width_mm
                && cw != e.kind.default_clearance_width_mm()
            {
                let _ = writeln!(out, "clearance_width_mm = {cw}");
            }
            if let Some([h0, h1]) = e.portal_heights_mm {
                let _ = writeln!(out, "portal_heights_mm = [{h0}, {h1}]");
            }
        }
        out
    }
}
