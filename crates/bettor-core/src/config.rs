//! Sport presets.
//!
//! Ported from `sportDefaults.ts`, `poissonConfig.ts`, `nbinomConfig.ts`,
//! `propSimConfig.ts` and `regressionConfig.ts`. These are parameters, not
//! math: nothing here computes anything, it only says what a reasonable
//! starting point looks like for a given sport and stat.
//!
//! They live in the core rather than the frontend for one reason — a
//! mistranscribed σ or regression constant is invisible in a unit test and
//! silently wrong in every number the calculator prints. Kept beside the
//! models they parameterise, there is one copy of each figure.
//!
//! # Divergence from the TypeScript
//!
//! **`poissonConfig.ts` and `nbinomConfig.ts` were the same table twice.**
//! Every field matched except `defaultR` and `rHint`, which only the negative
//! binomial uses. They are one table here, mirroring [`crate::match_model`],
//! where `poisson.ts` and `nbinom.ts` merged for the same reason: two copies
//! of a table drift, and the drift shows up as two calculators disagreeing
//! about how many goals a soccer team scores.

use crate::distributions::Distribution;
use crate::middle::Market;
use serde::Serialize;

/// Everything the frontend needs to populate its sport pickers.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SportConfig {
    /// Scoring spreads for line inversion and alternate-line ladders.
    pub lines: Vec<LineSport>,
    /// Scoring rates for the match models.
    pub matches: Vec<MatchSport>,
    /// Player prop presets, by sport and position.
    pub props: Vec<PropSport>,
    /// Regression constants and league averages, by sport and stat.
    pub regression: Vec<RegressionSport>,
}

/// Scoring variance for one sport, and the markets it posts.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LineSport {
    /// Stable identifier, e.g. `"nba"`.
    pub key: &'static str,
    /// Display name, e.g. `"NBA"`.
    pub label: &'static str,
    /// Standard deviation of the margin of victory, in points.
    pub spread_std: f64,
    /// Standard deviation of the game total, in points.
    pub total_std: f64,
    /// Which markets this sport is usually priced on.
    pub markets: Vec<Market>,
    /// True where a draw is a distinct moneyline outcome.
    pub three_way_moneyline: bool,
}

/// Scoring rates and line menus for the match models.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MatchSport {
    /// Stable identifier, e.g. `"soccer"`.
    pub key: &'static str,
    /// Display name.
    pub label: &'static str,
    /// What a unit of scoring is called — "Goals", "Runs".
    pub score_label: &'static str,
    /// Typical home scoring rate.
    pub default_home: f64,
    /// Typical away scoring rate.
    pub default_away: f64,
    /// Negative binomial dispersion. Smaller means more overdispersed.
    pub default_r: f64,
    /// Guidance on choosing `r`, shown beside the input.
    pub r_hint: &'static str,
    /// Largest score the matrix is evaluated to.
    ///
    /// Truncating drops real probability mass — see
    /// [`crate::match_model::MatchMarkets::truncation_mass`], which reports
    /// how much, so this can be checked rather than assumed.
    pub max_score: u32,
    /// Spread lines worth pricing.
    pub spread_lines: Vec<f64>,
    /// Total lines worth pricing.
    pub total_lines: Vec<f64>,
    /// True where a draw is a possible result.
    pub allow_draw: bool,
}

/// Prop presets for one sport.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PropSport {
    /// Stable identifier.
    pub key: &'static str,
    /// Display name.
    pub label: &'static str,
    /// Positions this sport prices props for.
    pub positions: Vec<PropPosition>,
}

/// One position within a sport.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PropPosition {
    /// Stable identifier.
    pub key: &'static str,
    /// Display name.
    pub label: &'static str,
    /// Stats commonly posted for this position.
    pub stats: Vec<PropStat>,
}

/// One prop market, and the distribution that fits its shape.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PropStat {
    /// Stable identifier.
    pub key: &'static str,
    /// Display name.
    pub label: &'static str,
    /// The distribution this stat is simulated from.
    pub distribution: Distribution,
    /// A typical projection.
    pub default_projection: f64,
    /// A typical posted line.
    pub default_line: f64,
    /// What the numbers are measured in.
    pub unit: &'static str,
}

