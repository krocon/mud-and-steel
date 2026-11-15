//! Stable 64-bit hash for state comparison (FNV-1a).
//!
//! Unlike `std::collections::hash_map::DefaultHasher`, the output of this hasher
//! is specified and never changes between Rust versions or platforms. All
//! integers are fed in little-endian byte order.

const OFFSET_BASIS: u64 = 0xCBF2_9CE4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01B3;

#[derive(Clone, Debug)]
pub struct StableHasher {
    state: u64,
}

impl Default for StableHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl StableHasher {
    pub const fn new() -> Self {
        Self {
            state: OFFSET_BASIS,
        }
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state ^= u64::from(byte);
            self.state = self.state.wrapping_mul(PRIME);
        }
    }

    pub fn write_u8(&mut self, value: u8) {
        self.write_bytes(&[value]);
    }

    pub fn write_u32(&mut self, value: u32) {
        self.write_bytes(&value.to_le_bytes());
    }

    pub fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    pub fn write_i64(&mut self, value: i64) {
        self.write_bytes(&value.to_le_bytes());
    }

    pub const fn finish(&self) -> u64 {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_fnv1a_reference_values() {
        // Published FNV-1a 64-bit test vectors.
        assert_eq!(StableHasher::new().finish(), 0xCBF2_9CE4_8422_2325);
        let mut h = StableHasher::new();
        h.write_bytes(b"a");
        assert_eq!(h.finish(), 0xAF63_DC4C_8601_EC8C);
        let mut h = StableHasher::new();
        h.write_bytes(b"foobar");
        assert_eq!(h.finish(), 0x8594_4171_F739_67E8);
    }
}
