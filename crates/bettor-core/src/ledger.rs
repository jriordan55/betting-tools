//! Reading a real betting record: what it returned, and what it was worth.
//!
//! New here. The storage lives in the desktop shell — this module never sees a
//! database, only a slice of bets — so the arithmetic that decides whether a
//! season was good stays testable without one.
//!
//! # The distinction this module is built around
//!
//! A record has two numbers and they answer different questions.
//!
//! **Profit** says what happened. **Expected profit**, measured against the
//! devigged closing line, says what should have happened. Their difference is
//! variance, and over any realistic number of bets it is large enough to
//! reverse the sign of the first number while leaving the second untouched.
//!
//! A bettor with a real edge and a losing year, and a bettor with no edge and a
//! winning one, are both common. [`LedgerSummary::luck`] is that gap, stated
//! rather than left for the reader to infer from two rows that look unrelated.
//!
//! # Closing lines, and why the opposing price matters
//!
//! Closing line value is only measurable against a *fair* closing probability,
//! which needs both sides of the closing market. Given only the side you bet,
//! the price still carries the book's margin, and every edge computed from it
//! is overstated by roughly half the hold. [`BetAnalysis::ev`] is therefore
//! `None` unless the opposing close was recorded — see [`crate::clv`], where
//! the same rule applies to a single bet.

use crate::odds::{american_from_cents, american_to_decimal, cents_between, cents_from_even};
use crate::variance::MixLeg;
use crate::{MathError, Result};
use serde::{Deserialize, Serialize};

/// How a bet finished.
///
/// Exported as `BetOutcome`: TypeScript has one flat namespace and
/// [`crate::middle::Outcome`] is already in it. Same reason `middle::Leg` and
/// `arbitrage::Leg` are disambiguated on export — the Rust names stay
/// idiomatic, and the collision is resolved once, here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[cfg_attr(feature = "specta", specta(rename = "BetOutcome"))]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    /// Not settled yet. Counted in the book, excluded from every return figure.
    Pending,
    /// Won.
    Won,
    /// Lost.
    Lost,
    /// Pushed — stake returned.
    Push,
    /// Voided or cancelled — stake returned.
    Void,
}

impl Outcome {
    /// Whether the bet has settled.
    #[must_use]
    pub fn is_settled(self) -> bool {
        self != Self::Pending
    }

    /// Whether the bet was decided one way or the other.
    ///
    /// A push is settled but not decided, and including it in a win rate
    /// quietly understates one.
    #[must_use]
    pub fn is_decided(self) -> bool {
        matches!(self, Self::Won | Self::Lost)
    }
}

/// One bet as it was recorded.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LoggedBet {
    /// American price taken.
    pub price_taken: f64,
    /// American price this side closed at, when it was recorded.
    pub closing_price: Option<f64>,
    /// American closing price on the *other* side of the same market.
    ///
    /// Without it the closing price cannot be devigged, so no honest edge can
    /// be derived from it.
    pub opposing_closing_price: Option<f64>,
    /// Money risked.
    pub stake: f64,
    /// How it finished.
    pub outcome: Outcome,
}

/// What one bet was worth, in every sense that can be measured.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct BetAnalysis {
    /// Decimal price taken.
    pub decimal_taken: f64,
    /// Probability the price taken implies, 0–1. Contains vig.
    pub implied_taken: f64,
    /// Realised profit. `None` while the bet is pending.
    pub profit: Option<f64>,

    /// Probability points gained against the raw closing price.
    ///
    /// Available whenever a closing price was recorded, because it compares
    /// two vigged numbers and the margins largely cancel.
    pub clv_points: Option<f64>,
    /// The same move in American cents, measured on the cents axis.
    pub clv_cents: Option<f64>,

    /// Fair closing probability, 0–1. `None` without the opposing close.
    pub fair_prob: Option<f64>,
    /// Edge against the fair close, as a fraction of stake.
    pub ev: Option<f64>,
    /// The same edge in currency.
    pub ev_dollars: Option<f64>,
}

