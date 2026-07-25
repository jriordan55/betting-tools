//! Teasers — buying points across several legs at a single price.
//!
//! Ported from `bettor-calculator-main/src/lib/math/teaserEv.ts`.
//!
//! # Divergences from the TypeScript
//!
//! 1. **An empty teaser is rejected.** `analyzeTeaser([])` reduced over no legs
//!    with an initial value of 1, so the combined probability came out to a
//!    certainty and the function reported an enormous positive EV for a bet
//!    with nothing in it. Fewer than two legs is now an error.
//! 2. **The vig is removed before inverting.** Inherited from `bestline.ts` —
//!    see [`crate::line`].
//! 3. **EV is a fraction, not a percentage**, matching the rest of the crate.
//!
//! # The limitation that matters most
//!
//! Cover probabilities come from a **normal** model, which has no idea that
//! football margins pile up on specific numbers. Roughly 15% of NFL games end
//! with a margin of exactly 3 and around 9% with exactly 7; a normal with
//! σ ≈ 13.9 puts under 3% on each. Crossing those numbers is the entire
//! premise of a Wong teaser, so this model **understates** the value of the
//! bets teaser players actually make.
//!
//! The TypeScript computed and displayed `keyNumbersCrossed` alongside a
//! probability that ignored them, which reads as though the crossing had been
//! priced in. [`TeaserLegAnalysis::key_numbers_crossed`] is still reported —
//! it is genuinely useful — but [`Teaser::model_ignores_key_numbers`] is
//! returned beside it so the caller can say so plainly in the UI.
//!
//! Pricing the discrete mass properly needs an empirical margin distribution
//! per sport, which is a data problem rather than a math one and belongs in
//! its own change.

use crate::line::{fair_prob_at_line, implied_true_line, BetType};
use crate::odds::{decimal_to_implied, implied_to_american};
use crate::{MathError, Result};
use serde::Serialize;

/// Margins that football games land on far more often than a normal predicts.
pub const FOOTBALL_KEY_NUMBERS: [f64; 4] = [3.0, 7.0, 10.0, 14.0];

/// One leg of a teaser, as posted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TeaserLeg {
    /// The spread before teasing, in the bettor's favour when positive.
    pub spread: f64,
    /// Fair (devigged) probability of covering that spread — see
    /// [`crate::line::fair_cover_prob`].
    pub fair_cover_prob: f64,
}

/// One leg, priced after the points are applied.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeaserLegAnalysis {
    /// The spread as posted.
    pub spread: f64,
    /// The spread after the teaser points are added.
    pub teased_spread: f64,
    /// True line backed out of the posted spread and its fair price.
    pub true_line: f64,
    /// Probability of covering the teased spread, 0–1.
    pub cover_prob: f64,
    /// The American price that probability implies.
    pub fair_odds: i32,
    /// Key numbers the tease moves through.
    ///
    /// Reported for judgement, **not** priced in — see the module docs.
    pub key_numbers_crossed: Vec<f64>,
}

/// A complete teaser, priced.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Teaser {
    /// Per-leg analysis, in the order supplied.
    pub legs: Vec<TeaserLegAnalysis>,
    /// Probability every leg covers, assuming independence.
    pub combined_prob: f64,
    /// American price that probability implies.
    pub fair_odds: i32,
    /// Win rate needed to break even at the price offered, 0–1.
    pub breakeven: f64,
    /// Expected value as a fraction of stake.
    pub ev_fraction: f64,
    /// True when any leg crosses a key number, meaning the normal model is
    /// understating this teaser's true value.
    pub model_ignores_key_numbers: bool,
}

impl Teaser {
    /// Whether the teaser prices as positive expectation.
    ///
    /// Read alongside [`Self::model_ignores_key_numbers`]: a teaser that
    /// crosses 3 and 7 is worth more than this figure says.
    #[must_use]
    pub fn is_positive_ev(&self) -> bool {
        self.ev_fraction > 0.0
    }
}

/// Key numbers strictly between a spread and its teased version.
#[must_use]
pub fn key_numbers_crossed(spread: f64, teased_spread: f64) -> Vec<f64> {
    let lo = spread.min(teased_spread);
    let hi = spread.max(teased_spread);
    FOOTBALL_KEY_NUMBERS
        .into_iter()
        .filter(|k| (lo < -k && -k < hi) || (lo < *k && *k < hi))
        .collect()
}

