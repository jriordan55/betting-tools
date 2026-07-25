//! A book's hold on a two-sided market, and the same computation across books.
//!
//! Ported from `hold.ts` and `vigComparison.ts`.
//!
//! # Divergences from the TypeScript
//!
//! 1. **Hold is a fraction, not "percentage points".** The TS returned
//!    `holdPct: 4.76` from a module whose every other field was 0–1. Here it is
//!    `0.0476` and the UI multiplies. Mixed scales in one struct are how a
//!    hold gets displayed as 476%.
//! 2. **Books that sum below 1.0 are kept, not silently dropped.** The TS
//!    filter removed non-positive entries but happily reported a *negative*
//!    hold for an arb without comment. A negative hold is real and worth
//!    seeing — it is free money — so it is flagged rather than hidden.

use crate::{MathError, Result};
use serde::Serialize;

/// What a book is charging on a two-sided market.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hold {
    /// Implied probability of side A, as priced.
    pub implied_a: f64,
    /// Implied probability of side B, as priced.
    pub implied_b: f64,
    /// The two implied probabilities added together. Above 1.0 for a real book.
    pub total_implied: f64,
    /// The overround as a fraction of the book: `total − 1`.
    ///
    /// Negative means the two sides can be backed together for a guaranteed
    /// profit. See [`Hold::is_arbitrage`].
    pub hold: f64,
    /// Fair probability of side A once the margin is removed proportionally.
    pub no_vig_prob_a: f64,
    /// Fair probability of side B once the margin is removed proportionally.
    pub no_vig_prob_b: f64,
}

impl Hold {
    /// Whether the two sides can be backed together for a guaranteed profit.
    #[must_use]
    pub fn is_arbitrage(&self) -> bool {
        self.total_implied < 1.0
    }

    /// Break-even win rate needed on side A at the price offered.
    ///
    /// This is just `implied_a`, named for what it means to a bettor.
    #[must_use]
    pub const fn breakeven_a(&self) -> f64 {
        self.implied_a
    }
}

/// Computes the hold on a two-sided market.
///
/// # Errors
///
/// [`MathError::ProbabilityOutOfRange`] if either side is not in `(0, 1)`.
pub fn calculate_hold(implied_a: f64, implied_b: f64) -> Result<Hold> {
    for value in [implied_a, implied_b] {
        if !value.is_finite() || value <= 0.0 || value >= 1.0 {
            return Err(MathError::ProbabilityOutOfRange {
                value,
                reason: "each side must imply a probability strictly between 0 and 1",
            });
        }
    }
    let total_implied = implied_a + implied_b;
    Ok(Hold {
        implied_a,
        implied_b,
        total_implied,
        hold: total_implied - 1.0,
        no_vig_prob_a: implied_a / total_implied,
        no_vig_prob_b: implied_b / total_implied,
    })
}

/// One book's quote on a shared market.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookQuote {
    /// Book name, carried through to the result for display.
    pub name: String,
    /// The hold that book is charging.
    pub hold: Hold,
}

/// Ranks books on the same market by hold, cheapest first.
///
/// Quotes that fail validation are returned as errors alongside their book
/// name rather than being dropped: a book vanishing from a comparison table
/// with no explanation is worse than one showing why it could not be parsed.
#[must_use]
pub fn compare_vig(books: &[(String, f64, f64)]) -> Vec<(String, Result<Hold>)> {
    let mut rows: Vec<(String, Result<Hold>)> = books
        .iter()
        .map(|(name, a, b)| (name.clone(), calculate_hold(*a, *b)))
        .collect();

    // Valid quotes sort by ascending hold; failures sink to the bottom in the
    // order they were entered. `total_cmp` avoids the partial-ordering dance
    // and is total even in the presence of NaN.
    rows.sort_by(|left, right| match (&left.1, &right.1) {
        (Ok(a), Ok(b)) => a.hold.total_cmp(&b.hold),
        (Ok(_), Err(_)) => core::cmp::Ordering::Less,
        (Err(_), Ok(_)) => core::cmp::Ordering::Greater,
        (Err(_), Err(_)) => core::cmp::Ordering::Equal,
    });
    rows
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn standard_minus_110_both_sides_holds_about_four_and_a_half_percent() {
        let h = calculate_hold(0.523_809_5, 0.523_809_5).unwrap();
        assert_relative_eq!(h.hold, 0.047_619, epsilon = 1e-6);
        assert_relative_eq!(h.no_vig_prob_a, 0.5, epsilon = 1e-9);
        assert!(!h.is_arbitrage());
    }

    #[test]
    fn no_vig_probabilities_always_sum_to_one() {
        let h = calculate_hold(0.9091, 0.1667).unwrap();
        assert_relative_eq!(h.no_vig_prob_a + h.no_vig_prob_b, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn a_book_summing_below_one_is_flagged_as_an_arb() {
        let h = calculate_hold(0.48, 0.48).unwrap();
        assert!(h.is_arbitrage());
        assert!(h.hold < 0.0);
    }

    #[test]
    fn hold_is_a_fraction_not_a_percentage() {
        // TS returned 4.76 here; a UI multiplying by 100 would show 476%.
        let h = calculate_hold(0.523_809_5, 0.523_809_5).unwrap();
        assert!(h.hold < 1.0);
    }

    #[test]
    fn invalid_sides_are_rejected() {
        assert!(calculate_hold(0.0, 0.5).is_err());
        assert!(calculate_hold(0.5, 1.0).is_err());
        assert!(calculate_hold(-0.1, 0.5).is_err());
        assert!(calculate_hold(f64::NAN, 0.5).is_err());
    }

    #[test]
    fn comparison_sorts_cheapest_first_and_keeps_bad_rows_visible() {
        let books = vec![
            ("Expensive".to_owned(), 0.55, 0.55),
            ("Cheap".to_owned(), 0.5155, 0.4975),
            ("Broken".to_owned(), 0.0, 0.5),
        ];
        let ranked = compare_vig(&books);
        assert_eq!(ranked[0].0, "Cheap");
        assert_eq!(ranked[1].0, "Expensive");
        assert_eq!(ranked[2].0, "Broken");
        assert!(ranked[2].1.is_err());
        assert_eq!(ranked.len(), 3, "no book should silently disappear");
    }
}
