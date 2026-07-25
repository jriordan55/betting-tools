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

/// Converts a probability to the decimal price that implies it.
///
/// The exact inverse of [`decimal_to_implied`], and unlike
/// [`implied_to_american`] it does not clamp or round — a fair price derived
/// from a devig or a model is quoted to more precision than the American
/// integer scale can hold, and rounding it before display would move the
/// number the caller is about to compare against a book.
///
/// # Errors
///
/// [`MathError::ProbabilityOutOfRange`] unless `0 < prob < 1`. A certainty has
/// no price, and returning `inf` for one is how the TS produced `Infinity` in
/// `calculateMLEdge`.
pub fn implied_to_decimal(prob: f64) -> Result<f64> {
    if prob > 0.0 && prob < 1.0 {
        Ok(1.0 / prob)
    } else {
        Err(MathError::ProbabilityOutOfRange {
            value: prob,
            reason: "must be strictly between 0 and 1 to have a price",
        })
    }
}

/// Implied probability from a price string in American or decimal format.
///
/// # Errors
///
/// See [`to_decimal`].
pub fn odds_to_implied(value: &str, format: OddsFormat) -> Result<f64> {
    Ok(1.0 / to_decimal(value, format)?)
}

// ---------------------------------------------------------------------------
// The cents axis
// ---------------------------------------------------------------------------
//
// American odds have a hole in them. Nothing lives strictly between -100 and
// +100, and those two endpoints are the same price. Subtracting one American
// number from another therefore only measures cents while both prices stay on
// the same side of the pivot: `+105` to `-115` is twenty cents, and naive
// subtraction calls it two hundred and twenty.
//
// Everything that measures or moves a price by cents goes through the
// transform below, so the discontinuity is handled in exactly one place.

/// Maps an American price onto a continuous axis where distance is cents.
fn cent_line(american: f64) -> f64 {
    if american > 0.0 {
        american - 100.0
    } else {
        american + 100.0
    }
}

/// Inverse of [`cent_line`]. Zero maps to even money.
fn from_cent_line(cents: f64) -> f64 {
    if cents > 0.0 {
        cents + 100.0
    } else {
        cents - 100.0
    }
}

/// Where an American price sits on the cents axis, with even money at zero.
///
/// Negative for favorites, positive for dogs, and monotone in price — which
/// makes it the axis to bucket or interpolate prices on, since the American
/// scale itself is discontinuous.
///
/// # Errors
///
/// [`MathError::DomainError`] if the price is under 100 in magnitude.
pub fn cents_from_even(american: f64) -> Result<f64> {
    american_to_decimal(american)?;
    Ok(cent_line(american))
}

/// Inverse of [`cents_from_even`]. Zero is even money.
#[must_use]
pub fn american_from_cents(cents: f64) -> f64 {
    from_cent_line(cents)
}

/// Cents between two American prices, correct across the ±100 pivot.
///
/// Positive when `from` is the longer price — that is, when moving from `from`
/// to `to` means the price shortened.
///
/// # Errors
///
/// [`MathError::DomainError`] if either price is under 100 in magnitude.
pub fn cents_between(from: f64, to: f64) -> Result<f64> {
    american_to_decimal(from)?;
    american_to_decimal(to)?;
    Ok(cent_line(from) - cent_line(to))
}

/// Shortens an American price by `cents`. A negative `cents` lengthens it.
///
/// # Errors
///
/// [`MathError::DomainError`] if the price is under 100 in magnitude or the
/// move is not finite.
pub fn shift_cents(american: f64, cents: f64) -> Result<f64> {
    american_to_decimal(american)?;
    if !cents.is_finite() {
        return Err(MathError::DomainError {
            param: "cents",
            constraint: "finite",
            value: cents,
        });
    }
    Ok(from_cent_line(cent_line(american) - cents))
}

