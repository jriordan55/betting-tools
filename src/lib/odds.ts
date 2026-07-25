import { commands, type MathError, type OddsFormat } from './bindings';

/**
 * Odds helpers that cross the IPC boundary.
 *
 * Every conversion here is a `commands.*` call. Nothing in this file does
 * arithmetic on a price — `1 / decimal` is a math operation and math lives in
 * `bettor-core`.
 */

export interface ParsedOdds {
	decimal: number | null;
	error: MathError | null;
}

/** Parse one odds string in the given format. */
export async function parseOdds(value: string, format: OddsFormat): Promise<ParsedOdds> {
	if (value.trim() === '') {
		return { decimal: null, error: null };
	}
	const result = await commands.toDecimal(value, format);
	return result.status === 'ok'
		? { decimal: result.data, error: null }
		: { decimal: null, error: result.error };
}

/**
 * Parse several odds strings, failing on the first bad one.
 *
 * Deliberately all-or-nothing: `ParlayCalculator` in the web app dropped
 * unparseable legs and priced the parlay from the survivors, so a typo in one
 * leg returned a shorter parlay's price presented as yours.
 */
export async function parseAll(
	values: string[],
	format: OddsFormat
): Promise<{ decimals: number[] | null; error: MathError | null; badIndex: number | null }> {
	const decimals: number[] = [];
	for (let i = 0; i < values.length; i++) {
		const { decimal, error } = await parseOdds(values[i], format);
		if (error) return { decimals: null, error, badIndex: i };
		if (decimal === null) return { decimals: null, error: null, badIndex: i };
		decimals.push(decimal);
	}
	return { decimals, error: null, badIndex: null };
}

export interface FairOdds {
	american: number;
	decimal: number | null;
}

/**
 * The fair price a probability implies, in both formats.
 *
 * Both conversions happen in Rust. The American figure is rounded to the
 * integer scale books quote on; the decimal one is not, because a fair price
 * is usually being compared against a book to four decimal places.
 */
export async function fairOdds(prob: number): Promise<FairOdds> {
	const [american, decimal] = await Promise.all([
		commands.impliedToAmerican(prob),
		commands.impliedToDecimal(prob)
	]);
	return { american, decimal: decimal.status === 'ok' ? decimal.data : null };
}

/** American odds for a decimal price, for display beside a fair probability. */
export async function americanFor(decimal: number): Promise<number | null> {
	const view = await commands.fromDecimal(decimal);
	return view.status === 'ok' ? view.data.american : null;
}

/** American odds for a 0–1 probability. */
export async function americanForProb(prob: number): Promise<number> {
	return commands.impliedToAmerican(prob);
}

export const ODDS_FORMAT_OPTIONS: { value: OddsFormat; label: string }[] = [
	{ value: 'american', label: 'American' },
	{ value: 'decimal', label: 'Decimal' },
	{ value: 'fractional', label: 'Fractional' }
];

/** Books quote American or decimal; fractional is a display format. */
export const SIMPLE_FORMAT_OPTIONS: { value: OddsFormat; label: string }[] = [
	{ value: 'american', label: 'American' },
	{ value: 'decimal', label: 'Decimal' }
];

export const ODDS_PLACEHOLDER: Record<OddsFormat, string> = {
	american: '-110',
	decimal: '1.91',
	fractional: '10/11'
};
