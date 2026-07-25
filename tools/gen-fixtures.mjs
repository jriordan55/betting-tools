/**
 * Golden-vector generator.
 *
 * Runs the ORIGINAL TypeScript math from bettor-calculator-main across a wide
 * input sweep and dumps the results to JSON. The Rust port is then asserted
 * against these files, so "the port is correct" becomes a test result rather
 * than a claim.
 *
 * Usage:
 *   node tools/gen-fixtures.mjs
 *   TS_ROOT=/path/to/bettor-calculator-main/src/lib/math node tools/gen-fixtures.mjs
 *
 * The output is COMMITTED. Do not regenerate it to make a failing test pass —
 * a mismatch means either the port is wrong or the divergence is deliberate,
 * and a deliberate divergence gets recorded in EXPECTED_DIVERGENCES below.
 */
import {
  writeFileSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { registerHooks } from "node:module";

// The source modules import siblings without a file extension (`from './odds'`),
// which a bundler resolves and raw Node ESM does not. Rather than take a
// dependency on a TypeScript loader, retry extensionless relative specifiers
// with `.ts` appended. Leaf modules import nothing, which is why the first
// batch of fixtures generated without this.
registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier.startsWith(".") && !/\.[cm]?[jt]s$/.test(specifier)) {
      try {
        return nextResolve(`${specifier}.ts`, context);
      } catch {
        // Fall through to the default resolver for a better error message.
      }
    }
    return nextResolve(specifier, context);
  },
});

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = join(HERE, "..", "crates", "bettor-core", "tests", "fixtures");
const SOURCE_ROOT =
  process.env.TS_ROOT ??
  "/Users/adamwickwire/Code/bettor-calculator-main/src/lib/math";

/**
 * Stages a patched copy of the reference source in a temp directory.
 *
 * `altline.ts` imports `BetType` and `TotalSide` from `./bestline` as ordinary
 * named imports, but both are type aliases. A bundler elides them; native type
 * stripping leaves the import in place and the module fails to instantiate.
 * That is a real latent bug in the reference repo — the file will not load
 * under `verbatimModuleSyntax` or any non-bundler toolchain — but it is not
 * ours to fix, so the copy is patched instead of the original.
 */
function stageSource(root) {
  const staged = mkdtempSync(join(tmpdir(), "bettor-ts-reference-"));
  const patches = [
    {
      file: "altline.ts",
      from: "import { impliedTrueLine, BetType, TotalSide } from './bestline'",
      to: "import { impliedTrueLine } from './bestline'\ntype BetType = 'spread' | 'total'\ntype TotalSide = 'over' | 'under'",
    },
  ];
  for (const name of readdirSync(root)) {
    if (!name.endsWith(".ts")) continue;
    let text = readFileSync(join(root, name), "utf8");
    for (const p of patches) {
      if (p.file === name) {
        if (!text.includes(p.from)) {
          console.warn(`  ! patch for ${name} no longer applies — verify by hand`);
          continue;
        }
        text = text.replace(p.from, p.to);
      }
    }
    writeFileSync(join(staged, name), text);
  }
  return staged;
}

const TS_ROOT = stageSource(SOURCE_ROOT);

const odds = await import(`${TS_ROOT}/odds.ts`);
const probability = await import(`${TS_ROOT}/probability.ts`);
const hold = await import(`${TS_ROOT}/hold.ts`);
const vigComparison = await import(`${TS_ROOT}/vigComparison.ts`);
const devig = await import(`${TS_ROOT}/devig.ts`);
const poisson = await import(`${TS_ROOT}/poisson.ts`);
const nbinom = await import(`${TS_ROOT}/nbinom.ts`);
const bestline = await import(`${TS_ROOT}/bestline.ts`);
const altline = await import(`${TS_ROOT}/altline.ts`);
const regression = await import(`${TS_ROOT}/regression.ts`);

