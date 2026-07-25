//! Combining a market price with your own model.
//!
//! Ported from `bettor-calculator-main/src/lib/math/bayesian.ts`.
//!
//! The market is the prior — it aggregates far more information than any one
//! model — and your model is the evidence. How much the posterior moves
//! depends on how many notional observations each side is given. Setting
//! `market_n` high and `model_n` low says the market is hard to move, which is
//! usually the honest setting.
//!
//! # Divergences from the TypeScript
//!
//! 1. **`removeVig` and `removeVig3Way` are gone.** Both were proportional
//!    devigging, which already existed as `devigMPTO` — a third and fourth
//!    implementation of the same three lines. Callers use
//!    [`crate::devig::devig`] with [`crate::devig::DevigMethod::Mpto`], which
//!    also gets them the other four methods for free.
//! 2. **Fair odds no longer run to infinity.** `calculateMLEdge` computed its
//!    own American conversion without the clamp that `impliedToAmerican` has,
//!    so a posterior of 0 or 1 produced `Infinity`. It now calls the shared
//!    [`crate::odds::implied_to_american`].
//! 3. **A zero standard deviation is an error, not a clamp.**
//!    `bayesianSpreadUpdate` silently raised it to 0.001, which is a precision
//!    of a million — that source wins outright and the other is discarded
//!    without a word.
//! 4. **Inputs are validated.** A negative `marketN` produced negative Beta
//!    parameters and a posterior probability outside `[0, 1]`.
//! 5. **The Dirichlet update takes any number of outcomes**, not exactly three.

use crate::odds::implied_to_american;
use crate::{MathError, Result};
use serde::Serialize;

/// A Beta posterior over a two-way market.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct BetaPosterior {
    /// Prior successes, from the market.
    pub prior_alpha: f64,
    /// Prior failures, from the market.
    pub prior_beta: f64,
    /// Posterior successes, after folding in the model.
    pub posterior_alpha: f64,
    /// Posterior failures.
    pub posterior_beta: f64,
    /// Posterior probability of the first side, 0–1.
    pub posterior_prob: f64,
    /// How far the posterior moved off the market, in probability points.
    ///
    /// This is the number to sanity-check: a model that drags a liquid market
    /// several points is more likely miscalibrated than right.
    pub shift_from_market: f64,
}

fn check_prob(value: f64, what: &'static str) -> Result<()> {
    if !value.is_finite() || value <= 0.0 || value >= 1.0 {
        return Err(MathError::ProbabilityOutOfRange {
            value,
            reason: what,
        });
    }
    Ok(())
}

fn check_weight(value: f64, param: &'static str) -> Result<()> {
    if !value.is_finite() || value < 0.0 {
        return Err(MathError::DomainError {
            param,
            constraint: "non-negative",
            value,
        });
    }
    Ok(())
}

/// Folds a model probability into a market probability, Beta-binomial style.
///
/// `market_n` and `model_n` are notional sample sizes — how many observations
/// each source is worth. Their ratio is the only thing that matters.
///
/// # Errors
///
/// [`MathError::ProbabilityOutOfRange`] if either probability is outside
/// `(0, 1)`, [`MathError::DomainError`] for a negative weight or two zero
/// weights.
pub fn beta_update(
    market_prob: f64,
    model_prob: f64,
    market_n: f64,
    model_n: f64,
) -> Result<BetaPosterior> {
    check_prob(market_prob, "market probability must be between 0 and 1")?;
    check_prob(model_prob, "model probability must be between 0 and 1")?;
    check_weight(market_n, "market_n")?;
    check_weight(model_n, "model_n")?;
    if market_n + model_n <= 0.0 {
        return Err(MathError::DomainError {
            param: "market_n + model_n",
            constraint: "greater than zero (with no weight on either source there is nothing to update)",
            value: 0.0,
        });
    }

    let prior_alpha = market_prob * market_n;
    let prior_beta = (1.0 - market_prob) * market_n;
    let posterior_alpha = prior_alpha + model_prob * model_n;
    let posterior_beta = prior_beta + (1.0 - model_prob) * model_n;
    let posterior_prob = posterior_alpha / (posterior_alpha + posterior_beta);

    Ok(BetaPosterior {
        prior_alpha,
        prior_beta,
        posterior_alpha,
        posterior_beta,
        posterior_prob,
        shift_from_market: posterior_prob - market_prob,
    })
}

