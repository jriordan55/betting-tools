# bettor-desktop

Sports betting calculators and simulations. A Tauri v2 desktop app with a Rust
math engine.

The point of the project is one specific demonstration: **how the range of odds
you bet drives your breakeven and your variance — even when every bet is +EV and
every bet gets closing line value.** You can lose a lot of bets and sit through
long drawdowns with a genuine edge, purely because of the mix of prices you took.
A +400 winner that closed +350 is not the same result as a -110 winner that
closed -130, and the usual way of reporting CLV actively hides that.

---

## Status

| Phase | | |
|---|---|---|
| 0 | Scaffold | ✅ |
| 1 | Core math, tier 1 — odds, probability, hold, devig | ✅ |
| 1.5 | Math extracted from the React components | ✅ |
| 2 | Core math, tier 2 — distributions, models, simulation | ✅ |
| 3 | Typed IPC — 32 commands, generated bindings | ✅ |
| 4 | Design system + four calculators end-to-end | ⏭ next |
| 5 | Odds-range / variance module | |
| 6 | SQLite bet log | |
| 7 | Probability visualizer | |
| 8 | Mass port of the remaining 17 calculators | |

**The math engine is done and tested. The UI is not built yet.** `src/routes/`
currently holds a single smoke-test page that proves the typed IPC boundary
works end to end. Phase 4 is the architecture gate — the patterns settled there
are what the other 17 calculators get built against.

Current test surface:

```
190  unit tests           crates/bettor-core
 10  parity suites        803 golden vectors replayed against the TypeScript
  1  bindings export      regenerates src/lib/bindings.ts
     svelte-check         180 files, 0 errors, 0 warnings
     clippy -D warnings   clean
```

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

---

## Architecture

```
bettor-desktop/
├─ crates/bettor-core/     pure math — no Tauri, no io, no entropy
│  ├─ src/                 18 modules
│  └─ tests/
│     ├─ parity.rs         golden-vector replay
│     └─ fixtures/         803 committed reference vectors
├─ src-tauri/              shell: windowing, IPC, later SQLite
├─ src/                    SvelteKit (adapter-static, SSR off, Svelte 5)
│  └─ lib/bindings.ts      GENERATED from the Rust — never hand-edited
├─ tools/gen-fixtures.mjs  runs the reference TS, dumps golden vectors
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

## Development

Requires Rust (stable), Node 22+, and pnpm. Regenerating fixtures additionally
requires **Node 25+** for native TypeScript type stripping.

```bash
pnpm install
pnpm tauri dev          # run the app
pnpm verify             # clippy -D warnings → cargo test → svelte-check
```

`pnpm verify` is the gate; nothing lands without it. Individual pieces:

```bash
pnpm rs:test            # cargo test --workspace
pnpm rs:lint            # cargo clippy --workspace --all-targets -- -D warnings
pnpm rs:fmt             # cargo fmt --all
pnpm check              # svelte-check
cargo test -p bettor-core   # math only, no Tauri toolchain needed
```

To regenerate golden vectors — read [`docs/TESTING.md`](docs/TESTING.md) first,
because doing this to make a failing test pass defeats the entire verification
strategy:

```bash
node tools/gen-fixtures.mjs
TS_ROOT=/path/to/bettor-calculator-main/src/lib/math node tools/gen-fixtures.mjs
```

---

## Stack

Tauri v2 · Rust 2021 · SvelteKit (adapter-static, SSR off) · Svelte 5 runes ·
TypeScript strict · D3 · KaTeX · rusqlite (Phase 6) · pnpm

`tauri-specta` for Tauri v2 exists only as a release candidate
(`2.0.0-rc.21`, with `specta` `2.0.0-rc.22`). Both are pinned with `=`. It is the
standard Tauri v2 solution, but the type-safety backbone does rest on an RC.

---

## Documentation

- [`CLAUDE.md`](CLAUDE.md) — working rules for the repo, in short form
- [`tasks/todo.md`](tasks/todo.md) — the plan, and the record of every phase
- [`docs/DIVERGENCES.md`](docs/DIVERGENCES.md) — every bug found in the
  reference TypeScript and every deliberate departure from it
- [`docs/TESTING.md`](docs/TESTING.md) — the parity harness, and how to work
  with golden vectors without defeating them

## License

MIT
