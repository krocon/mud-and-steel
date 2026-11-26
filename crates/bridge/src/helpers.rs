//! Shared conversion and event-queue helpers for `SimHost` (prompt 216).
//!
//! Keeps float ↔ fixed-point conversions, coordinate-axis mapping (sim `(x, y)`
//! with height `z` → Godot `(x, y, z)` where Godot Y is up), and bounded event
//! buffers in one place so the `*_funcs.rs` modules stay concise and testable.

use godot::prelude::*;
use sim::{Fixed, UnitId, Vec2, Vec3};

pub(crate) fn fixed_to_f32(value: Fixed) -> f32 {
    value.to_num::<f32>()
}

/// Converts meters from Godot (`f32`) to integer millimeters for `sim`.
/// Rejects non-finite values and coordinates beyond ±2,147 km (`i32::MAX` mm).
pub(crate) fn meters_to_millimeters(meters: f32) -> Option<i32> {
    let millimeters = (f64::from(meters) * 1000.0).round();
    (millimeters.is_finite() && millimeters.abs() <= f64::from(i32::MAX))
        .then_some(millimeters as i32)
}

/// Converts a Godot ground coordinate `(x, z)` in meters to a `sim::Vec2`.
pub(crate) fn ground_point(x: f32, z: f32) -> Option<Vec2> {
    Some(Vec2::from_millimeters(
        meters_to_millimeters(x)?,
        meters_to_millimeters(z)?,
    ))
}

/// Converts a simulation ground point `(x, y)` and height to Godot `(x, h, y)`.
pub(crate) fn ground_to_godot(position: Vec2, height: Fixed) -> Vector3 {
    Vector3::new(
        fixed_to_f32(position.x),
        fixed_to_f32(height),
        fixed_to_f32(position.y),
    )
}

/// Converts a simulation 3D point `(x, y, z=up)` to Godot `(x, z, y)`.
pub(crate) fn vec3_to_godot(p: Vec3) -> Vector3 {
    Vector3::new(fixed_to_f32(p.x), fixed_to_f32(p.z), fixed_to_f32(p.y))
}

/// Converts a simulation 2D point `(x, y)` to a flat Godot `(x, 0, y)` vector.
pub(crate) fn flat_to_godot(p: Vec2) -> Vector3 {
    Vector3::new(fixed_to_f32(p.x), 0.0, fixed_to_f32(p.y))
}

/// Converts Godot's `PackedInt64Array` of unit ids into simulation `UnitId`s,
/// dropping negative or out-of-range entries.
pub(crate) fn to_unit_ids(ids: &PackedInt64Array) -> Vec<UnitId> {
    ids.as_slice()
        .iter()
        .filter_map(|&id| u32::try_from(id).ok())
        .map(UnitId)
        .collect()
}

/// Appends `items` to `pending`, dropping the oldest entries if the length
/// exceeds `cap` so an unpolled event stream cannot grow without bound.
pub(crate) fn push_capped<T>(pending: &mut Vec<T>, items: impl IntoIterator<Item = T>, cap: usize) {
    pending.extend(items);
    if pending.len() > cap {
        let excess = pending.len() - cap;
        pending.drain(..excess);
    }
}

/// Appends tick-stamped events from the latest tick (which belongs to
/// `sim_tick.saturating_sub(1)`) while enforcing `cap`.
pub(crate) fn collect_tick_events<T: Copy>(
    pending: &mut Vec<(u64, T)>,
    sim_tick: u64,
    events: &[T],
    cap: usize,
) {
    if events.is_empty() {
        return;
    }
    let tick = sim_tick.saturating_sub(1);
    push_capped(pending, events.iter().map(|&e| (tick, e)), cap);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meters_to_millimeters_converts_and_rejects_invalid() {
        assert_eq!(meters_to_millimeters(0.0), Some(0));
        assert_eq!(meters_to_millimeters(12.345), Some(12_345));
        assert_eq!(meters_to_millimeters(-42.5), Some(-42_500));
        assert_eq!(meters_to_millimeters(0.0006), Some(1));
        assert_eq!(meters_to_millimeters(f32::NAN), None);
        assert_eq!(meters_to_millimeters(f32::INFINITY), None);
        assert_eq!(meters_to_millimeters(f32::NEG_INFINITY), None);
        assert_eq!(meters_to_millimeters(3_000_000.0), None);
    }

    #[test]
    fn ground_point_roundtrips_and_rejects_non_finite() {
        let p = ground_point(123.5, -45.25).expect("valid point");
        assert!((fixed_to_f32(p.x) - 123.5).abs() < 1e-3);
        assert!((fixed_to_f32(p.y) - (-45.25)).abs() < 1e-3);
        assert!(ground_point(f32::NAN, 0.0).is_none());
        assert!(ground_point(0.0, f32::INFINITY).is_none());
    }

    #[test]
    fn coordinate_axis_mappings_place_height_on_godot_y() {
        let pos = Vec2::from_millimeters(10_000, -25_000);
        let h = Fixed::from_num(3.5);
        assert_eq!(ground_to_godot(pos, h), Vector3::new(10.0, 3.5, -25.0));
        assert_eq!(flat_to_godot(pos), Vector3::new(10.0, 0.0, -25.0));

        let p3 = Vec3 {
            x: Fixed::from_num(4),
            y: Fixed::from_num(-9),
            z: Fixed::from_num(12),
        };
        assert_eq!(vec3_to_godot(p3), Vector3::new(4.0, 12.0, -9.0));
    }

    #[test]
    fn push_capped_keeps_newest_entries_in_order() {
        let mut buf = Vec::new();
        push_capped(&mut buf, [1, 2, 3], 4);
        assert_eq!(buf, vec![1, 2, 3]);
        push_capped(&mut buf, [4, 5, 6], 4);
        assert_eq!(buf, vec![3, 4, 5, 6]);
        push_capped(&mut buf, [7, 8, 9, 10, 11], 4);
        assert_eq!(buf, vec![8, 9, 10, 11]);
    }

    #[test]
    fn collect_tick_events_stamps_previous_tick_and_caps() {
        let mut buf = Vec::new();
        collect_tick_events(&mut buf, 0, &[10u32], 3);
        assert_eq!(buf, vec![(0, 10)]);
        collect_tick_events(&mut buf, 5, &[20, 30], 3);
        assert_eq!(buf, vec![(0, 10), (4, 20), (4, 30)]);
        collect_tick_events(&mut buf, 6, &[], 3);
        assert_eq!(buf, vec![(0, 10), (4, 20), (4, 30)]);
        collect_tick_events(&mut buf, 7, &[40, 50], 3);
        assert_eq!(buf, vec![(4, 30), (6, 40), (6, 50)]);
    }
}
