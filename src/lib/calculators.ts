import type { Component } from 'svelte';

export type Category =
	| 'conversion'
	| 'analysis'
	| 'sizing'
	| 'variance'
	| 'comparison'
	| 'simulation';

export interface Calculator {
	slug: string;
	title: string;
	description: string;
	category: Category;
	icon: string;
	/** Loads the calculator's Svelte component on demand. */
	load: () => Promise<{ default: Component }>;
}

export const CATEGORY_LABELS: Record<Category, string> = {
	conversion: 'Conversion',
	analysis: 'Analysis',
	sizing: 'Sizing',
	comparison: 'Comparison',
	variance: 'Odds Range & Variance',
	simulation: 'Simulation'
};

export const CATEGORY_ORDER: Category[] = [
	'conversion',
	'analysis',
	'sizing',
	'variance',
	'comparison',
	'simulation'
];

/**
 * Every calculator, with no free/premium split — this is a desktop app and
 * everything is unlocked (tasks/todo.md, Phase 8).
 */
export const CALCULATORS: Calculator[] = [
	{
		slug: 'odds-converter',
		title: 'Odds Converter',
		description: 'Convert between American, Decimal, and Fractional odds formats instantly',
		category: 'conversion',
		icon: '<->',
		load: () => import('./calculators/OddsConverter.svelte')
	},
	{
		slug: 'hold-calculator',
		title: 'Hold Calculator',
		description:
			"See the book's hold percentage, break-even margins, and no-vig fair odds for any two-sided market",
		category: 'conversion',
		icon: '%',
		load: () => import('./calculators/HoldCalculator.svelte')
	},
	{
		slug: 'parlay-calculator',
		title: 'Parlay Calculator',
		description: 'Calculate potential payouts for 2-10 leg parlay bets',
		category: 'conversion',
		icon: '+',
		load: () => import('./calculators/ParlayCalculator.svelte')
	},
	{
		slug: 'sharp-implied',
		title: 'Sharp Implied',
		description:
			'Derive no-vig probabilities and implied scores from sharp moneyline, spread, and total',
		category: 'analysis',
		icon: '#',
		load: () => import('./calculators/SharpImplied.svelte')
	},
	{
		slug: 'devig-calculator',
		title: 'Devig Calculator',
		description:
			'Remove bookmaker vig with 5 different methods — EM, MPTO, Shin, Odds Ratio, and Logarithmic',
		category: 'analysis',
		icon: '*',
		load: () => import('./calculators/DevigCalculator.svelte')
	},
	{
		slug: 'expected-value',
		title: 'Expected Value (EV)',
		description: 'Determine the expected value of any betting opportunity',
		category: 'analysis',
		icon: 'EV',
		load: () => import('./calculators/ExpectedValue.svelte')
	},
	{
		slug: 'clv-calculator',
		title: 'Closing Line Value',
		description:
			'Measure how much you beat the closing line — the best predictor of long-term profit',
		category: 'analysis',
		icon: 'CLV',
		load: () => import('./calculators/ClvCalculator.svelte')
	},
	{
		slug: 'bayesian-calculator',
		title: 'Bayesian Odds',
		description:
			'Combine market odds with your model using Bayesian inference for posterior probabilities',
		category: 'analysis',
		icon: 'B',
		load: () => import('./calculators/BayesianCalculator.svelte')
	},
	{
		slug: 'regression-mean',
		title: 'Regression to the Mean',
		description: 'Regress small-sample player stats toward true talent using Bayesian shrinkage',
		category: 'analysis',
		icon: 'R',
		load: () => import('./calculators/RegressionMean.svelte')
	},
	{
		slug: 'alt-line-pricer',
		title: 'Alternate Line Pricer',
		description: 'Generate fair odds for every alternate spread and total line from a single market price',
		category: 'analysis',
		icon: 'AL',
		load: () => import('./calculators/AltLinePricer.svelte')
	},
	{
		slug: 'middle-finder',
		title: 'Middle Finder',
		description:
			'Calculate the probability and expected value of landing in the middle of two positions',
		category: 'analysis',
		icon: 'M',
		load: () => import('./calculators/MiddleFinder.svelte')
	},
	{
		slug: 'teaser-ev',
		title: 'Teaser EV',
		description:
			'Calculate cover probabilities, fair odds, and expected value for teasers — compare your teaser to the book',
		category: 'analysis',
		icon: 'T',
		load: () => import('./calculators/TeaserEv.svelte')
	},
	{
		slug: 'kelly-criterion',
		title: 'Kelly Criterion',
		description: 'Calculate optimal bet sizing based on your edge and bankroll',
		category: 'sizing',
		icon: 'K',
		load: () => import('./calculators/KellyCriterion.svelte')
	},
	{
		slug: 'hedge-calculator',
		title: 'Hedge Calculator',
		description: 'Calculate the exact hedge bet needed to guarantee profit or minimize loss',
		category: 'sizing',
		icon: 'H',
		load: () => import('./calculators/HedgeCalculator.svelte')
	},
	{
		slug: 'risk-of-ruin',
		title: 'Risk of Ruin',
		description:
			'Monte Carlo simulation of bankroll survival — see the probability of going broke given your edge and bet sizing',
		category: 'sizing',
		icon: '!',
		load: () => import('./calculators/RiskOfRuin.svelte')
	},
	{
		slug: 'arbitrage-calculator',
		title: 'Arbitrage Calculator',
		description: 'Find profitable arbitrage opportunities across 2-3 sportsbooks',
		category: 'comparison',
		icon: '$',
		load: () => import('./calculators/ArbitrageCalculator.svelte')
	},
	{
		slug: 'vig-comparison',
		title: 'Vig Comparison',
		description:
			'Compare odds from multiple books on the same market to find the lowest vig and sharpest line',
		category: 'comparison',
		icon: 'V',
		load: () => import('./calculators/VigComparison.svelte')
	},
	{
		slug: 'better-line',
		title: 'Better Line',
		description: 'Compare two lines using CDF analysis to find which has less embedded vig',
		category: 'comparison',
		icon: '><',
		load: () => import('./calculators/BetterLine.svelte')
	},
	{
		slug: 'breakeven-ladder',
		title: 'Breakeven Ladder',
		description:
			'Break-even and required win rate across a whole price range, with the sample size each one needs before its edge can be told from noise',
		category: 'variance',
		icon: '=',
		load: () => import('./calculators/BreakevenLadder.svelte')
	},
	{
		slug: 'clv-translator',
		title: 'CLV Translator',
		description:
			'What a cents move is actually worth in probability points, across every price — the same twenty cents is worth twenty times more at -110 than at +900',
		category: 'variance',
		icon: '~>',
		load: () => import('./calculators/ClvTranslator.svelte')
	},
	{
		slug: 'bet-mix',
		title: 'Bet Mix Builder',
		description:
			'Blended break-even, per-season standard deviation, and which price buckets supply the swing rather than the profit',
		category: 'variance',
		icon: '#',
		load: () => import('./calculators/BetMixBuilder.svelte')
	},
	{
		slug: 'season-simulator',
		title: 'Season Simulator',
		description:
			'Monte Carlo equity-curve fan for a whole season of your bet mix — how often a real edge still ends the year down',
		category: 'variance',
		icon: '^',
		load: () => import('./calculators/SeasonSimulator.svelte')
	},
	{
		slug: 'prop-simulator',
		title: 'Player Prop Simulator',
		description:
			'Monte Carlo simulation for player props with sport-specific distributions and fair odds',
		category: 'simulation',
		icon: '~',
		load: () => import('./calculators/PropSimulator.svelte')
	},
	{
		slug: 'game-visualizer',
		title: 'Game Probability Visualizer',
		description:
			'Model a whole football or basketball game from its spread and total — margin and score distributions on integers, key numbers, and the spread-to-probability curve',
		category: 'simulation',
		icon: '/\\',
		load: () => import('./calculators/GameVisualizer.svelte')
	},
	{
		slug: 'poisson-match',
		title: 'Poisson Match Predictor',
		description:
			'Model match outcomes with Poisson distributions — scoreline probabilities, moneyline, spreads, and totals',
		category: 'simulation',
		icon: 'P',
		load: () => import('./calculators/PoissonMatch.svelte')
	},
	{
		slug: 'nbinom-match',
		title: 'Negative Binomial Match Predictor',
		description:
			'Model match outcomes with overdispersed scoring — better for blowout-prone matchups in baseball, hockey, and soccer',
		category: 'simulation',
		icon: 'NB',
		load: () => import('./calculators/NBinomMatch.svelte')
	}
];

export function getCalculator(slug: string): Calculator | undefined {
	return CALCULATORS.find((c) => c.slug === slug);
}

export function byCategory(): { category: Category; items: Calculator[] }[] {
	return CATEGORY_ORDER.map((category) => ({
		category,
		items: CALCULATORS.filter((c) => c.category === category)
	})).filter((group) => group.items.length > 0);
}
