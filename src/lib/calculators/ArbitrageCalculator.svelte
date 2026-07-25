<script lang="ts">
	import { commands, type Arbitrage, type MathError, type OddsFormat } from '$lib/bindings';
	import { Async, positive } from '$lib/async.svelte';
	import { parseAll, SIMPLE_FORMAT_OPTIONS, ODDS_PLACEHOLDER } from '$lib/odds';
	import { money, moneySigned, num, pct, pctSigned } from '$lib/format';
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

	const COUNT_OPTIONS = [
		{ value: '2', label: '2-Way' },
		{ value: '3', label: '3-Way' }
	];

	let format = $state<OddsFormat>('american');
	let totalStake = $state('1000');
	let count = $state('2');
	let names = $state(['Outcome 1', 'Outcome 2', 'Draw']);
	let odds = $state(['', '', '']);

	const active = $derived(Number.parseInt(count, 10));

	const result = new Async<
		{ odds: string[]; totalStake: string; format: OddsFormat; active: number },
		{ data: Arbitrage | null; error: MathError | string | null }
	>(
		() => ({ odds: [...odds], totalStake, format, active }),
		async ({ odds, totalStake, format, active }) => {
			const stake = positive(totalStake);
			if (stake === null) {
				return {
					data: null,
					error: totalStake.trim() === '' ? null : 'Total stake must be greater than 0.'
				};
			}

			const entered = odds.slice(0, active);
			if (entered.some((o) => o.trim() === '')) return { data: null, error: null };

			const parsed = await parseAll(entered, format);
			if (parsed.error) return { data: null, error: parsed.error };
			if (!parsed.decimals) return { data: null, error: null };

			const arb = await commands.arbitrage(parsed.decimals, stake);
			return arb.status === 'ok'
				? { data: arb.data, error: null }
				: { data: null, error: arb.error };
		}
	);

	// `is_arb()` in the core is exactly this comparison; a comparison is not math.
	const isArb = $derived((result.current?.data?.totalImplied ?? 1) < 1);
</script>

<InputCard title="Settings">
	<FormRow>
		<FormGroup label="Total Stake ($)">
			<input type="number" bind:value={totalStake} placeholder="1000" min="0" step="any" />
		</FormGroup>
		<FormGroup label="Odds Format">
			<ToggleGroup options={SIMPLE_FORMAT_OPTIONS} bind:value={format} />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Number of Outcomes">
			<ToggleGroup options={COUNT_OPTIONS} bind:value={count} />
		</FormGroup>
	</FormRow>
</InputCard>

<InputCard title="Outcomes">
	{#each { length: active } as _, index (index)}
		<div class="outcome">
			<div class="outcome-title">Outcome {index + 1}</div>
			<FormRow>
				<FormGroup label="Name (optional)">
					<input type="text" bind:value={names[index]} placeholder="Outcome {index + 1}" />
				</FormGroup>
				<FormGroup label="Odds">
					<input type="text" bind:value={odds[index]} placeholder={ODDS_PLACEHOLDER[format]} />
				</FormGroup>
			</FormRow>
		</div>
	{/each}
</InputCard>

<OutputSection title="Results">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const arb = result.current.data}
		<ResultLarge
			value={isArb ? pctSigned(arb.roi) : 'NO ARB'}
			label={isArb ? 'Guaranteed Return' : 'These prices do not arb'}
			color={isArb ? 'positive' : 'negative'}
		/>
		<ResultRow
			label="Total implied probability"
			value={pct(arb.totalImplied)}
			color={isArb ? 'positive' : 'negative'}
			hint="Below 100% is free money; above it, the books keep the difference"
		/>

		{#if isArb}
			<ResultRow label="Guaranteed profit" value={moneySigned(arb.profit)} color="positive" />
			<ResultRow label="ROI on total stake" value={pctSigned(arb.roi)} color="positive" />
		{:else}
			<ResultRow
				label="Cost of betting all sides"
				value={moneySigned(arb.profit)}
				color="negative"
				hint="Staking every outcome at these prices loses this much whatever happens"
			/>
		{/if}

		<div class="block">
			<div class="block-title">Stake Distribution</div>
			{#each arb.legs as leg, index (index)}
				<ResultRow
					label={names[index]?.trim() || `Outcome ${index + 1}`}
					value={`${money(leg.stake)} → ${money(leg.payout)}`}
					color={isArb ? 'highlight' : 'default'}
					hint="{num(leg.decimal, 3)} decimal · {pct(leg.implied)} implied"
				/>
			{/each}
		</div>
	{:else if !result.current?.error}
		<EmptyState icon="$" message="Enter a price for every outcome to check for an arb" />
	{/if}
</OutputSection>

<InfoSection title="About Arbitrage">
	<p>
		An arb exists when the implied probabilities of every outcome sum to less than 100% — usually
		because two books disagree. Stakes are split so that every outcome returns the same amount.
	</p>
	<p>
		When the prices do not arb, the row below reports what backing every side would cost you rather
		than calling a negative number a "guaranteed profit". The core returns that figure as an
		<code>Option</code> gated on whether the market actually arbs, so the guarantee cannot be claimed
		by accident.
	</p>
	<p>
		Arbs are thin, short-lived, and books limit accounts that hit them repeatedly. Treat the
		return as gross of the risk that one leg is voided and you are left with a naked position.
	</p>
</InfoSection>

<style>
	.outcome {
		margin-bottom: 1rem;
	}

	.outcome:last-child {
		margin-bottom: 0;
	}

	.outcome-title,
	.block-title {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		margin-bottom: 0.5rem;
	}

	.block {
		margin-top: 1.25rem;
		padding-top: 1.25rem;
		border-top: 1px solid var(--border);
	}
</style>
