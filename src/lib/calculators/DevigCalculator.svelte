<script lang="ts">
	import { commands, type DevigRow, type MathError, type OddsFormat } from '$lib/bindings';
	import { Async } from '$lib/async.svelte';
	import { parseOdds, fairOdds, SIMPLE_FORMAT_OPTIONS, type FairOdds } from '$lib/odds';
	import { describeError } from '$lib/errors';
	import { american, pct, pctSigned } from '$lib/format';
	import {
		InputCard,
		OutputSection,
		FormRow,
		FormGroup,
		ToggleGroup,
		EmptyState,
		InfoSection,
		ErrorNote
	} from '$lib/ui';

	const MARKET_OPTIONS = [
		{ value: '2', label: '2-Way' },
		{ value: '3', label: '3-Way' }
	];

	let format = $state<OddsFormat>('american');
	let marketSize = $state('2');
	let marketOdds = $state(['', '', '']);
	let betOdds = $state(['', '', '']);

	const outcomes = $derived(Number.parseInt(marketSize, 10));
	// A 3-way market is listed home / draw / away, so the draw sits in the middle.
	const labels = $derived(outcomes === 2 ? ['Outcome 1', 'Outcome 2'] : ['Outcome 1', 'Draw', 'Outcome 2']);

	interface MethodRow extends DevigRow {
		fair: (FairOdds | null)[];
		ev: (number | null)[];
	}

	interface DevigView {
		totalImplied: number;
		rows: MethodRow[];
	}

	const result = new Async<
		{ marketOdds: string[]; betOdds: string[]; format: OddsFormat; outcomes: number },
		{ data: DevigView | null; error: MathError | string | null }
	>(
		() => ({ marketOdds: [...marketOdds], betOdds: [...betOdds], format, outcomes }),
		async ({ marketOdds, betOdds, format, outcomes }) => {
			const entered = marketOdds.slice(0, outcomes);
			if (entered.some((o) => o.trim() === '')) return { data: null, error: null };

			const implied: number[] = [];
			for (const value of entered) {
				const parsed = await parseOdds(value, format);
				if (parsed.error) return { data: null, error: parsed.error };
				if (parsed.decimal === null) return { data: null, error: null };
				const view = await commands.fromDecimal(parsed.decimal);
				if (view.status === 'error') return { data: null, error: view.error };
				implied.push(view.data.probability);
			}

			// Your own prices, for the EV column. Optional and independent per outcome.
			const betDecimals: (number | null)[] = [];
			for (const value of betOdds.slice(0, outcomes)) {
				const parsed = await parseOdds(value, format);
				betDecimals.push(parsed.decimal);
			}

			// The core reports the overround and, when a book sums below 1.0, says
			// so with an error rather than leaving the UI to notice a number.
			const overround = await commands.marketOverround(implied);
			if (overround.status === 'error') return { data: null, error: overround.error };

			const rows = await commands.devigAll(implied);

			const enriched: MethodRow[] = [];
			for (const row of rows) {
				const fair: (FairOdds | null)[] = [];
				const ev: (number | null)[] = [];

				for (let i = 0; i < outcomes; i++) {
					const prob = row.fairProbs?.[i];
					if (prob === undefined) {
						fair.push(null);
						ev.push(null);
						continue;
					}
					fair.push(await fairOdds(prob));

					const decimal = betDecimals[i];
					if (decimal === null || decimal === undefined) {
						ev.push(null);
					} else {
						const value = await commands.expectedValue(decimal, prob, 1);
						ev.push(value.status === 'ok' ? value.data.evFraction : null);
					}
				}

				enriched.push({ ...row, fair, ev });
			}

			return { data: { totalImplied: overround.data, rows: enriched }, error: null };
		}
	);

	function evColor(ev: number | null) {
		if (ev === null) return 'muted';
		return ev >= 0 ? 'positive' : 'negative';
	}
</script>

