//! Predicting a match scoreline, and deriving every market from it.
//!
//! Ported from `poisson.ts` and `nbinom.ts`, which shared a shape but not a
//! file. Each team's scoring is modelled independently; the joint grid is the
//! outer product of the two marginals, and moneyline, spread, and total prices
//! all fall out of summing regions of that grid.
//!
//! Poisson forces variance to equal the mean. The negative binomial adds a
//! dispersion parameter `r`, giving variance `μ + μ²/r` — more blowouts. As
//! `r → ∞` it converges back to Poisson.
//!
//! # Divergences from the TypeScript
//!
//! 1. **The truncated grid is renormalised.** Both builders cut the grid off
//!    at `max_score` and used it as-is, so the probabilities summed to less
//!    than one and every derived price was biased low. How much is lost depends
//!    entirely on the grid: at baseball rates (μ ≈ 4.5 and 4.2) a `max_score`
//!    of 10 drops **1.07%** of the joint distribution, while the 16 that
//!    `crate::config` actually ships for baseball drops 0.0007%. The mass lost
//!    is renormalised away and reported as [`ScoreMatrix::truncation_mass`] so
//!    the choice of `max_score` can be checked rather than trusted.
//!
//!    (An earlier revision of this note quoted 0.7% for the `max_score` = 10
//!    case. That was a single marginal's tail, not the joint truncation this
//!    module reports.)
//! 2. **Pushes are counted.** The TS assigned a scoreline to `homeCovers` when
//!    `margin > spread` and `awayCovers` when `margin < spread`, silently
//!    dropping exact ties on an integer line. The two sides then failed to sum
//!    to 1 with no indication why. [`SpreadProb::push`] holds that mass.

use crate::{MathError, Result};
use serde::Serialize;

/// Natural log of the gamma function, via the Lanczos approximation.
///
/// Coefficients kept identical to the TypeScript (`g = 7`, 9 terms).
#[must_use]
pub fn log_gamma(z: f64) -> f64 {
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    const G: f64 = 7.0;

    if z < 0.5 {
        // Reflection formula for the left half-plane.
        return (core::f64::consts::PI / (core::f64::consts::PI * z).sin()).ln() - log_gamma(1.0 - z);
    }
    let z = z - 1.0;
    let mut x = C.first().copied().unwrap_or(0.0);
    for (i, c) in C.iter().enumerate().skip(1) {
        #[allow(clippy::cast_precision_loss, reason = "index below 9")]
        let i_f = i as f64;
        x += c / (z + i_f);
    }
    let t = z + G + 0.5;
    0.5 * (2.0 * core::f64::consts::PI).ln() + (z + 0.5) * t.ln() - t + x.ln()
}

/// `P(X = k)` for a Poisson with rate `lambda`.
#[must_use]
pub fn poisson_pmf(k: u32, lambda: f64) -> f64 {
    if lambda < 0.0 || lambda.is_nan() {
        return 0.0;
    }
    if lambda == 0.0 {
        return if k == 0 { 1.0 } else { 0.0 };
    }
    // Log space: large k overflows the factorial otherwise.
    let mut log_p = -lambda + f64::from(k) * lambda.ln();
    for i in 2..=k {
        log_p -= f64::from(i).ln();
    }
    log_p.exp()
}

/// `P(X <= k)` for a Poisson with rate `lambda`.
#[must_use]
pub fn poisson_cdf(k: u32, lambda: f64) -> f64 {
    (0..=k).map(|i| poisson_pmf(i, lambda)).sum()
}

/// `P(X = k)` for a negative binomial with the given mean and dispersion.
///
/// Parameterised by mean `μ` and dispersion `r`, so `p = r / (r + μ)` and the
/// variance is `μ + μ²/r`.
#[must_use]
pub fn nbinom_pmf(k: u32, mean: f64, r: f64) -> f64 {
    if mean < 0.0 || r <= 0.0 || mean.is_nan() || r.is_nan() {
        return 0.0;
    }
    if mean == 0.0 {
        return if k == 0 { 1.0 } else { 0.0 };
    }
    let k_f = f64::from(k);
    let p = r / (r + mean);
    let log_binom = log_gamma(k_f + r) - log_gamma(k_f + 1.0) - log_gamma(r);
    (log_binom + r * p.ln() + k_f * (1.0 - p).ln()).exp()
}

