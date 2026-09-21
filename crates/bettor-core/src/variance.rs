//! Odds range and variance — what a price does to the *ride*, not to the edge.
//!
//! New here; there is no TypeScript counterpart and therefore no golden
//! vectors. Every figure below is checked against a closed form or against a
//! hand-computed value, and the two simulated ones are checked against the
//! closed forms they should converge to.
//!
//! # The thing this module says
//!
//! Two bets carrying the same edge are not the same bet. A 5% edge at -110 and
//! a 5% edge at +400 have identical expectation and wildly different variance,
//! which shows up as a different drawdown, a different losing streak, and —
//! the figure nobody computes — a different number of bets before you can tell
//! the edge from noise:
//!
//! | Price | SD per unit | Bets to clear 2σ at a 5% edge |
//! |---|---|---|
//! | -110 | 0.95 | ~1,440 |
//! | +400 | 2.04 | ~6,640 |
//!
//! Same edge, four and a half times the sample. The rest of the app answers
//! "is this bet good?"; this module answers "and what will holding it feel
//! like?".
//!
//! # Conventions
//!
//! - A *unit* is one unit of stake. Returns are `decimal - 1` on a win and
//!   `-1` on a loss, so an edge of `0.05` means five cents per dollar risked.
//! - `edge` is always EV per unit staked, never a win-rate surplus. Those are
//!   different numbers and the reference app displayed one under the other's
//!   name.
//! - Legs are assumed **independent**. Correlated bets have larger variance
//!   than anything here reports; see [`crate::correlation`] for why one scalar
//!   correlation is not enough to fix that.

use crate::odds::{american_to_decimal, shift_cents, to_american};
use crate::{MathError, Result};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// Standard errors a result must clear to be called real.
///
/// Two, not 1.96: the extra precision is false comfort on a figure whose
/// inputs are estimates, and a round number invites the reader to notice that
/// the threshold is a choice.
const DETECTION_Z: f64 = 2.0;

/// The most bets a single simulated season may contain.
///
/// A guard, not a modelling limit. Someone typing an extra zero into a bet
/// count should get an error rather than a frozen window.
const MAX_SEASON_BETS: usize = 200_000;

/// The most individual wagers a whole run may simulate.
///
/// `bets × seasons`, which is the quantity that actually costs time — a legal
/// 200,000-bet season across 10,000 runs is two billion wagers and would hang
/// the window for minutes. Capping the product rather than each factor lets a
/// long season and a wide sweep both stay available, just not together.
const MAX_SIMULATED_WAGERS: usize = 50_000_000;

// ---------------------------------------------------------------------------
// Per-bet moments
// ---------------------------------------------------------------------------

/// Mean and variance of a one-unit bet at decimal price `d` with win probability `p`.
///
/// Returns `(mean, variance)`. The mean is the edge.
fn moments(decimal: f64, win_prob: f64) -> (f64, f64) {
    let mean = win_prob * decimal - 1.0;
    let win = decimal - 1.0;
    // E[X²] over the two outcomes, then the usual subtraction.
    let second = win_prob * win * win + (1.0 - win_prob) * 1.0;
    (mean, second - mean * mean)
}

/// The win probability that produces `edge` at decimal price `decimal`.
///
/// `edge = p * decimal - 1`, so `p = (1 + edge) / decimal`. Endpoints are
/// allowed: `edge = -1` is a 0% win rate; `edge = decimal - 1` is 100%.
fn win_prob_for_edge(decimal: f64, edge: f64) -> Result<f64> {
    let p = (1.0 + edge) / decimal;
    if !p.is_finite() || p < 0.0 || p > 1.0 {
        return Err(MathError::ProbabilityOutOfRange {
            value: p,
            reason: "no win rate produces that edge at that price",
        });
    }
    Ok(p)
}

/// How many bets before an edge clears `DETECTION_Z` standard errors.
///
/// `ev` and `sd` describe a sample of `bets` wagers; the answer scales that
/// sample up until its t-statistic reaches the threshold. `None` when the edge
/// is not positive, because a losing bettor never confirms anything.
fn detection_horizon(ev: f64, sd: f64, bets: f64) -> Option<f64> {
    if ev <= 0.0 || sd <= 0.0 || bets <= 0.0 {
        return None;
    }
    Some(bets * (DETECTION_Z * sd / ev).powi(2))
}