/// Regression presets for one sport.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RegressionSport {
    /// Stable identifier.
    pub key: &'static str,
    /// Display name.
    pub label: &'static str,
    /// Stats with published regression constants.
    pub stats: Vec<RegressionStat>,
}

/// One regressable stat.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RegressionStat {
    /// Stable identifier.
    pub key: &'static str,
    /// Display name.
    pub label: &'static str,
    /// Sample size at which observed and league average carry equal weight.
    pub regression_constant: f64,
    /// The population mean this stat regresses toward.
    pub league_average: f64,
    /// An illustrative hot-start observation.
    pub default_observed: f64,
    /// An illustrative sample size.
    pub default_sample_size: f64,
    /// What the stat is measured in.
    pub unit: &'static str,
    /// What the sample is counted in — plate appearances, shots, attempts.
    pub sample_unit: &'static str,
}

/// Standard spread and total lines, shared by every match sport.
fn standard_spreads() -> Vec<f64> {
    vec![-2.5, -1.5, -0.5, 0.5, 1.5, 2.5]
}

/// Every preset, assembled.
#[must_use]
pub fn sport_config() -> SportConfig {
    SportConfig {
        lines: line_sports(),
        matches: match_sports(),
        props: prop_sports(),
        regression: regression_sports(),
    }
}

/// Scoring variance by sport, from `sportDefaults.ts`.
#[must_use]
pub fn line_sports() -> Vec<LineSport> {
    vec![
        LineSport {
            key: "nba",
            label: "NBA",
            spread_std: 14.5,
            total_std: 22.2,
            markets: vec![Market::Spread, Market::Total],
            three_way_moneyline: false,
        },
        LineSport {
            key: "nfl",
            label: "NFL",
            spread_std: 14.2,
            total_std: 13.9,
            markets: vec![Market::Spread, Market::Total],
            three_way_moneyline: false,
        },
        LineSport {
            key: "ncaab",
            label: "NCAAB",
            spread_std: 16.6,
            total_std: 19.7,
            markets: vec![Market::Spread, Market::Total],
            three_way_moneyline: false,
        },
        LineSport {
            key: "ncaaf",
            label: "NCAAF",
            spread_std: 22.6,
            total_std: 17.9,
            markets: vec![Market::Spread, Market::Total],
            three_way_moneyline: false,
        },
        LineSport {
            key: "nhl",
            label: "NHL",
            spread_std: 2.49,
            total_std: 2.29,
            markets: vec![Market::Total],
            three_way_moneyline: false,
        },
        LineSport {
            key: "mlb",
            label: "MLB",
            spread_std: 4.48,
            total_std: 4.61,
            markets: vec![Market::Total],
            three_way_moneyline: false,
        },
        LineSport {
            key: "soccer",
            label: "Soccer",
            spread_std: 1.9,
            total_std: 1.66,
            markets: vec![Market::Total],
            three_way_moneyline: true,
        },
    ]
}

/// Scoring rates by sport, from `poissonConfig.ts` and `nbinomConfig.ts`.
#[must_use]
pub fn match_sports() -> Vec<MatchSport> {
    vec![
        MatchSport {
            key: "soccer",
            label: "Soccer",
            score_label: "Goals",
            default_home: 1.45,
            default_away: 1.2,
            default_r: 8.0,
            r_hint: "Soccer has low variance; r = 6–12 typical",
            max_score: 7,
            spread_lines: standard_spreads(),
            total_lines: vec![1.5, 2.5, 3.5, 4.5, 5.5],
            allow_draw: true,
        },
        MatchSport {
            key: "hockey",
            label: "Hockey",
            score_label: "Goals",
            default_home: 3.1,
            default_away: 2.8,
            default_r: 5.0,
            r_hint: "Power plays add variance; r = 4–7 typical",
            max_score: 10,
            spread_lines: standard_spreads(),
            total_lines: vec![4.5, 5.5, 6.5, 7.5, 8.5],
            allow_draw: false,
        },
        MatchSport {
            key: "baseball",
            label: "Baseball",
            score_label: "Runs",
            default_home: 4.5,
            default_away: 4.2,
            default_r: 4.0,
            r_hint: "Big innings add variance; r = 3–6 typical",
            max_score: 16,
            spread_lines: standard_spreads(),
            total_lines: vec![6.5, 7.5, 8.5, 9.5, 10.5, 11.5],
            allow_draw: false,
        },
    ]
}

