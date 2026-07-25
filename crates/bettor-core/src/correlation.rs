//! Correlated parlays — a Gaussian copula over equicorrelated legs.
//!
//! Ported from `bettor-calculator-main/src/lib/math/parlayCorrelation.ts`.
//!
//! Multiplying leg probabilities assumes independence. Same-game legs are not
//! independent: if the quarterback throws for 350 yards, his receiver probably
//! went over too. A copula couples the legs through a shared latent normal so
//! the joint probability can move away from the product.
//!
//! # Divergences from the TypeScript
//!
//! 1. **No Monte Carlo when the answer is exact.** For non-negative
//!    equicorrelation the copula has a one-factor representation
//!
//!    ```text
//!    Xᵢ = √ρ·Y + √(1−ρ)·εᵢ
//!    P(all legs win) = ∫ φ(y) · Π Φ((tᵢ − √ρ·y) / √(1−ρ)) dy
//!    ```
//!
//!    which is a single smooth integral, evaluated here by Simpson's rule to
//!    around 1e-12. The TS drew 50,000 samples for a figure carrying roughly
//!    0.2% of sampling noise, and gave a different answer every time it ran.
//!    Negative correlation has no such factorisation, so it still simulates —
//!    but with a seed.
//! 2. **Clamping is reported.** An equicorrelated matrix is only positive
//!    definite for `ρ > −1/(n−1)`, so a five-leg parlay cannot be more
//!    negatively correlated than −0.25. The TS clamped silently, returning an
//!    answer for a correlation the caller never asked for.
//! 3. **Degenerate legs are rejected** rather than producing a zero
//!    probability and an infinite decimal price.

use crate::probability::{inverse_normal_cdf, normal_cdf};
use crate::{MathError, Result};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, StandardNormal};
use serde::Serialize;

/// Integration half-width in standard deviations. Beyond ±9 the normal density
/// contributes below 1e-18.
const INTEGRATION_LIMIT: f64 = 9.0;

/// Simpson intervals. Must be even.
const INTEGRATION_STEPS: usize = 4_000;

/// How a correlated parlay prices against its independent counterpart.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Correlated {
    /// Product of the leg probabilities — what a parlay calculator assumes.
    pub independent_prob: f64,
    /// Joint probability once the legs are coupled.
    pub correlated_prob: f64,
    /// Independent price, in decimal odds.
    pub independent_decimal: f64,
    /// Correlated price, in decimal odds.
    pub correlated_decimal: f64,
    /// Relative change in probability, as a fraction.
    ///
    /// Positive means the correlation helps: the parlay is likelier than the
    /// product of its legs.
    pub relative_change: f64,
    /// The correlation actually used, after any clamping.
    pub correlation_used: f64,
    /// True when the requested correlation was outside the range an
    /// equicorrelated matrix can represent for this many legs.
    pub correlation_was_clamped: bool,
    /// `None` when solved exactly; `Some(seed)` when simulated.
    ///
    /// Crosses the wire as a decimal string — see [`crate::seed_repr`].
    #[serde(serialize_with = "crate::seed_repr::serialize_option")]
    #[cfg_attr(feature = "specta", specta(type = Option<String>))]
    pub simulation_seed: Option<u64>,
}

impl Correlated {
    /// Whether the correlation makes the parlay more likely than independence.
    #[must_use]
    pub fn is_favourable(&self) -> bool {
        self.correlated_prob > self.independent_prob
    }
}

/// Smallest correlation an `n`-leg equicorrelated matrix can hold.
///
/// Below `−1/(n−1)` the matrix stops being positive definite: `n` variables
/// cannot all disagree with each other at once.
#[must_use]
pub fn min_correlation(legs: usize) -> f64 {
    if legs < 2 {
        return -1.0;
    }
    #[allow(clippy::cast_precision_loss, reason = "leg count is small")]
    let n = legs as f64;
    -1.0 / (n - 1.0)
}

