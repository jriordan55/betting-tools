import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MathError } from './bindings';

/**
 * `parseAll` is the frontend half of a bug the port exists to stop repeating:
 * the web `ParlayCalculator` dropped unparseable legs and priced the parlay
 * from the survivors, so a typo in one leg produced a shorter parlay's price
 * presented as the user's.
 *
 * The conversions themselves are Rust's and are tested there. What is tested
 * here is the only decision this file makes — what happens when one input in a
 * set is bad — so `commands` is mocked rather than reached.
 */

const BAD_ODDS: MathError = {
	kind: 'parseOdds',
	detail: { value: 'xyz', format: 'american' }
};

const toDecimal = vi.fn();

vi.mock('./bindings', () => ({
	commands: {
		toDecimal: (value: string, format: string) => toDecimal(value, format)
	}
}));

const { parseAll, parseOdds, ODDS_PLACEHOLDER, SIMPLE_FORMAT_OPTIONS } = await import('./odds');

beforeEach(() => {
	toDecimal.mockReset();
	toDecimal.mockImplementation(async (value: string) =>
		value === 'xyz'
			? { status: 'error', error: BAD_ODDS }
			: { status: 'ok', data: Number.parseFloat(value) }
	);
});

describe('parseOdds', () => {
	it('treats a blank field as absent rather than as an error', () => {
		// An untouched input is not a mistake, and reporting one on every empty
		// field would make the form shout before it has been filled in.
		return expect(parseOdds('   ', 'american')).resolves.toEqual({
			decimal: null,
			error: null
		});
	});

	it('does not call into Rust for a blank field', async () => {
		await parseOdds('', 'american');
		expect(toDecimal).not.toHaveBeenCalled();
	});

	it('passes the format through unchanged', async () => {
		await parseOdds('1.91', 'decimal');
		expect(toDecimal).toHaveBeenCalledWith('1.91', 'decimal');
	});

	it('surfaces the error rather than a null', async () => {
		const result = await parseOdds('xyz', 'american');
		expect(result.decimal).toBeNull();
		expect(result.error).toEqual(BAD_ODDS);
	});
});

describe('parseAll', () => {
	it('returns every decimal when they all parse', async () => {
		const result = await parseAll(['1.91', '2.5', '3.0'], 'decimal');
		expect(result.decimals).toEqual([1.91, 2.5, 3.0]);
		expect(result.error).toBeNull();
		expect(result.badIndex).toBeNull();
	});

	it('refuses the whole set when one leg is unparseable', async () => {
		const result = await parseAll(['1.91', 'xyz', '3.0'], 'decimal');
		// The bug being prevented: `[1.91, 3.0]` here would be a two-leg parlay
		// quoted as the user's three-leg one.
		expect(result.decimals).toBeNull();
		expect(result.error).toEqual(BAD_ODDS);
		expect(result.badIndex).toBe(1);
	});

	it('refuses the whole set when one leg is merely blank', async () => {
		// No error to report, but still not a priceable set — a half-filled
		// parlay must not quote as a complete one.
		const result = await parseAll(['1.91', '', '3.0'], 'decimal');
		expect(result.decimals).toBeNull();
		expect(result.error).toBeNull();
		expect(result.badIndex).toBe(1);
	});

	it('stops at the first bad leg instead of parsing the rest', async () => {
		await parseAll(['xyz', '2.0', '3.0'], 'decimal');
		expect(toDecimal).toHaveBeenCalledTimes(1);
	});

	it('accepts an empty set without inventing a result', async () => {
		const result = await parseAll([], 'decimal');
		expect(result.decimals).toEqual([]);
		expect(result.error).toBeNull();
	});
});

describe('format metadata', () => {
	it('offers a placeholder for every format', () => {
		expect(ODDS_PLACEHOLDER.american).toBe('-110');
		expect(ODDS_PLACEHOLDER.decimal).toBe('1.91');
		expect(ODDS_PLACEHOLDER.fractional).toBe('10/11');
	});

	it('leaves fractional out of the input toggles', () => {
		// Books quote American or decimal; fractional is a display format, and
		// offering it as an input invites a spread to be typed into it.
		expect(SIMPLE_FORMAT_OPTIONS.map((o) => o.value)).toEqual(['american', 'decimal']);
	});
});
