//! Fixed-point math. All positions and distances are in meters.

use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

/// Fixed-point scalar: 32 integer bits, 32 fractional bits (Q32.32).
///
/// Range is about ±2.1 million km with a resolution of ~0.23 nm, far more than a
/// battlefield needs, while multiplication stays exact via 128-bit intermediates.
pub type Fixed = fixed::types::I32F32;

/// A point or direction on the ground plane.
///
/// `x` points east, `y` points south. The bridge maps `(x, y)` to Godot `(x, 0, y)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: Fixed,
    pub y: Fixed,
}

impl Vec2 {
    pub const ZERO: Self = Self {
        x: Fixed::ZERO,
        y: Fixed::ZERO,
    };

    pub const fn new(x: Fixed, y: Fixed) -> Self {
        Self { x, y }
    }

    /// Creates a vector from whole meters.
    pub fn from_meters(x: i32, y: i32) -> Self {
        Self::new(Fixed::from_num(x), Fixed::from_num(y))
    }

    /// Creates a vector from millimeters (exact for any `i32` input).
    pub fn from_millimeters(x: i32, y: i32) -> Self {
        Self::new(
            Fixed::from_num(x) / Fixed::from_num(1000),
            Fixed::from_num(y) / Fixed::from_num(1000),
        )
    }

    /// Euclidean length, computed with an integer square root.
    ///
    /// The result is the exact floor of the true length at Q32.32 resolution,
    /// so it is identical on every platform.
    pub fn length(self) -> Fixed {
        let x = i128::from(self.x.to_bits());
        let y = i128::from(self.y.to_bits());
        // Each square is < 2^126, so the sum fits into u128.
        let squared = x.unsigned_abs() * x.unsigned_abs() + y.unsigned_abs() * y.unsigned_abs();
        let root = squared.isqrt();
        // root < 2^64. It only exceeds i64::MAX for lengths beyond Fixed::MAX,
        // in which case we saturate instead of wrapping.
        Fixed::from_bits(i64::try_from(root).unwrap_or(i64::MAX))
    }

    /// Multiplies both components by a scalar.
    pub fn scale(self, factor: Fixed) -> Self {
        Self::new(self.x * factor, self.y * factor)
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_of_3_4_is_5() {
        assert_eq!(Vec2::from_meters(3, -4).length(), Fixed::from_num(5));
    }

    #[test]
    fn length_of_zero_is_zero() {
        assert_eq!(Vec2::ZERO.length(), Fixed::ZERO);
    }

    #[test]
    fn millimeters_convert_exactly_for_round_values() {
        let two_and_a_half = Fixed::from_num(5) / Fixed::from_num(2);
        assert_eq!(
            Vec2::from_millimeters(2500, -1000),
            Vec2::new(two_and_a_half, Fixed::from_num(-1))
        );
    }
}
