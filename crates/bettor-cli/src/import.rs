//! Pikkit-style `transactions.csv` → logged bets.
//!
//! Same column mapping as `src-tauri/examples/import_transactions.rs`. The
//! desktop example writes SQLite; this one only parses, so the browser app
//! can upload a file without a database.

use bettor_core::ledger::{analyze_bet, LoggedBet, Outcome};
use bettor_core::odds;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedBet {
    pub placed_at: String,
    pub sport: String,
    pub market: String,
    pub selection: String,
    pub book: String,
    pub price_taken: f64,
    pub closing_price: Option<f64>,
    pub opposing_closing_price: Option<f64>,
    pub stake: f64,
    pub outcome: Outcome,
    pub profit: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedRow {
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportBatch {
    pub bets: Vec<ImportedBet>,
    pub skipped: Vec<SkippedRow>,
}

pub fn import_csv(text: &str) -> Result<ImportBatch, String> {
    let mut reader = csv::Reader::from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|error| format!("could not read CSV headers: {error}"))?
        .clone();

    let mut bets = Vec::new();
    let mut skipped = Vec::new();

    for (index, row) in reader.records().enumerate() {
        let line = index + 2;
        let row = match row {
            Ok(row) => row,
            Err(error) => {
                skipped.push(SkippedRow {
                    line,
                    message: error.to_string(),
                });
                continue;
            }
        };
        match row_to_bet(&row, &headers) {
            Ok(bet) => bets.push(bet),
            Err(message) => skipped.push(SkippedRow { line, message }),
        }
    }

    Ok(ImportBatch { bets, skipped })
}

fn row_to_bet(row: &csv::StringRecord, headers: &csv::StringRecord) -> Result<ImportedBet, String> {
    let get = |name: &str| -> String {
        headers
            .iter()
            .position(|header| header == name)
            .and_then(|index| row.get(index))
            .unwrap_or_default()
            .trim()
            .to_owned()
    };

    let bet_info = get("bet_info");
    if bet_info.is_empty() {
        return Err("missing bet_info".to_owned());
    }

    let stake: f64 = get("amount")
        .parse()
        .map_err(|_| format!("invalid stake: {}", get("amount")))?;
    if !stake.is_finite() || stake <= 0.0 {
        return Err(format!("stake must be positive: {stake}"));
    }

    let price_taken = decimal_to_american(&get("odds"))
        .ok_or_else(|| format!("invalid odds: {}", get("odds")))?;

    let closing_line = get("closing_line");
    let closing_price = if closing_line.is_empty() {
        None
    } else {
        decimal_to_american(&closing_line)
    };

    let logged = LoggedBet {
        price_taken,
        closing_price,
        opposing_closing_price: None,
        stake,
        outcome: parse_outcome(&get("status")),
    };
    let profit = analyze_bet(&logged)
        .map_err(|error| error.to_string())?
        .profit;

    Ok(ImportedBet {
        placed_at: placed_date(&get("time_placed_iso")),
        sport: sport_label(&get("sports"), &get("leagues")),
        market: infer_market(&get("type"), &bet_info),
        selection: bet_info,
        book: get("sportsbook"),
        price_taken,
        closing_price,
        opposing_closing_price: None,
        stake,
        outcome: logged.outcome,
        profit,
    })
}

fn decimal_to_american(value: &str) -> Option<f64> {
    let decimal: f64 = value.trim().parse().ok()?;
    if !decimal.is_finite() || decimal <= 1.0 {
        return None;
    }
    odds::to_american(decimal).ok().map(f64::from)
}

fn parse_outcome(status: &str) -> Outcome {
    match status {
        "SETTLED_WIN" => Outcome::Won,
        "SETTLED_LOSS" => Outcome::Lost,
        "SETTLED_PUSH" => Outcome::Push,
        "SETTLED_VOID" | "SETTLED_CASH_OUT" => Outcome::Void,
        _ => Outcome::Pending,
    }
}

fn infer_market(bet_type: &str, bet_info: &str) -> String {
    let kind = bet_type.to_ascii_lowercase();
    if kind.starts_with("round_robin") || kind == "parlay" {
        return "parlay".to_owned();
    }

    let info = bet_info.to_ascii_lowercase();
    if info.contains("spread") {
        "spread".to_owned()
    } else if info.contains(" o/u") || info.contains("over ") || info.contains("under ") {
        if info.contains("yards")
            || info.contains("points")
            || info.contains("birdies")
            || info.contains("rebounds")
            || info.contains("assists")
            || info.contains("strikeouts")
            || info.contains("home runs")
        {
            "prop".to_owned()
        } else {
            "total".to_owned()
        }
    } else if info.contains("moneyline") {
        "moneyline".to_owned()
    } else if info.contains("2 ball") || info.contains("method of victory") {
        "matchup".to_owned()
    } else if info.contains("sgp") {
        "sgp".to_owned()
    } else {
        "other".to_owned()
    }
}

fn sport_label(sports: &str, leagues: &str) -> String {
    let sports = sports.trim();
    if !sports.is_empty() {
        return sports.to_owned();
    }
    leagues.trim().to_owned()
}

fn placed_date(iso: &str) -> String {
    iso.get(..10)
        .filter(|date| date.len() == 10)
        .unwrap_or(iso)
        .to_owned()
}
