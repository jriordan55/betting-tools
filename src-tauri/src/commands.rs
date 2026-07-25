//! The IPC surface.
//!
//! Every command here is a thin adapter: deserialize, call `bettor_core`,
//! serialize. No arithmetic lives in this file, and none should — if a command
//! starts computing something, that computation belongs in the core crate
//! where it can be tested without a webview.
//!
//! Commands return `Result<T, MathError>`. `MathError` serializes as a tagged
//! union, so the frontend gets a discriminated type it can match on rather
//! than a string it has to parse.

use crate::betlog;
use bettor_core::{
    arbitrage, bayesian, clv, correlation, devig, distributions, hold, ledger, line, match_model,
    margin_model, middle, odds, parlay, probability, regression, risk_of_ruin, teaser,
    variance, wager, MathError,
};

/// Result of a command, with the error type the frontend sees.
type CmdResult<T> = Result<T, MathError>;

/// Parses a seed supplied by the frontend, or draws a fresh one.
///
/// Seeds cross the wire as decimal strings because JSON numbers are doubles
/// and a `u64` above 2^53 would arrive corrupted — which would silently break
/// the guarantee that a run can be reproduced from its reported seed.
fn resolve_seed(seed: Option<String>) -> CmdResult<u64> {
    match seed {
        None => Ok(super::fresh_seed()),
        Some(text) => text.trim().parse::<u64>().map_err(|_| MathError::ParseOdds {
            value: text,
            format: "seed (expected a decimal integer)",
        }),
    }
}

/// Build and version information for the math engine.
#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    /// Version of the `bettor-core` math crate.
    pub core_version: String,
    /// Version of this desktop shell.
    pub shell_version: String,
    /// Whether the engine was compiled with optimizations. Monte Carlo results
    /// from an unoptimized build are correct but far slower.
    pub optimized: bool,
}

/// Reports the math engine's version.
#[tauri::command]
#[specta::specta]
pub fn engine_info() -> EngineInfo {
    EngineInfo {
        core_version: bettor_core::VERSION.to_owned(),
        shell_version: env!("CARGO_PKG_VERSION").to_owned(),
        optimized: !cfg!(debug_assertions),
    }
}

// ------------------------------------------------------------------- odds

/// Converts a price in any format to decimal odds.
#[tauri::command]
#[specta::specta]
pub fn to_decimal(value: String, format: odds::OddsFormat) -> CmdResult<f64> {
    odds::to_decimal(&value, format)
}

/// Expresses a decimal price in every other format.
#[tauri::command]
#[specta::specta]
pub fn from_decimal(decimal: f64) -> CmdResult<odds::OddsView> {
    odds::from_decimal(decimal)
}

/// Converts a probability to the American price implying it.
#[tauri::command]
#[specta::specta]
pub fn implied_to_american(prob: f64) -> i32 {
    odds::implied_to_american(prob)
}

/// Converts a probability to the decimal price implying it.
///
/// The frontend needs this to show fair odds beside a devigged or modelled
/// probability. It cannot compute `1 / p` itself — that is math, and math
/// lives in the core.
#[tauri::command]
#[specta::specta]
pub fn implied_to_decimal(prob: f64) -> CmdResult<f64> {
    odds::implied_to_decimal(prob)
}

// ------------------------------------------------------------------- hold

/// Computes the hold on a two-sided market.
#[tauri::command]
#[specta::specta]
pub fn calculate_hold(implied_a: f64, implied_b: f64) -> CmdResult<hold::Hold> {
    hold::calculate_hold(implied_a, implied_b)
}

/// One book's quote in a vig comparison.
#[derive(Debug, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BookInput {
    /// Book name, carried through to the result.
    pub name: String,
    /// Implied probability of side A.
    pub implied_a: f64,
    /// Implied probability of side B.
    pub implied_b: f64,
}

/// One row of a vig comparison.
///
/// Exactly one of `hold` and `error` is set. Failed rows are kept rather than
/// dropped so a book never silently vanishes from the table.
#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BookRow {
    /// Book name.
    pub name: String,
    /// The hold it charges, when the quote could be read.
    pub hold: Option<hold::Hold>,
    /// Why the quote could not be read, when it could not.
    pub error: Option<MathError>,
}