fn validate(probs: &[f64]) -> Result<()> {
    if probs.len() < 2 {
        return Err(MathError::ShapeError {
            what: "legs",
            expected: "at least 2",
            got: probs.len(),
        });
    }
    for &p in probs {
        if !p.is_finite() || p <= 0.0 || p >= 1.0 {
            return Err(MathError::ProbabilityOutOfRange {
                value: p,
                reason: "every leg must be strictly between 0 and 1",
            });
        }
    }
    Ok(())
}

/// Exact joint probability for non-negative equicorrelation.
///
/// Conditional on the shared factor `y`, the legs are independent, so the
/// integrand is a product of marginal conditionals weighted by the normal
/// density. Simpson's rule over a smooth, rapidly-decaying integrand.
fn joint_prob_one_factor(thresholds: &[f64], rho: f64) -> f64 {
    let sqrt_rho = rho.sqrt();
    let sqrt_1m = (1.0 - rho).sqrt();

    let integrand = |y: f64| -> f64 {
        let density = (-0.5 * y * y).exp() / (2.0 * core::f64::consts::PI).sqrt();
        if density == 0.0 {
            return 0.0;
        }
        let conditional: f64 = thresholds
            .iter()
            .map(|t| normal_cdf((t - sqrt_rho * y) / sqrt_1m))
            .product();
        density * conditional
    };

    #[allow(clippy::cast_precision_loss, reason = "step count is 4000")]
    let steps = INTEGRATION_STEPS as f64;
    let h = 2.0 * INTEGRATION_LIMIT / steps;
    let mut total = integrand(-INTEGRATION_LIMIT) + integrand(INTEGRATION_LIMIT);
    for i in 1..INTEGRATION_STEPS {
        #[allow(clippy::cast_precision_loss, reason = "index below 4000")]
        let idx = i as f64;
        let y = -INTEGRATION_LIMIT + idx * h;
        total += integrand(y) * if i % 2 == 0 { 2.0 } else { 4.0 };
    }
    (total * h / 3.0).clamp(0.0, 1.0)
}

/// Lower-triangular Cholesky factor of an equicorrelated matrix.
fn cholesky_equicorrelated(n: usize, rho: f64) -> Vec<Vec<f64>> {
    let mut l = vec![vec![0.0_f64; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let entry = if i == j { 1.0 } else { rho };
            let sum: f64 = (0..j)
                .map(|k| {
                    let a = l.get(i).and_then(|r| r.get(k)).copied().unwrap_or(0.0);
                    let b = l.get(j).and_then(|r| r.get(k)).copied().unwrap_or(0.0);
                    a * b
                })
                .sum();
            let diag = l.get(j).and_then(|r| r.get(j)).copied().unwrap_or(0.0);
            let value = if i == j {
                (entry - sum).max(0.0).sqrt()
            } else if diag > 0.0 {
                (entry - sum) / diag
            } else {
                0.0
            };
            if let Some(slot) = l.get_mut(i).and_then(|r| r.get_mut(j)) {
                *slot = value;
            }
        }
    }
    l
}

/// Simulated joint probability, for correlations the one-factor form cannot express.
fn joint_prob_simulated(thresholds: &[f64], rho: f64, sims: usize, seed: u64) -> f64 {
    let n = thresholds.len();
    let l = cholesky_equicorrelated(n, rho);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut hits = 0_usize;

    for _ in 0..sims {
        let z: Vec<f64> = (0..n)
            .map(|_| StandardNormal.sample(&mut rng))
            .collect();
        let all_win = thresholds.iter().enumerate().all(|(i, t)| {
            let x: f64 = (0..=i)
                .map(|j| {
                    let lij = l.get(i).and_then(|r| r.get(j)).copied().unwrap_or(0.0);
                    lij * z.get(j).copied().unwrap_or(0.0)
                })
                .sum();
            x < *t
        });
        if all_win {
            hits += 1;
        }
    }

    #[allow(clippy::cast_precision_loss, reason = "counts for a proportion")]
    let p = hits as f64 / sims as f64;
    p
}

