<script lang="ts">
	import {
		commands,
		type BetType,
		type Ladder,
		type LineSport,
		type MathError,
		type TotalSide
	} from '$lib/bindings';
	import { Async, numeric, positive } from '$lib/async.svelte';
	import { sportConfig } from '$lib/config';
	import { american, line as fmtLine, pct } from '$lib/format';
	import {
		InputCard,
		OutputSection,
		FormRow,
		FormGroup,
		ToggleGroup,
		ResultLarge,
		EmptyState,
		InfoSection,
		ErrorNote
	} from '$lib/ui';

	const SIDE_OPTIONS: { value: TotalSide; label: string }[] = [
		{ value: 'over', label: 'Over' },
		{ value: 'under', label: 'Under' }
	];

	/** How far either side of the main line is worth pricing, by sport. */
	const DEFAULT_RANGE: Record<string, { spread: number; total: number }> = {
		nba: { spread: 7, total: 10 },
		nfl: { spread: 10, total: 10 },
		ncaab: { spread: 7, total: 10 },
		ncaaf: { spread: 10, total: 10 },
		nhl: { spread: 3, total: 3 },
		mlb: { spread: 3, total: 3 },
		soccer: { spread: 2, total: 2 }
	};

	let sports = $state<LineSport[]>([]);
	let sportKey = $state('nba');
	let market = $state<'spread' | 'total'>('spread');
	let side = $state<TotalSide>('over');

	let mainLine = $state('');
	let mainOdds = $state('');
	let opposingOdds = $state('');
	let rangeInput = $state('');
	let stepInput = $state('');

	$effect(() => {
		sportConfig().then((config) => {
			sports = config.lines;
		});
	});

	const sport = $derived(sports.find((s) => s.key === sportKey) ?? null);
	const marketOptions = $derived(
		(sport?.markets ?? ['spread', 'total']).map((m) => ({
			value: m,
			label: m === 'spread' ? 'Spread' : 'Total'
		}))
	);
	const stdDev = $derived(sport ? (market === 'spread' ? sport.spreadStd : sport.totalStd) : null);
	const betType = $derived<BetType>(market === 'spread' ? 'spread' : { total: side });
	const defaultRange = $derived(DEFAULT_RANGE[sportKey]?.[market] ?? 7);
	const typeLabel = $derived(market === 'spread' ? 'Spread' : 'Total');

	$effect(() => {
		if (sport && !sport.markets.includes(market)) market = sport.markets[0];
	});

	const result = new Async<
		{
			mainLine: string;
			mainOdds: string;
			opposingOdds: string;
			rangeInput: string;
			stepInput: string;
			stdDev: number | null;
			betType: BetType;
			defaultRange: number;
		},
		{ data: Ladder | null; error: MathError | string | null }
	>(
		() => ({ mainLine, mainOdds, opposingOdds, rangeInput, stepInput, stdDev, betType, defaultRange }),
		async ({ mainLine, mainOdds, opposingOdds, rangeInput, stepInput, stdDev, betType, defaultRange }) => {
			if (stdDev === null) return { data: null, error: null };

			const line = numeric(mainLine);
			const price = numeric(mainOdds);
			if (line === null || price === null) return { data: null, error: null };

			const opposing = numeric(opposingOdds);
			if (opposing === null) {
				return {
					data: null,
					error:
						'The opposing price is required. Without it the vig stays in, and every rung of the ladder is shifted by the same amount.'
				};
			}

			const range = rangeInput.trim() === '' ? defaultRange : positive(rangeInput);
			const step = stepInput.trim() === '' ? 0.5 : positive(stepInput);
			if (range === null || step === null) {
				return { data: null, error: 'Range and step must both be greater than 0.' };
			}

			const fair = await commands.fairCoverProb(price, opposing);
			if (fair.status === 'error') return { data: null, error: fair.error };

			const ladder = await commands.generateLadder(line, fair.data, stdDev, betType, range, step);
			return ladder.status === 'ok'
				? { data: ladder.data, error: null }
				: { data: null, error: ladder.error };
		}
	);

	const mainValue = $derived(numeric(mainLine));

	/**
	 * Which rungs are easier to hit than the main line.
	 *
	 * A comparison of two numbers the core produced, not a computation on them.
	 */
	function rungClass(rung: number): string {
		if (mainValue === null) return '';
		if (Math.abs(rung - mainValue) < 1e-9) return 'main';
		if (market === 'spread') return rung > mainValue ? 'easier' : 'harder';
		if (side === 'over') return rung < mainValue ? 'easier' : 'harder';
		return rung > mainValue ? 'easier' : 'harder';
	}
</script>

