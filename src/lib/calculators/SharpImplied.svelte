<script lang="ts">
	import { commands, type ImpliedScores, type MathError, type OddsFormat } from '$lib/bindings';
	import { Async, numeric } from '$lib/async.svelte';
	import { parseOdds, fairOdds, SIMPLE_FORMAT_OPTIONS, ODDS_PLACEHOLDER, type FairOdds } from '$lib/odds';
	import { american, num, pct } from '$lib/format';
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
	let homeMl = $state('');
	let awayMl = $state('');
	let homeSpread = $state('');
	let total = $state('');

	interface SharpView {
		homeProb: number;
		awayProb: number;
		homeFair: FairOdds;
		awayFair: FairOdds;
		hold: number;
		scores: ImpliedScores | null;
		scoreError: MathError | null;
	}

	const result = new Async<
		{ homeMl: string; awayMl: string; homeSpread: string; total: string; format: OddsFormat },
		{ data: SharpView | null; error: MathError | null }
	>(
		() => ({ homeMl, awayMl, homeSpread, total, format }),
		async ({ homeMl, awayMl, homeSpread, total, format }) => {
			const home = await parseOdds(homeMl, format);
			if (home.error) return { data: null, error: home.error };
			const away = await parseOdds(awayMl, format);
			if (away.error) return { data: null, error: away.error };
			if (home.decimal === null || away.decimal === null) return { data: null, error: null };

			const [homeView, awayView] = await Promise.all([
				commands.fromDecimal(home.decimal),
				commands.fromDecimal(away.decimal)
			]);
			if (homeView.status === 'error') return { data: null, error: homeView.error };
			if (awayView.status === 'error') return { data: null, error: awayView.error };

			// MPTO is the proportional method the original used. The other four
			// live in the Devig Calculator, where they can be compared.
			const rows = await commands.devigAll([homeView.data.probability, awayView.data.probability]);
			const mpto = rows.find((r) => r.method === 'MPTO');
			if (!mpto) return { data: null, error: null };
			if (mpto.error) return { data: null, error: mpto.error };
			if (!mpto.fairProbs || mpto.fairProbs.length < 2) return { data: null, error: null };

			const holdResult = await commands.calculateHold(
				homeView.data.probability,
				awayView.data.probability
			);
			if (holdResult.status === 'error') return { data: null, error: holdResult.error };

			const [homeFair, awayFair] = await Promise.all([
				fairOdds(mpto.fairProbs[0]),
				fairOdds(mpto.fairProbs[1])
			]);

			const spreadValue = numeric(homeSpread);
			const totalValue = numeric(total);
			let scores: ImpliedScores | null = null;
			let scoreError: MathError | null = null;
			if (spreadValue !== null && totalValue !== null) {
				const projected = await commands.impliedScores(spreadValue, totalValue);
				if (projected.status === 'ok') scores = projected.data;
				else scoreError = projected.error;
			}

			return {
				data: {
					homeProb: mpto.fairProbs[0],
					awayProb: mpto.fairProbs[1],
					homeFair,
					awayFair,
					hold: holdResult.data.hold,
					scores,
					scoreError
				},
				error: null
			};
		}
	);

	function fairText(fair: FairOdds) {
		return fair.decimal === null
			? american(fair.american)
			: `${american(fair.american)} (${num(fair.decimal, 3)})`;
	}
</script>

<InputCard title="Sharp Lines">
	<FormRow>
		<FormGroup label="Odds Format">
			<ToggleGroup options={SIMPLE_FORMAT_OPTIONS} bind:value={format} />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Home Moneyline">
			<input
				type="text"
				bind:value={homeMl}
				placeholder={format === 'american' ? '-150' : '1.67'}
			/>
		</FormGroup>
		<FormGroup label="Away Moneyline">
			<input
				type="text"
				bind:value={awayMl}
				placeholder={format === 'american' ? '+130' : '2.30'}
			/>
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Home Spread (optional)" hint="Negative means home is favored">
			<input type="text" bind:value={homeSpread} placeholder="-3" />
		</FormGroup>
		<FormGroup label="Game Total (optional)" hint="Combined points">
			<input type="text" bind:value={total} placeholder="47.5" />
		</FormGroup>
	</FormRow>
</InputCard>

<OutputSection title="No-Vig Fair Probabilities">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const view = result.current.data}
		<div class="split">
			<ResultLarge value={pct(view.homeProb, 1)} label="Home Win" color="highlight" />
			<ResultLarge value={pct(view.awayProb, 1)} label="Away Win" color="highlight" />
		</div>
		<ResultRow label="Home fair odds" value={fairText(view.homeFair)} />
		<ResultRow label="Away fair odds" value={fairText(view.awayFair)} />
		<ResultRow
			label="Market hold"
			value={pct(view.hold)}
			color="amber"
			hint="How much vig the book charged on this market"
		/>

		{#if view.scores}
			<div class="block">
				<div class="block-title">Implied Score</div>
				<ResultRow label="Home" value={num(view.scores.home, 1)} color="blue" />
				<ResultRow label="Away" value={num(view.scores.away, 1)} color="blue" />
			</div>
		{:else if view.scoreError}
			<div class="block">
				<ErrorNote error={view.scoreError} />
			</div>
		{/if}
	{:else if !result.current?.error}
		<EmptyState icon="#" message="Enter both moneylines to derive fair probabilities" />
	{/if}
</OutputSection>

<InfoSection title="How It Works">
	<p>
		Sharp books post the tightest markets, but they still charge vig. This strips it with the
		multiplicative (MPTO) method to recover the probabilities behind the prices. Compare methods in
		the Devig Calculator — on a lopsided market they disagree by several points, and MPTO is
		usually the least defensible of the five.
	</p>
	<p>
		Given a spread and a total, each side's projected score follows from
		<code>home + away = total</code> and <code>away − home = spread</code>. A spread wide enough to
		project a negative score is rejected rather than displayed.
	</p>
</InfoSection>

<style>
	.split {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
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
</style>
