<script lang="ts">
	import { commands, type ClvRung, type MathError } from '$lib/bindings';
	import { Async, numeric, positive } from '$lib/async.svelte';
	import { american, num, pct, points } from '$lib/format';
	import LineChart from '$lib/charts/LineChart.svelte';
	import {
		InputCard,
		OutputSection,
		FormRow,
		FormGroup,
		ToggleGroup,
		ResultRow,
		EmptyState,
		InfoSection,
		ErrorNote
	} from '$lib/ui';

	const MOVE_OPTIONS = [
		{ value: '10', label: '10¢' },
		{ value: '20', label: '20¢' },
		{ value: '50', label: '50¢' }
	];

	let fromPrice = $state('-400');
	let toPrice = $state('900');
	let move = $state('20');

	let error = $state<MathError | string | null>(null);

	const ladder = new Async(
		() => ({ from: numeric(fromPrice), to: numeric(toPrice), cents: positive(move) }),
		async (d): Promise<ClvRung[] | null> => {
			error = null;
			if (d.from === null || d.to === null || d.cents === null) return null;

			const prices = await commands.priceLadder(d.from, d.to, 25);
			if (prices.status === 'error') {
				error = prices.error;
				return null;
			}

			const rungs = await commands.clvLadder(prices.data, d.cents);
			if (rungs.status === 'error') {
				error = rungs.error;
				return null;
			}
			return rungs.data;
		}
	);

	const rows = $derived(ladder.current ?? []);

	const series = $derived([
		{
			label: 'Probability points',
			data: rows.map((r) => ({ x: r.decimal, y: r.probPoints })),
			color: 'var(--accent-cyan)'
		},
		{
			label: 'Ratio (b/c − 1)',
			data: rows.map((r) => ({ x: r.decimal, y: r.ratio })),
			color: 'var(--accent-amber)',
			dashed: true
		}
	]);

	const distortionSeries = $derived([
		{
			label: 'Ratio ÷ points',
			data: rows.map((r) => ({ x: r.decimal, y: r.distortion })),
			color: 'var(--accent-red)'
		}
	]);
</script>

<InputCard title="The move">
	<FormRow>
		<FormGroup label="From" hint="Shortest price to chart">
			<input type="text" inputmode="numeric" bind:value={fromPrice} placeholder="-400" />
		</FormGroup>
		<FormGroup label="To" hint="Longest">
			<input type="text" inputmode="numeric" bind:value={toPrice} placeholder="900" />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Cents the line moved in your favour">
			<ToggleGroup options={MOVE_OPTIONS} bind:value={move} />
		</FormGroup>
	</FormRow>
</InputCard>

<OutputSection title="What {move}¢ is actually worth">
	<ErrorNote {error} />
	{#if rows.length > 0}
		<LineChart
			{series}
			height={280}
			formatX={(v) => num(v, 2)}
			formatY={(v) => pct(v, 1)}
			labelX="Decimal price"
		/>

		<div class="chart-head">How many times bigger the ratio measure looks</div>
		<LineChart
			series={distortionSeries}
			height={220}
			formatX={(v) => num(v, 2)}
			formatY={(v) => `${num(v, 1)}×`}
			labelX="Decimal price"
		/>

		<div class="table-wrap">
			<table>
				<thead>
					<tr>
						<th>Bet at</th>
						<th>Closed</th>
						<th>Points gained</th>
						<th>Ratio</th>
						<th>Overstated by</th>
					</tr>
				</thead>
				<tbody>
					{#each rows as rung (rung.american)}
						<tr>
							<td class="price">{american(rung.american)}</td>
							<td class="price">{american(rung.closedAmerican)}</td>
							<td class="lead">{points(rung.probPoints)}</td>
							<td>{pct(rung.ratio)}</td>
							<td class="warn">
								{Number.isFinite(rung.distortion) ? `${num(rung.distortion, 1)}×` : '—'}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{:else if !error}
		<EmptyState icon="~" message="Pick a price range and a move" />
	{/if}
</OutputSection>

{#if rows.length > 1}
	{@const short = rows[0]}
	{@const long = rows[rows.length - 1]}
	<OutputSection title="The two ends of your range">
		<ResultRow
			label="{move}¢ at {american(short.american)}"
			value={points(short.probPoints)}
			color="positive"
		/>
		<ResultRow
			label="{move}¢ at {american(long.american)}"
			value={points(long.probPoints)}
			color="negative"
			hint="Identical in cents, worth {num(short.probPoints / long.probPoints, 1)}× less in the currency that compounds"
		/>
		<ResultRow
			label="What the ratio measure reports instead"
			value="{pct(short.ratio)} → {pct(long.ratio)}"
			color="amber"
			hint="A fall of only {num(short.ratio / long.ratio, 1)}×, which is how the longshot keeps looking respectable"
		/>
	</OutputSection>
{/if}

<InfoSection title="Cents are a unit of price, not a unit of value">
	<p>
		A twenty-cent move is twenty cents at every price. What it is <em>worth</em> is not: the same
		move is 4.14 probability points at -110 and 0.20 at +900, a factor of twenty. Probability points
		are what compound a bankroll, so that factor is the real one.
	</p>
	<p>
		<strong>The ratio measure hides most of it.</strong> Dividing the price you took by the price it
		closed at — the figure the web app displayed three times under three names — falls off only
		about fourfold over the same range. That understatement is what makes "I got fifty cents of
		closing line value" on a +600 shot sound better than twenty cents on a -110 favorite, when in
		points it is worth roughly half as much.
	</p>
	<p>
		<strong>The pivot is handled properly.</strong> A line moving from +105 to -115 is an ordinary
		twenty-cent move, but subtracting the two American numbers gives 220. Nothing between -100 and
		+100 exists and the two endpoints are the same price, so every measurement here runs through a
		continuous cents axis rather than through raw subtraction.
	</p>
</InfoSection>

<style>
	.chart-head {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		margin-top: 1.75rem;
		margin-bottom: 0.25rem;
	}

	.table-wrap {
		margin-top: 1.5rem;
		overflow-x: auto;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.8rem;
	}

	th {
		text-align: right;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-size: 0.7rem;
		padding: 0.5rem 0.6rem;
		border-bottom: 1px solid var(--border-strong);
		white-space: nowrap;
	}

	th:first-child {
		text-align: left;
	}

	td {
		text-align: right;
		padding: 0.4rem 0.6rem;
		border-bottom: 1px solid var(--border);
		font-family: var(--font-mono);
		white-space: nowrap;
	}

	td.price {
		text-align: left;
	}

	td.lead {
		color: var(--accent-cyan);
	}

	td.warn {
		color: var(--accent-red);
	}
</style>