/**
 * JSON cannot represent Infinity or NaN. Encode them as tagged strings; the
 * Rust side decodes the same three spellings back into f64.
 */
function encode(v) {
  if (typeof v === "number") {
    if (Number.isNaN(v)) return "NaN";
    if (v === Infinity) return "Infinity";
    if (v === -Infinity) return "-Infinity";
    return v;
  }
  if (Array.isArray(v)) return v.map(encode);
  if (v && typeof v === "object") {
    return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, encode(x)]));
  }
  return v;
}

function write(name, payload) {
  mkdirSync(OUT_DIR, { recursive: true });
  const body = {
    _generated_by: "tools/gen-fixtures.mjs",
    _source: `${name}.ts (bettor-calculator-main)`,
    _warning: "COMMITTED REFERENCE DATA. Do not regenerate to silence a test.",
    ...encode(payload),
  };
  const path = join(OUT_DIR, `${name}.json`);
  writeFileSync(path, JSON.stringify(body, null, 2) + "\n");
  const n = Object.entries(payload)
    .filter(([, v]) => Array.isArray(v))
    .reduce((a, [, v]) => a + v.length, 0);
  console.log(`${name}.json — ${n} cases`);
}

// ---------------------------------------------------------------- odds.ts

// Valid American prices, plus values that are NOT valid American odds but that
// the TS accepts anyway (0, ±50, ±99) — captured so the Rust's stricter
// behavior is a documented divergence rather than an accident.
const AMERICAN = [
  "-10000", "-1000", "-500", "-300", "-250", "-200", "-150", "-130", "-120",
  "-110", "-105", "-101", "-100", "100", "101", "105", "110", "120", "150",
  "200", "250", "300", "350", "400", "500", "1000", "2500", "10000",
  "0", "-0", "50", "-50", "99", "-99", "1.5", "-1.5",
  "", "abc", "+150", " -110 ", "1e3", "NaN", "Infinity",
];

const DECIMAL_STR = [
  "1", "1.0", "1.01", "1.1", "1.5", "1.909", "2", "2.5", "3", "5", "11", "51",
  "101", "0", "0.5", "-2", "abc", "",
];

const DECIMAL_NUM = [
  1, 1.0001, 1.01, 1.1, 1.25, 1.5, 1.6667, 1.8182, 1.909090909090909, 2, 2.1,
  2.5, 3, 3.5, 4, 5, 6, 8, 11, 21, 51, 101, 0, 0.5, -2,
];

const FRACTIONAL = [
  "1/2", "1/1", "5/1", "2/5", "10/11", "100/30", "85/40", "4/6", "20/1",
  "0/1", "1/0", "abc/2", "2/abc", "3", "", "1/2/3", "-1/2",
];

const PROBS = [
  0, 0.0001, 0.001, 0.005, 0.01, 0.05, 0.0909, 0.1, 0.1667, 0.2, 0.25,
  0.3333333333333333, 0.4, 0.45, 0.4762, 0.5, 0.5238, 0.55, 0.6,
  0.6666666666666666, 0.75, 0.8, 0.9, 0.95, 0.99, 0.999, 0.9999, 1, -0.5, 1.5,
];

