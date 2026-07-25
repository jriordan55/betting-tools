<script lang="ts">
	import { commands, type LineSport, type Market, type MathError, type Middle } from '$lib/bindings';
	import { Async, numeric, positive } from '$lib/async.svelte';
	import { sportConfig } from '$lib/config';
	import { line as fmtLine, money, moneySigned, num, pct, pctSigned, signColor } from '$lib/format';
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

	interface Position {
		line: string;
		odds: string;
		opposing: string;
		stake: string;
	}

	let sports = $state<LineSport[]>([]);
	let sportKey = $state('nba');
	let market = $state<Market>('spread');

	let high = $state<Position>({ line: '', odds: '-110', opposing: '-110', stake: '100' });
	let low = $state<Position>({ line: '', odds: '-110', opposing: '-110', stake: '100' });

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

	$effect(() => {
		if (sport && !sport.markets.includes(market)) market = sport.markets[0];
	});

	const highLabel = $derived(
		market === 'spread'
			? 'High side — wins when the favorite’s margin is above the number'
			: 'Over — wins when the total is above the number'
	);
	const lowLabel = $derived(
		market === 'spread'
			? 'Low side — wins when the margin is below the number'
			: 'Under — wins when the total is below the number'
	);

	const result = new Async<
		{ high: Position; low: Position; stdDev: number | null; market: Market },
		{ data: Middle | null; error: MathError | string | null }
	>(
		() => ({ high: { ...high }, low: { ...low }, stdDev, market }),
		async ({ high, low, stdDev, market }) => {
			if (stdDev === null) return { data: null, error: null };

			const legs = [];
			for (const position of [high, low]) {
				const line = numeric(position.line);
				const price = numeric(position.odds);
				const opposing = numeric(position.opposing);
				const stake = positive(position.stake);
				if (line === null || price === null) return { data: null, error: null };
				if (opposing === null) {
					return {
						data: null,
						error:
							'Each position needs the opposing price too, so the vig can be removed before the line is inverted.'
					};
				}
				if (stake === null) return { data: null, error: 'Both stakes must be greater than 0.' };

				const fair = await commands.fairCoverProb(price, opposing);
				if (fair.status === 'error') return { data: null, error: fair.error };

				// The core takes decimal prices; the inputs above are American.
				const decimal = await commands.toDecimal(String(price), 'american');
				if (decimal.status === 'error') return { data: null, error: decimal.error };

				legs.push({ line, fairCoverProb: fair.data, stake, decimalOdds: decimal.data });
			}

			const middle = await commands.calculateMiddle(market, legs[0], legs[1], stdDev);
			return middle.status === 'ok'
				? { data: middle.data, error: null }
				: { data: null, error: middle.error };
		}
	);
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
</InputCard>

<InputCard title={market === 'spread' ? 'High Side' : 'Over'}>
	<p class="side-note">{highLabel}</p>
	<FormRow min={140}>
		<FormGroup label="Line">
			<input
				type="text"
				bind:value={high.line}
				placeholder={market === 'spread' ? '-3.5' : '222.5'}
			/>
		</FormGroup>
		<FormGroup label="Your Price" hint="American">
			<input type="text" bind:value={high.odds} placeholder="-110" />
		</FormGroup>
		<FormGroup label="Opposing Price" hint="American">
			<input type="text" bind:value={high.opposing} placeholder="-110" />
		</FormGroup>
		<FormGroup label="Stake ($)">
			<input type="text" bind:value={high.stake} placeholder="100" />
		</FormGroup>
	</FormRow>
</InputCard>

