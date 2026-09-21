//! Import Pikkit-style `transactions.csv` into the bet log, optionally replacing
//! everything already stored.
//!
//! ```text
//! cargo run --example import_transactions -- --replace C:\path\to\transactions.csv
//! ```

use bettor_core::ledger::Outcome;
use bettor_core::odds;
use bettor_desktop_lib::betlog::{BetDraft, BetLog};
use std::env;
use std::path::PathBuf;

fn default_db_path() -> PathBuf {
    let base = env::var("APPDATA")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_else(|_| ".".to_owned());
    PathBuf::from(base)
        .join("com.bettorcalculator.desktop")
        .join("betlog.sqlite3")
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
    } else if info.contains("moneyline") {
        "moneyline".to_owned()
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

fn decimal_to_american(value: &str) -> Option<f64> {
    let decimal: f64 = value.trim().parse().ok()?;
    if !decimal.is_finite() || decimal <= 1.0 {
        return None;
    }
    odds::to_american(decimal).ok().map(f64::from)
}

fn row_to_draft(
    row: &csv::StringRecord,
    headers: &csv::StringRecord,
) -> Result<BetDraft, String> {
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

    let profit = get("profit");
    let ev = get("ev");
    let bet_id = get("bet_id");
    let book = get("sportsbook");
    let bet_type = get("type");
    let status = get("status");

    let mut notes = Vec::new();
    if !bet_id.is_empty() {
        notes.push(bet_id);
    }
    if !profit.is_empty() {
        notes.push(format!("profit {profit}"));
    }
    if !ev.is_empty() {
        notes.push(format!("ev {ev}"));
    }
    if status == "SETTLED_CASH_OUT" {
        notes.push("cashed out".to_owned());
    }

    Ok(BetDraft {
        placed_at: placed_date(&get("time_placed_iso")),
        sport: sport_label(&get("sports"), &get("leagues")),
        market: infer_market(&bet_type, &bet_info),
        selection: bet_info,
        book,
        price_taken,
        closing_price,
        opposing_closing_price: None,
        stake,
        outcome: parse_outcome(&status),
        notes: notes.join(" · "),
    })
}

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let replace = args.first().is_some_and(|arg| arg == "--replace");
    if replace {
        args.remove(0);
    }

    let csv_path = args.first().map(PathBuf::from).unwrap_or_else(|| {
        eprintln!("usage: import_transactions [--replace] <transactions.csv> [betlog.sqlite3]");
        std::process::exit(1);
    });
    let db_path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_db_path);

    let mut reader = csv::Reader::from_path(&csv_path).unwrap_or_else(|error| {
        eprintln!("could not read {}: {error}", csv_path.display());
        std::process::exit(1);
    });

    let headers = reader.headers().cloned().unwrap_or_else(|error| {
        eprintln!("could not read CSV headers: {error}");
        std::process::exit(1);
    });

    let mut drafts = Vec::new();
    let mut errors = Vec::new();

    for (index, row) in reader.records().enumerate() {
        let row = row.unwrap_or_else(|error| {
            eprintln!("CSV row {index} is malformed: {error}");
            std::process::exit(1);
        });
        match row_to_draft(&row, &headers) {
            Ok(draft) => drafts.push(draft),
            Err(message) => errors.push((index + 1, message)),
        }
    }

    let log = BetLog::open(&db_path).unwrap_or_else(|error| {
        eprintln!(
            "could not open {}: {error}. Close Bettor Desktop and try again.",
            db_path.display()
        );
        std::process::exit(1);
    });

    if replace {
        let cleared = log.clear_all().unwrap_or_else(|error| {
            eprintln!("could not clear bet log: {error}");
            std::process::exit(1);
        });
        println!("Cleared {cleared} existing bet(s).");
    }

    let result = log.import_many(&drafts).unwrap_or_else(|error| {
        eprintln!("import failed: {error}");
        std::process::exit(1);
    });

    println!(
        "Imported {} bet(s), skipped {} → {}",
        result.imported,
        result.skipped,
        db_path.display()
    );

    for (line, message) in errors.iter().take(10) {
        eprintln!("  skipped CSV line {line}: {message}");
    }
    if errors.len() > 10 {
        eprintln!("  …and {} more skipped rows before import", errors.len() - 10);
    }

    for row in &result.errors {
        eprintln!("  row {}: {}", row.index, row.message);
    }

    if result.imported == 0 {
        std::process::exit(1);
    }
}
