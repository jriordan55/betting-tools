<script lang="ts">
	import { onMount } from 'svelte';
	import { commands, type MathError, type MixLeg, type SeasonResult } from '$lib/bindings';
	import { numeric, positive } from '$lib/async.svelte';
	import { takeMix } from '$lib/handoff';
	import { count, money, moneySigned, num, pct, pctSigned, signColor } from '$lib/format';
	import FanChart from '$lib/charts/FanChart.svelte';
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
		{ value: '10000', label: '10,000' }
	];

	const MAX_BUCKETS = 6;

	interface Bucket {
		price: string;
		stake: string;
		edge: string;
		count: string;
	}

	let buckets = $state<Bucket[]>([
		{ price: '-110', stake: '100', edge: '2', count: '300' },
		{ price: '600', stake: '100', edge: '5', count: '40' }
	]);

	/**
	 * The bet log can hand a real mix over on the way here. Consumed once on
	 * mount, so a later visit shows the defaults rather than a stale book.
	 */
	let fromBetLog = $state(false);

	onMount(() => {
		const legs = takeMix();
		if (!legs || legs.length === 0) return;
		buckets = legs.map((leg) => ({
			price: String(Math.round(leg.american)),
			stake: leg.stake.toFixed(0),
			edge: (leg.edge * 100).toFixed(2),
			count: String(leg.count)
		}));
		fromBetLog = true;
	});

	let bankroll = $state('10000');
	let numSims = $state('5000');
	let stopAtRuin = $state(false);
	let seedInput = $state('');

	let result = $state<SeasonResult | null>(null);
	let error = $state<MathError | string | null>(null);
	let running = $state(false);

	async function run() {
		running = true;
		error = null;

		try {
			const legs: MixLeg[] = [];
			for (const b of buckets) {
				if (b.price.trim() === '') continue;
				const price = numeric(b.price);
				const stake = positive(b.stake);
				const edge = numeric(b.edge);
				const n = positive(b.count);
				if (price === null || stake === null || edge === null || n === null) {
					error = 'Every bucket needs a price, a stake, an edge and a bet count.';
					return;
				}
				legs.push({ american: price, stake, edge: edge / 100, count: Math.round(n) });
			}
			if (legs.length === 0) {
				error = 'Add at least one price bucket.';
				return;
			}

			const bank = positive(bankroll);
			const sims = positive(numSims);
			if (bank === null || sims === null) {
				error = 'Bankroll and simulation count must both be greater than 0.';
				return;
			}

			const response = await commands.simulateSeason(
				{ legs, bankroll: bank, numSims: Math.round(sims), stopAtRuin },
				// Seeds cross the wire as decimal strings — a u64 above 2^53
				// arrives corrupted as a JSON number, which would silently break
				// the guarantee that this run can be reproduced.
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

	function addBucket() {
		if (buckets.length < MAX_BUCKETS)
			buckets.push({ price: '', stake: '100', edge: '2', count: '50' });
	}

	function removeBucket(index: number) {
		if (buckets.length > 1) buckets.splice(index, 1);
	}

	const losingColor = $derived.by(() => {
		const p = result?.losingSeasonProb ?? 0;
		if (p > 0.4) return 'negative' as const;
		if (p > 0.25) return 'amber' as const;
		return 'positive' as const;
	});

</script>

<InputCard title="The season">
	{#if fromBetLog}
		<p class="imported">
			Loaded from your bet log — each bucket carries the edge its closing lines say you had, not
			the return you actually got.
		</p>
	{/if}
	<div class="grid head">
		<span>Price</span>
		<span>Stake</span>
		<span>Edge %</span>
		<span>Bets</span>
		<span></span>
	</div>
	{#each buckets as b, i (i)}
		<div class="grid">
			<input type="text" inputmode="numeric" bind:value={buckets[i].price} placeholder="-110" />
			<input type="text" inputmode="decimal" bind:value={buckets[i].stake} placeholder="100" />
			<input type="text" inputmode="decimal" bind:value={buckets[i].edge} placeholder="2" />
			<input type="text" inputmode="numeric" bind:value={buckets[i].count} placeholder="300" />
			<button
				type="button"
				class="remove"
				onclick={() => removeBucket(i)}
				disabled={buckets.length === 1}
				aria-label="Remove bucket {i + 1}">×</button
			>
		</div>
	{/each}
	<button type="button" class="add" onclick={addBucket} disabled={buckets.length >= MAX_BUCKETS}>
		Add a price bucket
	</button>
</InputCard>

<InputCard title="Settings">
	<FormRow>
		<FormGroup label="Bankroll ($)">
			<input type="text" inputmode="decimal" bind:value={bankroll} placeholder="10000" />
		</FormGroup>
		<FormGroup label="Seasons" hint="How many independent runs">
			<ToggleGroup options={SIM_OPTIONS} bind:value={numSims} />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Seed" hint="Leave blank for a fresh run; paste one to reproduce it exactly">
			<input type="text" inputmode="numeric" bind:value={seedInput} placeholder="(random)" />
		</FormGroup>
	</FormRow>
	<label class="check">
		<input type="checkbox" bind:checked={stopAtRuin} />
		<span>
			Stop a season when the bankroll cannot cover the next stake
			<em>Off by default: the barrier truncates exactly the paths the fan is about.</em>
		</span>
	</label>
	<div class="actions">
		<button type="button" class="run" onclick={run} disabled={running}>
			{running ? 'Simulating…' : 'Run seasons'}
		</button>
		<button type="button" onclick={reroll} disabled={running}>New seed</button>
	</div>
</InputCard>

<OutputSection title="How the year goes">
	<ErrorNote {error} />
	{#if result}
		<ResultLarge
			value={pct(result.losingSeasonProb)}
			label="Seasons that end below where they started"
			color={losingColor}
		/>

		<ResultRow
			label="Expected profit"
			value={moneySigned(result.mix.ev)}
			color={signColor(result.mix.ev)}
			hint="{count(result.bets)} bets at {pctSigned(result.mix.roi)} ROI — the closed form, not the simulation"
		/>
		<ResultRow
			label="Median season"
			value={money(result.endingMedian)}
			hint="Half of seasons end below this"
		/>
		<ResultRow
			label="Mean season"
			value={money(result.endingMean)}
			hint="Above the median, because the upside tail is longer than the downside"
		/>

		<div class="block">
			<div class="block-title">The spread of outcomes</div>
			<ResultRow label="Bad year (5th percentile)" value={money(result.endingP05)} color="negative" />
			<ResultRow label="Good year (95th percentile)" value={money(result.endingP95)} color="positive" />
			<ResultRow
				label="Median max drawdown"
				value={pct(result.medianMaxDrawdown)}
				color="amber"
				hint="The typical worst peak-to-trough drop within a season"
			/>
			<ResultRow
				label="Bad-year max drawdown (95th)"
				value={pct(result.p95MaxDrawdown)}
				color="negative"
			/>
			<ResultRow
				label="Longest losing streak seen"
				value="{count(result.longestLosingStreak)} bets"
				color="negative"
			/>
			{#if result.ruinProb > 0}
				<ResultRow
					label="Seasons that ran out of bankroll"
					value={pct(result.ruinProb)}
					color="negative"
				/>
			{/if}
		</div>

		<div class="chart-head">Equity curve, every season overlaid</div>
		<FanChart
			data={result.fan}
			start={result.startingBankroll}
			height={340}
			formatX={(v) => count(Math.round(v))}
			formatY={(v) => money(v, 0)}
			labelX="bets in"
		/>

		<p class="seed">
			Seed <code data-selectable>{result.seed}</code> — paste it back above to reproduce this run
			exactly.
		</p>
	{:else if !error}
		<EmptyState icon="^" message="Describe a season and run it a few thousand times" />
	{/if}
</OutputSection>

<InfoSection title="A real edge loses money surprisingly often">
	<p>
		Three hundred bets at -110 with a genuine 2% edge is a good year's work, and it ends below where
		it started in roughly a third of seasons. The same edge at +600 does it closer to half the time.
		Neither figure says the bets were wrong; both say a season is too short a sample to tell you
		much, and that how short depends on the prices you take rather than on the size of your edge.
	</p>
	<p>
		<strong>Each season plays the same bets in its own order.</strong> Order cannot change where the
		bankroll ends up, but it is the whole story for drawdown and for losing streaks — so the fan's
		width comes from which bets win, and its shape from when they do.
	</p>
	<p>
		<strong>Expected profit comes from the closed form, not the simulation.</strong> It is exact, and
		printing it beside the simulated median makes the gap between the two visible: the median season
		sits below the mean because a bankroll's upside is unbounded and its downside is not.
	</p>
	<p>
		<strong>Every run is reproducible.</strong> The seed is both an input and an output, so any
		number here can be reproduced exactly — including by someone you quote it to.
	</p>
</InfoSection>

<style>
	.grid {
		display: grid;
		grid-template-columns: 1.1fr 1fr 0.9fr 0.9fr 2rem;
		gap: 0.5rem;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.grid.head {
		font-size: 0.68rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		margin-bottom: 0.35rem;
	}

	.grid.head span {
		text-align: center;
	}

	.remove {
		padding: 0.35rem 0;
		line-height: 1;
		color: var(--text-muted);
	}

	.remove:hover:not(:disabled) {
		color: var(--accent-red);
		border-color: var(--accent-red);
	}

	.add {
		width: 100%;
		margin-top: 0.5rem;
		color: var(--accent-cyan);
	}

	.imported {
		font-size: 0.75rem;
		color: var(--accent-cyan);
		line-height: 1.6;
		margin-bottom: 0.85rem;
	}

	.check {
		display: flex;
		align-items: flex-start;
		gap: 0.6rem;
		margin-top: 1rem;
		font-size: 0.8rem;
		color: var(--text-secondary);
		cursor: pointer;
	}

	.check input {
		width: auto;
		margin-top: 0.2rem;
	}

	.check em {
		display: block;
		font-style: normal;
		font-size: 0.72rem;
		color: var(--text-muted);
		margin-top: 0.15rem;
	}

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
		margin-top: 1.75rem;
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
