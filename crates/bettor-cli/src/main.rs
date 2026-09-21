//! One JSON object on stdin, one JSON object on stdout.
//!
//! `{"ok": ...}` or `{"error": "..."}`. The Streamlit app ships the
//! `wasm32-wasip1` build of this binary and does not reimplement the math.

mod import;

use std::io::{self, Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use bettor_core::ledger::{analyze, to_book_mix, to_mix, LedgerMix, LoggedBet};
use bettor_core::odds::{self, OddsFormat};
use bettor_core::risk_of_ruin::{self, RuinInput};
use bettor_core::variance::{self, MixLeg, SeasonInput};
use bettor_core::wager;
use import::import_csv;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "camelCase", rename_all_fields = "camelCase")]
enum Request {
    ImportCsv {
        csv: String,
    },
    Snapshot {
        bets: Vec<LoggedBet>,
    },
    Kelly {
        decimal: f64,
        true_prob: f64,
        bankroll: f64,
        multiplier: f64,
    },
    ExpectedValue {
        decimal: f64,
        true_prob: f64,
        stake: f64,
    },
    BreakevenLadder {
        prices: Vec<f64>,
        target_edge: f64,
    },
    BetMix {
        legs: Vec<MixLeg>,
    },
    SimulateSeason {
        input: SeasonInput,
        seed: String,
    },
    SimulateRuin {
        input: RuinInput,
        seed: String,
    },
    ConvertOdds {
        value: String,
        format: OddsFormat,
    },
    Parlay {
        american: Vec<f64>,
        stake: f64,
    },
    Hold {
        american_a: f64,
        american_b: f64,
    },
    PriceLadder {
        from: f64,
        to: f64,
        step_cents: f64,
    },
}

fn main() {
    let mut raw = String::new();
    if let Err(error) = io::stdin().read_to_string(&mut raw) {
        emit(&envelope_err(error.to_string()));
        return;
    }
    emit(&dispatch(&raw));
}

fn emit(message: &str) {
    let mut out = io::stdout();
    let _ = writeln!(out, "{message}");
    let _ = out.flush();
}

fn dispatch(raw: &str) -> String {
    let request: Request = match serde_json::from_str(raw) {
        Ok(request) => request,
        Err(error) => return envelope_err(format!("could not read request: {error}")),
    };
    match handle(request) {
        Ok(value) => json!({ "ok": value }).to_string(),
        Err(error) => envelope_err(error),
    }
}

fn handle(request: Request) -> Result<Value, String> {
    match request {
        Request::ImportCsv { csv } => {
            let batch = import_csv(&csv)?;
            let logged = logged_from_imported(&batch.bets);
            let snapshot = snapshot(&logged)?;
            Ok(json!({
                "bets": batch.bets,
                "skipped": batch.skipped,
                "snapshot": snapshot,
            }))
        }
        Request::Snapshot { bets } => snapshot(&bets),
        Request::Kelly {
            decimal,
            true_prob,
            bankroll,
            multiplier,
        } => to_json(wager::kelly(decimal, true_prob, bankroll, multiplier)),
        Request::ExpectedValue {
            decimal,
            true_prob,
            stake,
        } => to_json(wager::expected_value(decimal, true_prob, stake)),
        Request::BreakevenLadder {
            prices,
            target_edge,
        } => to_json(variance::breakeven_ladder(&prices, target_edge)),
        Request::BetMix { legs } => to_json(variance::bet_mix(&legs)),
        Request::SimulateSeason { input, seed } => {
            let seed = resolve_seed(&seed)?;
            to_json(variance::simulate_season(&input, seed))
        }
        Request::SimulateRuin { input, seed } => {
            let seed = resolve_seed(&seed)?;
            to_json(risk_of_ruin::simulate_ruin(&input, seed))
        }
        Request::ConvertOdds { value, format } => {
            let decimal = odds::to_decimal(&value, format).map_err(|error| error.to_string())?;
            let mut view = to_json(odds::from_decimal(decimal))?;
            if let Some(object) = view.as_object_mut() {
                object.insert("decimal".to_owned(), json!(decimal));
            }
            Ok(view)
        }
        Request::Parlay { american, stake } => {
            let mut decimals = Vec::with_capacity(american.len());
            for price in american {
                decimals.push(odds::american_to_decimal(price).map_err(|error| error.to_string())?);
            }
            to_json(bettor_core::parlay::parlay(&decimals, stake))
        }
        Request::Hold {
            american_a,
            american_b,
        } => {
            let implied_a = implied(american_a)?;
            let implied_b = implied(american_b)?;
            to_json(bettor_core::hold::calculate_hold(implied_a, implied_b))
        }
        Request::PriceLadder {
            from,
            to,
            step_cents,
        } => to_json(odds::price_ladder(from, to, step_cents)),
    }
}

