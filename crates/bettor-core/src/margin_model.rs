//! Scoring as a distribution over integers, graded against a spread and a total.
//!
//! Ported from `~/Code/probabilites-main/betting-probability-viz.html`, which
//! is a standalone visualizer rather than part of the calculator app — so
//! there are no golden vectors here, and the normal CDF used is this crate's
//! own rather than the visualizer's `erf`. Internal consistency beats matching
//! a page nothing else in the app talks to.
//!
//! [`crate::match_model`] already covers Poisson and negative-binomial
//! scoring, which fit low-scoring sports. This module covers the other case:
//! **football and basketball**, where scores are high enough that a normal is
//! the right shape but must still be evaluated on integers, because a spread
//! of exactly 3 is graded against a margin of exactly 3.
//!
//! # What the derivation buys
//!
//! A book prices two things about a game: how far apart the teams finish (the
//! spread) and how much they score together (the total). Those imply different
//! standard deviations — around 13.5 and 10.2 points in the NFL — and the two
//! together pin down both a per-team standard deviation and the correlation
//! between the teams' scores:
//!
//! ```text
//! σ_team = √((σ_margin² + σ_total²) / 4)
//! ρ      = (σ_total² − σ_margin²) / (σ_total² + σ_margin²)
//! ```
//!
//! Football comes out at ρ ≈ −0.27 — teams' scores are *anti*-correlated, which
//! is game script — and basketball at ρ ≈ +0.31, where pace is shared. Neither
//! number is an input anywhere; both fall out of two lines a book already
//! posts.
//!
//! # Key numbers
//!
//! Football margins are not smoothly distributed. They pile up on 3 and 7, and
//! a normal cannot see it. [`KEY_NUMBER_WEIGHTS`] reweights the margin
//! distribution to match the observed pile-ups and renormalises.
//!
//! This is the empirical distribution that [`crate::teaser`] says it does not
//! have. It is now available — but it is a table of weights, not a fitted
//! model, and it is one sport's. See [`KEY_NUMBER_WEIGHTS`] for what that means
//! before relying on it to price anything.

use crate::probability::normal_cdf;
use crate::{MathError, Result};
use serde::{Deserialize, Serialize};

/// One integer outcome and its probability.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PmfPoint {
    /// The score, margin, or total.
    pub k: i32,
    /// Probability of exactly that value, 0–1.
    pub p: f64,
}

/// Relative weight of each absolute football margin against a smooth normal.
///
/// From the reference visualizer, where it is described as empirical American
/// football pile-ups. Read it as a shape correction rather than as a measured
/// distribution: it is not sourced to a season range, it is one sport's, and
/// applying it to basketball or soccer would be nonsense.
///
/// The two that matter are 3 (weight 2.35) and 7 (1.72) — field goal and
/// touchdown-plus-extra-point. Zero is weighted almost out of existence, since
/// regulation ties are vanishingly rare in a sport that plays overtime.
///
/// Indexed by `|margin|`; anything not listed weighs 1.
pub const KEY_NUMBER_WEIGHTS: &[(i32, f64)] = &[
    (0, 0.06),
    (1, 1.00),
    (2, 0.92),
    (3, 2.35),
    (4, 0.95),
    (5, 0.86),
    (6, 1.10),
    (7, 1.72),
    (8, 0.90),
    (9, 0.84),
    (10, 1.35),
    (11, 0.80),
    (12, 0.70),
    (13, 0.80),
    (14, 1.22),
    (15, 0.85),
    (16, 0.86),
    (17, 1.12),
    (18, 0.76),
    (19, 0.82),
    (20, 0.95),
    (21, 1.06),
    (22, 0.82),
    (23, 0.84),
    (24, 0.92),
    (25, 0.80),
    (27, 0.86),
    (28, 0.88),
    (31, 0.86),
    (35, 0.85),
];

/// The weight for one absolute margin, defaulting to 1.
#[must_use]
pub fn key_number_weight(abs_margin: i32) -> f64 {
    KEY_NUMBER_WEIGHTS
        .iter()
        .find(|(k, _)| *k == abs_margin)
        .map_or(1.0, |(_, w)| *w)
}

/// A game, described the way a book prices one.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GameShape {
    /// Points the home team is expected to score.
    pub mu_home: f64,
    /// Points the away team is expected to score.
    pub mu_away: f64,
    /// Standard deviation of the margin.
    pub sd_margin: f64,
    /// Standard deviation of the combined score.
    pub sd_total: f64,
    /// Whether to reweight margins onto football's key numbers.
    ///
    /// Only meaningful for American football. See [`KEY_NUMBER_WEIGHTS`].
    pub key_numbers: bool,
}

