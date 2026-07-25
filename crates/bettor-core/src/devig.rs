//! Removing the bookmaker's margin from a set of implied probabilities.
//!
//! Ported from `bettor-calculator-main/src/lib/math/devig.ts`.
//!
//! Five methods disagree about *where* the margin sits. Proportional assumes
//! the book marks every outcome up by the same factor; equal-margin assumes a
//! flat additive charge; Shin assumes the book is defending against insiders
//! and so marks longshots up more. Which one is right is an empirical question
//! about a specific book, which is exactly why the app shows all five.
//!
//! # Divergences from the TypeScript
//!
//! 1. **Shin is actually Shin now.** The TS put `q/S` inside the radical where
//!    Shin (1993) has `q²/S`. With the exponent missing, the bisection has no
//!    interior root: `z` pinned to the search ceiling on every input and the
//!    method silently collapsed onto [`mpto`]. Two of the five "different"
//!    methods were returning the same numbers. On a -1000/+500 market the fix
//!    moves the longshot's fair probability from 0.1550 to 0.1288.
//! 2. **A book that sums below 1.0 is rejected.** It has no vig to remove — it
//!    is an arbitrage or a typo. The TS "devigged" `[0.30, 0.30]` to
//!    `[0.50, 0.50]`, reporting total confidence in a market it had been given
//!    almost no information about.
//! 3. **Single-outcome markets are rejected.** The TS returned `[1.0]`.
//! 4. **Non-convergence is reported.** The TS returned its last bisection
//!    iterate regardless of whether the solver had actually converged.

use crate::{MathError, Result};
use serde::Serialize;

/// Which assumption to make about where the margin sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DevigMethod {
    /// Equal margin — subtract the overround evenly across outcomes.
    Em,
    /// Margin proportional to odds — divide through by the book total.
    Mpto,
    /// Shin (1993) — insider-trading model, marks longshots up more.
    Shin,
    /// Odds ratio — find the exponent `c` where `Σ pᶜ = 1`.
    Or,
    /// Logarithmic — shift every outcome by a constant in log-odds space.
    Log,
}

impl DevigMethod {
    /// Every method, in display order.
    pub const ALL: [Self; 5] = [Self::Em, Self::Mpto, Self::Shin, Self::Or, Self::Log];

    /// Human-readable name for the UI.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Em => "Equal Margin",
            Self::Mpto => "Proportional",
            Self::Shin => "Shin",
            Self::Or => "Odds Ratio",
            Self::Log => "Logarithmic",
        }
    }

    /// Applies this method.
    ///
    /// # Errors
    ///
    /// See [`devig`].
    pub fn apply(self, probs: &[f64]) -> Result<Vec<f64>> {
        devig(probs, self)
    }
}

/// Iteration cap for the bisection solvers.
const MAX_ITERATIONS: u32 = 200;

/// Residual at which a solver is considered converged.
const TOLERANCE: f64 = 1e-12;

/// Validates a market and returns its total implied probability.
fn validate(probs: &[f64]) -> Result<f64> {
    if probs.len() < 2 {
        return Err(MathError::ShapeError {
            what: "outcomes",
            expected: "at least 2",
            got: probs.len(),
        });
    }
    for &p in probs {
        if !p.is_finite() || p <= 0.0 || p >= 1.0 {
            return Err(MathError::ProbabilityOutOfRange {
                value: p,
                reason: "every outcome must be strictly between 0 and 1",
            });
        }
    }
    let sum: f64 = probs.iter().sum();
    if sum < 1.0 {
        // Not a vigged book. Worth saying out loud rather than "devigging" it:
        // a two-way market summing below 1.0 is a live arbitrage.
        return Err(MathError::DomainError {
            param: "total implied probability",
            constraint: "at least 1.0 (below that there is no vig to remove — this is an arb)",
            value: sum,
        });
    }
    Ok(sum)
}