/// A joint distribution over scorelines, `P(home = h, away = a)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreMatrix {
    grid: Vec<f64>,
    max_score: u32,
    truncation_mass: f64,
}

fn check_positive(param: &'static str, value: f64) -> Result<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(MathError::DomainError {
            param,
            constraint: "finite and greater than zero",
            value,
        });
    }
    Ok(())
}

impl ScoreMatrix {
    /// Builds the grid from two marginals and renormalises it.
    fn from_marginals(home: &[f64], away: &[f64], max_score: u32) -> Self {
        let mut grid = Vec::with_capacity(home.len() * away.len());
        for ph in home {
            for pa in away {
                grid.push(ph * pa);
            }
        }
        let total: f64 = grid.iter().sum();
        // The TS shipped the grid unnormalised, so every price derived from it
        // was biased low by exactly this much.
        let truncation_mass = 1.0 - total;
        if total > 0.0 {
            for cell in &mut grid {
                *cell /= total;
            }
        }
        Self {
            grid,
            max_score,
            truncation_mass,
        }
    }

    /// Independent Poisson scoring for both sides.
    ///
    /// # Errors
    ///
    /// [`MathError::DomainError`] for a non-positive rate.
    pub fn poisson(lambda_home: f64, lambda_away: f64, max_score: u32) -> Result<Self> {
        check_positive("lambda_home", lambda_home)?;
        check_positive("lambda_away", lambda_away)?;
        let home: Vec<f64> = (0..=max_score).map(|k| poisson_pmf(k, lambda_home)).collect();
        let away: Vec<f64> = (0..=max_score).map(|k| poisson_pmf(k, lambda_away)).collect();
        Ok(Self::from_marginals(&home, &away, max_score))
    }

    /// Independent negative-binomial scoring, for blowout-prone matchups.
    ///
    /// # Errors
    ///
    /// [`MathError::DomainError`] for a non-positive mean or dispersion.
    pub fn negative_binomial(
        mean_home: f64,
        mean_away: f64,
        r_home: f64,
        r_away: f64,
        max_score: u32,
    ) -> Result<Self> {
        check_positive("mean_home", mean_home)?;
        check_positive("mean_away", mean_away)?;
        check_positive("r_home", r_home)?;
        check_positive("r_away", r_away)?;
        let home: Vec<f64> = (0..=max_score)
            .map(|k| nbinom_pmf(k, mean_home, r_home))
            .collect();
        let away: Vec<f64> = (0..=max_score)
            .map(|k| nbinom_pmf(k, mean_away, r_away))
            .collect();
        Ok(Self::from_marginals(&home, &away, max_score))
    }

    /// Probability of an exact scoreline.
    #[must_use]
    pub fn get(&self, home: u32, away: u32) -> f64 {
        if home > self.max_score || away > self.max_score {
            return 0.0;
        }
        let width = self.max_score as usize + 1;
        let idx = home as usize * width + away as usize;
        self.grid.get(idx).copied().unwrap_or(0.0)
    }

    /// Highest score the grid represents.
    #[must_use]
    pub const fn max_score(&self) -> u32 {
        self.max_score
    }

    /// Probability mass that fell outside the grid before renormalising.
    ///
    /// A large value means `max_score` is too low for these rates. The TS
    /// neither reported nor corrected for it.
    #[must_use]
    pub const fn truncation_mass(&self) -> f64 {
        self.truncation_mass
    }

