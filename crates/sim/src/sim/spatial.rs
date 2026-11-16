//! Uniform grid of unit positions for neighborhood and area queries
//! (prompt 016): separation, target selection, line of fire and projectile
//! hits ask for the units near a point or a segment instead of looking at
//! every unit.
//!
//! Not state: the grid is rebuilt from the unit positions where they are
//! read (before combat, again before projectile flight), is neither hashed
//! nor part of snapshots, and every query applies the same exact test as
//! the loop over all units it replaces. Where the order of candidates can
//! change a result (the first of equally early projectile hits), they come
//! in increasing unit index, the order of the old loop.

use super::Unit;
use crate::math::{Fixed, Vec2};
use serde::{Deserialize, Serialize};

/// Edge of a grid cell in meters. Sight and weapon ranges are a few hundred
/// meters, separation a few meters; 64 m keeps both queries at a handful of
/// cells and the grid of a 4 km map at 4 000 cells.
const CELL_M: i32 = 64;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct UnitGrid {
    origin: Vec2,
    columns: u32,
    rows: u32,
    /// Per cell: index of its first unit in `items` (counting sort);
    /// `columns * rows + 1` entries.
    start: Vec<u32>,
    /// Unit indices, cell by cell, increasing within a cell.
    items: Vec<u32>,
    /// Largest `spread` of a living unit (separation reach).
    pub(super) max_spread: Fixed,
    /// Largest hit-box reach of any unit (wrecks included).
    pub(super) max_reach: Fixed,
}

impl UnitGrid {
    /// Sorts the units (all of them, dead ones too: wrecks stop rounds;
    /// not passengers, prompt 034: they are inside their vehicle) into the
    /// cells of a map of `size` centered on the origin.
    /// `reach` is the hit-box reach of a unit (its largest is kept).
    pub(super) fn rebuild(&mut self, units: &[Unit], size: Vec2, reach: impl Fn(&Unit) -> Fixed) {
        let cell = Fixed::from_num(CELL_M);
        self.origin = Vec2::new(-size.x / 2, -size.y / 2);
        self.columns = (size.x / cell).to_num::<u32>() + 1;
        self.rows = (size.y / cell).to_num::<u32>() + 1;
        let cells = (self.columns * self.rows) as usize;
        self.start.clear();
        self.start.resize(cells + 1, 0);
        let listed = |u: &Unit| !super::transport::aboard(u);
        for u in units.iter().filter(|u| listed(u)) {
            let c = self.cell_index(u.position);
            self.start[c + 1] += 1;
        }
        for c in 0..cells {
            self.start[c + 1] += self.start[c];
        }
        self.items.clear();
        self.items.resize(self.start[cells] as usize, 0);
        let mut next = self.start.clone();
        for (i, u) in (0u32..).zip(units).filter(|(_, u)| listed(u)) {
            let c = self.cell_index(u.position);
            self.items[next[c] as usize] = i;
            next[c] += 1;
        }
        self.max_spread = units
            .iter()
            .filter(|u| u.alive())
            .map(|u| u.spread)
            .max()
            .unwrap_or(Fixed::ZERO);
        self.max_reach = units.iter().map(reach).max().unwrap_or(Fixed::ZERO);
    }

    fn cell_of(&self, p: Vec2) -> (u32, u32) {
        let cell = Fixed::from_num(CELL_M);
        let clamp = |v: Fixed, n: u32| {
            if v <= Fixed::ZERO {
                0
            } else {
                (v / cell).to_num::<u32>().min(n - 1)
            }
        };
        (
            clamp(p.x - self.origin.x, self.columns),
            clamp(p.y - self.origin.y, self.rows),
        )
    }

    fn cell_index(&self, p: Vec2) -> usize {
        let (x, y) = self.cell_of(p);
        (y * self.columns + x) as usize
    }

    /// Indices of the units in the cells that overlap the box `low..high`
    /// into `out` (cleared first), in no particular order. A superset of
    /// the units inside the box: callers keep their exact test.
    pub(super) fn query(&self, low: Vec2, high: Vec2, out: &mut Vec<u32>) {
        out.clear();
        if self.columns == 0 {
            return;
        }
        let (x0, y0) = self.cell_of(low);
        let (x1, y1) = self.cell_of(high);
        for y in y0..=y1 {
            // Cells x0..=x1 of a row are contiguous in `items`.
            let row = y * self.columns;
            let (a, b) = (
                self.start[(row + x0) as usize],
                self.start[(row + x1) as usize + 1],
            );
            out.extend_from_slice(&self.items[a as usize..b as usize]);
        }
    }

    /// [`Self::query`] in increasing index order, for callers whose result
    /// depends on the order.
    pub(super) fn query_sorted(&self, low: Vec2, high: Vec2, out: &mut Vec<u32>) {
        self.query(low, high, out);
        out.sort_unstable();
    }
}
