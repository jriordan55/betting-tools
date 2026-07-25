<script lang="ts">
	import {
		commands,
		type BetType,
		type LineComparison,
		type LineSport,
		type MathError,
		type TotalSide
	} from '$lib/bindings';
	import { Async, numeric } from '$lib/async.svelte';
	import { sportConfig } from '$lib/config';
	import { line as fmtLine, num, pct } from '$lib/format';
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

	const SIDE_OPTIONS: { value: TotalSide; label: string }[] = [
		{ value: 'over', label: 'Over' },
		{ value: 'under', label: 'Under' }
	];

	let sports = $state<LineSport[]>([]);
	let sportKey = $state('nba');
	let market = $state<'spread' | 'total'>('spread');
	let side = $state<TotalSide>('over');

	let lineA = $state('');
	let oddsA = $state('');
	let opposingA = $state('');
	let lineB = $state('');
	let oddsB = $state('');
	let opposingB = $state('');

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
	const typeLabel = $derived(
		market === 'spread' ? 'spread' : side === 'over' ? 'over total' : 'under total'
	);

	// A sport that does not post spreads should not stay on the spread tab.
	$effect(() => {
		if (sport && !sport.markets.includes(market)) market = sport.markets[0];
	});

	interface CompareView {
		comparison: LineComparison;
		coverA: number;
		coverB: number;
	}

	const result = new Async<
		{
			lineA: string;
			oddsA: string;
			opposingA: string;
			lineB: string;
			oddsB: string;
			opposingB: string;
			stdDev: number | null;
			betType: BetType;
		},
		{ data: CompareView | null; error: MathError | string | null }
	>(
		() => ({ lineA, oddsA, opposingA, lineB, oddsB, opposingB, stdDev, betType }),
		async ({ lineA, oddsA, opposingA, lineB, oddsB, opposingB, stdDev, betType }) => {
			if (stdDev === null) return { data: null, error: null };

			const valueA = numeric(lineA);
			const valueB = numeric(lineB);
			const priceA = numeric(oddsA);
			const priceB = numeric(oddsB);
			const otherA = numeric(opposingA);
			const otherB = numeric(opposingB);
			if (valueA === null || valueB === null || priceA === null || priceB === null) {
				return { data: null, error: null };
			}
			if (otherA === null || otherB === null) {
				return {
					data: null,
					error:
						'Both sides of each market are required. Without the opposing price the vig cannot be removed, and every implied line comes out shifted.'
				};
			}

			// Devig first. The web app fed the raw vigged probability straight into
			// Φ⁻¹ — eight tenths of a point of pure hold, reported as market info.
			const [fairA, fairB] = await Promise.all([
				commands.fairCoverProb(priceA, otherA),
				commands.fairCoverProb(priceB, otherB)
			]);
			if (fairA.status === 'error') return { data: null, error: fairA.error };
			if (fairB.status === 'error') return { data: null, error: fairB.error };

			const comparison = await commands.compareLines(
				valueA,
				fairA.data,
				valueB,
				fairB.data,
				stdDev,
				betType
			);
			if (comparison.status === 'error') return { data: null, error: comparison.error };

			return {
				data: { comparison: comparison.data, coverA: fairA.data, coverB: fairB.data },
				error: null
			};
		}
	);

	const winner = $derived(result.current?.data?.comparison.better ?? null);
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

<InputCard title="Line A">
	<FormRow min={150}>
		<FormGroup label={market === 'spread' ? 'Spread' : 'Total'}>
			<input type="text" bind:value={lineA} placeholder={market === 'spread' ? '-10.5' : '220.5'} />
		</FormGroup>
		<FormGroup label="Your Price" hint="American">
			<input type="text" bind:value={oddsA} placeholder="-110" />
		</FormGroup>
		<FormGroup label="Opposing Price" hint="American — needed to devig">
			<input type="text" bind:value={opposingA} placeholder="-110" />
		</FormGroup>
	</FormRow>
</InputCard>

<InputCard title="Line B">
	<FormRow min={150}>
		<FormGroup label={market === 'spread' ? 'Spread' : 'Total'}>
			<input type="text" bind:value={lineB} placeholder={market === 'spread' ? '-11.5' : '221.5'} />
		</FormGroup>
		<FormGroup label="Your Price" hint="American">
			<input type="text" bind:value={oddsB} placeholder="+100" />
		</FormGroup>
		<FormGroup label="Opposing Price" hint="American — needed to devig">
			<input type="text" bind:value={opposingB} placeholder="-120" />
		</FormGroup>
	</FormRow>