/// A scoring model, as distributions over integers.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MarginModel {
    /// Home score distribution.
    pub home_scores: Vec<PmfPoint>,
    /// Away score distribution.
    pub away_scores: Vec<PmfPoint>,
    /// Margin distribution, home minus away.
    pub margin: Vec<PmfPoint>,
    /// Combined score distribution.
    pub total: Vec<PmfPoint>,

    /// Standard deviation of the margin, as supplied.
    pub sd_margin: f64,
    /// Standard deviation of the total, as supplied.
    pub sd_total: f64,
    /// Per-team standard deviation implied by the two above.
    pub sd_team: f64,
    /// Correlation between the two teams' scores, implied by the two above.
    ///
    /// Negative means game script pulls the scores apart — one team runs the
    /// clock out while the other throws. Positive means shared pace.
    pub implied_correlation: f64,

    /// Mean margin of the distribution, after any key-number reweighting.
    pub mean_margin: f64,
    /// Mean total.
    pub mean_total: f64,
}

/// How a spread grades against a margin distribution.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SpreadGrade {
    /// The line, from the home team's perspective. `-3.5` means laying 3.5.
    pub spread: f64,
    /// Probability the home side covers, 0–1.
    pub home_covers: f64,
    /// Probability of a push, 0–1. Always zero on a half-point line.
    pub push: f64,
    /// Probability the away side covers, 0–1.
    pub away_covers: f64,
}

/// How a total grades.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TotalGrade {
    /// The line.
    pub total: f64,
    /// Probability of the over, 0–1.
    pub over: f64,
    /// Probability of a push, 0–1.
    pub push: f64,
    /// Probability of the under, 0–1.
    pub under: f64,
}

/// A discretised normal over `lo..=hi`, renormalised to sum to 1.
///
/// Each integer takes the mass of the half-open interval around it, which is
/// what makes a push at an integer line a real event rather than a measure-zero
/// one.
fn normal_integer_pmf(mu: f64, sd: f64, lo: i32, hi: i32) -> Result<Vec<PmfPoint>> {
    if !sd.is_finite() || sd <= 0.0 {
        return Err(MathError::DomainError {
            param: "standard deviation",
            constraint: "greater than zero",
            value: sd,
        });
    }
    if hi < lo {
        return Err(MathError::ShapeError {
            what: "score range",
            expected: "a non-empty range",
            got: 0,
        });
    }

    let mut points: Vec<PmfPoint> = (lo..=hi)
        .map(|k| {
            let x = f64::from(k);
            let upper = normal_cdf((x + 0.5 - mu) / sd);
            let lower = normal_cdf((x - 0.5 - mu) / sd);
            PmfPoint {
                k,
                p: (upper - lower).max(0.0),
            }
        })
        .collect();

    // Truncating the tails drops real mass, and every derived price would be
    // biased low by exactly that amount. Same correction `match_model` applies
    // to its score grid, and for the same reason.
    let mass: f64 = points.iter().map(|p| p.p).sum();
    if mass <= 0.0 {
        return Err(MathError::DomainError {
            param: "score range",
            constraint: "wide enough to hold some probability",
            value: mass,
        });
    }
    for point in &mut points {
        point.p /= mass;
    }
    Ok(points)
}

/// How many standard deviations of tail to keep on each side.
///
/// Beyond four, the mass being renormalised away is under 1e-4 and the extra
/// bins are all but empty.
const TAIL_SIGMAS: f64 = 4.2;

/// Range wide enough to hold a distribution, as integers.
#[allow(
    clippy::cast_possible_truncation,
    reason = "scores and margins are far inside i32 at any real mu and sd"
)]
fn span(mu: f64, sd: f64, floor_at_zero: bool) -> (i32, i32) {
    let lo = (mu - TAIL_SIGMAS * sd).floor();
    let hi = (mu + TAIL_SIGMAS * sd).ceil();
    let lo = if floor_at_zero { lo.max(0.0) } else { lo };
    (lo as i32, hi as i32)
}

fn check_shape(shape: &GameShape) -> Result<()> {
    for (param, value) in [
        ("home mean score", shape.mu_home),
        ("away mean score", shape.mu_away),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(MathError::DomainError {
                param,
                constraint: "zero or greater",
                value,
            });
        }
    }
    for (param, value) in [
        ("margin standard deviation", shape.sd_margin),
        ("total standard deviation", shape.sd_total),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(MathError::DomainError {
                param,
                constraint: "greater than zero",
                value,
            });
        }
    }
    Ok(())
}

