//! Backing a true line out of a posted price, and pricing alternates from it.
//!
//! Ported from `bestline.ts` and `altline.ts` — separate files in the TS, but
//! `altline` is built entirely on `bestline`'s inversion, so they share a
//! module here.
//!
//! The model is normal: a spread's margin is `N(true_line, std²)`, so a posted
//! line and price pin down where the market thinks the true line sits.
//!
//! # Divergences from the TypeScript
//!
//! **The vig is removed before inverting.** `impliedTrueLine` fed
//! `americanToImplied(odds)` straight into `Φ⁻¹`, using a probability that
//! still contained the book's margin. For a team at -10.5 priced -110 into a
//! -110/-110 market, the fair cover probability is 0.50 and the true line is
//! -10.5 — but the TS computed `Φ⁻¹(0.5238) = 0.0597` and reported
//! `-10.5 − 13.86 × 0.0597 = -11.33`. Eight tenths of a point of pure vig,
//! reported as market information.
//!
//! The error cancels when comparing two books at identical prices, which is
//! presumably why it survived, but every *line value* the app displayed was
//! shifted, and the alternate-line ladder inherited the shift wholesale.
//!
//! [`implied_true_line`] therefore takes a **fair** cover probability. Use
//! [`fair_cover_prob`] to devig a two-sided market first.

use crate::odds::american_to_implied;
use crate::probability::{inverse_normal_cdf, normal_cdf};
use crate::{MathError, Result};
use serde::{Deserialize, Serialize};

/// Which kind of line is being modelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum BetType {
    /// A point spread.
    Spread,
    /// A total, with a side.
    Total(TotalSide),
}

/// Which side of a total.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum TotalSide {
    /// Betting the game goes over.
    Over,
    /// Betting the game stays under.
    Under,
}

/// Removes the vig from a two-sided market, returning the fair probability of
/// the first side.
///
/// # Errors
///
/// [`MathError::DomainError`] if either price is not a valid American quote.
pub fn fair_cover_prob(american: f64, opposing_american: f64) -> Result<f64> {
    let a = american_to_implied(american)?;
    let b = american_to_implied(opposing_american)?;
    let total = a + b;
    if total <= 0.0 {
        return Err(MathError::DomainError {
            param: "market total",
            constraint: "positive",
            value: total,
        });
    }
    Ok(a / total)
}

fn check_std(std_dev: f64) -> Result<()> {
    if !std_dev.is_finite() || std_dev <= 0.0 {
        return Err(MathError::DomainError {
            param: "std_dev",
            constraint: "greater than zero",
            value: std_dev,
        });
    }
    Ok(())
}

/// Backs the market's true line out of a posted line and a **fair** cover
/// probability.
///
/// Pass a devigged probability — see [`fair_cover_prob`]. Feeding a raw price
/// in here is the bug this module exists to document.
///
/// # Errors
///
/// [`MathError::ProbabilityOutOfRange`] if the probability is not in `(0, 1)`,
/// [`MathError::DomainError`] for a non-positive standard deviation.
pub fn implied_true_line(
    line: f64,
    fair_cover_prob: f64,
    std_dev: f64,
    bet_type: BetType,
) -> Result<f64> {
    check_std(std_dev)?;
    if !fair_cover_prob.is_finite() || fair_cover_prob <= 0.0 || fair_cover_prob >= 1.0 {
        return Err(MathError::ProbabilityOutOfRange {
            value: fair_cover_prob,
            reason: "cover probability must be strictly between 0 and 1",
        });
    }
    let z = inverse_normal_cdf(fair_cover_prob);
    Ok(match bet_type {
        BetType::Spread | BetType::Total(TotalSide::Under) => line - std_dev * z,
        BetType::Total(TotalSide::Over) => line + std_dev * z,
    })
}

/// Fair probability of covering an alternate line, given the true line.
#[must_use]
pub fn fair_prob_at_line(true_line: f64, alt_line: f64, std_dev: f64, bet_type: BetType) -> f64 {
    let z = (alt_line - true_line) / std_dev;
    match bet_type {
        BetType::Spread | BetType::Total(TotalSide::Under) => normal_cdf(z),
        BetType::Total(TotalSide::Over) => 1.0 - normal_cdf(z),
    }
}

/// One rung of an alternate-line ladder.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LadderRow {
    /// The alternate line.
    pub line: f64,
    /// Fair probability of covering it, 0–1.
    pub fair_prob: f64,
    /// The American price that probability implies.
    pub fair_odds: i32,
}