    fn iter_cells(&self) -> impl Iterator<Item = (u32, u32, f64)> + '_ {
        let width = self.max_score + 1;
        self.grid.iter().enumerate().map(move |(i, p)| {
            #[allow(clippy::cast_possible_truncation, reason = "grid index fits u32")]
            let i = i as u32;
            (i / width, i % width, *p)
        })
    }
}

/// One exact scoreline and its probability.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Scoreline {
    /// Home team's score.
    pub home: u32,
    /// Away team's score.
    pub away: u32,
    /// Probability of exactly this result.
    pub prob: f64,
}

/// Cover probabilities for one spread.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SpreadProb {
    /// The line, from the home side's perspective.
    pub spread: f64,
    /// Probability the home side covers.
    pub home_covers: f64,
    /// Probability the away side covers.
    pub away_covers: f64,
    /// Probability of a push. Nonzero only on whole-number lines.
    ///
    /// The TS dropped this mass entirely, so the two sides did not sum to 1.
    pub push: f64,
}

/// Over/under probabilities for one total.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TotalProb {
    /// The line.
    pub line: f64,
    /// Probability the total goes over.
    pub over: f64,
    /// Probability the total stays under.
    pub under: f64,
    /// Probability of a push.
    pub push: f64,
}

/// Every market derivable from a score matrix.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MarketProbs {
    /// Probability the home side wins outright.
    pub home_win: f64,
    /// Probability of a draw. Zero when draws were redistributed.
    pub draw: f64,
    /// Probability the away side wins outright.
    pub away_win: f64,
    /// The ten most likely exact scorelines, most likely first.
    pub top_scorelines: Vec<Scoreline>,
    /// Cover probabilities at each requested spread.
    pub spreads: Vec<SpreadProb>,
    /// Over/under probabilities at each requested total.
    pub totals: Vec<TotalProb>,
}

