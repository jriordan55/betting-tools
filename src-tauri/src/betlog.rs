//! The bet log: SQLite storage for a real betting record.
//!
//! This module owns the database and nothing else. Every number derived from a
//! bet — profit, closing line value, edge against the fair close — comes from
//! `bettor_core::ledger`, which has never heard of SQLite and can be tested
//! without one.
//!
//! # Schema versioning
//!
//! The version lives in SQLite's own `user_version` pragma rather than in a
//! table of our own. It is transactional, it costs nothing to read, and it
//! cannot get out of step with the schema it describes because there is no
//! separate row to forget to update. Migrations run in order, each inside a
//! transaction, and the version is set in the same transaction as the change
//! it describes: a half-applied migration is not reachable.
//!
//! # WAL
//!
//! Write-ahead logging is on. A desktop app is read-heavy — every keystroke in
//! a filter re-queries — and WAL keeps those reads from blocking behind the one
//! writer. It also survives a hard kill without corrupting the file, which
//! matters when the process is a window someone can force-quit.

use bettor_core::ledger::{LoggedBet, Outcome};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

/// Something went wrong talking to the bet log.
///
/// Deliberately separate from `MathError`: a disk failure and a bad
/// probability are not the same kind of problem, and collapsing them would
/// leave the UI unable to tell the user which one happened.
#[derive(Debug, Clone, thiserror::Error, Serialize, specta::Type)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum LogError {
    /// The database could not be read or written.
    #[error("the bet log could not be opened or written: {message}")]
    Storage {
        /// What SQLite reported.
        message: String,
    },

    /// No bet with that id.
    #[error("no bet with id {id}")]
    NotFound {
        /// The id that was looked up.
        id: i64,
    },

    /// A field failed validation before it could be stored.
    #[error("{field} is not valid: {reason}")]
    Invalid {
        /// Which field.
        field: &'static str,
        /// Why it was rejected.
        reason: String,
    },
}

impl From<rusqlite::Error> for LogError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage {
            message: error.to_string(),
        }
    }
}

type Result<T> = core::result::Result<T, LogError>;

/// A bet as it is stored and returned.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Bet {
    /// Row id, assigned on insert.
    pub id: i64,
    /// ISO 8601 date the bet was placed.
    pub placed_at: String,
    /// Sport or league, free text.
    pub sport: String,
    /// Market type, free text: moneyline, spread, total, prop.
    pub market: String,
    /// What was actually backed.
    pub selection: String,
    /// Which book.
    pub book: String,
    /// American price taken.
    pub price_taken: f64,
    /// American price this side closed at, when known.
    pub closing_price: Option<f64>,
    /// American closing price on the other side, when known.
    pub opposing_closing_price: Option<f64>,
    /// Money risked.
    pub stake: f64,
    /// How it finished.
    pub outcome: Outcome,
    /// What the book actually paid, when the record says so.
    #[serde(default)]
    pub realized_profit: Option<f64>,
    /// Anything else worth remembering.
    pub notes: String,
}

/// The fields a caller supplies when writing a bet.
///
/// Separate from [`Bet`] because the id is the database's to assign, and a
/// shape that carries one on the way in invites a caller to invent it.
#[derive(Debug, Clone, PartialEq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BetDraft {
    /// ISO 8601 date the bet was placed.
    pub placed_at: String,
    /// Sport or league.
    pub sport: String,
    /// Market type.
    pub market: String,
    /// What was backed.
    pub selection: String,
    /// Which book.
    pub book: String,
    /// American price taken.
    pub price_taken: f64,
    /// American price this side closed at.
    pub closing_price: Option<f64>,
    /// American closing price on the other side.
    pub opposing_closing_price: Option<f64>,
    /// Money risked.
    pub stake: f64,
    /// How it finished.
    pub outcome: Outcome,
    /// What the book actually paid, when known.
    #[serde(default)]
    pub realized_profit: Option<f64>,
    /// Free-text notes.
    pub notes: String,
}

/// One row that failed during a bulk import.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportRowError {
    /// Zero-based index in the submitted batch.
    pub index: u32,
    /// Why the row was skipped.
    pub message: String,
}

