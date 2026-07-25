//! Closing line value — measured three ways, because the three disagree.
//!
//! Extracted from `CLVCalculator.tsx`, where the math lived inside a
//! `useMemo` in the component.
//!
//! # The problem this module exists to fix
//!
//! The TSX computed CLV as `betDecimal / closeDecimal - 1` and displayed it
//! three times under three names:
//!
//! | Row label | Expression | |
//! |---|---|---|
//! | Closing Line Value | `b/c - 1` | |
//! | Edge (cents per dollar) | `(b-c)/c` | algebraically the same |
//! | Expected Value | `(1/c)·b - 1` | also the same |
//!
//! Three rows, one number. Worse, that number is a *ratio of decimal odds*,
//! which systematically flatters longshots:
//!
//! | Bet → Close | ratio CLV | probability points |
//! |---|---|---|
//! | -110 → -130 | 7.9% | **+4.14 pts** |
//! | +400 → +350 | 11.1% | **+2.22 pts** |
//!
//! The ratio ranks the longshot as the better bet. In the currency that
//! actually compounds a bankroll — probability — it is worth barely half as
//! much. [`Clv::prob_points`] is the honest measure and the one to lead with.
//!
//! # The second problem
//!
//! The TSX set `trueProbability = 1 / closeDecimal`, taking the raw closing
//! price as truth. That price still contains the book's margin, so every EV it
//! reported was inflated. Removing the vig needs the *other* side of the
//! closing market, which is why [`clv`] asks for it and reports
//! [`Clv::ev_vs_fair`] as `None` when it is absent rather than guessing.

use crate::odds::decimal_to_implied;
use crate::{MathError, Result};
use serde::Serialize;

/// Closing line value, measured every way that is defensible.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Clv {
    /// Decimal price taken.
    pub bet_decimal: f64,
    /// Decimal price at the close.
    pub close_decimal: f64,
    /// Probability implied by the price taken, 0–1.
    pub bet_implied: f64,
    /// Probability implied by the closing price, 0–1. Still contains vig.
    pub close_implied: f64,

    /// `bet/close - 1`. What the TS called CLV, EV, and "cents" alike.
    ///
    /// Retained for continuity with the web app, but it is a ratio of prices
    /// and should never be compared across different price levels.
    pub ratio: f64,

    /// Probability points gained: `close_implied - bet_implied`.
    ///
    /// The measure that survives comparison across price levels. Positive
    /// means the market moved toward your side after you bet.
    pub prob_points: f64,

    /// American cents the line moved, which is what people actually quote.
    ///
    /// Reported alongside [`Self::prob_points`] precisely so the gap between
    /// the two is visible: fifty cents on a longshot is worth less than twenty
    /// on a favorite.
    pub cents: i32,

    /// EV against the raw closing price — vig included, so overstated.
    ///
    /// This is the number the web app reported as "Expected Value".
    pub ev_vs_raw_close: f64,

    /// EV against the devigged closing price.
    ///
    /// `None` when the opposing closing price was not supplied, because
    /// without it the vig cannot be removed and any figure would be a guess.
    pub ev_vs_fair: Option<f64>,

    /// Fair probability at the close once the vig is removed, 0–1.
    pub fair_prob: Option<f64>,
}

impl Clv {
    /// Whether the line moved in your favor.
    #[must_use]
    pub fn is_positive(&self) -> bool {
        self.prob_points > 0.0
    }

    /// How much the ratio measure overstates a longshot relative to points.
    ///
    /// Near 1.0 for even-money prices and climbing steeply as the price
    /// lengthens — this ratio *is* the misconception, quantified.
    #[must_use]
    pub fn ratio_to_points_distortion(&self) -> f64 {
        if self.prob_points.abs() < f64::EPSILON {
            return f64::NAN;
        }
        self.ratio / self.prob_points
    }
}

