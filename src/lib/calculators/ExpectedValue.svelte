<script lang="ts">
	import { commands, type ExpectedValue, type MathError, type OddsFormat } from '$lib/bindings';
	import { Async, positive } from '$lib/async.svelte';
	import { parseOdds, SIMPLE_FORMAT_OPTIONS, ODDS_PLACEHOLDER } from '$lib/odds';
	import { moneySigned, pct, pctSigned, points, signColor } from '$lib/format';
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

	let format = $state<OddsFormat>('american');
	let yourOdds = $state('');
	let fairOddsInput = $state('');
	let stake = $state('100');

	interface EvView {
		ev: ExpectedValue;
		fairProb: number;
	}

	const result = new Async<
		{ yourOdds: string; fairOddsInput: string; stake: string; format: OddsFormat },
		{ data: EvView | null; error: MathError | string | null }
	>(
		() => ({ yourOdds, fairOddsInput, stake, format }),
		async ({ yourOdds, fairOddsInput, stake, format }) => {
			const stakeAmount = positive(stake);
			if (stakeAmount === null) {
				return { data: null, error: stake.trim() === '' ? null : 'Stake must be greater than 0.' };
			}

			const yours = await parseOdds(yourOdds, format);
			if (yours.error) return { data: null, error: yours.error };
			const fair = await parseOdds(fairOddsInput, format);
			if (fair.error) return { data: null, error: fair.error };
			if (yours.decimal === null || fair.decimal === null) return { data: null, error: null };

			const fairView = await commands.fromDecimal(fair.decimal);
			if (fairView.status === 'error') return { data: null, error: fairView.error };
			const fairProb = fairView.data.probability;

			const ev = await commands.expectedValue(yours.decimal, fairProb, stakeAmount);
			if (ev.status === 'error') return { data: null, error: ev.error };

			return { data: { ev: ev.data, fairProb }, error: null };
		}
	);
</script>

<InputCard title="Input">
	<FormRow>
		<FormGroup label="Odds Format">
			<ToggleGroup options={SIMPLE_FORMAT_OPTIONS} bind:value={format} />
		</FormGroup>
		<FormGroup label="Stake ($)">
			<input type="number" bind:value={stake} placeholder="100" min="0" step="any" />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Your Odds" hint="The price you are getting at the book">
			<input type="text" bind:value={yourOdds} placeholder={ODDS_PLACEHOLDER[format]} />
		</FormGroup>
		<FormGroup label="Fair / No-Vig Odds" hint="True price — e.g. a sharp book's no-vig line">
			<input
				type="text"
				bind:value={fairOddsInput}
				placeholder={format === 'american' ? '+130' : '2.30'}
			/>
		</FormGroup>
	</FormRow>
</InputCard>

<OutputSection title="Results">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const { ev, fairProb } = result.current.data}
		<ResultLarge
			value={pctSigned(ev.evFraction)}
			label="Expected Value"
			color={signColor(ev.evFraction)}
		/>
		<ResultRow label="EV on this bet" value={moneySigned(ev.evAmount)} color={signColor(ev.evAmount)} />
		<ResultRow
			label="Your edge"
			value={points(ev.edgePoints)}
			color={signColor(ev.edgePoints)}
			hint="True probability minus the probability your price implies"
		/>
		<ResultRow label="True win probability" value={pct(fairProb)} color="highlight" />
		<ResultRow label="Implied by your odds" value={pct(ev.impliedProb)} />
		<ResultRow label="Break-even win rate" value={pct(ev.breakeven)} />
	{:else if !result.current?.error}
		<EmptyState icon="EV" message="Enter your odds and the fair odds to calculate expected value" />
	{/if}
</OutputSection>

<InfoSection title="About Expected Value">
	<p>
		Expected value is the average you win or lose per bet over the long run. A sharp book's no-vig
		line is the usual stand-in for the true price: if your book pays better than that, the bet is
		+EV.
	</p>
	<p>
		Edge is reported in <strong>probability points</strong>, not as a ratio of prices. A ratio
		flatters longshots — see the Closing Line Value calculator, where the web app's single number
		ranked +400→+350 above -110→-130 despite the second being nearly twice as valuable.
	</p>
</InfoSection>
