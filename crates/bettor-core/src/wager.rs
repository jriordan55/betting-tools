//! Expected value and Kelly stake sizing.
//!
//! Extracted from `ExpectedValue.tsx` and `KellyCriterion.tsx`, where the math
//! lived inside `useMemo` blocks in the components.
//!
//! # Divergences from the TypeScript
//!
//! 1. **Everything is a 0–1 fraction.** The components mixed scales freely —
//!    `ev` as a dollar amount, `evPercent` as 0–100, `edge` in percentage
//!    points — and formatted with `.toFixed()` at the point of computation.
//!    Formatting is the frontend's job.
//! 2. **Negative stakes and bankrolls are rejected.** The TS guard was
//!    `if (!stakeNum)`, which is false for `-500`, so a negative stake flowed
//!    through the arithmetic and produced a confident negative EV.

use crate::odds::decimal_to_implied;
use crate::{MathError, Result};
use serde::Serialize;

/// What a bet is worth at a given price and true probability.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ExpectedValue {
    /// EV as a fraction of stake. `0.05` is a 5% edge.
    pub ev_fraction: f64,
    /// EV in currency, for the stake supplied.
    pub ev_amount: f64,
    /// Probability implied by the price taken, 0–1.
    pub implied_prob: f64,
    /// Win rate needed to break even at this price, 0–1. Identical to
    /// [`Self::implied_prob`], named for what it means to a bettor.
    pub breakeven: f64,
    /// True probability minus implied probability, in probability points.
    pub edge_points: f64,
}

impl ExpectedValue {
    /// Whether the bet is worth making.
    #[must_use]
    pub fn is_positive(&self) -> bool {
        self.ev_fraction > 0.0
    }
}

/// Expected value of a bet.
///
/// # Errors
///
/// [`MathError::DomainError`] for a price at or below 1.0 or a negative stake,
/// [`MathError::ProbabilityOutOfRange`] if `true_prob` is outside `(0, 1)`.
pub fn expected_value(decimal: f64, true_prob: f64, stake: f64) -> Result<ExpectedValue> {
    let implied_prob = decimal_to_implied(decimal)?;
    if !true_prob.is_finite() || true_prob <= 0.0 || true_prob >= 1.0 {
        return Err(MathError::ProbabilityOutOfRange {
            value: true_prob,
            reason: "true win probability must be strictly between 0 and 1",
        });
    }
    if !stake.is_finite() || stake < 0.0 {
        return Err(MathError::DomainError {
            param: "stake",
            constraint: "non-negative",
            value: stake,
        });
    }
    let ev_fraction = true_prob * decimal - 1.0;
    Ok(ExpectedValue {
        ev_fraction,
        ev_amount: ev_fraction * stake,
        implied_prob,
        breakeven: implied_prob,
        edge_points: true_prob - implied_prob,
    })
}

/// Optimal stake under the Kelly criterion.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Kelly {
    /// Full-Kelly fraction of bankroll. Negative means do not bet.
    pub full_fraction: f64,
    /// Fraction after applying the chosen Kelly multiplier.
    pub adjusted_fraction: f64,
    /// Stake implied by [`Self::full_fraction`], floored at zero.
    pub full_stake: f64,
    /// Stake implied by [`Self::adjusted_fraction`], floored at zero.
    pub adjusted_stake: f64,
    /// EV of the bet as a fraction of stake.
    pub ev_fraction: f64,
    /// True probability minus implied, in probability points.
    pub edge_points: f64,
}

impl Kelly {
    /// Whether Kelly says to bet at all.
    #[must_use]
    pub fn should_bet(&self) -> bool {
        self.adjusted_fraction > 0.0
    }
}

