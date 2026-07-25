import { describe, expect, it } from 'vitest';
import { renderMarkdown } from './markdown';

/**
 * The reference articles use `$` for three different things — a math
 * delimiter, a literal dollar inside math (`$\$2100$`), and a currency symbol
 * in prose. Getting that wrong does not throw; it silently eats the rest of a
 * paragraph into an equation, which is the failure these tests exist for.
 */

describe('markdown', () => {
	it('renders ordinary markdown', () => {
		const html = renderMarkdown('## Heading\n\nSome **bold** text.');
		expect(html).toContain('<h2');
		expect(html).toContain('<strong>bold</strong>');
	});

	it('renders GFM tables, which the articles lean on', () => {
		const html = renderMarkdown('| A | B |\n|---|---|\n| 1 | 2 |');
		expect(html).toContain('<table>');
		expect(html).toContain('<td>1</td>');
	});
});

describe('math', () => {
	it('renders display math', () => {
		const html = renderMarkdown('$$\n\\text{CLV} = p_c - p_b\n$$');
		expect(html).toContain('katex');
		expect(html).toContain('katex-display');
	});

	it('renders inline math', () => {
		const html = renderMarkdown('The true line $\\mu$ is what we want.');
		expect(html).toContain('katex');
		expect(html).toContain('is what we want');
	});

	it('protects LaTeX from the markdown parser', () => {
		// `_` is emphasis to markdown and a subscript to LaTeX. If the math is
		// parsed as markdown first, `d_{\text{hedge}}` loses its underscore.
		const html = renderMarkdown('Stake $d_{\\text{hedge}}$ on the other side.');
		expect(html).not.toContain('<em>');
		expect(html).toContain('katex');
	});

	it('leaves a bare dollar amount in prose alone', () => {
		// `$100` is a price. Treating it as an open delimiter would swallow
		// everything up to the next `$` in the document.
		const html = renderMarkdown('A $100 bet at -110 returns $190.91 in total.');
		expect(html).not.toContain('katex');
		expect(html).toContain('$100');
		expect(html).toContain('$190.91');
	});

	it('handles an escaped dollar inside math', () => {
		// Straight out of the hedging article: `$\$2100$` is an equation whose
		// content is a dollar amount.
		const html = renderMarkdown('You would collect $\\$2100$ on the hedge.');
		expect(html).toContain('katex');
		expect(html).toContain('on the hedge');
	});

	it('does not treat a dollar inside code as math', () => {
		const html = renderMarkdown('Use `$100` as the stake, then $x$ elsewhere.');
		expect(html).toContain('<code>$100</code>');
		expect(html).toContain('katex');
	});

	it('does not let an unclosed delimiter run away', () => {
		// A stray `$` must not consume the rest of the document.
		const html = renderMarkdown('Costs $5 to enter.\n\nA separate paragraph entirely.');
		expect(html).toContain('A separate paragraph entirely');
		expect(html).not.toContain('katex');
	});

	it('shows the source rather than blanking on malformed LaTeX', () => {
		const html = renderMarkdown('Broken: $\\frac{1}{$');
		expect(html).toContain('Broken');
	});
});

describe('every bundled article', () => {
	it('renders without throwing and produces real content', async () => {
		const { DOCS, renderDoc } = await import('./docs');
		expect(DOCS.length).toBeGreaterThan(10);

		for (const doc of DOCS) {
			const html = renderDoc(doc);
			expect(html.length, doc.slug).toBeGreaterThan(500);
			// A placeholder that survived to the output is a math span that was
			// lifted out and never put back.
			expect(html, doc.slug).not.toContain('xKaTeXPlaceholderx');
			expect(html, doc.slug).not.toContain('math-error');
		}
	});
});