/// A whole betting record, summarised.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LedgerSummary {
    /// Bets in the record.
    pub bets: usize,
    /// Bets that have settled.
    pub settled: usize,
    /// Bets still open.
    pub pending: usize,
    /// Bets won.
    pub won: usize,
    /// Bets lost.
    pub lost: usize,
    /// Bets pushed or voided.
    pub pushed: usize,

    /// Money risked on settled bets.
    pub staked: f64,
    /// Realised profit.
    pub profit: f64,
    /// Profit as a fraction of money risked.
    pub roi: f64,
    /// Share of *decided* bets won, 0–1. Pushes excluded.
    pub win_rate: f64,

    /// Settled bets that also have a closing price recorded.
    pub with_close: usize,
    /// Mean probability points gained against the close.
    ///
    /// `None` when no bet in the record has a closing price.
    pub mean_clv_points: Option<f64>,
    /// Share of those bets that beat the close, 0–1.
    pub beat_close_rate: Option<f64>,

    /// Settled bets whose closing market was recorded on both sides.
    pub with_fair_close: usize,
    /// Profit the record should have produced, against the fair close.
    ///
    /// `None` when no bet has both closing prices.
    pub expected_profit: Option<f64>,
    /// Expected profit as a fraction of the money those bets risked.
    pub expected_roi: Option<f64>,
    /// Realised profit minus expected profit.
    ///
    /// Variance, named. Positive means the record ran better than the prices
    /// it took deserved; negative means worse. It is normally the largest
    /// number on this list, and treating it as skill in either direction is
    /// the most common way a betting record is misread.
    pub luck: Option<f64>,
}

/// A record, bet by bet and in total.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Ledger {
    /// One entry per input bet, in the order supplied.
    pub bets: Vec<BetAnalysis>,
    /// The whole record.
    pub summary: LedgerSummary,
}

fn check_stake(stake: f64) -> Result<()> {
    if !stake.is_finite() || stake <= 0.0 {
        return Err(MathError::DomainError {
            param: "stake",
            constraint: "greater than zero",
            value: stake,
        });
    }
    Ok(())
}

/// Analyses one bet.
///
/// # Errors
///
/// [`MathError::DomainError`] for a non-positive stake or a price under 100 in
/// magnitude.
pub fn analyze_bet(bet: &LoggedBet) -> Result<BetAnalysis> {
    check_stake(bet.stake)?;
    let decimal_taken = american_to_decimal(bet.price_taken)?;
    let implied_taken = 1.0 / decimal_taken;

    let profit = match bet.outcome {
        Outcome::Pending => None,
        Outcome::Won => Some(bet.stake * (decimal_taken - 1.0)),
        Outcome::Lost => Some(-bet.stake),
        Outcome::Push | Outcome::Void => Some(0.0),
    };

    let (clv_points, clv_cents, closing_implied) = match bet.closing_price {
        Some(close) => {
            let closing_decimal = american_to_decimal(close)?;
            let closing_implied = 1.0 / closing_decimal;
            (
                Some(closing_implied - implied_taken),
                Some(cents_between(bet.price_taken, close)?),
                Some(closing_implied),
            )
        }
        None => (None, None, None),
    };

    // Devigging needs both sides. Without the opposing close the closing price
    // still carries the margin, and an edge derived from it is overstated by
    // roughly half the hold — so there is no figure to report, not a smaller one.
    let (fair_prob, ev, ev_dollars) = match (closing_implied, bet.opposing_closing_price) {
        (Some(mine), Some(other)) => {
            let other_implied = 1.0 / american_to_decimal(other)?;
            let total = mine + other_implied;
            if total <= 0.0 {
                return Err(MathError::DomainError {
                    param: "closing market total",
                    constraint: "positive",
                    value: total,
                });
            }
            let fair = mine / total;
            let edge = fair * decimal_taken - 1.0;
            (Some(fair), Some(edge), Some(edge * bet.stake))
        }
        _ => (None, None, None),
    };

    Ok(BetAnalysis {
        decimal_taken,
        implied_taken,
        profit,
        clv_points,
        clv_cents,
        fair_prob,
        ev,
        ev_dollars,
    })
}