/// Prop presets, from `propSimConfig.ts`.
#[must_use]
pub fn prop_sports() -> Vec<PropSport> {
    vec![
        PropSport {
            key: "nfl",
            label: "NFL",
            positions: vec![
                PropPosition {
                    key: "qb",
                    label: "QB",
                    stats: vec![
                        PropStat {
                            key: "passing_yards",
                            label: "Passing Yards",
                            distribution: Distribution::Gamma,
                            default_projection: 250.0,
                            default_line: 249.5,
                            unit: "yards",
                        },
                        PropStat {
                            key: "passing_tds",
                            label: "Passing TDs",
                            distribution: Distribution::Poisson,
                            default_projection: 1.5,
                            default_line: 1.5,
                            unit: "TDs",
                        },
                        PropStat {
                            key: "interceptions",
                            label: "Interceptions",
                            distribution: Distribution::Poisson,
                            default_projection: 0.8,
                            default_line: 0.5,
                            unit: "INTs",
                        },
                        PropStat {
                            key: "rushing_yards",
                            label: "Rushing Yards",
                            distribution: Distribution::Lognormal,
                            default_projection: 20.0,
                            default_line: 19.5,
                            unit: "yards",
                        },
                    ],
                },
                PropPosition {
                    key: "rb",
                    label: "RB",
                    stats: vec![
                        PropStat {
                            key: "rushing_yards",
                            label: "Rushing Yards",
                            distribution: Distribution::Gamma,
                            default_projection: 65.0,
                            default_line: 64.5,
                            unit: "yards",
                        },
                        PropStat {
                            key: "receptions",
                            label: "Receptions",
                            distribution: Distribution::Nbinom,
                            default_projection: 3.0,
                            default_line: 2.5,
                            unit: "receptions",
                        },
                        PropStat {
                            key: "receiving_yards",
                            label: "Receiving Yards",
                            distribution: Distribution::Gamma,
                            default_projection: 25.0,
                            default_line: 24.5,
                            unit: "yards",
                        },
                    ],
                },
                PropPosition {
                    key: "wr",
                    label: "WR",
                    stats: vec![
                        PropStat {
                            key: "receptions",
                            label: "Receptions",
                            distribution: Distribution::Nbinom,
                            default_projection: 5.0,
                            default_line: 4.5,
                            unit: "receptions",
                        },
                        PropStat {
                            key: "receiving_yards",
                            label: "Receiving Yards",
                            distribution: Distribution::Gamma,
                            default_projection: 65.0,
                            default_line: 64.5,
                            unit: "yards",
                        },
                        PropStat {
                            key: "rushing_yards",
                            label: "Rushing Yards",
                            distribution: Distribution::Lognormal,
                            default_projection: 10.0,
                            default_line: 9.5,
                            unit: "yards",
                        },
                    ],
                },
                PropPosition {
                    key: "te",
                    label: "TE",
                    stats: vec![
                        PropStat {
                            key: "receptions",
                            label: "Receptions",
                            distribution: Distribution::Nbinom,
                            default_projection: 4.0,
                            default_line: 3.5,
                            unit: "receptions",
                        },
                        PropStat {
                            key: "receiving_yards",
                            label: "Receiving Yards",
                            distribution: Distribution::Gamma,
                            default_projection: 45.0,
                            default_line: 44.5,
                            unit: "yards",
                        },
                        PropStat {
                            key: "rushing_yards",
                            label: "Rushing Yards",
                            distribution: Distribution::Lognormal,
                            default_projection: 5.0,
                            default_line: 4.5,
                            unit: "yards",
                        },
                    ],
                },
            ],
        },
        PropSport {
            key: "nba",
            label: "NBA",
            positions: vec![PropPosition {
                key: "all",
                label: "All Positions",
                stats: vec![
                    PropStat {
                        key: "points",
                        label: "Points",
                        distribution: Distribution::Gamma,
                        default_projection: 25.0,
                        default_line: 24.5,
                        unit: "points",
                    },
                    PropStat {
                        key: "assists",
                        label: "Assists",
                        distribution: Distribution::Poisson,
                        default_projection: 6.0,
                        default_line: 5.5,
                        unit: "assists",
                    },
                    PropStat {
                        key: "rebounds",
                        label: "Rebounds",
                        distribution: Distribution::Poisson,
                        default_projection: 8.0,
                        default_line: 7.5,
                        unit: "rebounds",
                    },
                ],
            }],
        },
    ]
}

