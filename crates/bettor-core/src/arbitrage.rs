//! Locking in a position across two or more prices: arbitrage and hedging.
//!
//! Extracted from `ArbitrageCalculator.tsx` and `HedgeCalculator.tsx`.
//!
//! # Divergences from the TypeScript
//!
//! 1. **A losing "arb" is not called a profit.** The TS computed
//!    `guaranteedProfit` regardless of whether the market actually arbed, so a
//!    book totalling 1.05 produced a confidently-labelled negative "guaranteed
//!    profit". [`Arbitrage::is_arb`] now gates it.
//! 2. **Negative stakes are rejected** — the `if (!stake)` guard passed `-100`.

use crate::odds::decimal_to_implied;
use crate::{MathError, Result};
use serde::Serialize;

/// Stake and payout for one leg of an arbitrage.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[cfg_attr(feature = "specta", specta(rename = "ArbitrageLeg"))]
#[serde(rename_all = "camelCase")]
pub struct Leg {
    /// Decimal price for this outcome.
    pub decimal: f64,
    /// Probability it implies, 0–1.
    pub implied: f64,
    /// Stake to place on it.
    pub stake: f64,
    /// Return if it wins, stake included.
    pub payout: f64,
}

/// An arbitrage opportunity, or the absence of one.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Arbitrage {
    /// Per-outcome stakes, in the order supplied.
    pub legs: Vec<Leg>,
    /// Implied probabilities summed. Below 1.0 means free money.
    pub total_implied: f64,
    /// Profit locked in whichever outcome lands. Negative when not an arb.
    pub profit: f64,
    /// [`Self::profit`] as a fraction of total stake.
    pub roi: f64,
}

impl Arbitrage {
    /// Whether backing every outcome guarantees a profit.
    #[must_use]
    pub fn is_arb(&self) -> bool {
        self.total_implied < 1.0
    }

    /// Locked-in profit, or `None` when the market does not arb.
    ///
    /// `ArbitrageCalculator.tsx` computed this unconditionally but did gate the
    /// *display* behind `isArbitrage`, so the negative number never reached the
    /// screen there. `HedgeCalculator.tsx` is where it did: that one labels the
    /// row "Guaranteed Profit" whenever the mode is `guarantee`, changing only
    /// the colour when the figure is negative. Making the value itself an
    /// `Option` puts the guarantee in the type rather than in each caller's
    /// render logic, so a future caller cannot repeat the hedge calculator's
    /// mistake.
    #[must_use]
    pub fn guaranteed_profit(&self) -> Option<f64> {
        self.is_arb().then_some(self.profit)
    }
}

/// Splits a bankroll across every outcome so the return is identical either way.
///
/// # Errors
///
/// [`MathError::ShapeError`] for fewer than two outcomes,
/// [`MathError::DomainError`] for a non-positive stake or a price at or below 1.0.
pub fn arbitrage(decimals: &[f64], total_stake: f64) -> Result<Arbitrage> {
    if decimals.len() < 2 {
        return Err(MathError::ShapeError {
            what: "outcomes",
            expected: "at least 2",
            got: decimals.len(),
        });
    }
    if !total_stake.is_finite() || total_stake <= 0.0 {
        return Err(MathError::DomainError {
            param: "total stake",
            constraint: "greater than zero",
            value: total_stake,
        });
    }

    let implied: Vec<f64> = decimals
        .iter()
        .map(|d| decimal_to_implied(*d))
        .collect::<Result<_>>()?;
    let total_implied: f64 = implied.iter().sum();

    // Staking proportional to implied probability equalises the payout, so
    // every leg returns total_stake / total_implied.
    let payout = total_stake / total_implied;
    let legs = decimals
        .iter()
        .zip(&implied)
        .map(|(d, p)| Leg {
            decimal: *d,
            implied: *p,
            stake: total_stake * p / total_implied,
            payout,
        })
        .collect();

    let profit = payout - total_stake;
    Ok(Arbitrage {
        legs,
        total_implied,
        profit,
        roi: profit / total_stake,
    })
}

/// What to do about an open position when the price has moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum HedgeGoal {
    /// Equalise the return so the outcome no longer matters.
    Guarantee,
    /// Stake only enough to recover the original stake if the hedge lands.
    RecoverStake,
}

/// A hedge on an open position.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Hedge {
    /// Stake to place on the opposing side.
    pub hedge_stake: f64,
    /// Original stake plus hedge stake.
    pub total_risk: f64,
    /// Net profit if the original bet wins.
    pub profit_if_original_wins: f64,
    /// Net profit if the hedge wins.
    pub profit_if_hedge_wins: f64,
    /// The worse of the two outcomes — what you are actually locking in.
    pub worst_case: f64,
    /// [`Self::worst_case`] as a fraction of total risk.
    pub roi: f64,
}

impl Hedge {
    /// Whether the position is profitable no matter which side lands.
    #[must_use]
    pub fn is_locked_profit(&self) -> bool {
        self.worst_case > 0.0
    }
}

