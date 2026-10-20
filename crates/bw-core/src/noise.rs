//! Counter-based deterministic noise.
//!
//! Design 0001 D8: every noise sample is a pure function of `(seed, stream, x, y)`.
//! There is no generator state, so output is bit-identical regardless of thread
//! count, tile size or evaluation order.
//!
//! Bit-exactness holds for a given platform and toolchain. The Gaussian transform
//! uses `ln` and `cos`, whose last-ulp results may differ between libm
//! implementations, so cross-platform output is statistically identical but not
//! guaranteed bit-identical.

/// SplitMix64 finaliser: a bijective 64-bit mixer with good avalanche behaviour.
#[inline]
pub fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Hash of a pixel coordinate within a named stream (e.g. one per grain octave).
#[inline]
pub fn hash_pixel(seed: u64, stream: u32, x: u32, y: u32) -> u64 {
    let key = mix64(seed ^ mix64(u64::from(stream).wrapping_add(0x9e37_79b9_7f4a_7c15)));
    mix64(key ^ ((u64::from(y) << 32) | u64::from(x)))
}

/// Uniform sample in the open interval (0, 1) from 32 bits.
#[inline]
fn unit_open(bits: u32) -> f64 {
    (f64::from(bits) + 0.5) * (1.0 / 4_294_967_296.0)
}

/// Standard normal sample for a pixel (Box–Muller, one output per hash).
#[inline]
pub fn gaussian(seed: u64, stream: u32, x: u32, y: u32) -> f32 {
    let h = hash_pixel(seed, stream, x, y);
    let u1 = unit_open((h >> 32) as u32);
    let u2 = unit_open(h as u32);
    ((-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_order_independent() {
        let forward: Vec<f32> = (0..64).map(|i| gaussian(42, 0, i, 7)).collect();
        let backward: Vec<f32> = (0..64).rev().map(|i| gaussian(42, 0, i, 7)).collect();
        assert!(forward.iter().eq(backward.iter().rev()));
    }

    #[test]
    fn seeds_streams_and_axes_are_independent() {
        let a = hash_pixel(1, 0, 3, 5);
        assert_ne!(a, hash_pixel(2, 0, 3, 5));
        assert_ne!(a, hash_pixel(1, 1, 3, 5));
        assert_ne!(a, hash_pixel(1, 0, 5, 3));
    }

    #[test]
    fn gaussian_moments() {
        let n = 1000u32;
        let (mut s1, mut s2, mut s4) = (0.0f64, 0.0f64, 0.0f64);
        for y in 0..n {
            for x in 0..n {
                let v = f64::from(gaussian(7, 0, x, y));
                s1 += v;
                s2 += v * v;
                s4 += v * v * v * v;
            }
        }
        let count = f64::from(n * n);
        let mean = s1 / count;
        let var = s2 / count - mean * mean;
        let kurtosis = s4 / count / (var * var);
        // Standard errors at 1e6 samples: mean ~1e-3, var ~1.4e-3, kurtosis ~5e-3.
        assert!(mean.abs() < 5e-3, "mean {mean}");
        assert!((var - 1.0).abs() < 7e-3, "var {var}");
        assert!((kurtosis - 3.0).abs() < 3e-2, "kurtosis {kurtosis}");
    }

    #[test]
    fn neighbouring_pixels_are_uncorrelated() {
        let n = 1000u32;
        let mut sxy = 0.0f64;
        for y in 0..n {
            for x in 0..n - 1 {
                sxy += f64::from(gaussian(9, 0, x, y)) * f64::from(gaussian(9, 0, x + 1, y));
            }
        }
        let corr = sxy / f64::from(n * (n - 1));
        assert!(corr.abs() < 5e-3, "lag-1 correlation {corr}");
    }
}