/// A Dirichlet posterior over a market with any number of outcomes.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct DirichletPosterior {
    /// Prior concentration per outcome, from the market.
    pub prior_alphas: Vec<f64>,
    /// Posterior concentration per outcome.
    pub posterior_alphas: Vec<f64>,
    /// Posterior probabilities, summing to 1.
    pub posterior_probs: Vec<f64>,
    /// Largest move off the market across outcomes, in probability points.
    pub max_shift_from_market: f64,
}

/// Folds a model into a market across three or more outcomes.
///
/// The TypeScript fixed this at exactly three (home/draw/away); nothing in the
/// mathematics requires that.
///
/// # Errors
///
/// [`MathError::ShapeError`] for mismatched lengths or fewer than two
/// outcomes, [`MathError::ProbabilityOutOfRange`] if either set does not sum
/// to 1, [`MathError::DomainError`] for a negative weight.
pub fn dirichlet_update(
    market_probs: &[f64],
    model_probs: &[f64],
    market_n: f64,
    model_n: f64,
) -> Result<DirichletPosterior> {
    if market_probs.len() < 2 || market_probs.len() != model_probs.len() {
        return Err(MathError::ShapeError {
            what: "outcomes",
            expected: "at least 2, matching between market and model",
            got: market_probs.len().min(model_probs.len()),
        });
    }
    check_weight(market_n, "market_n")?;
    check_weight(model_n, "model_n")?;

    for set in [market_probs, model_probs] {
        for &p in set {
            check_prob(p, "every outcome must be between 0 and 1")?;
        }
        let sum: f64 = set.iter().sum();
        if (sum - 1.0).abs() > 1e-6 {
            // The TS never checked, so a set that had not been devigged was
            // treated as a distribution and silently misweighted the prior.
            return Err(MathError::ProbabilityOutOfRange {
                value: sum,
                reason: "probabilities must sum to 1 — devig the market first",
            });
        }
    }

    let prior_alphas: Vec<f64> = market_probs.iter().map(|p| p * market_n).collect();
    let posterior_alphas: Vec<f64> = prior_alphas
        .iter()
        .zip(model_probs)
        .map(|(a, m)| a + m * model_n)
        .collect();
    let total: f64 = posterior_alphas.iter().sum();
    if total <= 0.0 {
        return Err(MathError::DomainError {
            param: "market_n + model_n",
            constraint: "greater than zero",
            value: total,
        });
    }
    let posterior_probs: Vec<f64> = posterior_alphas.iter().map(|a| a / total).collect();
    let max_shift_from_market = posterior_probs
        .iter()
        .zip(market_probs)
        .map(|(post, mkt)| (post - mkt).abs())
        .fold(0.0_f64, f64::max);

    Ok(DirichletPosterior {
        prior_alphas,
        posterior_alphas,
        posterior_probs,
        max_shift_from_market,
    })
}

/// A precision-weighted combination of two margin estimates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MarginPosterior {
    /// Posterior expected margin.
    pub margin: f64,
    /// The same figure as a spread, which is its negation.
    pub spread: f64,
    /// Posterior standard deviation. Always below both inputs — combining two
    /// estimates cannot make you less certain.
    pub std_dev: f64,
    /// Share of the posterior mean contributed by the market, 0–1.
    pub market_weight: f64,
}

