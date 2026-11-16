//! Formations and move modes of prompt 046: the shapes a group takes at
//! its target, and whether its units stop to fight on the way.
//!
//! A formation is laid out in local coordinates (`lateral`, `forward`) in
//! units of the group's slot spacing, centered on the target and turned so
//! that `forward` points along the facing of the order. Everything is
//! integer and fixed-point math, so every peer gets the same slots.
//!
//! [`Formation::Auto`] is resolved by the simulation from the ground at the
//! target (`sim/drill.rs`); this module only knows the concrete shapes.

use crate::math::{Fixed, Vec2};
use serde::{Deserialize, Serialize};

/// Shape of a group at its target (prompt 046).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Formation {
    /// Chosen by the simulation from the ground at the target: narrow
    /// ground → column, woods → dispersed, road → column, open field →
    /// wedge (with vehicles) or line (infantry only).
    #[default]
    Auto,
    /// Side by side across the facing, at most [`LINE_WIDTH`] per rank.
    Line,
    /// Point first, the others behind it to both sides (a "V" pointing
    /// forward).
    Wedge,
    /// One behind the other (two files from [`TWO_FILES_FROM`] units).
    Column,
    /// Inverted wedge (German "Breitkeil"): the flanks forward, the
    /// center back.
    BroadWedge,
    /// Staggered rows with twice the gap (woods, under artillery fire).
    Dispersed,
}

impl Formation {
    pub const ALL: [Self; 6] = [
        Self::Auto,
        Self::Line,
        Self::Wedge,
        Self::Column,
        Self::BroadWedge,
        Self::Dispersed,
    ];

    /// Stable code for the bridge and the HUD (index in [`Self::ALL`]).
    pub fn code(self) -> u8 {
        Self::ALL
            .iter()
            .position(|&f| f == self)
            .and_then(|i| u8::try_from(i).ok())
            .unwrap_or(0)
    }

    /// The formation of a code; unknown codes are `Auto`.
    pub fn from_code(code: i64) -> Self {
        usize::try_from(code)
            .ok()
            .and_then(|i| Self::ALL.get(i).copied())
            .unwrap_or(Self::Auto)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Line => "line",
            Self::Wedge => "wedge",
            Self::Column => "column",
            Self::BroadWedge => "broad_wedge",
            Self::Dispersed => "dispersed",
        }
    }
}

/// Who stops to fight on the way (prompt 046, attack-move).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Engage {
    /// Nobody: a plain move (units still fire on the move at will).
    Never,
    /// Infantry squads halt while an enemy is in range, vehicles drive on
    /// (the right-click default).
    #[default]
    Infantry,
    /// Every unit halts while an enemy is in range (attack-move).
    All,
}

impl Engage {
    pub const ALL: [Self; 3] = [Self::Never, Self::Infantry, Self::All];

    /// The mode of a bridge code (index in [`Self::ALL`]); unknown codes
    /// are `Never`.
    pub fn from_code(code: i64) -> Self {
        usize::try_from(code)
            .ok()
            .and_then(|i| Self::ALL.get(i).copied())
            .unwrap_or(Self::Never)
    }
}

/// Units per rank of a line; more form further ranks behind.
pub const LINE_WIDTH: usize = 8;

/// From this many units a column has two files.
pub const TWO_FILES_FROM: usize = 9;

/// Slot offsets `(lateral, forward)` of a concrete formation for `count`
/// units in units of the spacing, before centering. `Auto` is laid out as a
/// line (the simulation resolves it first).
fn layout(formation: Formation, count: usize) -> Vec<(i64, i64)> {
    let n = i64::try_from(count).unwrap_or(0);
    let centered_rows = |per_row: i64, step: i64, stagger: bool| {
        let per_row = per_row.max(1);
        (0..n)
            .map(|k| {
                let row = k / per_row;
                let in_row = per_row.min(n - row * per_row);
                // 2 * column - (in_row - 1): symmetric around zero, in
                // half steps (the caller halves all values at the end).
                let column = k % per_row;
                let shift = if stagger && row % 2 == 1 { 1 } else { 0 };
                ((2 * column - (in_row - 1) + shift) * step, -2 * row * step)
            })
            .collect::<Vec<_>>()
    };
    match formation {
        Formation::Auto | Formation::Line => {
            centered_rows(i64::try_from(LINE_WIDTH).unwrap_or(8), 1, false)
        }
        Formation::Column => {
            let files = if count >= TWO_FILES_FROM { 2 } else { 1 };
            centered_rows(files, 1, false)
        }
        Formation::Dispersed => {
            let mut columns = 1;
            while columns * columns < n {
                columns += 1;
            }
            centered_rows(columns, 2, true)
        }
        Formation::Wedge | Formation::BroadWedge => {
            let sign = if formation == Formation::Wedge { -1 } else { 1 };
            (0..n)
                .map(|k| {
                    let rank = (k + 1) / 2;
                    let side = if k % 2 == 1 { -1 } else { 1 };
                    (2 * side * rank, 2 * sign * rank)
                })
                .collect()
        }
    }
}

