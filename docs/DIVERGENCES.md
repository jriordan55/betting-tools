# Divergences from the reference TypeScript

Every place `bettor-core` deliberately differs from
`~/Code/bettor-calculator-main/src/lib/math`, and why.

Two kinds of entry:

- **Bug** — the TypeScript is wrong. The Rust is correct and a named regression
  test asserts the difference.
- **Deliberate** — both are defensible; the Rust made a different choice
  (`Result` instead of `null`, 0–1 instead of 0–100).

Anything that is *neither* is a port error, and the parity harness fails the
build for it. See [`TESTING.md`](TESTING.md).

Every numeric figure below was produced by **running the original TypeScript**,
not derived on paper.

---

## The four that change real numbers

These alter answers a user may already have acted on.

### 1. `devigShin` was not Shin — it was a second copy of MPTO

`devig.ts` had `q/S` inside the radical where Shin (1993) has `q²/S`:

```
π_i = (√(z² + 4(1−z)·q_i²/S) − z) / (2(1−z))
```

With the wrong term there is no interior root, so the bisection pinned `z` to
the ceiling of its search range on **every** input and the method returned
proportional (MPTO) numbers. The application offered five devig methods; two of
them were the same method.

Shin exists precisely to model insider-driven longshot bias, so the error is
largest exactly where the method was worth using:

| Market | TS (= MPTO) | Correct Shin |
|---|---|---|
| -1000 / +500, longshot fair prob | 0.1550 | **0.1288** |

Longshot fair probabilities were overstated by roughly 20% relative. Anyone who
devigged a longshot with "Shin" and compared it to MPTO was comparing a number
to itself.

`crates/bettor-core/src/devig.rs` · parity: `shin` fixture, 17 of 18 valid
markets differ.

### 2. `middle.ts` averaged a quantity with its own negation

`impliedTrueLine(-3.5, …)` returns the mean of *minus* the margin.
`impliedTrueLine(+7.5, …)` returns the mean of *plus* the margin. `middle.ts`
averaged the two.

| Case | TS | Correct |
|---|---|---|
| Both sides -5.5 at -110 → P(favorite covers) | **32.40%** | 50% |
| Same case → implied true center | **−0.8277** | 5.5 |
| Fav -3.5 (-110) $100 / dog +7.5 (+105) $120 → center | **1.798** *(outside the gap)* | ~5.5 |
| Same case → reported EV | **+$28.39 (+12.90%)** | recomputed |

Spreads were wrong on every input, always understating the favorite. Totals were
unaffected — over/under already sit in the same frame.

The Rust removes the trap by construction: legs are named `high_side` and
`low_side` for **the direction that wins them**, not for which team holds them,
and spread lines are folded onto the margin axis with `.abs()` before anything
else happens. `+7.5` and `-7.5` describe the same bet, so there is no second
convention to disagree with.

`crates/bettor-core/src/middle.rs`

### 3. `impliedTrueLine` never removed the vig

It fed the raw implied probability straight into `Φ⁻¹`. A team at -10.5 priced
-110 in a -110/-110 market has a fair cover probability of 0.50 and a true line
of exactly -10.5. The TypeScript computed `Φ⁻¹(0.5238)` and reported **−11.33**.

Eight tenths of a point of pure hold, presented as market information. It
cancels when comparing two books at identical prices — presumably why it
survived — but every displayed line value was shifted, and `altline.ts` built
its entire alternate-line ladder on top of it.

`crates/bettor-core/src/line.rs`

### 4. CLV was measured as a ratio of decimal odds

`CLVCalculator.tsx` displayed three rows:

| Label | Formula |
|---|---|
| Closing Line Value | `b/c − 1` |
| Edge (cents per dollar) | `(b − c)/c` |
| Expected Value | `(1/c)·b − 1` |

All three are the same expression. One number under three names.

Worse, that number is a ratio of decimal odds, which flatters longshots:

| Move | As a ratio | In probability points |
|---|---|---|
| +400 → +350 | **11.1%** | 2.22 |
| -110 → -130 | 7.9% | **4.14** |

The ratio ranks them in the opposite order from the truth. **This is exactly the
misconception the project exists to correct**, and the existing calculator
taught it.

`clv.rs` reports `ratio`, `prob_points`, and `cents` side by side, plus
`ratio_to_points_distortion()` to make the gap explicit.

