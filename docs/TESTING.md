# Testing strategy

Three layers, each covering what the others cannot.

| Layer | Location | Covers |
|---|---|---|
| Golden-vector parity | `crates/bettor-core/tests/parity.rs` | The port matches the original TypeScript, or differs for a declared reason |
| Unit / behavioral | `#[cfg(test)]` in each module | Hand-computed values, edge cases, and every deliberate divergence |
| Distributional | simulation modules | What parity cannot reach, because the TS used `Math.random()` |

`pnpm verify` runs all of it plus clippy and svelte-check. Nothing is done until
it passes.

---

## Golden-vector parity

`tools/gen-fixtures.mjs` imports the **original TypeScript modules** from
`~/Code/bettor-calculator-main/src/lib/math`, runs them across a wide input
sweep — including the awkward cases: odds at ±100, probabilities at 0 and 1,
empty arrays, negative lines, degenerate distributions — and dumps the results
to `crates/bettor-core/tests/fixtures/*.json`.

`parity.rs` replays those files against the Rust. "The port is correct" becomes
a test result rather than a claim.

**803 vectors across 8 fixture files.**

| Fixture | Identical | Declared divergences |
|---|---|---|
| `probability.ts` | 132 / 132 | 0 |
| `odds.ts` | 284 | 24 |
| `devig.ts` | 83 | 21 |
| `hold.ts` | 14 | 1 |
| `vigComparison.ts` | 5 / 5 | 0 |
| `poisson.ts` / `nbinom.ts` | 439 / 439 | 0 |
| `bestline.ts` / `altline.ts` | 95 / 95 | 0 |
| `regression.ts` | 30 | 1 |
| `shin` | *rewritten* | 17 of 18 valid markets |

### The rule that makes it work

The harness fails on **both** kinds of drift:

- an **undeclared mismatch** — the port is wrong, or the divergence has not been
  thought about yet;
- a **stale declaration** — a case declared as divergent where the values now
  agree.

The second is the one that matters over time. Without it, the divergence list
rots into a permanent excuse and the harness quietly stops testing anything. If
you fix a divergence, delete its declaration in the same commit.

### Fixtures are committed. Do not regenerate them to make a test pass.

```bash
node tools/gen-fixtures.mjs
TS_ROOT=/path/to/bettor-calculator-main/src/lib/math node tools/gen-fixtures.mjs
```

Requires **Node 25+** (native TypeScript type stripping — no loader dependency).

They are deliberately **not** gitignored. Regenerating on demand would let a
regression rewrite its own expected output, which is the exact failure mode the
whole strategy exists to prevent. A red parity test means one of two things:

1. the port is wrong — fix the port; or
2. the difference is intended — record it in `EXPECTED_DIVERGENCES` **and** in
   [`DIVERGENCES.md`](DIVERGENCES.md), with the reason.

There is no third option where you re-run the generator.

### Two techniques worth reusing

**Where the port deliberately transforms the result, isolate the transform.**
Score matrices are renormalised in Rust and were not in the TS. The fixture
stores the raw TS grid; the test divides by its total before comparing. That
isolates the one intended difference and still checks the marginal arithmetic
exactly.

**Where the port deliberately changes an input, feed the old input.**
`implied_true_line` now removes vig and the TS did not. The parity test hands
the Rust the same raw probability the TS used, which verifies the inversion
arithmetic. A separate named test asserts the behavior change itself — 0.828
points at -110.

### The generator stages a patched copy of the source

`altline.ts` imports two type aliases as value imports, which a bundler elides
and native type stripping does not. `stageSource()` copies the reference source
to a temp dir and patches it there. The reference repo is read-only; that bug is
recorded in [`DIVERGENCES.md`](DIVERGENCES.md) but not fixed in place.

---

## Unit and behavioral tests

190 of them, colocated in each module.

Two things they carry that parity cannot:

**Phase 1.5 has no golden vectors at all.** The math for EV, Kelly, Arbitrage,
Hedge, Parlay, and CLV lived inside `useMemo` blocks, entangled with React state
and `.toFixed()` formatting. There was no callable function to capture. Those
modules are covered by hand-computed values instead.

**Every deliberate divergence gets a named test.** `negative_stakes_are_rejected`
sits directly under the comment explaining that `if (!stakeNum)` is `false` for
`-500`. The test is the enforcement; the doc comment is only the reason.

Tests opt out of the strict clippy set at the top of the file, with a reason:

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests { … }
```

---

## Simulation cannot be parity-tested

The TypeScript calls `Math.random()`, so its output is not reproducible even
against itself. There is nothing to capture.

Those modules get:

- **Distributional tests** — mean, variance, and skew against closed forms.
  This is what caught `samplePoisson` using a symmetric normal above λ = 30
  while claiming Ahrens-Dieter rejection: the test asserts skew ≈ 1/√λ at λ = 50.
- **Determinism tests** — same seed, identical output. Possible for the first
  time because of the seeded `ChaCha8Rng`.
- **Closed-form cross-checks** where one exists. The correlated-parlay integral
  is verified against the exact bivariate orthant `¼ + arcsin(ρ)/2π` and matches
  to 5e-7 — the accuracy of `normal_cdf` itself, not of the integration.

---

## Epsilons

Stated per test, never loosened silently.

- `1e-9` — closed-form math.
- Looser, with a comment — iterative solvers, where the last few ulps depend on
  bisection ordering.
- `2e-7` — anything downstream of `normal_cdf`. The Abramowitz & Stegun
  constants are truncated in the reference (`0.3989423`), giving ~1.5e-7 error at
  z=0. **Parity beats accuracy here**: these approximations price lines, and
  changing them would move every spread the app has quoted. See
  [`DIVERGENCES.md`](DIVERGENCES.md).

`approx`'s `epsilon` is **absolute** unless `max_relative` is also given. Getting
that wrong is how a parlay test came to expect `6.9596` against an actual
`6.957926`.

---

## Property tests

`proptest` is wired in for invariants the fixtures cannot enumerate:

- devig output always sums to 1.0
- American → decimal → American round-trips (except ±100, which are the same
  price — decimal 2.0 — so the mapping is deliberately not injective there;
  `plus_and_minus_one_hundred_are_the_same_price` asserts that on purpose)
- every PMF sums to ~1.0 over its support
- CDFs are monotonic
- arbitrage stakes always sum to the total

---

## Definition of done for a module

1. Golden vectors pass, or every divergence is declared in both places.
2. Properties pass.
3. `cargo clippy -D warnings` is clean.
4. Every deliberate divergence has a named test asserting it.

---

## The bindings export is a test

`export_typescript_bindings` in `src-tauri/src/lib.rs` regenerates
`src/lib/bindings.ts`.

A test rather than a build script, deliberately: `pnpm verify` already runs
`cargo test`, so the bindings refresh as part of the normal loop, and a forgotten
regeneration surfaces as a **dirty working tree** rather than as a runtime
`undefined` in the frontend.

`builder()` is shared between `run()` and the test, so the generated TypeScript
can never describe a different set of commands than the ones the application
actually registers.