/// Combines a market margin and a model margin by precision.
///
/// # Errors
///
/// [`MathError::DomainError`] for a non-positive standard deviation. The TS
/// clamped these to 0.001, which silently handed that source a precision of a
/// million and discarded the other.
pub fn margin_update(
    market_margin: f64,
    model_margin: f64,
    market_std: f64,
    model_std: f64,
) -> Result<MarginPosterior> {
    for (param, value) in [("market_std", market_std), ("model_std", model_std)] {
        if !value.is_finite() || value <= 0.0 {
            return Err(MathError::DomainError {
                param,
                constraint: "greater than zero",
                value,
            });
        }
    }
    for (param, value) in [("market_margin", market_margin), ("model_margin", model_margin)] {
        if !value.is_finite() {
            return Err(MathError::DomainError {
                param,
                constraint: "finite",
                value,
            });
        }
    }

    let market_precision = 1.0 / (market_std * market_std);
    let model_precision = 1.0 / (model_std * model_std);
    let total_precision = market_precision + model_precision;
    let margin = (market_margin * market_precision + model_margin * model_precision) / total_precision;

    Ok(MarginPosterior {
        margin,
        spread: -margin,
        std_dev: (1.0 / total_precision).sqrt(),
        market_weight: market_precision / total_precision,
    })
}

/// Which direction an outcome has to go for a bet to win.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Wins when the result comes in above the cover point.
    Above,
    /// Wins when it comes in below.
    Below,
}

/// Probability of finishing on the right side of a number.
///
/// `cover_point` is the value that must be beaten, already on the same axis as
/// `expected` — a favorite laying 3.5 has a cover point of `+3.5` on the
/// margin axis, not `-3.5`. Taking a direction explicitly is what keeps this
/// from repeating the sign confusion documented in [`crate::middle`].
///
/// # Errors
///
/// [`MathError::DomainError`] for a non-positive standard deviation.
pub fn cover_prob(
    expected: f64,
    cover_point: f64,
    std_dev: f64,
    direction: Direction,
) -> Result<f64> {
    if !std_dev.is_finite() || std_dev <= 0.0 {
        return Err(MathError::DomainError {
            param: "std_dev",
            constraint: "greater than zero",
            value: std_dev,
        });
    }
    let z = (expected - cover_point) / std_dev;
    Ok(match direction {
        Direction::Above => crate::probability::normal_cdf(z),
        Direction::Below => crate::probability::normal_cdf(-z),
    })
}

/// What a posterior is worth against a price on offer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    /// Posterior probability minus the price's implied probability, in points.
    pub edge_points: f64,
    /// Expected value as a fraction of stake.
    pub ev_fraction: f64,
    /// The American price the posterior implies.
    pub fair_odds: i32,
    /// Whether the edge clears the threshold and the EV is positive.
    pub is_bet: bool,
}