/// Removes the margin using the chosen method.
///
/// # Errors
///
/// [`MathError::ShapeError`] for fewer than two outcomes,
/// [`MathError::ProbabilityOutOfRange`] if any outcome is not in `(0, 1)`,
/// [`MathError::DomainError`] if the book sums below 1.0, and
/// [`MathError::NoConvergence`] if an iterative method fails to settle.
pub fn devig(probs: &[f64], method: DevigMethod) -> Result<Vec<f64>> {
    let sum = validate(probs)?;
    match method {
        DevigMethod::Em => em_inner(probs, sum),
        DevigMethod::Mpto => Ok(probs.iter().map(|p| p / sum).collect()),
        DevigMethod::Shin => shin_inner(probs, sum),
        DevigMethod::Or => or_inner(probs),
        DevigMethod::Log => log_inner(probs),
    }
}

/// Runs every method, pairing each with its result.
///
/// Methods that fail keep their error rather than being dropped, so the UI can
/// show *why* a column is empty.
#[must_use]
pub fn devig_all(probs: &[f64]) -> Vec<(DevigMethod, Result<Vec<f64>>)> {
    DevigMethod::ALL
        .into_iter()
        .map(|m| (m, devig(probs, m)))
        .collect()
}

fn em_inner(probs: &[f64], sum: f64) -> Result<Vec<f64>> {
    let n = probs.len();
    // `n` is at least 2 and bounded by the outcome count of a real market.
    #[allow(clippy::cast_precision_loss, reason = "outcome count is small")]
    let margin_share = (sum - 1.0) / n as f64;
    let fair: Vec<f64> = probs.iter().map(|p| p - margin_share).collect();
    if let Some(&bad) = fair.iter().find(|p| **p <= 0.0) {
        return Err(MathError::ProbabilityOutOfRange {
            value: bad,
            reason: "equal-margin removal drove an outcome to zero or below",
        });
    }
    Ok(fair)
}

/// Shin's fair probabilities for a given insider fraction `z`.
///
/// π_i = (√(z² + 4(1−z)·q_i²/S) − z) / (2(1−z))
fn shin_probs(probs: &[f64], sum: f64, z: f64) -> Vec<f64> {
    probs
        .iter()
        .map(|q| {
            let t = q * q / sum;
            let inner = z * z + 4.0 * (1.0 - z) * t;
            (inner.sqrt() - z) / (2.0 * (1.0 - z))
        })
        .collect()
}

fn shin_inner(probs: &[f64], sum: f64) -> Result<Vec<f64>> {
    // The sum is monotonically decreasing in z: it starts at √S ≥ 1 when there
    // are no insiders and falls to Σq²/S as z → 1. A fair book (S = 1) lands
    // exactly on z = 0, which the TS could not represent — its search floor was
    // 1e-4, so it reported a distortion for a market with no vig at all.
    let (mut lo, mut hi) = (0.0_f64, 1.0 - 1e-12);
    let mut residual = f64::INFINITY;
    let mut iterations = 0;

    for i in 0..MAX_ITERATIONS {
        iterations = i + 1;
        let mid = (lo + hi) / 2.0;
        let s: f64 = shin_probs(probs, sum, mid).iter().sum();
        residual = (s - 1.0).abs();
        // Bracket first, then test — see the note in `or_inner`.
        if s > 1.0 {
            lo = mid;
        } else {
            hi = mid;
        }
        if residual < TOLERANCE {
            break;
        }
    }

    let z = (lo + hi) / 2.0;
    let fair = shin_probs(probs, sum, z);
    let total: f64 = fair.iter().sum();
    if (total - 1.0).abs() > 1e-6 {
        return Err(MathError::NoConvergence {
            solver: "shin",
            iterations,
            residual,
        });
    }
    Ok(fair)
}