fn logged_from_imported(bets: &[import::ImportedBet]) -> Vec<LoggedBet> {
    bets.iter()
        .map(|bet| LoggedBet {
            price_taken: bet.price_taken,
            closing_price: bet.closing_price,
            opposing_closing_price: bet.opposing_closing_price,
            stake: bet.stake,
            outcome: bet.outcome,
        })
        .collect()
}

fn snapshot(bets: &[LoggedBet]) -> Result<Value, String> {
    let summary = analyze(bets).map_err(|error| error.to_string())?.summary;
    let clv_mix = to_mix(bets, 100.0).ok();
    let book_mix = match to_book_mix(bets, 100.0) {
        Ok(mix) => mix,
        Err(_) => LedgerMix {
            legs: Vec::new(),
            bets_used: 0,
            bets_skipped: bets.len(),
        },
    };

    let settled: Vec<&LoggedBet> = bets.iter().filter(|bet| bet.outcome.is_settled()).collect();
    let total_stake: f64 = settled.iter().map(|bet| bet.stake).sum();
    let avg_stake = if settled.is_empty() {
        0.0
    } else {
        total_stake / settled.len() as f64
    };
    let avg_american = if total_stake > 0.0 {
        settled
            .iter()
            .map(|bet| bet.stake * bet.price_taken)
            .sum::<f64>()
            / total_stake
    } else {
        -110.0
    };
    let prices: Vec<f64> = book_mix.legs.iter().map(|leg| leg.american).collect();
    let price_min = prices.iter().copied().fold(f64::INFINITY, f64::min);
    let price_max = prices.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let price_min = if price_min.is_finite() { price_min } else { -110.0 };
    let price_max = if price_max.is_finite() { price_max } else { 100.0 };

    Ok(json!({
        "summary": summary,
        "clvMix": clv_mix,
        "bookMix": book_mix,
        "avgStake": avg_stake,
        "avgAmerican": avg_american,
        "priceMin": price_min,
        "priceMax": price_max,
    }))
}

fn implied(american: f64) -> Result<f64, String> {
    odds::american_to_implied(american).map_err(|error| error.to_string())
}

fn to_json<T: serde::Serialize>(result: bettor_core::Result<T>) -> Result<Value, String> {
    let value = result.map_err(|error| error.to_string())?;
    serde_json::to_value(value).map_err(|error| error.to_string())
}

fn resolve_seed(seed: &str) -> Result<u64, String> {
    let trimmed = seed.trim();
    if trimmed.is_empty() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(1, |duration| duration.as_nanos());
        return Ok(nanos as u64);
    }
    trimmed
        .parse()
        .map_err(|_| format!("seed must be a whole number, got {trimmed}"))
}

fn envelope_err(message: String) -> String {
    json!({ "error": message }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_won_bet_survives_the_csv_round_trip() {
        let csv = "\
bet_id,sportsbook,type,status,odds,closing_line,amount,profit,time_placed_iso,bet_info,sports,leagues
1,DraftKings,single,SETTLED_WIN,1.909,,25,22.73,2026-01-02T15:00:00Z,Team moneyline,NBA,NBA
";
        let raw = dispatch(&json!({ "cmd": "importCsv", "csv": csv }).to_string());
        let parsed: Value = serde_json::from_str(&raw).unwrap();
        assert!(parsed.get("error").is_none(), "{raw}");
        assert_eq!(parsed["ok"]["snapshot"]["summary"]["won"], 1);
        assert_eq!(parsed["ok"]["bets"][0]["book"], "DraftKings");
    }
}
