//! Parlay pricing.
//!
//! Extracted from `ParlayCalculator.tsx`.
//!
//! # Divergences from the TypeScript
//!
//! **Invalid legs are no longer silently dropped.** The TSX filtered out any
//! leg that failed to parse and priced the parlay from whatever survived:
//!
//! ```js
//! const decimalOdds = allOdds.filter((d) => d !== null)
//! if (decimalOdds.length < 2) return null
//! ```
//!
//! Enter five legs, fat-finger one, and it quoted a four-leg parlay — a
//! different bet, at a much shorter price, presented as yours. A `skippedLegs`
//! count appeared elsewhere in the UI, but the headline payout was simply
//! wrong. Here a bad leg is an error.
//!
//! # A caveat the arithmetic cannot express
//!
//! Multiplying decimal prices assumes the legs are **independent**. Same-game
//! legs rarely are — a quarterback's passing yards and his receiver's are
//! strongly correlated, and books price that in. Treat this as an upper bound
//! on a same-game parlay's true value.

use crate::odds::decimal_to_implied;
use crate::{MathError, Result};
use serde::Serialize;

/// A priced parlay.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parlay {
    /// Combined decimal price — the product of every leg.
    pub decimal: f64,
    /// Combined probability implied, 0–1, assuming independence.
    pub implied_prob: f64,
    /// Return if every leg lands, stake included.
    pub payout: f64,
    /// Return minus stake.
    pub profit: f64,
    /// How many legs were priced.
    pub leg_count: usize,
}

/// Prices a parlay from its legs.
///
/// # Errors
///
/// [`MathError::ShapeError`] for fewer than two legs,
/// [`MathError::DomainError`] for a non-positive stake or any leg at or below 1.0.
/// Note that an unparseable leg is the caller's error to surface *before*
/// calling this — every leg here must already be a valid decimal price.
pub fn parlay(legs: &[f64], stake: f64) -> Result<Parlay> {
    if legs.len() < 2 {
        return Err(MathError::ShapeError {
            what: "legs",
            expected: "at least 2",
            got: legs.len(),
        });
    }
    if !stake.is_finite() || stake <= 0.0 {
        return Err(MathError::DomainError {
            param: "stake",
            constraint: "greater than zero",
            value: stake,
        });
    }

    let mut decimal = 1.0_f64;
    for leg in legs {
        // Rejects any leg at or below 1.0 rather than dropping it.
        decimal_to_implied(*leg)?;
        decimal *= leg;
    }
    if !decimal.is_finite() {
        return Err(MathError::DomainError {
            param: "combined price",
            constraint: "finite (too many long legs to represent)",
            value: decimal,
        });
    }

    let payout = stake * decimal;
    Ok(Parlay {
        decimal,
        implied_prob: 1.0 / decimal,
        payout,
        profit: payout - stake,
        leg_count: legs.len(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::odds::american_to_decimal;
    use approx::assert_relative_eq;

    #[test]
    fn two_even_money_legs_pay_three_to_one() {
        let p = parlay(&[2.0, 2.0], 100.0).unwrap();
        assert_relative_eq!(p.decimal, 4.0, epsilon = 1e-12);
        assert_relative_eq!(p.payout, 400.0, epsilon = 1e-9);
        assert_relative_eq!(p.profit, 300.0, epsilon = 1e-9);
        assert_relative_eq!(p.implied_prob, 0.25, epsilon = 1e-12);
    }

    #[test]
    fn implied_probability_is_the_product_of_the_legs() {
        let legs = [1.91, 1.91, 2.5];
        let p = parlay(&legs, 100.0).unwrap();
        let expected: f64 = legs.iter().map(|d| 1.0 / d).product();
        assert_relative_eq!(p.implied_prob, expected, epsilon = 1e-12);
    }

    #[test]
    fn a_bad_leg_is_an_error_not_a_silent_drop() {
        // The TS priced this as a two-leg parlay and reported the payout as if
        // it were the bet the user entered.
        let d = american_to_decimal(-110.0).unwrap();
        assert!(parlay(&[d, d, 1.0], 100.0).is_err());
        assert!(parlay(&[d, d, 0.5], 100.0).is_err());
    }

    #[test]
    fn dropping_a_leg_would_have_changed_the_price_substantially() {
        // Quantifies what the silent filter cost: a leg is worth ~2x here.
        let d = american_to_decimal(-110.0).unwrap();
        let three = parlay(&[d, d, d], 100.0).unwrap();
        let two = parlay(&[d, d], 100.0).unwrap();
        assert!(three.payout > 1.8 * two.payout);
    }

    #[test]
    fn standard_three_leg_parlay_is_about_plus_six_hundred() {
        let d = american_to_decimal(-110.0).unwrap();
        let p = parlay(&[d, d, d], 100.0).unwrap();
        // 1.909091³ — about +596 in American terms.
        assert_relative_eq!(p.decimal, 6.957_926, epsilon = 1e-5);
        assert_eq!(crate::odds::to_american(p.decimal).unwrap(), 596);
    }

    #[test]
    fn rejects_too_few_legs_and_bad_stakes() {
        assert!(parlay(&[2.0], 100.0).is_err());
        assert!(parlay(&[], 100.0).is_err());
        assert!(parlay(&[2.0, 2.0], 0.0).is_err());
        assert!(parlay(&[2.0, 2.0], -50.0).is_err());
    }

    #[test]
    fn a_long_parlay_still_produces_a_finite_price() {
        let legs = vec![11.0; 20];
        let p = parlay(&legs, 1.0).unwrap();
        assert!(p.decimal.is_finite());
        assert!(p.implied_prob > 0.0);
    }
}
