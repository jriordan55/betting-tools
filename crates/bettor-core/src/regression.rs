//! Regression to the mean — shrinking a small-sample rate toward a baseline.
//!
//! Ported from `bettor-calculator-main/src/lib/math/regression.ts`.
//!
//! A hitter batting .400 over 40 plate appearances is not a .400 hitter. The
//! estimate of true talent is
//!
//! ```text
//! regressed = baseline + (observed − baseline) · n / (n + k)
//! ```
//!
//! where `k` is the sample size at which observed and baseline get equal
//! weight — a property of the statistic, not the player.
//!
//! # Divergences from the TypeScript
//!
//! 1. **Inputs are validated.** The TS returned a weight of zero for a
//!    negative sample size, which silently reported the baseline as if it were
//!    a finding.
//! 2. **The convergence series steps evenly.** The TS accumulated a
//!    floating-point step (`n += maxSample / 100`) and rounded each point, so
//!    whenever `maxSample / 100` was not a whole number the x-axis came out
//!    unevenly spaced: a max of 150 produced sample sizes 0, 2, 3, 5, 6, 8, 9,
//!    11 — gaps of 2 and 1 alternating. The step here is exact, so the spacing
//!    is uniform.
//!
//! # A caveat the model cannot express
//!
//! The confidence interval assumes a binomial statistic, `Var = p(1−p)/(n+k)`.
//! That holds for batting average, completion percentage, save percentage and
//! the like. It understates the spread of weighted rates such as wOBA, whose
//! outcomes are not Bernoulli. Read those intervals as optimistic.

use crate::{MathError, Result};
use serde::Serialize;

/// z-score for a 90% interval.
const Z_90: f64 = 1.645;

/// Weight placed on the observed sample, 0–1.
///
/// Equals 0.5 exactly when `sample_size == regression_constant`.
///
/// # Errors
///
/// [`MathError::DomainError`] for a negative sample size or a non-positive
/// regression constant.
pub fn regression_weight(sample_size: f64, regression_constant: f64) -> Result<f64> {
    if !sample_size.is_finite() || sample_size < 0.0 {
        return Err(MathError::DomainError {
            param: "sample_size",
            constraint: "non-negative",
            value: sample_size,
        });
    }
    if !regression_constant.is_finite() || regression_constant <= 0.0 {
        return Err(MathError::DomainError {
            param: "regression_constant",
            constraint: "greater than zero",
            value: regression_constant,
        });
    }
    Ok(sample_size / (sample_size + regression_constant))
}

/// Shrinks an observed rate toward the population baseline.
///
/// # Errors
///
/// See [`regression_weight`].
pub fn regress_to_mean(
    observed: f64,
    baseline: f64,
    sample_size: f64,
    regression_constant: f64,
) -> Result<f64> {
    let weight = regression_weight(sample_size, regression_constant)?;
    Ok(baseline + (observed - baseline) * weight)
}

/// A regressed estimate with its uncertainty.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Regressed {
    /// Shrinkage-adjusted estimate of true talent.
    pub estimate: f64,
    /// Weight the observed sample earned, 0–1.
    pub weight: f64,
    /// Lower bound of the 90% interval.
    pub lower: f64,
    /// Upper bound of the 90% interval.
    pub upper: f64,
}

/// Regresses a rate and attaches a 90% interval.
///
/// # Errors
///
/// See [`regression_weight`]; also [`MathError::ProbabilityOutOfRange`] if
/// `observed` or `baseline` is outside `[0, 1]`.
pub fn regress_with_interval(
    observed: f64,
    baseline: f64,
    sample_size: f64,
    regression_constant: f64,
) -> Result<Regressed> {
    for value in [observed, baseline] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(MathError::ProbabilityOutOfRange {
                value,
                reason: "rates must be between 0 and 1",
            });
        }
    }
    let weight = regression_weight(sample_size, regression_constant)?;
    let estimate = baseline + (observed - baseline) * weight;

    // Clamped so a rate pinned at 0 or 1 still has some width.
    let p = estimate.clamp(0.001, 0.999);
    let se = (p * (1.0 - p) / (sample_size + regression_constant)).sqrt();

    Ok(Regressed {
        estimate,
        weight,
        lower: (estimate - Z_90 * se).max(0.0),
        upper: (estimate + Z_90 * se).min(1.0),
    })
}

/// One point on a convergence curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvergencePoint {
    /// Sample size at this point.
    pub sample_size: f64,
    /// Regressed estimate at that sample size.
    pub regressed: f64,
}

