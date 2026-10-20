//! Monotone curve representation for characteristic (D–log H) curves.
//!
//! Design 0001 §3.2: digitised `(log H, D)` points are interpolated with a monotone
//! piecewise-cubic Hermite spline using Steffen's slope rule, which cannot overshoot
//! between knots. Outside the knots the curve continues linearly with the end slopes.
//! For the per-pixel hot path, [`MonotoneCurve::to_lut`] samples the spline into a
//! uniform [`Lut1D`].
//!
//! Reference: M. Steffen, "A simple method for monotonic interpolation in one
//! dimension", Astron. Astrophys. 239, 443–450 (1990).

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum CurveError {
    #[error("a curve needs at least 2 points (got {0})")]
    TooFewPoints(usize),
    #[error("point {0} is not finite")]
    NonFinite(usize),
    #[error("x values must be strictly increasing (point {0})")]
    NotStrictlyIncreasing(usize),
    #[error("y values must be non-decreasing (point {0})")]
    NotMonotone(usize),
    #[error("invalid LUT specification: {0}")]
    InvalidLut(&'static str),
}

/// A non-decreasing curve through a set of knots.
#[derive(Clone, Debug, PartialEq)]
pub struct MonotoneCurve {
    xs: Vec<f64>,
    ys: Vec<f64>,
    /// Hermite slopes at each knot; all ≥ 0.
    ms: Vec<f64>,
}

impl MonotoneCurve {
    /// Build a curve from `(x, y)` points sorted by strictly increasing `x` with
    /// non-decreasing `y`. Unsorted or non-monotone data is rejected rather than
    /// repaired, because silently altering calibration data would hide errors.
    pub fn new(points: &[(f64, f64)]) -> Result<Self, CurveError> {
        let n = points.len();
        if n < 2 {
            return Err(CurveError::TooFewPoints(n));
        }
        for (i, &(x, y)) in points.iter().enumerate() {
            if !(x.is_finite() && y.is_finite()) {
                return Err(CurveError::NonFinite(i));
            }
            if i > 0 {
                let (px, py) = points[i - 1];
                if x <= px {
                    return Err(CurveError::NotStrictlyIncreasing(i));
                }
                if y < py {
                    return Err(CurveError::NotMonotone(i));
                }
            }
        }

        let xs: Vec<f64> = points.iter().map(|p| p.0).collect();
        let ys: Vec<f64> = points.iter().map(|p| p.1).collect();
        let ms = steffen_slopes(&xs, &ys);
        Ok(Self { xs, ys, ms })
    }

    /// The knot range `(x_min, x_max)`. Evaluation outside it is linear extrapolation.
    pub fn domain(&self) -> (f64, f64) {
        (self.xs[0], self.xs[self.xs.len() - 1])
    }

    /// Whether `x` lies within the knot range (i.e. is interpolated, not extrapolated).
    pub fn contains(&self, x: f64) -> bool {
        let (lo, hi) = self.domain();
        (lo..=hi).contains(&x)
    }

    pub fn eval(&self, x: f64) -> f64 {
        let last = self.xs.len() - 1;
        if x <= self.xs[0] {
            return self.ys[0] + self.ms[0] * (x - self.xs[0]);
        }
        if x >= self.xs[last] {
            return self.ys[last] + self.ms[last] * (x - self.xs[last]);
        }
        let i = self.segment(x);
        let h = self.xs[i + 1] - self.xs[i];
        let t = (x - self.xs[i]) / h;
        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        h00 * self.ys[i] + h10 * h * self.ms[i] + h01 * self.ys[i + 1] + h11 * h * self.ms[i + 1]
    }

    /// First derivative dy/dx (the local gradient, γ, of a characteristic curve).
    pub fn slope(&self, x: f64) -> f64 {
        let last = self.xs.len() - 1;
        if x <= self.xs[0] {
            return self.ms[0];
        }
        if x >= self.xs[last] {
            return self.ms[last];
        }
        let i = self.segment(x);
        let h = self.xs[i + 1] - self.xs[i];
        let t = (x - self.xs[i]) / h;
        let t2 = t * t;
        let d00 = 6.0 * t2 - 6.0 * t;
        let d10 = 3.0 * t2 - 4.0 * t + 1.0;
        let d01 = -6.0 * t2 + 6.0 * t;
        let d11 = 3.0 * t2 - 2.0 * t;
        (d00 * self.ys[i] + d10 * h * self.ms[i] + d01 * self.ys[i + 1] + d11 * h * self.ms[i + 1])
            / h
    }

    /// Sample the curve at `n` uniformly spaced points over `[lo, hi]`.
    pub fn to_lut(&self, lo: f64, hi: f64, n: usize) -> Result<Lut1D, CurveError> {
        Lut1D::from_fn(lo, hi, n, |x| self.eval(x))
    }

    /// Index `i` of the segment `[xs[i], xs[i+1])` containing interior `x`.
    fn segment(&self, x: f64) -> usize {
        self.xs.partition_point(|&k| k <= x) - 1
    }
}

/// Steffen (1990) slopes: monotone by construction, no overshoot.
fn steffen_slopes(xs: &[f64], ys: &[f64]) -> Vec<f64> {
    let n = xs.len();
    let h: Vec<f64> = xs.windows(2).map(|w| w[1] - w[0]).collect();
    let d: Vec<f64> = (0..n - 1).map(|i| (ys[i + 1] - ys[i]) / h[i]).collect();

    if n == 2 {
        return vec![d[0], d[0]];
    }

    let mut ms = vec![0.0; n];
    for i in 1..n - 1 {
        let p = (d[i - 1] * h[i] + d[i] * h[i - 1]) / (h[i - 1] + h[i]);
        ms[i] =
            (d[i - 1].signum() + d[i].signum()) * d[i - 1].abs().min(d[i].abs()).min(0.5 * p.abs());
    }
    ms[0] = steffen_end_slope(d[0], d[1], h[0], h[1]);
    ms[n - 1] = steffen_end_slope(d[n - 2], d[n - 3], h[n - 2], h[n - 3]);
    ms
}

/// One-sided end slope from Steffen (1990), eq. 26–27. `d0`/`h0` belong to the end
/// segment and `d1`/`h1` to its neighbour.
fn steffen_end_slope(d0: f64, d1: f64, h0: f64, h1: f64) -> f64 {
    let p = d0 * (1.0 + h0 / (h0 + h1)) - d1 * h0 / (h0 + h1);
    if p * d0 <= 0.0 {
        0.0
    } else if p.abs() > 2.0 * d0.abs() {
        2.0 * d0
    } else {
        p
    }
}

/// Uniformly sampled lookup table with linear interpolation.
///
/// Inputs outside `[lo, hi]` clamp to the end values. Callers choose a range that
/// covers every input the stage can produce (design 0001 uses `log H ∈ [−5, +2]`),
/// so clamping only guards against pathological values.
#[derive(Clone, Debug, PartialEq)]
pub struct Lut1D {
    lo: f32,
    hi: f32,
    inv_step: f32,
    values: Vec<f32>,
}

impl Lut1D {
    /// Sample an arbitrary function at `n` uniformly spaced points over `[lo, hi]`.
    pub fn from_fn(lo: f64, hi: f64, n: usize, f: impl Fn(f64) -> f64) -> Result<Self, CurveError> {
        if !(lo.is_finite() && hi.is_finite() && hi > lo) {
            return Err(CurveError::InvalidLut("range must be finite with hi > lo"));
        }
        if n < 2 {
            return Err(CurveError::InvalidLut("need at least 2 entries"));
        }
        let step = (hi - lo) / (n - 1) as f64;
        let values = (0..n).map(|i| f(lo + step * i as f64) as f32).collect();
        Ok(Self {
            lo: lo as f32,
            hi: hi as f32,
            inv_step: (1.0 / step) as f32,
            values,
        })
    }

    pub fn range(&self) -> (f32, f32) {
        (self.lo, self.hi)
    }

    #[inline]
    pub fn eval(&self, x: f32) -> f32 {
        let last = self.values.len() - 1;
        // NaN maps to the lower end rather than propagating into the image.
        let pos = ((x - self.lo) * self.inv_step).clamp(0.0, last as f32);
        let pos = if pos.is_nan() { 0.0 } else { pos };
        let i = (pos as usize).min(last - 1);
        let frac = pos - i as f32;
        self.values[i] + frac * (self.values[i + 1] - self.values[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// A plausible negative-film curve shape (toe, straight line, mild shoulder).
    /// Illustrative only; not calibration data.
    fn sample_curve() -> MonotoneCurve {
        MonotoneCurve::new(&[
            (-3.4, 0.00),
            (-3.0, 0.03),
            (-2.6, 0.12),
            (-2.2, 0.32),
            (-1.8, 0.56),
            (-1.0, 1.06),
            (-0.2, 1.52),
            (0.4, 1.78),
            (1.0, 1.90),
        ])
        .unwrap()
    }

    #[test]
    fn interpolates_knots_exactly() {
        let c = sample_curve();
        for (&x, &y) in c.xs.iter().zip(&c.ys) {
            assert!((c.eval(x) - y).abs() < 1e-12, "knot ({x}, {y})");
        }
    }

    #[test]
    fn extrapolation_is_continuous_and_linear() {
        let c = sample_curve();
        let (lo, hi) = c.domain();
        let eps = 1e-9;
        assert!((c.eval(lo - eps) - c.eval(lo + eps)).abs() < 1e-6);
        assert!((c.eval(hi - eps) - c.eval(hi + eps)).abs() < 1e-6);
        let s = c.slope(hi);
        assert!((c.eval(hi + 1.0) - c.eval(hi) - s).abs() < 1e-12);
        assert!(c.slope(lo - 5.0) >= 0.0);
        assert!(!c.contains(hi + 0.1) && c.contains(hi));
    }

    #[test]
    fn slope_matches_finite_difference() {
        let c = sample_curve();
        let mut x = -3.3;
        while x < 0.9 {
            let fd = (c.eval(x + 1e-6) - c.eval(x - 1e-6)) / 2e-6;
            assert!((c.slope(x) - fd).abs() < 1e-5, "x = {x}");
            x += 0.037;
        }
    }

    #[test]
    fn flat_regions_stay_flat() {
        // Steffen's rule must not overshoot a plateau.
        let c = MonotoneCurve::new(&[(0.0, 0.0), (1.0, 1.0), (2.0, 1.0), (3.0, 1.0), (4.0, 2.0)])
            .unwrap();
        for i in 0..=100 {
            let x = 1.0 + 2.0 * i as f64 / 100.0;
            assert!((c.eval(x) - 1.0).abs() < 1e-12, "x = {x}");
        }
    }

    #[test]
    fn two_points_are_a_line() {
        let c = MonotoneCurve::new(&[(0.0, 1.0), (2.0, 2.0)]).unwrap();
        assert!((c.eval(1.0) - 1.5).abs() < 1e-12);
        assert!((c.eval(4.0) - 3.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_invalid_points() {
        assert_eq!(
            MonotoneCurve::new(&[(0.0, 0.0)]),
            Err(CurveError::TooFewPoints(1))
        );
        assert_eq!(
            MonotoneCurve::new(&[(0.0, 0.0), (0.0, 1.0)]),
            Err(CurveError::NotStrictlyIncreasing(1))
        );
        assert_eq!(
            MonotoneCurve::new(&[(0.0, 1.0), (1.0, 0.5)]),
            Err(CurveError::NotMonotone(1))
        );
        assert_eq!(
            MonotoneCurve::new(&[(0.0, 0.0), (1.0, f64::NAN)]),
            Err(CurveError::NonFinite(1))
        );
    }

    #[test]
    fn lut_matches_spline_within_tolerance() {
        // Design 0001 §3.2: 4096 entries over log H ∈ [−5, +2], error < 1e-4 D.
        let c = sample_curve();
        let lut = c.to_lut(-5.0, 2.0, 4096).unwrap();
        let mut max_err: f64 = 0.0;
        for i in 0..=70_000 {
            let x = -5.0 + 7.0 * i as f64 / 70_000.0;
            max_err = max_err.max((lut.eval(x as f32) as f64 - c.eval(x)).abs());
        }
        assert!(max_err < 1e-4, "max LUT error {max_err}");
    }

    #[test]
    fn lut_clamps_out_of_range_and_nan() {
        let c = sample_curve();
        let lut = c.to_lut(-5.0, 2.0, 256).unwrap();
        assert_eq!(lut.eval(-100.0), lut.eval(-5.0));
        assert_eq!(lut.eval(100.0), lut.eval(2.0));
        assert_eq!(lut.eval(f32::NAN), lut.eval(-5.0));
    }

    fn monotone_points() -> impl Strategy<Value = Vec<(f64, f64)>> {
        prop::collection::vec((0.01f64..2.0, 0.0f64..2.0), 2..20).prop_map(|steps| {
            let (mut x, mut y) = (-4.0, 0.0);
            steps
                .into_iter()
                .map(|(dx, dy)| {
                    x += dx;
                    // Include occasional exact plateaus.
                    if dy > 0.3 {
                        y += dy;
                    }
                    (x, y)
                })
                .collect()
        })
    }

    proptest! {
        #[test]
        fn spline_is_monotone_and_bounded(points in monotone_points()) {
            let c = MonotoneCurve::new(&points).unwrap();
            let (lo, hi) = c.domain();
            let (ymin, ymax) = (points[0].1, points[points.len() - 1].1);
            let mut prev = f64::NEG_INFINITY;
            for i in 0..=2000 {
                let x = lo + (hi - lo) * i as f64 / 2000.0;
                let y = c.eval(x);
                prop_assert!(y >= prev - 1e-12, "decrease at x={x}");
                prop_assert!(y >= ymin - 1e-12 && y <= ymax + 1e-12, "overshoot at x={x}");
                prop_assert!(c.slope(x) >= -1e-12);
                prev = y;
            }
        }
    }
}