write("odds", {
  to_decimal: [
    ...AMERICAN.map((value) => ({
      value,
      format: "american",
      out: odds.toDecimal(value, "american"),
    })),
    ...DECIMAL_STR.map((value) => ({
      value,
      format: "decimal",
      out: odds.toDecimal(value, "decimal"),
    })),
    ...FRACTIONAL.map((value) => ({
      value,
      format: "fractional",
      out: odds.toDecimal(value, "fractional"),
    })),
  ],
  to_american: DECIMAL_NUM.map((decimal) => ({
    decimal,
    out: odds.toAmerican(decimal),
  })),
  from_decimal: DECIMAL_NUM.map((decimal) => ({
    decimal,
    out: odds.fromDecimal(decimal),
  })),
  american_to_implied: AMERICAN.map((value) => ({
    value,
    out: odds.americanToImplied(value),
  })),
  implied_to_american: PROBS.map((prob) => ({
    prob,
    out: odds.impliedToAmerican(prob),
  })),
  decimal_to_implied: DECIMAL_NUM.map((decimal) => ({
    decimal,
    out: odds.decimalToImplied(decimal),
  })),
  odds_to_implied: [
    ...AMERICAN.map((value) => ({
      value,
      format: "american",
      out: odds.oddsToImplied(value, "american"),
    })),
    ...DECIMAL_STR.map((value) => ({
      value,
      format: "decimal",
      out: odds.oddsToImplied(value, "decimal"),
    })),
  ],
});

// --------------------------------------------------------- probability.ts

const ZS = [
  -8, -6, -5, -4, -3, -2.5, -2, -1.96, -1.645, -1, -0.5, -0.25, -0.01, 0, 0.01,
  0.25, 0.5, 1, 1.645, 1.96, 2, 2.5, 3, 4, 5, 6, 8, Infinity, -Infinity, NaN,
];

// Straddles Acklam's branch boundaries at p = 0.02425 and 1 - 0.02425 exactly,
// which is where a transcription error in the coefficient tables would show up.
const INV_PS = [
  0, 1e-12, 1e-10, 1e-6, 0.001, 0.01, 0.02424, 0.02425, 0.02426, 0.05, 0.1,
  0.25, 0.4, 0.5, 0.6, 0.75, 0.9, 0.95, 0.97574, 0.97575, 0.97576, 0.99,
  0.999, 1 - 1e-6, 1 - 1e-10, 1, -0.1, 1.1,
];

const SPREAD_STDS = [13.86, 13.5, 10.5, 6.0, 4.0, 1.0];

write("probability", {
  normal_cdf: ZS.map((z) => ({ z, out: probability.normalCDF(z) })),
  inverse_normal_cdf: INV_PS.map((p) => ({
    p,
    out: probability.inverseNormalCDF(p),
  })),
  prob_to_spread: SPREAD_STDS.flatMap((std) =>
    [0.05, 0.1, 0.25, 0.4, 0.5, 0.6, 0.75, 0.9, 0.95].map((p) => ({
      p,
      std,
      out: probability.probToSpread(p, std),
    })),
  ),
  prob_to_beta: [0.1, 0.25, 0.5, 0.75, 0.9].flatMap((p) =>
    [1, 10, 100, 1000].map((n) => ({ p, n, out: probability.probToBeta(p, n) })),
  ),
});

// ---------------------------------------------------- hold.ts / vigComparison.ts

const IMPLIED_PAIRS = [
  [0.5238, 0.5238], [0.5, 0.5], [0.55, 0.5], [0.6, 0.45], [0.9091, 0.1667],
  [0.4762, 0.4762], [0.5238, 0.4762], [0.48, 0.48], [0.3, 0.3],
  [0.99, 0.99], [0.0001, 0.9999], [1, 1], [0, 0.5], [0.5, 0], [-0.1, 0.5],
];

write("hold", {
  calculate_hold: IMPLIED_PAIRS.map(([a, b]) => ({
    implied_a: a,
    implied_b: b,
    out: hold.calculateHold(a, b),
  })),
});

const BOOK_SETS = [
  [
    { name: "Pinnacle", impliedA: 0.5155, impliedB: 0.4975 },
    { name: "DraftKings", impliedA: 0.5238, impliedB: 0.5238 },
    { name: "FanDuel", impliedA: 0.5263, impliedB: 0.5217 },
  ],
  [
    { name: "A", impliedA: 0.5, impliedB: 0.5 },
    { name: "B", impliedA: 0.52, impliedB: 0.52 },
  ],
  // Includes entries the filter must drop (zero / negative implied).
  [
    { name: "Valid", impliedA: 0.51, impliedB: 0.52 },
    { name: "ZeroA", impliedA: 0, impliedB: 0.5 },
    { name: "NegB", impliedA: 0.5, impliedB: -0.2 },
  ],
  // Ties on vigPct — exercises sort stability.
  [
    { name: "First", impliedA: 0.52, impliedB: 0.52 },
    { name: "Second", impliedA: 0.53, impliedB: 0.51 },
  ],
  [],
];

