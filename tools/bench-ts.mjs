/**
 * Times the ORIGINAL TypeScript math, so the Rust port's speed claim is a
 * measurement rather than an assumption.
 *
 * Usage:
 *   node tools/bench-ts.mjs
 *   node tools/bench-ts.mjs --json          # machine-readable, for pnpm bench
 *   TS_ROOT=/path/to/.../lib/math node tools/bench-ts.mjs
 *
 * Prints nanoseconds per operation for a handful of workloads that also exist
 * in `crates/bettor-core/examples/bench.rs`. Both sides run the same shapes and
 * the same iteration counts; see that file for the caveats, which are real.
 *
 * Needs Node 25+ for native TypeScript type stripping, same as the fixture
 * generator.
 */
import { mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { registerHooks } from "node:module";

registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier.startsWith(".") && !/\.[cm]?[jt]s$/.test(specifier)) {
      try {
        return nextResolve(`${specifier}.ts`, context);
      } catch {
        // Fall through for a better error message from the default resolver.
      }
    }
    return nextResolve(specifier, context);
  },
});

const SOURCE_ROOT =
  process.env.TS_ROOT ??
  "/Users/adamwickwire/Code/bettor-calculator-main/src/lib/math";

/** Copies the reference source so the original is never touched. */
function stageSource(root) {
  const staged = mkdtempSync(join(tmpdir(), "bettor-ts-bench-"));
  for (const name of readdirSync(root)) {
    if (!name.endsWith(".ts")) continue;
    writeFileSync(join(staged, name), readFileSync(join(root, name), "utf8"));
  }
  return staged;
}

const TS_ROOT = stageSource(SOURCE_ROOT);
const odds = await import(`${TS_ROOT}/odds.ts`);
const probability = await import(`${TS_ROOT}/probability.ts`);
const devig = await import(`${TS_ROOT}/devig.ts`);
const poisson = await import(`${TS_ROOT}/poisson.ts`);
const riskOfRuin = await import(`${TS_ROOT}/riskOfRuin.ts`);

/**
 * Runs `fn(i)` `iters` times and returns nanoseconds per operation.
 *
 * `fn` takes the iteration index and must use it, so the argument is not a
 * constant. Without that, V8 hoists a pure call straight out of the loop and
 * the "measurement" is loop overhead — which is how a 3 ns `normalCDF` shows
 * up beside a 26 ns string parse and looks like a language difference.
 */
function time(fn, iters) {
  // A warm-up pass, because V8's first few thousand calls run in the
  // interpreter and would otherwise be charged to the algorithm.
  for (let i = 0; i < Math.min(iters, 10_000); i += 1) fn(i);

  const start = process.hrtime.bigint();
  let sink = 0;
  for (let i = 0; i < iters; i += 1) {
    const out = fn(i);
    // Keep the result observable so the optimiser cannot elide the call.
    if (out !== undefined && out !== null) sink += 1;
  }
  const elapsed = process.hrtime.bigint() - start;
  if (sink < 0) console.log("unreachable");
  return Number(elapsed) / iters;
}

/** Inputs cycled through so no call site sees a constant. */
const ZS = [0.7231, -1.4, 0.05, 2.2, -0.33, 1.11, -2.6, 0.9];
const PRICES = ["-110", "150", "-250", "+320", "-105", "900", "-1200", "225"];
const MARKETS = [
  [0.5238, 0.5238],
  [0.62, 0.43],
  [0.3, 0.75],
  [0.51, 0.52],
];
const LAMBDAS = [
  [2.4, 1.8],
  [1.55, 1.2],
  [3.15, 2.85],
  [4.5, 4.2],
];

const CASES = [
  {
    name: "normal_cdf",
    iters: 2_000_000,
    run: (i) => probability.normalCDF(ZS[i % ZS.length]),
  },
  {
    name: "american_to_decimal",
    iters: 2_000_000,
    run: (i) => odds.toDecimal(PRICES[i % PRICES.length], "american"),
  },
  {
    name: "devig_shin (2-way)",
    iters: 200_000,
    run: (i) => devig.devigShin(MARKETS[i % MARKETS.length]),
  },
  {
    name: "devig_or (2-way)",
    iters: 200_000,
    run: (i) => devig.devigOR(MARKETS[i % MARKETS.length]),
  },
  {
    name: "devig_all (5 methods)",
    iters: 50_000,
    run: (i) => devig.runAllDevigMethods(MARKETS[i % MARKETS.length]),
  },
  {
    name: "score_matrix 16x16",
    iters: 20_000,
    run: (i) => {
      const [h, a] = LAMBDAS[i % LAMBDAS.length];
      return poisson.buildScoreMatrix(h, a, 16);
    },
  },
  {
    name: "risk_of_ruin 1k paths x 500 bets",
    iters: 3,
    run: (i) =>
      riskOfRuin.simulateRuin({
        winProb: 0.55 + (i % 3) * 0.001,
        decimalOdds: 1.909_090_909,
        betSize: 100,
        bankroll: 10_000,
        numBets: 500,
        numSims: 1_000,
      }),
  },
];

const results = [];
for (const c of CASES) {
  let nsPerOp;
  try {
    nsPerOp = time(c.run, c.iters);
  } catch (error) {
    results.push({ name: c.name, error: String(error).slice(0, 120) });
    continue;
  }
  results.push({ name: c.name, iters: c.iters, nsPerOp });
}

if (process.argv.includes("--json")) {
  console.log(JSON.stringify(results));
} else {
  console.log("TypeScript (Node " + process.versions.node + ")\n");
  for (const r of results) {
    if (r.error) {
      console.log(`  ${r.name.padEnd(34)} failed: ${r.error}`);
    } else {
      console.log(`  ${r.name.padEnd(34)} ${r.nsPerOp.toFixed(1).padStart(12)} ns/op`);
    }
  }
}
