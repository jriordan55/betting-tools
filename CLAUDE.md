# CLAUDE.md — working rules for this repo

Read this before touching anything. `tasks/todo.md` is the living plan and the
per-phase record; `docs/DIVERGENCES.md` is the catalog of bugs found in the
reference implementation. This file is the short version: the rules that, if
broken, quietly undo the point of the project.

---

## What this is

A Tauri v2 desktop port of `~/Code/bettor-calculator-main` (Next.js 16 / React
19, 21 calculators, ~2,800 lines of TypeScript math), with **all math moved to
Rust**, plus three new capabilities: an odds-range/variance module, a SQLite bet
log, and a ported probability visualizer.

The reference repo is **read-only**. Never edit it. It is the behavioral source
of truth that the golden vectors are captured from.

---

## Non-negotiable rules

1. **No math in TypeScript. No Python anywhere.** If a number is computed, it is
   computed in `bettor-core`. The frontend formats and charts; it does not
   calculate. A `.toFixed()` is fine; a `1 / decimal` is not.

2. **`bettor-core` never depends on Tauri, on io, or on global state.**
   `cargo test -p bettor-core` must pass with no webview in sight. The core is
   the product; the desktop shell is one caller of it.

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

---

## Commands

```bash
pnpm verify        # clippy -D warnings → cargo test --workspace → svelte-check
pnpm tauri dev     # run the app
pnpm rs:test       # cargo test --workspace
pnpm rs:lint       # cargo clippy --workspace --all-targets -- -D warnings
pnpm check         # svelte-check

node tools/gen-fixtures.mjs   # regenerate golden vectors — see rule 6 first
```

`pnpm verify` is the gate. Nothing is "done" until it passes.

The fixture generator needs **Node 25+** (native TypeScript type stripping) and
reads `TS_ROOT`, defaulting to `~/Code/bettor-calculator-main/src/lib/math`.

---

## Layout

```
crates/bettor-core/     pure math — 18 modules, no Tauri, no io, no entropy
  src/                  odds, probability, hold, devig, distributions,
                        wager, arbitrage, parlay, clv, risk_of_ruin,
                        match_model, line, regression, middle, teaser,
                        correlation, bayesian
  tests/parity.rs       golden-vector replay against the TypeScript
  tests/fixtures/       803 committed reference vectors
src-tauri/              shell: windowing, 32 IPC commands, later SQLite
  src/commands.rs       thin adapters — deserialize, call core, serialize
  src/lib.rs            command registry, fresh_seed(), bindings export test
src/                    SvelteKit — adapter-static, SSR off, Svelte 5 runes
  lib/bindings.ts       GENERATED. Do not edit.
tools/gen-fixtures.mjs  runs the reference TS, dumps golden vectors
tasks/todo.md           the plan and the per-phase record
docs/                   divergences, testing strategy
```

---

## Conventions that matter

- **Everything is a 0–1 fraction inside the core.** The TypeScript mixed scales
  freely — EV in dollars, `evPercent` on 0–100, edge in percentage points — and
  formatted at the point of computation. Formatting is the frontend's job.
- **Struct fields are `#[serde(rename_all = "camelCase")]`.** Rust stays snake,
  TypeScript stays camel, nobody translates by hand.
- **`Result<T, E>` is not expressible in a specta struct field.** Where a row
  can individually fail, use paired `Option<T>` + `Option<MathError>`.
- **Name legs for the direction that wins them**, not for which team holds them.
  `high_side` / `low_side` in `middle.rs` is why the sign bug cannot recur.
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

That is the actual argument for the Rust port, and it is why the type-level
work (newtypes, named sides, `MathError`, generated bindings) is not ceremony.
When you add a module, ask where its seams are before you check its formulas.

---

## Working style for this repo

- Plan before non-trivial work; write the plan into `tasks/todo.md` and update
  it with results, decisions, and bugs found as each phase closes.
- New feature or fix → **code review and refactor** → commit. Not straight to
  commit.
- Never mark something complete without proving it: tests run, output shown.
- Search only paths the user names. Do not sweep the filesystem guessing at
  filenames.
