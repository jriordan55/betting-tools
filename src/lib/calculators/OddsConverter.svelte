<script lang="ts">
	import { commands, type MathError, type OddsFormat, type OddsView } from '$lib/bindings';
	import { Async } from '$lib/async.svelte';
	import { parseOdds, ODDS_FORMAT_OPTIONS, ODDS_PLACEHOLDER } from '$lib/odds';
	import { american, num, pct } from '$lib/format';
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

	let format = $state<OddsFormat>('american');
	let value = $state('');

	interface Converted {
		decimal: number;
		view: OddsView;
	}

	const result = new Async<
		{ value: string; format: OddsFormat },
		{ data: Converted | null; error: MathError | null }
	>(
		() => ({ value, format }),
		async ({ value, format }) => {
			// `to_decimal` range-checks American odds. The web app accepted
			// "-1.5" and returned 67.67, so a spread typed into an odds field
			// produced a plausible price; here it is a parse error.
			const { decimal, error } = await parseOdds(value, format);
			if (decimal === null) return { data: null, error };

			const view = await commands.fromDecimal(decimal);
			if (view.status === 'error') return { data: null, error: view.error };

			return { data: { decimal, view: view.data }, error: null };
		}
	);

	const hint = $derived(
		format === 'american'
			? 'Use + for underdogs, - for favorites'
			: format === 'decimal'
				? 'Must be greater than 1.00'
				: 'Format: numerator/denominator'
	);

	/** Switching format clears the field: "-110" means nothing as a fraction. */
	function onFormatChange() {
		value = '';
	}
</script>

<InputCard title="Input">
	<FormRow>
		<FormGroup label="Format">
			<ToggleGroup options={ODDS_FORMAT_OPTIONS} bind:value={format} onchange={onFormatChange} />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Odds Value" {hint}>
			<input
				type="text"
				bind:value
				placeholder={ODDS_PLACEHOLDER[format]}
				aria-invalid={result.current?.error != null}
			/>
		</FormGroup>
	</FormRow>
</InputCard>

<OutputSection title="Converted Odds">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const { decimal, view } = result.current.data}
		<ResultRow label="American" value={american(view.american)} color="highlight" />
		<ResultRow label="Decimal" value={num(decimal, 3)} />
		<ResultRow label="Fractional" value={`${view.fractionalNum}/${view.fractionalDen}`} />
		<ResultRow label="Implied Probability" value={pct(view.probability)} color="positive" />
	{:else if !result.current?.error}
		<EmptyState icon="<->" message="Enter odds above to see conversions" />
	{/if}
</OutputSection>

<InfoSection title="About Odds Formats">
	<p>
		<strong>American:</strong> Shows profit on a $100 bet (positive) or the stake needed to win $100
		(negative).<br />
		<strong>Decimal:</strong> Total return including stake. Multiply by stake for total payout.<br />
		<strong>Fractional:</strong> Traditional UK format showing profit relative to stake.
	</p>
	<p>
		The web version reported implied probability on a 0–100 scale from a module where every other
		function returned 0–1, and printed <code>0/1</code> for short prices. Both are fixed here.
	</p>
</InfoSection>
