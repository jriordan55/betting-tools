# CLAUDE.md — working rules for this repo

Read this before touching anything. `tasks/todo.md` is the living plan and the
per-phase record; `docs/DIVERGENCES.md` is the catalog of bugs found in the
reference implementation; `tasks/lessons.md` is the list of mistakes already
made here. This file is the short version: the rules that, if broken, quietly
undo the point of the project.

---

## What this is

A Tauri v2 desktop app: **26 calculators, a SQLite bet log, and a bundled
reference library, with all math in Rust.**

It began as a port of `~/Code/bettor-calculator-main` (Next.js 16 / React 19, 21
calculators, ~2,800 lines of TypeScript math) and added three capabilities the
web app never had — an odds-range/variance module, the bet log, and a game
probability visualizer. All of that is now complete; work from here is
maintenance and extension, not porting.

The reference repo is **read-only**. Never edit it. It is the behavioral source
of truth that the golden vectors are captured from, and it is still the thing
`tools/gen-fixtures.mjs` runs.

---

## Non-negotiable rules

1. **No math in TypeScript. No Python anywhere.** If a number is computed, it is
   computed in `bettor-core`. The frontend formats and charts; it does not
   calculate. A `.toFixed()` is fine; a `1 / decimal` is not. This rule has been
   broken twice by accident, both times by a `1 - x` that looked too small to
   count — see `probability::complete_simplex` and
   `distributions::over_under_prob`, which exist because of it.

2. **`bettor-core` never depends on Tauri, on io, or on global state.**
   `cargo test -p bettor-core` must pass with no webview in sight. The core is
   the product; the desktop shell is one caller of it. The bet log obeys the same
   split: `ledger.rs` reads a betting record and has never heard of a database;
   `src-tauri/src/betlog.rs` owns the database and computes nothing.

3. **`bettor-core` never reads entropy.** Every simulation takes an explicit
   seed and reports back the one it used, so any figure a user quotes can be
   reproduced exactly. `fresh_seed()` in `src-tauri/src/lib.rs` is the only
   entropy in the application — keep it that way.

4. **Seeds cross the IPC boundary as decimal strings**, never as numbers. JSON
   numbers are IEEE doubles; a `u64` seed above 2^53 arrives corrupted, which
   silently breaks reproducibility in the least detectable way possible. See
   `bettor_core::seed_repr` and `commands::resolve_seed`.

5. **`src/lib/bindings.ts` is generated. Never hand-edit it.** It is produced by
   `cargo test -p bettor-desktop` (the `export_typescript_bindings` test), which
   `pnpm verify` runs. A forgotten regeneration shows up as a dirty working
   tree, not as a runtime `undefined`.

6. **Golden-vector fixtures are committed and must not be regenerated to make a
   test pass.** `crates/bettor-core/tests/fixtures/*.json` is the captured
   contract. A mismatch means either the port is wrong or the divergence is
   deliberate — and a deliberate divergence gets *declared*, not silenced.

7. **Declared divergences cannot rot.** The parity harness fails on an
   undeclared mismatch *and* on a stale declaration (one where the values now
   agree). If you fix a divergence, delete its declaration in the same commit.

8. **Bad user input returns `MathError`, never a panic.** Clippy denies
   `unwrap_used`, `expect_used`, `indexing_slicing`, `panic`, and `float_cmp` in
   `bettor-core`. Tests opt out at the top of the file with a `reason`.

9. **A number in a doc comment, an `InfoSection`, or a reference article is a
   claim, and it gets a test.** Write the assertion first with a placeholder,
   run it, read the real value out of the failure, and put *that* in both
   places. Never the other order. Eleven predicted figures have been wrong here
   and every one was caught this way; the four that were not caught shipped as
   explanatory text the app contradicted on screen. Prose reads as more
   authoritative than a number, so a wrong sentence beats a right calculation.

---

## Commands

```bash
pnpm verify        # THE GATE: clippy → cargo test → svelte-check → vitest → build
pnpm tauri dev     # run the app
pnpm tauri build   # produce a bundle

pnpm rs:test       # cargo test --workspace
pnpm rs:lint       # cargo clippy --workspace --all-targets -- -D warnings
pnpm check         # svelte-check
pnpm test          # vitest
pnpm bench         # Rust vs TypeScript on the same workloads

node tools/gen-fixtures.mjs   # regenerate golden vectors — see rule 6 first
```

`pnpm verify` is the gate. Nothing is "done" until it passes — and passing it is
not the same as the app working, which is rule 2 of `tasks/lessons.md`.

The fixture generator needs **Node 25+** (native TypeScript type stripping) and
reads `TS_ROOT`, defaulting to `~/Code/bettor-calculator-main/src/lib/math`.

---

## Layout

```
crates/bettor-core/     pure math — 21 modules, no Tauri, no io, no entropy
  src/                  odds, probability, hold, devig, distributions,
                        wager, arbitrage, parlay, clv, risk_of_ruin,
                        match_model, margin_model, line, regression, middle,
                        teaser, correlation, bayesian, variance, ledger, config
  examples/bench.rs     the Rust half of `pnpm bench`
  tests/parity.rs       golden-vector replay against the TypeScript
  tests/fixtures/       803 committed reference vectors
src-tauri/              shell: windowing, 53 IPC commands, SQLite
  src/commands.rs       thin adapters — deserialize, call core, serialize
  src/betlog.rs         rusqlite, WAL, migrations. Owns storage, computes nothing.
  src/lib.rs            command registry, fresh_seed(), bindings export test
src/                    SvelteKit — adapter-static, SSR off, Svelte 5 runes
  lib/bindings.ts       GENERATED. Do not edit.
  lib/calculators/      26 calculators
  lib/charts/           LineChart (multi-series), FanChart, Histogram, PmfChart
  lib/ui/               the shared component kit
  content/docs/         15 bundled reference articles
site/                   the GitHub Pages marketing page
tools/                  gen-fixtures.mjs, bench-ts.mjs
tasks/todo.md           the plan and the per-phase record
tasks/lessons.md        mistakes already made here
docs/                   divergences, testing strategy
```