<InputCard title="Settings">
	<FormRow>
		<FormGroup label="Sport" hint={stdDev !== null ? `σ = ${stdDev}` : undefined}>
			<select bind:value={sportKey}>
				{#each sports as s (s.key)}
					<option value={s.key}>{s.label}</option>
				{/each}
			</select>
		</FormGroup>
		<FormGroup label="Market">
			<ToggleGroup options={marketOptions} bind:value={market} />
		</FormGroup>
	</FormRow>
	{#if market === 'total'}
		<FormRow>
			<FormGroup label="Side">
				<ToggleGroup options={SIDE_OPTIONS} bind:value={side} />
			</FormGroup>
		</FormRow>
	{/if}
</InputCard>

<InputCard title="Main Line">
	<FormRow min={150}>
		<FormGroup label={typeLabel}>
			<input
				type="text"
				bind:value={mainLine}
				placeholder={market === 'spread' ? '-7.5' : '220.5'}
			/>
		</FormGroup>
		<FormGroup label="Your Price" hint="American">
			<input type="text" bind:value={mainOdds} placeholder="-110" />
		</FormGroup>
		<FormGroup label="Opposing Price" hint="American — needed to devig">
			<input type="text" bind:value={opposingOdds} placeholder="-110" />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Range" hint="± from the main line (default {defaultRange})">
			<input type="text" bind:value={rangeInput} placeholder={String(defaultRange)} />
		</FormGroup>
		<FormGroup label="Step" hint="Increment (default 0.5)">
			<input type="text" bind:value={stepInput} placeholder="0.5" />
		</FormGroup>
	</FormRow>
</InputCard>

<OutputSection title="Alternate Line Ladder">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const ladder = result.current.data}
		<ResultLarge
			value={fmtLine(ladder.trueLine, 2)}
			label="Implied true {typeLabel.toLowerCase()}"
			color="highlight"
		/>

		{#if ladder.omitted > 0}
			<p class="omitted">
				{ladder.omitted}
				{ladder.omitted === 1 ? 'rung was' : 'rungs were'} omitted — the fair probability there is too
				close to 0 or 1 to quote a price.
			</p>
		{/if}

		<div class="table-wrap">
			<table>
				<thead>
					<tr>
						<th>Line</th>
						<th>Fair Probability</th>
						<th>Fair Odds</th>
					</tr>
				</thead>
				<tbody>
					{#each ladder.rows as row (row.line)}
						<tr class={rungClass(row.line)}>
							<td class="line-cell">{fmtLine(row.line)}</td>
							<td>{pct(row.fairProb, 1)}</td>
							<td class="odds-cell">{american(row.fairOdds)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<p class="legend">
			<span class="swatch easier"></span> easier to hit than your line ·
			<span class="swatch harder"></span> harder · the highlighted row is your line
		</p>
	{:else if !result.current?.error}
		<EmptyState icon="AL" message="Enter a main line with both sides of the market" />
	{/if}
</OutputSection>

<InfoSection title="About Alternate Line Pricing">
	<p>
		One line and one price imply a whole ladder. Back out the true line by inverting the normal CDF
		at the fair cover probability, then read the fair probability off that same normal at every
		other number. Sport-specific σ carries the scoring variance.
	</p>
	<p>
		The row at your own line comes back at the <em>fair</em> price, not the price you entered — the
		vig has been taken out. Enter -110/-110 and that rung reads -100, which is the round-trip check:
		a symmetric market is a coin flip once the hold is removed.
	</p>
	<p>
		<strong>The vig is removed first.</strong> The web version's ladder inherited
		<code>impliedTrueLine</code>'s missing devig, so every rung on it was shifted by the book's hold —
		about eight tenths of a point at -110.
	</p>
</InfoSection>

<style>
	.omitted {
		font-size: 0.78rem;
		color: var(--accent-amber);
		margin-bottom: 0.75rem;
	}

	.table-wrap {
		overflow-x: auto;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-family: var(--font-mono);
		font-size: 0.82rem;
	}

	th {
		text-align: right;
		font-weight: 600;
		font-size: 0.68rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		padding: 0.4rem 0.6rem;
		border-bottom: 1px solid var(--border);
	}

	td {
		text-align: right;
		padding: 0.35rem 0.6rem;
		border-bottom: 1px solid var(--border);
	}

	tbody tr:last-child td {
		border-bottom: none;
	}

	tr.main {
		background: var(--bg-tertiary);
	}

	tr.main .line-cell {
		color: var(--accent-cyan);
		font-weight: 700;
	}

	tr.easier .line-cell {
		color: var(--accent-green);
	}

	tr.harder .line-cell {
		color: var(--accent-red);
	}

	.odds-cell {
		color: var(--text-secondary);
	}

	.legend {
		margin-top: 0.85rem;
		font-size: 0.72rem;
		color: var(--text-muted);
	}

	.swatch {
		display: inline-block;
		width: 8px;
		height: 8px;
		border-radius: 2px;
		vertical-align: middle;
	}

	.swatch.easier {
		background: var(--accent-green);
	}

	.swatch.harder {
		background: var(--accent-red);
	}
</style>