<InputCard title={market === 'spread' ? 'Low Side' : 'Under'}>
	<p class="side-note">{lowLabel}</p>
	<FormRow min={140}>
		<FormGroup label="Line">
			<input
				type="text"
				bind:value={low.line}
				placeholder={market === 'spread' ? '+7.5' : '218.5'}
			/>
		</FormGroup>
		<FormGroup label="Your Price" hint="American">
			<input type="text" bind:value={low.odds} placeholder="-110" />
		</FormGroup>
		<FormGroup label="Opposing Price" hint="American">
			<input type="text" bind:value={low.opposing} placeholder="-110" />
		</FormGroup>
		<FormGroup label="Stake ($)">
			<input type="text" bind:value={low.stake} placeholder="100" />
		</FormGroup>
	</FormRow>
</InputCard>

<OutputSection title="Middle Analysis">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const middle = result.current.data}
		<ResultLarge
			value={pct(middle.gapProb, 1)}
			label={middle.isMiddle ? 'Chance both tickets win' : 'Chance both tickets lose (trap)'}
			color={middle.isMiddle ? 'positive' : 'negative'}
		/>
		<ResultRow label="Gap size" value="{num(middle.gapSize, 1)} pts" color="highlight" />
		<ResultRow
			label="Expected value"
			value="{moneySigned(middle.ev)} ({pctSigned(middle.evFraction)})"
			color={signColor(middle.ev)}
		/>
		<ResultRow label="Total staked" value={money(middle.totalStaked)} />
		<ResultRow
			label="Estimated true center"
			value={fmtLine(middle.trueCenter, 2)}
			color="blue"
			hint={market === 'spread'
				? "Where the market thinks the favorite's margin sits"
				: 'Where the market thinks the total sits'}
		/>

		<div class="block">
			<div class="block-title">Every way this resolves</div>
			<table>
				<thead>
					<tr>
						<th class="left">Outcome</th>
						<th>Probability</th>
						<th>Net profit</th>
					</tr>
				</thead>
				<tbody>
					{#each middle.outcomes as outcome (outcome.label)}
						<tr>
							<td class="left">{outcome.label}</td>
							<td>{pct(outcome.probability, 1)}</td>
							<td class={signColor(outcome.netProfit)}>{moneySigned(outcome.netProfit)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{:else if !result.current?.error}
		<EmptyState icon="M" message="Enter both positions to price the middle" />
	{/if}
</OutputSection>

<InfoSection title="The bug this calculator was built around">
	<p>
		A middle is two tickets on opposite sides of the same market with a gap between them. Land in
		the gap and both win; a trap is the mirror image, where a zone exists in which both lose.
	</p>
	<p>
		<strong>The web version was wrong on every spread it was given.</strong>
		<code>impliedTrueLine(-3.5, …)</code> returns the mean of <em>minus</em> the margin;
		<code>impliedTrueLine(+7.5, …)</code> returns the mean of <em>plus</em> the margin. It averaged
		them — averaging a quantity with its own negation. On a symmetric market (both sides -5.5 at
		-110) it reported a true center of <code>-0.83</code> and gave the favorite a
		<strong>32.4%</strong> chance of covering. The answer is 50%. Totals were unaffected, because
		those already inverted in a single frame.
	</p>
	<p>
		The fix is structural rather than arithmetic: the legs are named for the direction that wins
		them rather than for which team holds them, and spread lines are folded onto the margin axis
		before anything else happens. Two sign conventions can no longer meet in one function.
	</p>
</InfoSection>

<style>
	.side-note {
		font-size: 0.75rem;
		color: var(--text-muted);
		margin-bottom: 0.85rem;
	}

	.block {
		margin-top: 1.25rem;
		padding-top: 1.25rem;
		border-top: 1px solid var(--border);
	}

	.block-title {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		margin-bottom: 0.75rem;
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
		padding: 0.4rem 0.5rem;
		border-bottom: 1px solid var(--border);
	}

	td {
		text-align: right;
		padding: 0.4rem 0.5rem;
		border-bottom: 1px solid var(--border);
	}

	tbody tr:last-child td {
		border-bottom: none;
	}

	.left {
		text-align: left;
	}

	.positive {
		color: var(--accent-green);
	}
	.negative {
		color: var(--accent-red);
	}
</style>
