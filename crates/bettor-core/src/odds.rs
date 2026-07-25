//! Odds conversion between American, decimal, and fractional formats.
//!
//! Ported from `bettor-calculator-main/src/lib/math/odds.ts`.
//!
//! # Divergences from the TypeScript
//!
//! 1. **American prices are validated.** The TS accepts any number, so
//!    `toDecimal("-50", "american")` returns `3.0` and `toDecimal("-1.5",
//!    "american")` returns `67.67`. Neither is a real price — American odds
//!    have magnitude ≥ 100 by definition. Typing a spread (`-1.5`) into an
//!    odds field silently produced a 1.5%-implied quote. Here that is a
//!    [`MathError::DomainError`].
//! 2. **Probabilities are on a 0–1 scale everywhere.** [`from_decimal`]
//!    returns `0.9901`, where the TS returned `99.0099` from that one function
//!    while every sibling returned 0–1.
//! 3. **No degenerate fractions.** The TS gcd fallback rounds the numerator to
//!    zero for very short prices, emitting `"0/1"` for a decimal of 1.0001.
//!    Here the nearest tabulated fraction is used instead.
//! 4. **Formatting stays in the frontend.** [`to_american`] returns `i32`, not
//!    `"+150"`. Rust owns the arithmetic; the UI owns the plus sign.

use crate::{MathError, Result};
use serde::{Deserialize, Serialize};

/// The three ways a price gets written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum OddsFormat {
    /// `-110`, `+250`.
    American,
    /// `1.909`, `3.5`.
    Decimal,
    /// `10/11`, `5/2`.
    Fractional,
}

impl OddsFormat {
    const fn as_str(self) -> &'static str {
        match self {
            Self::American => "american",
            Self::Decimal => "decimal",
            Self::Fractional => "fractional",
        }
    }
}

/// Smallest magnitude a real American price can have.
///
/// `+100` / `-100` are the same price (evens); nothing sits between them.
pub const MIN_AMERICAN_MAGNITUDE: f64 = 100.0;

/// A decimal price expressed every other way.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct OddsView {
    /// American price, e.g. `-110`.
    pub american: i32,
    /// Fractional numerator, e.g. `10` in `10/11`.
    pub fractional_num: i32,
    /// Fractional denominator, e.g. `11` in `10/11`.
    pub fractional_den: i32,
    /// Implied probability on a **0–1** scale (the TS used 0–100 here).
    pub probability: f64,
}

/// Common fractional prices, in the order a UK board lists them.
///
/// Conversion snaps to the nearest of these when it is within a cent, so
/// `2.5` renders as `6/4` rather than `150/100`.
const FRACTIONS: [(i32, i32); 66] = [
    (1, 10), (1, 9), (1, 8), (1, 7), (1, 6), (1, 5), (2, 9), (1, 4), (2, 7),
    (3, 10), (1, 3), (4, 11), (2, 5), (4, 9), (1, 2), (8, 15), (4, 7), (8, 13),
    (4, 6), (8, 11), (4, 5), (5, 6), (10, 11), (1, 1), (21, 20), (11, 10),
    (6, 5), (5, 4), (11, 8), (6, 4), (13, 8), (7, 4), (15, 8), (2, 1),
    (85, 40), (11, 5), (12, 5), (5, 2), (13, 5), (11, 4), (3, 1), (100, 30),
    (7, 2), (4, 1), (9, 2), (5, 1), (11, 2), (6, 1), (13, 2), (7, 1),
    (15, 2), (8, 1), (17, 2), (9, 1), (10, 1), (11, 1), (12, 1), (14, 1),
    (16, 1), (20, 1), (25, 1), (33, 1), (40, 1), (50, 1), (66, 1), (100, 1),
];

/// Parses a leading numeric prefix the way JavaScript's `parseFloat` does.
///
/// Kept deliberately permissive so the port accepts the same keystrokes the
/// web app did — `" -110 "`, `"+150"`, `"1e3"` all parse. Unlike Rust's own
/// `f64::from_str`, trailing garbage is ignored rather than fatal; unlike
/// `from_str`, the literal `"NaN"` is rejected, because `parseFloat("NaN")`
/// yields `NaN` and every caller treats that as failure.
fn js_parse_float(input: &str) -> Option<f64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Longest parseable prefix wins, mirroring parseFloat's scan-and-stop.
    let mut best = None;
    for (idx, ch) in trimmed.char_indices() {
        let end = idx + ch.len_utf8();
        if let Some(prefix) = trimmed.get(..end) {
            if let Ok(value) = prefix.parse::<f64>() {
                if value.is_finite() || prefix.contains("Infinity") {
                    best = Some(value);
                }
            }
        }
    }
    best.filter(|v| !v.is_nan())
}

fn parse_err(value: &str, format: OddsFormat) -> MathError {
    MathError::ParseOdds {
        value: value.to_owned(),
        format: format.as_str(),
    }
}