/// Analyses a whole record.
///
/// An empty record is not an error — a bet log starts empty, and a zeroed
/// summary is the honest description of one.
///
/// # Errors
///
/// Whatever [`analyze_bet`] rejects, on the first bet that fails.
pub fn analyze(bets: &[LoggedBet]) -> Result<Ledger> {
    let rows: Vec<BetAnalysis> = bets.iter().map(analyze_bet).collect::<Result<_>>()?;

    let mut settled = 0_usize;
    let mut won = 0_usize;
    let mut lost = 0_usize;
    let mut pushed = 0_usize;
    let mut decided = 0_usize;
    let mut staked = 0.0_f64;
    let mut profit = 0.0_f64;

    let mut with_close = 0_usize;
    let mut clv_total = 0.0_f64;
    let mut beat_close = 0_usize;

    let mut with_fair_close = 0_usize;
    let mut fair_staked = 0.0_f64;
    let mut expected = 0.0_f64;
    let mut realised_on_fair = 0.0_f64;

    for (bet, row) in bets.iter().zip(&rows) {
        if !bet.outcome.is_settled() {
            continue;
        }
        settled += 1;
        staked += bet.stake;
        profit += row.profit.unwrap_or(0.0);

        match bet.outcome {
            Outcome::Won => {
                won += 1;
                decided += 1;
            }
            Outcome::Lost => {
                lost += 1;
                decided += 1;
            }
            Outcome::Push | Outcome::Void => pushed += 1,
            Outcome::Pending => {}
        }

        if let Some(points) = row.clv_points {
            with_close += 1;
            clv_total += points;
            if points > 0.0 {
                beat_close += 1;
            }
        }

        if let Some(ev) = row.ev_dollars {
            with_fair_close += 1;
            fair_staked += bet.stake;
            expected += ev;
            // Luck must compare like with like: only the bets that have an
            // expectation may contribute to the realised side of the gap.
            realised_on_fair += row.profit.unwrap_or(0.0);
        }
    }

    #[allow(clippy::cast_precision_loss, reason = "bet counts")]
    let (decided_f, with_close_f, beat_close_f, won_f) = (
        decided as f64,
        with_close as f64,
        beat_close as f64,
        won as f64,
    );

    Ok(Ledger {
        summary: LedgerSummary {
            bets: bets.len(),
            settled,
            pending: bets.len() - settled,
            won,
            lost,
            pushed,
            staked,
            profit,
            roi: if staked > 0.0 { profit / staked } else { 0.0 },
            win_rate: if decided > 0 { won_f / decided_f } else { 0.0 },
            with_close,
            mean_clv_points: (with_close > 0).then(|| clv_total / with_close_f),
            beat_close_rate: (with_close > 0).then(|| beat_close_f / with_close_f),
            with_fair_close,
            expected_profit: (with_fair_close > 0).then_some(expected),
            expected_roi: (with_fair_close > 0 && fair_staked > 0.0)
                .then(|| expected / fair_staked),
            luck: (with_fair_close > 0).then_some(realised_on_fair - expected),
        },
        bets: rows,
    })
}

/// A real record, reshaped into the input the variance module takes.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LedgerMix {
    /// One leg per price bucket, shortest price first.
    pub legs: Vec<MixLeg>,
    /// Bets that contributed.
    pub bets_used: usize,
    /// Bets skipped because no fair closing line was recorded for them.
    ///
    /// Reported rather than quietly dropped: a mix built from a third of a
    /// record is a different claim than one built from all of it.
    pub bets_skipped: usize,
}

