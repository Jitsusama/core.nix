//! A small seeded generator, SplitMix64, for the forecast's simulated
//! paths: the same seed gives the same paths, so a forecast and its tests
//! repeat exactly.

#[derive(Debug, Clone)]
pub struct Random {
    state: u64,
}

impl Random {
    pub fn seeded(seed: u64) -> Random {
        Random { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform on [0, 1).
    #[expect(
        clippy::cast_precision_loss,
        reason = "both are at most 2^53, which f64 holds exactly"
    )]
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the product lies in [0, bound), and cutting it toward zero is the draw"
    )]
    pub fn below(&mut self, bound: usize) -> usize {
        (self.unit() * crate::stats::count(bound)) as usize % bound.max(1)
    }

    pub fn pick<'a, T>(&mut self, from: &'a [T]) -> &'a T {
        &from[self.below(from.len())]
    }

    /// A standard normal, by Box-Muller.
    pub fn normal(&mut self) -> f64 {
        let u = self.unit().max(f64::MIN_POSITIVE);
        let v = self.unit();
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_repeats() {
        let mut one = Random::seeded(7);
        let mut other = Random::seeded(7);
        assert!((0..100).all(|_| one.next_u64() == other.next_u64()));
    }

    #[test]
    fn units_stay_in_range_and_centre() {
        let mut random = Random::seeded(1);
        let draws: Vec<f64> = (0..10_000).map(|_| random.unit()).collect();
        assert!(draws.iter().all(|u| (0.0..1.0).contains(u)));
        let mean = crate::stats::mean(&draws);
        assert!((mean - 0.5).abs() < 0.02);
    }
}