/// Prices a posterior probability against an available quote.
///
/// # Errors
///
/// [`MathError::ProbabilityOutOfRange`] for a posterior outside `(0, 1)`,
/// [`MathError::DomainError`] for a price at or below 1.0 or a negative
/// threshold.
pub fn edge_vs_price(posterior_prob: f64, decimal_odds: f64, threshold: f64) -> Result<Edge> {
    check_prob(posterior_prob, "posterior must be between 0 and 1")?;
    let implied = crate::odds::decimal_to_implied(decimal_odds)?;
    if !threshold.is_finite() || threshold < 0.0 {
        return Err(MathError::DomainError {
            param: "threshold",
            constraint: "non-negative",
            value: threshold,
        });
    }

    let edge_points = posterior_prob - implied;
    let ev_fraction = posterior_prob * decimal_odds - 1.0;

    Ok(Edge {
        edge_points,
        ev_fraction,
        // Shared conversion, which clamps. The TS reimplemented this without
        // the clamp and returned Infinity at a posterior of 0 or 1.
        fair_odds: implied_to_american(posterior_prob),
        is_bet: edge_points >= threshold && ev_fraction > 0.0,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;
    use crate::odds::american_to_decimal;
    use approx::assert_relative_eq;

    #[test]
    fn a_weightless_model_leaves_the_market_alone() {
        let p = beta_update(0.55, 0.75, 1_000.0, 0.0).unwrap();
        assert_relative_eq!(p.posterior_prob, 0.55, epsilon = 1e-12);
        assert_relative_eq!(p.shift_from_market, 0.0, epsilon = 1e-12);
    }

    #[test]
    fn a_weightless_market_hands_over_to_the_model() {
        let p = beta_update(0.55, 0.75, 0.0, 500.0).unwrap();
        assert_relative_eq!(p.posterior_prob, 0.75, epsilon = 1e-12);
    }

    #[test]
    fn equal_weights_split_the_difference() {
        let p = beta_update(0.50, 0.70, 100.0, 100.0).unwrap();
        assert_relative_eq!(p.posterior_prob, 0.60, epsilon = 1e-12);
    }

    #[test]
    fn a_heavy_market_prior_barely_moves() {
        // The usual honest setting: the market knows more than you do.
        let p = beta_update(0.55, 0.75, 10_000.0, 100.0).unwrap();
        assert!(p.shift_from_market < 0.01, "moved {}", p.shift_from_market);
        assert!(p.posterior_prob > 0.55);
    }

    #[test]
    fn the_posterior_always_lands_between_the_two_sources() {
        for (mkt, model) in [(0.3, 0.8), (0.8, 0.3), (0.5, 0.5), (0.05, 0.95)] {
            let p = beta_update(mkt, model, 300.0, 120.0).unwrap();
            let (lo, hi) = (mkt.min(model), mkt.max(model));
            assert!(
                (lo..=hi).contains(&p.posterior_prob),
                "{} escaped [{lo}, {hi}]",
                p.posterior_prob
            );
        }
    }

    #[test]
    fn beta_rejects_what_the_ts_accepted() {
        // A negative marketN gave negative Beta parameters and a posterior
        // outside [0, 1].
        assert!(beta_update(0.55, 0.75, -100.0, 100.0).is_err());
        assert!(beta_update(1.0, 0.75, 100.0, 100.0).is_err());
        assert!(beta_update(0.55, 0.0, 100.0, 100.0).is_err());
        assert!(beta_update(0.55, 0.75, 0.0, 0.0).is_err());
    }

    #[test]
    fn dirichlet_posteriors_are_distributions() {
        let market = [0.45, 0.28, 0.27];
        let model = [0.55, 0.25, 0.20];
        let p = dirichlet_update(&market, &model, 500.0, 100.0).unwrap();
        assert_relative_eq!(p.posterior_probs.iter().sum::<f64>(), 1.0, epsilon = 1e-12);
        assert_eq!(p.posterior_probs.len(), 3);
        assert!(p.posterior_probs[0] > market[0], "should move toward the model");
    }

    #[test]
    fn dirichlet_handles_more_than_three_outcomes() {
        // The TS was hard-coded to exactly three.
        let market = [0.4, 0.3, 0.2, 0.1];
        let model = [0.25, 0.25, 0.25, 0.25];
        let p = dirichlet_update(&market, &model, 100.0, 100.0).unwrap();
        assert_eq!(p.posterior_probs.len(), 4);
        assert_relative_eq!(p.posterior_probs.iter().sum::<f64>(), 1.0, epsilon = 1e-12);
        assert_relative_eq!(p.posterior_probs[0], 0.325, epsilon = 1e-12);
    }

    #[test]
    fn dirichlet_rejects_a_market_that_was_never_devigged() {
        // Raw implied probabilities sum past 1; the TS took them as a prior.
        let vigged = [0.48, 0.30, 0.29];
        assert!(dirichlet_update(&vigged, &[0.45, 0.30, 0.25], 500.0, 100.0).is_err());
        assert!(dirichlet_update(&[0.5, 0.5], &[0.5], 100.0, 100.0).is_err());
    }

    #[test]
    fn precision_weighting_favors_the_tighter_estimate() {
        let p = margin_update(3.0, 7.0, 2.0, 6.0).unwrap();
        // The market is three times tighter, so it carries nine times the weight.
        assert_relative_eq!(p.market_weight, 0.9, epsilon = 1e-12);
        assert_relative_eq!(p.margin, 3.4, epsilon = 1e-12);
        assert_relative_eq!(p.spread, -3.4, epsilon = 1e-12);
    }

    #[test]
    fn combining_estimates_never_widens_the_posterior() {
        let (market_std, model_std) = (2.0_f64, 6.0_f64);
        let p = margin_update(3.0, 7.0, market_std, model_std).unwrap();
        assert!(
            p.std_dev < market_std.min(model_std),
            "posterior {} is wider than the tighter input {}",
            p.std_dev,
            market_std.min(model_std)
        );
        assert_relative_eq!(p.std_dev, (1.0_f64 / (0.25 + 1.0 / 36.0)).sqrt(), epsilon = 1e-12);
    }

    #[test]
    fn a_zero_standard_deviation_is_an_error_not_a_clamp() {
        // The TS raised it to 0.001 — a precision of a million — and let that
        // source win outright without saying anything.
        assert!(margin_update(3.0, 7.0, 0.0, 6.0).is_err());
        assert!(margin_update(3.0, 7.0, 2.0, -1.0).is_err());
        assert!(margin_update(f64::NAN, 7.0, 2.0, 6.0).is_err());
    }

    #[test]
    fn cover_probability_is_symmetric_across_directions() {
        let above = cover_prob(3.4, 3.5, 13.86, Direction::Above).unwrap();
        let below = cover_prob(3.4, 3.5, 13.86, Direction::Below).unwrap();
        assert_relative_eq!(above + below, 1.0, epsilon = 1e-9);
    }

    #[test]
    fn a_bigger_expected_margin_covers_more_often() {
        let weak = cover_prob(1.0, 3.5, 13.86, Direction::Above).unwrap();
        let strong = cover_prob(9.0, 3.5, 13.86, Direction::Above).unwrap();
        assert!(strong > weak);
        assert!(weak < 0.5 && strong > 0.5);
    }

    #[test]
    fn edge_is_zero_at_the_fair_price() {
        let e = edge_vs_price(0.5, 2.0, 0.0).unwrap();
        assert_relative_eq!(e.edge_points, 0.0, epsilon = 1e-12);
        assert_relative_eq!(e.ev_fraction, 0.0, epsilon = 1e-12);
        assert!(!e.is_bet, "a zero edge is not a bet");
        assert_eq!(e.fair_odds, -100);
    }

    #[test]
    fn a_real_edge_clears_the_threshold() {
        // 55% on a +100 shot.
        let e = edge_vs_price(0.55, 2.0, 0.02).unwrap();
        assert_relative_eq!(e.edge_points, 0.05, epsilon = 1e-12);
        assert_relative_eq!(e.ev_fraction, 0.10, epsilon = 1e-12);
        assert!(e.is_bet);
    }

    #[test]
    fn a_thin_edge_fails_a_strict_threshold() {
        let e = edge_vs_price(0.53, 2.0, 0.05).unwrap();
        assert!(e.ev_fraction > 0.0, "still +EV");
        assert!(!e.is_bet, "but below the threshold asked for");
    }

    #[test]
    fn fair_odds_stay_finite_at_the_extremes() {
        // The TS reimplemented this conversion without the clamp and returned
        // Infinity for a posterior of 0 or 1.
        for p in [0.000_001, 0.999_999] {
            let e = edge_vs_price(p, american_to_decimal(-110.0).unwrap(), 0.0).unwrap();
            assert!(
                e.fair_odds.abs() < 1_000_000,
                "fair odds ran away to {}",
                e.fair_odds
            );
        }
    }

    #[test]
    fn edge_rejects_bad_input() {
        assert!(edge_vs_price(0.0, 2.0, 0.0).is_err());
        assert!(edge_vs_price(1.0, 2.0, 0.0).is_err());
        assert!(edge_vs_price(0.5, 1.0, 0.0).is_err());
        assert!(edge_vs_price(0.5, 2.0, -0.1).is_err());
    }
}