/// Traces how the estimate approaches the observed rate as the sample grows.
///
/// # Errors
///
/// See [`regression_weight`]; also [`MathError::DomainError`] for a
/// non-positive `max_sample`.
pub fn convergence_series(
    observed: f64,
    baseline: f64,
    regression_constant: f64,
    max_sample: f64,
    steps: usize,
) -> Result<Vec<ConvergencePoint>> {
    if !max_sample.is_finite() || max_sample <= 0.0 {
        return Err(MathError::DomainError {
            param: "max_sample",
            constraint: "greater than zero",
            value: max_sample,
        });
    }
    let steps = steps.clamp(1, 10_000);
    (0..=steps)
        .map(|i| {
            #[allow(clippy::cast_precision_loss, reason = "step index bounded at 10000")]
            let frac = i as f64 / steps as f64;
            let sample_size = frac * max_sample;
            Ok(ConvergencePoint {
                sample_size,
                regressed: regress_to_mean(observed, baseline, sample_size, regression_constant)?,
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn weight_is_one_half_at_the_regression_constant() {
        assert_relative_eq!(regression_weight(200.0, 200.0).unwrap(), 0.5, epsilon = 1e-12);
    }

    #[test]
    fn no_sample_means_no_information() {
        let r = regress_to_mean(0.400, 0.260, 0.0, 200.0).unwrap();
        assert_relative_eq!(r, 0.260, epsilon = 1e-12);
    }

    #[test]
    fn a_hot_start_regresses_hard() {
        // .400 over 40 PA against a .260 league, k = 200.
        let r = regress_to_mean(0.400, 0.260, 40.0, 200.0).unwrap();
        assert_relative_eq!(r, 0.260 + 0.140 * (40.0 / 240.0), epsilon = 1e-12);
        assert!(r < 0.290, "got {r}");
    }

    #[test]
    fn a_large_sample_barely_regresses() {
        let r = regress_to_mean(0.400, 0.260, 20_000.0, 200.0).unwrap();
        assert!(r > 0.398, "got {r}");
    }

    #[test]
    fn the_estimate_never_leaves_the_interval_between_observed_and_baseline() {
        for n in [1.0, 10.0, 100.0, 1_000.0] {
            let r = regress_to_mean(0.400, 0.260, n, 200.0).unwrap();
            assert!((0.260..=0.400).contains(&r), "n={n} gave {r}");
        }
    }

    #[test]
    fn a_cold_start_regresses_upward() {
        let r = regress_to_mean(0.150, 0.260, 40.0, 200.0).unwrap();
        assert!(r > 0.150 && r < 0.260);
    }

    #[test]
    fn intervals_tighten_as_the_sample_grows() {
        let small = regress_with_interval(0.400, 0.260, 40.0, 200.0).unwrap();
        let large = regress_with_interval(0.400, 0.260, 4_000.0, 200.0).unwrap();
        assert!((large.upper - large.lower) < (small.upper - small.lower));
        assert!(small.lower <= small.estimate && small.estimate <= small.upper);
    }

    #[test]
    fn intervals_stay_inside_zero_and_one() {
        let r = regress_with_interval(1.0, 0.99, 1.0, 2.0).unwrap();
        assert!(r.lower >= 0.0 && r.upper <= 1.0);
    }

    #[test]
    fn convergence_steps_evenly() {
        // The TS accumulated a float step and rounded, so the spacing came out
        // uneven whenever max/steps was not a whole number.
        let series = convergence_series(0.400, 0.260, 200.0, 150.0, 100).unwrap();
        assert_eq!(series.len(), 101);
        assert_relative_eq!(series[0].sample_size, 0.0, epsilon = 1e-12);
        assert_relative_eq!(series[100].sample_size, 150.0, epsilon = 1e-12);
        // Uniform spacing is the actual fix: every gap identical.
        let gap = series[1].sample_size - series[0].sample_size;
        for w in series.windows(2) {
            assert_relative_eq!(w[1].sample_size - w[0].sample_size, gap, epsilon = 1e-9);
        }
    }

    #[test]
    fn convergence_is_monotonic_toward_the_observed_rate() {
        let series = convergence_series(0.400, 0.260, 200.0, 2_000.0, 100).unwrap();
        for w in series.windows(2) {
            assert!(w[1].regressed >= w[0].regressed - 1e-12);
        }
        assert!(series.last().unwrap().regressed > 0.38);
    }

    #[test]
    fn invalid_input_is_rejected() {
        assert!(regression_weight(-1.0, 200.0).is_err());
        assert!(regression_weight(100.0, 0.0).is_err());
        assert!(regress_with_interval(1.5, 0.26, 40.0, 200.0).is_err());
        assert!(convergence_series(0.4, 0.26, 200.0, 0.0, 100).is_err());
    }
}
