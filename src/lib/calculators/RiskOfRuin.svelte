<script lang="ts">
	import { commands, type MathError, type OddsFormat, type RuinResult } from '$lib/bindings';
	import { numeric, positive } from '$lib/async.svelte';
	import { parseOdds, SIMPLE_FORMAT_OPTIONS, ODDS_PLACEHOLDER } from '$lib/odds';
	import { count, money, moneySigned, pct, pctSigned, signColor } from '$lib/format';
	import LineChart from '$lib/charts/LineChart.svelte';
	import {
		InputCard,
		OutputSection,
		FormRow,
		FormGroup,
		ToggleGroup,
		ResultRow,
		ResultLarge,
		EmptyState,
		InfoSection,
		ErrorNote
	} from '$lib/ui';

	const SIM_OPTIONS = [
		{ value: '1000', label: '1,000' },
		{ value: '5000', label: '5,000' },
		{ value: '10000', label: '10,000' },
		{ value: '50000', label: '50,000' }
	];

	let format = $state<OddsFormat>('american');
	let winRate = $state('55');
	let avgOdds = $state('-110');
	let bankroll = $state('1000');
	let betSize = $state('20');
	let numBets = $state('500');
	let numSims = $state('10000');
	let seedInput = $state('');

	let result = $state<RuinResult | null>(null);
	let error = $state<MathError | string | null>(null);
	let running = $state(false);

	async function run() {
		running = true;
		error = null;

		try {
			const rate = numeric(winRate);
			if (rate === null) {
				error = 'Enter a win rate.';
				return;
			}
			// A typed percentage into the 0–1 fraction the core works in.
			const winProb = rate / 100;

			const price = await parseOdds(avgOdds, format);
			if (price.error) {
				error = price.error;
				return;
			}
			if (price.decimal === null) {
				error = 'Enter the average price you bet at.';
				return;
			}

			const bank = positive(bankroll);
			const size = positive(betSize);
			const bets = positive(numBets);
			const sims = positive(numSims);
			if (bank === null || size === null || bets === null || sims === null) {
				error = 'Bankroll, bet size, bet count and simulation count must all be greater than 0.';
				return;
			}

			const response = await commands.simulateRuin(
				{
					winProb,
					decimalOdds: price.decimal,
					betSize: size,
					bankroll: bank,
					numBets: bets,
					numSims: sims
				},
				// Seeds cross the wire as decimal strings — a u64 above 2^53
				// arrives corrupted as a JSON number, which would break the one
				// guarantee this whole module exists to make.
				seedInput.trim() === '' ? null : seedInput.trim()
			);

			if (response.status === 'error') {
				error = response.error;
				result = null;
			} else {
				result = response.data;
				seedInput = response.data.seed;
			}
		} finally {
			running = false;
		}
	}

	function reroll() {
		seedInput = '';
		run();
	}

	const ruinColor = $derived.by(() => {
		const p = result?.ruinProb ?? 0;
		if (p > 0.2) return 'negative' as const;
		if (p > 0.05) return 'amber' as const;
		return 'positive' as const;
	});
</script>

<InputCard title="Settings">
	<FormRow>
		<FormGroup label="Odds Format">
			<ToggleGroup options={SIMPLE_FORMAT_OPTIONS} bind:value={format} />
		</FormGroup>
		<FormGroup label="Simulations">
			<ToggleGroup options={SIM_OPTIONS} bind:value={numSims} />
		</FormGroup>
	</FormRow>
</InputCard>

<InputCard title="Inputs">
	<FormRow>
		<FormGroup label="Win Rate (%)" hint="Your expected strike rate at this price">
			<input type="text" inputmode="decimal" bind:value={winRate} placeholder="55" />
		</FormGroup>
		<FormGroup label="Average Odds" hint="The price you typically bet at">
			<input type="text" bind:value={avgOdds} placeholder={ODDS_PLACEHOLDER[format]} />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Bankroll ($)">
			<input type="text" inputmode="decimal" bind:value={bankroll} placeholder="1000" />
		</FormGroup>
		<FormGroup label="Bet Size ($)" hint="Flat stake per wager">
			<input type="text" inputmode="decimal" bind:value={betSize} placeholder="20" />
		</FormGroup>
		<FormGroup label="Number of Bets" hint="How long a run to simulate">
			<input type="text" inputmode="numeric" bind:value={numBets} placeholder="500" />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Seed" hint="Leave blank for a fresh run; paste one to reproduce it exactly">
			<input type="text" inputmode="numeric" bind:value={seedInput} placeholder="(random)" />
		</FormGroup>
	</FormRow>
	<div class="actions">
		<button type="button" class="run" onclick={run} disabled={running}>
			{running ? 'Simulating…' : 'Run simulation'}
		</button>
		<button type="button" onclick={reroll} disabled={running}>New seed</button>
	</div>
