pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    // SplitMix64
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, n: usize) -> usize {
        ((u128::from(self.next_u64()) * n as u128) >> 64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn below_stays_in_range() {
        let mut rng = Rng::new(7);
        for _ in 0..10_000 {
            assert!(rng.below(5) < 5);
        }
    }

    #[test]
    fn the_same_seed_replays() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn every_bucket_gets_hit() {
        let mut rng = Rng::new(3);
        let mut seen = [0usize; 4];
        for _ in 0..10_000 {
            seen[rng.below(4)] += 1;
        }
        assert!(seen.iter().all(|&n| n > 2_000));
    }
}