/// Builds the scoring model a spread and a total imply.
///
/// # Errors
///
/// [`MathError::DomainError`] for a negative mean score or a non-positive
/// standard deviation.
pub fn normal_game(shape: &GameShape) -> Result<MarginModel> {
    check_shape(shape)?;

    // Var(H − A) = 2σ² (1 − ρ) and Var(H + A) = 2σ² (1 + ρ). Adding them
    // eliminates ρ and gives the per-team variance; their difference over
    // their sum gives ρ back. Two posted lines, two latent parameters.
    let var_margin = shape.sd_margin * shape.sd_margin;
    let var_total = shape.sd_total * shape.sd_total;
    let sd_team = ((var_margin + var_total) / 4.0).sqrt();
    let implied_correlation = (var_total - var_margin) / (var_total + var_margin);

    let (home_lo, home_hi) = span(shape.mu_home, sd_team, true);
    let (away_lo, away_hi) = span(shape.mu_away, sd_team, true);
    let home_scores = normal_integer_pmf(shape.mu_home, sd_team, home_lo, home_hi)?;
    let away_scores = normal_integer_pmf(shape.mu_away, sd_team, away_lo, away_hi)?;

    let mean_margin_in = shape.mu_home - shape.mu_away;
    let (margin_lo, margin_hi) = span(mean_margin_in, shape.sd_margin, false);
    let mut margin = normal_integer_pmf(mean_margin_in, shape.sd_margin, margin_lo, margin_hi)?;

    if shape.key_numbers {
        for point in &mut margin {
            point.p *= key_number_weight(point.k.abs());
        }
        let mass: f64 = margin.iter().map(|p| p.p).sum();
        if mass <= 0.0 {
            return Err(MathError::DomainError {
                param: "key-number weighting",
                constraint: "must leave some probability",
                value: mass,
            });
        }
        for point in &mut margin {
            point.p /= mass;
        }
    }

    let mean_total_in = shape.mu_home + shape.mu_away;
    let (total_lo, total_hi) = span(mean_total_in, shape.sd_total, true);
    let total = normal_integer_pmf(mean_total_in, shape.sd_total, total_lo, total_hi)?;

    Ok(MarginModel {
        // Reweighting moves the mean, so it is measured from the distribution
        // rather than echoed back from the input.
        mean_margin: margin.iter().map(|p| f64::from(p.k) * p.p).sum(),
        mean_total: total.iter().map(|p| f64::from(p.k) * p.p).sum(),
        home_scores,
        away_scores,
        margin,
        total,
        sd_margin: shape.sd_margin,
        sd_total: shape.sd_total,
        sd_team,
        implied_correlation,
    })
}

/// Sums the mass above, at, and below a threshold.
fn split_at(pmf: &[PmfPoint], threshold: f64) -> (f64, f64, f64) {
    let mut above = 0.0;
    let mut at = 0.0;
    let mut below = 0.0;
    for point in pmf {
        let k = f64::from(point.k);
        if k > threshold {
            above += point.p;
        } else if (k - threshold).abs() < 1e-9 {
            at += point.p;
        } else {
            below += point.p;
        }
    }
    (above, at, below)
}

impl MarginModel {
    /// Grades a spread against the margin distribution.
    ///
    /// `spread` is the home side's line: `-3.5` means the home team is laying
    /// 3.5 points and covers when the margin exceeds 3.5.
    #[must_use]
    pub fn grade_spread(&self, spread: f64) -> SpreadGrade {
        // The home side covers when margin > −spread. Folding the line onto
        // the margin axis before comparing is what stops the two sign
        // conventions for a spread meeting inside one function — the bug
        // `middle.rs` exists to make unrepeatable.
        let threshold = -spread;
        let (home_covers, push, away_covers) = split_at(&self.margin, threshold);
        SpreadGrade {
            spread,
            home_covers,
            push,
            away_covers,
        }
    }

    /// Grades a total.
    #[must_use]
    pub fn grade_total(&self, total: f64) -> TotalGrade {
        let (over, push, under) = split_at(&self.total, total);
        TotalGrade {
            total,
            over,
            push,
            under,
        }
    }

