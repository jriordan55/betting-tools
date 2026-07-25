//! Parity against the TypeScript implementation.
//!
//! Every case in `tests/fixtures/` was produced by running the original
//! `bettor-calculator-main` code. This suite replays them through the Rust and
//! requires agreement — except where the port deliberately diverges, in which
//! case the divergence must be declared in [`Divergence`] with a reason.
//!
//! An undeclared mismatch fails the build. A declared one is reported in the
//! summary so the list of intentional behavior changes stays visible instead of
//! decaying into folklore.

#![allow(
    clippy::float_cmp,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test harness"
)]

use bettor_core::devig::{self, DevigMethod};
use bettor_core::hold;
use bettor_core::odds::{self, OddsFormat};
use bettor_core::probability;
use serde_json::Value;

// ---------------------------------------------------------------- machinery

/// Relative tolerance for values produced by identical arithmetic.
///
/// Not zero: `exp`, `ln`, and `powf` may differ by an ulp between V8's libm and
/// Rust's, and those feed the iterative solvers.
const EPS: f64 = 1e-12;

/// A deliberate behavior change, with the reason it exists.
struct Divergence {
    reason: &'static str,
}

fn diverges(reason: &'static str) -> Option<Divergence> {
    Some(Divergence { reason })
}

#[derive(Default)]
struct Report {
    checked: usize,
    matched: usize,
    declared: Vec<String>,
    failures: Vec<String>,
}

impl Report {
    /// Records one comparison.
    ///
    /// `agree` is whether Rust reproduced the TypeScript. `expected` is
    /// `Some(..)` when this specific input is a declared divergence.
    fn record(&mut self, label: String, agree: bool, expected: Option<Divergence>) {
        self.checked += 1;
        match (agree, expected) {
            (true, None) => self.matched += 1,
            (true, Some(d)) => self.failures.push(format!(
                "{label}: declared a divergence ({}) but the values MATCHED — \
                 the declaration is stale and should be removed",
                d.reason
            )),
            (false, Some(d)) => self.declared.push(format!("{label}: {}", d.reason)),
            (false, None) => self
                .failures
                .push(format!("{label}: UNDECLARED mismatch vs TypeScript")),
        }
    }

    fn finish(self, name: &str) {
        println!(
            "\n{name}: {} cases — {} identical, {} declared divergences",
            self.checked,
            self.matched,
            self.declared.len()
        );
        let mut seen = std::collections::BTreeSet::new();
        for d in &self.declared {
            let reason = d.split(": ").nth(1).unwrap_or(d);
            if seen.insert(reason.to_owned()) {
                println!("    ~ {reason}");
            }
        }
        assert!(
            self.failures.is_empty(),
            "\n{} parity failure(s) in {name}:\n  {}\n",
            self.failures.len(),
            self.failures.join("\n  ")
        );
    }
}

fn load(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("missing fixture {path}: {e}\nrun: node tools/gen-fixtures.mjs"));
    serde_json::from_str(&text).expect("fixture is not valid JSON")
}

fn cases<'a>(fixture: &'a Value, key: &str) -> &'a Vec<Value> {
    fixture[key]
        .as_array()
        .unwrap_or_else(|| panic!("fixture has no case array named {key}"))
}

/// Decodes a JSON number, including the tagged non-finite spellings.
fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => match s.as_str() {
            "NaN" => Some(f64::NAN),
            "Infinity" => Some(f64::INFINITY),
            "-Infinity" => Some(f64::NEG_INFINITY),
            _ => None,
        },
        _ => None,
    }
}

fn close(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a == b {
        return true;
    }
    let scale = a.abs().max(b.abs()).max(1.0);
    (a - b).abs() <= EPS * scale
}

/// Compares a Rust `Result` against a TypeScript value that may be `null`.
fn agrees(rust: &Result<f64, bettor_core::MathError>, ts: &Value) -> bool {
    match (rust, num(ts)) {
        (Ok(r), Some(t)) => close(*r, t),
        (Err(_), None) => true, // TS null ≙ Rust Err
        _ => false,
    }
}

fn slice_agrees(rust: &Result<Vec<f64>, bettor_core::MathError>, ts: &Value) -> bool {
    match (rust, ts.as_array()) {
        (Ok(r), Some(t)) => {
            r.len() == t.len()
                && r.iter()
                    .zip(t)
                    .all(|(a, b)| num(b).is_some_and(|b| close(*a, b)))
        }
        (Err(_), None) => true,
        _ => false,
    }
}

// ------------------------------------------------------------------- odds.ts