/// Outcome of [`BetLog::import_many`].
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    /// Rows written successfully.
    pub imported: u32,
    /// Rows rejected.
    pub skipped: u32,
    /// Per-row failures, in submission order.
    pub errors: Vec<ImportRowError>,
}

/// Singles versus parlays and other multi-leg tickets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum BetKind {
    /// One selection, one ticket.
    Single,
    /// Parlay or round-robin stored as `market = parlay`.
    Parlay,
}

/// Which bets to return.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BetFilter {
    /// Only bets with this outcome.
    pub outcome: Option<Outcome>,
    /// Only this sport, matched exactly.
    pub sport: Option<String>,
    /// Only this sportsbook, matched exactly.
    pub book: Option<String>,
    /// Only this market type — spread, parlay, prop, and so on.
    pub market: Option<String>,
    /// Only bets placed in this calendar year.
    pub year: Option<i32>,
    /// Only bets with stake at or above this amount.
    pub min_stake: Option<f64>,
    /// Only bets with stake at or below this amount.
    pub max_stake: Option<f64>,
    /// Singles only, or parlays only.
    pub kind: Option<BetKind>,
    /// Only bets placed on or after this ISO date.
    pub from_date: Option<String>,
    /// Only bets placed on or before this ISO date.
    pub to_date: Option<String>,
}

/// Distinct values for filter dropdowns.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BetLogFacets {
    /// Every sport in the log.
    pub sports: Vec<String>,
    /// Every sportsbook in the log.
    pub books: Vec<String>,
    /// Every market type in the log.
    pub markets: Vec<String>,
    /// Every placement year in the log, newest first.
    pub years: Vec<i32>,
}

/// Text as SQLite stores it: `Outcome` is an enum here and a word there.
fn outcome_to_text(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Pending => "pending",
        Outcome::Won => "won",
        Outcome::Lost => "lost",
        Outcome::Push => "push",
        Outcome::Void => "void",
    }
}

/// The inverse of [`outcome_to_text`].
///
/// An unrecognised word is [`Outcome::Pending`] rather than an error: a row
/// written by a newer version of the app should still be listable by an older
/// one, and refusing to open the whole log over one unknown word would be a
/// worse failure than showing that bet as unsettled.
fn outcome_from_text(text: &str) -> Outcome {
    match text {
        "won" => Outcome::Won,
        "lost" => Outcome::Lost,
        "push" => Outcome::Push,
        "void" => Outcome::Void,
        _ => Outcome::Pending,
    }
}

/// Every migration, in order. The index is the version it produces.
const MIGRATIONS: &[&str] = &[
    // v1 — the initial schema.
    "CREATE TABLE bets (
        id                     INTEGER PRIMARY KEY AUTOINCREMENT,
        placed_at              TEXT    NOT NULL,
        sport                  TEXT    NOT NULL DEFAULT '',
        market                 TEXT    NOT NULL DEFAULT '',
        selection              TEXT    NOT NULL DEFAULT '',
        book                   TEXT    NOT NULL DEFAULT '',
        price_taken            REAL    NOT NULL,
        closing_price          REAL,
        opposing_closing_price REAL,
        stake                  REAL    NOT NULL,
        outcome                TEXT    NOT NULL DEFAULT 'pending',
        notes                  TEXT    NOT NULL DEFAULT ''
     );
     CREATE INDEX bets_placed_at ON bets (placed_at);
     CREATE INDEX bets_outcome   ON bets (outcome);",
    // v2 — recorded payouts and a seed marker for the bundled book.
    "ALTER TABLE bets ADD COLUMN realized_profit REAL;
     CREATE TABLE app_meta (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
     );",
];

/// The bet log.
#[derive(Debug)]
pub struct BetLog {
    connection: Mutex<Connection>,
    /// Why this log is not on disk, when it is not.
    ///
    /// A corrupt or unwritable file must not stop the application starting:
    /// twenty-five calculators have nothing to do with the bet log, and making
    /// them unreachable because one file will not open is a worse failure than
    /// the one being reported. The app falls back to memory and says so.
    ephemeral: Option<String>,
}