/// Kelly stake for a price, a true probability, and a bankroll.
///
/// `multiplier` scales the recommendation — `0.5` is half Kelly. Full Kelly
/// maximizes long-run growth but is famously volatile, and it assumes the
/// probability estimate is exact; overstating an edge overbets it quadratically.
///
/// # Errors
///
/// [`MathError::DomainError`] for a price at or below 1.0, a non-positive
/// bankroll, or a multiplier outside `(0, 1]`.
/// [`MathError::ProbabilityOutOfRange`] if `true_prob` is outside `(0, 1)`.
pub fn kelly(decimal: f64, true_prob: f64, bankroll: f64, multiplier: f64) -> Result<Kelly> {
    let ev = expected_value(decimal, true_prob, 0.0)?;
    if !bankroll.is_finite() || bankroll <= 0.0 {
        return Err(MathError::DomainError {
            param: "bankroll",
            constraint: "greater than zero",
            value: bankroll,
        });
    }
    if !multiplier.is_finite() || multiplier <= 0.0 || multiplier > 1.0 {
        return Err(MathError::DomainError {
            param: "kelly multiplier",
            constraint: "in (0, 1]",
            value: multiplier,
        });
    }

    // f* = (bp - q) / b, where b is net decimal odds.
    let b = decimal - 1.0;
    let p = true_prob;
    let q = 1.0 - p;
    let full_fraction = (b * p - q) / b;
    let adjusted_fraction = full_fraction * multiplier;

    Ok(Kelly {
        full_fraction,
        adjusted_fraction,
        full_stake: (full_fraction * bankroll).max(0.0),
        adjusted_stake: (adjusted_fraction * bankroll).max(0.0),
        ev_fraction: ev.ev_fraction,
        edge_points: ev.edge_points,
    })
}

/// One point on an expected-value curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct EvPoint {
    /// The true win probability being assumed, 0–1.
    pub true_prob: f64,
    /// Expected value per unit staked at that probability.
    pub ev: f64,
}