/// A priced ladder of alternate lines.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Ladder {
    /// The true line backed out of the main market.
    pub true_line: f64,
    /// One row per alternate, cheapest to longest.
    pub rows: Vec<LadderRow>,
    /// Rows omitted because the probability was beyond `[0.005, 0.995]`.
    ///
    /// The TS dropped these silently.
    pub omitted: usize,
}

/// Prices every alternate line around a main line.
///
/// # Errors
///
/// [`MathError::DomainError`] for a non-positive step, standard deviation, or
/// range, or a ladder longer than 1000 rows.
pub fn generate_ladder(
    main_line: f64,
    fair_cover_prob: f64,
    std_dev: f64,
    bet_type: BetType,
    range: f64,
    step: f64,
) -> Result<Ladder> {
    let true_line = implied_true_line(main_line, fair_cover_prob, std_dev, bet_type)?;
    for (param, value) in [("range", range), ("step", step)] {
        if !value.is_finite() || value <= 0.0 {
            return Err(MathError::DomainError {
                param,
                constraint: "greater than zero",
                value,
            });
        }
    }
    let steps = (2.0 * range / step).floor();
    if steps > 1000.0 {
        return Err(MathError::DomainError {
            param: "range / step",
            constraint: "at most 1000 rows",
            value: steps,
        });
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bounded at 1000 immediately above"
    )]
    let count = steps as usize;

    let mut rows = Vec::new();
    let mut omitted = 0;
    for i in 0..=count {
        #[allow(clippy::cast_precision_loss, reason = "index bounded at 1000")]
        let offset = i as f64 * step;
        // Round to a tenth: lines are quoted in half and whole points, and
        // accumulating `+= step` in a float drifts off them.
        let line = ((main_line - range + offset) * 10.0).round() / 10.0;
        let fair_prob = fair_prob_at_line(true_line, line, std_dev, bet_type);
        if fair_prob <= 0.005 || fair_prob >= 0.995 {
            omitted += 1;
            continue;
        }
        rows.push(LadderRow {
            line,
            fair_prob,
            fair_odds: crate::odds::implied_to_american(fair_prob),
        });
    }

    Ok(Ladder {
        true_line,
        rows,
        omitted,
    })
}

/// Which of two competing lines is better, and by how much.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LineComparison {
    /// True line implied by the first quote.
    pub true_line_a: f64,
    /// True line implied by the second quote.
    pub true_line_b: f64,
    /// Which is better: `None` for a tie.
    pub better: Option<Side>,
    /// Distance between the two implied true lines, in points.
    pub diff: f64,
}

/// Identifies one of two compared quotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Side {
    /// The first quote.
    A,
    /// The second quote.
    B,
}

/// Compares two line-and-price quotes on the same market.
///
/// Both cover probabilities must already be devigged.
///
/// # Errors
///
/// See [`implied_true_line`].
pub fn compare_lines(
    line_a: f64,
    fair_prob_a: f64,
    line_b: f64,
    fair_prob_b: f64,
    std_dev: f64,
    bet_type: BetType,
) -> Result<LineComparison> {
    let true_line_a = implied_true_line(line_a, fair_prob_a, std_dev, bet_type)?;
    let true_line_b = implied_true_line(line_b, fair_prob_b, std_dev, bet_type)?;

    // For spreads and unders a higher implied line is the better deal; for
    // overs a lower one is.
    let a_wins = match bet_type {
        BetType::Spread | BetType::Total(TotalSide::Under) => true_line_a > true_line_b,
        BetType::Total(TotalSide::Over) => true_line_a < true_line_b,
    };
    let tied = (true_line_a - true_line_b).abs() < 1e-12;

    Ok(LineComparison {
        true_line_a,
        true_line_b,
        better: if tied {
            None
        } else if a_wins {
            Some(Side::A)
        } else {
            Some(Side::B)
        },
        diff: (true_line_a - true_line_b).abs(),
    })
}

