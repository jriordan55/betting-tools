import { describe, expect, it } from 'vitest';
import type { BetLogError, LogError, MathError } from './bindings';
import { describeBetLogError, describeError, describeLogError, err, ok } from './errors';

/**
 * The error text is the only part of a `MathError` a user ever sees, and the
 * switch that produces it is exhaustive by design — adding a variant in Rust
 * should break the build here rather than fall through to a blank string.
 * These tests check that every variant says something, and that the two
 * failure families stay apart.
 */

const MATH_ERRORS: MathError[] = [
	{ kind: 'parseOdds', detail: { value: '-1.5', format: 'american' } },
	{
		kind: 'probabilityOutOfRange',
		detail: { value: 1.4, reason: 'must be strictly between 0 and 1' }
	},
	{ kind: 'domainError', detail: { param: 'stake', constraint: 'greater than zero', value: -5 } },
	{ kind: 'shapeError', detail: { what: 'parlay legs', expected: 'at least 2', got: 1 } },
	{ kind: 'noConvergence', detail: { solver: 'shin', iterations: 100, residual: 1e-4 } }
];

describe('describeError', () => {
	it('says something specific for every variant', () => {
		for (const error of MATH_ERRORS) {
			const message = describeError(error);
			expect(message.length).toBeGreaterThan(10);
			expect(message).not.toContain('undefined');
			expect(message).not.toContain('[object');
		}
	});

	it('quotes the offending input back', () => {
		expect(describeError(MATH_ERRORS[0])).toContain('-1.5');
		expect(describeError(MATH_ERRORS[0])).toContain('american');
	});

	it('names an empty input rather than quoting nothing', () => {
		// `"" is not valid american odds` reads like a bug in the app.
		const message = describeError({
			kind: 'parseOdds',
			detail: { value: '   ', format: 'american' }
		});
		expect(message).toContain('(empty)');
	});

	it('reports a solver failure with its residual', () => {
		const message = describeError(MATH_ERRORS[4]);
		expect(message).toContain('shin');
		expect(message).toContain('100');
		expect(message).toMatch(/e-4/);
	});
});

describe('describeLogError', () => {
	const LOG_ERRORS: LogError[] = [
		{ kind: 'storage', detail: { message: 'disk I/O error' } },
		{ kind: 'notFound', detail: { id: 42 } },
		{ kind: 'invalid', detail: { field: 'stake', reason: '0 is not an amount of money' } }
	];

	it('says something specific for every variant', () => {
		for (const error of LOG_ERRORS) {
			const message = describeLogError(error);
			expect(message.length).toBeGreaterThan(10);
			expect(message).not.toContain('undefined');
		}
	});

	it('suggests the likely cause of a missing row', () => {
		// The realistic way this happens is a second window, and saying so
		// beats leaving the user to wonder whether the app lost their bet.
		expect(describeLogError(LOG_ERRORS[1])).toContain('another window');
	});
});

describe('describeBetLogError', () => {
	/**
	 * A disk failure and a bad probability are different problems, which is
	 * why the Rust keeps them in separate types. This is the check that the
	 * frontend does not flatten them back together.
	 */
	it('routes each source to its own wording', () => {
		const storage: BetLogError = {
			source: 'storage',
			error: { kind: 'storage', detail: { message: 'database is locked' } }
		};
		const math: BetLogError = { source: 'math', error: MATH_ERRORS[3] };

		expect(describeBetLogError(storage)).toContain('database is locked');
		expect(describeBetLogError(math)).toContain('parlay legs');
		expect(describeBetLogError(storage)).not.toBe(describeBetLogError(math));
	});
});

describe('ok / err', () => {
	it('narrows a success', () => {
		const result = { status: 'ok', data: 42 } as const;
		expect(ok(result)).toBe(42);
		expect(err(result)).toBeNull();
	});

	it('narrows a failure', () => {
		const result = { status: 'error', error: MATH_ERRORS[0] } as const;
		expect(ok(result)).toBeNull();
		expect(err(result)).toEqual(MATH_ERRORS[0]);
	});

	it('does not mistake a zero result for a failure', () => {
		// `ok()` returning null on a legitimate 0 would silently blank an EV of
		// exactly zero, which is a real and meaningful result.
		expect(ok({ status: 'ok', data: 0 } as const)).toBe(0);
	});
});