/// American inputs the TS mapped to real-looking decimals despite not being
/// prices at all. This is the single most user-visible divergence.
const NOT_REAL_PRICES: [&str; 6] = ["50", "-50", "99", "-99", "1.5", "-1.5"];

fn american_divergence(value: &str) -> Option<Divergence> {
    if NOT_REAL_PRICES.contains(&value) {
        return diverges("American prices inside ±100 are rejected; the TS converted them anyway");
    }
    if value.trim() == "Infinity" {
        // TS: 1 + Infinity/100 = Infinity, a "price" with zero implied probability.
        return diverges("an infinite price is rejected; the TS returned infinite decimal odds");
    }
    None
}

#[test]
fn odds_parity() {
    let fx = load("odds");
    let mut r = Report::default();

    for c in cases(&fx, "to_decimal") {
        let value = c["value"].as_str().unwrap();
        let format = match c["format"].as_str().unwrap() {
            "american" => OddsFormat::American,
            "decimal" => OddsFormat::Decimal,
            _ => OddsFormat::Fractional,
        };
        let got = odds::to_decimal(value, format);
        let expected = match format {
            OddsFormat::American => american_divergence(value),
            // "-1/2" gave decimal 0.5 and "0/1" gave decimal 1.0 — prices that
            // pay back less than the stake, and exactly the stake.
            OddsFormat::Fractional
                if value.starts_with('-') || value.starts_with("0/") =>
            {
                diverges(
                    "fractional terms must both be positive; the TS allowed a decimal of 1.0 or below",
                )
            }
            _ => None,
        };
        r.record(
            format!("to_decimal({value:?}, {format:?})"),
            agrees(&got, &c["out"]),
            expected,
        );
    }

    for c in cases(&fx, "to_american") {
        let decimal = num(&c["decimal"]).unwrap();
        // TS returned a formatted string; Rust returns the number.
        let ts = c["out"].as_str().map(|s| s.trim_start_matches('+').parse::<i32>().unwrap());
        let got = odds::to_american(decimal);
        let agree = match (&got, ts) {
            (Ok(g), Some(t)) => *g == t,
            (Err(_), None) => true,
            _ => false,
        };
        r.record(format!("to_american({decimal})"), agree, None);
    }

    for c in cases(&fx, "from_decimal") {
        let decimal = num(&c["decimal"]).unwrap();
        let got = odds::from_decimal(decimal);
        let out = &c["out"];
        let agree = match (&got, num(&out["probability"])) {
            // TS reported probability on a 0-100 scale from this one function.
            (Ok(g), Some(p)) => close(g.probability, p / 100.0),
            (Err(_), None) => true,
            _ => false,
        };
        r.record(format!("from_decimal({decimal}).probability"), agree, None);

        // Fractional rendering is compared separately: the TS emitted "0/1"
        // for prices too short to round to a whole numerator.
        if let (Ok(g), Some(frac)) = (&got, out["fractional"].as_str()) {
            let ours = format!("{}/{}", g.fractional_num, g.fractional_den);
            let expected = (frac == "0/1").then_some(Divergence {
                reason: "\"0/1\" replaced by the nearest tabulated fraction",
            });
            r.record(
                format!("from_decimal({decimal}).fractional"),
                ours == frac,
                expected,
            );
        }
    }

    for c in cases(&fx, "american_to_implied") {
        let value = c["value"].as_str().unwrap();
        let got = match odds::to_decimal(value, OddsFormat::American) {
            Ok(d) => Ok(1.0 / d),
            Err(e) => Err(e),
        };
        r.record(
            format!("american_to_implied({value:?})"),
            agrees(&got, &c["out"]),
            american_divergence(value),
        );
    }

    for c in cases(&fx, "implied_to_american") {
        let prob = num(&c["prob"]).unwrap();
        let ts = num(&c["out"]).unwrap();
        let got = odds::implied_to_american(prob);
        r.record(
            format!("implied_to_american({prob})"),
            close(f64::from(got), ts),
            None,
        );
    }

    for c in cases(&fx, "decimal_to_implied") {
        let decimal = num(&c["decimal"]).unwrap();
        r.record(
            format!("decimal_to_implied({decimal})"),
            agrees(&odds::decimal_to_implied(decimal), &c["out"]),
            None,
        );
    }

    for c in cases(&fx, "odds_to_implied") {
        let value = c["value"].as_str().unwrap();
        let format = if c["format"].as_str().unwrap() == "american" {
            OddsFormat::American
        } else {
            OddsFormat::Decimal
        };
        let expected = if format == OddsFormat::American {
            american_divergence(value)
        } else {
            None
        };
        r.record(
            format!("odds_to_implied({value:?}, {format:?})"),
            agrees(&odds::odds_to_implied(value, format), &c["out"]),
            expected,
        );
    }

    r.finish("odds.ts");
}