/// Computes closing line value.
///
/// `opposing_close_decimal` is the closing price on the *other* side of the
/// market. Supply it and the vig can be removed, making [`Clv::ev_vs_fair`]
/// meaningful; omit it and only the inflated figure is available.
///
/// # Errors
///
/// [`MathError::DomainError`] if any price is not above 1.0.
pub fn clv(
    bet_decimal: f64,
    close_decimal: f64,
    opposing_close_decimal: Option<f64>,
) -> Result<Clv> {
    let bet_implied = decimal_to_implied(bet_decimal)?;
    let close_implied = decimal_to_implied(close_decimal)?;

    let (fair_prob, ev_vs_fair) = match opposing_close_decimal {
        Some(other) => {
            let other_implied = decimal_to_implied(other)?;
            let total = close_implied + other_implied;
            if total <= 0.0 {
                return Err(MathError::DomainError {
                    param: "closing market total",
                    constraint: "positive",
                    value: total,
                });
            }
            let fair = close_implied / total;
            (Some(fair), Some(fair * bet_decimal - 1.0))
        }
        None => (None, None),
    };

    Ok(Clv {
        bet_decimal,
        close_decimal,
        bet_implied,
        close_implied,
        ratio: bet_decimal / close_decimal - 1.0,
        prob_points: close_implied - bet_implied,
        cents: crate::odds::to_american(bet_decimal)? - crate::odds::to_american(close_decimal)?,
        ev_vs_raw_close: close_implied * bet_decimal - 1.0,
        ev_vs_fair,
        fair_prob,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::odds::american_to_decimal;
    use approx::assert_relative_eq;

    fn at(bet: f64, close: f64) -> Clv {
        clv(
            american_to_decimal(bet).unwrap(),
            american_to_decimal(close).unwrap(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn the_three_ts_rows_really_were_one_number() {
        // Documents the bug rather than reproducing it: ratio and the old EV
        // are the same expression, which is why they must not both be shown.
        let c = at(400.0, 350.0);
        assert_relative_eq!(c.ratio, c.ev_vs_raw_close, epsilon = 1e-12);
    }

    #[test]
    fn ratio_and_points_disagree_about_which_bet_was_better() {
        // The headline: the ratio measure ranks these backwards.
        let favorite = at(-110.0, -130.0);
        let longshot = at(400.0, 350.0);

        assert!(longshot.ratio > favorite.ratio, "ratio should favor the longshot");
        assert!(
            longshot.prob_points < favorite.prob_points,
            "but in probability points the favorite gained more"
        );

        assert_relative_eq!(favorite.prob_points, 0.0414, epsilon = 5e-4);
        assert_relative_eq!(longshot.prob_points, 0.0222, epsilon = 5e-4);
    }

    #[test]
    fn fifty_cents_on_a_longshot_beats_twenty_on_a_favorite_only_in_cents() {
        let favorite = at(-110.0, -130.0);
        let longshot = at(400.0, 350.0);
        assert_eq!(favorite.cents, 20);
        assert_eq!(longshot.cents, 50);
        // Two and a half times the cents, barely half the actual value.
        assert!(longshot.prob_points < favorite.prob_points);
    }

    #[test]
    fn distortion_grows_with_price() {
        let short = at(-200.0, -220.0).ratio_to_points_distortion();
        let long = at(1000.0, 900.0).ratio_to_points_distortion();
        assert!(long > short, "distortion should worsen at longer prices");
    }

    #[test]
    fn no_movement_is_zero_by_every_measure() {
        let c = at(-110.0, -110.0);
        assert_relative_eq!(c.ratio, 0.0, epsilon = 1e-12);
        assert_relative_eq!(c.prob_points, 0.0, epsilon = 1e-12);
        assert_eq!(c.cents, 0);
        assert!(!c.is_positive());
    }

    #[test]
    fn ev_against_the_raw_close_hides_the_vig_you_paid() {
        // Bet -110 into a market that closed -110/-110. The raw-close measure
        // reports exactly break-even, because betting the closing price always
        // does. But that close carries 4.5% hold: the fair price was even
        // money, so this bet is -4.5% EV. The TS only ever showed the zero.
        let d = american_to_decimal(-110.0).unwrap();
        let c = clv(d, d, Some(d)).unwrap();

        assert_relative_eq!(c.ev_vs_raw_close, 0.0, epsilon = 1e-12);
        assert_relative_eq!(c.fair_prob.unwrap(), 0.5, epsilon = 1e-12);
        assert_relative_eq!(c.ev_vs_fair.unwrap(), -0.0454545, epsilon = 1e-6);

        // The gap between the two is precisely the vig, and it is never small.
        let gap = c.ev_vs_raw_close - c.ev_vs_fair.unwrap();
        assert!(gap > 0.045, "raw EV overstates fair EV by {gap}");
    }

    #[test]
    fn devigging_requires_the_other_side() {
        let c = at(400.0, 350.0);
        assert!(c.ev_vs_fair.is_none());
        assert!(c.fair_prob.is_none());
    }

    #[test]
    fn beating_the_close_shows_as_positive_points() {
        let c = at(150.0, 120.0);
        assert!(c.is_positive());
        assert!(c.prob_points > 0.0);
    }

    #[test]
    fn invalid_prices_are_rejected() {
        assert!(clv(1.0, 2.0, None).is_err());
        assert!(clv(2.0, 0.5, None).is_err());
        assert!(clv(2.0, 2.0, Some(1.0)).is_err());
    }
}