/// Builds a ladder of American prices, evenly spaced in cents.
///
/// The step is applied on the cents axis, so consecutive rungs are a fixed
/// number of cents apart rather than jumping by 200 across the pivot.
///
/// # Errors
///
/// [`MathError::DomainError`] for a price under 100 in magnitude, a
/// non-positive step, or a range whose second price is not the longer one;
/// [`MathError::ShapeError`] if the range would produce more than 2,000 rungs.
pub fn price_ladder(from_american: f64, to_american: f64, step_cents: f64) -> Result<Vec<f64>> {
    american_to_decimal(from_american)?;
    american_to_decimal(to_american)?;
    if !step_cents.is_finite() || step_cents <= 0.0 {
        return Err(MathError::DomainError {
            param: "step",
            constraint: "greater than zero",
            value: step_cents,
        });
    }
    let lo = cent_line(from_american);
    let hi = cent_line(to_american);
    if hi <= lo {
        return Err(MathError::DomainError {
            param: "range",
            constraint: "the second price must be longer than the first",
            value: hi - lo,
        });
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "non-negative by the check above, and capped on the next line"
    )]
    let rungs = ((hi - lo) / step_cents).floor() as usize + 1;
    if rungs > 2_000 {
        return Err(MathError::ShapeError {
            what: "ladder rungs",
            expected: "at most 2000",
            got: rungs,
        });
    }

    Ok((0..rungs)
        .map(|i| {
            #[allow(clippy::cast_precision_loss, reason = "rung index, capped at 2000")]
            let offset = i as f64 * step_cents;
            from_cent_line(lo + offset)
        })
        .collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn cents_are_plain_subtraction_while_both_prices_share_a_side() {
        assert_relative_eq!(cents_between(-110.0, -130.0).unwrap(), 20.0, epsilon = 1e-9);
        assert_relative_eq!(cents_between(400.0, 350.0).unwrap(), 50.0, epsilon = 1e-9);
    }

    #[test]
    fn cents_across_the_pivot_do_not_jump_by_two_hundred() {
        // The bug this transform exists to prevent. A line moving from +105 to
        // -115 is an ordinary twenty-cent move; naive subtraction calls it two
        // hundred and twenty.
        assert_relative_eq!(cents_between(105.0, -115.0).unwrap(), 20.0, epsilon = 1e-9);
        assert_relative_eq!(105.0 - -115.0, 220.0, epsilon = 1e-9);

        // +100 and -100 are the same price, so the distance between them is nil.
        assert_relative_eq!(cents_between(100.0, -100.0).unwrap(), 0.0, epsilon = 1e-9);
    }

    #[test]
    fn shifting_and_measuring_are_inverses() {
        for american in [-400.0, -110.0, -101.0, 100.0, 145.0, 900.0] {
            for cents in [-75.0, -20.0, 0.0, 15.0, 60.0] {
                let moved = shift_cents(american, cents).unwrap();
                assert_relative_eq!(cents_between(american, moved).unwrap(), cents, epsilon = 1e-9);
            }
        }
    }

    #[test]
    fn a_shortened_price_is_always_a_shorter_price() {
        for american in [-300.0, -110.0, 110.0, 500.0] {
            let d = american_to_decimal(american).unwrap();
            let moved = american_to_decimal(shift_cents(american, 20.0).unwrap()).unwrap();
            assert!(moved < d, "{american} did not shorten: {d} -> {moved}");
        }
    }

    #[test]
    fn the_price_ladder_steps_evenly_in_cents_and_skips_the_hole() {
        let rungs = price_ladder(-300.0, 300.0, 50.0).unwrap();
        assert!(rungs.iter().all(|a| a.abs() >= 100.0), "{rungs:?}");
        for w in rungs.windows(2) {
            assert_relative_eq!(cents_between(w[1], w[0]).unwrap(), 50.0, epsilon = 1e-9);
        }
        assert_relative_eq!(rungs[0], -300.0, epsilon = 1e-9);
        assert!(rungs.contains(&-100.0), "even money should be a rung");
    }

    #[test]
    fn the_price_ladder_rejects_a_reversed_or_oversized_range() {
        assert!(price_ladder(300.0, -300.0, 10.0).is_err());
        assert!(price_ladder(-300.0, 300.0, 0.0).is_err());
        assert!(price_ladder(-100.0, 100_000.0, 1.0).is_err());
        assert!(price_ladder(-50.0, 300.0, 10.0).is_err());
    }

    #[test]
    fn implied_to_decimal_inverts_decimal_to_implied() {
        for decimal in [1.01, 1.5, 1.909_090_909, 2.0, 3.75, 51.0] {
            let prob = decimal_to_implied(decimal).unwrap();
            assert_relative_eq!(implied_to_decimal(prob).unwrap(), decimal, epsilon = 1e-12);
        }
    }

    #[test]
    fn implied_to_decimal_rejects_certainty() {
        // The TS returned Infinity here and priced a bet off it.
        assert!(implied_to_decimal(0.0).is_err());
        assert!(implied_to_decimal(1.0).is_err());
        assert!(implied_to_decimal(-0.1).is_err());
    }

    #[test]
    fn implied_to_decimal_does_not_round_like_american() {
        // 0.5238 is a -110 side before the vig comes out. The American scale
        // rounds it to -110 flat; the decimal price keeps the precision that
        // makes a fair-odds comparison meaningful.
        let fair = implied_to_decimal(0.523_809_5).unwrap();
        assert_relative_eq!(fair, 1.909_090_9, epsilon = 1e-6);
    }

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
