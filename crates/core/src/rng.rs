//! Small deterministic RNG (xoshiro256**), seeded via SplitMix64.
//!
//! Hand-rolled so the state is trivially serializable into saves and the
//! sequence never changes under us with a dependency upgrade.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut sm = seed;
        let mut next = || {
            sm = sm.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = sm;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        Self {
            s: [next(), next(), next(), next()],
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform in `[0, 1)`.
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f64()
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.f64() < p
    }

    /// Standard normal via Box–Muller.
    pub fn normal(&mut self) -> f64 {
        let u1 = self.f64().max(f64::MIN_POSITIVE);
        let u2 = self.f64();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }

    /// Index chosen with probability proportional to `weights`.
    /// Returns `None` if all weights are zero (or the slice is empty).
    pub fn weighted(&mut self, weights: &[f64]) -> Option<usize> {
        let total: f64 = weights.iter().sum();
        if total <= 0.0 {
            return None;
        }
        let mut x = self.f64() * total;
        for (i, w) in weights.iter().enumerate() {
            if x < *w {
                return Some(i);
            }
            x -= w;
        }
        weights.iter().rposition(|w| *w > 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn f64_in_unit_interval() {
        let mut r = Rng::new(1);
        for _ in 0..10_000 {
            let x = r.f64();
            assert!((0.0..1.0).contains(&x));
        }
    }

    #[test]
    fn weighted_respects_zero_weights() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            assert_eq!(r.weighted(&[0.0, 1.0, 0.0]), Some(1));
        }
        assert_eq!(r.weighted(&[0.0, 0.0]), None);
    }

    #[test]
    fn weighted_roughly_proportional() {
        let mut r = Rng::new(3);
        let mut counts = [0u32; 2];
        for _ in 0..20_000 {
            counts[r.weighted(&[1.0, 3.0]).unwrap()] += 1;
        }
        let ratio = f64::from(counts[1]) / f64::from(counts[0]);
        assert!((2.7..3.3).contains(&ratio), "ratio {ratio}");
    }
}