write("vig_comparison", {
  compare_vig: BOOK_SETS.map((books) => ({
    books: books.map((b) => ({
      name: b.name,
      implied_a: b.impliedA,
      implied_b: b.impliedB,
    })),
    out: vigComparison.compareVig(books).map((r) => ({
      name: r.name,
      implied_a: r.impliedA,
      implied_b: r.impliedB,
      total_implied: r.totalImplied,
      vig_pct: r.vigPct,
      no_vig_prob_a: r.noVigProbA,
      no_vig_prob_b: r.noVigProbB,
    })),
  })),
});

// --------------------------------------------------------------- devig.ts

const DEVIG_SETS = [
  // Standard two-way markets at various holds.
  [0.5238, 0.5238],
  [0.5238, 0.4762],
  [0.55, 0.5],
  [0.6, 0.45],
  [0.5155, 0.4975],
  [0.5, 0.5],
  // Heavy favorite — where devig methods disagree most, and the whole reason
  // there are five of them.
  [0.9091, 0.1667],
  [0.8333, 0.2222],
  [0.9524, 0.0909],
  [0.99, 0.0198],
  // Longshots, the case this project exists to explain.
  [0.2, 0.8333],
  [0.0909, 0.9524],
  // Three-way (soccer 1X2).
  [0.4545, 0.3125, 0.2857],
  [0.35, 0.34, 0.36],
  [0.5, 0.3, 0.25],
  // Degenerate / adversarial.
  [0.48, 0.48],
  [0.3, 0.3],
  [1, 1],
  [0.9999, 0.9999],
  [0.0001, 0.0001],
  [0, 0.5],
  [0.5],
  [1e-9, 1 - 1e-9],
  [0.25, 0.25, 0.25, 0.25],
];

const call = (fn, probs) => {
  try {
    return fn(probs);
  } catch (e) {
    return { _threw: String(e && e.message ? e.message : e) };
  }
};

write("devig", {
  em: DEVIG_SETS.map((probs) => ({ probs, out: call(devig.devigEM, probs) })),
  mpto: DEVIG_SETS.map((probs) => ({ probs, out: call(devig.devigMPTO, probs) })),
  shin: DEVIG_SETS.map((probs) => ({ probs, out: call(devig.devigShin, probs) })),
  or: DEVIG_SETS.map((probs) => ({ probs, out: call(devig.devigOR, probs) })),
  log: DEVIG_SETS.map((probs) => ({ probs, out: call(devig.devigLOG, probs) })),
  calc_devig_ev: [
    [0.5, 0.4762], [0.5, 0.5238], [0.55, 0.5238], [0.21, 0.2],
    [0.5, 0], [0.5, -0.1], [0.5, null], [0.5, 1],
  ].map(([fair, betImplied]) => ({
    fair_prob: fair,
    bet_implied: betImplied,
    out: devig.calcDevigEV(fair, betImplied),
  })),
});

// ------------------------------------------------- poisson.ts / nbinom.ts

// Soccer, baseball, hockey, and a low-scoring outlier.
const RATE_PAIRS = [
  [1.5, 1.2], [1.35, 1.15], [4.5, 4.2], [2.9, 2.7], [0.9, 1.1], [3.0, 3.0],
];
const MAX_SCORE = 10;
const NB_PARAMS = [
  [1.5, 1.2, 5.0, 5.0], [4.5, 4.2, 3.0, 3.0], [2.9, 2.7, 12.0, 8.0],
];

