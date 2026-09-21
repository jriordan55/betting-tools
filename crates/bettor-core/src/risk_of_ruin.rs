//! Bankroll survival under flat betting — Monte Carlo.
//!
//! Ported from `bettor-calculator-main/src/lib/math/riskOfRuin.ts`.
//!
//! # Divergences from the TypeScript
//!
//! 1. **Seeded and reproducible.** The TS used `Math.random()`, so a tool
//!    telling you your probability of going broke gave a different answer on
//!    every click and could not be tested. Every run here takes a seed and
//!    reports it back.
//! 2. **Median and mean are computed over the same population.** The TS took
//!    the median over *survivors* and the mean over *all paths* (scoring ruined
//!    ones as zero), then displayed them side by side. With a 30% ruin rate the
//!    median looked healthy while the mean was dragged down, and nothing on
//!    screen explained why. Both bases are reported here, named.
//! 3. **Invalid input is an error.** The TS returned a zeroed struct, so a
//!    typo displayed as `0%` — the safest-looking possible answer.
//! 4. **Parallel across paths** via rayon, with per-path RNG streams derived
//!    from the run seed so results stay reproducible regardless of thread count.

use crate::{MathError, Result};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// Parameters for a survival simulation.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RuinInput {
    /// True win probability, 0–1.
    pub win_prob: f64,
    /// Decimal price of each bet.
    pub decimal_odds: f64,
    /// Flat stake per bet.
    pub bet_size: f64,
    /// Starting bankroll.
    pub bankroll: f64,
    /// How many bets to simulate.
    pub num_bets: usize,
    /// How many independent paths to run.
    pub num_sims: usize,
}

/// Fraction of paths still solvent at a given bet number.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SurvivalPoint {
    /// Bet index.
    pub bet: usize,
    /// Fraction of paths still able to place a bet, 0–1.
    pub survival: f64,
}

/// Outcome of a survival simulation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RuinResult {
    /// Seed that produced this run. Feed it back to reproduce exactly.
    ///
    /// Crosses the wire as a decimal string — see [`crate::seed_repr`].
    #[serde(serialize_with = "crate::seed_repr::serialize")]
    #[cfg_attr(feature = "specta", specta(type = String))]
    pub seed: u64,
    /// Fraction of paths that went broke, 0–1.
    pub ruin_prob: f64,
    /// EV of a single bet, in currency.
    pub ev_per_bet: f64,
    /// EV as a fraction of stake.
    pub edge: f64,

    /// Median ending bankroll across **all** paths, ruined counted as zero.
    pub median_ending_all: f64,
    /// Mean ending bankroll across **all** paths, ruined counted as zero.
    pub mean_ending_all: f64,
    /// Median ending bankroll across **surviving** paths only.
    ///
    /// This is the figure the TS labelled simply "median bankroll", beside a
    /// mean computed over a different population.
    pub median_ending_survivors: f64,

    /// Mean of each path's worst peak-to-trough drop, as a fraction of peak.
    pub mean_max_drawdown: f64,
    /// Median of the same, which is the more honest summary of a skewed figure.
    pub median_max_drawdown: f64,
    /// Longest run of consecutive losses seen in any path.
    ///
    /// Not in the TS, and the statistic bettors most underestimate at long
    /// prices: a real 21% shot loses more than twenty in a row routinely.
    pub longest_losing_streak: usize,

    /// Solvency over time, for charting.
    pub survival_curve: Vec<SurvivalPoint>,
}

/// One path's outcome.
struct Path {
    ending: f64,
    ruined: bool,
    max_drawdown: f64,
    longest_losing_streak: usize,
    /// Whether the path was still solvent at each sample point.
    alive_at: Vec<bool>,
}

