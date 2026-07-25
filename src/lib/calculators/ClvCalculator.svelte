<script lang="ts">
	import {
		commands,
		type Clv,
		type ExpectedValue,
		type MathError,
		type OddsFormat
	} from '$lib/bindings';
	import { Async, positive } from '$lib/async.svelte';
	import { parseOdds, SIMPLE_FORMAT_OPTIONS, ODDS_PLACEHOLDER } from '$lib/odds';
	import { american, money, moneySigned, num, pct, pctSigned, points, signColor } from '$lib/format';
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
	let betOdds = $state('');
	let closingOdds = $state('');
	let opposingClose = $state('');
	let stake = $state('100');

	interface ClvView {
		clv: Clv;
		betAmerican: number;
		closeAmerican: number;
		/** Dollar EV against the fair close — computed in Rust, not `stake × fraction` here. */
		fairEv: ExpectedValue | null;
	}

	const result = new Async<
		{
			betOdds: string;
			closingOdds: string;
			opposingClose: string;
			stake: string;
			format: OddsFormat;
		},
		{ data: ClvView | null; error: MathError | null }
	>(
		() => ({ betOdds, closingOdds, opposingClose, stake, format }),
		async ({ betOdds, closingOdds, opposingClose, stake, format }) => {
			const bet = await parseOdds(betOdds, format);
			if (bet.error) return { data: null, error: bet.error };
			const close = await parseOdds(closingOdds, format);
			if (close.error) return { data: null, error: close.error };
			if (bet.decimal === null || close.decimal === null) return { data: null, error: null };

			const opposing = await parseOdds(opposingClose, format);
			if (opposing.error) return { data: null, error: opposing.error };

			const clv = await commands.clv(bet.decimal, close.decimal, opposing.decimal);
			if (clv.status === 'error') return { data: null, error: clv.error };

			const [betView, closeView] = await Promise.all([
				commands.fromDecimal(bet.decimal),
				commands.fromDecimal(close.decimal)
			]);

			const stakeAmount = positive(stake);
			let fairEv: ExpectedValue | null = null;
			if (stakeAmount !== null && clv.data.fairProb !== null) {
				const ev = await commands.expectedValue(bet.decimal, clv.data.fairProb, stakeAmount);
				if (ev.status === 'ok') fairEv = ev.data;
			}

			return {
				data: {
					clv: clv.data,
					betAmerican: betView.status === 'ok' ? betView.data.american : Number.NaN,
					closeAmerican: closeView.status === 'ok' ? closeView.data.american : Number.NaN,
					fairEv
				},
				error: null
			};
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
		<FormGroup label="Your Bet Odds" hint="The price when you placed the bet">
			<input type="text" bind:value={betOdds} placeholder={ODDS_PLACEHOLDER[format]} />
		</FormGroup>
		<FormGroup label="Closing Odds" hint="Your side's final price before kickoff">
			<input
				type="text"
				bind:value={closingOdds}
				placeholder={format === 'american' ? '-130' : '1.77'}
			/>
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup
			label="Opposing Closing Odds (optional)"
			hint="The other side's close. Without it the vig cannot be removed and EV is overstated."
		>
			<input
				type="text"
				bind:value={opposingClose}
				placeholder={format === 'american' ? '+110' : '2.10'}
			/>
		</FormGroup>
	</FormRow>
</InputCard>

<OutputSection title="CLV Analysis">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const { clv, betAmerican, closeAmerican, fairEv } = result.current.data}
		<ResultLarge
			value={points(clv.probPoints)}
			label="Closing Line Value (probability points)"
			color={signColor(clv.probPoints)}
		/>

		<ResultRow
			label="Probability points"
			value={points(clv.probPoints)}
			color={signColor(clv.probPoints)}
			hint="The measure that compares across price levels"
		/>
		<ResultRow
			label="American cents"
			value={`${clv.cents > 0 ? '+' : ''}${clv.cents}`}
			color={signColor(clv.cents)}
			hint="What people quote. Fifty cents on a longshot is worth less than twenty on a favorite."
		/>
		<ResultRow
			label="Ratio of prices"
			value={pctSigned(clv.ratio)}
			color="muted"
			hint="What the web app displayed under three separate labels. Flatters longshots — kept only for continuity."
		/>

		<div class="block">
			<div class="block-title">Expected Value</div>
			<ResultRow
				label="vs raw closing price"
				value={pctSigned(clv.evVsRawClose)}
				color="muted"
				hint="Treats the closing price as truth, vig included. Always overstated."
			/>
			{#if clv.evVsFair !== null}
				<ResultRow
					label="vs fair (devigged) close"
					value={pctSigned(clv.evVsFair)}
					color={signColor(clv.evVsFair)}
					hint="The honest figure — the vig has been removed using the opposing close."
				/>
				{#if fairEv}
					<ResultRow
						label="EV on this stake"
						value={moneySigned(fairEv.evAmount)}
						color={signColor(fairEv.evAmount)}
					/>
				{/if}
				{#if clv.fairProb !== null}
					<ResultRow label="Fair win probability" value={pct(clv.fairProb)} color="highlight" />
				{/if}
			{:else}
				<ResultRow
					label="vs fair (devigged) close"
					value="—"
					color="muted"
					hint="Enter the opposing closing price to remove the vig."
				/>
			{/if}
		</div>

		<div class="block">
			<div class="block-title">Prices</div>
			<ResultRow
				label="Your odds"
				value={`${american(betAmerican)} (${num(clv.betDecimal, 3)})`}
				color="highlight"
			/>
			<ResultRow
				label="Closing odds"
				value={`${american(closeAmerican)} (${num(clv.closeDecimal, 3)})`}
			/>
			<ResultRow label="Your implied probability" value={pct(clv.betImplied)} />
			<ResultRow
				label="Closing implied probability"
				value={pct(clv.closeImplied)}
				hint="Still contains the book's vig"
			/>
			<ResultRow
				label="Line movement"
				value={clv.probPoints > 0 ? 'In your favor' : clv.probPoints < 0 ? 'Against you' : 'No move'}
				color={signColor(clv.probPoints)}
			/>
		</div>
	{:else if !result.current?.error}
		<EmptyState icon="CLV" message="Enter your price and the closing price to measure CLV" />
	{/if}
</OutputSection>

<InfoSection title="Why three numbers, not one">
	<p>
		The web version showed <em>one</em> value under three labels — "Closing Line Value", "Edge (cents
		per dollar)", and "Expected Value" are all
		<code>bet/close − 1</code>. Three rows, one expression.
	</p>
	<p>
		Worse, that expression is a <strong>ratio of decimal odds</strong>, which flatters longshots.
		It ranks +400 → +350 (11.1%) above -110 → -130 (7.9%). In probability points the order reverses:
		2.22 against 4.14. The second bet is nearly twice as valuable and the web app said otherwise.
	</p>
	<p>
		The EV figure had a second problem: it took <code>1 / closing price</code> as the true
		probability, vig and all. Betting the exact closing price reported 0% EV when the honest figure is
		about −4.5%. Supply the opposing closing price and the vig comes out properly.
	</p>
</InfoSection>

<style>
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
</style>
