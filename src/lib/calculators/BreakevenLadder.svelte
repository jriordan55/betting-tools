<script lang="ts">
	import { onMount } from 'svelte';
	import { commands, type LadderRung, type MathError } from '$lib/bindings';
	import { Async, numeric } from '$lib/async.svelte';
	import { betLog } from '$lib/betlog-store.svelte';
	import { american, count, num, pct } from '$lib/format';
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
		ErrorNote,
		BetLogNote
	} from '$lib/ui';

	const STEP_OPTIONS = [
		{ value: '25', label: '25¢' },
		{ value: '50', label: '50¢' },
		{ value: '100', label: '$1' }
	];

	let fromPrice = $state('-400');
	let toPrice = $state('600');
	let step = $state('50');
	let edgePct = $state('5');
	let fromBetLog = $state(false);

	onMount(async () => {
		await betLog.ensureLoaded();
		const snapshot = betLog.snapshot;
		if (!snapshot || snapshot.summary.settled === 0) return;
		fromPrice = String(Math.round(snapshot.priceMin));
		toPrice = String(Math.round(snapshot.priceMax));
		edgePct = (snapshot.summary.roi * 100).toFixed(2);
		fromBetLog = true;
	});

	let error = $state<MathError | string | null>(null);

	const ladder = new Async(
		() => ({
			from: numeric(fromPrice),
			to: numeric(toPrice),
			step: numeric(step),
			// A typed percentage into the 0–1 fraction the core works in.
			edge: numeric(edgePct)
		}),
		async (d): Promise<LadderRung[] | null> => {
			error = null;
			if (d.from === null || d.to === null || d.step === null || d.edge === null) {
				return null;
			}

			const prices = await commands.priceLadder(d.from, d.to, d.step);
			if (prices.status === 'error') {
				error = prices.error;
				return null;
			}

			const rungs = await commands.breakevenLadder(prices.data, d.edge / 100);
			if (rungs.status === 'error') {
				error = rungs.error;
				return null;
			}
			return rungs.data;
		}
	);

	/**
	 * Rungs are evenly spaced in cents, so plotting against the American number
	 * would bunch the favorites and stretch the dogs. Decimal price is the axis
	 * on which the curve is actually a curve.
	 */
	const rows = $derived(ladder.current ?? []);

	const detectionSeries = $derived([
		{
			label: 'Bets to confirm',
			data: rows
				.filter((r) => r.betsToDetect !== null)
				.map((r) => ({ x: r.decimal, y: r.betsToDetect as number })),
			color: 'var(--accent-amber)'
		}
	]);

	const winRateSeries = $derived([
		{
			label: 'Break even',
			data: rows.map((r) => ({ x: r.decimal, y: r.breakeven })),
			color: 'var(--text-secondary)',
			dashed: true
		},
		{
			label: 'Needed for the edge',
			data: rows.map((r) => ({ x: r.decimal, y: r.requiredWinRate })),
			color: 'var(--accent-cyan)'
		}
	]);
</script>

