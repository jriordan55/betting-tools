import { describe, expect, it } from 'vitest';
import {
	american,
	count,
	line,
	money,
	moneySigned,
	num,
	pct,
	pctSigned,
	points,
	signColor
} from './format';

/**
 * The formatters are the last thing between a correct number and a wrong
 * screen, and they are the only place in the frontend allowed near a number at
 * all. Two things are worth pinning: that they never invent a value, and that
 * a non-finite input renders as an em dash rather than as `NaN%`.
 */

describe('percentages', () => {
	it('scales a 0-1 fraction to a percentage', () => {
		expect(pct(0.5238)).toBe('52.38%');
		expect(pct(0.05, 0)).toBe('5%');
		expect(pct(1)).toBe('100.00%');
	});

	it('signs a percentage only when it is positive', () => {
		expect(pctSigned(0.05)).toBe('+5.00%');
		expect(pctSigned(-0.05)).toBe('-5.00%');
		// Zero takes no sign: "+0.00%" reads as a win that is not there.
		expect(pctSigned(0)).toBe('0.00%');
	});

	it('renders probability points with their unit', () => {
		expect(points(0.0414)).toBe('+4.14 pts');
		expect(points(-0.0222)).toBe('-2.22 pts');
	});
});

describe('money', () => {
	it('formats plain currency', () => {
		expect(money(1234.5)).toBe('$1234.50');
		expect(money(0)).toBe('$0.00');
		expect(money(99.999, 0)).toBe('$100');
	});

	it('puts the sign outside the currency symbol', () => {
		// `-$100.00`, never `$-100.00`.
		expect(moneySigned(-100)).toBe('-$100.00');
		expect(moneySigned(100)).toBe('+$100.00');
		expect(moneySigned(0)).toBe('$0.00');
	});
});

describe('odds and lines', () => {
	it('gives a positive American price its plus', () => {
		expect(american(150)).toBe('+150');
		expect(american(-110)).toBe('-110');
		expect(american(100)).toBe('+100');
	});

	it('rounds to the integer scale books quote on', () => {
		expect(american(149.6)).toBe('+150');
		expect(american(-110.4)).toBe('-110');
	});

	it('signs a line the way a book posts it', () => {
		expect(line(-3.5)).toBe('-3.5');
		expect(line(7)).toBe('+7.0');
		expect(line(48.5, 1)).toBe('+48.5');
	});
});

describe('counts', () => {
	it('separates thousands', () => {
		expect(count(1440)).toBe('1,440');
		expect(count(6640)).toBe('6,640');
		expect(count(7)).toBe('7');
	});
});

describe('non-finite input', () => {
	/**
	 * Several core fields are legitimately NaN — `Clv.distortion` when a line
	 * did not move, `BetMix.tStat` when the mix has no variance. Rendering
	 * those as `NaN%` looks like a crash; an em dash looks like what it is.
	 */
	it('renders as an em dash rather than NaN', () => {
		for (const bad of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY]) {
			expect(pct(bad)).toBe('—');
			expect(pctSigned(bad)).toBe('—');
			expect(points(bad)).toBe('—');
			expect(money(bad)).toBe('—');
			expect(moneySigned(bad)).toBe('—');
			expect(american(bad)).toBe('—');
			expect(num(bad)).toBe('—');
			expect(line(bad)).toBe('—');
			expect(count(bad)).toBe('—');
		}
	});
});

describe('signColor', () => {
	it('distinguishes zero from both signs', () => {
		expect(signColor(1)).toBe('positive');
		expect(signColor(-1)).toBe('negative');
		// Not "positive": a break-even result must not be painted as a win.
		expect(signColor(0)).toBe('default');
	});
});

describe('what the formatters must not do', () => {
	/**
	 * CLAUDE.md rule 1: the frontend formats, it does not calculate. A
	 * formatter that took two arguments and returned a ratio would be math
	 * that had escaped Rust. These assertions are a tripwire on that — each
	 * one is a value passing through unchanged apart from its presentation.
	 */
	it('passes the value through untouched apart from scale and precision', () => {
		expect(pct(0.123456, 4)).toBe('12.3456%');
		expect(num(1 / 3, 6)).toBe('0.333333');
		expect(money(0.005, 2)).toBe('$0.01');
	});
});