Separately: CLV's EV never removed the vig either — `trueProbability =
1/closeDecimal` takes the raw closing price as truth, so betting the exact
closing number reports **0%** EV when the honest figure is **−4.5%**.
`ev_vs_fair` requires the opposing close and returns `None` without it rather
than guessing.

`crates/bettor-core/src/clv.rs`

---

## Input that was accepted when it should not have been

| # | Bug | Effect |
|---|---|---|
| 5 | American odds were never range-checked | `toDecimal("-1.5", "american")` → 67.67. Typing a spread into an odds field produced a plausible price. |
| 6 | `devigEM([0, 0.5])` → `[0.25, 0.75]` | Invented 25% for an outcome the book priced at zero. |
| 7 | Every devig method accepted a book summing below 1.0 | `[0.30, 0.30]` became a confident `[0.50, 0.50]`. That book is an arb, not a vigged market. |
| 8 | `if (!stake)` let negative stakes through | `!(-500)` is `false`, so EV, Kelly, Arbitrage and Hedge all computed confidently on a negative stake. |
| 9 | `analyzeTeaser([])` reduced over no legs from an initial `1` | An empty teaser reported certainty and an enormous positive EV. |
| 10 | `bayesianSpreadUpdate` clamped σ = 0 to 0.001 | A precision of a million — hands that source the answer outright and discards the other silently. Now an error. |
| 11 | A negative `marketN` produced negative Beta parameters | Posterior outside [0, 1]. |
| 12 | `dirichletProbUpdate` never checked its priors summed to 1 | An un-devigged set was accepted as a prior and silently misweighted. |
| 13 | `simulateRuin` returned a zeroed result on invalid input | A typo rendered as **0% risk of ruin** — the safest-looking answer possible. |
| 14 | `generateSamples` returned zeros for a lognormal with `mu ≤ 0` | Charts as certainty at zero. |

---

## Silent data loss

| # | Bug | Effect |
|---|---|---|
| 15 | `ParlayCalculator` dropped unparseable legs | Enter five legs, typo one, get a four-leg price presented as yours. |
| 16 | Score matrices were never renormalised | Truncating the grid at `max_score` drops real mass, so every derived price was biased low. How much depends on the grid: at baseball rates (μ≈4.5 and 4.2) a max of 10 drops **1.07%** of the joint distribution; the 16 shipped in `config.rs` drops 0.0007%. `truncation_mass()` now reports it. *(Corrected: this row previously read ~0.7%, which was a single marginal's tail rather than the joint truncation.)* |
| 17 | Spread and total **pushes** were dropped | `margin > spread` → home, `margin < spread` → away, exact tie → nowhere. The two sides silently failed to sum to 1 on whole-number lines. |
| 18 | `devigOR` gave up at `c > 50` with a bracket capped at `hi < 100` | Returned `null` for markets whose true exponent was larger. `[0.9999, 0.9999]` has the perfectly ordinary answer `[0.5, 0.5]` (true c ≈ 6931). |
| 19 | Solvers never reported non-convergence | They returned the last bisection iterate either way. `MathError::NoConvergence` now carries the residual. |
| 20 | `HedgeCalculator` reported "Guaranteed Profit" on a hedge that locks in a loss | `HedgeCalculator.tsx:141` renders that label whenever the mode is `guarantee`, changing only the colour when the number is negative. Now `Option`, gated on `is_arb()` / `worst_case`, so the guarantee lives in the type rather than in each caller's render logic. *(Corrected: this row previously named `ArbitrageCalculator`, which did gate the row on `isArbitrage` and never displayed it.)* |
| 21 | `convergenceSeries` accumulated a float step and rounded | Whenever `maxSample/100` was not whole, the x-axis came out unevenly spaced: a max of 150 gives 0, 2, 3, 5, 6, 8, 9, 11 — alternating gaps of 2 and 1. |

### 22. `samplePoisson` used a rounded normal above λ = 30

Its doc comment claimed Ahrens-Dieter rejection sampling. A normal is
symmetric; a Poisson is right-skewed. The approximation flattened exactly the
tail a prop line sits in. A regression test now asserts skew ≈ 1/√λ at λ = 50.

### 23. `simulateRuin` mixed populations in adjacent statistics

Median over *survivors*, mean over *all paths* with ruined runs scored as zero —
displayed side by side with nothing to explain the gap. At a 30% ruin rate the
median looks healthy while the mean is dragged down.

Now `median_ending_all`, `mean_ending_all`, and `median_ending_survivors`, each
named for its base.

---

## Duplicated implementations

`bayesian.ts` contained code that already existed elsewhere in the same
codebase:

- **`removeVig` / `removeVig3Way` were proportional devigging** — the third and
  fourth copies of `devigMPTO`. Deleted; callers use `devig::DevigMethod::Mpto`
  and get the other four methods for free.
- **`calculateMLEdge` reimplemented American conversion without the clamp**
  that `impliedToAmerican` has, so a posterior of exactly 0 or 1 returned
  `Infinity`.

Elsewhere: `nbinom.ts` was `poisson.ts` with a different marginal, and
`altline.ts` was built entirely on `bestline.ts`. Both collapsed into one Rust
module each (`match_model.rs`, `line.rs`).

Also: the Dirichlet update was hard-coded to exactly three outcomes. Nothing in
the mathematics requires that.

---

## Deliberate — not bugs

| Change | Reason |
|---|---|
| `Result<T, MathError>` everywhere, not `null` | The TS returned `null` on every failure mode, so the caller could not tell a bad price from a non-converging solver. `MathError` is a tagged union; the frontend matches on `kind`, it does not parse strings. |
| Everything is a 0–1 fraction | The components mixed scales freely — EV in dollars, `evPercent` on 0–100, edge in percentage points — and formatted with `.toFixed()` at the point of computation. Formatting is the frontend's job. (`fromDecimal` was itself returning probability on 0–100 in a module where every sibling returned 0–1.) |
| `to_american` returns `i32`, not a formatted string | Same reason. |
| Correlated parlays use a closed form for ρ ≥ 0 | One-factor Gaussian copula → a single Simpson integral. The TS drew 50,000 samples for a figure carrying ~0.2% noise and gave a different answer every run. Verified against the exact bivariate orthant `¼ + arcsin(ρ)/2π` to 5e-7 — the accuracy of `normal_cdf` itself, not of the integration. |
| Negative equicorrelation is reported, not silently clamped | An equicorrelated matrix is only positive definite for ρ > −1/(n−1), so five legs cannot be more negatively correlated than −0.25. The TS clamped and answered a question the caller had not asked. `correlation_was_clamped` says so. |
| Simulations are seeded | `Math.random()` made the TS output unreproducible even against itself. Every simulation takes a seed and returns the one it used. |
| Added: `Hedge::worst_case` | What you actually lock in, rather than making the reader compare two numbers. |
| Added: `longest_losing_streak`, `median_max_drawdown` | The statistic most underestimated at long prices, and the fact that drawdown is skewed enough that the mean alone misleads. |
| Kept: the truncated Abramowitz & Stegun constants | The TS uses `0.3989423`, giving ~1.5e-7 error at z=0 rather than the published 7.5e-8. **These price lines.** Changing them — even toward more accuracy — would silently move every spread the app has ever quoted. That is its own change, with its own migration note. Parity tolerance is relaxed to 2e-7 here and the reason is in the test. |

---

## Known limitations, stated rather than hidden

**Teaser cover probabilities come from a normal, which cannot see key numbers.**
Roughly 15% of NFL games end with a margin of exactly 3 and ~9% with exactly 7;
a normal at σ≈13.9 puts under 3% on each. So the model prices the textbook
two-team Wong teaser (-7.5 and -8.5, six points) at 44.6% against a 52.4%
breakeven and calls it -EV, while those legs are commonly reported hitting in
the low seventies.

The TypeScript displayed `keyNumbersCrossed` beside a probability that ignored
them, which reads as though the crossing were priced in.
`Teaser::model_ignores_key_numbers` now says plainly that it is not. Pricing the
discrete mass needs an empirical margin distribution per sport — a data problem,
and its own change.

---

## Bugs found in the port itself

Recorded because they are the argument for the harness.

**The bisection loops tested for convergence before narrowing the bracket**,
where the TypeScript narrows first. Same algorithm, final midpoint off in the
last few ulps. It would never have surfaced without golden vectors. Fixed in
`or_inner`, `log_inner`, and `shin_inner`, with the ordering commented so it is
not "cleaned up" later.

**An overstatement in the port's own documentation.** `convergenceSeries` was
described here as producing *duplicated and skipped* sample sizes. Duplicates
are impossible — the step is `max(1, …)`, so rounding cannot collide. Only the
spacing is uneven. Claim, test, and parity check all corrected.

---

## Latent bug in the reference repo, not fixed there

`altline.ts` imports `BetType` and `TotalSide` from `./bestline` as ordinary
named imports, but both are **type aliases**. A bundler elides them; native type
stripping does not, and the module fails to instantiate. The file will not load
under `verbatimModuleSyntax` or any non-bundler toolchain.

`tools/gen-fixtures.mjs` works around it by staging a patched copy of the source
in a temp directory. It is a real bug, but that repo is not ours to edit.
