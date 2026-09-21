# betting-tools

**Open the app from anywhere:**  
https://jriordan55.github.io/betting-tools/

That page downloads the Windows installer for the Rust desktop app. Your bet
book is already loaded — add new bets one at a time in the Bet Log.

Direct installer (latest release):  
https://github.com/jriordan55/betting-tools/releases/latest

---

Sports betting calculators and simulations. A Tauri v2 desktop app with a Rust
math engine — 26 calculators, a bet log, and a bundled reference library, all
offline.

![Bettor Desktop](site/assets/screenshot.png)

> [!IMPORTANT]
> **Early development.** This repository is being built in public. Interfaces,
> behavior, and the database schema may change without notice.

## Follow or run the project

On [`jriordan55/betting-tools`](https://github.com/jriordan55/betting-tools), the
Rust desktop app is the main product. Use the Pages link above, or run
**Actions → Release desktop app** to rebuild the Windows installer.

Or build from source:

```bash
git clone https://github.com/jriordan55/betting-tools.git
cd betting-tools
pnpm install
pnpm tauri build
```

## Use it in a browser (optional)

The same Rust engine also runs as a [Streamlit](https://streamlit.io) app. Upload
a Pikkit `transactions.csv` on **Your book**; the file is kept under `data/`
(gitignored) and reloads the next time you open the app.

Locally:

```bash
pip install -r requirements.txt
streamlit run streamlit_app.py
```

On Streamlit Community Cloud, point a new app at this repository with main file
`streamlit_app.py`. The math is `web/bettor.wasm` (a `wasm32-wasip1` build of
`bettor-cli`). Rebuild that file after engine changes:

```bash
cargo build -p bettor-cli --release --target wasm32-wasip1 --no-default-features
```

Copy `target/wasm32-wasip1/release/bettor-cli.wasm` to `web/bettor.wasm`.

This requires the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/)
for your operating system, Rust stable, Node 22+, and pnpm. See
[Build from source](#build-from-source) for verification and bundle commands.

The point of the project is one specific demonstration: **how the range of odds
you bet drives your breakeven and your variance — even when every bet is +EV and
every bet gets closing line value.** You can lose a lot of bets and sit through
long drawdowns with a genuine edge, purely because of the mix of prices you took.
A +400 winner that closed +350 is not the same result as a -110 winner that
closed -130, and the usual way of reporting CLV actively hides that.

Two numbers make the case. At a 5% edge:

| Price | SD per unit | Bets before the edge clears 2σ |
|---|---|---|
| -110 | 0.95 | ~1,440 |
| +400 | 2.04 | ~6,640 |

Same expectation. Four and a half times the evidence.

---

## Project status

The calculation engine, desktop shell, bet log, visualizer, and reference
library are taking shape, but this is still an early build-in-public project.
Expect incomplete packaging, breaking changes, and active iteration.

```
305  Rust tests          280 core · 15 shell · 10 parity suites
 67  frontend tests      formatting, error wording, registry, markdown
803  golden vectors      replayed against the original TypeScript
     svelte-check        0 errors, 0 warnings
     clippy -D warnings  clean
```

`pnpm verify` runs all of it plus a production build. Pull requests run the same
gate in CI.

---

## What is in it

**26 calculators.** Odds conversion, hold, devig (five methods), EV, Kelly,
arbitrage, hedging, parlays, CLV, middles, teasers, alternate lines, line
shopping, Bayesian updating, regression to the mean, prop simulation, Poisson
and negative-binomial match models, risk of ruin.

**An odds-range and variance module**, which is the original idea:

- **Breakeven ladder** — required win rate across a whole price range, and the
  sample size each price needs before its edge can be told from noise
- **CLV translator** — what a cents move is actually worth in probability
  points. The same twenty cents is worth twenty times more at -110 than at +900
- **Bet mix builder** — blended breakeven, per-season standard deviation, and
  which price buckets supply the *swing* rather than the profit
- **Season simulator** — a Monte Carlo equity-curve fan. A real 3% edge over 200
  bets ends the year down 32% of the time at -110 and 45% at +600

**A bet log.** SQLite, WAL, local. Records the price you took, the close, and
the *opposing* close — because without both sides the vig cannot be removed and
any edge computed from a closing price is overstated by roughly half the hold.
It reports realised profit, expected profit against the devigged close, and the
gap between them. That gap is normally the largest number on the page, and
treating it as skill in either direction is the commonest way a betting record
is misread.

**A game probability visualizer.** A book posts a spread and a total; their
standard deviations differ, and the pair pins down two things nobody posts — a
per-team standard deviation and the correlation between the two teams' scores.
Football comes out at ρ ≈ −0.27 (game script pulls the scores apart), basketball
at +0.31 (shared pace). Everything grades on integers, so a push at a
whole-number line is a real event with a real probability.

**A reference library.** Fifteen explainers bundled into the binary, rendered
with KaTeX. No network.

## Privacy, data, and responsible use

Bettor Desktop has no accounts, telemetry, or required network connection. Bet
log records stay in the operating system's per-user application-data directory
under the bundle identifier `com.bettorcalculator.desktop`. Back up that
directory before upgrading an alpha build or moving to another machine. Removing
the app does not necessarily remove its data.

The calculators are educational analysis tools, not betting or financial advice.
Outputs are estimates built from the inputs and assumptions you provide; they
cannot guarantee an edge or a profit. If gambling is no longer recreational,
stop and use the support resources available in your country.

---

## Why Rust

The port is not a rewrite for its own sake. Replaying the original TypeScript
against the new implementation surfaced **nineteen-plus bugs**, and almost none
of them were formula errors. They were seams — a scale, a sign, or a convention
that changed meaning across a function boundary:

- **`devigShin` was not Shin.** `q/S` inside the radical where Shin (1993) has
  `q²/S`. The bisection had no interior root, so the method silently returned the
  same numbers as MPTO — the app offered five devig methods and two were
  duplicates. On a -1000/+500 market the longshot's fair probability moves
  0.1550 → 0.1288.
- **`middle.ts` averaged a quantity with its own negation** on spreads. A
  symmetric -5.5/-110 middle reported the favorite covering **32.4%** when the
  answer is 50%. Verified by running the original, not derived on paper.
- **`impliedTrueLine` never removed the vig** — 0.83 points of pure hold
  reported as market information, and `altline.ts` built its entire ladder on it.
- **The CLV calculator showed one number under three labels**, and measured CLV
  as a ratio of decimal odds — which ranks +400→+350 (11.1%) *above* -110→-130
  (7.9%), when in probability points it is 2.22 vs 4.14, the opposite order.
  That is precisely the misconception this project exists to correct.

The full catalog, with the reproduction for each, is in
[`docs/DIVERGENCES.md`](docs/DIVERGENCES.md).

Two other things Rust bought that the TypeScript could not have:

- **Reproducible simulations.** The original called `Math.random()`, so its Monte
  Carlo was neither reproducible nor testable. Every simulation here takes a seed
  and reports the one it used, so any number a user quotes can be reproduced
  exactly.
- **A closed form where the original simulated.** Correlated parlays with
  non-negative equicorrelation factor through a single latent variable and become
  one smooth integral. The TypeScript drew 50,000 samples for a figure carrying
  ~0.2% sampling noise and returned a different answer every run.

### What Rust did *not* buy

Speed, mostly. `pnpm bench` runs the same workloads through both:

| Workload | Rust | TypeScript | |
|---|---|---|---|
| `normal_cdf` | 7.4 ns | 8.9 ns | a tie |
| `american_to_decimal` | 28.3 ns | 19.0 ns | **0.7×** |
| `devig_all` (5 methods) | 2.39 µs | 6.06 µs | 2.5× |
| `score_matrix` 16×16 | 1.01 µs | 12.3 µs | **12.2×** |
| `risk_of_ruin` 1k × 500 | 1.12 ms | 6.93 ms | 6.2× |

Scalar work is a wash or slightly worse — most of the `american_to_decimal` gap
is the range check that stopped `toDecimal("-1.5", "american")` returning 67.67.
Array and matrix work is where it shows. The Monte Carlo figure is parallelism
across eleven cores, not language: per-core it is *slower*, because ChaCha8 is
slower than xorshift128+ and a reproducible seed was worth the cycles.

The justification was always the bugs.

---

## Architecture

```
bettor-desktop/
├─ crates/bettor-core/     pure math — 21 modules, no Tauri, no io, no entropy
│  ├─ src/
│  ├─ examples/bench.rs    the Rust half of `pnpm bench`
│  └─ tests/
│     ├─ parity.rs         golden-vector replay
│     └─ fixtures/         803 committed reference vectors
├─ src-tauri/              shell: windowing, 53 IPC commands, SQLite bet log
│  ├─ src/commands.rs      thin adapters — deserialize, call core, serialize
│  └─ src/betlog.rs        rusqlite, WAL, migrations
├─ src/                    SvelteKit (adapter-static, SSR off, Svelte 5)
│  ├─ lib/bindings.ts      GENERATED from the Rust — never hand-edited
│  ├─ lib/calculators/     26 calculators
│  └─ content/docs/        15 bundled reference articles
├─ site/                   the GitHub Pages marketing page
├─ tools/                  fixture generator, TypeScript benchmark
├─ tasks/todo.md           the plan and per-phase record
└─ docs/
```

Three properties hold this together:

**The core knows nothing about the shell.** `bettor-core` has no Tauri
dependency and no io. It compiles and tests standalone with
`cargo test -p bettor-core`, which keeps the math testable without spinning up a
webview. It gained an *optional* `specta` feature for type export; specta is not
Tauri, and the default build does not include it.

**Types cross the IPC boundary automatically.** `tauri-specta` generates
`src/lib/bindings.ts` from the Rust command signatures, so TypeScript never
re-declares a shape the Rust already knows. Drift is a compile error rather than
a runtime `undefined`. The export runs as a **test**, not a build script — so
`pnpm verify` refreshes it and a forgotten regeneration shows up as a dirty
working tree.

**Correctness is a test result, not a claim.** `tools/gen-fixtures.mjs` runs the
original TypeScript across a wide input sweep and dumps golden vectors to JSON.
The Rust asserts against those files. Where the port deliberately differs, the
divergence is *declared* — and the harness fails both on an undeclared mismatch
and on a stale declaration, so the list cannot rot.

---

## Build from source

Install the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/)
for your operating system, Rust stable, Node 22+, and pnpm. Regenerating
fixtures additionally requires **Node 25+** for native TypeScript type stripping.

```bash
git clone https://github.com/WalrusQuant/bettor-desktop.git
cd bettor-desktop
pnpm install
pnpm tauri dev          # run the app
pnpm verify             # the gate — see below
pnpm tauri build        # produce a bundle
```

Release bundles are written below `target/release/bundle/`. A development run
is not an installed application and may use debug-speed simulations.

`pnpm verify` is clippy → cargo test → svelte-check → vitest → production build.
CI runs this gate on pull requests and `main`. Individual pieces:

```bash
pnpm rs:test            # cargo test --workspace
pnpm rs:lint            # cargo clippy --workspace --all-targets -- -D warnings
pnpm rs:fmt             # cargo fmt --all
pnpm check              # svelte-check
pnpm test               # vitest
pnpm bench              # Rust vs TypeScript, same workloads
cargo test -p bettor-core   # math only, no Tauri toolchain needed
```

To regenerate golden vectors — read [`docs/TESTING.md`](docs/TESTING.md) first,
because doing this to make a failing test pass defeats the entire verification
strategy:

```bash
node tools/gen-fixtures.mjs
TS_ROOT=/path/to/bettor-calculator-main/src/lib/math node tools/gen-fixtures.mjs
```

The app icon is drawn from a committed SVG (`src-tauri/icons/icon.svg`);
`pnpm tauri icon <1024.png>` regenerates every size from a rasterised copy.

---

## Stack

Tauri v2 · Rust 2021 · SvelteKit (adapter-static, SSR off) · Svelte 5 runes ·
TypeScript strict · D3 · KaTeX · marked · rusqlite (bundled, WAL) · vitest · pnpm

`tauri-specta` for Tauri v2 exists only as a release candidate
(`2.0.0-rc.21`, with `specta` `2.0.0-rc.22`). Both are pinned with `=`. It is the
standard Tauri v2 solution, but the type-safety backbone does rest on an RC.

---

## Documentation

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — development workflow and pull requests
- [`SECURITY.md`](SECURITY.md) — how to report a vulnerability privately
- [`docs/DIVERGENCES.md`](docs/DIVERGENCES.md) — every bug found in the
  reference TypeScript and every deliberate departure from it
- [`docs/TESTING.md`](docs/TESTING.md) — the parity harness, and how to work
  with golden vectors without defeating them

## License

[MIT](LICENSE)
