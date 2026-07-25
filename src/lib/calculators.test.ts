import { describe, expect, it } from 'vitest';
import { CALCULATORS, CATEGORY_LABELS, CATEGORY_ORDER, byCategory, getCalculator } from './calculators';

/**
 * The registry is the routing table: a slug typo is a dead link and a missing
 * category is a whole group that never renders. None of that is a type error,
 * so it is checked here.
 */

describe('the registry', () => {
	it('is not empty', () => {
		// Several assertions below are vacuously true over an empty array, so
		// this one stops a broken registry from reading as a clean suite.
		expect(CALCULATORS.length).toBeGreaterThan(20);
	});

	it('has a unique slug per calculator', () => {
		const slugs = CALCULATORS.map((c) => c.slug);
		expect(new Set(slugs).size).toBe(slugs.length);
	});

	it('uses url-safe slugs', () => {
		for (const calc of CALCULATORS) {
			expect(calc.slug, calc.title).toMatch(/^[a-z0-9]+(-[a-z0-9]+)*$/);
		}
	});

	it('gives every calculator a title, a description and an icon', () => {
		for (const calc of CALCULATORS) {
			expect(calc.title.trim(), calc.slug).not.toBe('');
			expect(calc.description.trim().length, calc.slug).toBeGreaterThan(20);
			expect(calc.icon.trim(), calc.slug).not.toBe('');
		}
	});

	it('puts every calculator in a category that has a label and an order', () => {
		for (const calc of CALCULATORS) {
			expect(CATEGORY_LABELS[calc.category], calc.slug).toBeTruthy();
			expect(CATEGORY_ORDER, calc.slug).toContain(calc.category);
		}
	});

	it('labels and orders exactly the same set of categories', () => {
		expect([...CATEGORY_ORDER].sort()).toEqual(Object.keys(CATEGORY_LABELS).sort());
	});

	it('finds a calculator by slug and nothing by a wrong one', () => {
		expect(getCalculator('odds-converter')?.title).toBe('Odds Converter');
		expect(getCalculator('not-a-calculator')).toBeUndefined();
	});
});

describe('grouping', () => {
	it('accounts for every calculator exactly once', () => {
		const grouped = byCategory().flatMap((g) => g.items);
		expect(grouped).toHaveLength(CALCULATORS.length);
		expect(new Set(grouped.map((c) => c.slug)).size).toBe(CALCULATORS.length);
	});

	it('returns groups in the declared order and drops empty ones', () => {
		const groups = byCategory();
		const expected = CATEGORY_ORDER.filter((c) => CALCULATORS.some((x) => x.category === c));
		expect(groups.map((g) => g.category)).toEqual(expected);
		expect(groups.every((g) => g.items.length > 0)).toBe(true);
	});
});

describe('deliberate omissions', () => {
	/**
	 * Parlay correlation was cut on purpose — one scalar rho cannot describe a
	 * real same-game parlay, and fitting it from an SGP price is one equation
	 * in two unknowns. The reference repo left the component on disk with no
	 * note and the port re-added it once already. See tasks/todo.md.
	 */
	it('does not offer a parlay correlation calculator', () => {
		expect(getCalculator('parlay-correlation')).toBeUndefined();
		expect(CALCULATORS.some((c) => /correlat/i.test(c.title))).toBe(false);
	});
});

describe('lazy loading', () => {
	it('loads every registered component', async () => {
		// A `load` pointing at a moved or renamed file fails at click time, in
		// production, on exactly one route. Importing them all here turns that
		// into a test failure.
		for (const calc of CALCULATORS) {
			const module = await calc.load();
			expect(module.default, calc.slug).toBeTruthy();
		}
	}, 60_000);
});