/// Derives moneyline, spread, and total probabilities from a score matrix.
///
/// When `allow_draw` is false — baseball, hockey, anything that plays until
/// somebody wins — draw probability is redistributed to the two sides in
/// proportion to their existing win probabilities.
#[must_use]
pub fn derive_markets(
    matrix: &ScoreMatrix,
    spread_lines: &[f64],
    total_lines: &[f64],
    allow_draw: bool,
) -> MarketProbs {
    let mut home_win = 0.0;
    let mut draw = 0.0;
    let mut away_win = 0.0;
    let mut scorelines = Vec::with_capacity(matrix.grid.len());

    for (h, a, p) in matrix.iter_cells() {
        match h.cmp(&a) {
            core::cmp::Ordering::Greater => home_win += p,
            core::cmp::Ordering::Equal => draw += p,
            core::cmp::Ordering::Less => away_win += p,
        }
        scorelines.push(Scoreline {
            home: h,
            away: a,
            prob: p,
        });
    }

    if !allow_draw && draw > 0.0 {
        let decided = home_win + away_win;
        if decided > 0.0 {
            let home_share = home_win / decided;
            home_win += draw * home_share;
            away_win += draw * (1.0 - home_share);
        }
        draw = 0.0;
    }

    scorelines.sort_by(|a, b| b.prob.total_cmp(&a.prob));
    scorelines.truncate(10);

    let spreads = spread_lines
        .iter()
        .map(|&spread| {
            let (mut home_covers, mut away_covers, mut push) = (0.0, 0.0, 0.0);
            for (h, a, p) in matrix.iter_cells() {
                let margin = f64::from(h) - f64::from(a);
                if margin > spread {
                    home_covers += p;
                } else if margin < spread {
                    away_covers += p;
                } else {
                    push += p;
                }
            }
            SpreadProb {
                spread,
                home_covers,
                away_covers,
                push,
            }
        })
        .collect();

    let totals = total_lines
        .iter()
        .map(|&line| {
            let (mut over, mut under, mut push) = (0.0, 0.0, 0.0);
            for (h, a, p) in matrix.iter_cells() {
                let total = f64::from(h) + f64::from(a);
                if total > line {
                    over += p;
                } else if total < line {
                    under += p;
                } else {
                    push += p;
                }
            }
            TotalProb {
                line,
                over,
                under,
                push,
            }
        })
        .collect();

    MarketProbs {
        home_win,
        draw,
        away_win,
        top_scorelines: scorelines,
        spreads,
        totals,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn poisson_pmf_matches_hand_computed_values() {
        // e^-2.5 and e^-2.5 * 2.5^2 / 2
        assert_relative_eq!(poisson_pmf(0, 2.5), 0.082_084_998_6, epsilon = 1e-9);
        assert_relative_eq!(poisson_pmf(2, 2.5), 0.256_515_620_6, epsilon = 1e-9);
    }

    #[test]
    fn poisson_pmf_sums_to_one() {
        for lambda in [0.5, 1.5, 4.5, 12.0] {
            let total: f64 = (0..200).map(|k| poisson_pmf(k, lambda)).sum();
            assert_relative_eq!(total, 1.0, epsilon = 1e-12);
        }
    }

    #[test]
    fn poisson_cdf_is_monotonic_and_reaches_one() {
        let mut prev = 0.0;
        for k in 0..60 {
            let c = poisson_cdf(k, 4.5);
            assert!(c >= prev - 1e-15);
            prev = c;
        }
        assert_relative_eq!(prev, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn log_gamma_reproduces_factorials() {
        // Γ(n) = (n-1)!
        assert_relative_eq!(log_gamma(1.0), 0.0, epsilon = 1e-9);
        assert_relative_eq!(log_gamma(5.0), 24.0_f64.ln(), epsilon = 1e-9);
        assert_relative_eq!(log_gamma(11.0), 3_628_800.0_f64.ln(), epsilon = 1e-9);
        // Γ(1/2) = √π
        assert_relative_eq!(
            log_gamma(0.5),
            core::f64::consts::PI.sqrt().ln(),
            epsilon = 1e-9
        );
    }

    #[test]
    fn nbinom_pmf_sums_to_one() {
        for (mean, r) in [(1.5, 5.0), (4.5, 3.0), (2.0, 20.0)] {
            let total: f64 = (0..500).map(|k| nbinom_pmf(k, mean, r)).sum();
            assert_relative_eq!(total, 1.0, epsilon = 1e-9);
        }
    }

    #[test]
    fn nbinom_converges_to_poisson_as_dispersion_grows() {
        for k in 0..8 {
            assert_relative_eq!(nbinom_pmf(k, 2.5, 1e7), poisson_pmf(k, 2.5), epsilon = 1e-5);
        }
    }

    #[test]
    fn nbinom_has_more_variance_than_poisson() {
        // Var = μ + μ²/r, so a low r puts more mass in the blowout tail.
        let mean = 3.0;
        let tail_nb: f64 = (8..60).map(|k| nbinom_pmf(k, mean, 2.0)).sum();
        let tail_pois: f64 = (8..60).map(|k| poisson_pmf(k, mean)).sum();
        assert!(tail_nb > tail_pois, "{tail_nb} should exceed {tail_pois}");
    }

    #[test]
    fn the_matrix_is_normalised() {
        // The TS shipped this summing to less than 1.
        let m = ScoreMatrix::poisson(4.5, 4.2, 10).unwrap();
        let total: f64 = (0..=10).flat_map(|h| (0..=10).map(move |a| (h, a))).map(|(h, a)| m.get(h, a)).sum();
        assert_relative_eq!(total, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn truncation_mass_is_reported_and_material_at_baseball_rates() {
        let tight = ScoreMatrix::poisson(1.4, 1.2, 10).unwrap();
        let loose = ScoreMatrix::poisson(4.5, 4.2, 10).unwrap();
        assert!(tight.truncation_mass() < 1e-4);
        assert!(
            loose.truncation_mass() > 1e-3,
            "expected material truncation, got {}",
            loose.truncation_mass()
        );
    }

    #[test]
    fn truncation_mass_matches_the_figures_quoted_in_the_docs() {
        // Pinned because the module docs, DIVERGENCES.md and the UI all quote
        // these numbers to users, and a loose `> 1e-3` bound let an incorrect
        // 0.7% sit in all three unchallenged. 1 - Σ₀¹⁰P(4.5) × Σ₀¹⁰P(4.2).
        let undersized = ScoreMatrix::poisson(4.5, 4.2, 10).unwrap();
        assert_relative_eq!(undersized.truncation_mass(), 0.010_71, epsilon = 1e-5);

        // And the grid `crate::config` actually ships for baseball, where the
        // loss is three orders of magnitude smaller.
        let shipped = ScoreMatrix::poisson(4.5, 4.2, 16).unwrap();
        assert_relative_eq!(shipped.truncation_mass(), 0.000_007, epsilon = 1e-6);
    }

    #[test]
    fn moneyline_probabilities_sum_to_one() {
        let m = ScoreMatrix::poisson(1.5, 1.2, 12).unwrap();
        let mk = derive_markets(&m, &[], &[], true);
        assert_relative_eq!(mk.home_win + mk.draw + mk.away_win, 1.0, epsilon = 1e-12);
        assert!(mk.home_win > mk.away_win, "the stronger side should be favored");
    }

    #[test]
    fn draws_are_redistributed_for_no_draw_sports() {
        let m = ScoreMatrix::poisson(4.5, 4.2, 14).unwrap();
        let with_draw = derive_markets(&m, &[], &[], true);
        let no_draw = derive_markets(&m, &[], &[], false);
        assert!(with_draw.draw > 0.0);
        assert_relative_eq!(no_draw.draw, 0.0, epsilon = 1e-15);
        assert_relative_eq!(no_draw.home_win + no_draw.away_win, 1.0, epsilon = 1e-12);
        assert!(no_draw.home_win > with_draw.home_win);
    }

    #[test]
    fn a_whole_number_spread_has_push_mass() {
        // The TS dropped this, leaving the two sides short of 1.
        let m = ScoreMatrix::poisson(1.5, 1.5, 12).unwrap();
        let mk = derive_markets(&m, &[0.0, -0.5], &[], true);
        let pk = mk.spreads[0];
        assert!(pk.push > 0.0, "a pick'em must be able to push");
        assert_relative_eq!(pk.home_covers + pk.away_covers + pk.push, 1.0, epsilon = 1e-12);
        // Half-point lines cannot push.
        assert_relative_eq!(mk.spreads[1].push, 0.0, epsilon = 1e-15);
    }

    #[test]
    fn totals_account_for_every_outcome() {
        let m = ScoreMatrix::poisson(1.5, 1.2, 12).unwrap();
        let mk = derive_markets(&m, &[], &[2.5, 3.0], true);
        for t in &mk.totals {
            assert_relative_eq!(t.over + t.under + t.push, 1.0, epsilon = 1e-12);
        }
        assert!(mk.totals[0].push < 1e-15, "a half-point total cannot push");
        assert!(mk.totals[1].push > 0.0, "a whole-number total can");
        // Raising the line must not raise the over probability.
        assert!(mk.totals[1].over < mk.totals[0].over);
    }

    #[test]
    fn top_scorelines_are_sorted_and_capped() {
        let m = ScoreMatrix::poisson(1.5, 1.2, 12).unwrap();
        let mk = derive_markets(&m, &[], &[], true);
        assert_eq!(mk.top_scorelines.len(), 10);
        for w in mk.top_scorelines.windows(2) {
            assert!(w[0].prob >= w[1].prob);
        }
    }

    #[test]
    fn invalid_rates_are_rejected() {
        assert!(ScoreMatrix::poisson(0.0, 1.0, 10).is_err());
        assert!(ScoreMatrix::poisson(-1.0, 1.0, 10).is_err());
        assert!(ScoreMatrix::negative_binomial(1.0, 1.0, 0.0, 1.0, 10).is_err());
    }
}