/// Ranks books on the same market by hold, cheapest first.
///
/// Rows that fail validation are kept with their error rather than dropped, so
/// a book never silently vanishes from the table.
#[tauri::command]
#[specta::specta]
pub fn compare_vig(books: Vec<BookInput>) -> Vec<BookRow> {
    let tuples: Vec<(String, f64, f64)> = books
        .into_iter()
        .map(|b| (b.name, b.implied_a, b.implied_b))
        .collect();
    hold::compare_vig(&tuples)
        .into_iter()
        .map(|(name, result)| match result {
            Ok(hold) => BookRow { name, hold: Some(hold), error: None },
            Err(error) => BookRow { name, hold: None, error: Some(error) },
        })
        .collect()
}

// ------------------------------------------------------------------ devig

/// One devig method's verdict on a market.
#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DevigRow {
    /// Which method produced this.
    pub method: devig::DevigMethod,
    /// Human-readable name for display.
    pub name: String,
    /// Fair probabilities, when the method converged.
    pub fair_probs: Option<Vec<f64>>,
    /// Why the method could not produce them, when it could not.
    ///
    /// Kept beside the row so the UI can explain an empty column instead of
    /// just showing a dash.
    pub error: Option<MathError>,
}

/// Runs every devig method against a market.
#[tauri::command]
#[specta::specta]
pub fn devig_all(implied_probs: Vec<f64>) -> Vec<DevigRow> {
    devig::devig_all(&implied_probs)
        .into_iter()
        .map(|(method, result)| DevigRow {
            method,
            name: method.name().to_owned(),
            fair_probs: result.as_ref().ok().cloned(),
            error: result.err(),
        })
        .collect()
}

/// Expected value of a bet against a fair probability.
#[tauri::command]
#[specta::specta]
pub fn ev_vs_fair(fair_prob: f64, bet_implied: f64) -> CmdResult<f64> {
    devig::ev_vs_fair(fair_prob, bet_implied)
}

// ------------------------------------------------- wager / parlay / clv

/// Expected value of a bet at a price and true probability.
#[tauri::command]
#[specta::specta]
pub fn expected_value(
    decimal: f64,
    true_prob: f64,
    stake: f64,
) -> CmdResult<wager::ExpectedValue> {
    wager::expected_value(decimal, true_prob, stake)
}

/// Kelly stake for a price, probability, and bankroll.
#[tauri::command]
#[specta::specta]
pub fn kelly(
    decimal: f64,
    true_prob: f64,
    bankroll: f64,
    multiplier: f64,
) -> CmdResult<wager::Kelly> {
    wager::kelly(decimal, true_prob, bankroll, multiplier)
}

/// Splits a bankroll across every outcome of an arbitrage.
#[tauri::command]
#[specta::specta]
pub fn arbitrage(decimals: Vec<f64>, total_stake: f64) -> CmdResult<arbitrage::Arbitrage> {
    arbitrage::arbitrage(&decimals, total_stake)
}

/// Sizes a hedge against an open position.
#[tauri::command]
#[specta::specta]
pub fn hedge(
    original_stake: f64,
    original_decimal: f64,
    hedge_decimal: f64,
    goal: arbitrage::HedgeGoal,
) -> CmdResult<arbitrage::Hedge> {
    arbitrage::hedge(original_stake, original_decimal, hedge_decimal, goal)
}

/// Prices a parlay from its legs.
#[tauri::command]
#[specta::specta]
pub fn parlay(legs: Vec<f64>, stake: f64) -> CmdResult<parlay::Parlay> {
    parlay::parlay(&legs, stake)
}

/// Closing line value, measured every way that is defensible.
#[tauri::command]
#[specta::specta]
pub fn clv(
    bet_decimal: f64,
    close_decimal: f64,
    opposing_close_decimal: Option<f64>,
) -> CmdResult<clv::Clv> {
    clv::clv(bet_decimal, close_decimal, opposing_close_decimal)
}

// -------------------------------------------------------- lines / models

/// Removes the vig from a two-sided market.
#[tauri::command]
#[specta::specta]
pub fn fair_cover_prob(american: f64, opposing_american: f64) -> CmdResult<f64> {
    line::fair_cover_prob(american, opposing_american)
}

/// Backs the market's true line out of a posted line and a fair probability.
#[tauri::command]
#[specta::specta]
pub fn implied_true_line(
    line_value: f64,
    fair_cover_prob: f64,
    std_dev: f64,
    bet_type: line::BetType,
) -> CmdResult<f64> {
    line::implied_true_line(line_value, fair_cover_prob, std_dev, bet_type)
}