// ------------------------------------------------------------ probability.ts

#[test]
fn probability_parity() {
    let fx = load("probability");
    let mut r = Report::default();

    for c in cases(&fx, "normal_cdf") {
        let z = num(&c["z"]).unwrap();
        let ts = num(&c["out"]).unwrap();
        r.record(
            format!("normal_cdf({z})"),
            close(probability::normal_cdf(z), ts),
            None,
        );
    }

    for c in cases(&fx, "inverse_normal_cdf") {
        let p = num(&c["p"]).unwrap();
        let ts = num(&c["out"]).unwrap();
        r.record(
            format!("inverse_normal_cdf({p})"),
            close(probability::inverse_normal_cdf(p), ts),
            None,
        );
    }

    for c in cases(&fx, "prob_to_spread") {
        let (p, std) = (num(&c["p"]).unwrap(), num(&c["std"]).unwrap());
        let ts = num(&c["out"]).unwrap();
        r.record(
            format!("prob_to_spread({p}, {std})"),
            close(probability::prob_to_spread(p, std), ts),
            None,
        );
    }

    for c in cases(&fx, "prob_to_beta") {
        let (p, n) = (num(&c["p"]).unwrap(), num(&c["n"]).unwrap());
        let out = &c["out"];
        let got = probability::prob_to_beta(p, n).unwrap();
        let agree = close(got.alpha, num(&out["alpha"]).unwrap())
            && close(got.beta, num(&out["beta"]).unwrap());
        r.record(format!("prob_to_beta({p}, {n})"), agree, None);
    }

    r.finish("probability.ts");
}

// -------------------------------------------------------------- hold.ts

#[test]
fn hold_parity() {
    let fx = load("hold");
    let mut r = Report::default();

    for c in cases(&fx, "calculate_hold") {
        let a = num(&c["implied_a"]).unwrap();
        let b = num(&c["implied_b"]).unwrap();
        let got = hold::calculate_hold(a, b);
        let out = &c["out"];

        let agree = match (&got, out.is_null()) {
            (Ok(g), false) => {
                // TS holdPct was in percentage points; ours is a fraction.
                close(g.hold, num(&out["holdPct"]).unwrap() / 100.0)
                    && close(g.no_vig_prob_a, num(&out["noVigProbA"]).unwrap())
                    && close(g.no_vig_prob_b, num(&out["noVigProbB"]).unwrap())
            }
            (Err(_), true) => true,
            _ => false,
        };
        let expected = ((a >= 1.0) || (b >= 1.0)).then_some(Divergence {
            reason: "an implied probability of 1.0 or more is rejected; the TS accepted it",
        });
        r.record(format!("calculate_hold({a}, {b})"), agree, expected);
    }

    r.finish("hold.ts");
}

#[test]
fn vig_comparison_parity() {
    let fx = load("vig_comparison");
    let mut r = Report::default();

    for (i, c) in cases(&fx, "compare_vig").iter().enumerate() {
        let books: Vec<(String, f64, f64)> = c["books"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| {
                (
                    b["name"].as_str().unwrap().to_owned(),
                    num(&b["implied_a"]).unwrap(),
                    num(&b["implied_b"]).unwrap(),
                )
            })
            .collect();

        let ranked = hold::compare_vig(&books);
        // The TS dropped invalid books entirely; we keep them at the bottom.
        // Compare the valid prefix, which is what the table actually shows.
        let ours: Vec<&String> = ranked
            .iter()
            .filter(|(_, res)| res.is_ok())
            .map(|(name, _)| name)
            .collect();
        let theirs: Vec<String> = c["out"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["name"].as_str().unwrap().to_owned())
            .collect();

        let agree = ours.len() == theirs.len()
            && ours.iter().zip(&theirs).all(|(a, b)| **a == *b);
        r.record(format!("compare_vig(set {i}) ordering"), agree, None);
    }

    r.finish("vigComparison.ts");
}

// ------------------------------------------------------------------ devig.ts

