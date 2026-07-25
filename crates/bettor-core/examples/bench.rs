//! Times the Rust math against the same workloads as `tools/bench-ts.mjs`.
//!
//! ```text
//! cargo run --release --example bench
//! cargo run --release --example bench -- --json
//! ```
//!
//! # What this measures, and what it does not
//!
//! It is a wall-clock loop with a warm-up, not a statistical benchmark. There
//! are no confidence intervals and no outlier rejection, so read a 2× result
//! as noise and a 20× result as real. Criterion would give better numbers; it
//! would also be a dependency and a minute of build time for a figure that is
//! quoted once in a README.
//!
//! Three caveats that matter more than the measurement error:
//!
//! 1. **The RNGs differ.** `Math.random()` is xorshift128+; this crate uses
//!    ChaCha8, which is cryptographic and deliberately slower. The risk-of-ruin
//!    comparison therefore *understates* Rust, and the reason for the choice —
//!    a seed that reproduces a run exactly — is worth more than the cycles.
//! 2. **The risk-of-ruin case is parallel here and serial in Node.** Divide by
//!    core count for the single-thread figure. Both are true things about the
//!    shipped software; only one is a language comparison.
//! 3. **`devig_shin` is not the same algorithm.** The reference's bisection had
//!    no interior root and pinned to its search ceiling on every input, so it
//!    was doing a fixed 100 iterations of nothing. Timing a correct solver
//!    against a broken one is not a fair race in either direction.
//!
//! Everything else — `normal_cdf`, `american_to_decimal`, `devig_or`, the
//! score matrix — is the same algorithm on both sides and is a fair comparison.

// A timing harness, not shipped code. The crate's restriction lints exist so
// that bad user input cannot unwind a desktop app; nothing here takes user
// input, and the indices are all `i % len`.
#![allow(
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    reason = "benchmark harness: every index is `i % len`, and a nanosecond \
              count that exceeds f64's mantissa is a run measured in decades"
)]

use bettor_core::{devig, match_model, odds, probability, risk_of_ruin};
use std::hint::black_box;
use std::time::Instant;

/// Runs `run(i)` `iters` times and returns nanoseconds per operation.
///
/// `run` takes the iteration index and must use it. Both harnesses cycle
/// inputs for the same reason: a call with a constant argument gets hoisted
/// out of the loop — by LLVM here, by V8 there — and what is left is loop
/// overhead wearing the algorithm's name.
fn time(name: &str, iters: u32, mut run: impl FnMut(u32)) -> (String, f64) {
    // Warm-up, so the first-touch page faults and cache misses are not charged
    // to the algorithm. The TypeScript side warms up for the same reason.
    for i in 0..iters.min(10_000) {
        run(i);
    }

    let start = Instant::now();
    for i in 0..iters {
        run(i);
    }
    let elapsed = start.elapsed();
    (name.to_owned(), elapsed.as_nanos() as f64 / f64::from(iters))
}

fn main() {
    let json = std::env::args().any(|a| a == "--json");

    let ruin_input = risk_of_ruin::RuinInput {
        win_prob: 0.55,
        decimal_odds: 1.909_090_909,
        bet_size: 100.0,
        bankroll: 10_000.0,
        num_bets: 500,
        num_sims: 1_000,
    };

    // The same inputs the TypeScript harness cycles, in the same order.
    const ZS: [f64; 8] = [0.7231, -1.4, 0.05, 2.2, -0.33, 1.11, -2.6, 0.9];
    const PRICES: [&str; 8] = [
        "-110", "150", "-250", "+320", "-105", "900", "-1200", "225",
    ];
    const MARKETS: [[f64; 2]; 4] = [
        [0.5238, 0.5238],
        [0.62, 0.43],
        [0.3, 0.75],
        [0.51, 0.52],
    ];
    const LAMBDAS: [[f64; 2]; 4] = [[2.4, 1.8], [1.55, 1.2], [3.15, 2.85], [4.5, 4.2]];

    let pick = |i: u32, n: usize| (i as usize) % n;

    let results = vec![
        time("normal_cdf", 2_000_000, |i| {
            black_box(probability::normal_cdf(black_box(ZS[pick(i, ZS.len())])));
        }),
        time("american_to_decimal", 2_000_000, |i| {
            let price = PRICES[pick(i, PRICES.len())];
            black_box(odds::to_decimal(black_box(price), odds::OddsFormat::American).ok());
        }),
        time("devig_shin (2-way)", 200_000, |i| {
            let market = &MARKETS[pick(i, MARKETS.len())];
            black_box(devig::devig(black_box(market), devig::DevigMethod::Shin).ok());
        }),
        time("devig_or (2-way)", 200_000, |i| {
            let market = &MARKETS[pick(i, MARKETS.len())];
            black_box(devig::devig(black_box(market), devig::DevigMethod::Or).ok());
        }),
        time("devig_all (5 methods)", 50_000, |i| {
            let market = &MARKETS[pick(i, MARKETS.len())];
            black_box(devig::devig_all(black_box(market)));
        }),
        time("score_matrix 16x16", 20_000, |i| {
            let [home, away] = LAMBDAS[pick(i, LAMBDAS.len())];
            black_box(match_model::ScoreMatrix::poisson(black_box(home), black_box(away), 16).ok());
        }),
        time("risk_of_ruin 1k paths x 500 bets", 3, |i| {
            let input = risk_of_ruin::RuinInput {
                win_prob: 0.55 + f64::from(i % 3) * 0.001,
                ..ruin_input
            };
            black_box(risk_of_ruin::simulate_ruin(black_box(&input), 42).ok());
        }),
    ];

    if json {
        let rows: Vec<String> = results
            .iter()
            .map(|(name, ns)| format!("{{\"name\":\"{name}\",\"nsPerOp\":{ns}}}"))
            .collect();
        println!("[{}]", rows.join(","));
    } else {
        let threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        println!("Rust (release, {threads} cores available to rayon)\n");
        for (name, ns) in &results {
            println!("  {name:<34} {ns:>12.1} ns/op");
        }
    }
}