/// Target points of `formation` for `count` units around `center`, facing
/// `forward` (unit length), `spacing` meters apart, sorted front to back
/// and, within a rank, from left to right (see [`assign`]).
///
/// The slots are centered: their mean is `center`.
pub fn formation_points(
    formation: Formation,
    center: Vec2,
    forward: Vec2,
    count: usize,
    spacing: Fixed,
) -> Vec<(Fixed, Fixed, Vec2)> {
    let offsets = layout(formation, count);
    if offsets.is_empty() {
        return Vec::new();
    }
    let n = i64::try_from(offsets.len()).unwrap_or(1);
    let mean_lat = offsets.iter().map(|o| o.0).sum::<i64>();
    let mean_fwd = offsets.iter().map(|o| o.1).sum::<i64>();
    let half = spacing / 2;
    // Offsets are in half steps; the mean is subtracted in Fixed.
    let to_m = |v: i64, mean: i64| {
        half * Fixed::from_num(v) - half * Fixed::from_num(mean) / Fixed::from_num(n)
    };
    let right = right_of(forward);
    let mut points: Vec<(Fixed, Fixed, Vec2)> = offsets
        .into_iter()
        .map(|(lat, fwd)| {
            let (lat, fwd) = (to_m(lat, mean_lat), to_m(fwd, mean_fwd));
            let p = center + right.scale(lat) + forward.scale(fwd);
            (fwd, lat, p)
        })
        .collect();
    points.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    points
}

/// The lateral axis of `forward` (turned by 90°): "left to right" within
/// a rank means along this axis.
pub fn right_of(forward: Vec2) -> Vec2 {
    Vec2::new(forward.y, -forward.x)
}

/// Assigns `units` (any key with its position) to the slots of
/// [`formation_points`]: rank by rank from the front, the units most
/// forward along `forward` (relative to `origin`) to the front rank, and
/// within a rank from left to right. Independent of the order of `units`;
/// ties are broken by the key.
pub fn assign<K: Copy + Ord>(
    units: &[(K, Vec2)],
    slots: &[(Fixed, Fixed, Vec2)],
    origin: Vec2,
    forward: Vec2,
) -> Vec<(K, Vec2)> {
    let right = right_of(forward);
    let project = |p: Vec2| {
        let d = p - origin;
        (dot(d, forward), dot(d, right))
    };
    let mut order: Vec<(Fixed, Fixed, K)> = units
        .iter()
        .map(|&(k, p)| {
            let (f, l) = project(p);
            (f, l, k)
        })
        .collect();
    order.sort_by(|a, b| b.0.cmp(&a.0).then(a.2.cmp(&b.2)));
    let mut result = Vec::with_capacity(units.len().min(slots.len()));
    let mut start = 0;
    while start < slots.len() && start < order.len() {
        let rank = slots[start].0;
        let mut end = start;
        while end < slots.len() && slots[end].0 == rank {
            end += 1;
        }
        let end = end.min(order.len());
        let mut chunk: Vec<(Fixed, Fixed, K)> = order[start..end].to_vec();
        chunk.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));
        for (slot, unit) in slots[start..end].iter().zip(chunk) {
            result.push((unit.2, slot.2));
        }
        start = end;
    }
    result
}

fn dot(a: Vec2, b: Vec2) -> Fixed {
    a.x * b.x + a.y * b.y
}

