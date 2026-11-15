//! Undo and redo, one step per brush stroke.
//!
//! While a stroke is open, every change first records the old state of what
//! it touches: raster samples (one growing rectangle), the object list and
//! the metadata. Closing the stroke stores old and new state as one
//! [`Change`]. Only the touched rectangle of the rasters is kept, so long
//! strokes on large maps stay cheap.

use crate::{EditMap, Meta, PlacedObject, SampleRect};

/// Undo steps kept at most (the oldest are dropped).
pub const MAX_UNDO: usize = 100;

/// What an edit, undo or redo changed, for the presentation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Changed {
    /// Samples whose height or terrain may have changed.
    pub rect: Option<SampleRect>,
    pub objects: bool,
    pub meta: bool,
}

impl Changed {
    pub const fn any(self) -> bool {
        self.rect.is_some() || self.objects || self.meta
    }
}

#[derive(Clone, Debug)]
struct Rasters {
    rect: SampleRect,
    heights: Vec<i32>,
    terrain: Vec<u8>,
}

/// Old state recorded during the open stroke.
#[derive(Debug, Default)]
struct Stroke {
    rasters: Option<Rasters>,
    objects: Option<Vec<PlacedObject>>,
    meta: Option<Meta>,
}

#[derive(Debug)]
struct Change {
    rasters: Option<(Rasters, Rasters)>,
    objects: Option<(Vec<PlacedObject>, Vec<PlacedObject>)>,
    meta: Option<(Meta, Meta)>,
}

#[derive(Debug, Default)]
pub struct History {
    open: Option<Stroke>,
    undo: Vec<Change>,
    redo: Vec<Change>,
}

fn copy_rect<T: Copy>(data: &[T], stride: u32, rect: SampleRect) -> Vec<T> {
    let mut out = Vec::with_capacity(rect.count());
    for j in rect.j0..rect.j1() {
        let row = j as usize * stride as usize;
        out.extend_from_slice(&data[row + rect.i0 as usize..row + rect.i1() as usize]);
    }
    out
}

fn paste_rect<T: Copy>(data: &mut [T], stride: u32, rect: SampleRect, values: &[T]) {
    for (k, j) in (rect.j0..rect.j1()).enumerate() {
        let row = j as usize * stride as usize;
        let src = k * rect.w as usize;
        data[row + rect.i0 as usize..row + rect.i1() as usize]
            .copy_from_slice(&values[src..src + rect.w as usize]);
    }
}

impl History {
    pub const fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn undo_steps(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_steps(&self) -> usize {
        self.redo.len()
    }

    /// Records the old rasters of `rect` unless already recorded.
    fn touch_rasters(&mut self, rect: SampleRect, heights: &[i32], terrain: &[u8], stride: u32) {
        let Some(stroke) = self.open.as_mut() else {
            return;
        };
        match &mut stroke.rasters {
            None => {
                stroke.rasters = Some(Rasters {
                    rect,
                    heights: copy_rect(heights, stride, rect),
                    terrain: copy_rect(terrain, stride, rect),
                });
            }
            Some(old) if old.rect.contains(rect) => {}
            Some(old) => {
                // Grow: the new area comes from the current (still
                // untouched) rasters, the old area keeps its recorded values.
                let grown = old.rect.union(rect);
                let mut h = copy_rect(heights, stride, grown);
                let mut t = copy_rect(terrain, stride, grown);
                let inner = SampleRect {
                    i0: old.rect.i0 - grown.i0,
                    j0: old.rect.j0 - grown.j0,
                    ..old.rect
                };
                paste_rect(&mut h, grown.w, inner, &old.heights);
                paste_rect(&mut t, grown.w, inner, &old.terrain);
                *old = Rasters {
                    rect: grown,
                    heights: h,
                    terrain: t,
                };
            }
        }
    }

    pub(crate) fn touch_objects(&mut self, objects: &[PlacedObject]) {
        if let Some(stroke) = self.open.as_mut()
            && stroke.objects.is_none()
        {
            stroke.objects = Some(objects.to_vec());
        }
    }

    pub(crate) fn touch_meta(&mut self, meta: &Meta) {
        if let Some(stroke) = self.open.as_mut()
            && stroke.meta.is_none()
        {
            stroke.meta = Some(meta.clone());
        }
    }
}

impl EditMap {
    /// Starts a stroke: everything until [`EditMap::end_stroke`] is undone
    /// in one step. Edits outside a stroke are a step each.
    pub fn begin_stroke(&mut self) {
        if !self.history.is_open() {
            self.history.open = Some(Stroke::default());
        }
    }

