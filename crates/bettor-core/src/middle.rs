//! Middles and traps — holding both sides of a market at different numbers.
//!
//! Ported from `bettor-calculator-main/src/lib/math/middle.ts`.
//!
//! Take the favorite at -3.5 and the underdog at +7.5 and any margin of 4, 5,
//! 6 or 7 wins both tickets. Reverse the numbers and the same gap loses both —
//! a trap. Either way the arithmetic is one normal distribution over a single
//! **value axis**: the favorite's margin for a spread, the combined score for
//! a total.
//!
//! # Divergences from the TypeScript
//!
//! 1. **Spread middles were computed across two opposite sign conventions.**
//!    `impliedTrueLine(-3.5, …)` returns the mean of *minus* the margin, while
//!    `impliedTrueLine(+7.5, …)` returns the mean of *plus* the margin. The TS
//!    averaged them, which averages a quantity with its own negation. On a
//!    symmetric market — both sides -5.5 at -110 — it produced a true center of
//!    −0.83 and gave the favorite a **32.4%** chance of covering. The answer is
//!    50%. Totals were unaffected, because those already used over and under
//!    inversions in the same frame.
//!
//!    This module removes the trap by construction: legs are named for the
//!    direction that wins them, not for which team they are on, and spread
//!    lines are folded onto the margin axis before anything else happens.
//! 2. **The vig is removed first.** Inherited from `bestline.ts` — see
//!    [`crate::line`].
//! 3. **Prices are validated.** `americanProfit` did its own `parseFloat` and
//!    accepted quotes inside ±100, so "-50" paid out at 2:1.

use crate::line::{fair_prob_at_line, implied_true_line, BetType, TotalSide};
use crate::probability::normal_cdf;
use crate::{MathError, Result};
use serde::Serialize;

/// Which market the two positions sit on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Market {
    /// A point spread. Lines are folded onto the favorite's margin axis, so
    /// `-3.5` and `+3.5` describe the same number.
    Spread,
    /// A total. Lines are used as posted.
    Total,
}

/// One side of the position.
///
/// Named for the direction that wins it rather than the team it is on, which
/// is what stops the two legs drifting into different sign conventions.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[cfg_attr(feature = "specta", specta(rename = "MiddleLeg"))]
#[serde(rename_all = "camelCase")]
pub struct Leg {
    /// The number bet, as posted.
    pub line: f64,
    /// Fair (devigged) probability this leg covers — see
    /// [`crate::line::fair_cover_prob`].
    pub fair_cover_prob: f64,
    /// Amount staked.
    pub stake: f64,
    /// Decimal price taken.
    pub decimal_odds: f64,
}

impl Leg {
    fn profit(&self) -> f64 {
        self.stake * (self.decimal_odds - 1.0)
    }

    fn validate(&self, which: &'static str) -> Result<()> {
        if !self.stake.is_finite() || self.stake <= 0.0 {
            return Err(MathError::DomainError {
                param: which,
                constraint: "stake greater than zero",
                value: self.stake,
            });
        }
        if !self.decimal_odds.is_finite() || self.decimal_odds <= 1.0 {
            return Err(MathError::DomainError {
                param: which,
                constraint: "decimal odds greater than 1",
                value: self.decimal_odds,
            });
        }
        if !self.fair_cover_prob.is_finite()
            || self.fair_cover_prob <= 0.0
            || self.fair_cover_prob >= 1.0
        {
            return Err(MathError::ProbabilityOutOfRange {
                value: self.fair_cover_prob,
                reason: "fair cover probability must be strictly between 0 and 1",
            });
        }
        Ok(())
    }
}

/// One way the position can resolve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    /// What happened, for display.
    pub label: &'static str,
    /// Probability of it, 0–1.
    pub probability: f64,
    /// Net profit across both tickets if it happens.
    pub net_profit: f64,
}

/// A middle or trap, priced.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Middle {
    /// True when the gap wins both tickets, false when it loses both.
    pub is_middle: bool,
    /// Width of the gap between the two numbers.
    pub gap_size: f64,
    /// Probability the result lands inside the gap, 0–1.
    pub gap_prob: f64,
    /// Market's expected value on the shared axis — the favorite's expected
    /// margin, or the expected total.
    pub true_center: f64,
    /// Expected profit in currency.
    pub ev: f64,
    /// [`Self::ev`] as a fraction of everything staked.
    pub ev_fraction: f64,
    /// Total staked across both tickets.
    pub total_staked: f64,
    /// Every way this resolves. Probabilities sum to 1.
    pub outcomes: Vec<Outcome>,
}