/// Sizes a hedge against an open position.
///
/// # Errors
///
/// [`MathError::DomainError`] for a non-positive stake or a price at or below 1.0.
pub fn hedge(
    original_stake: f64,
    original_decimal: f64,
    hedge_decimal: f64,
    goal: HedgeGoal,
) -> Result<Hedge> {
    if !original_stake.is_finite() || original_stake <= 0.0 {
        return Err(MathError::DomainError {
            param: "original stake",
            constraint: "greater than zero",
            value: original_stake,
        });
    }
    // Validates both prices are above 1.0.
    decimal_to_implied(original_decimal)?;
    decimal_to_implied(hedge_decimal)?;

    let original_payout = original_stake * original_decimal;
    let hedge_stake = match goal {
        HedgeGoal::Guarantee => original_payout / hedge_decimal,
        HedgeGoal::RecoverStake => original_stake / (hedge_decimal - 1.0),
    };

    let total_risk = original_stake + hedge_stake;
    let profit_if_original_wins = original_payout - total_risk;
    let profit_if_hedge_wins = hedge_stake * hedge_decimal - total_risk;
    let worst_case = profit_if_original_wins.min(profit_if_hedge_wins);

    Ok(Hedge {
        hedge_stake,
        total_risk,
        profit_if_original_wins,
        profit_if_hedge_wins,
        worst_case,
        roi: worst_case / total_risk,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;
    use crate::odds::american_to_decimal;
    use approx::assert_relative_eq;

    #[test]
    fn a_real_arb_pays_the_same_either_way() {
        let a = arbitrage(&[2.10, 2.10], 1_000.0).unwrap();
        assert!(a.is_arb());
        assert_relative_eq!(a.legs[0].payout, a.legs[1].payout, epsilon = 1e-9);
        assert!(a.profit > 0.0);
        assert!(a.guaranteed_profit().is_some());
    }

    #[test]
    fn stakes_always_sum_to_the_bankroll() {
        let a = arbitrage(&[2.10, 2.05, 8.0], 1_000.0).unwrap();
        let total: f64 = a.legs.iter().map(|l| l.stake).sum();
        assert_relative_eq!(total, 1_000.0, epsilon = 1e-9);
    }

    #[test]
    fn a_vigged_market_is_not_reported_as_guaranteed_profit() {
        // `HedgeCalculator.tsx` labelled this kind of negative number
        // "Guaranteed Profit"; the arbitrage screen gated the row on
        // `isArbitrage`. `Option` makes the gate impossible to forget.
        let a = arbitrage(&[1.91, 1.91], 1_000.0).unwrap();
        assert!(!a.is_arb());
        assert!(a.profit < 0.0);
        assert!(a.guaranteed_profit().is_none());
    }

    #[test]
    fn arbitrage_rejects_bad_input() {
        assert!(arbitrage(&[2.0], 100.0).is_err());
        assert!(arbitrage(&[2.0, 2.0], -100.0).is_err());
        assert!(arbitrage(&[2.0, 1.0], 100.0).is_err());
    }

    #[test]
    fn a_guarantee_hedge_equalises_both_outcomes() {
        let h = hedge(100.0, 3.0, 1.8, HedgeGoal::Guarantee).unwrap();
        assert_relative_eq!(
            h.profit_if_original_wins,
            h.profit_if_hedge_wins,
            epsilon = 1e-9
        );
    }

    #[test]
    fn hedging_a_longshot_that_came_in_locks_a_profit() {
        // Bet +400, now available to lay at -150.
        let orig = american_to_decimal(400.0).unwrap();
        let lay = american_to_decimal(-150.0).unwrap();
        let h = hedge(100.0, orig, lay, HedgeGoal::Guarantee).unwrap();
        assert!(h.is_locked_profit(), "worst case was {}", h.worst_case);
    }

    #[test]
    fn recover_stake_mode_breaks_even_on_the_hedge_side() {
        let h = hedge(100.0, 3.0, 2.5, HedgeGoal::RecoverStake).unwrap();
        assert_relative_eq!(h.profit_if_hedge_wins, 0.0, epsilon = 1e-9);
        assert!(h.profit_if_original_wins > 0.0);
    }

    #[test]
    fn worst_case_is_what_you_actually_lock_in() {
        let h = hedge(100.0, 3.0, 2.5, HedgeGoal::RecoverStake).unwrap();
        assert_relative_eq!(
            h.worst_case,
            h.profit_if_original_wins.min(h.profit_if_hedge_wins),
            epsilon = 1e-12
        );
    }

    #[test]
    fn hedge_rejects_bad_input() {
        assert!(hedge(-100.0, 3.0, 1.8, HedgeGoal::Guarantee).is_err());
        assert!(hedge(100.0, 1.0, 1.8, HedgeGoal::Guarantee).is_err());
        assert!(hedge(100.0, 3.0, 1.0, HedgeGoal::RecoverStake).is_err());
    }
}