</InputCard>

<OutputSection title="Results">
	<ErrorNote {error} />
	{#if result}
		<ResultLarge value={pct(result.ruinProb)} label="Probability of going broke" color={ruinColor} />

		<ResultRow
			label="Return per bet"
			value={pctSigned(result.edge)}
			color={signColor(result.edge)}
			hint="Expected profit as a fraction of each stake — not probability points, which are a different scale"
		/>
		<ResultRow
			label="EV per bet"
			value={moneySigned(result.evPerBet)}
			color={signColor(result.evPerBet)}
		/>

		<div class="block">
			<div class="block-title">Where you end up</div>
			<ResultRow
				label="Median, all paths"
				value={money(result.medianEndingAll)}
				hint="Ruined runs counted as zero"
			/>
			<ResultRow label="Mean, all paths" value={money(result.meanEndingAll)} />
			<ResultRow
				label="Median, survivors only"
				value={money(result.medianEndingSurvivors)}
				hint="A different population — do not read it against the two above without noticing that"
			/>
		</div>

		<div class="block">
			<div class="block-title">What the ride feels like</div>
			<ResultRow label="Median max drawdown" value={pct(result.medianMaxDrawdown)} color="amber" />
			<ResultRow
				label="Mean max drawdown"
				value={pct(result.meanMaxDrawdown)}
				color="amber"
				hint="Drawdown is skewed, so the mean sits above the typical experience"
			/>
			<ResultRow
				label="Longest losing streak"
				value="{count(result.longestLosingStreak)} bets"
				color="negative"
				hint="The statistic most underestimated at long prices"
			/>
		</div>

		<div class="chart-head">Survival curve</div>
		<LineChart
			data={result.survivalCurve.map((p) => ({ x: p.bet, y: p.survival }))}
			height={280}
			fill
			formatX={(v) => count(Math.round(v))}
			formatY={(v) => pct(v, 0)}
			labelX="Bets placed"
			labelY="Still solvent"
		/>

		<p class="seed">
			Seed <code data-selectable>{result.seed}</code> — paste it back above to reproduce this run
			exactly.
		</p>
	{:else if !error}
		<EmptyState icon="!" message="Set your numbers and run the simulation" />
	{/if}
</OutputSection>

<InfoSection title="Reading these numbers">
	<p>
		Risk of ruin is the chance a bankroll hits zero before the horizon ends, even with a real edge.
		It depends far more on bet size relative to bankroll than most bettors expect: at a 2% edge,
		flat-betting 5% of your roll is a materially different proposition from flat-betting 1%.
	</p>
	<p>
		<strong>Three ending-bankroll figures, each named for its population.</strong> The web app
		showed a median over <em>survivors</em> beside a mean over <em>all paths</em> with ruined runs
		scored as zero — side by side, unlabelled. At a 30% ruin rate the median looks healthy while the
		mean is dragged down and nothing on screen explains the gap.
	</p>
	<p>
		<strong>Every run is reproducible.</strong> The web version called <code>Math.random()</code>,
		so no figure it printed could be checked or quoted. Here the seed is an input and an output: same
		seed, same numbers, always. The longest-losing-streak and median-drawdown rows are new — a
		streak of 14 losers at +150 is entirely ordinary and does not feel that way when it happens.
	</p>
</InfoSection>

<style>
	.actions {
		display: flex;
		gap: 0.5rem;
		margin-top: 1rem;
	}

	.run {
		flex: 1;
		color: var(--accent-cyan);
		font-weight: 600;
	}

	.block {
		margin-top: 1.25rem;
		padding-top: 1.25rem;
		border-top: 1px solid var(--border);
	}

	.block-title,
	.chart-head {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		margin-bottom: 0.75rem;
	}

	.chart-head {
		margin-top: 1.5rem;
	}

	.seed {
		margin-top: 0.75rem;
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.seed code {
		font-family: var(--font-mono);
		color: var(--accent-cyan);
	}
</style>