write("match_model", {
  poisson_pmf: [0.5, 1.5, 2.5, 4.5, 12.0, 30.0].flatMap((lambda) =>
    [0, 1, 2, 3, 5, 8, 12, 20, 40].map((k) => ({
      k,
      lambda,
      out: poisson.poissonPMF(k, lambda),
    })),
  ),
  poisson_cdf: [1.5, 4.5, 12.0].flatMap((lambda) =>
    [0, 1, 3, 5, 10, 25].map((k) => ({
      k,
      lambda,
      out: poisson.poissonCDF(k, lambda),
    })),
  ),
  nbinom_pmf: [[1.5, 5.0], [4.5, 3.0], [2.0, 20.0], [3.0, 1.5]].flatMap(
    ([mean, r]) =>
      [0, 1, 2, 4, 7, 12, 25].map((k) => ({
        k,
        mean,
        r,
        out: nbinom.nbinomPMF(k, mean, r),
      })),
  ),
  // Emitted UNNORMALISED, exactly as the TS produced them. The Rust
  // renormalises; the parity test divides by the grid total to isolate that
  // one deliberate difference from the marginal arithmetic underneath.
  score_matrix: RATE_PAIRS.map(([lh, la]) => ({
    lambda_home: lh,
    lambda_away: la,
    max_score: MAX_SCORE,
    out: poisson.buildScoreMatrix(lh, la, MAX_SCORE),
  })),
  nb_score_matrix: NB_PARAMS.map(([mh, ma, rh, ra]) => ({
    mean_home: mh,
    mean_away: ma,
    r_home: rh,
    r_away: ra,
    max_score: MAX_SCORE,
    out: nbinom.buildNBScoreMatrix(mh, ma, rh, ra, MAX_SCORE),
  })),
  markets: RATE_PAIRS.flatMap(([lh, la]) =>
    [true, false].map((allowDraw) => {
      const matrix = poisson.buildScoreMatrix(lh, la, MAX_SCORE);
      return {
        lambda_home: lh,
        lambda_away: la,
        max_score: MAX_SCORE,
        allow_draw: allowDraw,
        spread_lines: [-2.5, -1.5, -1, -0.5, 0, 0.5, 1, 1.5, 2.5],
        total_lines: [1.5, 2, 2.5, 3, 3.5, 7, 8.5],
        out: poisson.deriveMarketProbs(
          matrix,
          [-2.5, -1.5, -1, -0.5, 0, 0.5, 1, 1.5, 2.5],
          [1.5, 2, 2.5, 3, 3.5, 7, 8.5],
          allowDraw,
        ),
      };
    }),
  ),
});

// ------------------------------------------------- bestline.ts / altline.ts

const LINE_CASES = [
  { line: -10.5, odds: "-110", std: 13.86, betType: "spread" },
  { line: -3.5, odds: "-115", std: 13.86, betType: "spread" },
  { line: -7, odds: "+100", std: 13.86, betType: "spread" },
  { line: 6.5, odds: "-130", std: 13.86, betType: "spread" },
  { line: -4.5, odds: "-108", std: 10.5, betType: "spread" },
  { line: 47.5, odds: "-110", std: 10.0, betType: "total", side: "over" },
  { line: 47.5, odds: "-110", std: 10.0, betType: "total", side: "under" },
  { line: 8.5, odds: "-125", std: 4.0, betType: "total", side: "over" },
  { line: 220.5, odds: "-105", std: 16.0, betType: "total", side: "under" },
];