/// Expected value across a range of assumed true probabilities.
///
/// A bet is only good relative to a belief, and the belief is the input nobody
/// can check. Drawing EV as a function of it moves the question from "is this
/// +EV" — which invites a made-up number — to "how wrong would I have to be
/// for this to stop being +EV", which is answerable. The break-even point is
/// the price's own implied probability.
///
/// # Errors
///
/// [`MathError::DomainError`] for a price at or below 1.0, a non-positive
/// step, or a reversed range; [`MathError::ProbabilityOutOfRange`] if the
/// range falls outside `[0, 1]`; [`MathError::ShapeError`] beyond 400 points.
pub fn ev_curve(decimal: f64, from_prob: f64, to_prob: f64, step: f64) -> Result<Vec<EvPoint>> {
    crate::odds::decimal_to_implied(decimal)?;
    if !step.is_finite() || step <= 0.0 {
        return Err(MathError::DomainError {
            param: "step",
            constraint: "greater than zero",
            value: step,
        });
    }
    for value in [from_prob, to_prob] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(MathError::ProbabilityOutOfRange {
                value,
                reason: "the range must lie between 0 and 1",
            });
        }
    }
    if to_prob <= from_prob {
        return Err(MathError::DomainError {
            param: "probability range",
            constraint: "the second probability must be above the first",
            value: to_prob - from_prob,
        });
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "non-negative by the checks above, and capped on the next line"
    )]
    let points = ((to_prob - from_prob) / step).floor() as usize + 1;
    if points > 400 {
        return Err(MathError::ShapeError {
            what: "curve points",
            expected: "at most 400",
            got: points,
        });
    }

    Ok((0..points)
        .map(|i| {
            #[allow(clippy::cast_precision_loss, reason = "point index, capped at 400")]
            let true_prob = from_prob + i as f64 * step;
            EvPoint {
                true_prob,
                ev: true_prob * decimal - 1.0,
            }
        })
        .collect())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::odds::american_to_decimal;
    use approx::assert_relative_eq;

    #[test]
    fn a_fair_bet_has_no_edge() {
        let ev = expected_value(2.0, 0.5, 100.0).unwrap();
        assert_relative_eq!(ev.ev_fraction, 0.0, epsilon = 1e-12);
        assert_relative_eq!(ev.edge_points, 0.0, epsilon = 1e-12);
        assert!(!ev.is_positive());
    }

    #[test]
    fn the_longshot_example_from_the_readme() {
        // +400 at a true 21% is a 5% edge, and the breakeven is 20%.
        let d = american_to_decimal(400.0).unwrap();
        let ev = expected_value(d, 0.21, 100.0).unwrap();
        assert_relative_eq!(ev.ev_fraction, 0.05, epsilon = 1e-12);
        assert_relative_eq!(ev.breakeven, 0.20, epsilon = 1e-12);
        assert_relative_eq!(ev.ev_amount, 5.0, epsilon = 1e-12);
        assert_relative_eq!(ev.edge_points, 0.01, epsilon = 1e-12);
    }

    #[test]
    fn equal_ev_at_different_prices_means_very_different_edges_in_points() {
        // The whole thesis: same 5% EV, wildly different margin for error.
        let long = expected_value(american_to_decimal(400.0).unwrap(), 0.21, 100.0).unwrap();
        let short = expected_value(american_to_decimal(-110.0).unwrap(), 0.55, 100.0).unwrap();
        assert_relative_eq!(long.ev_fraction, short.ev_fraction, epsilon = 1e-3);
        // ...but the favorite's edge is nearly three points, the longshot's one.
        assert!(short.edge_points > 2.5 * long.edge_points);
    }

    #[test]
    fn negative_stakes_are_rejected() {
        // The TS guard `if (!stakeNum)` let -500 straight through.
        assert!(expected_value(2.0, 0.5, -500.0).is_err());
    }

    #[test]
    fn kelly_matches_the_textbook_coin_flip() {
        // Even money on a 60% shot: f* = 0.2 of bankroll.
        let k = kelly(2.0, 0.6, 10_000.0, 1.0).unwrap();
        assert_relative_eq!(k.full_fraction, 0.2, epsilon = 1e-12);
        assert_relative_eq!(k.full_stake, 2_000.0, epsilon = 1e-9);
        assert!(k.should_bet());
    }

    #[test]
    fn fractional_kelly_scales_linearly() {
        let full = kelly(2.0, 0.6, 10_000.0, 1.0).unwrap();
        let half = kelly(2.0, 0.6, 10_000.0, 0.5).unwrap();
        assert_relative_eq!(half.adjusted_stake, full.full_stake / 2.0, epsilon = 1e-9);
    }

    #[test]
    fn kelly_refuses_a_negative_edge() {
        let k = kelly(2.0, 0.4, 10_000.0, 1.0).unwrap();
        assert!(k.full_fraction < 0.0);
        assert!(!k.should_bet());
        assert_relative_eq!(k.adjusted_stake, 0.0, epsilon = 1e-12);
    }

    #[test]
    fn kelly_stakes_longshots_far_smaller_for_the_same_ev() {
        // Same 5% edge; Kelly sizes the longshot at a fraction of the favorite,
        // which is the variance story showing up in the stake itself.
        let long = kelly(american_to_decimal(400.0).unwrap(), 0.21, 10_000.0, 1.0).unwrap();
        let short = kelly(american_to_decimal(-110.0).unwrap(), 0.55, 10_000.0, 1.0).unwrap();
        assert!(
            short.full_fraction > 4.0 * long.full_fraction,
            "favorite {} vs longshot {}",
            short.full_fraction,
            long.full_fraction
        );
    }

    #[test]
    fn invalid_inputs_are_rejected() {
        assert!(kelly(2.0, 0.6, 0.0, 1.0).is_err());
        assert!(kelly(2.0, 0.6, 10_000.0, 0.0).is_err());
        assert!(kelly(2.0, 0.6, 10_000.0, 1.5).is_err());
        assert!(kelly(1.0, 0.6, 10_000.0, 1.0).is_err());
        assert!(kelly(2.0, 1.0, 10_000.0, 1.0).is_err());
    }
    #[test]
    fn the_ev_curve_crosses_zero_at_the_implied_probability() {
        let d = american_to_decimal(-110.0).unwrap();
        let curve = ev_curve(d, 0.0, 1.0, 0.005).unwrap();
        let crossing = curve
            .iter()
            .find(|p| p.ev >= 0.0)
            .expect("the curve must reach break-even somewhere");
        assert_relative_eq!(crossing.true_prob, 0.525, epsilon = 3e-3);
    }

    #[test]
    fn the_ev_curve_rises_with_the_belief_and_is_steeper_at_long_prices() {
        // Slope is the decimal price: a longshot's EV swings far harder on the
        // same change of mind, which is the same fact the variance module
        // reports as a longer confirmation horizon.
        let short = ev_curve(american_to_decimal(-110.0).unwrap(), 0.1, 0.9, 0.1).unwrap();
        let long = ev_curve(american_to_decimal(600.0).unwrap(), 0.1, 0.9, 0.1).unwrap();

        for w in short.windows(2) {
            assert!(w[1].ev > w[0].ev);
        }
        let short_slope = short[1].ev - short[0].ev;
        let long_slope = long[1].ev - long[0].ev;
        assert!(long_slope > 3.0 * short_slope);
    }

    #[test]
    fn the_ev_curve_rejects_bad_input() {
        assert!(ev_curve(1.0, 0.0, 1.0, 0.1).is_err());
        assert!(ev_curve(2.0, 0.0, 1.0, 0.0).is_err());
        assert!(ev_curve(2.0, 0.9, 0.1, 0.1).is_err());
        assert!(ev_curve(2.0, -0.1, 1.0, 0.1).is_err());
        assert!(ev_curve(2.0, 0.0, 1.0, 0.0001).is_err());
    }

}