/// Buckets a record by price so [`crate::variance`] can model the season it implies.
///
/// The edge assigned to each bucket is the stake-weighted mean edge against
/// the *fair closing line*, not the realised return. That is the point: the
/// question a variance model answers is "given bets this good at prices this
/// long, what should a season look like", and realised profit is the answer
/// to a different one.
///
/// Bets without both closing prices are skipped, because they carry no edge
/// estimate at all and assuming zero would drag every bucket toward break even.
///
/// # Errors
///
/// [`MathError::DomainError`] for a non-positive bucket width or a bad bet;
/// [`MathError::ShapeError`] if no bet in the record has a fair closing line.
pub fn to_mix(bets: &[LoggedBet], bucket_cents: f64) -> Result<LedgerMix> {
    if !bucket_cents.is_finite() || bucket_cents <= 0.0 {
        return Err(MathError::DomainError {
            param: "bucket width",
            constraint: "greater than zero",
            value: bucket_cents,
        });
    }

    struct Bucket {
        key: i64,
        stake: f64,
        stake_cents: f64,
        stake_edge: f64,
        count: usize,
    }

    let mut buckets: Vec<Bucket> = Vec::new();
    let mut skipped = 0_usize;

    for bet in bets {
        let row = analyze_bet(bet)?;
        let Some(edge) = row.ev else {
            skipped += 1;
            continue;
        };

        let cents = cents_from_even(bet.price_taken)?;
        #[allow(
            clippy::cast_possible_truncation,
            reason = "prices are bounded well inside i64 at any sane bucket width"
        )]
        let key = (cents / bucket_cents).floor() as i64;

        if let Some(b) = buckets.iter_mut().find(|b| b.key == key) {
            b.stake += bet.stake;
            b.stake_cents += bet.stake * cents;
            b.stake_edge += bet.stake * edge;
            b.count += 1;
        } else {
            buckets.push(Bucket {
                key,
                stake: bet.stake,
                stake_cents: bet.stake * cents,
                stake_edge: bet.stake * edge,
                count: 1,
            });
        }
    }

    if buckets.is_empty() {
        return Err(MathError::ShapeError {
            what: "bets with a fair closing line",
            expected: "at least 1",
            got: 0,
        });
    }

    buckets.sort_by_key(|b| b.key);
    let bets_used = buckets.iter().map(|b| b.count).sum();

    let legs = buckets
        .iter()
        .map(|b| {
            #[allow(clippy::cast_precision_loss, reason = "bet count")]
            let count = b.count as f64;
            MixLeg {
                // The bucket's representative price is where its money
                // actually sat, not the middle of the range it fell in.
                american: american_from_cents(b.stake_cents / b.stake),
                stake: b.stake / count,
                edge: b.stake_edge / b.stake,
                count: b.count,
            }
        })
        .collect();

    Ok(LedgerMix {
        legs,
        bets_used,
        bets_skipped: skipped,
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

    fn bet(price: f64, outcome: Outcome) -> LoggedBet {
        LoggedBet {
            price_taken: price,
            closing_price: None,
            opposing_closing_price: None,
            stake: 100.0,
            outcome,
        }
    }

    fn with_close(price: f64, close: f64, opposing: f64, outcome: Outcome) -> LoggedBet {
        LoggedBet {
            closing_price: Some(close),
            opposing_closing_price: Some(opposing),
            ..bet(price, outcome)
        }
    }

    #[test]
    fn profit_follows_the_settlement_rules() {
        assert_relative_eq!(
            analyze_bet(&bet(-110.0, Outcome::Won)).unwrap().profit.unwrap(),
            90.909_090_9,
            epsilon = 1e-6
        );
        assert_relative_eq!(
            analyze_bet(&bet(-110.0, Outcome::Lost)).unwrap().profit.unwrap(),
            -100.0,
            epsilon = 1e-9
        );
        assert_relative_eq!(
            analyze_bet(&bet(-110.0, Outcome::Push)).unwrap().profit.unwrap(),
            0.0,
            epsilon = 1e-9
        );
        assert!(analyze_bet(&bet(-110.0, Outcome::Pending)).unwrap().profit.is_none());
    }

    #[test]
    fn a_push_is_settled_but_not_decided() {
        // Counting pushes as losses in a win rate is a quiet way to understate
        // one, and counting them as wins is worse.
        let l = analyze(&[
            bet(-110.0, Outcome::Won),
            bet(-110.0, Outcome::Lost),
            bet(-110.0, Outcome::Push),
        ])
        .unwrap();
        assert_eq!(l.summary.settled, 3);
        assert_eq!(l.summary.pushed, 1);
        assert_relative_eq!(l.summary.win_rate, 0.5, epsilon = 1e-12);
    }

    #[test]
    fn pending_bets_stay_out_of_every_return_figure() {
        let l = analyze(&[bet(-110.0, Outcome::Won), bet(500.0, Outcome::Pending)]).unwrap();
        assert_eq!(l.summary.bets, 2);
        assert_eq!(l.summary.pending, 1);
        assert_relative_eq!(l.summary.staked, 100.0, epsilon = 1e-9);
        assert_relative_eq!(l.summary.roi, 0.909_090_9, epsilon = 1e-6);
    }

    #[test]
    fn clv_needs_only_one_side_but_edge_needs_both() {
        let one_sided = LoggedBet {
            closing_price: Some(-130.0),
            ..bet(-110.0, Outcome::Won)
        };
        let row = analyze_bet(&one_sided).unwrap();
        assert_relative_eq!(row.clv_points.unwrap(), 0.0414, epsilon = 5e-4);
        assert_relative_eq!(row.clv_cents.unwrap(), 20.0, epsilon = 1e-9);
        assert!(row.ev.is_none(), "no opposing close, so no honest edge");
        assert!(row.fair_prob.is_none());
    }

    #[test]
    fn betting_the_closing_price_is_a_losing_bet_once_the_vig_is_removed() {
        // Bet -110 into a market that closed -110/-110. Zero CLV, and a
        // genuinely negative edge, because the fair price was even money.
        let row = analyze_bet(&with_close(-110.0, -110.0, -110.0, Outcome::Won)).unwrap();
        assert_relative_eq!(row.clv_points.unwrap(), 0.0, epsilon = 1e-12);
        assert_relative_eq!(row.fair_prob.unwrap(), 0.5, epsilon = 1e-12);
        assert_relative_eq!(row.ev.unwrap(), -0.045_454_5, epsilon = 1e-6);
    }

    #[test]
    fn luck_is_the_gap_between_what_happened_and_what_was_deserved() {
        // Four identical +EV bets; three of them lost. The record is deeply
        // negative and the bets were good — which is the entire point.
        let good = with_close(150.0, 120.0, -140.0, Outcome::Lost);
        let l = analyze(&[
            LoggedBet { outcome: Outcome::Won, ..good },
            good,
            good,
            good,
        ])
        .unwrap();

        assert!(l.summary.profit < 0.0, "got {}", l.summary.profit);
        assert!(
            l.summary.expected_profit.unwrap() > 0.0,
            "the bets carried a real edge"
        );
        assert!(l.summary.luck.unwrap() < 0.0);
        assert_relative_eq!(
            l.summary.luck.unwrap(),
            l.summary.profit - l.summary.expected_profit.unwrap(),
            epsilon = 1e-9
        );
        assert_relative_eq!(l.summary.beat_close_rate.unwrap(), 1.0, epsilon = 1e-12);
    }

    #[test]
    fn luck_compares_like_with_like() {
        // A bet with no closing line contributes to profit but not to
        // expectation. If it also contributed to the realised side of the gap,
        // luck would absorb it and stop meaning anything.
        let priced = with_close(150.0, 120.0, -140.0, Outcome::Lost);
        let unpriced = bet(150.0, Outcome::Won);
        let l = analyze(&[priced, unpriced]).unwrap();

        assert_eq!(l.summary.with_fair_close, 1);
        assert!(l.summary.profit > 0.0, "the unpriced winner dominates");
        assert_relative_eq!(
            l.summary.luck.unwrap(),
            -100.0 - l.summary.expected_profit.unwrap(),
            epsilon = 1e-9
        );
    }

    #[test]
    fn an_empty_record_summarises_to_zero_rather_than_erroring() {
        let l = analyze(&[]).unwrap();
        assert_eq!(l.summary.bets, 0);
        assert_relative_eq!(l.summary.roi, 0.0, epsilon = 1e-12);
        assert!(l.summary.mean_clv_points.is_none());
        assert!(l.summary.luck.is_none());
    }

    #[test]
    fn a_bad_bet_is_an_error_not_a_skipped_row() {
        assert!(analyze(&[bet(-50.0, Outcome::Won)]).is_err());
        assert!(analyze(&[LoggedBet { stake: 0.0, ..bet(-110.0, Outcome::Won) }]).is_err());
        assert!(analyze(&[LoggedBet {
            closing_price: Some(-10.0),
            ..bet(-110.0, Outcome::Won)
        }])
        .is_err());
    }

    // --- reshaping into a bet mix -----------------------------------------

    #[test]
    fn bets_at_similar_prices_land_in_one_bucket() {
        let l = to_mix(
            &[
                with_close(-110.0, -120.0, 100.0, Outcome::Won),
                with_close(-105.0, -115.0, -105.0, Outcome::Lost),
                with_close(600.0, 550.0, -750.0, Outcome::Lost),
            ],
            100.0,
        )
        .unwrap();

        assert_eq!(l.legs.len(), 2, "two clusters, not three");
        assert_eq!(l.bets_used, 3);
        assert_eq!(l.bets_skipped, 0);
        // Shortest price first, and the short bucket holds two bets.
        assert_eq!(l.legs[0].count, 2);
        assert_eq!(l.legs[1].count, 1);
        assert!(l.legs[0].american < 0.0 && l.legs[1].american > 0.0);
    }

    #[test]
    fn a_bucket_prices_itself_where_its_money_actually_sat() {
        // One bet at -110 and one at -190 in the same bucket: the
        // representative price is between them, not at the bucket's edge.
        let l = to_mix(
            &[
                with_close(-110.0, -120.0, 100.0, Outcome::Won),
                with_close(-190.0, -200.0, 160.0, Outcome::Won),
            ],
            200.0,
        )
        .unwrap();
        assert_eq!(l.legs.len(), 1);
        assert!(
            l.legs[0].american > -190.0 && l.legs[0].american < -110.0,
            "got {}",
            l.legs[0].american
        );
    }

    #[test]
    fn bets_without_a_fair_close_are_skipped_and_counted() {
        let l = to_mix(
            &[
                with_close(-110.0, -120.0, 100.0, Outcome::Won),
                bet(-110.0, Outcome::Won),
                LoggedBet {
                    closing_price: Some(-120.0),
                    ..bet(-110.0, Outcome::Won)
                },
            ],
            100.0,
        )
        .unwrap();
        assert_eq!(l.bets_used, 1);
        assert_eq!(l.bets_skipped, 2, "one with no close, one with only one side");
    }

    #[test]
    fn a_record_with_no_fair_closes_cannot_become_a_mix() {
        // Better than a mix of zero-edge legs, which would look like a
        // break-even bettor rather than like missing data.
        assert!(to_mix(&[bet(-110.0, Outcome::Won)], 100.0).is_err());
        assert!(to_mix(&[], 100.0).is_err());
        assert!(to_mix(&[with_close(-110.0, -120.0, 100.0, Outcome::Won)], 0.0).is_err());
    }

    #[test]
    fn the_mix_carries_the_edge_against_the_close_not_the_realised_return() {
        // Every bet lost, so the realised return is -100%. The mix must still
        // report the positive edge the closing lines say these bets had.
        let losers = [
            with_close(150.0, 120.0, -140.0, Outcome::Lost),
            with_close(150.0, 120.0, -140.0, Outcome::Lost),
        ];
        let l = to_mix(&losers, 100.0).unwrap();
        assert_eq!(l.legs.len(), 1);
        assert!(l.legs[0].edge > 0.0, "got {}", l.legs[0].edge);

        let realised = analyze(&losers).unwrap();
        assert!(realised.summary.roi < 0.0);
    }
}
