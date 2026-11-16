//! Snapshots of the complete simulation state (prompt 038).
//!
//! A snapshot is everything [`Sim`] holds, so that a restored simulation
//! continues exactly like the original one: the same state hash now and
//! after any number of further ticks with the same commands. It is used for
//! saved games and for jumping in replays.
//!
//! Every field of `Sim` is saved through serde (`Sim` derives `Serialize`
//! and `Deserialize`), also derived data such as the path grid and the
//! vision rasters: rebuilding them would have to reproduce every incremental
//! change exactly, saving them cannot go wrong. A new field is included as
//! soon as its type derives the two traits; the compiler refuses a type that
//! does not.
//!
//! The map is the exception: it is large (20 MB for 4 km) and the caller
//! has it anyway. A snapshot names it by [`MapRef`] and stores only what
//! the match changed (craters, destroyed bridges) as a [`MapDelta`];
//! [`Sim::restore`] applies that to the loaded map.
//!
//! The byte encoding (postcard, compression, file header) lives in
//! `net::snapshot`; this crate has no encoder.

use crate::map::{Map, MapError, MapLayout, MapRef};
use crate::sim::Sim;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, OnceLock};

/// What a match changed in its map: heights and terrain ids of single
/// samples (craters, prompt 021) and the bridges' intact flags (prompt
/// 014). Samples are listed by row-major index, ascending.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapDelta {
    pub map: MapRef,
    pub heights: Vec<(u32, i32)>,
    pub terrain: Vec<(u32, u8)>,
    /// One flag per bridge of the map, in map order (laid bridges
    /// included).
    pub bridges_intact: Vec<bool>,
    /// Bridges laid by bridge-laying tanks (prompt 055), by index into
    /// the bridges of the map, ascending; they follow the map's own.
    #[serde(default)]
    pub laid: Vec<(u32, crate::bridges::LaidBridge)>,
}

/// A snapshot for writing: borrows the simulation.
#[derive(Serialize)]
pub struct SimSnapshot<'a> {
    pub map: MapDelta,
    pub sim: &'a Sim,
}

/// A snapshot as read back; turn it into a simulation with
/// [`Sim::restore`].
#[derive(Deserialize)]
pub struct SimSnapshotOwned {
    pub map: MapDelta,
    pub sim: Sim,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SnapshotError {
    /// The snapshot was taken on a different map.
    MapMismatch { expected: MapRef, actual: MapRef },
    /// The changes do not fit the map (index out of range, bridge count).
    BadDelta,
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MapMismatch { expected, actual } => write!(
                f,
                "snapshot needs map '{}' ({:#018x}), got '{}' ({:#018x})",
                expected.name, expected.hash, actual.name, actual.hash
            ),
            Self::BadDelta => write!(f, "snapshot map changes do not fit the map"),
        }
    }
}

impl std::error::Error for SnapshotError {}

impl Sim {
    /// The complete state for saving; `loaded` is the map as loaded (the
    /// one passed to [`Sim::new`]), against which the changes are taken.
    ///
    /// # Panics
    /// Panics if `loaded` is not the map of this match.
    pub fn save_state(&self, loaded: &Map) -> SimSnapshot<'_> {
        SimSnapshot {
            map: MapDelta::between(loaded, self.map()),
            sim: self,
        }
    }

    /// Rebuilds a simulation from a snapshot; `loaded` is the map as loaded
    /// (same content hash as the one of the snapshot).
    pub fn restore(snapshot: SimSnapshotOwned, loaded: Arc<Map>) -> Result<Self, SnapshotError> {
        let SimSnapshotOwned {
            map: delta,
            mut sim,
        } = snapshot;
        let map = delta.apply(loaded)?;
        sim.set_restored_map(map);
        Ok(sim)
    }
}

impl MapDelta {
    /// The changes from `loaded` to `now`.
    ///
    /// # Panics
    /// Panics if the two are not the same map (content hash).
    pub fn between(loaded: &Map, now: &Map) -> Self {
        assert_eq!(
            loaded.reference(),
            now.reference(),
            "snapshot against a different map"
        );
        let changed = |a: &[i32], b: &[i32]| -> Vec<(u32, i32)> {
            (0u32..)
                .zip(a.iter().zip(b))
                .filter(|(_, (x, y))| x != y)
                .map(|(i, (_, &y))| (i, y))
                .collect()
        };
        let heights = changed(loaded.heights_mm(), now.heights_mm());
        let terrain = (0u32..)
            .zip(loaded.terrain_ids().iter().zip(now.terrain_ids()))
            .filter(|(_, (x, y))| x != y)
            .map(|(i, (_, &y))| (i, y))
            .collect();
        Self {
            map: now.reference(),
            heights,
            terrain,
            bridges_intact: now.bridges().iter().map(|b| b.intact).collect(),
            laid: (0u32..)
                .zip(now.bridges())
                .filter_map(|(i, b)| Some((i, b.laid_spec()?)))
                .collect(),
        }
    }

    /// `loaded` with these changes; shares `loaded` if nothing changed.
    pub fn apply(&self, loaded: Arc<Map>) -> Result<Arc<Map>, SnapshotError> {
        if loaded.reference() != self.map {
            return Err(SnapshotError::MapMismatch {
                expected: self.map.clone(),
                actual: loaded.reference(),
            });
        }
        if self.bridges_intact.len() != loaded.bridges().len() + self.laid.len() {
            return Err(SnapshotError::BadDelta);
        }
        let bridges_changed = loaded
            .bridges()
            .iter()
            .zip(&self.bridges_intact)
            .any(|(b, &intact)| b.intact != intact);
        if self.heights.is_empty()
            && self.terrain.is_empty()
            && !bridges_changed
            && self.laid.is_empty()
        {
            return Ok(loaded);
        }
        let mut map = Arc::unwrap_or_clone(loaded);
        for (index, laid) in &self.laid {
            if map.put_laid_bridge(*index as usize, laid).is_none() {
                return Err(SnapshotError::BadDelta);
            }
        }
        let (heights, terrain) = map.rasters_mut();
        for &(i, h) in &self.heights {
            *heights.get_mut(i as usize).ok_or(SnapshotError::BadDelta)? = h;
        }
        for &(i, t) in &self.terrain {
            *terrain.get_mut(i as usize).ok_or(SnapshotError::BadDelta)? = t;
        }
        for (index, &intact) in self.bridges_intact.iter().enumerate() {
            map.set_bridge_intact(index, intact);
        }
        Ok(Arc::new(map))
    }
}

/// Stands in for the map while a simulation is deserialized;
/// [`Sim::restore`] replaces it at once.
pub(crate) fn placeholder_map() -> Arc<Map> {
    static MAP: OnceLock<Arc<Map>> = OnceLock::new();
    MAP.get_or_init(|| {
        let layout = MapLayout {
            name: "snapshot placeholder".into(),
            samples_x: 3,
            samples_y: 3,
            cell_size_mm: 1000,
            starts: Vec::new(),
            markers: Vec::new(),
        };
        let map: Result<Map, MapError> = Map::from_rasters(layout, vec![0; 9], vec![0; 9]);
        Arc::new(map.expect("a flat 2 m x 2 m map is valid"))
    })
    .clone()
}