impl BetLog {
    /// Opens the log at `path`, creating and migrating it as needed.
    ///
    /// # Errors
    ///
    /// [`LogError::Storage`] if the file cannot be opened or a migration fails.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| LogError::Storage {
                message: format!("could not create {}: {e}", parent.display()),
            })?;
        }
        let connection = Connection::open(path)?;
        Self::prepare(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
            ephemeral: None,
        })
    }

    /// Opens the log at `path`, falling back to memory if that is impossible.
    ///
    /// Never fails, because the caller is application startup and there is no
    /// useful thing for it to do with a failure. [`Self::ephemeral_reason`]
    /// reports the fallback so the UI can warn that nothing is being saved.
    ///
    /// # Panics
    ///
    /// Only if SQLite cannot open an in-memory database, which means the
    /// process is already in a state nothing can be done about.
    #[must_use]
    pub fn open_or_ephemeral(path: &Path) -> Self {
        match Self::open(path) {
            Ok(log) => log,
            Err(error) => {
                let reason = error.to_string();
                let mut log = Self::in_memory()
                    .unwrap_or_else(|e| panic!("SQLite cannot open an in-memory database: {e}"));
                log.ephemeral = Some(reason);
                log
            }
        }
    }

    /// Why this log is running in memory, or `None` when it is on disk.
    #[must_use]
    pub fn ephemeral_reason(&self) -> Option<&str> {
        self.ephemeral.as_deref()
    }

    /// Opens an in-memory log. Used by the tests, which must not touch a disk.
    ///
    /// # Errors
    ///
    /// [`LogError::Storage`] if the schema cannot be created.
    pub fn in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory()?;
        Self::prepare(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
            ephemeral: None,
        })
    }

    fn prepare(connection: &Connection) -> Result<()> {
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Self::migrate(connection)
    }

    /// Applies every migration the file has not seen yet.
    fn migrate(connection: &Connection) -> Result<()> {
        let version: i64 =
            connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;

        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the version is one this code wrote, and MIGRATIONS is tiny"
        )]
        let applied = version.max(0) as usize;

        for (index, migration) in MIGRATIONS.iter().enumerate().skip(applied) {
            let target = index + 1;
            // The schema change and the version bump share a transaction, so a
            // crash between them is not a state the file can be found in.
            connection.execute_batch(&format!(
                "BEGIN;
                 {migration}
                 PRAGMA user_version = {target};
                 COMMIT;"
            ))?;
        }
        Ok(())
    }

    /// The schema version currently on disk.
    ///
    /// # Errors
    ///
    /// [`LogError::Storage`] if the pragma cannot be read.
    pub fn schema_version(&self) -> Result<i64> {
        let connection = self.lock();
        Ok(connection.query_row("PRAGMA user_version", [], |row| row.get(0))?)
    }

    /// A poisoned mutex means a previous caller panicked while holding it.
    ///
    /// The connection itself is still usable — SQLite does not care that a
    /// Rust thread unwound — so recovering beats propagating a failure the
    /// user can do nothing about.
    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Writes a new bet and returns it with its assigned id.
    ///
    /// # Errors
    ///
    /// [`LogError::Invalid`] if the draft fails validation,
    /// [`LogError::Storage`] on a write failure.
    pub fn import_many(&self, drafts: &[BetDraft]) -> Result<ImportResult> {
        let mut imported = 0_u32;
        let mut errors = Vec::new();

        for (index, draft) in drafts.iter().enumerate() {
            match self.add(draft) {
                Ok(_) => imported += 1,
                Err(error) => errors.push(ImportRowError {
                    index: u32::try_from(index).unwrap_or(u32::MAX),
                    message: error.to_string(),
                }),
            }
        }

        Ok(ImportResult {
            imported,
            skipped: u32::try_from(errors.len()).unwrap_or(u32::MAX),
            errors,
        })
    }

    pub fn add(&self, draft: &BetDraft) -> Result<Bet> {
        validate(draft)?;
        let connection = self.lock();
        connection.execute(
            "INSERT INTO bets (
                placed_at, sport, market, selection, book,
                price_taken, closing_price, opposing_closing_price,
                stake, outcome, notes, realized_profit
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                draft.placed_at,
                draft.sport,
                draft.market,
                draft.selection,
                draft.book,
                draft.price_taken,
                draft.closing_price,
                draft.opposing_closing_price,
                draft.stake,
                outcome_to_text(draft.outcome),
                draft.notes,
                draft.realized_profit,
            ],
        )?;
        let id = connection.last_insert_rowid();
        drop(connection);
        self.get(id)
    }

    /// Replaces an existing bet.
    ///
    /// # Errors
    ///
    /// [`LogError::NotFound`] if no such bet exists, [`LogError::Invalid`] if
    /// the draft fails validation, [`LogError::Storage`] on a write failure.
    pub fn update(&self, id: i64, draft: &BetDraft) -> Result<Bet> {
        validate(draft)?;
        let connection = self.lock();
        let changed = connection.execute(
            "UPDATE bets SET
                placed_at = ?2, sport = ?3, market = ?4, selection = ?5, book = ?6,
                price_taken = ?7, closing_price = ?8, opposing_closing_price = ?9,
                stake = ?10, outcome = ?11, notes = ?12, realized_profit = ?13
             WHERE id = ?1",
            params![
                id,
                draft.placed_at,
                draft.sport,
                draft.market,
                draft.selection,
                draft.book,
                draft.price_taken,
                draft.closing_price,
                draft.opposing_closing_price,
                draft.stake,
                outcome_to_text(draft.outcome),
                draft.notes,
                draft.realized_profit,
            ],
        )?;
        drop(connection);
        if changed == 0 {
            return Err(LogError::NotFound { id });
        }
        self.get(id)
    }

    /// Deletes every bet in the log.
    ///
    /// # Errors
    ///
    /// [`LogError::Storage`] if the delete fails.
    pub fn clear_all(&self) -> Result<u32> {
        let changed = self.lock().execute("DELETE FROM bets", [])?;
        Ok(u32::try_from(changed).unwrap_or(u32::MAX))
    }

    /// Which bundled book this file has already imported, if any.
    ///
    /// # Errors
    ///
    /// [`LogError::Storage`] if the query fails.
    pub fn seed_id(&self) -> Result<Option<String>> {
        let connection = self.lock();
        let value: Option<String> = connection
            .query_row(
                "SELECT value FROM app_meta WHERE key = 'seed_id'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value)
    }

    /// Records that the bundled book has been applied.
    ///
    /// # Errors
    ///
    /// [`LogError::Storage`] if the write fails.
    pub fn set_seed_id(&self, seed_id: &str) -> Result<()> {
        let connection = self.lock();
        connection.execute(
            "INSERT INTO app_meta (key, value) VALUES ('seed_id', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![seed_id],
        )?;
        Ok(())
    }

    /// Deletes a bet.
    ///
    /// # Errors
    ///
    /// [`LogError::NotFound`] if no such bet exists.
    pub fn delete(&self, id: i64) -> Result<()> {
        let changed = self
            .lock()
            .execute("DELETE FROM bets WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(LogError::NotFound { id });
        }
        Ok(())
    }

    /// Fetches one bet.
    ///
    /// # Errors
    ///
    /// [`LogError::NotFound`] if no such bet exists.
    pub fn get(&self, id: i64) -> Result<Bet> {
        let connection = self.lock();
        connection
            .query_row(
                "SELECT id, placed_at, sport, market, selection, book,
                        price_taken, closing_price, opposing_closing_price,
                        stake, outcome, notes, realized_profit
                 FROM bets WHERE id = ?1",
                params![id],
                row_to_bet,
            )
            .optional()?
            .ok_or(LogError::NotFound { id })
    }

    /// Lists bets, newest first.
    ///
    /// # Errors
    ///
    /// [`LogError::Storage`] if the query fails.
    pub fn list(&self, filter: &BetFilter) -> Result<Vec<Bet>> {
        let (sql, values) = filter_query(filter);
        let connection = self.lock();
        let mut statement = connection.prepare(&sql)?;
        let bound: Vec<&dyn rusqlite::ToSql> = values.iter().map(AsRef::as_ref).collect();
        let rows = statement.query_map(bound.as_slice(), row_to_bet)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Distinct filter values drawn from the whole log.
    ///
    /// # Errors
    ///
    /// [`LogError::Storage`] if any query fails.
    pub fn facets(&self) -> Result<BetLogFacets> {
        let connection = self.lock();
        Ok(BetLogFacets {
            sports: distinct_strings(&connection, "sport")?,
            books: distinct_strings(&connection, "book")?,
            markets: distinct_strings(&connection, "market")?,
            years: distinct_years(&connection)?,
        })
    }

    /// Every distinct sport in the log, for a filter dropdown.
    ///
    /// # Errors
    ///
    /// [`LogError::Storage`] if the query fails.
    pub fn sports(&self) -> Result<Vec<String>> {
        let connection = self.lock();
        let mut statement = connection
            .prepare("SELECT DISTINCT sport FROM bets WHERE sport <> '' ORDER BY sport")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

fn filter_query(filter: &BetFilter) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    // Built by pushing bound parameters rather than by interpolating the
    // values, so a sport named `'; DROP TABLE bets; --` is just a sport
    // that matches nothing.
    let mut sql = String::from(
        "SELECT id, placed_at, sport, market, selection, book,
                price_taken, closing_price, opposing_closing_price,
                stake, outcome, notes, realized_profit
         FROM bets WHERE 1 = 1",
    );
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(outcome) = filter.outcome {
        sql.push_str(" AND outcome = ?");
        values.push(Box::new(outcome_to_text(outcome)));
    }
    if let Some(sport) = &filter.sport {
        sql.push_str(" AND sport = ?");
        values.push(Box::new(sport.clone()));
    }
    if let Some(book) = &filter.book {
        sql.push_str(" AND book = ?");
        values.push(Box::new(book.clone()));
    }
    if let Some(market) = &filter.market {
        sql.push_str(" AND market = ?");
        values.push(Box::new(market.clone()));
    }
    if let Some(year) = filter.year {
        sql.push_str(" AND placed_at >= ? AND placed_at < ?");
        values.push(Box::new(format!("{year}-01-01")));
        values.push(Box::new(format!("{}-01-01", year + 1)));
    }
    if let Some(min_stake) = filter.min_stake {
        sql.push_str(" AND stake >= ?");
        values.push(Box::new(min_stake));
    }
    if let Some(max_stake) = filter.max_stake {
        sql.push_str(" AND stake <= ?");
        values.push(Box::new(max_stake));
    }
    match filter.kind {
        Some(BetKind::Single) => sql.push_str(" AND market <> 'parlay'"),
        Some(BetKind::Parlay) => sql.push_str(" AND market = 'parlay'"),
        None => {}
    }
    if let Some(from) = &filter.from_date {
        sql.push_str(" AND placed_at >= ?");
        values.push(Box::new(from.clone()));
    }
    if let Some(to) = &filter.to_date {
        sql.push_str(" AND placed_at <= ?");
        values.push(Box::new(to.clone()));
    }
    sql.push_str(" ORDER BY placed_at DESC, id DESC");
    (sql, values)
}

fn distinct_strings(connection: &Connection, column: &str) -> Result<Vec<String>> {
    let sql = format!(
        "SELECT DISTINCT {column} FROM bets WHERE {column} <> '' ORDER BY {column}"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn distinct_years(connection: &Connection) -> Result<Vec<i32>> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT CAST(substr(placed_at, 1, 4) AS INTEGER)
         FROM bets
         WHERE length(placed_at) >= 4
         ORDER BY 1 DESC",
    )?;
    let rows = statement.query_map([], |row| row.get::<_, i32>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// A stored bet in the shape `bettor_core::ledger` reads.
#[must_use]
pub fn to_logged(bet: &Bet) -> LoggedBet {
    LoggedBet {
        price_taken: bet.price_taken,
        closing_price: bet.closing_price,
        opposing_closing_price: bet.opposing_closing_price,
        stake: bet.stake,
        outcome: bet.outcome,
        realized_profit: bet.realized_profit,
    }
}

fn row_to_bet(row: &rusqlite::Row<'_>) -> rusqlite::Result<Bet> {
    Ok(Bet {
        id: row.get(0)?,
        placed_at: row.get(1)?,
        sport: row.get(2)?,
        market: row.get(3)?,
        selection: row.get(4)?,
        book: row.get(5)?,
        price_taken: row.get(6)?,
        closing_price: row.get(7)?,
        opposing_closing_price: row.get(8)?,
        stake: row.get(9)?,
        outcome: outcome_from_text(&row.get::<_, String>(10)?),
        notes: row.get(11)?,
        realized_profit: row.get(12)?,
    })
}

/// Rejects a draft the math would only reject later.
///
/// Storage validates what storage can see — a price that is not a price, a
/// stake that is not money, a date that is not a date. It deliberately does
/// not re-derive anything: the moment this function starts computing an edge,
/// there are two implementations of the edge.
fn validate(draft: &BetDraft) -> Result<()> {
    if draft.placed_at.trim().is_empty() {
        return Err(LogError::Invalid {
            field: "date",
            reason: "a bet needs a date".to_owned(),
        });
    }
    if !draft.stake.is_finite() || draft.stake <= 0.0 {
        return Err(LogError::Invalid {
            field: "stake",
            reason: format!("{} is not an amount of money", draft.stake),
        });
    }
    for (field, price) in [
        ("price taken", Some(draft.price_taken)),
        ("closing price", draft.closing_price),
        ("opposing closing price", draft.opposing_closing_price),
    ] {
        if let Some(value) = price {
            // The same rule the core enforces, applied here so a bad row never
            // reaches the file rather than failing every later read of it.
            if bettor_core::odds::american_to_decimal(value).is_err() {
                return Err(LogError::Invalid {
                    field,
                    reason: format!("{value} is not an American price"),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(price: f64, outcome: Outcome) -> BetDraft {
        BetDraft {
            placed_at: "2026-01-15".to_owned(),
            sport: "NFL".to_owned(),
            market: "spread".to_owned(),
            selection: "Bears -3.5".to_owned(),
            book: "Pinnacle".to_owned(),
            price_taken: price,
            closing_price: None,
            opposing_closing_price: None,
            stake: 100.0,
            outcome,
            realized_profit: None,
            notes: String::new(),
        }
    }

    #[test]
    fn a_fresh_log_is_migrated_to_the_current_version() {
        let log = BetLog::in_memory().unwrap();
        assert_eq!(log.schema_version().unwrap(), MIGRATIONS.len() as i64);
    }

    #[test]
    fn migrating_twice_is_a_no_op() {
        // The guard that lets `open` run unconditionally on every launch.
        let log = BetLog::in_memory().unwrap();
        let connection = log.lock();
        BetLog::migrate(&connection).unwrap();
        BetLog::migrate(&connection).unwrap();
        drop(connection);
        assert_eq!(log.schema_version().unwrap(), MIGRATIONS.len() as i64);
    }

    #[test]
    fn clear_all_removes_every_row() {
        let log = BetLog::in_memory().unwrap();
        log.add(&draft(-110.0, Outcome::Won)).unwrap();
        log.add(&draft(150.0, Outcome::Lost)).unwrap();
        assert_eq!(log.clear_all().unwrap(), 2);
        assert!(log.list(&BetFilter::default()).unwrap().is_empty());
    }

    #[test]
    fn import_many_reports_partial_failures() {
        let log = BetLog::in_memory().unwrap();
        let result = log
            .import_many(&[
                draft(-110.0, Outcome::Won),
                BetDraft {
                    stake: 0.0,
                    ..draft(-110.0, Outcome::Lost)
                },
                draft(150.0, Outcome::Won),
            ])
            .unwrap();

        assert_eq!(result.imported, 2);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].index, 1);
        assert_eq!(log.list(&BetFilter::default()).unwrap().len(), 2);
    }

    #[test]
    fn a_bet_survives_a_round_trip() {
        let log = BetLog::in_memory().unwrap();
        let stored = log
            .add(&BetDraft {
                closing_price: Some(-130.0),
                opposing_closing_price: Some(110.0),
                ..draft(-110.0, Outcome::Won)
            })
            .unwrap();

        assert!(stored.id > 0);
        assert_eq!(log.get(stored.id).unwrap(), stored);
        assert_eq!(stored.outcome, Outcome::Won);
        assert_eq!(stored.closing_price, Some(-130.0));
    }

    #[test]
    fn updating_changes_the_row_rather_than_adding_one() {
        let log = BetLog::in_memory().unwrap();
        let stored = log.add(&draft(-110.0, Outcome::Pending)).unwrap();

        let settled = log
            .update(
                stored.id,
                &BetDraft {
                    outcome: Outcome::Won,
                    closing_price: Some(-140.0),
                    ..draft(-110.0, Outcome::Pending)
                },
            )
            .unwrap();

        assert_eq!(settled.id, stored.id);
        assert_eq!(settled.outcome, Outcome::Won);
        assert_eq!(log.list(&BetFilter::default()).unwrap().len(), 1);
    }

    #[test]
    fn a_missing_row_is_not_found_rather_than_a_storage_failure() {
        let log = BetLog::in_memory().unwrap();
        assert!(matches!(log.get(42), Err(LogError::NotFound { id: 42 })));
        assert!(matches!(log.delete(42), Err(LogError::NotFound { id: 42 })));
        assert!(matches!(
            log.update(42, &draft(-110.0, Outcome::Won)),
            Err(LogError::NotFound { id: 42 })
        ));
    }

    #[test]
    fn deleting_removes_exactly_one_bet() {
        let log = BetLog::in_memory().unwrap();
        let a = log.add(&draft(-110.0, Outcome::Won)).unwrap();
        let b = log.add(&draft(150.0, Outcome::Lost)).unwrap();
        log.delete(a.id).unwrap();

        let remaining = log.list(&BetFilter::default()).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, b.id);
    }

    #[test]
    fn extended_filters_narrow_the_list() {
        let log = BetLog::in_memory().unwrap();
        log.add(&BetDraft {
            book: "DraftKings".to_owned(),
            market: "parlay".to_owned(),
            stake: 25.0,
            placed_at: "2026-03-01".to_owned(),
            ..draft(-110.0, Outcome::Won)
        })
        .unwrap();
        log.add(&BetDraft {
            book: "FanDuel".to_owned(),
            market: "spread".to_owned(),
            stake: 100.0,
            placed_at: "2025-11-15".to_owned(),
            ..draft(150.0, Outcome::Lost)
        })
        .unwrap();

        assert_eq!(
            log.list(&BetFilter {
                book: Some("DraftKings".to_owned()),
                ..BetFilter::default()
            })
            .unwrap()
            .len(),
            1
        );
        assert_eq!(
            log.list(&BetFilter {
                market: Some("spread".to_owned()),
                ..BetFilter::default()
            })
            .unwrap()
            .len(),
            1
        );
        assert_eq!(
            log.list(&BetFilter {
                year: Some(2026),
                ..BetFilter::default()
            })
            .unwrap()
            .len(),
            1
        );
        assert_eq!(
            log.list(&BetFilter {
                min_stake: Some(50.0),
                ..BetFilter::default()
            })
            .unwrap()
            .len(),
            1
        );
        assert_eq!(
            log.list(&BetFilter {
                kind: Some(BetKind::Parlay),
                ..BetFilter::default()
            })
            .unwrap()
            .len(),
            1
        );
        assert_eq!(
            log.list(&BetFilter {
                kind: Some(BetKind::Single),
                ..BetFilter::default()
            })
            .unwrap()
            .len(),
            1
        );
    }

    #[test]
    fn facets_lists_distinct_filter_values() {
        let log = BetLog::in_memory().unwrap();
        log.add(&BetDraft {
            sport: "Golf".to_owned(),
            book: "DraftKings".to_owned(),
            market: "prop".to_owned(),
            placed_at: "2026-01-15".to_owned(),
            ..draft(-110.0, Outcome::Won)
        })
        .unwrap();
        log.add(&BetDraft {
            sport: "NBA".to_owned(),
            book: "FanDuel".to_owned(),
            market: "parlay".to_owned(),
            placed_at: "2025-06-01".to_owned(),
            ..draft(150.0, Outcome::Lost)
        })
        .unwrap();

        let facets = log.facets().unwrap();
        assert_eq!(facets.sports, vec!["Golf".to_owned(), "NBA".to_owned()]);
        assert!(facets.books.contains(&"DraftKings".to_owned()));
        assert!(facets.markets.contains(&"parlay".to_owned()));
        assert_eq!(facets.years, vec![2026, 2025]);
    }

    #[test]
    fn filters_narrow_the_list() {
        let log = BetLog::in_memory().unwrap();
        log.add(&draft(-110.0, Outcome::Won)).unwrap();
        log.add(&BetDraft {
            sport: "NBA".to_owned(),
            placed_at: "2026-02-01".to_owned(),
            ..draft(150.0, Outcome::Lost)
        })
        .unwrap();

        let won = log.list(&BetFilter {
            outcome: Some(Outcome::Won),
            ..BetFilter::default()
        });
        assert_eq!(won.unwrap().len(), 1);

        let nba = log.list(&BetFilter {
            sport: Some("NBA".to_owned()),
            ..BetFilter::default()
        });
        assert_eq!(nba.unwrap().len(), 1);

        let february = log.list(&BetFilter {
            from_date: Some("2026-01-20".to_owned()),
            ..BetFilter::default()
        });
        assert_eq!(february.unwrap().len(), 1);

        let none = log.list(&BetFilter {
            sport: Some("MLB".to_owned()),
            ..BetFilter::default()
        });
        assert!(none.unwrap().is_empty());
    }

    #[test]
    fn a_filter_value_cannot_become_sql() {
        let log = BetLog::in_memory().unwrap();
        log.add(&draft(-110.0, Outcome::Won)).unwrap();

        let hostile = log.list(&BetFilter {
            sport: Some("'; DROP TABLE bets; --".to_owned()),
            ..BetFilter::default()
        });
        assert!(hostile.unwrap().is_empty(), "it is a sport, not a statement");
        // And the table is still there.
        assert_eq!(log.list(&BetFilter::default()).unwrap().len(), 1);
    }

    #[test]
    fn the_list_comes_back_newest_first() {
        let log = BetLog::in_memory().unwrap();
        log.add(&BetDraft {
            placed_at: "2026-01-01".to_owned(),
            ..draft(-110.0, Outcome::Won)
        })
        .unwrap();
        log.add(&BetDraft {
            placed_at: "2026-03-01".to_owned(),
            ..draft(-110.0, Outcome::Won)
        })
        .unwrap();

        let bets = log.list(&BetFilter::default()).unwrap();
        assert_eq!(bets[0].placed_at, "2026-03-01");
    }

    #[test]
    fn a_bad_row_is_refused_before_it_reaches_the_file() {
        let log = BetLog::in_memory().unwrap();

        // -50 is not a price; the core would reject it on every later read.
        assert!(log.add(&draft(-50.0, Outcome::Won)).is_err());
        assert!(log
            .add(&BetDraft {
                stake: 0.0,
                ..draft(-110.0, Outcome::Won)
            })
            .is_err());
        assert!(log
            .add(&BetDraft {
                placed_at: "  ".to_owned(),
                ..draft(-110.0, Outcome::Won)
            })
            .is_err());
        assert!(log
            .add(&BetDraft {
                closing_price: Some(5.0),
                ..draft(-110.0, Outcome::Won)
            })
            .is_err());

        assert!(log.list(&BetFilter::default()).unwrap().is_empty());
    }

    #[test]
    fn an_unopenable_file_degrades_rather_than_stopping_the_app() {
        // A directory is never a database, so this is a reliable failure.
        let log = BetLog::open_or_ephemeral(Path::new("/"));
        assert!(log.ephemeral_reason().is_some());

        // And it still works, so the rest of the app is unaffected.
        let stored = log.add(&draft(-110.0, Outcome::Won)).unwrap();
        assert_eq!(log.get(stored.id).unwrap().id, stored.id);
    }

    #[test]
    fn a_log_that_opened_normally_is_not_marked_ephemeral() {
        assert!(BetLog::in_memory().unwrap().ephemeral_reason().is_none());
    }

    #[test]
    fn an_unknown_outcome_word_reads_as_pending() {
        // Forward compatibility: a row written by a newer build must not make
        // the whole log unreadable to an older one.
        assert_eq!(outcome_from_text("cashed-out"), Outcome::Pending);
        assert_eq!(outcome_from_text("won"), Outcome::Won);
    }

    #[test]
    fn sports_lists_each_one_once_and_skips_blanks() {
        let log = BetLog::in_memory().unwrap();
        log.add(&draft(-110.0, Outcome::Won)).unwrap();
        log.add(&draft(-110.0, Outcome::Lost)).unwrap();
        log.add(&BetDraft {
            sport: "NBA".to_owned(),
            ..draft(-110.0, Outcome::Won)
        })
        .unwrap();
        log.add(&BetDraft {
            sport: String::new(),
            ..draft(-110.0, Outcome::Won)
        })
        .unwrap();

        assert_eq!(log.sports().unwrap(), vec!["NBA".to_owned(), "NFL".to_owned()]);
    }
}
