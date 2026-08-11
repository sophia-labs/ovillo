//! A seeded deterministic generator for the finite factorization search.
//!
//! The crate depends only on `mus-watsn`, serde, and `sha2` (consensus §5.2),
//! so the search carries its own generator rather than pulling one in.
//! SplitMix64 is the right size for this job: twenty lines, no state beyond a
//! `u64`, and a fixed seed reproduces a search exactly — which is what
//! "candidate count reported, monotone vs baseline" has to mean for a receipt
//! to replay.
//!
//! This is not a statistical-quality generator and is not used for anything
//! that claims to be a distribution. It selects search candidates; correctness
//! never depends on which candidate is chosen, only on the independent replay
//! of each one.

/// Steele/Lea/Flood SplitMix64.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A uniform value in `0..bound`, by rejection so the result is unbiased.
    /// `bound` must be non-zero.
    pub fn below(&mut self, bound: usize) -> usize {
        assert!(bound > 0, "bound must be positive");
        let bound = bound as u64;
        let limit = u64::MAX - (u64::MAX % bound);
        loop {
            let draw = self.next_u64();
            if draw < limit {
                return (draw % bound) as usize;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_gives_the_same_stream() {
        let mut left = SplitMix64::new(0x0a71_ad0c);
        let mut right = SplitMix64::new(0x0a71_ad0c);
        for _ in 0..64 {
            assert_eq!(left.next_u64(), right.next_u64());
        }
    }

    #[test]
    fn below_stays_in_range_and_covers_it() {
        let mut rng = SplitMix64::new(5);
        let mut seen = [false; 7];
        for _ in 0..1_000 {
            let value = rng.below(7);
            assert!(value < 7);
            seen[value] = true;
        }
        assert!(seen.iter().all(|flag| *flag));
        assert_eq!(rng.below(1), 0);
    }
}