    /// Probability the home team simply wins, draws, and loses.
    ///
    /// A draw is only possible in sports that allow one; in football and
    /// basketball this is the probability of a regulation tie, which is what
    /// the margin distribution actually describes.
    #[must_use]
    pub fn moneyline(&self) -> Moneyline {
        let (home, draw, away) = split_at(&self.margin, 0.0);
        Moneyline { home, draw, away }
    }
}

/// The three-way market a margin distribution implies.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Moneyline {
    /// Probability the home team wins outright, 0–1.
    pub home: f64,
    /// Probability of a level score, 0–1.
    ///
    /// In football and basketball this is a regulation tie, not a settled
    /// draw — those sports play overtime. In soccer it is the draw itself.
    pub draw: f64,
    /// Probability the away team wins outright, 0–1.
    pub away: f64,
}

/// One rung of a spread-to-probability curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CurvePoint {
    /// The line.
    pub spread: f64,
    /// Probability the home side covers it, 0–1.
    pub home_covers: f64,
    /// Probability of a push at this line, 0–1.
    pub push: f64,
}

/// The whole spread-to-probability curve for one model.
///
/// The point of drawing it: on a smooth normal the curve is smooth, and with
/// key numbers on it has visible steps at 3 and 7. Those steps are why moving
/// a football line off a key number costs far more than moving it the same
/// half point anywhere else.
///
/// # Errors
///
/// [`MathError::DomainError`] for a non-positive step or a reversed range,
/// [`MathError::ShapeError`] if the range would produce more than 400 rungs.
pub fn cover_curve(
    model: &MarginModel,
    from_spread: f64,
    to_spread: f64,
    step: f64,
) -> Result<Vec<CurvePoint>> {
    if !step.is_finite() || step <= 0.0 {
        return Err(MathError::DomainError {
            param: "step",
            constraint: "greater than zero",
            value: step,
        });
    }
    if !(from_spread.is_finite() && to_spread.is_finite()) || to_spread <= from_spread {
        return Err(MathError::DomainError {
            param: "spread range",
            constraint: "the second line must be above the first",
            value: to_spread - from_spread,
        });
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "non-negative by the check above, and capped on the next line"
    )]
    let rungs = ((to_spread - from_spread) / step).floor() as usize + 1;
    if rungs > 400 {
        return Err(MathError::ShapeError {
            what: "curve points",
            expected: "at most 400",
            got: rungs,
        });
    }

    Ok((0..rungs)
        .map(|i| {
            #[allow(clippy::cast_precision_loss, reason = "rung index, capped at 400")]
            let spread = from_spread + i as f64 * step;
            let grade = model.grade_spread(spread);
            CurvePoint {
                spread,
                home_covers: grade.home_covers,
                push: grade.push,
            }
        })
        .collect())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::float_cmp,
    reason = "test code"
)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    /// A typical NFL game: home favored by 3.5, total 45.5.
    fn nfl(key_numbers: bool) -> GameShape {
        GameShape {
            mu_home: 24.5,
            mu_away: 21.0,
            sd_margin: 13.5,
            sd_total: 10.2,
            key_numbers,
        }
    }

    fn nba() -> GameShape {
        GameShape {
            mu_home: 114.5,
            mu_away: 110.0,
            sd_margin: 11.5,
            sd_total: 15.9,
            key_numbers: false,
        }
    }

    fn mass_at(pmf: &[PmfPoint], k: i32) -> f64 {
        pmf.iter().find(|p| p.k == k).map_or(0.0, |p| p.p)
    }

    #[test]
    fn every_distribution_sums_to_one() {
        for shape in [nfl(false), nfl(true), nba()] {
            let m = normal_game(&shape).unwrap();
            for (name, pmf) in [
                ("home", &m.home_scores),
                ("away", &m.away_scores),
                ("margin", &m.margin),
                ("total", &m.total),
            ] {
                let mass: f64 = pmf.iter().map(|p| p.p).sum();
                assert_relative_eq!(mass, 1.0, epsilon = 1e-12);
                assert!(pmf.iter().all(|p| p.p >= 0.0), "{name} went negative");
            }
        }
    }

    #[test]
    fn two_posted_lines_pin_down_the_team_sd_and_the_correlation() {
        // The derivation this module exists for. Neither figure is an input.
        let football = normal_game(&nfl(false)).unwrap();
        assert_relative_eq!(football.sd_team, 8.460, epsilon = 5e-3);
        assert_relative_eq!(football.implied_correlation, -0.273, epsilon = 5e-3);

        let basketball = normal_game(&nba()).unwrap();
        assert_relative_eq!(basketball.sd_team, 9.812, epsilon = 5e-3);
        assert_relative_eq!(basketball.implied_correlation, 0.313, epsilon = 5e-3);
    }

    #[test]
    fn the_correlation_sign_follows_which_line_is_more_volatile() {
        // Equal spread and total volatility means independent teams.
        let neutral = normal_game(&GameShape {
            sd_margin: 12.0,
            sd_total: 12.0,
            ..nfl(false)
        })
        .unwrap();
        assert_relative_eq!(neutral.implied_correlation, 0.0, epsilon = 1e-12);
        assert_relative_eq!(neutral.sd_team, 12.0 / 2.0_f64.sqrt(), epsilon = 1e-9);
    }

    #[test]
    fn a_half_point_line_has_no_push_and_the_two_sides_sum_to_one() {
        let m = normal_game(&nfl(true)).unwrap();
        let g = m.grade_spread(-3.5);
        assert_relative_eq!(g.push, 0.0, epsilon = 1e-12);
        assert_relative_eq!(g.home_covers + g.away_covers, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn a_whole_number_line_has_a_real_push_and_three_sides_sum_to_one() {
        // The reason the model is evaluated on integers at all.
        let m = normal_game(&nfl(true)).unwrap();
        let g = m.grade_spread(-3.0);
        assert!(g.push > 0.05, "a 3 push should be common, got {}", g.push);
        assert_relative_eq!(g.home_covers + g.push + g.away_covers, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn a_pick_em_is_a_coin_flip_and_a_favorite_is_not() {
        let even = normal_game(&GameShape {
            mu_home: 22.0,
            mu_away: 22.0,
            ..nfl(false)
        })
        .unwrap();
        let g = even.grade_spread(0.0);
        assert_relative_eq!(g.home_covers, g.away_covers, epsilon = 1e-9);

        let favored = normal_game(&nfl(false)).unwrap();
        assert!(favored.grade_spread(0.0).home_covers > 0.5);
    }

    #[test]
    fn laying_more_points_is_always_harder() {
        let m = normal_game(&nfl(true)).unwrap();
        let mut previous = 1.0;
        for tenths in 0..60 {
            #[allow(clippy::cast_precision_loss, reason = "loop index")]
            let spread = -(tenths as f64) * 0.5;
            let covers = m.grade_spread(spread).home_covers;
            assert!(covers <= previous + 1e-12, "covering rose at {spread}");
            previous = covers;
        }
    }

    // --- key numbers -------------------------------------------------------

    #[test]
    fn key_numbers_put_real_mass_on_three_and_seven() {
        // The claim `teaser.rs` says the app cannot make. Measured, not assumed:
        // with the weights on, a 3-point margin carries 12.7% against the ~15%
        // commonly quoted for the NFL, and a 7 carries 8.4% against ~9%. Far
        // closer than a smooth normal manages; still not the observed rates.
        // This is a shape correction, not a fitted margin distribution.
        let smooth = normal_game(&nfl(false)).unwrap();
        let keyed = normal_game(&nfl(true)).unwrap();

        let smooth_3 = mass_at(&smooth.margin, 3) + mass_at(&smooth.margin, -3);
        let keyed_3 = mass_at(&keyed.margin, 3) + mass_at(&keyed.margin, -3);
        assert_relative_eq!(keyed_3, 0.127, epsilon = 5e-3);
        assert!(keyed_3 > 2.0 * smooth_3);
        assert!(keyed_3 < 0.15, "the table does not reach the observed rate");

        // 8.4% at a 7, against the ~9% commonly quoted: the table lands much
        // closer here than it does at 3.
        let keyed_7 = mass_at(&keyed.margin, 7) + mass_at(&keyed.margin, -7);
        assert_relative_eq!(keyed_7, 0.084, epsilon = 5e-3);
        assert!(keyed_7 > mass_at(&smooth.margin, 7) + mass_at(&smooth.margin, -7));
    }

    #[test]
    fn key_numbers_all_but_eliminate_the_regulation_tie() {
        // Weight 0.06 at zero: a sport that plays overtime rarely ends level.
        let keyed = normal_game(&nfl(true)).unwrap();
        let smooth = normal_game(&nfl(false)).unwrap();
        assert!(mass_at(&keyed.margin, 0) < 0.1 * mass_at(&smooth.margin, 0));
    }

    #[test]
    fn reweighting_pulls_the_mean_margin_toward_zero() {
        // A caveat, not a bug, and the reason `mean_margin` is measured from
        // the distribution rather than echoed back from the input. The weights
        // are applied to |margin| and are heaviest near the middle, so on a
        // distribution that is not centred at zero they pull the mean in: a
        // game set up at -3.5 prices out around -3.2 once key numbers are on.
        let smooth = normal_game(&nfl(false)).unwrap();
        let keyed = normal_game(&nfl(true)).unwrap();

        assert_relative_eq!(smooth.mean_margin, 3.5, epsilon = 0.05);
        assert_relative_eq!(keyed.mean_margin, 3.196, epsilon = 5e-3);
        assert!(keyed.mean_margin < smooth.mean_margin);
    }

    #[test]
    fn an_unlisted_margin_weighs_one() {
        assert_relative_eq!(key_number_weight(3), 2.35, epsilon = 1e-12);
        assert_relative_eq!(key_number_weight(26), 1.0, epsilon = 1e-12);
        assert_relative_eq!(key_number_weight(500), 1.0, epsilon = 1e-12);
    }

    // --- totals and moneyline ---------------------------------------------

    #[test]
    fn a_total_grades_the_way_a_spread_does() {
        let m = normal_game(&nfl(false)).unwrap();
        let half = m.grade_total(45.5);
        assert_relative_eq!(half.push, 0.0, epsilon = 1e-12);
        assert_relative_eq!(half.over + half.under, 1.0, epsilon = 1e-12);

        let whole = m.grade_total(45.0);
        assert!(whole.push > 0.0);
        assert_relative_eq!(whole.over + whole.push + whole.under, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn the_total_sits_where_the_two_means_add_up() {
        let m = normal_game(&nfl(false)).unwrap();
        assert_relative_eq!(m.mean_total, 45.5, epsilon = 0.05);
        // A total of exactly the mean is a coin flip either way.
        let g = m.grade_total(45.5);
        assert_relative_eq!(g.over, 0.5, epsilon = 0.02);
    }

    #[test]
    fn the_moneyline_sums_to_one_and_favors_the_favorite() {
        let m = normal_game(&nfl(true)).unwrap();
        let ml = m.moneyline();
        assert_relative_eq!(ml.home + ml.draw + ml.away, 1.0, epsilon = 1e-12);
        assert!(ml.home > ml.away);
        assert!(ml.draw < 0.01, "a keyed football tie should be rare, got {}", ml.draw);
    }

    // --- the curve ---------------------------------------------------------

    #[test]
    fn the_cover_curve_rises_monotonically_across_the_range() {
        // The range runs from laying 14 to getting 14, so covering gets easier
        // as the line moves. `laying_more_points_is_always_harder` walks the
        // same relationship the other way.
        let m = normal_game(&nfl(false)).unwrap();
        let curve = cover_curve(&m, -14.0, 14.0, 0.5).unwrap();
        assert_eq!(curve.len(), 57);
        for w in curve.windows(2) {
            assert!(
                w[1].home_covers >= w[0].home_covers - 1e-12,
                "curve fell from {:?} to {:?}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn key_numbers_put_visible_steps_in_the_curve() {
        // The whole reason to draw it. Crossing 3 should cost far more than
        // crossing a neighbouring half point.
        let m = normal_game(&nfl(true)).unwrap();
        let at = |s: f64| m.grade_spread(s).home_covers;

        let across_three = at(-2.5) - at(-3.5);
        let across_five = at(-4.5) - at(-5.5);
        assert!(
            across_three > 2.0 * across_five,
            "crossing 3 cost {across_three}, crossing 5 cost {across_five}"
        );
    }

    #[test]
    fn a_smooth_model_has_no_such_step() {
        let m = normal_game(&nfl(false)).unwrap();
        let at = |s: f64| m.grade_spread(s).home_covers;
        let across_three = at(-2.5) - at(-3.5);
        let across_five = at(-4.5) - at(-5.5);
        assert!(across_three < 1.3 * across_five);
    }

    #[test]
    fn bad_input_errors() {
        assert!(normal_game(&GameShape { sd_margin: 0.0, ..nfl(false) }).is_err());
        assert!(normal_game(&GameShape { sd_total: -1.0, ..nfl(false) }).is_err());
        assert!(normal_game(&GameShape { mu_home: -3.0, ..nfl(false) }).is_err());

        let m = normal_game(&nfl(false)).unwrap();
        assert!(cover_curve(&m, -14.0, 14.0, 0.0).is_err());
        assert!(cover_curve(&m, 14.0, -14.0, 0.5).is_err());
        assert!(cover_curve(&m, -100.0, 100.0, 0.1).is_err());
    }
}