fn validate(input: &RuinInput) -> Result<()> {
    if !input.win_prob.is_finite() || input.win_prob <= 0.0 || input.win_prob >= 1.0 {
        return Err(MathError::ProbabilityOutOfRange {
            value: input.win_prob,
            reason: "win probability must be strictly between 0 and 1",
        });
    }
    for (param, value) in [
        ("decimal odds", input.decimal_odds - 1.0),
        ("bet size", input.bet_size),
        ("bankroll", input.bankroll),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(MathError::DomainError {
                param,
                constraint: "greater than zero (odds must exceed 1.0)",
                value,
            });
        }
    }
    if input.bet_size > input.bankroll {
        return Err(MathError::DomainError {
            param: "bet size",
            constraint: "no larger than the bankroll",
            value: input.bet_size,
        });
    }
    for (param, value) in [("num_bets", input.num_bets), ("num_sims", input.num_sims)] {
        if value == 0 {
            return Err(MathError::ShapeError {
                what: param,
                expected: "at least 1",
                got: 0,
            });
        }
    }
    Ok(())
}

/// Chooses the bet indices at which to sample the survival curve.
fn sample_points(num_bets: usize) -> Vec<usize> {
    let target = num_bets.min(200);
    let interval = (num_bets / target).max(1);
    let mut points: Vec<usize> = (1..=num_bets).step_by(interval).skip(1).collect();
    if points.last() != Some(&num_bets) {
        points.push(num_bets);
    }
    points
}

fn simulate_path(input: &RuinInput, points: &[usize], seed: u64) -> Path {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let win_payout = input.bet_size * (input.decimal_odds - 1.0);

    let mut balance = input.bankroll;
    let mut peak = input.bankroll;
    let mut max_drawdown = 0.0_f64;
    let mut losing_streak = 0_usize;
    let mut longest_losing_streak = 0_usize;
    let mut alive_at = vec![false; points.len()];
    let mut point_idx = 0_usize;
    let mut ruined = false;

    for bet in 1..=input.num_bets {
        if balance < input.bet_size {
            ruined = true;
            break;
        }

        if rng.random::<f64>() < input.win_prob {
            balance += win_payout;
            losing_streak = 0;
        } else {
            balance -= input.bet_size;
            losing_streak += 1;
            longest_losing_streak = longest_losing_streak.max(losing_streak);
        }

        peak = peak.max(balance);
        if peak > 0.0 {
            max_drawdown = max_drawdown.max((peak - balance) / peak);
        }

        if points.get(point_idx) == Some(&bet) {
            if let Some(slot) = alive_at.get_mut(point_idx) {
                *slot = balance >= input.bet_size;
            }
            point_idx += 1;
        }
    }

    let ruined = ruined || balance < input.bet_size;
    Path {
        ending: if ruined { 0.0 } else { balance },
        ruined,
        max_drawdown,
        longest_losing_streak,
        alive_at,
    }
}

fn median(sorted: &[f64]) -> f64 {
    match sorted.len() {
        0 => 0.0,
        n if n % 2 == 1 => sorted.get(n / 2).copied().unwrap_or(0.0),
        n => {
            let a = sorted.get(n / 2 - 1).copied().unwrap_or(0.0);
            let b = sorted.get(n / 2).copied().unwrap_or(0.0);
            (a + b) / 2.0
        }
    }
}