/// Converts a price in any format to decimal odds.
///
/// # Errors
///
/// Returns [`MathError::ParseOdds`] if the text is not a number, and
/// [`MathError::DomainError`] if it parses but is not a price that can exist —
/// an American quote inside ±100, or a decimal at or below 1.0.
pub fn to_decimal(value: &str, format: OddsFormat) -> Result<f64> {
    match format {
        OddsFormat::Fractional => {
            let mut parts = value.split('/');
            let (Some(num_str), Some(den_str)) = (parts.next(), parts.next()) else {
                return Err(parse_err(value, format));
            };
            let (Some(num), Some(den)) = (js_parse_float(num_str), js_parse_float(den_str))
            else {
                return Err(parse_err(value, format));
            };
            if den <= 0.0 || num <= 0.0 {
                // The TS only guarded `den !== 0`, so "-1/2" produced a decimal
                // of 0.5 — a price that pays less than the stake.
                return Err(MathError::DomainError {
                    param: "fractional odds",
                    constraint: "both terms positive",
                    value: if num <= 0.0 { num } else { den },
                });
            }
            Ok(1.0 + num / den)
        }
        OddsFormat::American => {
            let parsed = js_parse_float(value).ok_or_else(|| parse_err(value, format))?;
            american_to_decimal(parsed)
        }
        OddsFormat::Decimal => {
            let parsed = js_parse_float(value).ok_or_else(|| parse_err(value, format))?;
            if parsed > 1.0 {
                Ok(parsed)
            } else {
                Err(MathError::DomainError {
                    param: "decimal odds",
                    constraint: "greater than 1",
                    value: parsed,
                })
            }
        }
    }
}

/// Converts an American price to decimal odds.
///
/// # Errors
///
/// [`MathError::DomainError`] if `|american| < 100`, which is not a price.
pub fn american_to_decimal(american: f64) -> Result<f64> {
    if !american.is_finite() || american.abs() < MIN_AMERICAN_MAGNITUDE {
        return Err(MathError::DomainError {
            param: "american odds",
            constraint: "at least 100 in magnitude",
            value: american,
        });
    }
    if american > 0.0 {
        Ok(1.0 + american / 100.0)
    } else {
        Ok(1.0 + 100.0 / american.abs())
    }
}

/// Converts decimal odds to an American price.
///
/// Returns a number, not a string: `+150` is formatting, and formatting is the
/// frontend's job.
///
/// # Errors
///
/// [`MathError::DomainError`] if `decimal <= 1.0`.
pub fn to_american(decimal: f64) -> Result<i32> {
    if decimal.is_nan() || decimal <= 1.0 {
        return Err(MathError::DomainError {
            param: "decimal odds",
            constraint: "greater than 1",
            value: decimal,
        });
    }
    let raw = if decimal >= 2.0 {
        (decimal - 1.0) * 100.0
    } else {
        -100.0 / (decimal - 1.0)
    };
    Ok(to_i32(js_round(raw)))
}

