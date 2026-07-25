<script lang="ts">
	import { commands, type Hedge, type HedgeGoal, type MathError, type OddsFormat } from '$lib/bindings';
	import { Async, positive } from '$lib/async.svelte';
	import { parseOdds, SIMPLE_FORMAT_OPTIONS, ODDS_PLACEHOLDER } from '$lib/odds';
	import { money, moneySigned, pctSigned, signColor } from '$lib/format';
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

	const GOAL_OPTIONS: { value: HedgeGoal; label: string }[] = [
		{ value: 'guarantee', label: 'Equalise Outcomes' },
		{ value: 'recoverStake', label: 'Recover Stake' }
	];

	let format = $state<OddsFormat>('american');
	let goal = $state<HedgeGoal>('guarantee');
	let originalStake = $state('');
	let originalOdds = $state('');
	let hedgeOdds = $state('');

	const result = new Async<
		{
			originalStake: string;
			originalOdds: string;
			hedgeOdds: string;
			goal: HedgeGoal;
			format: OddsFormat;
		},
		{ data: Hedge | null; error: MathError | string | null }
	>(
		() => ({ originalStake, originalOdds, hedgeOdds, goal, format }),
		async ({ originalStake, originalOdds, hedgeOdds, goal, format }) => {
			const stake = positive(originalStake);
			if (stake === null) {
				return {
					data: null,
					error: originalStake.trim() === '' ? null : 'Stake must be greater than 0.'
				};
			}

			const original = await parseOdds(originalOdds, format);
			if (original.error) return { data: null, error: original.error };
			const hedge = await parseOdds(hedgeOdds, format);
			if (hedge.error) return { data: null, error: hedge.error };
			if (original.decimal === null || hedge.decimal === null) return { data: null, error: null };

			const res = await commands.hedge(stake, original.decimal, hedge.decimal, goal);
			return res.status === 'ok'
				? { data: res.data, error: null }
				: { data: null, error: res.error };
		}
	);
</script>

<InputCard title="Settings">
	<FormRow>
		<FormGroup label="Odds Format">
			<ToggleGroup options={SIMPLE_FORMAT_OPTIONS} bind:value={format} />
		</FormGroup>
		<FormGroup
			label="Hedge Goal"
			hint={goal === 'guarantee'
				? 'Stake so the outcome no longer matters'
				: 'Stake only enough to get the original stake back if the hedge lands'}
		>
			<ToggleGroup options={GOAL_OPTIONS} bind:value={goal} />
		</FormGroup>
	</FormRow>
</InputCard>

<InputCard title="Original Bet">
	<FormRow>
		<FormGroup label="Stake ($)">
			<input type="number" bind:value={originalStake} placeholder="100" min="0" step="any" />
		</FormGroup>
		<FormGroup label="Odds">
			<input
				type="text"
				bind:value={originalOdds}
				placeholder={format === 'american' ? '+300' : '4.00'}
			/>
		</FormGroup>
	</FormRow>
</InputCard>

<InputCard title="Hedge Bet">
	<FormRow>
		<FormGroup label="Hedge Odds" hint="The price available on the opposite outcome">
			<input
				type="text"
				bind:value={hedgeOdds}
				placeholder={format === 'american' ? '-150' : '1.67'}
			/>
		</FormGroup>
	</FormRow>
</InputCard>

<OutputSection title="Results">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const hedge = result.current.data}
		<ResultLarge value={money(hedge.hedgeStake)} label="Hedge Stake Required" color="blue" />
		<ResultRow
			label="Worst case"
			value={moneySigned(hedge.worstCase)}
			color={signColor(hedge.worstCase)}
			hint="What you actually lock in — the lower of the two outcomes"
		/>
		<ResultRow label="Total risk" value={money(hedge.totalRisk)} />
		<ResultRow
			label="If the original wins"
			value={moneySigned(hedge.profitIfOriginalWins)}
			color={signColor(hedge.profitIfOriginalWins)}
		/>
		<ResultRow
			label="If the hedge wins"
			value={moneySigned(hedge.profitIfHedgeWins)}
			color={signColor(hedge.profitIfHedgeWins)}
		/>
		<ResultRow label="ROI on total risk" value={pctSigned(hedge.roi)} color={signColor(hedge.roi)} />
	{:else if !result.current?.error}
		<EmptyState icon="H" message="Enter your original bet and the hedge price" />
	{/if}
</OutputSection>

<InfoSection title="About Hedging">
	<p>
		Hedging bets the opposite outcome to convert an open position into a settled one.
		<strong>Equalise Outcomes</strong> stakes so both results pay the same;
		<strong>Recover Stake</strong> stakes only enough to get your original money back if the hedge
		lands, keeping more upside on the original.
	</p>
	<p>
		<strong>Worst case is the number to read.</strong> The web app showed the two outcome profits
		side by side and left you to compare them — and in equalise mode it labelled the result
		"Guaranteed Profit" even when that number was negative. Hedging a bet that has not moved in your
		favour locks in a loss; it does not create one out of nothing, but it does make it final.
	</p>
</InfoSection>
