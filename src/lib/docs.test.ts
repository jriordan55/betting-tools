import { describe, expect, it } from 'vitest';
import { getCalculator } from './calculators';
import { DOCS, allTags, getDoc } from './docs';

/**
 * The reference library is fifteen files that ship inside the binary. Nothing
 * about a broken frontmatter key or a cross-link to a renamed calculator is a
 * type error, so it is checked here.
 */

describe('the library', () => {
	it('bundles every article', () => {
		expect(DOCS).toHaveLength(15);
	});

	it('parses frontmatter into a title, a date and an excerpt', () => {
		for (const doc of DOCS) {
			expect(doc.title, doc.slug).not.toBe(doc.slug);
			expect(doc.title.length, doc.slug).toBeGreaterThan(10);
			expect(doc.date, doc.slug).toMatch(/^\d{4}-\d{2}-\d{2}$/);
			expect(doc.excerpt.length, doc.slug).toBeGreaterThan(20);
			expect(doc.tags.length, doc.slug).toBeGreaterThan(0);
		}
	});

	it('strips the frontmatter from the body', () => {
		for (const doc of DOCS) {
			expect(doc.body.trimStart().startsWith('---'), doc.slug).toBe(false);
			expect(doc.body, doc.slug).not.toContain('excerpt:');
		}
	});

	it('has unique slugs and finds them', () => {
		expect(new Set(DOCS.map((d) => d.slug)).size).toBe(DOCS.length);
		expect(getDoc('devig-methods-compared')?.title).toContain('Devig');
		expect(getDoc('nope')).toBeUndefined();
	});

	it('sorts newest first', () => {
		const dates = DOCS.map((d) => d.date);
		expect([...dates].sort().reverse()).toEqual(dates);
	});

	it('collects tags most-used first', () => {
		const tags = allTags();
		expect(tags.length).toBeGreaterThan(5);
		expect(new Set(tags).size).toBe(tags.length);
		for (const doc of DOCS) {
			for (const tag of doc.tags) expect(tags).toContain(tag);
		}
	});
});

describe('cross-links', () => {
	it('points every related link at a calculator that exists', () => {
		// A renamed slug otherwise leaves a dead link at the bottom of an
		// article, which nothing else would catch.
		for (const doc of DOCS) {
			for (const slug of doc.related) {
				expect(getCalculator(slug), `${doc.slug} → ${slug}`).toBeTruthy();
			}
		}
	});

	it('links most articles to at least one calculator', () => {
		const linked = DOCS.filter((d) => d.related.length > 0);
		expect(linked.length).toBeGreaterThanOrEqual(DOCS.length - 1);
	});
});

describe('corrections', () => {
	/**
	 * These articles were written against an implementation this port changed.
	 * Shipping them unannotated would leave the app arguing with itself, and
	 * the prose would win — an article reads as more authoritative than a
	 * number on a screen.
	 */
	it('annotates the articles whose behaviour changed', () => {
		for (const slug of [
			'devig-methods-compared',
			'measuring-your-edge',
			'teaser-ev-analysis',
			'alternate-line-pricing',
			'bankroll-management-beyond-kelly'
		]) {
			const doc = getDoc(slug);
			expect(doc, slug).toBeTruthy();
			expect(doc?.correction, slug).toBeTruthy();
			expect((doc?.correction ?? '').length, slug).toBeGreaterThan(120);
		}
	});

	it('leaves the articles that still describe current behaviour alone', () => {
		expect(getDoc('understanding-betting-odds')?.correction).toBeNull();
		expect(getDoc('kelly-criterion-guide')?.correction).toBeNull();
	});
});