/// Regression constants, from `regressionConfig.ts`.
#[must_use]
pub fn regression_sports() -> Vec<RegressionSport> {
    vec![
        RegressionSport {
            key: "mlb",
            label: "MLB",
            stats: vec![
                RegressionStat {
                    key: "avg",
                    label: "Batting Average",
                    regression_constant: 910.0,
                    league_average: 0.245,
                    default_observed: 0.320,
                    default_sample_size: 100.0,
                    unit: "AVG",
                    sample_unit: "PA",
                },
                RegressionStat {
                    key: "hr_rate",
                    label: "HR Rate",
                    regression_constant: 170.0,
                    league_average: 0.035,
                    default_observed: 0.055,
                    default_sample_size: 100.0,
                    unit: "HR/PA",
                    sample_unit: "PA",
                },
                RegressionStat {
                    key: "k_rate",
                    label: "K Rate",
                    regression_constant: 60.0,
                    league_average: 0.225,
                    default_observed: 0.180,
                    default_sample_size: 50.0,
                    unit: "K%",
                    sample_unit: "PA",
                },
                RegressionStat {
                    key: "babip",
                    label: "BABIP",
                    regression_constant: 820.0,
                    league_average: 0.300,
                    default_observed: 0.370,
                    default_sample_size: 100.0,
                    unit: "BABIP",
                    sample_unit: "BIP",
                },
                RegressionStat {
                    key: "woba",
                    label: "wOBA",
                    regression_constant: 320.0,
                    league_average: 0.315,
                    default_observed: 0.370,
                    default_sample_size: 100.0,
                    unit: "wOBA",
                    sample_unit: "PA",
                },
            ],
        },
        RegressionSport {
            key: "nba",
            label: "NBA",
            stats: vec![
                RegressionStat {
                    key: "fg_pct",
                    label: "FG%",
                    regression_constant: 600.0,
                    league_average: 0.461,
                    default_observed: 0.520,
                    default_sample_size: 100.0,
                    unit: "FG%",
                    sample_unit: "FGA",
                },
                RegressionStat {
                    key: "three_pct",
                    label: "3PT%",
                    regression_constant: 750.0,
                    league_average: 0.362,
                    default_observed: 0.420,
                    default_sample_size: 100.0,
                    unit: "3P%",
                    sample_unit: "3PA",
                },
                RegressionStat {
                    key: "ft_pct",
                    label: "FT%",
                    regression_constant: 300.0,
                    league_average: 0.775,
                    default_observed: 0.850,
                    default_sample_size: 80.0,
                    unit: "FT%",
                    sample_unit: "FTA",
                },
            ],
        },
        RegressionSport {
            key: "nfl",
            label: "NFL",
            stats: vec![
                RegressionStat {
                    key: "comp_pct",
                    label: "Completion %",
                    regression_constant: 400.0,
                    league_average: 0.645,
                    default_observed: 0.710,
                    default_sample_size: 100.0,
                    unit: "CMP%",
                    sample_unit: "att",
                },
                RegressionStat {
                    key: "td_rate",
                    label: "TD Rate",
                    regression_constant: 350.0,
                    league_average: 0.045,
                    default_observed: 0.065,
                    default_sample_size: 100.0,
                    unit: "TD%",
                    sample_unit: "att",
                },
            ],
        },
        RegressionSport {
            key: "nhl",
            label: "NHL",
            stats: vec![
                RegressionStat {
                    key: "save_pct",
                    label: "Save %",
                    regression_constant: 1500.0,
                    league_average: 0.910,
                    default_observed: 0.935,
                    default_sample_size: 400.0,
                    unit: "SV%",
                    sample_unit: "shots",
                },
                RegressionStat {
                    key: "shooting_pct",
                    label: "Shooting %",
                    regression_constant: 150.0,
                    league_average: 0.095,
                    default_observed: 0.140,
                    default_sample_size: 50.0,
                    unit: "SH%",
                    sample_unit: "shots",
                },
            ],
        },
        RegressionSport {
            key: "soccer",
            label: "Soccer",
            stats: vec![
                RegressionStat {
                    key: "goals_per_90",
                    label: "Goals/90",
                    regression_constant: 34.0,
                    league_average: 0.035,
                    default_observed: 0.060,
                    default_sample_size: 10.0,
                    unit: "G/90",
                    sample_unit: "matches",
                },
                RegressionStat {
                    key: "xg_per_90",
                    label: "xG/90",
                    regression_constant: 34.0,
                    league_average: 0.040,
                    default_observed: 0.055,
                    default_sample_size: 10.0,
                    unit: "xG/90",
                    sample_unit: "matches",
                },
            ],
        },
    ]
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn every_key_is_unique_within_its_table() {
        let mut keys: Vec<&str> = line_sports().iter().map(|s| s.key).collect();
        keys.sort_unstable();
        let len = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), len, "duplicate sport key in line_sports");

        let mut keys: Vec<&str> = match_sports().iter().map(|s| s.key).collect();
        keys.sort_unstable();
        let len = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), len, "duplicate sport key in match_sports");
    }

    #[test]
    fn every_standard_deviation_is_positive() {
        // A zero or negative sigma reaches inverse_normal_cdf and produces
        // nonsense lines rather than an error, so it is worth catching here.
        for sport in line_sports() {
            assert!(sport.spread_std > 0.0, "{} spread_std", sport.key);
            assert!(sport.total_std > 0.0, "{} total_std", sport.key);
        }
    }

    #[test]
    fn every_sport_posts_at_least_one_market() {
        for sport in line_sports() {
            assert!(!sport.markets.is_empty(), "{} posts no markets", sport.key);
        }
    }

    #[test]
    fn scoring_rates_and_dispersion_are_positive() {
        for sport in match_sports() {
            assert!(sport.default_home > 0.0, "{} default_home", sport.key);
            assert!(sport.default_away > 0.0, "{} default_away", sport.key);
            assert!(sport.default_r > 0.0, "{} default_r", sport.key);
            assert!(sport.max_score > 0, "{} max_score", sport.key);
        }
    }

    #[test]
    fn regression_constants_and_averages_are_in_range() {
        for sport in regression_sports() {
            for stat in &sport.stats {
                assert!(
                    stat.regression_constant > 0.0,
                    "{}/{} regression_constant",
                    sport.key,
                    stat.key
                );
                assert!(
                    stat.league_average > 0.0 && stat.league_average < 1.0,
                    "{}/{} league_average is a rate, not a count",
                    sport.key,
                    stat.key
                );
            }
        }
    }

    #[test]
    fn prop_lines_sit_near_their_projections() {
        // A preset whose line is nowhere near its projection means one of the
        // two was mistyped — the over would price at 99% and look plausible.
        for sport in prop_sports() {
            for position in &sport.positions {
                for stat in &position.stats {
                    let spread = (stat.default_projection - stat.default_line).abs();
                    assert!(
                        spread <= stat.default_projection.max(1.0),
                        "{}/{}/{}: line {} is far from projection {}",
                        sport.key,
                        position.key,
                        stat.key,
                        stat.default_line,
                        stat.default_projection
                    );
                }
            }
        }
    }
}