write("line", {
  // NOTE: the TS derives its cover probability from a single price, vig
  // included. The Rust takes a fair probability. Parity is checked by feeding
  // the Rust the same raw figure, which isolates the inversion arithmetic from
  // the (deliberate) decision to devig first.
  implied_true_line: LINE_CASES.map((c) => ({
    ...c,
    cover_prob: odds.americanToImplied(c.odds),
    out: bestline.impliedTrueLine(c.line, c.odds, c.std, c.betType, c.side),
  })),
  fair_prob_at_line: LINE_CASES.flatMap((c) => {
    const trueLine = bestline.impliedTrueLine(c.line, c.odds, c.std, c.betType, c.side);
    return [-6, -3, -1, 0, 1, 3, 6].map((delta) => ({
      true_line: trueLine,
      alt_line: c.line + delta,
      std: c.std,
      betType: c.betType,
      side: c.side,
      out: altline.fairProbAtLine(trueLine, c.line + delta, c.std, c.betType, c.side),
    }));
  }),
  ladder: LINE_CASES.map((c) => ({
    ...c,
    cover_prob: odds.americanToImplied(c.odds),
    range: 5,
    step: 0.5,
    out: altline.generateLineLadder(c.line, c.odds, c.std, c.betType, c.side, 5, 0.5),
  })),
  compare: [
    { a: { line: -3.5, odds: "-110" }, b: { line: -7.5, odds: "-110" }, std: 13.86, betType: "spread" },
    { a: { line: -7, odds: "+100" }, b: { line: -6.5, odds: "-130" }, std: 13.86, betType: "spread" },
    { a: { line: -7, odds: "-110" }, b: { line: -7, odds: "-110" }, std: 13.86, betType: "spread" },
    { a: { line: 44.5, odds: "-110" }, b: { line: 47.5, odds: "-110" }, std: 10.0, betType: "total", side: "over" },
    { a: { line: 44.5, odds: "-110" }, b: { line: 47.5, odds: "-110" }, std: 10.0, betType: "total", side: "under" },
  ].map((c) => ({
    ...c,
    cover_a: odds.americanToImplied(c.a.odds),
    cover_b: odds.americanToImplied(c.b.odds),
    out: bestline.compareBestLine(
      c.a.line, c.a.odds, c.b.line, c.b.odds, c.std, c.betType, c.side,
    ),
  })),
});

// ---------------------------------------------------------- regression.ts

const REG_CASES = [
  { observed: 0.4, baseline: 0.26, n: 40, k: 200 },
  { observed: 0.4, baseline: 0.26, n: 200, k: 200 },
  { observed: 0.4, baseline: 0.26, n: 2000, k: 200 },
  { observed: 0.15, baseline: 0.26, n: 40, k: 200 },
  { observed: 0.6, baseline: 0.45, n: 120, k: 750 },
  { observed: 0.92, baseline: 0.905, n: 900, k: 1500 },
  { observed: 0.0, baseline: 0.26, n: 10, k: 200 },
  { observed: 1.0, baseline: 0.26, n: 10, k: 200 },
  { observed: 0.33, baseline: 0.33, n: 500, k: 200 },
];

write("regression", {
  regression_weight: REG_CASES.map((c) => ({
    sample_size: c.n,
    regression_constant: c.k,
    out: regression.regressionWeight(c.n, c.k),
  })),
  regress_to_mean: REG_CASES.map((c) => ({
    ...c,
    out: regression.regressToMean(c.observed, c.baseline, c.n, c.k),
  })),
  confidence_interval: REG_CASES.map((c) => {
    const [lower, upper] = regression.confidenceInterval(c.observed, c.baseline, c.n, c.k);
    return { ...c, out: { lower, upper } };
  }),
  // The TS accumulated a float step and rounded, so its x-axis has duplicates
  // and gaps. Only the mapping from sample size to estimate is compared.
  convergence_series: [
    { observed: 0.4, baseline: 0.26, k: 200, max: 2000 },
    { observed: 0.4, baseline: 0.26, k: 200, max: 150 },
  ].map((c) => ({
    ...c,
    out: regression.convergenceSeries(c.observed, c.baseline, c.k, c.max),
  })),
});

console.log(`\nfixtures written to ${OUT_DIR}`);