fn check_edge(edge: f64) -> Result<()> {
    if !edge.is_finite() || edge < -1.0 {
        return Err(MathError::DomainError {
            param: "edge",
            constraint: "at least -1 (a fraction of stake, not a percentage)",
            value: edge,
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Breakeven ladder
// ---------------------------------------------------------------------------

/// One price on the breakeven ladder.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LadderRung {
    /// American price.
    pub american: i32,
    /// Decimal price.
    pub decimal: f64,
    /// Win rate that breaks even at this price, 0–1. This is `1 / decimal`.
    pub breakeven: f64,
    /// Win rate needed to earn the target edge, 0–1.
    pub required_win_rate: f64,
    /// Percentage points between the two above.
    ///
    /// Worth its own field because it *shrinks* as the price lengthens: the
    /// same 5% edge needs 2.6 points of cushion at -110 and 1.0 at +400. That
    /// makes a longshot edge look easier to hit and harder to prove, which is
    /// exactly backwards from how it is usually discussed.
    pub cushion: f64,
    /// Standard deviation of a one-unit bet at the required win rate.
    pub sd_per_unit: f64,
    /// Bets before the edge clears two standard errors.
    ///
    /// `None` when the target edge is not positive.
    pub bets_to_detect: Option<f64>,
}

/// Breakeven and required win rate across a range of prices.
///
/// # Errors
///
/// [`MathError::ShapeError`] for an empty price list,
/// [`MathError::DomainError`] for a price under 100 in magnitude or an edge
/// outside `(-1, 1)`, [`MathError::ProbabilityOutOfRange`] if no win rate can
/// produce the target edge at some price in the range.
pub fn breakeven_ladder(american_prices: &[f64], target_edge: f64) -> Result<Vec<LadderRung>> {
    if american_prices.is_empty() {
        return Err(MathError::ShapeError {
            what: "prices",
            expected: "at least 1",
            got: 0,
        });
    }
    check_edge(target_edge)?;

    american_prices
        .iter()
        .map(|&american| {
            let decimal = american_to_decimal(american)?;
            let breakeven = 1.0 / decimal;
            let required = win_prob_for_edge(decimal, target_edge)?;
            let (ev, var) = moments(decimal, required);
            let sd = var.sqrt();
            Ok(LadderRung {
                american: to_american(decimal)?,
                decimal,
                breakeven,
                required_win_rate: required,
                cushion: required - breakeven,
                sd_per_unit: sd,
                bets_to_detect: detection_horizon(ev, sd, 1.0),
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// CLV translator
// ---------------------------------------------------------------------------

/// What a fixed cents move is worth at one price.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ClvRung {
    /// Price taken.
    pub american: i32,
    /// Price taken, decimal.
    pub decimal: f64,
    /// Probability it implies, 0–1.
    pub implied: f64,
    /// Price after the move.
    pub closed_american: i32,
    /// Price after the move, decimal.
    pub closed_decimal: f64,
    /// Probability the closing price implies, 0–1.
    pub closed_implied: f64,
    /// Probability points gained. The currency that compounds a bankroll.
    pub prob_points: f64,
    /// `taken / closed - 1` — the ratio measure, retained to be argued with.
    pub ratio: f64,
    /// How many times larger the ratio measure looks than the honest one.
    ///
    /// Climbs steeply with price. This ratio *is* the misconception.
    pub distortion: f64,
}

/// Translates a fixed cents move into probability points across a price range.
///
/// The same twenty cents is worth 4.14 points at -110 and 0.20 at +900 — a
/// factor of twenty, where the ratio measure shows a factor of four. Quoting
/// CLV in cents therefore rewards exactly the bets whose edge takes longest to
/// confirm.
///
/// # Errors
///
/// [`MathError::ShapeError`] for an empty price list,
/// [`MathError::DomainError`] for a price under 100 in magnitude, or if the
/// move would push a price through the top of the board.
pub fn clv_ladder(american_prices: &[f64], cents: f64) -> Result<Vec<ClvRung>> {
    if american_prices.is_empty() {
        return Err(MathError::ShapeError {
            what: "prices",
            expected: "at least 1",
            got: 0,
        });
    }

    american_prices
        .iter()
        .map(|&american| {
            let decimal = american_to_decimal(american)?;
            let closed_american = shift_cents(american, cents)?;
            let closed_decimal = american_to_decimal(closed_american)?;

            let implied = 1.0 / decimal;
            let closed_implied = 1.0 / closed_decimal;
            let prob_points = closed_implied - implied;
            let ratio = decimal / closed_decimal - 1.0;

            Ok(ClvRung {
                american: to_american(decimal)?,
                decimal,
                implied,
                closed_american: to_american(closed_decimal)?,
                closed_decimal,
                closed_implied,
                prob_points,
                ratio,
                distortion: if prob_points.abs() < f64::EPSILON {
                    f64::NAN
                } else {
                    ratio / prob_points
                },
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Bet mix
// ---------------------------------------------------------------------------

/// One kind of bet in a mix: a price, a stake, an edge, and how many of them.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MixLeg {
    /// American price.
    pub american: f64,
    /// Stake on each bet of this kind.
    pub stake: f64,
    /// EV per unit staked, as a fraction. `0.03` is a 3% edge.
    pub edge: f64,
    /// How many bets of this kind the mix contains.
    pub count: usize,
}

/// What one kind of bet contributes to the mix.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MixLegResult {
    /// American price, normalised.
    pub american: i32,
    /// Decimal price.
    pub decimal: f64,
    /// How many bets of this kind, echoed back from the input.
    ///
    /// Carried on the row rather than left to the caller to line up against
    /// its own input list: a leg the caller skipped shifts every index after
    /// it, and a table that reads counts from one list and prices from another
    /// would mislabel every row without failing.
    pub count: usize,
    /// Win rate implied by this leg's price and edge, 0–1.
    pub win_prob: f64,
    /// Win rate that would break even at this price, 0–1.
    pub breakeven: f64,
    /// Total staked on bets of this kind.
    pub total_stake: f64,
    /// Share of the mix's money, 0–1.
    pub stake_share: f64,
    /// Expected profit from bets of this kind.
    pub ev: f64,
    /// Share of the mix's *variance*, 0–1.
    ///
    /// The number the whole builder is for. Variance scales with the square of
    /// the stake and with the length of the price, so a price bucket routinely
    /// contributes a share of the swing several times its share of the money.
    pub variance_share: f64,
}

/// A whole bet mix, priced for both return and ride.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct BetMix {
    /// Per-kind breakdown, in the order supplied.
    pub legs: Vec<MixLegResult>,
    /// How many bets the mix contains.
    pub bets: usize,
    /// Total money risked.
    pub total_stake: f64,
    /// Expected profit over the whole mix.
    pub ev: f64,
    /// Expected profit as a fraction of money risked.
    pub roi: f64,

    /// The single win rate that would break the mix even, 0–1.
    ///
    /// Stake-weighted: `Σ stake / Σ (stake × decimal)`. It answers "what would
    /// I need to hit if every bet here hit at the same rate", which is a
    /// summary, not a prediction — the bets do not hit at the same rate.
    pub blended_breakeven: f64,
    /// Stake-weighted average of the legs' actual win rates, 0–1.
    pub blended_win_rate: f64,

    /// Standard deviation of the mix's total profit, in currency.
    pub sd: f64,
    /// [`Self::sd`] as a fraction of money risked.
    pub sd_per_unit: f64,
    /// Expected profit divided by its standard deviation.
    ///
    /// How much of the swing the edge actually is, over one run of this mix.
    /// Below about 0.5 the season is mostly noise no matter how good the bets.
    pub t_stat: f64,
    /// Bets before the edge clears two standard errors, at this mix's shape.
    pub bets_to_detect: Option<f64>,
}

/// Prices a mix of bets for return and for variance.
///
/// Legs are assumed independent. Real books of bets are not — same-game legs,
/// correlated sides, and whole-slate weather all push the true variance above
/// what this reports.
///
/// # Errors
///
/// [`MathError::ShapeError`] for an empty mix or a leg with a zero count,
/// [`MathError::DomainError`] for a non-positive stake, a price under 100 in
/// magnitude, or an edge below -1,
/// [`MathError::ProbabilityOutOfRange`] if a leg's edge is unreachable at its
/// price.
pub fn bet_mix(legs: &[MixLeg]) -> Result<BetMix> {
    if legs.is_empty() {
        return Err(MathError::ShapeError {
            what: "mix legs",
            expected: "at least 1",
            got: 0,
        });
    }

    struct Priced {
        decimal: f64,
        win_prob: f64,
        count: f64,
        bets: usize,
        stake: f64,
        ev: f64,
        variance: f64,
    }

    let priced: Vec<Priced> = legs
        .iter()
        .map(|leg| {
            let decimal = american_to_decimal(leg.american)?;
            if leg.edge < -1.0 || !leg.edge.is_finite() {
                return Err(MathError::DomainError {
                    param: "edge",
                    constraint: "at least -1 (a fraction of stake, not a percentage)",
                    value: leg.edge,
                });
            }
            if !leg.stake.is_finite() || leg.stake <= 0.0 {
                return Err(MathError::DomainError {
                    param: "stake",
                    constraint: "greater than zero",
                    value: leg.stake,
                });
            }
            if leg.count == 0 {
                return Err(MathError::ShapeError {
                    what: "bets of one kind",
                    expected: "at least 1",
                    got: 0,
                });
            }
            let win_prob = win_prob_for_edge(decimal, leg.edge)?;
            let (mean, var) = moments(decimal, win_prob);
            #[allow(clippy::cast_precision_loss, reason = "bet count")]
            let count = leg.count as f64;
            Ok(Priced {
                decimal,
                win_prob,
                count,
                bets: leg.count,
                stake: leg.stake,
                // Means add; variances add too, but stake enters squared.
                ev: count * leg.stake * mean,
                variance: count * leg.stake * leg.stake * var,
            })
        })
        .collect::<Result<_>>()?;

    let bets: usize = legs.iter().map(|l| l.count).sum();
    let total_stake: f64 = priced.iter().map(|p| p.count * p.stake).sum();
    let ev: f64 = priced.iter().map(|p| p.ev).sum();
    let total_variance: f64 = priced.iter().map(|p| p.variance).sum();
    let sd = total_variance.sqrt();

    if total_stake <= 0.0 {
        return Err(MathError::DomainError {
            param: "total stake",
            constraint: "greater than zero",
            value: total_stake,
        });
    }

    let payout_weighted: f64 = priced.iter().map(|p| p.count * p.stake * p.decimal).sum();
    let blended_win_rate =
        priced.iter().map(|p| p.count * p.stake * p.win_prob).sum::<f64>() / total_stake;

    let mix_legs = priced
        .iter()
        .map(|p| {
            let staked = p.count * p.stake;
            Ok(MixLegResult {
                american: to_american(p.decimal)?,
                decimal: p.decimal,
                count: p.bets,
                win_prob: p.win_prob,
                breakeven: 1.0 / p.decimal,
                total_stake: staked,
                stake_share: staked / total_stake,
                ev: p.ev,
                variance_share: if total_variance > 0.0 {
                    p.variance / total_variance
                } else {
                    0.0
                },
            })
        })
        .collect::<Result<Vec<_>>>()?;

    #[allow(clippy::cast_precision_loss, reason = "bet count")]
    let bet_count = bets as f64;

    Ok(BetMix {
        legs: mix_legs,
        bets,
        total_stake,
        ev,
        roi: ev / total_stake,
        blended_breakeven: total_stake / payout_weighted,
        blended_win_rate,
        sd,
        sd_per_unit: sd / total_stake,
        t_stat: if sd > 0.0 { ev / sd } else { f64::NAN },
        bets_to_detect: detection_horizon(ev, sd, bet_count),
    })
}

// ---------------------------------------------------------------------------
// Season simulation
// ---------------------------------------------------------------------------

/// A season to simulate: a bet mix, a bankroll, and a number of runs.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SeasonInput {
    /// The bets that make up one season. Their counts set its length.
    pub legs: Vec<MixLeg>,
    /// Starting bankroll.
    pub bankroll: f64,
    /// How many independent seasons to run.
    pub num_sims: usize,
    /// Whether a bankroll that cannot cover the next stake stops the season.
    ///
    /// Off by default in the UI: the fan chart is about the shape of an
    /// ordinary year, and a ruin barrier truncates exactly the paths that make
    /// the point. [`crate::risk_of_ruin`] is the module for the barrier.
    ///
    /// With it off the season plays every bet regardless, so a path may finish
    /// below zero. That is deliberate — it keeps the fan an unbiased picture of
    /// the profit distribution instead of one censored at the bottom — but it
    /// means a negative ending is a bookkeeping figure, not a real bankroll.
    /// [`SeasonResult::ruin_prob`] still reports how often it happened.
    pub stop_at_ruin: bool,
}

/// The percentile band across all paths at one point in the season.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct FanPoint {
    /// Bets placed so far.
    pub bet: usize,
    /// 5th percentile bankroll.
    pub p05: f64,
    /// 25th percentile bankroll.
    pub p25: f64,
    /// Median bankroll.
    pub median: f64,
    /// 75th percentile bankroll.
    pub p75: f64,
    /// 95th percentile bankroll.
    pub p95: f64,
}

/// What a season looked like across every simulated run.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SeasonResult {
    /// Seed that produced this run. Feed it back to reproduce exactly.
    ///
    /// Crosses the wire as a decimal string — see [`crate::seed_repr`].
    #[serde(serialize_with = "crate::seed_repr::serialize")]
    #[cfg_attr(feature = "specta", specta(type = String))]
    pub seed: u64,
    /// Bets in one season.
    pub bets: usize,
    /// Bankroll every season started from.
    ///
    /// Echoed back because it is the line between a winning year and a losing
    /// one: a chart that drew that line from a live input field would move it
    /// the moment the field was edited, and quietly relabel a finished run.
    pub starting_bankroll: f64,
    /// The closed-form summary of the same mix, for comparison with the fan.
    pub mix: BetMix,
    /// Percentile bands over the season, for the fan chart.
    pub fan: Vec<FanPoint>,

    /// Fraction of seasons that ended below the starting bankroll, 0–1.
    ///
    /// The headline. A real edge loses money over a season far more often than
    /// people expect, and the figure grows with the length of the prices.
    pub losing_season_prob: f64,
    /// Fraction of seasons that could not cover a stake at some point, 0–1.
    pub ruin_prob: f64,

    /// 5th percentile ending bankroll.
    pub ending_p05: f64,
    /// Median ending bankroll.
    pub ending_median: f64,
    /// 95th percentile ending bankroll.
    pub ending_p95: f64,
    /// Mean ending bankroll. Compare with the median: the gap is the skew.
    pub ending_mean: f64,

    /// Median worst peak-to-trough drop, as a fraction of the peak.
    pub median_max_drawdown: f64,
    /// 95th percentile of the same — the bad-but-not-unthinkable year.
    pub p95_max_drawdown: f64,
    /// Longest run of consecutive losses seen in any season.
    pub longest_losing_streak: usize,
}

/// One simulated season.
struct Season {
    ending: f64,
    ruined: bool,
    max_drawdown: f64,
    longest_losing_streak: usize,
    /// Bankroll at each sample point.
    balance_at: Vec<f64>,
}

/// Chooses the bet indices at which to sample the fan.
fn sample_points(bets: usize) -> Vec<usize> {
    let target = bets.min(200);
    let interval = (bets / target.max(1)).max(1);
    let mut points: Vec<usize> = (1..=bets).step_by(interval).collect();
    if points.last() != Some(&bets) {
        points.push(bets);
    }
    points
}

/// Linear-interpolated percentile of an ascending slice.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "position is bounded by the slice length, which is a path count"
)]
fn percentile(sorted: &[f64], q: f64) -> f64 {
    match sorted.len() {
        0 => 0.0,
        1 => sorted.first().copied().unwrap_or(0.0),
        n => {
            let pos = q.clamp(0.0, 1.0) * (n - 1) as f64;
            let lo = pos.floor();
            let a = sorted.get(lo as usize).copied().unwrap_or(0.0);
            let b = sorted.get(lo as usize + 1).copied().unwrap_or(a);
            a + (b - a) * (pos - lo)
        }
    }
}

/// A single bet in the shuffled season.
#[derive(Clone, Copy)]
struct Wager {
    stake: f64,
    win_payout: f64,
    win_prob: f64,
}

fn run_one_season(
    schedule: &[Wager],
    bankroll: f64,
    stop_at_ruin: bool,
    points: &[usize],
    seed: u64,
) -> Season {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    // Order matters to drawdown and to streaks but not to the ending balance,
    // so each season plays the same bets in its own order rather than in the
    // order they were typed.
    let mut order: Vec<usize> = (0..schedule.len()).collect();
    order.shuffle(&mut rng);

    let mut balance = bankroll;
    let mut peak = bankroll;
    let mut max_drawdown = 0.0_f64;
    let mut losing_streak = 0_usize;
    let mut longest_losing_streak = 0_usize;
    let mut balance_at = vec![balance; points.len()];
    let mut point_idx = 0_usize;
    let mut ruined = false;

    for (placed, idx) in order.iter().enumerate() {
        let bet = placed + 1;
        if let Some(w) = schedule.get(*idx) {
            if balance < w.stake {
                ruined = true;
                if stop_at_ruin {
                    // Every remaining sample point holds the final balance.
                    for slot in balance_at.iter_mut().skip(point_idx) {
                        *slot = balance;
                    }
                    break;
                }
            }
            if rng.random::<f64>() < w.win_prob {
                balance += w.win_payout;
                losing_streak = 0;
            } else {
                balance -= w.stake;
                losing_streak += 1;
                longest_losing_streak = longest_losing_streak.max(losing_streak);
            }
            peak = peak.max(balance);
            if peak > 0.0 {
                max_drawdown = max_drawdown.max((peak - balance) / peak);
            }
        }

        if points.get(point_idx) == Some(&bet) {
            if let Some(slot) = balance_at.get_mut(point_idx) {
                *slot = balance;
            }
            point_idx += 1;
        }
    }

    Season {
        ending: balance,
        ruined,
        max_drawdown,
        longest_losing_streak,
        balance_at,
    }
}

/// Simulates a season of the given bet mix, many times over.
///
/// The fan chart this produces is the argument the rest of the app cannot
/// make: a genuinely profitable mix spends a large fraction of its seasons
/// underwater, and how large depends on the prices, not on the edge.
///
/// # Errors
///
/// Everything [`bet_mix`] rejects, plus [`MathError::DomainError`] for a
/// non-positive bankroll and [`MathError::ShapeError`] for zero paths or a
/// season longer than 200,000 bets.
pub fn simulate_season(input: &SeasonInput, seed: u64) -> Result<SeasonResult> {
    let mix = bet_mix(&input.legs)?;

    if !input.bankroll.is_finite() || input.bankroll <= 0.0 {
        return Err(MathError::DomainError {
            param: "bankroll",
            constraint: "greater than zero",
            value: input.bankroll,
        });
    }
    if input.num_sims == 0 {
        return Err(MathError::ShapeError {
            what: "seasons",
            expected: "at least 1",
            got: 0,
        });
    }
    if mix.bets > MAX_SEASON_BETS {
        return Err(MathError::ShapeError {
            what: "bets in a season",
            expected: "at most 200000",
            got: mix.bets,
        });
    }
    let wagers = mix.bets.saturating_mul(input.num_sims);
    if wagers > MAX_SIMULATED_WAGERS {
        return Err(MathError::ShapeError {
            what: "bets × seasons",
            expected: "at most 50000000",
            got: wagers,
        });
    }

    let mut schedule: Vec<Wager> = Vec::with_capacity(mix.bets);
    for leg in &input.legs {
        let decimal = american_to_decimal(leg.american)?;
        let win_prob = win_prob_for_edge(decimal, leg.edge)?;
        let wager = Wager {
            stake: leg.stake,
            win_payout: leg.stake * (decimal - 1.0),
            win_prob,
        };
        schedule.extend(std::iter::repeat_n(wager, leg.count));
    }

    let points = sample_points(mix.bets);
    #[cfg(feature = "parallel")]
    let seasons: Vec<Season> = {
        use rayon::prelude::*;
        (0..input.num_sims)
            .into_par_iter()
            .map(|i| {
                run_one_season(
                    &schedule,
                    input.bankroll,
                    input.stop_at_ruin,
                    &points,
                    seed.wrapping_add(i as u64),
                )
            })
            .collect()
    };
    #[cfg(not(feature = "parallel"))]
    let seasons: Vec<Season> = (0..input.num_sims)
        .map(|i| {
            run_one_season(
                &schedule,
                input.bankroll,
                input.stop_at_ruin,
                &points,
                seed.wrapping_add(i as u64),
            )
        })
        .collect();

    #[allow(clippy::cast_precision_loss, reason = "path count")]
    let sims = input.num_sims as f64;

    let fan = std::iter::once(FanPoint {
        bet: 0,
        p05: input.bankroll,
        p25: input.bankroll,
        median: input.bankroll,
        p75: input.bankroll,
        p95: input.bankroll,
    })
    .chain(points.iter().enumerate().map(|(i, bet)| {
        let mut column: Vec<f64> = seasons
            .iter()
            .map(|s| s.balance_at.get(i).copied().unwrap_or(input.bankroll))
            .collect();
        column.sort_by(f64::total_cmp);
        FanPoint {
            bet: *bet,
            p05: percentile(&column, 0.05),
            p25: percentile(&column, 0.25),
            median: percentile(&column, 0.50),
            p75: percentile(&column, 0.75),
            p95: percentile(&column, 0.95),
        }
    }))
    .collect();

    let mut endings: Vec<f64> = seasons.iter().map(|s| s.ending).collect();
    endings.sort_by(f64::total_cmp);
    let mut drawdowns: Vec<f64> = seasons.iter().map(|s| s.max_drawdown).collect();
    drawdowns.sort_by(f64::total_cmp);

    #[allow(clippy::cast_precision_loss, reason = "path count")]
    let losing = seasons.iter().filter(|s| s.ending < input.bankroll).count() as f64;
    #[allow(clippy::cast_precision_loss, reason = "path count")]
    let ruined = seasons.iter().filter(|s| s.ruined).count() as f64;

    Ok(SeasonResult {
        seed,
        bets: mix.bets,
        starting_bankroll: input.bankroll,
        fan,
        losing_season_prob: losing / sims,
        ruin_prob: ruined / sims,
        ending_p05: percentile(&endings, 0.05),
        ending_median: percentile(&endings, 0.50),
        ending_p95: percentile(&endings, 0.95),
        ending_mean: endings.iter().sum::<f64>() / sims,
        median_max_drawdown: percentile(&drawdowns, 0.50),
        p95_max_drawdown: percentile(&drawdowns, 0.95),
        longest_losing_streak: seasons
            .iter()
            .map(|s| s.longest_losing_streak)
            .max()
            .unwrap_or(0),
        mix,
    })
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

    fn leg(american: f64, edge: f64, count: usize) -> MixLeg {
        MixLeg {
            american,
            stake: 100.0,
            edge,
            count,
        }
    }

    // --- moments -----------------------------------------------------------

    #[test]
    fn variance_at_zero_edge_is_exactly_decimal_minus_one() {
        // A closed form worth pinning: at the breakeven win rate the variance
        // of a one-unit bet is b, the amount you stand to win. It is why a
        // longshot's ride is longer in the most literal way available.
        for american in [-500.0, -110.0, 100.0, 250.0, 1000.0] {
            let d = american_to_decimal(american).unwrap();
            let (mean, var) = moments(d, 1.0 / d);
            assert_relative_eq!(mean, 0.0, epsilon = 1e-12);
            assert_relative_eq!(var, d - 1.0, epsilon = 1e-9);
        }
    }

    #[test]
    fn the_headline_table_in_the_module_doc_is_accurate() {
        let rungs = breakeven_ladder(&[-110.0, 400.0], 0.05).unwrap();

        assert_relative_eq!(rungs[0].sd_per_unit, 0.95, epsilon = 5e-3);
        assert_relative_eq!(rungs[1].sd_per_unit, 2.04, epsilon = 5e-3);

        let short = rungs[0].bets_to_detect.unwrap();
        let long = rungs[1].bets_to_detect.unwrap();
        assert_relative_eq!(short, 1_440.0, epsilon = 10.0);
        assert_relative_eq!(long, 6_640.0, epsilon = 10.0);

        // Same edge, four and a half times the sample.
        assert_relative_eq!(long / short, 4.6, epsilon = 0.1);
    }

    // --- breakeven ladder --------------------------------------------------

    #[test]
    fn breakeven_is_the_implied_probability() {
        let rungs = breakeven_ladder(&[-110.0, 100.0, 250.0], 0.0).unwrap();
        assert_relative_eq!(rungs[0].breakeven, 0.5238, epsilon = 5e-5);
        assert_relative_eq!(rungs[1].breakeven, 0.5, epsilon = 1e-12);
        assert_relative_eq!(rungs[2].breakeven, 0.2857, epsilon = 5e-5);
        // At a zero target the required rate is the breakeven rate.
        for r in &rungs {
            assert_relative_eq!(r.required_win_rate, r.breakeven, epsilon = 1e-12);
            assert_relative_eq!(r.cushion, 0.0, epsilon = 1e-12);
            assert!(r.bets_to_detect.is_none(), "no edge, nothing to confirm");
        }
    }

    #[test]
    fn the_cushion_shrinks_as_the_price_lengthens() {
        // The same edge asks for less of a win-rate surplus at long prices,
        // which is why a longshot edge sounds modest and proves brutal.
        let rungs = breakeven_ladder(&[-110.0, 400.0], 0.05).unwrap();
        assert_relative_eq!(rungs[0].cushion, 0.0262, epsilon = 5e-4);
        assert_relative_eq!(rungs[1].cushion, 0.0100, epsilon = 5e-4);
        assert!(rungs[1].cushion < rungs[0].cushion);
    }

    #[test]
    fn required_win_rate_really_does_produce_the_target_edge() {
        for american in [-2000.0, -110.0, 100.0, 350.0, 2500.0] {
            let r = &breakeven_ladder(&[american], 0.04).unwrap()[0];
            let (mean, _) = moments(r.decimal, r.required_win_rate);
            assert_relative_eq!(mean, 0.04, epsilon = 1e-12);
        }
    }

    #[test]
    fn an_unreachable_edge_is_an_error_not_a_probability_above_one() {
        // -100000 is decimal 1.001; no win rate earns 50% on it.
        assert!(breakeven_ladder(&[-100_000.0], 0.5).is_err());
        assert!(breakeven_ladder(&[], 0.05).is_err());
        assert!(breakeven_ladder(&[-110.0], 1.5).is_err());
        assert!(breakeven_ladder(&[-50.0], 0.05).is_err());
    }

    #[test]
    fn a_realised_bucket_can_sit_on_the_minus_one_boundary() {
        // Every bet in the bucket lost — realised edge is exactly -1.
        let mix = bet_mix(&[leg(-110.0, -1.0, 40)]).unwrap();
        assert_relative_eq!(mix.legs[0].win_prob, 0.0, epsilon = 1e-12);

        let result = simulate_season(
            &SeasonInput {
                legs: vec![leg(-110.0, -1.0, 40)],
                bankroll: 10_000.0,
                num_sims: 100,
                stop_at_ruin: false,
            },
            1,
        )
        .unwrap();
        assert_relative_eq!(result.losing_season_prob, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn a_longshot_bucket_can_carry_an_edge_above_one() {
        // 100% wins at +600 → edge = decimal − 1 = 6.
        let mix = bet_mix(&[leg(600.0, 6.0, 10)]).unwrap();
        assert_relative_eq!(mix.legs[0].win_prob, 1.0, epsilon = 1e-12);
    }

    // --- CLV translator ----------------------------------------------------

    #[test]
    fn twenty_cents_is_worth_twenty_times_more_at_a_short_price() {
        let rungs = clv_ladder(&[-110.0, 900.0], 20.0).unwrap();
        assert_relative_eq!(rungs[0].prob_points, 0.04141, epsilon = 5e-5);
        assert_relative_eq!(rungs[1].prob_points, 0.00204, epsilon = 5e-5);
        assert!(rungs[0].prob_points > 20.0 * rungs[1].prob_points);
    }

    #[test]
    fn at_a_fixed_move_the_ratio_measure_understates_how_bad_the_longshot_is() {
        // Both measures prefer the short price here — a fixed cents move is
        // worth more at -110 by either yardstick. What the ratio hides is the
        // *size* of the gap: it falls off about four times too slowly.
        let rungs = clv_ladder(&[-110.0, 900.0], 20.0).unwrap();
        assert!(rungs[0].ratio > rungs[1].ratio);
        assert!(rungs[0].prob_points > rungs[1].prob_points);

        let points_falloff = rungs[0].prob_points / rungs[1].prob_points;
        let ratio_falloff = rungs[0].ratio / rungs[1].ratio;
        assert_relative_eq!(points_falloff, 20.3, epsilon = 0.3);
        assert_relative_eq!(ratio_falloff, 3.9, epsilon = 0.3);
        assert!(rungs[1].distortion > rungs[0].distortion);
    }

    #[test]
    fn quoting_cents_inverts_the_ranking_once_the_moves_differ() {
        // The inversion `clv.rs` documents, which needs unequal moves to show:
        // fifty cents on a longshot beats twenty on a favorite in cents, and
        // loses to it badly in the currency that compounds a bankroll.
        let favorite = &clv_ladder(&[-110.0], 20.0).unwrap()[0];
        let longshot = &clv_ladder(&[400.0], 50.0).unwrap()[0];

        assert!(longshot.ratio > favorite.ratio, "cents favor the longshot");
        assert!(
            longshot.prob_points < favorite.prob_points,
            "points favor the favorite"
        );
        assert_relative_eq!(favorite.prob_points, 0.0414, epsilon = 5e-4);
        assert_relative_eq!(longshot.prob_points, 0.0222, epsilon = 5e-4);
    }

    #[test]
    fn zero_cents_is_worth_nothing_by_either_measure() {
        let r = &clv_ladder(&[-140.0], 0.0).unwrap()[0];
        assert_relative_eq!(r.prob_points, 0.0, epsilon = 1e-12);
        assert_relative_eq!(r.ratio, 0.0, epsilon = 1e-12);
        assert!(r.distortion.is_nan());
    }

    #[test]
    fn a_move_through_the_pivot_is_still_priced_correctly() {
        let r = &clv_ladder(&[105.0], 20.0).unwrap()[0];
        assert_eq!(r.closed_american, -115);
        assert!(r.prob_points > 0.0);
    }

    // --- bet mix -----------------------------------------------------------

    #[test]
    fn a_single_price_mix_matches_the_closed_form() {
        let m = bet_mix(&[leg(-110.0, 0.05, 500)]).unwrap();
        assert_eq!(m.bets, 500);
        assert_relative_eq!(m.total_stake, 50_000.0, epsilon = 1e-9);
        assert_relative_eq!(m.ev, 2_500.0, epsilon = 1e-9);
        assert_relative_eq!(m.roi, 0.05, epsilon = 1e-12);
        assert_relative_eq!(m.blended_breakeven, 0.5238, epsilon = 5e-5);
        // Variance adds across independent bets, so SD grows with √n.
        let d = american_to_decimal(-110.0).unwrap();
        let (_, var) = moments(d, (1.0 + 0.05) / d);
        assert_relative_eq!(m.sd, (500.0 * var).sqrt() * 100.0, epsilon = 1e-6);
        assert_relative_eq!(m.legs[0].variance_share, 1.0, epsilon = 1e-12);
        assert_relative_eq!(m.legs[0].stake_share, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn a_longshot_bucket_takes_a_bigger_share_of_the_swing_than_of_the_money() {
        // The whole point of the builder. Equal money, equal edge, and the
        // long price supplies most of the variance.
        let m = bet_mix(&[leg(-110.0, 0.03, 100), leg(600.0, 0.03, 100)]).unwrap();
        assert_relative_eq!(m.legs[0].stake_share, 0.5, epsilon = 1e-12);
        assert_relative_eq!(m.legs[1].stake_share, 0.5, epsilon = 1e-12);
        assert!(
            m.legs[1].variance_share > 0.8,
            "longshot supplied only {} of the variance",
            m.legs[1].variance_share
        );
    }

    #[test]
    fn shares_sum_to_one() {
        let m = bet_mix(&[
            leg(-200.0, 0.02, 40),
            MixLeg {
                stake: 250.0,
                ..leg(150.0, 0.04, 25)
            },
            leg(700.0, 0.06, 10),
        ])
        .unwrap();
        assert_relative_eq!(
            m.legs.iter().map(|l| l.stake_share).sum::<f64>(),
            1.0,
            epsilon = 1e-12
        );
        assert_relative_eq!(
            m.legs.iter().map(|l| l.variance_share).sum::<f64>(),
            1.0,
            epsilon = 1e-12
        );
        assert_relative_eq!(m.ev, m.legs.iter().map(|l| l.ev).sum::<f64>(), epsilon = 1e-9);
    }

    #[test]
    fn the_blended_breakeven_really_does_break_the_mix_even() {
        let legs = [leg(-200.0, 0.02, 40), leg(150.0, 0.04, 25), leg(700.0, 0.06, 10)];
        let m = bet_mix(&legs).unwrap();

        // Apply the blended rate uniformly and the mix should return zero.
        let profit: f64 = legs
            .iter()
            .map(|l| {
                let d = american_to_decimal(l.american).unwrap();
                #[allow(clippy::cast_precision_loss, reason = "test bet count")]
                let n = l.count as f64;
                n * l.stake * (m.blended_breakeven * d - 1.0)
            })
            .sum();
        assert_relative_eq!(profit, 0.0, epsilon = 1e-9);
    }

    #[test]
    fn detection_takes_longer_at_long_prices_for_the_same_roi() {
        let short = bet_mix(&[leg(-110.0, 0.05, 1_000)]).unwrap();
        let long = bet_mix(&[leg(600.0, 0.05, 1_000)]).unwrap();
        assert_relative_eq!(short.roi, long.roi, epsilon = 1e-12);
        assert!(short.t_stat > long.t_stat);
        assert!(long.bets_to_detect.unwrap() > 4.0 * short.bets_to_detect.unwrap());
    }

    #[test]
    fn a_losing_mix_never_confirms_itself() {
        let m = bet_mix(&[leg(-110.0, -0.05, 1_000)]).unwrap();
        assert!(m.ev < 0.0);
        assert!(m.bets_to_detect.is_none());
    }

    #[test]
    fn the_mix_rejects_bad_input() {
        assert!(bet_mix(&[]).is_err());
        assert!(bet_mix(&[leg(-110.0, 0.05, 0)]).is_err());
        assert!(bet_mix(&[leg(-50.0, 0.05, 10)]).is_err());
        assert!(bet_mix(&[leg(-110.0, 2.0, 10)]).is_err());
        assert!(bet_mix(&[MixLeg {
            stake: 0.0,
            ..leg(-110.0, 0.05, 10)
        }])
        .is_err());
    }

    // --- season simulation -------------------------------------------------

    fn season(legs: Vec<MixLeg>, sims: usize) -> SeasonInput {
        SeasonInput {
            legs,
            bankroll: 10_000.0,
            num_sims: sims,
            stop_at_ruin: false,
        }
    }

    #[test]
    fn the_same_seed_reproduces_the_season_exactly() {
        let s = season(vec![leg(-110.0, 0.03, 200)], 500);
        let a = simulate_season(&s, 4_242).unwrap();
        let b = simulate_season(&s, 4_242).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.seed, 4_242);
    }

    #[test]
    fn the_simulated_ending_converges_on_the_closed_form() {
        // The simulation and `bet_mix` must agree, or one of them is wrong.
        let s = season(vec![leg(-110.0, 0.04, 300)], 4_000);
        let r = simulate_season(&s, 11).unwrap();
        let expected = s.bankroll + r.mix.ev;
        // Standard error of the mean across 4,000 paths.
        let tolerance = 4.0 * r.mix.sd / 4_000.0_f64.sqrt();
        assert_relative_eq!(r.ending_mean, expected, epsilon = tolerance);
    }

    #[test]
    fn a_real_edge_still_loses_money_in_a_lot_of_seasons() {
        // The figure the fan chart exists to show. A 3% edge over 200 bets is
        // genuinely profitable either way, and loses money in 32% of seasons
        // at -110 against 45% at +600 — a coin flip, on bets that are right.
        let short = simulate_season(&season(vec![leg(-110.0, 0.03, 200)], 3_000), 7).unwrap();
        let long = simulate_season(&season(vec![leg(600.0, 0.03, 200)], 3_000), 7).unwrap();

        assert!(short.mix.ev > 0.0 && long.mix.ev > 0.0);
        assert_relative_eq!(short.losing_season_prob, 0.32, epsilon = 0.02);
        assert_relative_eq!(long.losing_season_prob, 0.45, epsilon = 0.02);
        assert!(long.losing_season_prob > short.losing_season_prob);
    }

    #[test]
    fn longshot_seasons_draw_down_deeper_and_lose_longer() {
        let short = simulate_season(&season(vec![leg(-110.0, 0.03, 300)], 1_500), 3).unwrap();
        let long = simulate_season(&season(vec![leg(600.0, 0.03, 300)], 1_500), 3).unwrap();
        assert!(long.median_max_drawdown > short.median_max_drawdown);
        assert!(long.p95_max_drawdown > short.p95_max_drawdown);
        assert!(long.longest_losing_streak > 3 * short.longest_losing_streak);
    }

    #[test]
    fn the_fan_starts_closed_and_opens() {
        let r = simulate_season(&season(vec![leg(-110.0, 0.03, 400)], 1_000), 5).unwrap();
        let first = r.fan.first().unwrap();
        assert_eq!(first.bet, 0);
        assert_relative_eq!(first.p05, 10_000.0, epsilon = 1e-12);
        assert_relative_eq!(first.p95, 10_000.0, epsilon = 1e-12);

        let last = r.fan.last().unwrap();
        assert_eq!(last.bet, 400);
        assert!(last.p95 - last.p05 > 1_000.0, "the fan never opened");

        for p in &r.fan {
            assert!(p.p05 <= p.p25 && p.p25 <= p.median);
            assert!(p.median <= p.p75 && p.p75 <= p.p95);
        }
    }

    #[test]
    fn ruin_is_only_reported_when_the_bankroll_can_actually_run_out() {
        let deep = simulate_season(&season(vec![leg(-110.0, 0.03, 200)], 500), 1).unwrap();
        assert_relative_eq!(deep.ruin_prob, 0.0, epsilon = 1e-12);

        let thin = SeasonInput {
            bankroll: 500.0,
            stop_at_ruin: true,
            ..season(vec![leg(600.0, -0.05, 200)], 500)
        };
        let r = simulate_season(&thin, 1).unwrap();
        assert!(r.ruin_prob > 0.3, "got {}", r.ruin_prob);
    }

    #[test]
    fn the_season_rejects_bad_input() {
        assert!(simulate_season(&season(vec![], 100), 1).is_err());
        assert!(simulate_season(&season(vec![leg(-110.0, 0.03, 10)], 0), 1).is_err());
        assert!(simulate_season(
            &SeasonInput {
                bankroll: 0.0,
                ..season(vec![leg(-110.0, 0.03, 10)], 100)
            },
            1
        )
        .is_err());
        assert!(simulate_season(&season(vec![leg(-110.0, 0.03, 500_000)], 2), 1).is_err());
    }

    #[test]
    fn a_run_too_large_to_finish_is_refused_rather_than_started() {
        // Each factor is legal on its own; the product is two billion wagers.
        let huge = season(vec![leg(-110.0, 0.03, 100_000)], 10_000);
        assert!(simulate_season(&huge, 1).is_err());

        // And the same season at a sane number of runs still works.
        let fine = season(vec![leg(-110.0, 0.03, 100_000)], 100);
        assert!(simulate_season(&fine, 1).is_ok());
    }

    #[test]
    fn every_leg_reports_its_own_bet_count() {
        // Guards a mislabelling failure rather than a numeric one: a caller
        // that lined its own input list up against these rows would shift
        // every label as soon as one leg was skipped.
        let m = bet_mix(&[leg(-110.0, 0.02, 300), leg(600.0, 0.05, 40)]).unwrap();
        assert_eq!(m.legs[0].count, 300);
        assert_eq!(m.legs[1].count, 40);
        assert_eq!(m.legs.iter().map(|l| l.count).sum::<usize>(), m.bets);
    }
}
