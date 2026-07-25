<script lang="ts">
	import { commands, type Kelly, type MathError, type OddsFormat } from '$lib/bindings';
	import { Async, positive } from '$lib/async.svelte';
	import { parseOdds, SIMPLE_FORMAT_OPTIONS, ODDS_PLACEHOLDER } from '$lib/odds';
	import { money, pct, pctSigned, points, signColor } from '$lib/format';
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

	const FRACTIONS = [
		{ value: '1', label: 'Full Kelly (100%)' },
		{ value: '0.75', label: '3/4 Kelly (75%)' },
		{ value: '0.5', label: 'Half Kelly (50%)' },
		{ value: '0.25', label: 'Quarter Kelly (25%)' }
	];

	let format = $state<OddsFormat>('american');
	let bankroll = $state('10000');
	let odds = $state('');
	let winProbPct = $state('');
	let multiplier = $state('1');

	const result = new Async<
		{ bankroll: string; odds: string; winProbPct: string; multiplier: string; format: OddsFormat },
		{ data: Kelly | null; error: MathError | string | null }
	>(
		() => ({ bankroll, odds, winProbPct, multiplier, format }),
		async ({ bankroll, odds, winProbPct, multiplier, format }) => {
			const bank = positive(bankroll);
			if (bank === null) {
				return {
					data: null,
					error: bankroll.trim() === '' ? null : 'Bankroll must be greater than 0.'
				};
			}

			if (winProbPct.trim() === '') return { data: null, error: null };
			const probPct = Number.parseFloat(winProbPct);
			if (!Number.isFinite(probPct)) return { data: null, error: null };
			// Rescaling a percentage the user typed into the 0–1 fraction the core
			// works in is unit conversion, not a derivation.
			const trueProb = probPct / 100;

			const price = await parseOdds(odds, format);
			if (price.error) return { data: null, error: price.error };
			if (price.decimal === null) return { data: null, error: null };

			const kelly = await commands.kelly(
				price.decimal,
				trueProb,
				bank,
				Number.parseFloat(multiplier)
			);
			return kelly.status === 'ok'
				? { data: kelly.data, error: null }
				: { data: null, error: kelly.error };
		}
	);

	const shouldBet = $derived((result.current?.data?.adjustedFraction ?? 0) > 0);
</script>

<InputCard title="Input">
	<FormRow>
		<FormGroup label="Bankroll ($)">
			<input type="number" bind:value={bankroll} placeholder="10000" min="0" step="any" />
		</FormGroup>
		<FormGroup label="Odds Format">
			<ToggleGroup options={SIMPLE_FORMAT_OPTIONS} bind:value={format} />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Odds" hint="The price you are getting at the book">
			<input type="text" bind:value={odds} placeholder={ODDS_PLACEHOLDER[format]} />
		</FormGroup>
		<FormGroup label="True Win Probability (%)" hint="Your estimate, not the book's">
			<input type="number" bind:value={winProbPct} placeholder="55" min="0" max="100" step="0.1" />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Kelly Fraction" hint="Fractional Kelly trades growth for a smoother ride">
			<select bind:value={multiplier}>
				{#each FRACTIONS as f (f.value)}
					<option value={f.value}>{f.label}</option>
				{/each}
			</select>
		</FormGroup>
	</FormRow>
</InputCard>

<OutputSection title="Results">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const kelly = result.current.data}
		<ResultLarge
			value={money(kelly.adjustedStake)}
			label={shouldBet ? 'Optimal Stake' : 'Do Not Bet — No Edge'}
			color={shouldBet ? 'positive' : 'negative'}
		/>
		<ResultRow
			label="Kelly percentage of bankroll"
			value={pct(kelly.adjustedFraction)}
			color={shouldBet ? 'positive' : 'negative'}
		/>
		<ResultRow label="Full Kelly stake" value={money(kelly.fullStake)} />
		<ResultRow label="Full Kelly percentage" value={pct(kelly.fullFraction)} />
		<ResultRow
			label="Expected value"
			value={pctSigned(kelly.evFraction)}
			color={signColor(kelly.evFraction)}
		/>
		<ResultRow
			label="Edge"
			value={points(kelly.edgePoints)}
			color={signColor(kelly.edgePoints)}
			hint="Your probability minus the probability the price implies"
		/>
	{:else if !result.current?.error}
		<EmptyState icon="K" message="Enter odds and a win probability to size the bet" />
	{/if}
</OutputSection>

<InfoSection title="About the Kelly Criterion">
	<p>
		Kelly maximises the long-run growth rate of a bankroll. Full Kelly is aggressive — the swings
		are large enough that most bettors use a half or quarter fraction, which gives up a little
		growth for a great deal less variance.
	</p>
	<p>
		Kelly is only as good as your probability estimate. Overstating your edge by a few points
		leads to systematic overbetting, and overbetting past the optimum is worse than underbetting by
		the same amount. A negative Kelly fraction means the bet is -EV at this price: the answer is
		zero, not a small stake.
	</p>
</InfoSection>