fn or_inner(probs: &[f64]) -> Result<Vec<f64>> {
    let power_sum = |c: f64| -> f64 { probs.iter().map(|p| p.powf(c)).sum() };

    // Σpᶜ decreases in c because every p is in (0, 1). c = 1 gives S ≥ 1, so
    // the root is at c ≥ 1; walk `hi` up until it brackets.
    let mut lo = 1.0_f64;
    let mut hi = 10.0_f64;
    while power_sum(hi) > 1.0 && hi < 1e6 {
        hi *= 2.0;
    }
    if power_sum(hi) > 1.0 {
        return Err(MathError::NoConvergence {
            solver: "odds-ratio",
            iterations: 0,
            residual: power_sum(hi) - 1.0,
        });
    }

    let mut residual = f64::INFINITY;
    let mut iterations = 0;
    for i in 0..MAX_ITERATIONS {
        iterations = i + 1;
        let mid = (lo + hi) / 2.0;
        let s = power_sum(mid);
        residual = (s - 1.0).abs();
        // Narrow the bracket before testing for convergence, so the final
        // midpoint uses the tighter interval. Ordering matters: testing first
        // leaves the bracket one step wider and shifts the last few ulps of
        // the result.
        if s > 1.0 {
            lo = mid;
        } else {
            hi = mid;
        }
        if residual < TOLERANCE {
            break;
        }
    }

    let c = (lo + hi) / 2.0;
    let fair: Vec<f64> = probs.iter().map(|p| p.powf(c)).collect();
    normalize(fair, "odds-ratio", iterations, residual)
}

fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

fn log_inner(probs: &[f64]) -> Result<Vec<f64>> {
    let log_odds: Vec<f64> = probs.iter().map(|p| (p / (1.0 - p)).ln()).collect();
    let shifted_sum = |k: f64| -> f64 { log_odds.iter().map(|l| logistic(l - k)).sum() };

    // Decreasing in k. Expand outward until the root is bracketed.
    let mut lo = -10.0_f64;
    let mut hi = 10.0_f64;
    while shifted_sum(lo) < 1.0 && lo > -700.0 {
        lo -= 10.0;
    }
    while shifted_sum(hi) > 1.0 && hi < 700.0 {
        hi += 10.0;
    }
    if shifted_sum(lo) < 1.0 || shifted_sum(hi) > 1.0 {
        return Err(MathError::NoConvergence {
            solver: "logarithmic",
            iterations: 0,
            residual: shifted_sum(lo) - 1.0,
        });
    }

    let mut residual = f64::INFINITY;
    let mut iterations = 0;
    for i in 0..MAX_ITERATIONS {
        iterations = i + 1;
        let mid = (lo + hi) / 2.0;
        let s = shifted_sum(mid);
        residual = (s - 1.0).abs();
        // Bracket first, then test — see the note in `or_inner`.
        if s > 1.0 {
            lo = mid;
        } else {
            hi = mid;
        }
        if residual < TOLERANCE {
            break;
        }
    }

    let k = (lo + hi) / 2.0;
    let fair: Vec<f64> = log_odds.iter().map(|l| logistic(l - k)).collect();
    normalize(fair, "logarithmic", iterations, residual)
}

/// Divides out any residual float drift so the result sums to exactly 1.
fn normalize(
    fair: Vec<f64>,
    solver: &'static str,
    iterations: u32,
    residual: f64,
) -> Result<Vec<f64>> {
    let total: f64 = fair.iter().sum();
    if !total.is_finite() || total <= 0.0 {
        return Err(MathError::NoConvergence {
            solver,
            iterations,
            residual,
        });
    }
    Ok(fair.into_iter().map(|p| p / total).collect())
}