    /// Closes the stroke and stores it for undo if anything changed.
    /// Returns what the stroke changed.
    pub fn end_stroke(&mut self) -> Changed {
        let Some(stroke) = self.history.open.take() else {
            return Changed::default();
        };
        let stride = self.samples_x;
        let rasters = stroke.rasters.and_then(|before| {
            let after = Rasters {
                rect: before.rect,
                heights: copy_rect(&self.heights, stride, before.rect),
                terrain: copy_rect(&self.terrain, stride, before.rect),
            };
            (after.heights != before.heights || after.terrain != before.terrain)
                .then_some((before, after))
        });
        let objects = stroke
            .objects
            .filter(|before| *before != self.objects)
            .map(|before| (before, self.objects.clone()));
        let meta = stroke
            .meta
            .filter(|before| *before != self.meta)
            .map(|before| (before, self.meta.clone()));
        let change = Change {
            rasters,
            objects,
            meta,
        };
        let changed = Self::changed_by(&change);
        if changed.any() {
            self.history.undo.push(change);
            if self.history.undo.len() > MAX_UNDO {
                self.history.undo.remove(0);
            }
            self.history.redo.clear();
        }
        changed
    }

    fn changed_by(change: &Change) -> Changed {
        Changed {
            rect: change.rasters.as_ref().map(|r| r.0.rect),
            objects: change.objects.is_some(),
            meta: change.meta.is_some(),
        }
    }

    /// Undoes the last stroke (closing an open one first). Returns what
    /// changed; nothing if there is nothing to undo.
    pub fn undo(&mut self) -> Changed {
        self.end_stroke();
        let Some(change) = self.history.undo.pop() else {
            return Changed::default();
        };
        self.apply(&change, false);
        let changed = Self::changed_by(&change);
        self.history.redo.push(change);
        changed
    }

    /// Redoes the last undone stroke.
    pub fn redo(&mut self) -> Changed {
        self.end_stroke();
        let Some(change) = self.history.redo.pop() else {
            return Changed::default();
        };
        self.apply(&change, true);
        let changed = Self::changed_by(&change);
        self.history.undo.push(change);
        changed
    }

    pub fn undo_steps(&self) -> usize {
        self.history.undo_steps()
    }

    pub fn redo_steps(&self) -> usize {
        self.history.redo_steps()
    }

    fn apply(&mut self, change: &Change, forward: bool) {
        let pick = |pair: &(Rasters, Rasters)| {
            if forward {
                pair.1.clone()
            } else {
                pair.0.clone()
            }
        };
        if let Some(pair) = &change.rasters {
            let r = pick(pair);
            paste_rect(&mut self.heights, self.samples_x, r.rect, &r.heights);
            paste_rect(&mut self.terrain, self.samples_x, r.rect, &r.terrain);
        }
        if let Some((before, after)) = &change.objects {
            self.objects = if forward {
                after.clone()
            } else {
                before.clone()
            };
        }
        if let Some((before, after)) = &change.meta {
            self.meta = if forward {
                after.clone()
            } else {
                before.clone()
            };
        }
        self.version += 1;
    }

    /// Records the rasters of `rect` before a brush changes them.
    pub(crate) fn touch(&mut self, rect: SampleRect) {
        self.history
            .touch_rasters(rect, &self.heights, &self.terrain, self.samples_x);
    }
}