/// Prices a middle or trap.
///
/// `high_side` is the leg that wins when the value comes in **high** — the
/// favorite covering a spread, or the over. `low_side` wins when it comes in
/// **low** — the underdog, or the under.
///
/// # Errors
///
/// [`MathError::DomainError`] for a non-positive stake, standard deviation, or
/// a price at or below 1.0; [`MathError::ProbabilityOutOfRange`] for a cover
/// probability outside `(0, 1)`.
pub fn calculate_middle(
    market: Market,
    high_side: Leg,
    low_side: Leg,
    std_dev: f64,
) -> Result<Middle> {
    high_side.validate("high side")?;
    low_side.validate("low side")?;
    if !std_dev.is_finite() || std_dev <= 0.0 {
        return Err(MathError::DomainError {
            param: "std_dev",
            constraint: "greater than zero",
            value: std_dev,
        });
    }

    // Both legs onto one axis. For a spread that axis is the favorite's
    // margin, so a posted -3.5 and a posted +3.5 are the same number 3.5.
    let (high_threshold, low_threshold) = match market {
        Market::Spread => (high_side.line.abs(), low_side.line.abs()),
        Market::Total => (high_side.line, low_side.line),
    };

    // Each leg implies where the market thinks the axis is centred. The
    // high side wins when the value exceeds its number, so it inverts like an
    // over; the low side inverts like an under. Using one convention for both
    // — as the TS did for spreads — mixes two opposite frames.
    let center_from_high = implied_true_line(
        high_threshold,
        high_side.fair_cover_prob,
        std_dev,
        BetType::Total(TotalSide::Over),
    )?;
    let center_from_low = implied_true_line(
        low_threshold,
        low_side.fair_cover_prob,
        std_dev,
        BetType::Total(TotalSide::Under),
    )?;
    let true_center = (center_from_high + center_from_low) / 2.0;

    let lo = high_threshold.min(low_threshold);
    let hi = high_threshold.max(low_threshold);
    let gap_size = hi - lo;
    let gap_prob = normal_cdf((hi - true_center) / std_dev) - normal_cdf((lo - true_center) / std_dev);

    // A middle when the low side's ceiling sits above the high side's floor,
    // leaving a band that satisfies both tickets.
    let is_middle = low_threshold > high_threshold;

    let p_high_covers = fair_prob_at_line(
        true_center,
        high_threshold,
        std_dev,
        BetType::Total(TotalSide::Over),
    );
    let p_low_covers = fair_prob_at_line(
        true_center,
        low_threshold,
        std_dev,
        BetType::Total(TotalSide::Under),
    );

    let (high_label, low_label) = match market {
        Market::Spread => ("Favorite only", "Underdog only"),
        Market::Total => ("Over only", "Under only"),
    };

    let mut outcomes = Vec::with_capacity(4);
    if is_middle {
        outcomes.push(Outcome {
            label: "Both win",
            probability: gap_prob,
            net_profit: high_side.profit() + low_side.profit(),
        });
    }
    outcomes.push(Outcome {
        label: high_label,
        probability: if is_middle {
            (p_high_covers - gap_prob).max(0.0)
        } else {
            p_high_covers
        },
        net_profit: high_side.profit() - low_side.stake,
    });
    outcomes.push(Outcome {
        label: low_label,
        probability: if is_middle {
            (p_low_covers - gap_prob).max(0.0)
        } else {
            p_low_covers
        },
        net_profit: low_side.profit() - high_side.stake,
    });
    if !is_middle {
        outcomes.push(Outcome {
            label: "Both lose",
            probability: gap_prob,
            net_profit: -(high_side.stake + low_side.stake),
        });
    }

    let total_staked = high_side.stake + low_side.stake;
    let ev: f64 = outcomes
        .iter()
        .map(|o| o.probability * o.net_profit)
        .sum();

    Ok(Middle {
        is_middle,
        gap_size,
        gap_prob,
        true_center,
        ev,
        ev_fraction: ev / total_staked,
        total_staked,
        outcomes,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;
    use crate::line::fair_cover_prob;
    use crate::odds::american_to_decimal;
    use approx::assert_relative_eq;

    const NFL_STD: f64 = 13.86;

    fn leg(line: f64, american: f64, opposing: f64, stake: f64) -> Leg {
        Leg {
            line,
            fair_cover_prob: fair_cover_prob(american, opposing).unwrap(),
            stake,
            decimal_odds: american_to_decimal(american).unwrap(),
        }
    }

    #[test]
    fn a_symmetric_spread_market_is_a_coin_flip() {
        // THE regression test. The TS reported 32.4% here; the answer is 50%.
        let m = calculate_middle(
            Market::Spread,
            leg(-5.5, -110.0, -110.0, 100.0),
            leg(5.5, -110.0, -110.0, 100.0),
            NFL_STD,
        )
        .unwrap();

        assert_relative_eq!(m.true_center, 5.5, epsilon = 1e-9);
        assert_relative_eq!(m.gap_size, 0.0, epsilon = 1e-12);
        let favorite = m.outcomes.iter().find(|o| o.label == "Favorite only").unwrap();
        assert_relative_eq!(favorite.probability, 0.5, epsilon = 1e-6);
    }

    #[test]
    fn the_sign_convention_no_longer_matters() {
        // Entering the dog side as +7.5 or -7.5 must describe the same bet.
        let positive = calculate_middle(
            Market::Spread,
            leg(-3.5, -110.0, -110.0, 100.0),
            leg(7.5, -110.0, -110.0, 100.0),
            NFL_STD,
        )
        .unwrap();
        let negative = calculate_middle(
            Market::Spread,
            leg(-3.5, -110.0, -110.0, 100.0),
            leg(-7.5, -110.0, -110.0, 100.0),
            NFL_STD,
        )
        .unwrap();
        assert_relative_eq!(positive.true_center, negative.true_center, epsilon = 1e-12);
        assert_relative_eq!(positive.gap_prob, negative.gap_prob, epsilon = 1e-12);
    }

    #[test]
    fn the_true_center_sits_between_the_two_numbers() {
        let m = calculate_middle(
            Market::Spread,
            leg(-3.5, -110.0, -110.0, 100.0),
            leg(7.5, -110.0, -110.0, 100.0),
            NFL_STD,
        )
        .unwrap();
        assert!(
            (3.5..=7.5).contains(&m.true_center),
            "center {} escaped the gap",
            m.true_center
        );
        assert_relative_eq!(m.true_center, 5.5, epsilon = 1e-6);
        assert_relative_eq!(m.gap_size, 4.0, epsilon = 1e-12);
    }

    #[test]
    fn outcome_probabilities_always_sum_to_one() {
        for (a, b) in [(-3.5, 7.5), (-7.5, 3.5), (-5.5, 5.5), (-1.5, 10.5)] {
            let m = calculate_middle(
                Market::Spread,
                leg(a, -110.0, -110.0, 100.0),
                leg(b, -110.0, -110.0, 100.0),
                NFL_STD,
            )
            .unwrap();
            let total: f64 = m.outcomes.iter().map(|o| o.probability).sum();
            assert_relative_eq!(total, 1.0, epsilon = 1e-9);
        }
    }

    #[test]
    fn a_real_middle_is_flagged_and_can_win_twice() {
        let m = calculate_middle(
            Market::Spread,
            leg(-3.5, -110.0, -110.0, 100.0),
            leg(7.5, -110.0, -110.0, 100.0),
            NFL_STD,
        )
        .unwrap();
        assert!(m.is_middle);
        assert!(m.gap_prob > 0.0);
        let both = m.outcomes.iter().find(|o| o.label == "Both win").unwrap();
        assert!(both.net_profit > 0.0);
        assert!(m.outcomes.iter().all(|o| o.label != "Both lose"));
    }

    #[test]
    fn a_reversed_gap_is_a_trap() {
        let m = calculate_middle(
            Market::Spread,
            leg(-7.5, -110.0, -110.0, 100.0),
            leg(3.5, -110.0, -110.0, 100.0),
            NFL_STD,
        )
        .unwrap();
        assert!(!m.is_middle);
        let both = m.outcomes.iter().find(|o| o.label == "Both lose").unwrap();
        assert_relative_eq!(both.net_profit, -200.0, epsilon = 1e-9);
        assert!(m.ev < 0.0, "a trap at fair prices cannot be +EV");
    }

    #[test]
    fn totals_work_the_same_way() {
        let m = calculate_middle(
            Market::Total,
            leg(218.5, -110.0, -110.0, 100.0),
            leg(222.5, -110.0, -110.0, 100.0),
            16.0,
        )
        .unwrap();
        assert!(m.is_middle);
        assert_relative_eq!(m.true_center, 220.5, epsilon = 1e-6);
        assert_relative_eq!(m.gap_size, 4.0, epsilon = 1e-12);
        let total: f64 = m.outcomes.iter().map(|o| o.probability).sum();
        assert_relative_eq!(total, 1.0, epsilon = 1e-9);
    }

    #[test]
    fn a_wider_gap_is_more_likely_to_land() {
        let narrow = calculate_middle(
            Market::Spread,
            leg(-4.5, -110.0, -110.0, 100.0),
            leg(5.5, -110.0, -110.0, 100.0),
            NFL_STD,
        )
        .unwrap();
        let wide = calculate_middle(
            Market::Spread,
            leg(-1.5, -110.0, -110.0, 100.0),
            leg(8.5, -110.0, -110.0, 100.0),
            NFL_STD,
        )
        .unwrap();
        assert!(wide.gap_prob > narrow.gap_prob);
    }

    #[test]
    fn holding_both_sides_at_the_same_number_is_just_paying_the_vig() {
        let m = calculate_middle(
            Market::Spread,
            leg(-5.5, -110.0, -110.0, 100.0),
            leg(5.5, -110.0, -110.0, 100.0),
            NFL_STD,
        )
        .unwrap();
        // Win one, lose one: profit 90.91 − 100 either way.
        assert_relative_eq!(m.ev, -9.0909, epsilon = 1e-3);
        assert!(m.ev_fraction < 0.0);
    }

    #[test]
    fn invalid_input_is_rejected() {
        let good = leg(-3.5, -110.0, -110.0, 100.0);
        assert!(calculate_middle(Market::Spread, good, good, 0.0).is_err());
        let zero_stake = Leg { stake: 0.0, ..good };
        assert!(calculate_middle(Market::Spread, zero_stake, good, NFL_STD).is_err());
        let bad_price = Leg { decimal_odds: 1.0, ..good };
        assert!(calculate_middle(Market::Spread, bad_price, good, NFL_STD).is_err());
        let bad_prob = Leg { fair_cover_prob: 0.0, ..good };
        assert!(calculate_middle(Market::Spread, bad_prob, good, NFL_STD).is_err());
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod ts_comparison {
    use super::*;
    use crate::line::fair_cover_prob;
    use crate::odds::american_to_decimal;
    use approx::assert_relative_eq;

    /// The concrete case used to document the sign-convention bug.
    ///
    /// Favorite -3.5 at -110 for $100, underdog +7.5 at +105 for $120, NFL
    /// volatility. The TypeScript reported a true center of 1.798 and an EV of
    /// +$28.39 (+12.90%). Both come from averaging two opposite frames.
    #[test]
    fn the_documented_asymmetric_middle() {
        let high = Leg {
            line: -3.5,
            fair_cover_prob: fair_cover_prob(-110.0, -110.0).unwrap(),
            stake: 100.0,
            decimal_odds: american_to_decimal(-110.0).unwrap(),
        };
        let low = Leg {
            line: 7.5,
            fair_cover_prob: fair_cover_prob(105.0, -125.0).unwrap(),
            stake: 120.0,
            decimal_odds: american_to_decimal(105.0).unwrap(),
        };
        let m = calculate_middle(Market::Spread, high, low, 13.86).unwrap();

        // The center belongs between the two numbers, not below both of them.
        assert!(
            (3.5..=7.5).contains(&m.true_center),
            "center {} is outside the gap; the TS put it at 1.798",
            m.true_center
        );
        assert!(
            (m.true_center - 1.798).abs() > 1.0,
            "should not reproduce the TS center"
        );

        let total: f64 = m.outcomes.iter().map(|o| o.probability).sum();
        assert_relative_eq!(total, 1.0, epsilon = 1e-9);
        assert!(m.is_middle);
    }
}