/// The score each side is projected for, given a spread and a total.
///
/// Solves the pair `home + away = total`, `away - home = spread` — the spread
/// is signed from the home team's perspective, so `-3` means home is favoured
/// by three.
///
/// # Errors
///
/// [`MathError::DomainError`] if `total` is not positive, or if the spread is
/// wide enough to project a negative score. The TS guarded the total but not
/// the spread, so a -60 spread on a 40-point total projected the away team for
/// -10 points.
pub fn implied_scores(spread: f64, total: f64) -> Result<ImpliedScores> {
    if total <= 0.0 {
        return Err(MathError::DomainError {
            param: "total",
            constraint: "greater than zero",
            value: total,
        });
    }

    let home = (total - spread) / 2.0;
    let away = (total + spread) / 2.0;

    if home < 0.0 || away < 0.0 {
        return Err(MathError::DomainError {
            param: "spread",
            constraint: "narrow enough that neither side projects below zero",
            value: spread,
        });
    }

    Ok(ImpliedScores { home, away })
}

/// The projected score line behind a spread and total.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ImpliedScores {
    /// Points projected for the home team.
    pub home: f64,
    /// Points projected for the away team.
    pub away: f64,
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
    use approx::assert_relative_eq;

    const NFL_STD: f64 = 13.86;

    #[test]
    fn implied_scores_split_the_total_around_the_spread() {
        let scores = implied_scores(-3.0, 47.5).unwrap();
        assert_relative_eq!(scores.home, 25.25, epsilon = 1e-12);
        assert_relative_eq!(scores.away, 22.25, epsilon = 1e-12);
        assert_relative_eq!(scores.home + scores.away, 47.5, epsilon = 1e-12);
        assert_relative_eq!(scores.away - scores.home, -3.0, epsilon = 1e-12);
    }

    #[test]
    fn a_pickem_splits_the_total_evenly() {
        let scores = implied_scores(0.0, 44.0).unwrap();
        assert_relative_eq!(scores.home, 22.0, epsilon = 1e-12);
        assert_relative_eq!(scores.away, 22.0, epsilon = 1e-12);
    }

    #[test]
    fn a_negative_projected_score_is_an_error() {
        // The TS returned -10 here and rendered it as an implied score.
        assert!(implied_scores(-60.0, 40.0).is_err());
        assert!(implied_scores(0.0, -1.0).is_err());
    }

    #[test]
    fn a_fair_market_implies_the_posted_line_exactly() {
        // The bug: the TS reported -11.33 for this, off by 0.83 points of vig.
        let fair = fair_cover_prob(-110.0, -110.0).unwrap();
        assert_relative_eq!(fair, 0.5, epsilon = 1e-12);
        let true_line = implied_true_line(-10.5, fair, NFL_STD, BetType::Spread).unwrap();
        assert_relative_eq!(true_line, -10.5, epsilon = 1e-9);
    }

    #[test]
    fn the_ts_bias_is_the_size_we_claimed() {
        // Reproduces the old behavior by feeding the raw, vigged probability.
        let raw = american_to_implied(-110.0).unwrap();
        let biased = implied_true_line(-10.5, raw, NFL_STD, BetType::Spread).unwrap();
        assert_relative_eq!(biased, -11.327, epsilon = 1e-2);
        assert!((biased - (-10.5)).abs() > 0.8);
    }

    #[test]
    fn a_juiced_side_implies_a_line_beyond_the_posted_one() {
        // -10.5 at -130 into a -130/+110 market: the market really likes them.
        let fair = fair_cover_prob(-130.0, 110.0).unwrap();
        assert!(fair > 0.5);
        let true_line = implied_true_line(-10.5, fair, NFL_STD, BetType::Spread).unwrap();
        assert!(true_line < -10.5, "got {true_line}");
    }

    #[test]
    fn inversion_round_trips_against_the_forward_model() {
        for prob in [0.35, 0.45, 0.5, 0.55, 0.65] {
            let true_line = implied_true_line(-7.0, prob, NFL_STD, BetType::Spread).unwrap();
            let back = fair_prob_at_line(true_line, -7.0, NFL_STD, BetType::Spread);
            assert_relative_eq!(back, prob, epsilon = 1e-6);
        }
    }

    #[test]
    fn easier_spreads_are_more_likely_to_cover() {
        let true_line = -7.0;
        let easy = fair_prob_at_line(true_line, -3.5, NFL_STD, BetType::Spread);
        let hard = fair_prob_at_line(true_line, -10.5, NFL_STD, BetType::Spread);
        assert!(easy > hard);
        assert!(easy > 0.5 && hard < 0.5);
    }

    #[test]
    fn overs_and_unders_move_in_opposite_directions() {
        let over = BetType::Total(TotalSide::Over);
        let under = BetType::Total(TotalSide::Under);
        let low = fair_prob_at_line(45.0, 41.5, 10.0, over);
        let high = fair_prob_at_line(45.0, 48.5, 10.0, over);
        assert!(low > high, "a lower total is easier to go over");
        // The two sides of a line must complement.
        let o = fair_prob_at_line(45.0, 47.5, 10.0, over);
        let u = fair_prob_at_line(45.0, 47.5, 10.0, under);
        assert_relative_eq!(o + u, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn a_ladder_is_monotonic_in_the_line() {
        let fair = fair_cover_prob(-110.0, -110.0).unwrap();
        let ladder = generate_ladder(-7.0, fair, NFL_STD, BetType::Spread, 5.0, 0.5).unwrap();
        assert_relative_eq!(ladder.true_line, -7.0, epsilon = 1e-9);
        assert!(!ladder.rows.is_empty());
        for w in ladder.rows.windows(2) {
            assert!(
                w[1].fair_prob >= w[0].fair_prob,
                "cover probability fell from {:?} to {:?}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn the_ladder_prices_the_main_line_at_the_input_probability() {
        let fair = fair_cover_prob(-110.0, -110.0).unwrap();
        let ladder = generate_ladder(-7.0, fair, NFL_STD, BetType::Spread, 5.0, 0.5).unwrap();
        let main = ladder
            .rows
            .iter()
            .find(|r| (r.line - (-7.0)).abs() < 1e-9)
            .expect("the main line should appear as a rung of its own ladder");
        assert_relative_eq!(main.fair_prob, 0.5, epsilon = 1e-6);
        assert_eq!(main.fair_odds, -100);
    }

    #[test]
    fn extreme_rungs_are_counted_not_silently_dropped() {
        let fair = fair_cover_prob(-110.0, -110.0).unwrap();
        let wide = generate_ladder(-7.0, fair, 3.0, BetType::Spread, 20.0, 0.5).unwrap();
        assert!(wide.omitted > 0, "far rungs should be omitted and counted");
    }

    #[test]
    fn comparison_prefers_the_better_number() {
        let fair = fair_cover_prob(-110.0, -110.0).unwrap();
        // -3.5 is better than -7.5 for a spread backer at the same price.
        let c = compare_lines(-3.5, fair, -7.5, fair, NFL_STD, BetType::Spread).unwrap();
        assert_eq!(c.better, Some(Side::A));
        assert_relative_eq!(c.diff, 4.0, epsilon = 1e-9);
    }

    #[test]
    fn comparison_flips_for_overs() {
        let fair = fair_cover_prob(-110.0, -110.0).unwrap();
        // A lower total is better if you are backing the over.
        let over = BetType::Total(TotalSide::Over);
        let c = compare_lines(44.5, fair, 47.5, fair, 10.0, over).unwrap();
        assert_eq!(c.better, Some(Side::A));
    }

    #[test]
    fn identical_quotes_tie() {
        let fair = fair_cover_prob(-110.0, -110.0).unwrap();
        let c = compare_lines(-7.0, fair, -7.0, fair, NFL_STD, BetType::Spread).unwrap();
        assert_eq!(c.better, None);
        assert_relative_eq!(c.diff, 0.0, epsilon = 1e-12);
    }

    #[test]
    fn a_better_price_can_beat_a_better_number() {
        // -7.0 at +100 versus -6.5 at -130: the extra half point costs too much.
        let cheap = fair_cover_prob(100.0, -120.0).unwrap();
        let pricey = fair_cover_prob(-130.0, 110.0).unwrap();
        let c = compare_lines(-7.0, cheap, -6.5, pricey, NFL_STD, BetType::Spread).unwrap();
        assert_eq!(c.better, Some(Side::A), "comparison was {c:?}");
    }

    #[test]
    fn invalid_input_is_rejected() {
        assert!(implied_true_line(-7.0, 0.0, NFL_STD, BetType::Spread).is_err());
        assert!(implied_true_line(-7.0, 1.0, NFL_STD, BetType::Spread).is_err());
        assert!(implied_true_line(-7.0, 0.5, 0.0, BetType::Spread).is_err());
        let fair = fair_cover_prob(-110.0, -110.0).unwrap();
        assert!(generate_ladder(-7.0, fair, NFL_STD, BetType::Spread, 5.0, 0.0).is_err());
        assert!(generate_ladder(-7.0, fair, NFL_STD, BetType::Spread, 5_000.0, 0.5).is_err());
    }
}
