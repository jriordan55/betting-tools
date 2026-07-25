import { renderMarkdown } from './markdown';

/**
 * The reference library: fifteen explainers, bundled rather than fetched.
 *
 * These were the web app's blog posts. They ship inside the binary because a
 * desktop app that needs a network connection to explain its own output is not
 * a desktop app — and because the explanations are the part of the original
 * project worth keeping when the SEO value it was written for does not travel.
 *
 * Where an article described behaviour this port changed, the article was
 * edited rather than annotated. They are reference material for this app, not
 * a record of a web app nobody is reading — a banner explaining what a
 * different program used to do is archaeology, and it belongs in
 * `docs/DIVERGENCES.md`, which is where it is.
 */

export interface Doc {
	slug: string;
	title: string;
	date: string;
	excerpt: string;
	tags: string[];
	/** The markdown body, frontmatter stripped. */
	body: string;
	/** Calculator slugs this article explains. */
	related: string[];
}

/** Which calculators each article explains, for the cross-links. */
const RELATED: Record<string, string[]> = {
	'understanding-betting-odds': ['odds-converter', 'hold-calculator'],
	'hold-vig-and-arbitrage': ['hold-calculator', 'vig-comparison', 'arbitrage-calculator'],
	'devig-methods-compared': ['devig-calculator', 'sharp-implied'],
	'kelly-criterion-guide': ['kelly-criterion', 'expected-value'],
	'bankroll-management-beyond-kelly': ['risk-of-ruin', 'season-simulator', 'bet-mix'],
	'measuring-your-edge': ['clv-calculator', 'clv-translator', 'breakeven-ladder'],
	'poisson-match-prediction': ['poisson-match', 'game-visualizer'],
	'negative-binomial-match-prediction': ['nbinom-match'],
	'player-prop-simulation': ['prop-simulator'],
	'regression-to-the-mean': ['regression-mean'],
	'bayesian-odds-explained': ['bayesian-calculator'],
	'alternate-line-pricing': ['alt-line-pricer', 'better-line'],
	'better-line-cdf-analysis': ['better-line'],
	'middle-finder-guide': ['middle-finder'],
	'teaser-ev-analysis': ['teaser-ev', 'game-visualizer']
};

/**
 * Reads the frontmatter block a document opens with.
 *
 * A three-key subset of YAML — a string, a date, and a bracketed list — is all
 * these files use, so a parser is cheaper than a dependency. Anything it does
 * not understand is left in the body rather than dropped, which shows up as
 * visible text instead of silently missing content.
 */
function parseFrontmatter(source: string): { data: Record<string, string[] | string>; body: string } {
	const match = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(source);
	if (!match) return { data: {}, body: source };

	const data: Record<string, string[] | string> = {};
	for (const rawLine of match[1].split(/\r?\n/)) {
		const line = rawLine.trim();
		if (line === '' || line.startsWith('#')) continue;

		const split = line.indexOf(':');
		if (split === -1) continue;
		const key = line.slice(0, split).trim();
		const value = line.slice(split + 1).trim();

		if (value.startsWith('[') && value.endsWith(']')) {
			data[key] = value
				.slice(1, -1)
				.split(',')
				.map((item) => item.trim().replace(/^["']|["']$/g, ''))
				.filter((item) => item !== '');
		} else {
			data[key] = value.replace(/^["']|["']$/g, '');
		}
	}
	return { data, body: source.slice(match[0].length) };
}

function asString(value: string[] | string | undefined, fallback = ''): string {
	return typeof value === 'string' ? value : fallback;
}

/**
 * Bundled at build time by Vite, so there is no filesystem read at runtime and
 * no path a slug could escape.
 */
const FILES = import.meta.glob('/src/content/docs/*.md', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

export const DOCS: Doc[] = Object.entries(FILES)
	.map(([path, source]) => {
		const slug = path.split('/').pop()?.replace(/\.md$/, '') ?? path;
		const { data, body } = parseFrontmatter(source);
		return {
			slug,
			title: asString(data.title, slug),
			date: asString(data.date),
			excerpt: asString(data.excerpt),
			tags: Array.isArray(data.tags) ? data.tags : [],
			body,
			related: RELATED[slug] ?? []
		};
	})
	.sort((a, b) => b.date.localeCompare(a.date));

export function getDoc(slug: string): Doc | undefined {
	return DOCS.find((d) => d.slug === slug);
}

/** Every tag in the library, most-used first. */
export function allTags(): string[] {
	const counts = new Map<string, number>();
	for (const doc of DOCS) {
		for (const tag of doc.tags) counts.set(tag, (counts.get(tag) ?? 0) + 1);
	}
	return [...counts.entries()]
		.sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
		.map(([tag]) => tag);
}

/** The rendered HTML for a document body. */
export function renderDoc(doc: Doc): string {
	return renderMarkdown(doc.body);
}
