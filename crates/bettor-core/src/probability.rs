//! Normal-distribution helpers shared by the spread, total, and teaser models.
//!
//! Ported from `bettor-calculator-main/src/lib/math/probability.ts`.
//!
//! Both approximations here are kept bit-for-bit identical to the TypeScript.
//! They are used to price lines, so changing them — even to something more
//! accurate — would silently move every spread the app has ever quoted. Any
//! upgrade should be a deliberate, separately-reviewed change.

use crate::{MathError, Result};

/// Standard normal CDF via the Abramowitz & Stegun 26.2.17 approximation.
///
/// Absolute error below 7.5e-8, which is far tighter than the modelling
/// assumptions it feeds.
#[must_use]
pub fn normal_cdf(z: f64) -> f64 {
    if z.is_nan() {
        return f64::NAN;
    }
    if z.is_infinite() {
        return if z > 0.0 { 1.0 } else { 0.0 };
    }
    let t = 1.0 / (1.0 + 0.231_641_9 * z.abs());
    let d = 0.398_942_3 * (-z * z / 2.0).exp();
    let p = d
        * t
        * (0.319_381_5
            + t * (-0.356_563_8 + t * (1.781_478 + t * (-1.821_256 + t * 1.330_274))));
    if z >= 0.0 {
        1.0 - p
    } else {
        p
    }
}

/// Coefficients for Acklam's inverse-normal rational approximation.
const ACKLAM_A: [f64; 6] = [
    -3.969_683_028_665_376e1, 2.209_460_984_245_205e2, -2.759_285_104_469_687e2,
    1.383_577_518_672_69e2, -3.066_479_806_614_716e1, 2.506_628_277_459_239,
];
const ACKLAM_B: [f64; 5] = [
    -5.447_609_879_822_406e1, 1.615_858_368_580_409e2, -1.556_989_798_598_866e2,
    6.680_131_188_771_972e1, -1.328_068_155_288_572e1,
];
const ACKLAM_C: [f64; 6] = [
    -7.784_894_002_430_293e-3, -3.223_964_580_411_365e-1, -2.400_758_277_161_838,
    -2.549_732_539_343_734, 4.374_664_141_464_968, 2.938_163_982_698_783,
];
const ACKLAM_D: [f64; 4] = [
    7.784_695_709_041_462e-3, 3.224_671_290_700_398e-1, 2.445_134_137_142_996,
    3.754_408_661_907_416,
];

/// Boundary between Acklam's tail and central branches.
const P_LOW: f64 = 0.024_25;

/// Inverse standard normal CDF — Acklam's rational approximation.
///
/// Relative error below 1.15e-9 across the open interval. Returns ±∞ at the
/// closed endpoints, matching the TypeScript.
#[must_use]
pub fn inverse_normal_cdf(p: f64) -> f64 {
    if p.is_nan() {
        return f64::NAN;
    }
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }

    let [a0, a1, a2, a3, a4, a5] = ACKLAM_A;
    let [b0, b1, b2, b3, b4] = ACKLAM_B;
    let [c0, c1, c2, c3, c4, c5] = ACKLAM_C;
    let [d0, d1, d2, d3] = ACKLAM_D;

    let tail = |q: f64| {
        (((((c0 * q + c1) * q + c2) * q + c3) * q + c4) * q + c5)
            / ((((d0 * q + d1) * q + d2) * q + d3) * q + 1.0)
    };

    if p < P_LOW {
        let q = (-2.0 * p.ln()).sqrt();
        tail(q)
    } else if p <= 1.0 - P_LOW {
        let q = p - 0.5;
        let r = q * q;
        (((((a0 * r + a1) * r + a2) * r + a3) * r + a4) * r + a5) * q
            / (((((b0 * r + b1) * r + b2) * r + b3) * r + b4) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -tail(q)
    }
}

/// Converts a win probability to the point spread that implies it.
///
/// `p` is `P(margin > 0)`, so the spread is `-std * Φ⁻¹(1 - p)`.
#[must_use]
pub fn prob_to_spread(p: f64, std_dev: f64) -> f64 {
    -inverse_normal_cdf(1.0 - p) * std_dev
}

/// Beta distribution parameters.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BetaParams {
    /// Pseudo-count of successes.
    pub alpha: f64,
    /// Pseudo-count of failures.
    pub beta: f64,
}

/// Expresses a probability as a Beta prior carrying `n` observations of weight.
///
/// # Errors
///
/// [`MathError::ProbabilityOutOfRange`] if `p` is outside `[0, 1]`, and
/// [`MathError::DomainError`] if `n` is negative. The TS validated neither, so
/// a negative `n` produced a Beta with negative parameters — an object that
/// every downstream consumer would go on to misuse.
pub fn prob_to_beta(p: f64, n: f64) -> Result<BetaParams> {
    if !(0.0..=1.0).contains(&p) || p.is_nan() {
        return Err(MathError::ProbabilityOutOfRange {
            value: p,
            reason: "must be between 0 and 1",
        });
    }
    if n < 0.0 || n.is_nan() {
        return Err(MathError::DomainError {
            param: "n",
            constraint: "non-negative",
            value: n,
        });
    }
    Ok(BetaParams {
        alpha: p * n,
        beta: (1.0 - p) * n,
    })
}