/// Expected value of a bet, given a fair probability and the price taken.
///
/// `EV = fair / implied − 1`, expressed as a fraction of stake.
///
/// # Errors
///
/// [`MathError::ProbabilityOutOfRange`] if either probability is outside
/// `(0, 1]`.
pub fn ev_vs_fair(fair_prob: f64, bet_implied: f64) -> Result<f64> {
    if !bet_implied.is_finite() || bet_implied <= 0.0 || bet_implied > 1.0 {
        return Err(MathError::ProbabilityOutOfRange {
            value: bet_implied,
            reason: "the price you bet must imply a probability in (0, 1]",
        });
    }
    if !fair_prob.is_finite() || !(0.0..=1.0).contains(&fair_prob) {
        return Err(MathError::ProbabilityOutOfRange {
            value: fair_prob,
            reason: "fair probability must be in [0, 1]",
        });
    }
    Ok(fair_prob / bet_implied - 1.0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    const MARKET: [f64; 2] = [0.909_1, 0.166_7];

    #[test]
    fn every_method_returns_a_distribution() {
        for method in DevigMethod::ALL {
            let fair = method.apply(&MARKET).unwrap();
            let sum: f64 = fair.iter().sum();
            assert_relative_eq!(sum, 1.0, epsilon = 1e-9);
            assert!(fair.iter().all(|p| *p > 0.0 && *p < 1.0));
        }
    }

    #[test]
    fn shin_is_no_longer_a_duplicate_of_proportional() {
        // The bug this port fixes: these two returned the same numbers.
        let shin = DevigMethod::Shin.apply(&MARKET).unwrap();
        let mpto = DevigMethod::Mpto.apply(&MARKET).unwrap();
        assert!(
            (shin[1] - mpto[1]).abs() > 0.02,
            "Shin {shin:?} still collapses onto MPTO {mpto:?}"
        );
    }

    #[test]
    fn shin_matches_the_published_model() {
        // Independently computed from π_i = (√(z² + 4(1−z)q_i²/S) − z) / (2(1−z)).
        let fair = DevigMethod::Shin.apply(&MARKET).unwrap();
        assert_relative_eq!(fair[0], 0.871_2, epsilon = 1e-3);
        assert_relative_eq!(fair[1], 0.128_8, epsilon = 1e-3);
    }

    #[test]
    fn shin_leaves_a_fair_book_alone() {
        // The TS floored its search at z = 1e-4 and so reported a distortion
        // even for a market with no vig in it at all.
        let fair = DevigMethod::Shin.apply(&[0.5, 0.5]).unwrap();
        assert_relative_eq!(fair[0], 0.5, epsilon = 1e-9);
        assert_relative_eq!(fair[1], 0.5, epsilon = 1e-9);
    }

    #[test]
    fn shin_marks_longshots_down_relative_to_proportional() {
        // The entire point of the insider model.
        let shin = DevigMethod::Shin.apply(&MARKET).unwrap();
        let mpto = DevigMethod::Mpto.apply(&MARKET).unwrap();
        assert!(shin[1] < mpto[1], "shin {shin:?} vs mpto {mpto:?}");
        assert!(shin[0] > mpto[0]);
    }

    #[test]
    fn an_arb_is_rejected_rather_than_devigged() {
        // TS turned [0.30, 0.30] into a confident [0.50, 0.50].
        for method in DevigMethod::ALL {
            assert!(
                matches!(method.apply(&[0.30, 0.30]), Err(MathError::DomainError { .. })),
                "{method:?} accepted a book summing to 0.6"
            );
        }
    }

    #[test]
    fn single_outcome_markets_are_rejected() {
        for method in DevigMethod::ALL {
            assert!(matches!(
                method.apply(&[0.5]),
                Err(MathError::ShapeError { .. })
            ));
        }
    }

    #[test]
    fn certainties_are_rejected() {
        for method in DevigMethod::ALL {
            assert!(method.apply(&[1.0, 1.0]).is_err());
            assert!(method.apply(&[0.0, 0.5]).is_err());
        }
    }

    #[test]
    fn three_way_markets_work() {
        let market = [0.454_5, 0.312_5, 0.285_7];
        for method in DevigMethod::ALL {
            let fair = method.apply(&market).unwrap();
            assert_eq!(fair.len(), 3);
            assert_relative_eq!(fair.iter().sum::<f64>(), 1.0, epsilon = 1e-9);
        }
    }

    #[test]
    fn methods_agree_on_a_symmetric_market() {
        // With no asymmetry there is nothing for the models to disagree about.
        for method in DevigMethod::ALL {
            let fair = method.apply(&[0.523_8, 0.523_8]).unwrap();
            assert_relative_eq!(fair[0], 0.5, epsilon = 1e-6);
        }
    }

    #[test]
    fn ev_is_zero_at_the_fair_price() {
        assert_relative_eq!(ev_vs_fair(0.5, 0.5).unwrap(), 0.0, epsilon = 1e-15);
    }

    #[test]
    fn ev_matches_the_longshot_example() {
        // Betting +400 (0.20 implied) on a true 21% shot is a 5% edge.
        assert_relative_eq!(ev_vs_fair(0.21, 0.20).unwrap(), 0.05, epsilon = 1e-12);
    }
}