/// Prices a single teaser leg.
///
/// # Errors
///
/// See [`crate::line::implied_true_line`].
pub fn analyze_leg(leg: TeaserLeg, teaser_points: f64, std_dev: f64) -> Result<TeaserLegAnalysis> {
    let true_line = implied_true_line(leg.spread, leg.fair_cover_prob, std_dev, BetType::Spread)?;
    let teased_spread = leg.spread + teaser_points;
    let cover_prob = fair_prob_at_line(true_line, teased_spread, std_dev, BetType::Spread);

    Ok(TeaserLegAnalysis {
        spread: leg.spread,
        teased_spread,
        true_line,
        cover_prob,
        fair_odds: implied_to_american(cover_prob),
        key_numbers_crossed: key_numbers_crossed(leg.spread, teased_spread),
    })
}

/// Prices a full teaser against the book's offered price.
///
/// # Errors
///
/// [`MathError::ShapeError`] for fewer than two legs,
/// [`MathError::DomainError`] for non-positive teaser points, a non-positive
/// standard deviation, or a price at or below 1.0.
pub fn analyze_teaser(
    legs: &[TeaserLeg],
    teaser_points: f64,
    teaser_decimal_odds: f64,
    std_dev: f64,
) -> Result<Teaser> {
    if legs.len() < 2 {
        // The TS reduced over an empty list with an initial value of 1, so a
        // teaser with no legs reported a certainty and a vast positive EV.
        return Err(MathError::ShapeError {
            what: "teaser legs",
            expected: "at least 2",
            got: legs.len(),
        });
    }
    if !teaser_points.is_finite() || teaser_points <= 0.0 {
        return Err(MathError::DomainError {
            param: "teaser_points",
            constraint: "greater than zero",
            value: teaser_points,
        });
    }
    let breakeven = decimal_to_implied(teaser_decimal_odds)?;

    let analyses: Vec<TeaserLegAnalysis> = legs
        .iter()
        .map(|leg| analyze_leg(*leg, teaser_points, std_dev))
        .collect::<Result<_>>()?;

    let combined_prob: f64 = analyses.iter().map(|l| l.cover_prob).product();
    let model_ignores_key_numbers = analyses.iter().any(|l| !l.key_numbers_crossed.is_empty());

    Ok(Teaser {
        combined_prob,
        fair_odds: implied_to_american(combined_prob),
        breakeven,
        ev_fraction: combined_prob * teaser_decimal_odds - 1.0,
        model_ignores_key_numbers,
        legs: analyses,
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

    fn leg(spread: f64) -> TeaserLeg {
        TeaserLeg {
            spread,
            fair_cover_prob: fair_cover_prob(-110.0, -110.0).unwrap(),
        }
    }

    #[test]
    fn the_classic_wong_teaser_crosses_three_and_seven() {
        // -7.5 teased six points to -1.5 moves through both key numbers.
        let a = analyze_leg(leg(-7.5), 6.0, NFL_STD).unwrap();
        assert_relative_eq!(a.teased_spread, -1.5, epsilon = 1e-12);
        assert_eq!(a.key_numbers_crossed, vec![3.0, 7.0]);
    }

    #[test]
    fn the_underdog_side_crosses_them_too() {
        // +1.5 teased six points to +7.5.
        let a = analyze_leg(leg(1.5), 6.0, NFL_STD).unwrap();
        assert_relative_eq!(a.teased_spread, 7.5, epsilon = 1e-12);
        assert_eq!(a.key_numbers_crossed, vec![3.0, 7.0]);
    }

    #[test]
    fn teasing_through_no_key_numbers_reports_none() {
        // -11.5 to -8.5 skips 10? No — it crosses it. Use -13.5 to -10.5.
        let a = analyze_leg(leg(-13.5), 2.0, NFL_STD).unwrap();
        assert!(a.key_numbers_crossed.is_empty(), "got {:?}", a.key_numbers_crossed);
    }

    #[test]
    fn buying_points_always_raises_the_cover_probability() {
        let plain = analyze_leg(leg(-7.5), 0.5, NFL_STD).unwrap();
        let teased = analyze_leg(leg(-7.5), 6.0, NFL_STD).unwrap();
        assert!(teased.cover_prob > plain.cover_prob);
        // A -1.5 favorite should be a clear favorite to cover.
        assert!(teased.cover_prob > 0.6, "got {}", teased.cover_prob);
    }

    #[test]
    fn a_fair_priced_spread_implies_its_own_line() {
        let a = analyze_leg(leg(-7.5), 6.0, NFL_STD).unwrap();
        assert_relative_eq!(a.true_line, -7.5, epsilon = 1e-9);
    }

    #[test]
    fn combined_probability_is_the_product_of_the_legs() {
        let legs = [leg(-7.5), leg(-8.0), leg(1.5)];
        let t = analyze_teaser(&legs, 6.0, american_to_decimal(-120.0).unwrap(), NFL_STD).unwrap();
        let expected: f64 = t.legs.iter().map(|l| l.cover_prob).product();
        assert_relative_eq!(t.combined_prob, expected, epsilon = 1e-12);
        assert!(t.combined_prob < t.legs[0].cover_prob, "more legs is harder");
    }

    #[test]
    fn the_normal_model_prices_a_wong_teaser_below_its_historical_hit_rate() {
        // -7.5 and -8.5 teased six points, the textbook two-team Wong.
        let legs = [leg(-7.5), leg(-8.5)];
        let t = analyze_teaser(&legs, 6.0, american_to_decimal(-110.0).unwrap(), NFL_STD).unwrap();

        // Each leg prices at 66.8% under a normal, and the pair at 44.6%.
        assert_relative_eq!(t.legs[0].cover_prob, 0.667_5, epsilon = 1e-3);
        assert_relative_eq!(t.combined_prob, 0.445_5, epsilon = 1e-3);

        // Which is below the -110 breakeven, so the model calls this -EV...
        assert_relative_eq!(t.breakeven, 0.523_809_5, epsilon = 1e-6);
        assert!(!t.is_positive_ev());

        // ...while flagging that it is crossing 3 and 7, where the normal is
        // known to understate. Wong legs are commonly reported hitting in the
        // low seventies, which would put the pair above breakeven. The model
        // cannot see that, and says so rather than pretending otherwise.
        assert!(t.model_ignores_key_numbers);
        assert_eq!(t.legs[0].key_numbers_crossed, vec![3.0, 7.0]);
    }

    #[test]
    fn ev_is_zero_when_the_price_matches_the_fair_probability() {
        let legs = [leg(-7.5), leg(-8.5)];
        let fair_price = {
            let t = analyze_teaser(&legs, 6.0, 2.0, NFL_STD).unwrap();
            1.0 / t.combined_prob
        };
        let t = analyze_teaser(&legs, 6.0, fair_price, NFL_STD).unwrap();
        assert_relative_eq!(t.ev_fraction, 0.0, epsilon = 1e-12);
        assert!(!t.is_positive_ev());
    }

    #[test]
    fn the_key_number_caveat_is_surfaced() {
        // Crossing 3 and 7 means the normal model is understating this teaser.
        let crossing = analyze_teaser(
            &[leg(-7.5), leg(1.5)],
            6.0,
            american_to_decimal(-120.0).unwrap(),
            NFL_STD,
        )
        .unwrap();
        assert!(crossing.model_ignores_key_numbers);

        let not_crossing = analyze_teaser(
            &[leg(-13.5), leg(-13.5)],
            2.0,
            american_to_decimal(-120.0).unwrap(),
            NFL_STD,
        )
        .unwrap();
        assert!(!not_crossing.model_ignores_key_numbers);
    }

    #[test]
    fn an_empty_teaser_is_rejected() {
        // The TS reported a certainty and a vast positive EV for this.
        let price = american_to_decimal(-110.0).unwrap();
        assert!(matches!(
            analyze_teaser(&[], 6.0, price, NFL_STD),
            Err(MathError::ShapeError { .. })
        ));
        assert!(analyze_teaser(&[leg(-7.5)], 6.0, price, NFL_STD).is_err());
    }

    #[test]
    fn invalid_input_is_rejected() {
        let legs = [leg(-7.5), leg(-8.5)];
        let price = american_to_decimal(-110.0).unwrap();
        assert!(analyze_teaser(&legs, 0.0, price, NFL_STD).is_err());
        assert!(analyze_teaser(&legs, -6.0, price, NFL_STD).is_err());
        assert!(analyze_teaser(&legs, 6.0, 1.0, NFL_STD).is_err());
        assert!(analyze_teaser(&legs, 6.0, price, 0.0).is_err());
    }
}