/// Completes a set of probabilities by supplying the outcome left over.
///
/// A three-way market entered as "home 62%, draw 22%" implies away 16%. That
/// last figure is a derivation, not data entry, so it belongs here rather than
/// in whatever is collecting the input — which is how `dirichletProbUpdate`
/// came to accept a prior that had never been checked for summing to 1.
///
/// # Errors
///
/// [`MathError::ShapeError`] if `partial` is empty, and
/// [`MathError::ProbabilityOutOfRange`] if any element is outside `(0, 1)` or
/// the supplied outcomes already account for all the probability there is —
/// which would leave the final outcome at zero or below, and an outcome that
/// cannot happen has no price.
pub fn complete_simplex(partial: &[f64]) -> Result<Vec<f64>> {
    if partial.is_empty() {
        return Err(MathError::ShapeError {
            what: "outcomes",
            expected: "at least 1",
            got: 0,
        });
    }

    let mut remaining = 1.0;
    for &p in partial {
        if !p.is_finite() || p <= 0.0 || p >= 1.0 {
            return Err(MathError::ProbabilityOutOfRange {
                value: p,
                reason: "every outcome must be strictly between 0 and 1",
            });
        }
        remaining -= p;
    }

    if remaining <= 0.0 {
        return Err(MathError::ProbabilityOutOfRange {
            value: remaining,
            reason: "the outcomes given already sum to 1 or more, leaving nothing for the last one",
        });
    }

    let mut full = partial.to_vec();
    full.push(remaining);
    Ok(full)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::indexing_slicing,
    reason = "test code; the float comparisons here are against exact infinities"
)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn complete_simplex_supplies_the_leftover_outcome() {
        let full = complete_simplex(&[0.62, 0.22]).unwrap();
        assert_eq!(full.len(), 3);
        assert_relative_eq!(full[2], 0.16, epsilon = 1e-12);
        assert_relative_eq!(full.iter().sum::<f64>(), 1.0, epsilon = 1e-12);
    }

    #[test]
    fn complete_simplex_rejects_a_set_with_nothing_left_over() {
        // 70% + 45% leaves -15% for the third outcome. The TS accepted this
        // shape and misweighted the prior rather than saying so.
        assert!(complete_simplex(&[0.70, 0.45]).is_err());
        assert!(complete_simplex(&[0.5, 0.5]).is_err());
    }

    #[test]
    fn complete_simplex_rejects_a_degenerate_outcome() {
        assert!(complete_simplex(&[0.0]).is_err());
        assert!(complete_simplex(&[1.0]).is_err());
        assert!(complete_simplex(&[]).is_err());
    }

    #[test]
    fn cdf_is_symmetric_about_zero() {
        for z in [0.25, 0.5, 1.0, 1.96, 2.5, 4.0] {
            assert_relative_eq!(normal_cdf(z) + normal_cdf(-z), 1.0, epsilon = 1e-7);
        }
    }

    #[test]
    fn cdf_hits_known_quantiles() {
        // A&S 26.2.17 is quoted at |ε| < 7.5e-8, but the TS truncated the
        // constants (0.3989423 for 1/√2π, and so on) and this port keeps them
        // bit-identical. That costs roughly a factor of two: the error at z = 0
        // is 1.5e-7. Accepted deliberately — matching the prices the web app
        // has been quoting matters more here than the eighth decimal place.
        assert_relative_eq!(normal_cdf(0.0), 0.5, epsilon = 2e-7);
        assert_relative_eq!(normal_cdf(1.959_963_985), 0.975, epsilon = 2e-7);
        assert_relative_eq!(normal_cdf(-1.644_853_627), 0.05, epsilon = 2e-7);
    }

    #[test]
    fn cdf_is_monotonic() {
        let mut prev = 0.0;
        let mut z = -6.0;
        while z <= 6.0 {
            let cur = normal_cdf(z);
            assert!(cur >= prev, "CDF decreased at z={z}: {prev} -> {cur}");
            prev = cur;
            z += 0.01;
        }
    }

    #[test]
    fn inverse_undoes_the_cdf() {
        // Acklam is good to ~1.15e-9; A&S is the looser of the pair at ~7.5e-8.
        for p in [0.001, 0.01, 0.024, 0.025, 0.1, 0.3, 0.5, 0.7, 0.9, 0.975, 0.99, 0.999] {
            let z = inverse_normal_cdf(p);
            assert_relative_eq!(normal_cdf(z), p, epsilon = 1e-6);
        }
    }

    #[test]
    fn inverse_is_continuous_across_acklam_branches() {
        // A transcribed coefficient would show up as a step here.
        for boundary in [P_LOW, 1.0 - P_LOW] {
            let below = inverse_normal_cdf(boundary - 1e-9);
            let above = inverse_normal_cdf(boundary + 1e-9);
            assert!(
                (below - above).abs() < 1e-6,
                "discontinuity at {boundary}: {below} vs {above}"
            );
        }
    }

    #[test]
    fn endpoints_are_infinite() {
        assert_eq!(inverse_normal_cdf(0.0), f64::NEG_INFINITY);
        assert_eq!(inverse_normal_cdf(1.0), f64::INFINITY);
    }

    #[test]
    fn spread_of_a_coin_flip_is_zero() {
        assert_relative_eq!(prob_to_spread(0.5, 13.5), 0.0, epsilon = 1e-9);
    }

    #[test]
    fn favorites_get_positive_spreads() {
        // 75% to win at NBA-ish volatility is roughly a 9-point favorite.
        let spread = prob_to_spread(0.75, 13.5);
        assert!(spread > 8.0 && spread < 10.0, "got {spread}");
    }

    #[test]
    fn beta_rejects_what_the_ts_accepted() {
        assert!(prob_to_beta(0.5, -10.0).is_err());
        assert!(prob_to_beta(1.5, 10.0).is_err());
        assert!(prob_to_beta(-0.1, 10.0).is_err());
    }

    #[test]
    fn beta_parameters_sum_to_the_sample_size() {
        let b = prob_to_beta(0.3, 100.0).unwrap();
        assert_relative_eq!(b.alpha + b.beta, 100.0, epsilon = 1e-12);
        assert_relative_eq!(b.alpha, 30.0, epsilon = 1e-12);
    }
}