/// Rounds half away from zero for negatives the way `Math.round` does.
///
/// `Math.round(-0.5)` is `-0` in JavaScript (ties go toward +∞), whereas Rust's
/// `f64::round` sends ties away from zero. Only matters at exact halves, but
/// odds arithmetic lands on them often enough to care.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// Clamps an already-integral `f64` into `i32`.
///
/// American prices sit comfortably inside `i32`, but a decimal of `1.0000001`
/// implies roughly -1,000,000,000, and a price one ulp above 1.0 implies more
/// than `i32` can hold. Saturating beats wrapping, and both beat a panic.
#[allow(
    clippy::cast_possible_truncation,
    reason = "value is clamped into i32's range on the line above and is already integral"
)]
fn to_i32(x: f64) -> i32 {
    x.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

/// Expresses a decimal price in every other format.
///
/// # Errors
///
/// [`MathError::DomainError`] if `decimal <= 1.0`.
pub fn from_decimal(decimal: f64) -> Result<OddsView> {
    let american = to_american(decimal)?;
    let profit = decimal - 1.0;

    let mut closest = (1, 10);
    let mut min_diff = f64::INFINITY;
    for &(num, den) in &FRACTIONS {
        let diff = (profit - f64::from(num) / f64::from(den)).abs();
        if diff < min_diff {
            min_diff = diff;
            closest = (num, den);
        }
    }

    let (fractional_num, fractional_den) = if min_diff < 0.01 {
        closest
    } else {
        let numerator = js_round(profit * 100.0);
        if numerator < 1.0 {
            // TS produced "0/1" here — a fraction that says the bet pays
            // nothing. Fall back to the nearest tabulated price instead.
            closest
        } else {
            let numerator = to_i32(numerator);
            let divisor = gcd(numerator, 100);
            (numerator / divisor, 100 / divisor)
        }
    };

    Ok(OddsView {
        american,
        fractional_num,
        fractional_den,
        probability: 1.0 / decimal,
    })
}

const fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Implied probability of an American price, on a 0–1 scale.
///
/// # Errors
///
/// [`MathError::DomainError`] if `|american| < 100`.
pub fn american_to_implied(american: f64) -> Result<f64> {
    let decimal = american_to_decimal(american)?;
    Ok(1.0 / decimal)
}

/// Implied probability of a decimal price, on a 0–1 scale.
///
/// # Errors
///
/// [`MathError::DomainError`] if `decimal <= 1.0`.
pub fn decimal_to_implied(decimal: f64) -> Result<f64> {
    if decimal > 1.0 {
        Ok(1.0 / decimal)
    } else {
        Err(MathError::DomainError {
            param: "decimal odds",
            constraint: "greater than 1",
            value: decimal,
        })
    }
}

/// Converts a probability to the American price that implies it.
///
/// Clamps to `[0.001, 0.999]` before converting, matching the TS: without the
/// clamp a certainty produces a division by zero.
#[must_use]
pub fn implied_to_american(prob: f64) -> i32 {
    let p = prob.clamp(0.001, 0.999);
    let raw = if p >= 0.5 {
        -js_round((p / (1.0 - p)) * 100.0)
    } else {
        js_round(((1.0 - p) / p) * 100.0)
    };
    to_i32(raw)
}

/// Implied probability from a price string in American or decimal format.
///
/// # Errors
///
/// See [`to_decimal`].
pub fn odds_to_implied(value: &str, format: OddsFormat) -> Result<f64> {
    Ok(1.0 / to_decimal(value, format)?)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn american_round_trips_through_decimal() {
        // Skips exactly ±100: those are the same price (evens, decimal 2.0), so
        // the mapping is deliberately not injective there. -100 comes back +100.
        for american in (105..=10_000).step_by(5) {
            for signed in [f64::from(american), f64::from(-american)] {
                let decimal = american_to_decimal(signed).unwrap();
                assert_eq!(
                    to_american(decimal).unwrap(),
                    to_i32(signed),
                    "round trip failed for {signed}"
                );
            }
        }
    }

    #[test]
    fn plus_and_minus_one_hundred_are_the_same_price() {
        let from_minus = american_to_decimal(-100.0).unwrap();
        let from_plus = american_to_decimal(100.0).unwrap();
        assert_relative_eq!(from_minus, 2.0, epsilon = 1e-15);
        assert_relative_eq!(from_plus, 2.0, epsilon = 1e-15);
        assert_eq!(to_american(2.0).unwrap(), 100);
    }

    #[test]
    fn implied_probability_and_decimal_are_reciprocal() {
        for american in [-10_000, -500, -110, -100, 100, 110, 500, 10_000] {
            let p = american_to_implied(f64::from(american)).unwrap();
            let d = american_to_decimal(f64::from(american)).unwrap();
            assert_relative_eq!(p, 1.0 / d, epsilon = 1e-15);
        }
    }

    #[test]
    fn breakeven_rates_match_the_textbook() {
        // The numbers this whole project exists to explain.
        assert_relative_eq!(
            american_to_implied(-110.0).unwrap(),
            0.5238095238095238,
            epsilon = 1e-15
        );
        assert_relative_eq!(american_to_implied(400.0).unwrap(), 0.2, epsilon = 1e-15);
        assert_relative_eq!(
            american_to_implied(1000.0).unwrap(),
            0.0909090909090909,
            epsilon = 1e-15
        );
    }

    #[test]
    fn prices_that_do_not_exist_are_rejected() {
        // The headline divergence: the TS mapped these to real-looking decimals.
        for bogus in ["-50", "50", "-99", "99", "0", "-1.5", "1.5"] {
            assert!(
                to_decimal(bogus, OddsFormat::American).is_err(),
                "{bogus} should not parse as an American price"
            );
        }
    }

    #[test]
    fn accepts_the_same_keystrokes_the_web_app_did() {
        assert_relative_eq!(
            to_decimal(" -110 ", OddsFormat::American).unwrap(),
            1.9090909090909092,
            epsilon = 1e-15
        );
        assert_relative_eq!(
            to_decimal("+150", OddsFormat::American).unwrap(),
            2.5,
            epsilon = 1e-15
        );
        assert_relative_eq!(
            to_decimal("1e3", OddsFormat::American).unwrap(),
            11.0,
            epsilon = 1e-15
        );
    }

    #[test]
    fn nan_and_empty_are_parse_failures() {
        for bad in ["", "abc", "NaN", "   "] {
            assert!(matches!(
                to_decimal(bad, OddsFormat::American),
                Err(MathError::ParseOdds { .. })
            ));
        }
    }

    #[test]
    fn negative_fractions_are_rejected() {
        // TS: "-1/2" -> decimal 0.5, a bet that pays less than it costs.
        assert!(to_decimal("-1/2", OddsFormat::Fractional).is_err());
        assert!(to_decimal("1/0", OddsFormat::Fractional).is_err());
    }

    #[test]
    fn short_prices_do_not_produce_zero_fractions() {
        // TS emitted "0/1" for this decimal.
        let view = from_decimal(1.0001).unwrap();
        assert!(view.fractional_num >= 1, "got {}/{}", view.fractional_num, view.fractional_den);
    }

    #[test]
    fn probability_is_on_a_zero_to_one_scale() {
        let view = from_decimal(1.01).unwrap();
        assert_relative_eq!(view.probability, 0.9900990099009901, epsilon = 1e-15);
    }
}