fn probs_of(c: &Value) -> Vec<f64> {
    c["probs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(num)
        .collect()
}

/// Why a devig input is expected to behave differently now.
///
/// Takes the TypeScript output as well as the input: where the TS *also*
/// failed, the stricter validation changes nothing observable and there is no
/// divergence to declare.
fn devig_divergence(probs: &[f64], ts_out: &Value) -> Option<Divergence> {
    if ts_out.is_null() {
        return None;
    }
    if probs.len() < 2 {
        return diverges("single-outcome markets are rejected; the TS returned [1.0]");
    }
    if probs.iter().any(|p| !p.is_finite() || *p <= 0.0 || *p >= 1.0) {
        return diverges("outcomes at exactly 0 or 1 are rejected; the TS devigged them anyway");
    }
    if probs.iter().sum::<f64>() < 1.0 {
        return diverges(
            "a book summing below 1.0 is reported as an arb; the TS devigged it to a confident split",
        );
    }
    None
}

#[test]
fn devig_parity() {
    let fx = load("devig");
    let mut r = Report::default();

    for (key, method) in [
        ("em", DevigMethod::Em),
        ("mpto", DevigMethod::Mpto),
        ("or", DevigMethod::Or),
        ("log", DevigMethod::Log),
    ] {
        for c in cases(&fx, key) {
            let probs = probs_of(c);
            let got = devig::devig(&probs, method);
            let expected = devig_divergence(&probs, &c["out"]).or_else(|| {
                // The TS capped the odds-ratio search at hi < 100 and then
                // bailed out whenever the solved exponent exceeded 50. For a
                // market like [0.9999, 0.9999] the true exponent is ~6931 and
                // the answer is a perfectly ordinary [0.5, 0.5], which every
                // other method returns. Here the bracket expands until it
                // actually contains the root.
                (method == DevigMethod::Or && c["out"].is_null() && got.is_ok()).then_some(
                    Divergence {
                        reason: "the TS abandoned the odds-ratio solve at c > 50; this one brackets the root and converges",
                    },
                )
            });
            r.record(
                format!("{key}({probs:?})"),
                slice_agrees(&got, &c["out"]),
                expected,
            );
        }
    }

    for c in cases(&fx, "calc_devig_ev") {
        let fair = num(&c["fair_prob"]).unwrap();
        let bet = num(&c["bet_implied"]);
        let got = match bet {
            Some(b) => devig::ev_vs_fair(fair, b),
            None => Err(bettor_core::MathError::ProbabilityOutOfRange {
                value: f64::NAN,
                reason: "absent",
            }),
        };
        r.record(
            format!("ev_vs_fair({fair}, {bet:?})"),
            agrees(&got, &c["out"]),
            None,
        );
    }

    r.finish("devig.ts");
}

/// Shin is a rewrite, not a port, so it gets its own test rather than a
/// blanket exemption inside the parity sweep.
#[test]
fn shin_is_deliberately_not_a_port() {
    let fx = load("devig");
    let mut differences = 0;
    let mut compared = 0;

    for c in cases(&fx, "shin") {
        let probs = probs_of(c);
        if devig_divergence(&probs, &c["out"]).is_some() || c["out"].is_null() {
            continue;
        }
        let Ok(ours) = devig::devig(&probs, DevigMethod::Shin) else {
            continue;
        };
        let Some(theirs) = c["out"].as_array() else {
            continue;
        };
        compared += 1;

        // Every Shin result must still be a valid distribution.
        let total: f64 = ours.iter().sum();
        assert!(
            (total - 1.0).abs() < 1e-9,
            "Shin output for {probs:?} sums to {total}, not 1"
        );

        let max_delta = ours
            .iter()
            .zip(theirs)
            .filter_map(|(a, b)| num(b).map(|b| (a - b).abs()))
            .fold(0.0_f64, f64::max);
        if max_delta > 1e-6 {
            differences += 1;
        }

        // The TS collapsed onto proportional. Confirm ours no longer does,
        // wherever the market is asymmetric enough for the models to disagree.
        if let Ok(mpto) = devig::devig(&probs, DevigMethod::Mpto) {
            let spread = probs.iter().fold(0.0_f64, |m, p| m.max(*p))
                - probs.iter().fold(1.0_f64, |m, p| m.min(*p));
            // The models only diverge when there is margin to distribute AND
            // an asymmetry to distribute it across. On a vig-free book Shin
            // agreeing with proportional is correct, not a regression.
            let vig = probs.iter().sum::<f64>() - 1.0;
            if spread > 0.3 && vig > 0.01 {
                let gap = ours
                    .iter()
                    .zip(&mpto)
                    .map(|(a, b)| (a - b).abs())
                    .fold(0.0_f64, f64::max);
                assert!(
                    gap > 1e-3,
                    "Shin still matches proportional on {probs:?} — the fix did not take"
                );
            }
        }
    }

    assert!(compared > 0, "no comparable Shin cases in the fixture");
    assert!(
        differences > 0,
        "Shin now matches the TS everywhere, which would mean the q²/S fix was lost"
    );
    println!(
        "\nshin: rewritten, not ported — differs from the TS on {differences}/{compared} valid markets"
    );
}