/// Prices a parlay whose legs move together.
///
/// `probs` are the individual leg probabilities. `correlation` is the pairwise
/// correlation shared by every pair; it is clamped into the range an
/// equicorrelated matrix can represent, and the clamping is reported.
///
/// `seed` is only consulted for negative correlation, where no exact
/// factorisation exists.
///
/// # Errors
///
/// [`MathError::ShapeError`] for fewer than two legs,
/// [`MathError::ProbabilityOutOfRange`] for a leg outside `(0, 1)`.
pub fn correlated_parlay(
    probs: &[f64],
    correlation: f64,
    seed: u64,
    sims: usize,
) -> Result<Correlated> {
    validate(probs)?;
    if !correlation.is_finite() {
        return Err(MathError::DomainError {
            param: "correlation",
            constraint: "finite",
            value: correlation,
        });
    }

    let independent_prob: f64 = probs.iter().product();
    // Leave headroom: exactly at the bound the matrix is singular.
    let floor = min_correlation(probs.len()) + 0.01;
    let rho = correlation.clamp(floor, 0.99);
    let clamped = (rho - correlation).abs() > 1e-12;

    let thresholds: Vec<f64> = probs.iter().map(|p| inverse_normal_cdf(*p)).collect();

    let (correlated_prob, simulation_seed) = if rho.abs() < 1e-9 {
        (independent_prob, None)
    } else if rho > 0.0 {
        (joint_prob_one_factor(&thresholds, rho), None)
    } else {
        (
            joint_prob_simulated(&thresholds, rho, sims.max(1), seed),
            Some(seed),
        )
    };

    Ok(Correlated {
        independent_prob,
        correlated_prob,
        independent_decimal: 1.0 / independent_prob,
        correlated_decimal: if correlated_prob > 0.0 {
            1.0 / correlated_prob
        } else {
            f64::INFINITY
        },
        relative_change: (correlated_prob - independent_prob) / independent_prob,
        correlation_used: rho,
        correlation_was_clamped: clamped,
        simulation_seed,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn run(probs: &[f64], rho: f64) -> Correlated {
        correlated_parlay(probs, rho, 42, 200_000).unwrap()
    }

    /// Closed form for two standard normals both below zero:
    /// `P = 1/4 + arcsin(ρ) / 2π`.
    fn bivariate_orthant(rho: f64) -> f64 {
        0.25 + rho.asin() / (2.0 * core::f64::consts::PI)
    }

    #[test]
    fn two_even_legs_match_the_closed_form() {
        // The strongest available check: an exact analytic answer.
        //
        // Tolerance is set by `normal_cdf`, not by the integration. Simpson
        // converges to about 1e-12 here, but every evaluation calls the
        // Abramowitz & Stegun approximation, whose truncated constants carry
        // roughly 1.5e-7 (see `probability`). The result cannot be more
        // accurate than the function it integrates.
        for rho in [0.1, 0.25, 0.5, 0.75, 0.9] {
            let c = run(&[0.5, 0.5], rho);
            assert_relative_eq!(c.correlated_prob, bivariate_orthant(rho), epsilon = 5e-7);
        }
    }

    #[test]
    fn negative_correlation_matches_the_closed_form_too() {
        // Simulated rather than integrated, so a looser tolerance.
        for rho in [-0.3, -0.6, -0.9] {
            let c = run(&[0.5, 0.5], rho);
            assert_relative_eq!(c.correlated_prob, bivariate_orthant(rho), epsilon = 3e-3);
            assert_eq!(c.simulation_seed, Some(42));
        }
    }

    #[test]
    fn zero_correlation_reproduces_independence() {
        let c = run(&[0.6, 0.55, 0.7], 0.0);
        assert_relative_eq!(c.correlated_prob, 0.6 * 0.55 * 0.7, epsilon = 1e-12);
        assert_relative_eq!(c.relative_change, 0.0, epsilon = 1e-12);
        assert!(c.simulation_seed.is_none());
    }

    #[test]
    fn positive_correlation_helps_a_parlay() {
        let c = run(&[0.6, 0.55, 0.7], 0.4);
        assert!(c.is_favourable());
        assert!(c.correlated_prob > c.independent_prob);
        assert!(c.correlated_decimal < c.independent_decimal, "a likelier parlay is a shorter price");
    }

    #[test]
    fn negative_correlation_hurts_a_parlay() {
        let c = run(&[0.6, 0.55], -0.5);
        assert!(!c.is_favourable());
        assert!(c.correlated_prob < c.independent_prob);
    }

    #[test]
    fn the_exact_path_is_deterministic() {
        // The TS returned a different number on every run.
        let a = run(&[0.6, 0.55, 0.7], 0.4);
        let b = run(&[0.6, 0.55, 0.7], 0.4);
        assert_eq!(a, b);
        assert!(a.simulation_seed.is_none(), "positive rho needs no simulation");
    }

    #[test]
    fn the_simulated_path_is_reproducible() {
        let a = run(&[0.6, 0.55], -0.4);
        let b = run(&[0.6, 0.55], -0.4);
        assert_eq!(a, b);
    }

    #[test]
    fn joint_probability_rises_monotonically_with_correlation() {
        let mut prev = 0.0;
        for rho in [0.0, 0.2, 0.4, 0.6, 0.8, 0.95] {
            let c = run(&[0.6, 0.55, 0.7], rho);
            assert!(c.correlated_prob >= prev, "fell at rho={rho}");
            prev = c.correlated_prob;
        }
    }

    #[test]
    fn perfect_correlation_approaches_the_weakest_leg() {
        // If the legs move as one, the parlay is only as hard as its hardest leg.
        let c = run(&[0.6, 0.55, 0.7], 0.99);
        assert!(c.correlated_prob > 0.5, "got {}", c.correlated_prob);
        assert!(c.correlated_prob < 0.55);
    }

    #[test]
    fn impossible_negative_correlation_is_clamped_and_reported() {
        // Five legs cannot be more negatively correlated than -0.25.
        let c = run(&[0.6; 5], -0.9);
        assert!(c.correlation_was_clamped);
        assert!(c.correlation_used > -0.25, "got {}", c.correlation_used);
        assert_relative_eq!(c.correlation_used, min_correlation(5) + 0.01, epsilon = 1e-12);
    }

    #[test]
    fn an_achievable_correlation_is_not_flagged() {
        let c = run(&[0.6, 0.55, 0.7], 0.4);
        assert!(!c.correlation_was_clamped);
        assert_relative_eq!(c.correlation_used, 0.4, epsilon = 1e-12);
    }

    #[test]
    fn min_correlation_matches_the_positive_definite_bound() {
        assert_relative_eq!(min_correlation(2), -1.0, epsilon = 1e-12);
        assert_relative_eq!(min_correlation(3), -0.5, epsilon = 1e-12);
        assert_relative_eq!(min_correlation(5), -0.25, epsilon = 1e-12);
    }

    #[test]
    fn degenerate_legs_are_rejected() {
        // TS produced a zero probability and an infinite decimal price.
        assert!(correlated_parlay(&[0.0, 0.5], 0.3, 1, 1_000).is_err());
        assert!(correlated_parlay(&[1.0, 0.5], 0.3, 1, 1_000).is_err());
        assert!(correlated_parlay(&[0.5], 0.3, 1, 1_000).is_err());
        assert!(correlated_parlay(&[0.5, 0.5], f64::NAN, 1, 1_000).is_err());
    }
}