{#if fromBetLog}
	<BetLogNote
		message="Price range and edge prefilled from your bet log's realised ROI across your price buckets."
	/>
{/if}

<InputCard title="Price range">
	<FormRow>
		<FormGroup label="From" hint="The shortest price on the ladder">
			<input type="text" inputmode="numeric" bind:value={fromPrice} placeholder="-400" />
		</FormGroup>
		<FormGroup label="To" hint="The longest">
			<input type="text" inputmode="numeric" bind:value={toPrice} placeholder="600" />
		</FormGroup>
		<FormGroup label="Target Edge (%)" hint="EV per dollar risked, not a win-rate surplus">
			<input type="text" inputmode="decimal" bind:value={edgePct} placeholder="5" />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Step">
			<ToggleGroup options={STEP_OPTIONS} bind:value={step} />
		</FormGroup>
	</FormRow>
</InputCard>

<OutputSection title="What each price asks of you">
	<ErrorNote {error} />
	{#if rows.length > 0}
		<LineChart
			series={winRateSeries}
			height={280}
			formatX={(v) => num(v, 2)}
			formatY={(v) => pct(v, 0)}
			labelX="Decimal price"
		/>

		{#if detectionSeries[0].data.length > 0}
			<div class="chart-head">Bets before the edge clears two standard errors</div>
			<LineChart
				series={detectionSeries}
				height={240}
				formatX={(v) => num(v, 2)}
				formatY={(v) => count(Math.round(v))}
				labelX="Decimal price"
			/>
		{/if}

		<div class="table-wrap">
			<table>
				<thead>
					<tr>
						<th>Price</th>
						<th>Break even</th>
						<th>Needed</th>
						<th>Cushion</th>
						<th>SD / unit</th>
						<th>Bets to confirm</th>
					</tr>
				</thead>
				<tbody>
					{#each rows as rung (rung.american)}
						<tr>
							<td class="price">{american(rung.american)}</td>
							<td>{pct(rung.breakeven)}</td>
							<td class="lead">{pct(rung.requiredWinRate)}</td>
							<td>{pct(rung.cushion)}</td>
							<td>{num(rung.sdPerUnit, 3)}</td>
							<td class="warn">
								{rung.betsToDetect === null ? '—' : count(Math.round(rung.betsToDetect))}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{:else if !error}
		<EmptyState icon="=" message="Set a price range to build the ladder" />
	{/if}
</OutputSection>

{#if rows.length > 1}
	{@const short = rows[0]}
	{@const long = rows[rows.length - 1]}
	<OutputSection title="The two ends of your range">
		<ResultRow
			label="Cushion at {american(short.american)}"
			value={pct(short.cushion)}
			hint="Win-rate points above break even that this edge requires"
		/>
		<ResultRow
			label="Cushion at {american(long.american)}"
			value={pct(long.cushion)}
			color="blue"
			hint="Smaller — which is why a longshot edge sounds modest and proves brutal"
		/>
		<ResultRow
			label="Standard deviation, {american(short.american)} → {american(long.american)}"
			value="{num(short.sdPerUnit, 2)} → {num(long.sdPerUnit, 2)}"
			color="amber"
		/>
		{#if short.betsToDetect !== null && long.betsToDetect !== null}
			<ResultRow
				label="Sample needed to confirm the same edge"
				value="{count(Math.round(short.betsToDetect))} → {count(Math.round(long.betsToDetect))} bets"
				color="negative"
				hint="Identical expectation, {num(long.betsToDetect / short.betsToDetect, 1)}× the evidence"
			/>
		{/if}
	</OutputSection>
{/if}

<InfoSection title="Why the cushion shrinks and the sample grows">
	<p>
		<strong>Break even is just the implied probability.</strong> At -110 you need 52.38% to stand
		still; at +600 you need 14.29%. Adding an edge on top asks for fewer extra win-rate points at the
		long price — a 5% edge is 2.6 points of cushion at -110 and 1.0 at +400 — because each win pays
		so much more. That makes the longshot sound like the easier target.
	</p>
	<p>
		<strong>It is the harder one.</strong> The variance of a one-unit bet at the break-even rate is
		exactly the amount you stand to win, so the standard deviation rises with the price: 0.95 at
		-110 against 2.04 at +400 once a 5% edge is added, and the column above is measured at the rate
		each price actually requires. Edge grows linearly with the number of bets while noise grows with
		its square root, so the sample needed to tell one from the other rises with the <em>square</em> of
		the standard deviation. Same 5% edge, four and a half times the evidence.
	</p>
	<p>
		<strong>Two standard errors is a choice, not a law.</strong> It is the conventional threshold for
		"probably not luck", and the column is there to be compared across prices rather than treated as
		a finish line. Nothing about hitting it makes an edge real; missing it just means the record so
		far cannot tell you either way.
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
		color: var(--text-primary);
		font-weight: 600;
	}

	td.lead {
		color: var(--accent-cyan);
	}

	td.warn {
		color: var(--accent-amber);
	}
</style>