/// Prices every alternate line around a main line.
#[tauri::command]
#[specta::specta]
pub fn generate_ladder(
    main_line: f64,
    fair_cover_prob: f64,
    std_dev: f64,
    bet_type: line::BetType,
    range: f64,
    step: f64,
) -> CmdResult<line::Ladder> {
    line::generate_ladder(main_line, fair_cover_prob, std_dev, bet_type, range, step)
}

/// Compares two line-and-price quotes on the same market.
#[tauri::command]
#[specta::specta]
pub fn compare_lines(
    line_a: f64,
    fair_prob_a: f64,
    line_b: f64,
    fair_prob_b: f64,
    std_dev: f64,
    bet_type: line::BetType,
) -> CmdResult<line::LineComparison> {
    line::compare_lines(line_a, fair_prob_a, line_b, fair_prob_b, std_dev, bet_type)
}

/// Completes a partially-entered set of probabilities with the leftover one.
#[tauri::command]
#[specta::specta]
pub fn complete_simplex(partial: Vec<f64>) -> CmdResult<Vec<f64>> {
    probability::complete_simplex(&partial)
}

/// Total implied probability of a market, before any devig.
#[tauri::command]
#[specta::specta]
pub fn market_overround(implied_probs: Vec<f64>) -> CmdResult<f64> {
    devig::overround(&implied_probs)
}

/// Every sport preset the frontend populates its pickers from.
///
/// Returned from the core rather than re-declared in TypeScript: a
/// mistranscribed σ or regression constant is invisible in a unit test and
/// wrong in every number the calculator prints.
#[tauri::command]
#[specta::specta]
pub fn sport_config() -> bettor_core::config::SportConfig {
    bettor_core::config::sport_config()
}

/// Splits a total around a spread into each side's projected score.
#[tauri::command]
#[specta::specta]
pub fn implied_scores(spread: f64, total: f64) -> CmdResult<line::ImpliedScores> {
    line::implied_scores(spread, total)
}

/// How a match model was parameterised.
#[derive(Debug, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum MatchModel {
    /// Independent Poisson scoring.
    Poisson {
        /// Home team's scoring rate.
        lambda_home: f64,
        /// Away team's scoring rate.
        lambda_away: f64,
    },
    /// Overdispersed scoring, for blowout-prone matchups.
    NegativeBinomial {
        /// Home team's mean score.
        mean_home: f64,
        /// Away team's mean score.
        mean_away: f64,
        /// Home dispersion. Lower means more variance.
        r_home: f64,
        /// Away dispersion.
        r_away: f64,
    },
}

/// Markets derived from a scoreline model, plus the grid's truncation mass.
#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MatchMarkets {
    /// Moneyline, spreads, totals, and top scorelines.
    pub markets: match_model::MarketProbs,
    /// Probability that fell outside the grid before renormalising.
    ///
    /// A large value means `max_score` is too low for these scoring rates.
    pub truncation_mass: f64,
}

/// Prices every market implied by a scoreline model.
#[tauri::command]
#[specta::specta]
pub fn derive_markets(
    model: MatchModel,
    max_score: u32,
    spread_lines: Vec<f64>,
    total_lines: Vec<f64>,
    allow_draw: bool,
) -> CmdResult<MatchMarkets> {
    let matrix = match model {
        MatchModel::Poisson {
            lambda_home,
            lambda_away,
        } => match_model::ScoreMatrix::poisson(lambda_home, lambda_away, max_score)?,
        MatchModel::NegativeBinomial {
            mean_home,
            mean_away,
            r_home,
            r_away,
        } => match_model::ScoreMatrix::negative_binomial(
            mean_home, mean_away, r_home, r_away, max_score,
        )?,
    };
    Ok(MatchMarkets {
        truncation_mass: matrix.truncation_mass(),
        markets: match_model::derive_markets(&matrix, &spread_lines, &total_lines, allow_draw),
    })
}

/// Regresses a rate toward a baseline and attaches a 90% interval.
#[tauri::command]
#[specta::specta]
pub fn regress(
    observed: f64,
    baseline: f64,
    sample_size: f64,
    regression_constant: f64,
) -> CmdResult<regression::Regressed> {
    regression::regress_with_interval(observed, baseline, sample_size, regression_constant)
}

/// Traces how a regressed estimate converges as the sample grows.
#[tauri::command]
#[specta::specta]
pub fn convergence_series(
    observed: f64,
    baseline: f64,
    regression_constant: f64,
    max_sample: f64,
    steps: u32,
) -> CmdResult<Vec<regression::ConvergencePoint>> {
    regression::convergence_series(
        observed,
        baseline,
        regression_constant,
        max_sample,
        steps as usize,
    )
}

