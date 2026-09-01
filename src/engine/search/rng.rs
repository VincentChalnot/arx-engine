//! Minimal, dependency-free PRNG for root-move selection noise (blunder
//! chance, softmax move picking — see `SearchConfig::for_level`). Not
//! cryptographic and not used anywhere search-correctness-sensitive; only
//! exists so the `gui` binary doesn't have to pull in the `rand` crate for
//! a handful of coin flips per move (see the size-budget note in
//! `Cargo.toml`'s `[profile.gui]`).

/// xorshift64* generator, seeded from `std`'s own randomized `RandomState`
/// (itself OS-seeded) so callers don't need to source entropy manually.
pub struct Rng(u64);

impl Rng {
    pub fn new() -> Self {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        let seed = RandomState::new().build_hasher().finish() ^ 0x9E37_79B9_7F4A_7C15;
        Self::seeded(seed)
    }

    /// Deterministic constructor for tests.
    pub fn seeded(seed: u64) -> Self {
        // xorshift64* is undefined at a zero state; nudge it off zero.
        Rng(if seed == 0 { 0xDEAD_BEEF } else { seed })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform float in `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
}

impl Default for Rng {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_f32_stays_in_unit_range() {
        let mut rng = Rng::seeded(1);
        for _ in 0..10_000 {
            let v = rng.next_f32();
            assert!((0.0..1.0).contains(&v), "value {v} out of range");
        }
    }

    #[test]
    fn same_seed_reproduces_the_same_sequence() {
        let mut a = Rng::seeded(42);
        let mut b = Rng::seeded(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn zero_seed_does_not_produce_a_stuck_generator() {
        let mut rng = Rng::seeded(0);
        let a = rng.next_u64();
        let b = rng.next_u64();
        assert_ne!(a, b);
    }
}