/// Runs the survival simulation.
///
/// # Errors
///
/// [`MathError::ProbabilityOutOfRange`] for a win probability outside `(0, 1)`,
/// [`MathError::DomainError`] for non-positive money or a price at or below
/// 1.0, [`MathError::ShapeError`] for a zero horizon or zero paths.
pub fn simulate_ruin(input: &RuinInput, seed: u64) -> Result<RuinResult> {
    validate(input)?;
    let points = sample_points(input.num_bets);

    // Each path gets its own stream derived from the run seed, so the result is
    // identical no matter how rayon schedules the work.
    #[cfg(feature = "parallel")]
    let paths: Vec<Path> = {
        use rayon::prelude::*;
        (0..input.num_sims)
            .into_par_iter()
            .map(|i| simulate_path(input, &points, seed.wrapping_add(i as u64)))
            .collect()
    };
    #[cfg(not(feature = "parallel"))]
    let paths: Vec<Path> = (0..input.num_sims)
        .map(|i| simulate_path(input, &points, seed.wrapping_add(i as u64)))
        .collect();

    #[allow(clippy::cast_precision_loss, reason = "simulation count")]
    let sims = input.num_sims as f64;

    let ruin_count = paths.iter().filter(|p| p.ruined).count();
    #[allow(clippy::cast_precision_loss, reason = "simulation count")]
    let ruin_prob = ruin_count as f64 / sims;

    let mut all_endings: Vec<f64> = paths.iter().map(|p| p.ending).collect();
    all_endings.sort_by(f64::total_cmp);
    let mut survivor_endings: Vec<f64> = paths
        .iter()
        .filter(|p| !p.ruined)
        .map(|p| p.ending)
        .collect();
    survivor_endings.sort_by(f64::total_cmp);

    let mut drawdowns: Vec<f64> = paths.iter().map(|p| p.max_drawdown).collect();
    drawdowns.sort_by(f64::total_cmp);

    let survival_curve = std::iter::once(SurvivalPoint {
        bet: 0,
        survival: 1.0,
    })
    .chain(points.iter().enumerate().map(|(i, bet)| {
        let alive = paths
            .iter()
            .filter(|p| p.alive_at.get(i).copied().unwrap_or(false))
            .count();
        #[allow(clippy::cast_precision_loss, reason = "path count")]
        let survival = alive as f64 / sims;
        SurvivalPoint { bet: *bet, survival }
    }))
    .collect();

    let win_payout = input.bet_size * (input.decimal_odds - 1.0);
    let ev_per_bet = input.win_prob * win_payout - (1.0 - input.win_prob) * input.bet_size;

    Ok(RuinResult {
        seed,
        ruin_prob,
        ev_per_bet,
        edge: ev_per_bet / input.bet_size,
        median_ending_all: median(&all_endings),
        mean_ending_all: all_endings.iter().sum::<f64>() / sims,
        median_ending_survivors: median(&survivor_endings),
        mean_max_drawdown: drawdowns.iter().sum::<f64>() / sims,
        median_max_drawdown: median(&drawdowns),
        longest_losing_streak: paths
            .iter()
            .map(|p| p.longest_losing_streak)
            .max()
            .unwrap_or(0),
        survival_curve,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::odds::american_to_decimal;
    use approx::assert_relative_eq;

    fn input(win_prob: f64, american: f64, bets: usize) -> RuinInput {
        RuinInput {
            win_prob,
            decimal_odds: american_to_decimal(american).unwrap(),
            bet_size: 100.0,
            bankroll: 10_000.0,
            num_bets: bets,
            num_sims: 2_000,
        }
    }

    #[test]
    fn the_same_seed_reproduces_the_run_exactly() {
        let i = input(0.55, -110.0, 500);
        let a = simulate_ruin(&i, 12_345).unwrap();
        let b = simulate_ruin(&i, 12_345).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.seed, 12_345);
    }

    #[test]
    fn a_bigger_edge_means_less_ruin() {
        // Needs a bankroll thin enough for ruin to be reachable — at 100 units
        // with any positive edge, essentially nobody goes broke over 1000 bets.
        let thin = |p: f64| RuinInput {
            bankroll: 1_000.0,
            ..input(p, -110.0, 1_000)
        };
        let weak = simulate_ruin(&thin(0.53), 1).unwrap();
        let strong = simulate_ruin(&thin(0.60), 1).unwrap();
        assert!(
            weak.ruin_prob > 0.05,
            "scenario should risk ruin, got {}",
            weak.ruin_prob
        );
        assert!(
            strong.ruin_prob < weak.ruin_prob,
            "strong {} should beat weak {}",
            strong.ruin_prob,
            weak.ruin_prob
        );
    }

    #[test]
    fn a_deep_bankroll_survives_a_real_edge() {
        // 100 units at a 5% edge: ruin is effectively off the table, which is
        // the whole argument for bankroll depth.
        let r = simulate_ruin(&input(0.55, -110.0, 1_000), 1).unwrap();
        assert!(r.ruin_prob < 0.01, "got {}", r.ruin_prob);
    }

    #[test]
    fn ev_per_bet_matches_the_closed_form() {
        let i = input(0.55, -110.0, 100);
        let r = simulate_ruin(&i, 1).unwrap();
        let expected = 0.55 * 100.0 * (i.decimal_odds - 1.0) - 0.45 * 100.0;
        assert_relative_eq!(r.ev_per_bet, expected, epsilon = 1e-9);
        assert_relative_eq!(r.edge, expected / 100.0, epsilon = 1e-9);
    }

    #[test]
    fn longshots_produce_far_longer_losing_streaks_at_equal_edge() {
        // The point of the whole project: same 5% edge, wildly different ride.
        let favorite = simulate_ruin(&input(0.55, -110.0, 1_000), 99).unwrap();
        let longshot = simulate_ruin(&input(0.21, 400.0, 1_000), 99).unwrap();
        assert_relative_eq!(favorite.edge, longshot.edge, epsilon = 0.02);
        assert!(
            longshot.longest_losing_streak > 2 * favorite.longest_losing_streak,
            "favorite {} vs longshot {}",
            favorite.longest_losing_streak,
            longshot.longest_losing_streak
        );
    }

    #[test]
    fn longshots_draw_down_deeper_at_equal_edge() {
        let favorite = simulate_ruin(&input(0.55, -110.0, 1_000), 7).unwrap();
        let longshot = simulate_ruin(&input(0.21, 400.0, 1_000), 7).unwrap();
        assert!(longshot.median_max_drawdown > favorite.median_max_drawdown);
    }

    #[test]
    fn survival_starts_at_one_and_never_increases() {
        let r = simulate_ruin(&input(0.50, -110.0, 1_000), 3).unwrap();
        assert_relative_eq!(r.survival_curve[0].survival, 1.0, epsilon = 1e-12);
        assert_eq!(r.survival_curve[0].bet, 0);
        for w in r.survival_curve.windows(2) {
            assert!(
                w[1].survival <= w[0].survival + 1e-12,
                "survival rose from {:?} to {:?}",
                w[0],
                w[1]
            );
        }
        assert_eq!(r.survival_curve.last().unwrap().bet, 1_000);
    }

    #[test]
    fn median_over_survivors_exceeds_median_over_all_when_paths_die() {
        // Exactly the inconsistency the TS shipped: these are different numbers
        // and the app showed one of each without saying so.
        let brutal = RuinInput {
            bet_size: 2_000.0,
            bankroll: 10_000.0,
            ..input(0.21, 400.0, 500)
        };
        let r = simulate_ruin(&brutal, 5).unwrap();
        assert!(r.ruin_prob > 0.1, "expected meaningful ruin, got {}", r.ruin_prob);
        assert!(r.median_ending_survivors > r.median_ending_all);
    }

    #[test]
    fn a_negative_edge_ruins_almost_everyone_eventually() {
        let r = simulate_ruin(&input(0.45, -110.0, 5_000), 2).unwrap();
        assert!(r.ruin_prob > 0.9, "got {}", r.ruin_prob);
        assert!(r.edge < 0.0);
    }

    #[test]
    fn invalid_input_errors_instead_of_reporting_zero_risk() {
        let base = input(0.55, -110.0, 100);
        assert!(simulate_ruin(&RuinInput { win_prob: 0.0, ..base }, 1).is_err());
        assert!(simulate_ruin(&RuinInput { win_prob: 1.0, ..base }, 1).is_err());
        assert!(simulate_ruin(&RuinInput { decimal_odds: 1.0, ..base }, 1).is_err());
        assert!(simulate_ruin(&RuinInput { bet_size: 0.0, ..base }, 1).is_err());
        assert!(simulate_ruin(&RuinInput { bankroll: -1.0, ..base }, 1).is_err());
        assert!(simulate_ruin(&RuinInput { num_bets: 0, ..base }, 1).is_err());
        assert!(simulate_ruin(&RuinInput { num_sims: 0, ..base }, 1).is_err());
        assert!(simulate_ruin(&RuinInput { bet_size: 99_999.0, ..base }, 1).is_err());
    }
}