// ----------------------------------------------- middles / teasers / copula

/// Prices a middle or trap.
#[tauri::command]
#[specta::specta]
pub fn calculate_middle(
    market: middle::Market,
    high_side: middle::Leg,
    low_side: middle::Leg,
    std_dev: f64,
) -> CmdResult<middle::Middle> {
    middle::calculate_middle(market, high_side, low_side, std_dev)
}

/// Prices a teaser against the book's offered number.
#[tauri::command]
#[specta::specta]
pub fn analyze_teaser(
    legs: Vec<teaser::TeaserLeg>,
    teaser_points: f64,
    teaser_decimal_odds: f64,
    std_dev: f64,
) -> CmdResult<teaser::Teaser> {
    teaser::analyze_teaser(&legs, teaser_points, teaser_decimal_odds, std_dev)
}

/// Prices a parlay whose legs move together.
#[tauri::command]
#[specta::specta]
pub fn correlated_parlay(
    probs: Vec<f64>,
    correlation: f64,
    seed: Option<String>,
    sims: Option<u32>,
) -> CmdResult<correlation::Correlated> {
    correlation::correlated_parlay(
        &probs,
        correlation,
        resolve_seed(seed)?,
        sims.unwrap_or(200_000) as usize,
    )
}

// -------------------------------------------------------------- bayesian

/// Folds a model probability into a market probability.
#[tauri::command]
#[specta::specta]
pub fn beta_update(
    market_prob: f64,
    model_prob: f64,
    market_n: f64,
    model_n: f64,
) -> CmdResult<bayesian::BetaPosterior> {
    bayesian::beta_update(market_prob, model_prob, market_n, model_n)
}

/// Folds a model into a market across three or more outcomes.
#[tauri::command]
#[specta::specta]
pub fn dirichlet_update(
    market_probs: Vec<f64>,
    model_probs: Vec<f64>,
    market_n: f64,
    model_n: f64,
) -> CmdResult<bayesian::DirichletPosterior> {
    bayesian::dirichlet_update(&market_probs, &model_probs, market_n, model_n)
}

/// Combines a market margin and a model margin by precision.
#[tauri::command]
#[specta::specta]
pub fn margin_update(
    market_margin: f64,
    model_margin: f64,
    market_std: f64,
    model_std: f64,
) -> CmdResult<bayesian::MarginPosterior> {
    bayesian::margin_update(market_margin, model_margin, market_std, model_std)
}

/// Prices a posterior probability against an available quote.
#[tauri::command]
#[specta::specta]
pub fn edge_vs_price(
    posterior_prob: f64,
    decimal_odds: f64,
    threshold: f64,
) -> CmdResult<bayesian::Edge> {
    bayesian::edge_vs_price(posterior_prob, decimal_odds, threshold)
}

// ------------------------------------------------------------ simulation

/// A prop simulation: samples, summarised.
#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PropSimulation {
    /// Seed that produced this run. Pass it back to reproduce exactly.
    ///
    /// A decimal string, not a number — see `resolve_seed`.
    pub seed: String,
    /// Binned samples, ready to chart.
    pub histogram: Vec<distributions::HistogramBin>,
    /// Mean and standard deviation of the sample.
    pub stats: distributions::Stats,
    /// Probability the prop goes over the line, 0–1.
    pub over_prob: f64,
    /// Probability it does not, 0–1.
    ///
    /// Both sides come back counted from `bettor_core`; neither is derived
    /// from the other here. See `distributions::over_under_prob`.
    pub under_prob: f64,
    /// Fair American price for the over.
    pub over_fair_odds: i32,
    /// Fair American price for the under.
    pub under_fair_odds: i32,
}

/// Simulates a player prop and prices both sides of the line.
///
/// Long-running, so it is `async`: Tauri runs `async` commands off the main
/// thread and the window keeps painting.
#[tauri::command]
#[specta::specta]
pub async fn simulate_prop(
    n: u32,
    distribution: distributions::Distribution,
    mu: f64,
    var_multiplier: f64,
    line_value: f64,
    seed: Option<String>,
) -> CmdResult<PropSimulation> {
    let seed = resolve_seed(seed)?;
    let samples =
        distributions::generate_samples(n as usize, distribution, mu, var_multiplier, seed)?;
    let sides = distributions::over_under_prob(&samples, line_value);

    Ok(PropSimulation {
        seed: seed.to_string(),
        histogram: distributions::histogram(&samples, distribution, None),
        stats: distributions::stats(&samples),
        over_prob: sides.over,
        under_prob: sides.under,
        over_fair_odds: odds::implied_to_american(sides.over),
        under_fair_odds: odds::implied_to_american(sides.under),
    })
}

