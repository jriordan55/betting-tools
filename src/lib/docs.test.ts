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

describe('content', () => {
	/**
	 * Three articles were edited because they described behaviour this port
	 * changed. The edits are the point — there is no banner explaining what a
	 * different program used to do — so these assert the corrected substance
	 * survived, since prose has no other guard.
	 */
	it('devig article states the Shin / equal-margin identity', () => {
		const body = getDoc('devig-methods-compared')?.body ?? '';
		expect(body).toContain('Same Thing on a Two-Way Market');
		// The old table gave EM and Shin different values, which is impossible.
		expect(body).toContain('| -300/+250 | 103.57% | 73.21% | 72.41% | 73.21%');
	});

	it('CLV article says to devig the close', () => {
		const body = getDoc('measuring-your-edge')?.body ?? '';
		expect(body).toContain('Devig the Close First');
		expect(body).toContain('probability points');
	});

	it('teaser article admits its model ignores key numbers', () => {
		const body = getDoc('teaser-ev-analysis')?.body ?? '';
		expect(body).toContain('What the Model Does Not Do');
		expect(body).toContain('modelIgnoresKeyNumbers');
	});

	it('does not name the web app it was written for', () => {
		for (const doc of DOCS) {
			expect(doc.body, doc.slug).not.toContain('Bettor Calculator');
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
