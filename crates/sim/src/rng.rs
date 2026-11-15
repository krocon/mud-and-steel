//! Seeded pseudo-random number generator (SplitMix64).
//!
//! Never seeded from OS entropy or time. The algorithm is fixed forever:
//! changing it changes every replay and breaks lockstep compatibility.

use serde::{Deserialize, Serialize};

/// SplitMix64 generator. Small, fast, and fully specified by its 64-bit state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Current internal state, e.g. for hashing.
    pub const fn state(&self) -> u64 {
        self.state
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform value in `0..bound` (Lemire's multiply-shift, without rejection).
    ///
    /// The tiny bias (< bound / 2^64) is irrelevant for gameplay and keeps the
    /// number of generator calls per draw constant.
    ///
    /// # Panics
    /// Panics if `bound` is zero.
    pub fn below(&mut self, bound: u32) -> u32 {
        assert!(bound > 0, "bound must be positive");
        let product = u128::from(self.next_u64()) * u128::from(bound);
        u32::try_from(product >> 64).expect("product >> 64 is below bound")
    }

    /// Uniform value in `min..=max`.
    ///
    /// # Panics
    /// Panics if `min > max`.
    pub fn range_i32(&mut self, min: i32, max: i32) -> i32 {
        assert!(min <= max, "min must not exceed max");
        let span = i64::from(max) - i64::from(min) + 1;
        let span = u128::try_from(span).expect("span is positive");
        let offset = (u128::from(self.next_u64()) * span) >> 64;
        let value = i64::from(min) + i64::try_from(offset).expect("offset < span");
        i32::try_from(value).expect("value lies within min..=max")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_reference_splitmix64_sequence() {
        // Reference values of SplitMix64 for seed 0.
        let mut rng = Rng::new(0);
        assert_eq!(rng.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(rng.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(rng.next_u64(), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn range_stays_in_bounds() {
        let mut rng = Rng::new(7);
        for _ in 0..10_000 {
            let v = rng.range_i32(-3, 3);
            assert!((-3..=3).contains(&v));
            assert!(rng.below(5) < 5);
        }
    }
}