/// Simulates bankroll survival under flat betting.
///
/// Long-running, so it is `async` for the same reason as [`simulate_prop`].
#[tauri::command]
#[specta::specta]
pub async fn simulate_ruin(
    input: risk_of_ruin::RuinInput,
    seed: Option<String>,
) -> CmdResult<risk_of_ruin::RuinResult> {
    risk_of_ruin::simulate_ruin(&input, resolve_seed(seed)?)
}

// --------------------------------------------------------------- variance

/// Builds a range of American prices evenly spaced in cents.
#[tauri::command]
#[specta::specta]
pub fn price_ladder(from_american: f64, to_american: f64, step_cents: f64) -> CmdResult<Vec<f64>> {
    odds::price_ladder(from_american, to_american, step_cents)
}

/// Cents between two American prices, correct across the ±100 pivot.
#[tauri::command]
#[specta::specta]
pub fn cents_between(from_american: f64, to_american: f64) -> CmdResult<f64> {
    odds::cents_between(from_american, to_american)
}

/// Breakeven and required win rate across a range of prices.
#[tauri::command]
#[specta::specta]
pub fn breakeven_ladder(
    american_prices: Vec<f64>,
    target_edge: f64,
) -> CmdResult<Vec<variance::LadderRung>> {
    variance::breakeven_ladder(&american_prices, target_edge)
}

/// Translates a fixed cents move into probability points across a price range.
#[tauri::command]
#[specta::specta]
pub fn clv_ladder(american_prices: Vec<f64>, cents: f64) -> CmdResult<Vec<variance::ClvRung>> {
    variance::clv_ladder(&american_prices, cents)
}

/// Prices a mix of bets for return and for variance.
#[tauri::command]
#[specta::specta]
pub fn bet_mix(legs: Vec<variance::MixLeg>) -> CmdResult<variance::BetMix> {
    variance::bet_mix(&legs)
}

/// Simulates a season of a bet mix many times over.
///
/// Long-running, so it is `async` for the same reason as [`simulate_ruin`].
#[tauri::command]
#[specta::specta]
pub async fn simulate_season(
    input: variance::SeasonInput,
    seed: Option<String>,
) -> CmdResult<variance::SeasonResult> {
    variance::simulate_season(&input, resolve_seed(seed)?)
}

// ---------------------------------------------------------------- bet log

