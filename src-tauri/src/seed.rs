//! First-run seed: the owner's Pikkit book, bundled into the desktop binary.
//!
//! The app opens with this record already written. There is no upload step —
//! new bets are added one at a time through the bet log form.

use bettor_core::ledger::Outcome;
use bettor_core::odds;
use crate::betlog::{BetDraft, BetLog};

/// Marker stored in `app_meta` once this seed has been applied.
pub const SEED_ID: &str = "pikkit-transactions-1-v1";

const SEED_CSV: &str = include_str!("../resources/seed-bets.csv");

/// Loads the bundled book when this install has not seen it yet.
///
/// Replacing an older incomplete import is deliberate: the seed is the source
/// of truth for the opening record. Bets added after seeding stay only if the
/// seed id already matches.
pub fn ensure_seeded(log: &BetLog) -> Result<(), String> {
    if log.seed_id().map_err(|error| error.to_string())? == Some(SEED_ID.to_owned()) {
        return Ok(());
    }

    let drafts = parse_seed_csv(SEED_CSV)?;
    log.clear_all().map_err(|error| error.to_string())?;
    let result = log.import_many(&drafts).map_err(|error| error.to_string())?;
    if result.imported == 0 {
        return Err("seed CSV produced no bets".to_owned());
    }
    log.set_seed_id(SEED_ID)
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn parse_seed_csv(text: &str) -> Result<Vec<BetDraft>, String> {
    let mut reader = csv::Reader::from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|error| format!("seed CSV headers: {error}"))?
        .clone();

    let mut drafts = Vec::new();
    for (index, row) in reader.records().enumerate() {
        let row = row.map_err(|error| format!("seed CSV line {}: {error}", index + 2))?;
        match row_to_draft(&row, &headers) {
            Ok(draft) => drafts.push(draft),
            Err(message) => {
                // A blank-odds parlay with a recorded payout still counts —
                // skip only rows that cannot be money at all.
                if message.contains("stake must be positive") || message.contains("missing bet_info")
                {
                    continue;
                }
                return Err(format!("seed CSV line {}: {message}", index + 2));
            }
        }
    }
    Ok(drafts)
}

fn row_to_draft(row: &csv::StringRecord, headers: &csv::StringRecord) -> Result<BetDraft, String> {
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

    let outcome = parse_outcome(&get("status"));
    let realized = parse_profit(&get("profit"));
    let price_taken = price_taken(&get("odds"), stake, realized, outcome)?;
    let closing_line = get("closing_line");
    let closing_price = if closing_line.is_empty() {
        None
    } else {
        decimal_to_american(&closing_line)
    };

    let mut notes = Vec::new();
    let bet_id = get("bet_id");
    if !bet_id.is_empty() {
        notes.push(bet_id);
    }
    if get("status") == "SETTLED_CASH_OUT" {
        notes.push("cashed out".to_owned());
    }

    Ok(BetDraft {
        placed_at: placed_date(&get("time_placed_iso")),
        sport: sport_label(&get("sports"), &get("leagues")),
        market: infer_market(&get("type"), &bet_info),
        selection: bet_info,
        book: get("sportsbook"),
        price_taken,
        closing_price,
        opposing_closing_price: None,
        stake,
        outcome,
        notes: notes.join(" · "),
        realized_profit: realized,
    })
}

fn parse_profit(value: &str) -> Option<f64> {
    let parsed: f64 = value.trim().parse().ok()?;
    parsed.is_finite().then_some(parsed)
}

fn price_taken(
    odds: &str,
    stake: f64,
    profit: Option<f64>,
    outcome: Outcome,
) -> Result<f64, String> {
    if let Some(price) = decimal_to_american(odds) {
        return Ok(price);
    }
    if !odds.trim().is_empty() {
        return Err(format!("invalid odds: {odds}"));
    }
    if outcome == Outcome::Won {
        if let Some(paid) = profit {
            if stake > 0.0 && paid > 0.0 {
                let decimal = 1.0 + paid / stake;
                if let Some(price) = odds::to_american(decimal).ok().map(f64::from) {
                    return Ok(price);
                }
            }
        }
    }
    Ok(-110.0)
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

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::betlog::BetFilter;

    #[test]
    fn the_bundled_book_loads_once() {
        let log = BetLog::in_memory().unwrap();
        ensure_seeded(&log).unwrap();
        assert_eq!(log.seed_id().unwrap().as_deref(), Some(SEED_ID));
        let first = log.list(&BetFilter::default()).unwrap().len();
        assert!(first > 1200, "got {first}");
        ensure_seeded(&log).unwrap();
        assert_eq!(log.list(&BetFilter::default()).unwrap().len(), first);
    }
}