<InputCard title="Market Odds">
	<FormRow>
		<FormGroup label="Odds Format">
			<ToggleGroup options={SIMPLE_FORMAT_OPTIONS} bind:value={format} />
		</FormGroup>
		<FormGroup label="Market Type">
			<ToggleGroup options={MARKET_OPTIONS} bind:value={marketSize} />
		</FormGroup>
	</FormRow>
	<FormRow min={150}>
		{#each labels as label, index (index)}
			<FormGroup {label}>
				<input
					type="text"
					bind:value={marketOdds[index]}
					placeholder={format === 'american' ? ['-150', '+260', '+130'][index] : ['1.67', '3.60', '2.30'][index]}
				/>
			</FormGroup>
		{/each}
	</FormRow>
</InputCard>

<InputCard title="Your Bet Odds (optional)">
	<FormRow min={150}>
		{#each labels as label, index (index)}
			<FormGroup label="Bet {label}" hint={index === 0 ? 'Enter to see the EV column' : undefined}>
				<input
					type="text"
					bind:value={betOdds[index]}
					placeholder={format === 'american' ? ['+165', '+280', '-185'][index] : ['1.70', '3.80', '2.40'][index]}
				/>
			</FormGroup>
		{/each}
	</FormRow>
</InputCard>

<OutputSection title="Devig Results">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const view = result.current.data}
		<div class="summary">
			<span><span class="summary-label">Overround</span> {pct(view.totalImplied)}</span>
			<span class="summary-note">everything above 100% is the book's margin</span>
		</div>

		<div class="table-wrap">
			<table>
				<thead>
					<tr>
						<th class="method-col">Method</th>
						{#each labels as label (label)}<th>Fair {label}</th>{/each}
						{#each labels as label (label)}<th>Odds {label}</th>{/each}
						{#each labels as label (label)}<th>EV {label}</th>{/each}
					</tr>
				</thead>
				<tbody>
					{#each view.rows as row (row.method)}
						<tr>
							<td class="method-col">
								<span class="tag">{row.method}</span>
								<span class="method-name">{row.name}</span>
							</td>
							{#if row.fairProbs === null}
								<td class="failed" colspan={outcomes * 3}>
									{row.error ? describeError(row.error) : 'Cannot compute'}
								</td>
							{:else}
								{#each row.fairProbs.slice(0, outcomes) as prob, i (i)}
									<td class="fair">{pct(prob)}</td>
								{/each}
								{#each row.fair as fair, i (i)}
									<td>{fair ? american(fair.american) : '—'}</td>
								{/each}
								{#each row.ev as ev, i (i)}
									<td class={evColor(ev)}>{ev === null ? '—' : pctSigned(ev, 1)}</td>
								{/each}
							{/if}
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<p class="note">
			EM removes equal vig per outcome. MPTO scales proportionally. Shin models informed money. OR
			applies a power adjustment. LOG works in log-odds space.
		</p>
	{:else if !result.current?.error}
		<EmptyState icon="*" message="Enter the market's prices to compare all five devig methods" />
	{/if}
</OutputSection>

<InfoSection title="About the five methods">
	<p>
		Every book builds a margin into its prices. These five methods each strip it differently, and
		they disagree most exactly where it matters — on lopsided markets, where the favorite's fair
		price is the one you are trying to pin down. When they agree, be more confident.
	</p>
	<p>
		<strong>The web app shipped five methods and two of them were the same.</strong>
		<code>devigShin</code> had <code>q/S</code> inside the radical where Shin (1993) has
		<code>q²/S</code>. The bisection had no interior root, so the solver pinned to its search ceiling
		on every input and Shin silently returned MPTO's numbers. On a -1000/+500 market the longshot's
		fair probability moves 0.1550 → 0.1288 once it is fixed — the old output overstated longshot fair
		probabilities by about 20% relative.
	</p>
	<p>
		Two more fixes worth knowing: a book whose prices sum below 1.0 is now rejected rather than
		"devigged" into false confidence, and a method that fails to converge says so instead of
		returning its last iterate.
	</p>
	<p>
		<strong>Shin and Equal Margin agree exactly on a two-way market, and that is not evidence of
		anything.</strong>
		It is an identity. Inverting Shin's formula gives
		<code>q = √(S·π·(z + (1−z)·π))</code>, and for two outcomes the insider fraction
		<code>z</code> cancels out of the difference — leaving <code>q₁ − q₂ = π₁ − π₂</code>, which
		is equal margin. So the two return the same numbers for whatever <code>z</code> balances the
		book, and Shin only says something of its own once there are three or more outcomes. Five
		methods, four distinct answers on the most common market shape. Worth knowing before you read
		two agreeing columns as corroboration.
	</p>
</InfoSection>

<style>
	.summary {
		display: flex;
		gap: 1.5rem;
		padding-bottom: 0.85rem;
		margin-bottom: 0.85rem;
		border-bottom: 1px solid var(--border);
		font-family: var(--font-mono);
		font-size: 0.85rem;
	}

	.summary-note {
		color: var(--text-muted);
		font-size: 0.72rem;
	}

	.summary-label {
		color: var(--text-muted);
		text-transform: uppercase;
		font-size: 0.72rem;
		letter-spacing: 0.06em;
		margin-right: 0.3rem;
	}

	.table-wrap {
		overflow-x: auto;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-family: var(--font-mono);
		font-size: 0.78rem;
	}

	th {
		text-align: right;
		font-weight: 600;
		font-size: 0.68rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		padding: 0.4rem 0.5rem;
		border-bottom: 1px solid var(--border);
		white-space: nowrap;
	}

	td {
		text-align: right;
		padding: 0.45rem 0.5rem;
		border-bottom: 1px solid var(--border);
		white-space: nowrap;
	}

	tbody tr:last-child td {
		border-bottom: none;
	}

	.method-col {
		text-align: left;
	}

	.tag {
		display: inline-block;
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		border-radius: 3px;
		padding: 1px 5px;
		font-size: 0.68rem;
		font-weight: 700;
		color: var(--accent-cyan);
		margin-right: 0.4rem;
	}

	.method-name {
		color: var(--text-secondary);
	}

	.fair {
		color: var(--accent-cyan);
		font-weight: 600;
	}

	.positive {
		color: var(--accent-green);
	}
	.negative {
		color: var(--accent-red);
	}
	.muted {
		color: var(--text-muted);
	}

	.failed {
		text-align: left;
		color: var(--text-muted);
		font-size: 0.75rem;
	}

	.note {
		margin-top: 0.85rem;
		font-size: 0.75rem;
		color: var(--text-muted);
		line-height: 1.5;
	}
</style>
