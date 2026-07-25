<script lang="ts">
	import {
		commands,
		type BetaPosterior,
		type DirichletPosterior,
		type Edge,
		type MarginPosterior,
		type MathError,
		type OddsFormat
	} from '$lib/bindings';
	import { Async, numeric, positive } from '$lib/async.svelte';
	import { parseOdds, fairOdds, SIMPLE_FORMAT_OPTIONS, type FairOdds } from '$lib/odds';
	import { line as fmtLine, num, pct, pctSigned, points, signColor } from '$lib/format';
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

	type Mode = 'twoWay' | 'threeWay' | 'margin';

	const MODE_OPTIONS: { value: Mode; label: string }[] = [
		{ value: 'twoWay', label: '2-Way' },
		{ value: 'threeWay', label: '3-Way' },
		{ value: 'margin', label: 'Spread' }
	];

	let mode = $state<Mode>('twoWay');
	let format = $state<OddsFormat>('american');

	// Market side
	let marketA = $state('');
	let marketB = $state('');
	let marketDraw = $state('');
	let marketMargin = $state('');
	let marketStd = $state('2');

	// Model side
	let modelProbA = $state('');
	let modelProbDraw = $state('');
	let modelMargin = $state('');
	let modelStd = $state('3');

	// Weights
	let marketN = $state('1000');
	let modelN = $state('300');

	// Price to compare against
	let bestPrice = $state('');
	let threshold = $state('2.5');

	interface BayesView {
		beta: BetaPosterior | null;
		dirichlet: DirichletPosterior | null;
		margin: MarginPosterior | null;
		marketProbs: number[];
		edge: Edge | null;
		fair: FairOdds | null;
	}

	const result = new Async<Record<string, string>, { data: BayesView | null; error: MathError | string | null }>(
		() => ({
			mode,
			format,
			marketA,
			marketB,
			marketDraw,
			marketMargin,
			marketStd,
			modelProbA,
			modelProbDraw,
			modelMargin,
			modelStd,
			marketN,
			modelN,
			bestPrice,
			threshold
		}),
		async () => {
			const nMarket = positive(marketN);
			const nModel = positive(modelN);
			if (nMarket === null || nModel === null) {
				return { data: null, error: 'Both weights must be greater than 0.' };
			}

			if (mode === 'margin') {
				const market = numeric(marketMargin);
				const model = numeric(modelMargin);
				const sdMarket = positive(marketStd);
				const sdModel = positive(modelStd);
				if (market === null || model === null) return { data: null, error: null };
				if (sdMarket === null || sdModel === null) {
					return {
						data: null,
						error:
							'Both standard deviations must be greater than 0. A zero means "this source is certain", which hands it the answer outright.'
					};
				}

				const posterior = await commands.marginUpdate(market, model, sdMarket, sdModel);
				return posterior.status === 'ok'
					? {
							data: {
								beta: null,
								dirichlet: null,
								margin: posterior.data,
								marketProbs: [],
								edge: null,
								fair: null
							},
							error: null
						}
					: { data: null, error: posterior.error };
			}

			// Both probability modes start from the market's devigged prices.
			const prices = mode === 'twoWay' ? [marketA, marketB] : [marketA, marketDraw, marketB];
			const implied: number[] = [];
			for (const price of prices) {
				const parsed = await parseOdds(price, format);
				if (parsed.error) return { data: null, error: parsed.error };
				if (parsed.decimal === null) return { data: null, error: null };
				const view = await commands.fromDecimal(parsed.decimal);
				if (view.status === 'error') return { data: null, error: view.error };
				implied.push(view.data.probability);
			}

			// MPTO strips the vig. The web version reimplemented proportional
			// devigging inline — a third and fourth copy of the same function.
			const rows = await commands.devigAll(implied);
			const mpto = rows.find((r) => r.method === 'MPTO');
			if (!mpto) return { data: null, error: null };
			if (mpto.error) return { data: null, error: mpto.error };
			if (!mpto.fairProbs) return { data: null, error: null };
			const marketProbs = mpto.fairProbs;

			const modelA = numeric(modelProbA);
			if (modelA === null) return { data: null, error: null };

			if (mode === 'twoWay') {
				const posterior = await commands.betaUpdate(marketProbs[0], modelA / 100, nMarket, nModel);
				if (posterior.status === 'error') return { data: null, error: posterior.error };

				const { edge, fair } = await priceAgainst(posterior.data.posteriorProb);
				return {
					data: {
						beta: posterior.data,
						dirichlet: null,
						margin: null,
						marketProbs,
						edge,
						fair
					},
					error: null
				};
			}

			const modelDraw = numeric(modelProbDraw);
			if (modelDraw === null) return { data: null, error: null };

			// Home and draw are typed; away is whatever is left. Working out
			// "whatever is left" is a derivation, so the core does it — and
			// rejects a pair that leaves nothing over instead of handing the
			// Dirichlet update a negative outcome.
			const completed = await commands.completeSimplex([modelA / 100, modelDraw / 100]);
			if (completed.status === 'error') return { data: null, error: completed.error };
			const modelProbs = completed.data;

			const posterior = await commands.dirichletUpdate(marketProbs, modelProbs, nMarket, nModel);
			if (posterior.status === 'error') return { data: null, error: posterior.error };

			const { edge, fair } = await priceAgainst(posterior.data.posteriorProbs[0]);
			return {
				data: {
					beta: null,
					dirichlet: posterior.data,
					margin: null,
					marketProbs,
					edge,
					fair
				},
				error: null
			};
		}
	);

	/** Compare a posterior against the best price available, if one was entered. */
	async function priceAgainst(
		posteriorProb: number
	): Promise<{ edge: Edge | null; fair: FairOdds | null }> {
		const fair = await fairOdds(posteriorProb);
		if (bestPrice.trim() === '') return { edge: null, fair };

		const parsed = await parseOdds(bestPrice, format);
		if (parsed.decimal === null) return { edge: null, fair };

		const cutoff = numeric(threshold);
		const edge = await commands.edgeVsPrice(
			posteriorProb,
			parsed.decimal,
			cutoff === null ? 0 : cutoff / 100
		);
		return { edge: edge.status === 'ok' ? edge.data : null, fair };
	}
