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
    let mut out = Snapshot::default();
    snapshot_into(sim, local, reveal_all, &mut out);
    out
}

/// [`snapshot`] into the buffers of `out` (prompt 016).
pub(crate) fn snapshot_into(sim: &Sim, local: PlayerId, reveal_all: bool, out: &mut Snapshot) {
    sim.snapshot_into((!reveal_all).then_some(local), out);
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

/// The fog overlay's mask (prompt 098): 255 where the local player sees,
/// 0 elsewhere (never seen and explored look the same now), with a border
/// of unseen cells around the grid so the map edge fades like the rest.
/// Each cell moves towards its state over a fraction of a second, so the
/// edge glides instead of jumping a whole 8 m cell. Presentation only.
#[derive(Default)]
pub(crate) struct FogFade {
    /// Cells per axis including the border on both sides.
    columns: usize,
    rows: usize,
    border: usize,
    /// `vision_version` the target was built from.
    version: Option<i64>,
    /// Reusable scratch buffer for `Sim::vision_states` (prompt 209).
    states: Vec<CellState>,
    target: Vec<u8>,
    shown: Vec<u8>,
    /// Whether `shown` still differs from `target`.
    moving: bool,
}

impl FogFade {
    /// Rebuilds the target from the cell states (row by row, unpadded).
    fn set_target_with(
        &mut self,
        columns: usize,
        rows: usize,
        border: usize,
        mut is_visible: impl FnMut(usize) -> bool,
    ) {
        let (w, h) = (columns + 2 * border, rows + 2 * border);
        let resized = w != self.columns || h != self.rows || border != self.border;
        self.columns = w;
        self.rows = h;
        self.border = border;
        self.target.clear();
        self.target.resize(w * h, 0);
        for r in 0..rows {
            let from = r * columns;
            let to = (r + border) * w + border;
            for (c, out) in self.target[to..to + columns].iter_mut().enumerate() {
                *out = if is_visible(from + c) {
                    VISIBLE
                } else {
                    UNEXPLORED
                };
            }
        }
        if resized {
            // No fade at the start of a match.
            self.shown.clone_from(&self.target);
            self.moving = false;
        } else {
            self.moving = true;
        }
    }

    #[cfg(test)]
    fn set_target(&mut self, columns: usize, rows: usize, border: usize, visible: &[bool]) {
        self.set_target_with(columns, rows, border, |i| visible[i]);
    }

    /// Rebuilds the target from `self.states` (populated via `Sim::vision_states`).
    fn set_target_from_states(&mut self, columns: usize, rows: usize, border: usize) {
        let (w, h) = (columns + 2 * border, rows + 2 * border);
        let resized = w != self.columns || h != self.rows || border != self.border;
        self.columns = w;
        self.rows = h;
        self.border = border;
        self.target.clear();
        self.target.resize(w * h, 0);
        for r in 0..rows {
            let from = r * columns;
            let to = (r + border) * w + border;
            for (out, &state) in self.target[to..to + columns]
                .iter_mut()
                .zip(&self.states[from..from + columns])
            {
                *out = if state == CellState::Visible {
                    VISIBLE
                } else {
                    UNEXPLORED
                };
            }
        }
        if resized {
            self.shown.clone_from(&self.target);
            self.moving = false;
        } else {
            self.moving = true;
        }
    }

    /// Moves every shown cell up to `step` towards its target.
    fn step(&mut self, step: u8) {
        let mut moving = false;
        for (shown, &target) in self.shown.iter_mut().zip(&self.target) {
            if *shown < target {
                *shown = shown.saturating_add(step).min(target);
            } else if *shown > target {
                *shown = shown.saturating_sub(step).max(target);
            }
            moving |= *shown != target;
        }
        self.moving = moving;
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// The fog overlay's mask (see [`FogFade`]): `(columns + 2 * border) *
    /// (rows + 2 * border)` bytes, advanced by `delta` seconds of a fade
    /// that takes `fade_seconds`. Empty when nothing changed since the last
    /// call, so the texture is only uploaded while the edge moves.
    #[func]
    fn vision_mask_fade(&mut self, delta: f32, fade_seconds: f32, border: i32) -> PackedByteArray {
        let version = self.vision_version();
        let Some(game) = &self.game else {
            return PackedByteArray::new();
        };
        let grid = game.sim.vision_grid();
        let (columns, rows) = (grid.columns as usize, grid.rows as usize);
        let border = usize::try_from(border).unwrap_or(0);
        let changed = self.fog_fade.version != Some(version)
            || self.fog_fade.columns != columns + 2 * border
            || self.fog_fade.rows != rows + 2 * border;
        if changed {
            if self.reveal_all {
                self.fog_fade
                    .set_target_with(columns, rows, border, |_| true);
            } else {
                game.sim
                    .vision_states(game.session.local_player(), &mut self.fog_fade.states);
                if self.fog_fade.states.len() != columns * rows {
                    return PackedByteArray::new();
                }
                self.fog_fade.set_target_from_states(columns, rows, border);
            }
            self.fog_fade.version = Some(version);
        } else if !self.fog_fade.moving {
            return PackedByteArray::new();
        }
        if self.fog_fade.moving {
            let share = if fade_seconds > 0.0 {
                (delta / fade_seconds).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let step = (share * 255.0).ceil().clamp(1.0, 255.0) as u8;
            self.fog_fade.step(step);
        }
        PackedByteArray::from(self.fog_fade.shown.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::FogFade;

    #[test]
    fn fog_fade_pads_and_glides() {
        let mut fade = FogFade::default();
        // 2 x 1 cells, the right one visible, one border cell around.
        fade.set_target(2, 1, 1, &[false, true]);
        assert_eq!(fade.shown, vec![0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 0, 0]);
        assert!(!fade.moving, "no fade at the start");
        fade.set_target(2, 1, 1, &[true, false]);
        fade.step(100);
        assert_eq!(&fade.shown[5..7], &[100, 155]);
        assert!(fade.moving);
        fade.step(200);
        assert_eq!(&fade.shown[5..7], &[255, 0]);
        assert!(!fade.moving);
    }
}
