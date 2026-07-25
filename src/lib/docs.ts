import { renderMarkdown } from './markdown';

/**
 * The reference library: fifteen explainers, bundled rather than fetched.
 *
 * These were the web app's blog posts. They ship inside the binary because a
 * desktop app that needs a network connection to explain its own output is not
 * a desktop app — and because the explanations are the part of the original
 * project worth keeping when the SEO value it was written for does not travel.
 *
 * Some of them describe behaviour this port deliberately changed. Those carry
 * a correction; see `CORRECTIONS` below and `docs/DIVERGENCES.md`.
 */

export interface Doc {
	slug: string;
	title: string;
	date: string;
	excerpt: string;
	tags: string[];
	/** The markdown body, frontmatter stripped. */
	body: string;
	/** A note shown above the article where the port changed the behaviour. */
	correction: string | null;
	/** Calculator slugs this article explains. */
	related: string[];
}

/**
 * Where an article describes the *old* behaviour.
 *
 * The port found nineteen-plus bugs in the reference implementation, and these
 * documents were written against it. Publishing them unchanged next to
 * calculators that now answer differently would leave the app arguing with
 * itself, and the article would win — prose reads as more authoritative than a
 * number. Each note says what changed rather than editing the author's text.
 */
const CORRECTIONS: Record<string, string> = {
	'devig-methods-compared':
		'Two corrections, in opposite directions. (1) The method descriptions are right and the implementation behind this article was not: it had q/S inside the radical where Shin (1993) has q²/S, so its bisection had no interior root and Shin silently returned Proportional\'s numbers — five methods, two of them the same one. Fixed here; on a -1000/+500 market the longshot\'s fair probability moves from 0.1550 to 0.1288. (2) The comparison table cannot be right either, whoever computed it. On a two-outcome market Shin and Equal Margin are provably identical — the insider fraction cancels out of the difference, leaving exactly the equal-margin adjustment — so those two columns must agree on every row, and in the table they do not. Run the numbers in the Devig Calculator instead.',
	'measuring-your-edge':
		'This article is right and the calculator it was written for was not. It leads with odds-based CLV — closing probability minus bet probability, in probability points — which is the honest measure. The web calculator implemented the other one: a ratio of decimal odds, displayed three times under three different labels ("Closing Line Value", "Edge (cents per dollar)", "Expected Value") that were algebraically the same expression. That ratio flatters longshots, ranking +400→+350 (11.1%) above -110→-130 (7.9%) when in points those are 2.22 and 4.14 — the opposite order. The calculator now leads with what this article always said to use. One thing the article does not mention: a closing price still contains the vig, so "beating the close by 2% means roughly 2% expected value" is optimistic by about half the hold unless you devig the close first.',
	'teaser-ev-analysis':
		'The cover probabilities here come from a normal distribution, which cannot see that football margins pile up on 3 and 7. Roughly 15% of NFL games end with a margin of exactly 3 and about 9% with exactly 7; a normal at σ≈13.9 puts under 3% on each. The teaser calculator reports keyNumbersCrossed beside a probability that ignores them and now says so explicitly. The Game Probability Visualizer has an empirical margin reweighting if you want to see the difference.',
	'alternate-line-pricing':
		'The method here is correct and specifies removing the vig before inverting the CDF — "extract the implied fair probability (after removing vig)". The implementation did not do that step. It fed the raw implied probability straight into Φ⁻¹, so a team at -10.5 priced -110 in a -110/-110 market, whose fair cover probability is 0.50 and whose true line is therefore exactly -10.5, came back as -11.33. Eight tenths of a point of pure hold, presented as market information. It cancels when you compare two books at identical prices, which is presumably why it survived. Fixed here, so the ladder now matches the article.',
	'bankroll-management-beyond-kelly':
		'Nothing here is wrong — the article is qualitative, and its ruin-risk table is a rule of thumb rather than simulator output. Two things about the tool behind it did change. The old simulator called Math.random(), so it gave a different answer on every run and no figure it printed could be checked; every simulation here takes a seed and reports it back, so any number you quote can be reproduced. And it showed a median over surviving paths beside a mean over all paths with ruined runs scored as zero — two different populations, side by side, unlabelled. Both bases are now named.'
};

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
			correction: CORRECTIONS[slug] ?? null,
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
