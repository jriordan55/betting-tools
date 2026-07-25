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
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = join(HERE, "..", "crates", "bettor-core", "tests", "fixtures");
const TS_ROOT =
  process.env.TS_ROOT ??
  "/Users/adamwickwire/Code/bettor-calculator-main/src/lib/math";

const odds = await import(`${TS_ROOT}/odds.ts`);
const probability = await import(`${TS_ROOT}/probability.ts`);
const hold = await import(`${TS_ROOT}/hold.ts`);
const vigComparison = await import(`${TS_ROOT}/vigComparison.ts`);
const devig = await import(`${TS_ROOT}/devig.ts`);

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

console.log(`\nfixtures written to ${OUT_DIR}`);
