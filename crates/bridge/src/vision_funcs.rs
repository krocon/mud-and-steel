//! `SimHost` functions for vision and fog of war (prompt 018). The
//! simulation decides what each player sees; Godot gets the local player's
//! snapshot (enemy units out of sight are left out) and the cell states as
//! an image for the fog overlay. "Reveal all" is a presentation-only debug
//! switch: it never reaches the simulation.

use crate::{SimHost, fixed_to_f32};
use godot::prelude::*;
use sim::{CellState, PlayerId, Sim, Snapshot};

/// Image values per cell state (L8): never seen, explored, visible.
const UNEXPLORED: u8 = 0;
const EXPLORED: u8 = 128;
const VISIBLE: u8 = 255;

/// What the local player gets to draw.
pub(crate) fn snapshot(sim: &Sim, local: PlayerId, reveal_all: bool) -> Snapshot {
    if reveal_all {
        sim.snapshot()
    } else {
        sim.snapshot_for(local)
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// Vision cells per axis (x, z). Zero before `start`.
    #[func]
    fn vision_grid_size(&self) -> Vector2i {
        self.game.as_ref().map_or(Vector2i::ZERO, |g| {
            let grid = g.sim.vision_grid();
            Vector2i::new(grid.columns as i32, grid.rows as i32)
        })
    }

    /// Edge of a vision cell in meters.
    #[func]
    fn vision_cell_size(&self) -> f32 {
        self.game
            .as_ref()
            .map_or(0.0, |g| fixed_to_f32(g.sim.vision_grid().cell))
    }

    /// Corner of the vision grid (x, z) in meters: the map's north-west corner.
    #[func]
    fn vision_origin(&self) -> Vector2 {
        self.game.as_ref().map_or(Vector2::ZERO, |g| {
            let o = g.sim.vision_grid().origin;
            Vector2::new(fixed_to_f32(o.x), fixed_to_f32(o.y))
        })
    }

    /// Changes whenever `vision_image` would return something else.
    #[func]
    fn vision_version(&self) -> i64 {
        self.game.as_ref().map_or(0, |g| {
            let v = g.sim.vision_version(g.session.local_player());
            let v = i64::try_from(v).unwrap_or(i64::MAX) & (i64::MAX >> 1);
            if self.reveal_all { -1 - v } else { v }
        })
    }

    /// The local player's cell states, one byte per cell row by row: 0 never
    /// seen, 128 explored, 255 visible (all 255 with "reveal all").
    #[func]
    fn vision_image(&self) -> PackedByteArray {
        let Some(game) = &self.game else {
            return PackedByteArray::new();
        };
        let cells = game.sim.vision_grid().cells();
        if self.reveal_all {
            return PackedByteArray::from(vec![VISIBLE; cells]);
        }
        let mut states = Vec::with_capacity(cells);
        game.sim
            .vision_states(game.session.local_player(), &mut states);
        states
            .iter()
            .map(|s| match s {
                CellState::Unexplored => UNEXPLORED,
                CellState::Explored => EXPLORED,
                CellState::Visible => VISIBLE,
            })
            .collect()
    }

    /// Debug: draw every unit and no fog (presentation only).
    #[func]
    fn set_reveal_all(&mut self, on: bool) {
        self.reveal_all = on;
        if let Some(game) = &self.game {
            self.current = snapshot(&game.sim, game.session.local_player(), on);
        }
    }

    #[func]
    fn is_reveal_all(&self) -> bool {
        self.reveal_all
    }
}