/// Something that can go wrong reading or analysing the bet log.
///
/// Two sources, kept apart. `MathError` must never learn that a disk exists —
/// `bettor-core` has no io — and a caller wants to know whether the log could
/// not be *read* or whether one of the bets in it does not make sense. The
/// frontend matches on `source` and then on the inner `kind`.
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type)]
#[serde(tag = "source", content = "error", rename_all = "camelCase")]
pub enum BetLogError {
    /// The log itself could not be read or written.
    #[error(transparent)]
    Storage(#[from] betlog::LogError),
    /// A bet in the log could not be analysed.
    #[error(transparent)]
    Math(#[from] MathError),
}

type LogResult<T> = Result<T, betlog::LogError>;
type AnalysisResult<T> = Result<T, BetLogError>;

/// Records a bet.
#[tauri::command]
#[specta::specta]
pub fn add_bet(log: tauri::State<'_, betlog::BetLog>, draft: betlog::BetDraft) -> LogResult<betlog::Bet> {
    log.add(&draft)
}

/// Replaces a recorded bet — settling it, or correcting a typo.
#[tauri::command]
#[specta::specta]
pub fn update_bet(
    log: tauri::State<'_, betlog::BetLog>,
    id: i64,
    draft: betlog::BetDraft,
) -> LogResult<betlog::Bet> {
    log.update(id, &draft)
}

/// Deletes a recorded bet.
#[tauri::command]
#[specta::specta]
pub fn delete_bet(log: tauri::State<'_, betlog::BetLog>, id: i64) -> LogResult<()> {
    log.delete(id)
}

/// Lists recorded bets, newest first.
#[tauri::command]
#[specta::specta]
pub fn list_bets(
    log: tauri::State<'_, betlog::BetLog>,
    filter: betlog::BetFilter,
) -> LogResult<Vec<betlog::Bet>> {
    log.list(&filter)
}

/// Every sport that appears in the log.
#[tauri::command]
#[specta::specta]
pub fn bet_log_sports(log: tauri::State<'_, betlog::BetLog>) -> LogResult<Vec<String>> {
    log.sports()
}

/// Why the bet log is not saving to disk, or `None` when it is.
///
/// Surfaced so the UI can say that nothing is being kept, rather than looking
/// like it works until the window closes.
#[tauri::command]
#[specta::specta]
pub fn bet_log_status(log: tauri::State<'_, betlog::BetLog>) -> Option<String> {
    log.ephemeral_reason().map(str::to_owned)
}

/// A slice of the bet log, with each bet analysed and the whole thing summed.
#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BetLogView {
    /// The bets themselves, newest first.
    pub bets: Vec<betlog::Bet>,
    /// Their analysis, in the same order, plus the summary.
    pub ledger: ledger::Ledger,
}

/// Reads a slice of the log and analyses it.
///
/// One command rather than two so the rows and the summary can never describe
/// different sets of bets — which is what a separate `list` and `summarise`
/// would produce the moment a filter changed between the calls.
#[tauri::command]
#[specta::specta]
pub fn analyze_bet_log(
    log: tauri::State<'_, betlog::BetLog>,
    filter: betlog::BetFilter,
) -> AnalysisResult<BetLogView> {
    let bets = log.list(&filter)?;
    let logged: Vec<ledger::LoggedBet> = bets.iter().map(betlog::to_logged).collect();
    Ok(BetLogView {
        ledger: ledger::analyze(&logged)?,
        bets,
    })
}

/// Reshapes the log into the bet mix the variance module takes.
#[tauri::command]
#[specta::specta]
pub fn bet_log_mix(
    log: tauri::State<'_, betlog::BetLog>,
    filter: betlog::BetFilter,
    bucket_cents: f64,
) -> AnalysisResult<ledger::LedgerMix> {
    let bets = log.list(&filter)?;
    let logged: Vec<ledger::LoggedBet> = bets.iter().map(betlog::to_logged).collect();
    Ok(ledger::to_mix(&logged, bucket_cents)?)
}

// ------------------------------------------------------- game visualizer

/// A whole game, modelled and graded.
#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameView {
    /// The scoring distributions, and the team figures they imply.
    pub model: margin_model::MarginModel,
    /// How the spread grades.
    pub spread: margin_model::SpreadGrade,
    /// How the total grades.
    pub total: margin_model::TotalGrade,
    /// The three-way market.
    pub moneyline: margin_model::Moneyline,
    /// Cover probability across a range of spreads.
    pub curve: Vec<margin_model::CurvePoint>,
}

/// Models a game and grades a spread and a total against it.
///
/// One command rather than four so every figure on screen comes from the same
/// distribution. Rebuilding the model per query would let a half-typed input
/// leave the curve describing one game and the grades another.
#[tauri::command]
#[specta::specta]
pub fn analyze_game(
    shape: margin_model::GameShape,
    spread: f64,
    total: f64,
    curve_from: f64,
    curve_to: f64,
    curve_step: f64,
) -> CmdResult<GameView> {
    let model = margin_model::normal_game(&shape)?;
    Ok(GameView {
        spread: model.grade_spread(spread),
        total: model.grade_total(total),
        moneyline: model.moneyline(),
        curve: margin_model::cover_curve(&model, curve_from, curve_to, curve_step)?,
        model,
    })
}

/// Expected value across a range of assumed true probabilities.
#[tauri::command]
#[specta::specta]
pub fn ev_curve(
    decimal: f64,
    from_prob: f64,
    to_prob: f64,
    step: f64,
) -> CmdResult<Vec<wager::EvPoint>> {
    wager::ev_curve(decimal, from_prob, to_prob, step)
}

// ------------------------------------------------------------ primitives

/// Standard normal CDF, exposed for charting the model directly.
#[tauri::command]
#[specta::specta]
pub fn normal_cdf(z: f64) -> f64 {
    probability::normal_cdf(z)
}

/// Converts a win probability to the point spread implying it.
#[tauri::command]
#[specta::specta]
pub fn prob_to_spread(p: f64, std_dev: f64) -> f64 {
    probability::prob_to_spread(p, std_dev)
}