/// `v` scaled to unit length; `None` for (nearly) zero vectors.
pub fn normalized(v: Vec2) -> Option<Vec2> {
    // Very long vectors are shortened first so the length cannot overflow.
    let limit = Fixed::from_num(1_000_000);
    if v.x.abs() > limit || v.y.abs() > limit {
        return None;
    }
    let length = v.length();
    (length > Fixed::from_num(1) / 1000).then(|| Vec2::new(v.x / length, v.y / length))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn south() -> Vec2 {
        Vec2::new(Fixed::ZERO, Fixed::ONE)
    }

    fn m(v: i32) -> Fixed {
        Fixed::from_num(v)
    }

    #[test]
    fn slots_are_centered_on_the_target() {
        for formation in Formation::ALL {
            for count in 1..20 {
                let center = Vec2::from_meters(100, -50);
                let slots = formation_points(formation, center, south(), count, m(10));
                assert_eq!(slots.len(), count);
                let n = Fixed::from_num(count);
                let mean = slots.iter().fold(Vec2::ZERO, |s, p| s + p.2);
                let mean = Vec2::new(mean.x / n, mean.y / n);
                let error = (mean - center).length();
                assert!(error < m(1) / 10, "{formation:?} {count}: {error}");
            }
        }
    }

    #[test]
    fn line_stands_across_the_facing_and_column_along_it() {
        let line = formation_points(Formation::Line, Vec2::ZERO, south(), 4, m(10));
        assert!(line.iter().all(|s| s.2.y == Fixed::ZERO));
        let xs: Vec<i32> = line.iter().map(|s| s.2.x.to_num()).collect();
        assert_eq!(xs, vec![-15, -5, 5, 15]);
        let column = formation_points(Formation::Column, Vec2::ZERO, south(), 4, m(10));
        assert!(column.iter().all(|s| s.2.x == Fixed::ZERO));
        let ys: Vec<i32> = column.iter().map(|s| s.2.y.to_num()).collect();
        assert_eq!(ys, vec![15, 5, -5, -15]);
        // Facing east: the line runs north-south.
        let east = Vec2::new(Fixed::ONE, Fixed::ZERO);
        let line = formation_points(Formation::Line, Vec2::ZERO, east, 3, m(10));
        assert!(line.iter().all(|s| s.2.x == Fixed::ZERO));
    }

    #[test]
    fn wedge_points_forward_and_broad_wedge_back() {
        let wedge = formation_points(Formation::Wedge, Vec2::ZERO, south(), 3, m(10));
        // The point is the most forward slot and in the middle.
        assert_eq!(wedge[0].2.x, Fixed::ZERO);
        assert!(wedge[0].2.y > wedge[1].2.y);
        let broad = formation_points(Formation::BroadWedge, Vec2::ZERO, south(), 3, m(10));
        // Two in front on the flanks, the center behind.
        assert_eq!(broad[2].2.x, Fixed::ZERO);
        assert!(broad[0].2.y > broad[2].2.y);
        assert!(broad[0].2.x != Fixed::ZERO && broad[1].2.x != Fixed::ZERO);
    }

    #[test]
    fn dispersed_keeps_twice_the_gap() {
        let slots = formation_points(Formation::Dispersed, Vec2::ZERO, south(), 4, m(10));
        for (i, a) in slots.iter().enumerate() {
            for b in &slots[i + 1..] {
                assert!((a.2 - b.2).length() >= m(19), "{:?} {:?}", a.2, b.2);
            }
        }
    }

    #[test]
    fn assignment_puts_the_front_units_in_front_and_ignores_order() {
        let slots = formation_points(Formation::Line, Vec2::ZERO, south(), 3, m(10));
        let units = [
            (2u32, Vec2::from_meters(10, -100)),
            (0, Vec2::from_meters(-10, -100)),
            (1, Vec2::from_meters(0, -100)),
        ];
        let a = assign(&units, &slots, Vec2::from_meters(0, -100), south());
        let mut reversed = units;
        reversed.reverse();
        let b = assign(&reversed, &slots, Vec2::from_meters(0, -100), south());
        assert_eq!(a, b);
        // Left to right: unit 0 (west) gets the western slot.
        let west = a.iter().find(|(k, _)| *k == 0).unwrap().1;
        assert_eq!(west.x.to_num::<i32>(), -10);
    }

    #[test]
    fn codes_round_trip() {
        for f in Formation::ALL {
            assert_eq!(Formation::from_code(i64::from(f.code())), f);
        }
        assert_eq!(Formation::from_code(99), Formation::Auto);
        assert_eq!(Engage::from_code(2), Engage::All);
        assert_eq!(Engage::from_code(-1), Engage::Never);
    }
}