</InputCard>

<OutputSection title="Comparison">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const { comparison, coverA, coverB } = result.current.data}
		<div class="verdict" class:tie={winner === null}>
			{winner === null ? 'The two lines are equivalent' : `Line ${winner.toUpperCase()} is better`}
		</div>
		<div class="meta">{sport?.label} · {typeLabel} · σ = {stdDev}</div>

		<div class="cards">
			<div class="card" class:winner={winner === 'a'}>
				<div class="card-title">Line A {winner === 'a' ? '★' : ''}</div>
				<div class="card-line">{fmtLine(numeric(lineA) ?? 0)} at {oddsA}</div>
				<div class="card-sub">Fair cover probability {pct(coverA)}</div>
				<div class="card-implied">
					Implied true {typeLabel}
					<span class="implied-value">{fmtLine(comparison.trueLineA, 3)}</span>
				</div>
			</div>
			<div class="card" class:winner={winner === 'b'}>
				<div class="card-title">Line B {winner === 'b' ? '★' : ''}</div>
				<div class="card-line">{fmtLine(numeric(lineB) ?? 0)} at {oddsB}</div>
				<div class="card-sub">Fair cover probability {pct(coverB)}</div>
				<div class="card-implied">
					Implied true {typeLabel}
					<span class="implied-value">{fmtLine(comparison.trueLineB, 3)}</span>
				</div>
			</div>
		</div>

		<div class="diff">
			Difference <strong>{num(comparison.diff, 3)} pts</strong>
			<span class="diff-note">
				{market === 'spread'
					? 'A higher implied spread means less embedded vig — better for the bettor.'
					: side === 'over'
						? 'A lower implied total means a lower break-even — better for an over.'
						: 'A higher implied total means more room — better for an under.'}
			</span>
		</div>
	{:else if !result.current?.error}
		<EmptyState icon="><" message="Enter both lines with both sides of each market" />
	{/if}
</OutputSection>

<InfoSection title="Why both sides are required">
	<p>
		A line and a price together pin down where the market thinks the true number sits: invert the
		normal CDF at the cover probability and you get the implied true line. -10.5 at -110 and -11.5
		at +100 are genuinely different bets, and this is how you tell which is better.
	</p>
	<p>
		<strong>The web version skipped the devig.</strong> It fed the raw implied probability into
		Φ⁻¹, so a team at -10.5 priced -110 in a -110/-110 market — fair cover probability exactly
		0.50, true line exactly -10.5 — came out at <code>-11.33</code>. Eight tenths of a point of pure
		hold, presented as market information. It cancels when both books post identical prices, which is
		presumably why nobody noticed, but every displayed line value was shifted and the Alternate Line
		Pricer built its whole ladder on top of it.
	</p>
</InfoSection>

<style>
	.verdict {
		font-size: 1.05rem;
		font-weight: 700;
		color: var(--accent-green);
	}

	.verdict.tie {
		color: var(--accent-amber);
	}

	.meta {
		font-size: 0.75rem;
		color: var(--text-muted);
		margin-top: 0.15rem;
		margin-bottom: 1rem;
	}

	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
		gap: 0.75rem;
	}

	.card {
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.85rem 1rem;
		background: var(--bg-tertiary);
	}

	.card.winner {
		border-color: var(--accent-green);
	}

	.card-title {
		font-size: 0.72rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--text-muted);
		margin-bottom: 0.4rem;
	}

	.card.winner .card-title {
		color: var(--accent-green);
	}

	.card-line {
		font-family: var(--font-mono);
		font-size: 1rem;
		font-weight: 600;
	}

	.card-sub {
		font-size: 0.78rem;
		color: var(--text-secondary);
		margin-top: 0.2rem;
	}

	.card-implied {
		margin-top: 0.6rem;
		padding-top: 0.6rem;
		border-top: 1px solid var(--border);
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.implied-value {
		display: block;
		font-family: var(--font-mono);
		font-size: 1.05rem;
		font-weight: 700;
		color: var(--accent-cyan);
	}

	.diff {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border);
		font-size: 0.82rem;
		color: var(--text-secondary);
	}

	.diff strong {
		font-family: var(--font-mono);
		color: var(--text-primary);
	}

	.diff-note {
		display: block;
		font-size: 0.75rem;
		color: var(--text-muted);
		margin-top: 0.2rem;
	}
</style>