---

## Frontend patterns

Settled in Phase 4; everything else is built against them.

- **`Async<D, T>`** (`src/lib/async.svelte.ts`) is how a calculator talks to the
  core. A deps function returns the inputs, a run function awaits the command.
  It is a class rather than an inline `$effect` because **`$effect` only tracks
  state read synchronously before the first `await`** — reading inputs inside
  the async callback yields an effect that never re-runs. The deps function makes
  the tracked read impossible to omit.
- **Parsing is all-or-nothing.** `parseAll` returns `null` if any leg fails.
  Pricing the legs that happened to parse and presenting the total as the user's
  is the Phase 1.5 parlay bug; the frontend must not reintroduce it at the input
  layer.
- **`describeError` switches on `MathError.kind`.** No string parsing crosses
  the boundary. `LogError` is a separate family with separate wording — a disk
  failure and a bad probability are different problems.
- **`format.ts` takes numbers and returns strings.** Nothing in it computes, and
  a non-finite input renders as an em dash rather than `NaN%`.
- **The one page that cannot use `Async`** (the bet log, which also refreshes
  imperatively after a write) guards staleness with a generation counter
  instead. Do not drop that.

---

## Conventions that matter

- **Everything is a 0–1 fraction inside the core.** The TypeScript mixed scales
  freely — EV in dollars, `evPercent` on 0–100, edge in percentage points — and
  formatted at the point of computation. Formatting is the frontend's job.
- **`edge` always means EV per unit staked**, never a win-rate surplus. Those are
  different numbers and the reference app displayed one under the other's name.
- **Struct fields are `#[serde(rename_all = "camelCase")]`.** Rust stays snake,
  TypeScript stays camel, nobody translates by hand.
- **TypeScript has one flat namespace.** `arbitrage::Leg` / `middle::Leg` and
  `ledger::Outcome` / `middle::Outcome` collide there. Disambiguate on export
  with `specta(rename)` and leave the Rust names idiomatic.
- **`Result<T, E>` is not expressible in a specta struct field.** Where a row
  can individually fail, use paired `Option<T>` + `Option<MathError>`.
- **Name legs for the direction that wins them**, not for which team holds them.
  `high_side` / `low_side` in `middle.rs` is why the sign bug cannot recur.
- **Anything that measures or moves a price in cents goes through
  `odds::cents_between` / `shift_cents`.** American odds have a hole in them:
  nothing lives strictly between -100 and +100 and the endpoints are the same
  price, so `+105` to `-115` is twenty cents and subtraction says 220.
- **A parity-critical approximation stays bit-identical to the TS even when the
  TS is less accurate.** The Abramowitz & Stegun constants are truncated in the
  reference (1.5e-7 error at z=0 rather than 7.5e-8). These price lines.
  Improving them would silently move every spread the app has ever quoted; that
  is its own change, with its own migration note.

---

## The recurring lesson

Nineteen-plus bugs were found in the reference TypeScript. Almost none were
formula errors. Nearly all were **seams** — a scale, sign, or convention
mismatch across a function boundary:

- `middle.ts` averaged a quantity with its own negation (two opposite sign
  conventions for spreads met in one function).
- `impliedTrueLine` fed a raw vigged probability into `Φ⁻¹`; `altline.ts` built
  a whole ladder on top of it.
- `devigShin` had `q/S` where Shin (1993) has `q²/S`, so the method silently
  returned MPTO's numbers — five devig methods, two of them duplicates.
- `fromDecimal` returned probability on 0–100 in a module where every sibling
  returned 0–1.
- `clv.rs` — mine, not theirs — measured cents by subtracting two American
  numbers, which is only cents while both sit on the same side of even money.

That is the actual argument for the Rust port, and it is why the type-level
work (newtypes, named sides, `MathError`, generated bindings) is not ceremony.
When you add a module, ask where its seams are before you check its formulas.

**And when two independent things agree exactly, suspect an identity.** Shin and
Equal Margin return bit-identical results on every two-way market. That looks
exactly like the bug above and is not one — they are provably the same function
there. Derive it before "fixing" it, then say so in the product, because an
identity the user cannot see is a claim the interface is making on their behalf.

---

## Working style for this repo

- Plan before non-trivial work; write the plan into `tasks/todo.md` and update
  it with results, decisions, and bugs found as each phase closes.
- New feature or fix → **code review and refactor** → commit. Not straight to
  commit.
- Never mark something complete without proving it: tests run, output shown.
  A green `pnpm verify` proves the code compiles and the math is right; it
  proves nothing about whether the UI looks correct. Say which one you have.
- Search only paths the user names. Do not sweep the filesystem guessing at
  filenames.
- Do not leave background processes running. A polling loop that outlives its
  build is a stray process the user has to find.
