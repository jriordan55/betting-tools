//! Import bet rows from a JSON file into the on-disk bet log.
//!
//! ```text
//! cargo run --example import_bets -- fixtures/draftkings-sample.json
//! ```

use bettor_desktop_lib::betlog::{BetDraft, BetLog};
use std::env;
use std::path::{Path, PathBuf};

fn default_db_path() -> PathBuf {
    dirs_fallback()
}

fn dirs_fallback() -> PathBuf {
    let base = env::var("APPDATA")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_else(|_| ".".to_owned());
    PathBuf::from(base)
        .join("com.bettorcalculator.desktop")
        .join("betlog.sqlite3")
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let json_path = args
        .get(1)
        .map(Path::new)
        .unwrap_or_else(|| {
            eprintln!("usage: import_bets <bets.json> [betlog.sqlite3]");
            std::process::exit(1);
        });
    let db_path = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(default_db_path);

    let text = std::fs::read_to_string(json_path).unwrap_or_else(|error| {
        eprintln!("could not read {}: {error}", json_path.display());
        std::process::exit(1);
    });

    let drafts: Vec<BetDraft> = serde_json::from_str(&text).unwrap_or_else(|error| {
        eprintln!("invalid JSON in {}: {error}", json_path.display());
        std::process::exit(1);
    });

    let log = BetLog::open(&db_path).unwrap_or_else(|error| {
        eprintln!(
            "could not open {}: {error}. Close Bettor Desktop and try again.",
            db_path.display()
        );
        std::process::exit(1);
    });

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

    for row in &result.errors {
        eprintln!("  row {}: {}", row.index, row.message);
    }

    if result.imported == 0 {
        std::process::exit(1);
    }
}