</script>

<InputCard title="Mode">
	<FormRow>
		<FormGroup label="Market Type">
			<ToggleGroup options={MODE_OPTIONS} bind:value={mode} />
		</FormGroup>
		{#if mode !== 'margin'}
			<FormGroup label="Odds Format">
				<ToggleGroup options={SIMPLE_FORMAT_OPTIONS} bind:value={format} />
			</FormGroup>
		{/if}
	</FormRow>
</InputCard>

{#if mode === 'margin'}
	<InputCard title="Spread Estimates">
		<FormRow>
			<FormGroup label="Market Spread" hint="Negative means home is favored">
				<input type="text" inputmode="decimal" bind:value={marketMargin} placeholder="-3.5" />
			</FormGroup>
			<FormGroup label="Market Uncertainty (σ)" hint="How tightly the market is priced">
				<input type="text" inputmode="decimal" bind:value={marketStd} placeholder="2" />
			</FormGroup>
		</FormRow>
		<FormRow>
			<FormGroup label="Your Spread" hint="What your model projects">
				<input type="text" inputmode="decimal" bind:value={modelMargin} placeholder="-5" />
			</FormGroup>
			<FormGroup label="Your Uncertainty (σ)" hint="Be honest — this decides the weighting">
				<input type="text" inputmode="decimal" bind:value={modelStd} placeholder="3" />
			</FormGroup>
		</FormRow>
	</InputCard>
{:else}
	<InputCard title="Market Prices">
		<FormRow min={150}>
			<FormGroup label="Home / Side A">
				<input type="text" bind:value={marketA} placeholder="-150" />
			</FormGroup>
			{#if mode === 'threeWay'}
				<FormGroup label="Draw">
					<input type="text" bind:value={marketDraw} placeholder="+260" />
				</FormGroup>
			{/if}
			<FormGroup label="Away / Side B">
				<input type="text" bind:value={marketB} placeholder="+130" />
			</FormGroup>
		</FormRow>
	</InputCard>

	<InputCard title="Your Model">
		<FormRow min={150}>
			<FormGroup label="Home / Side A (%)" hint="Your probability estimate">
				<input type="text" inputmode="decimal" bind:value={modelProbA} placeholder="62" />
			</FormGroup>
			{#if mode === 'threeWay'}
				<FormGroup label="Draw (%)" hint="Away is whatever is left">
					<input type="text" inputmode="decimal" bind:value={modelProbDraw} placeholder="22" />
				</FormGroup>
			{/if}
		</FormRow>
	</InputCard>
{/if}

<InputCard title="Weights">
	<FormRow>
		<FormGroup label="Market Weight (N)" hint="Higher trusts the market more">
			<input type="text" inputmode="numeric" bind:value={marketN} placeholder="1000" />
		</FormGroup>
		<FormGroup label="Model Weight (N)" hint="Higher trusts your model more">
			<input type="text" inputmode="numeric" bind:value={modelN} placeholder="300" />
		</FormGroup>
	</FormRow>
	{#if mode !== 'margin'}
		<FormRow>
			<FormGroup label="Best Available Price" hint="Optional — the price you can actually get">
				<input type="text" bind:value={bestPrice} placeholder="+140" />
			</FormGroup>
			<FormGroup label="Edge Threshold (%)" hint="Minimum edge before this calls it a bet">
				<input type="text" inputmode="decimal" bind:value={threshold} placeholder="2.5" />
			</FormGroup>
		</FormRow>
	{/if}
</InputCard>

<OutputSection title="Posterior">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data?.beta}
		{@const beta = result.current.data.beta}
		<ResultLarge value={pct(beta.posteriorProb)} label="Posterior probability" color="highlight" />
		<ResultRow
			label="Market fair probability"
			value={pct(result.current.data.marketProbs[0])}
			color="blue"
			hint="Devigged with MPTO"
		/>
		<ResultRow
			label="Shift from the market"
			value={points(beta.shiftFromMarket)}
			color={signColor(beta.shiftFromMarket)}
		/>
		<ResultRow
			label="Prior"
			value="Beta({num(beta.priorAlpha, 1)}, {num(beta.priorBeta, 1)})"
			color="muted"
		/>
		<ResultRow
			label="Posterior"
			value="Beta({num(beta.posteriorAlpha, 1)}, {num(beta.posteriorBeta, 1)})"
			color="muted"
		/>
	{:else if result.current?.data?.dirichlet}
		{@const dir = result.current.data.dirichlet}
		{@const labels = ['Home / Side A', 'Draw', 'Away / Side B']}
		<ResultLarge value={pct(dir.posteriorProbs[0])} label="Posterior — home" color="highlight" />
		{#each dir.posteriorProbs as prob, index (index)}
			<ResultRow
				label={labels[index] ?? `Outcome ${index + 1}`}
				value="{pct(prob)} (market {pct(result.current.data.marketProbs[index])})"
				color={index === 0 ? 'highlight' : 'default'}
			/>
		{/each}
		<ResultRow
			label="Largest shift from the market"
			value={points(dir.maxShiftFromMarket)}
			color="amber"
		/>
	{:else if result.current?.data?.margin}
		{@const margin = result.current.data.margin}
		<ResultLarge value={fmtLine(margin.spread, 2)} label="Posterior spread" color="highlight" />
		<ResultRow label="Posterior margin" value={num(margin.margin, 2)} />
		<ResultRow
			label="Posterior uncertainty (σ)"
			value={num(margin.stdDev, 3)}
			hint="Tighter than either source alone — that is the point of combining them"
		/>
		<ResultRow
			label="Weight on the market"
			value={pct(margin.marketWeight)}
			color="blue"
			hint="Precision-weighted: the more confident source gets more say"
		/>
	{:else if !result.current?.error}
		<EmptyState icon="B" message="Enter the market's prices and your model's estimate" />
	{/if}
</OutputSection>

{#if result.current?.data?.edge}
	{@const edge = result.current.data.edge}
	<OutputSection title="Against the Available Price">
		<ResultLarge
			value={edge.isBet ? 'BET' : 'PASS'}
			label="Verdict at your {threshold}% threshold"
			color={edge.isBet ? 'positive' : 'negative'}
		/>
		<ResultRow label="Edge" value={points(edge.edgePoints)} color={signColor(edge.edgePoints)} />
		<ResultRow
			label="Expected value"
			value={pctSigned(edge.evFraction)}
			color={signColor(edge.evFraction)}
		/>
		<ResultRow label="Fair decimal odds" value={num(edge.fairOdds, 3)} color="highlight" />
	</OutputSection>
{/if}

<InfoSection title="What this does, and four things it used to get wrong">
	<p>
		The market is a very good estimate with a lot of money behind it. Your model is another
		estimate. Bayesian updating combines them by weight rather than picking one: the market's price
		is the prior, your model is the evidence, and N on each side says how much each is worth.
	</p>
	<p>
		Spreads combine by <strong>precision</strong> — the reciprocal of variance — so the source that
		is more certain gets more say, and the posterior is tighter than either input.
	</p>
	<p><strong>The bugs found in the original:</strong></p>
	<p>
		<code>removeVig</code> and <code>removeVig3Way</code> were proportional devigging — a third and
		fourth copy of <code>devigMPTO</code> living in a different file. Deleted; this calls the devig
		module, which is also how the other four methods became available for free.
	</p>
	<p>
		<code>calculateMLEdge</code> reimplemented the American conversion <em>without</em> the clamp the
		real one has, so a posterior of exactly 0 or 1 returned <code>Infinity</code> and priced a bet off
		it.
	</p>
	<p>
		<code>bayesianSpreadUpdate</code> clamped a zero standard deviation to 0.001 — a precision of a
		million. That silently hands that source the entire answer and discards the other without a word.
		It is an error now.
	</p>
	<p>
		<code>dirichletProbUpdate</code> never checked that the market probabilities summed to 1, so a
		set that had not been devigged was accepted as a prior and misweighted. It was also hard-coded to
		exactly three outcomes, which nothing in the mathematics requires.
	</p>
</InfoSection>
