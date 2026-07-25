<script lang="ts">
	import {
		commands,
		type EvPoint,
		type GameSport,
		type GameView,
		type MathError
	} from '$lib/bindings';
	import { Async, numeric } from '$lib/async.svelte';
	import { sportConfig } from '$lib/config';
	import { line as fmtLine, num, pct } from '$lib/format';
	import { parseOdds } from '$lib/odds';
	import LineChart from '$lib/charts/LineChart.svelte';
	import PmfChart from '$lib/charts/PmfChart.svelte';
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

	const KEY_NUMBERS = new Set([3, 7, 10, 14]);

	let sports = $state<GameSport[]>([]);
	let sportKey = $state('football');

	let muHome = $state('24.5');
	let muAway = $state('21');
	let sdMargin = $state('13.5');
	let sdTotal = $state('10.2');
	let spread = $state('-3.5');
	let total = $state('45.5');
	let keyNumbers = $state(true);

	let priceInput = $state('-110');

	let error = $state<MathError | string | null>(null);

	const sport = $derived(sports.find((s) => s.key === sportKey) ?? null);

	const SPORT_OPTIONS = $derived(
		sports
			.filter((s) => s.normalModel)
			.map((s) => ({ value: s.key, label: s.label }))
	);

	$effect(() => {
		void sportConfig().then((config) => {
			sports = config.games;
		});
	});

	/**
	 * Loading a preset replaces every field it owns. Deriving the inputs
	 * instead would make them un-editable; syncing on change alone would leave
	 * a football spread attached to basketball variance.
	 */
	function loadPreset(key: string) {
		const preset = sports.find((s) => s.key === key);
		if (!preset) return;
		muHome = String(preset.muHome);
		muAway = String(preset.muAway);
		sdMargin = String(preset.sdMargin);
		sdTotal = String(preset.sdTotal);
		spread = String(preset.spread);
		total = String(preset.total);
		keyNumbers = preset.keyNumbers;
	}

	const game = new Async(
		() => ({
			muHome: numeric(muHome),
			muAway: numeric(muAway),
			sdMargin: numeric(sdMargin),
			sdTotal: numeric(sdTotal),
			spread: numeric(spread),
			total: numeric(total),
			keyNumbers
		}),
		async (d): Promise<GameView | null> => {
			error = null;
			if (
				d.muHome === null ||
				d.muAway === null ||
				d.sdMargin === null ||
				d.sdTotal === null ||
				d.spread === null ||
				d.total === null
			) {
				return null;
			}

			// Three standard deviations either side of the posted line covers
			// every alternate a book would offer, at half-point granularity.
			const centre = d.muHome - d.muAway;
			const reach = Math.min(3 * d.sdMargin, 45);
			const response = await commands.analyzeGame(
				{
					muHome: d.muHome,
					muAway: d.muAway,
					sdMargin: d.sdMargin,
					sdTotal: d.sdTotal,
					keyNumbers: d.keyNumbers
				},
				d.spread,
				d.total,
				-centre - reach,
				-centre + reach,
				0.5
			);

			if (response.status === 'error') {
				error = response.error;
				return null;
			}
			return response.data;
		}
	);

	const evCurve = new Async(
		() => ({ price: priceInput }),
		async (d): Promise<EvPoint[] | null> => {
			const parsed = await parseOdds(d.price, 'american');
			if (parsed.decimal === null) return null;
			const response = await commands.evCurve(parsed.decimal, 0.0, 1.0, 0.005);
			return response.status === 'ok' ? response.data : null;
		}
	);

	const view = $derived(game.current);

	const curveSeries = $derived([
		{
			label: 'Home covers',
			data: (view?.curve ?? []).map((p) => ({ x: p.spread, y: p.homeCovers })),
			color: 'var(--accent-cyan)'
		}
	]);

	const evSeries = $derived([
		{
			label: 'EV per unit',
			data: (evCurve.current ?? []).map((p) => ({ x: p.trueProb, y: p.ev })),
			color: 'var(--accent-cyan)'
		}
	]);

	const breakeven = $derived(
		(evCurve.current ?? []).find((p) => p.ev >= 0)?.trueProb ?? null
	);
</script>

<InputCard title="The game">
	<FormRow>
		<FormGroup label="Sport" hint="Presets load a whole shape; every field stays editable">
			<ToggleGroup options={SPORT_OPTIONS} bind:value={sportKey} onchange={loadPreset} />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Home mean score">
			<input type="text" inputmode="decimal" bind:value={muHome} />
		</FormGroup>
		<FormGroup label="Away mean score">
			<input type="text" inputmode="decimal" bind:value={muAway} />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Margin SD" hint="How far apart teams finish">
			<input type="text" inputmode="decimal" bind:value={sdMargin} />
		</FormGroup>
		<FormGroup label="Total SD" hint="How much they score together">
			<input type="text" inputmode="decimal" bind:value={sdTotal} />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Spread" hint="Home side. Negative means laying points.">
			<input type="text" inputmode="decimal" bind:value={spread} />
		</FormGroup>
		<FormGroup label="Total">
			<input type="text" inputmode="decimal" bind:value={total} />
		</FormGroup>
	</FormRow>
	<label class="check">
		<input type="checkbox" bind:checked={keyNumbers} />
		<span>
			Weight margins onto football's key numbers
			<em>American football only. See the note below for what the table is and is not.</em>
		</span>
	</label>
</InputCard>

<OutputSection title="What the two lines imply">
	<ErrorNote {error} />
	{#if view}
		<ResultRow
			label="Per-team standard deviation"
			value="{num(view.model.sdTeam, 2)} {sport?.unit ?? 'points'}"
			hint="Derived from the margin and total SDs together — not an input anywhere"
		/>
		<ResultRow
			label="Correlation between the teams' scores"
			value={num(view.model.impliedCorrelation, 3)}
			color={view.model.impliedCorrelation < 0 ? 'amber' : 'blue'}
			hint={view.model.impliedCorrelation < 0
				? 'Negative: game script pulls the scores apart'
				: 'Positive: the two share a pace'}
		/>
		<ResultRow
			label="Mean margin"
			value={fmtLine(view.model.meanMargin)}
			hint={keyNumbers
				? 'Measured after key-number weighting, which pulls it toward zero'
				: 'Home mean minus away mean'}
		/>
		<ResultRow label="Mean total" value={num(view.model.meanTotal, 1)} />

		<div class="block">
			<div class="block-title">Grading the posted lines</div>
			<ResultRow
				label="Home covers {fmtLine(view.spread.spread)}"
				value={pct(view.spread.homeCovers)}
				color="highlight"
			/>
			<ResultRow label="Away covers" value={pct(view.spread.awayCovers)} />
			{#if view.spread.push > 0}
				<ResultRow
					label="Push"
					value={pct(view.spread.push)}
					color="amber"
					hint="Only possible on a whole-number line, which is why one is priced differently"
				/>
			{/if}
			<ResultRow label="Over {num(view.total.total, 1)}" value={pct(view.total.over)} />
			<ResultRow label="Under" value={pct(view.total.under)} />
			{#if view.total.push > 0}
				<ResultRow label="Total push" value={pct(view.total.push)} color="amber" />
			{/if}
		</div>

		<div class="block">
			<div class="block-title">Moneyline the model implies</div>
			<ResultRow label="Home" value={pct(view.moneyline.home)} color="positive" />
			<ResultRow label="Away" value={pct(view.moneyline.away)} />
			{#if view.moneyline.draw > 0.001}
				<ResultRow
					label="Level at the whistle"
					value={pct(view.moneyline.draw)}
					color="muted"
					hint="A regulation tie, not a settled draw — these sports play overtime"
				/>
			{/if}
		</div>

		<div class="chart-head">Margin of victory, graded against {fmtLine(view.spread.spread)}</div>
		<PmfChart
			data={view.model.margin}
			line={-view.spread.spread}
			highlight={(k) => keyNumbers && KEY_NUMBERS.has(Math.abs(k))}
			bands={[
				{
					from: -view.spread.spread,
					to: Number.POSITIVE_INFINITY,
					color: 'var(--accent-green)',
					label: 'home covers'
				}
			]}
			formatX={(v) => fmtLine(v, 0)}
		/>

		<div class="chart-head">Combined score, graded against {num(view.total.total, 1)}</div>
		<PmfChart
			data={view.model.total}
			line={view.total.total}
			bands={[
				{
					from: view.total.total,
					to: Number.POSITIVE_INFINITY,
					color: 'var(--accent-blue)',
					label: 'over'
				}
			]}
			formatX={(v) => num(v, 0)}
		/>

		<div class="chart-head">One model, every line</div>
		<LineChart
			series={curveSeries}
			height={260}
			formatX={(v) => fmtLine(v, 1)}
			formatY={(v) => pct(v, 0)}
			labelX="Home spread"
		/>
		{#if keyNumbers}
			<p class="note">
				The steps at 3 and 7 are the point. Moving a football line across a key number costs far
				more probability than moving it the same half point anywhere else, which is why books
				resist doing it and why buying that half point is priced the way it is.
			</p>
		{/if}
	{:else if !error}
		<EmptyState icon="~" message="Set a game up and the distributions follow" />
	{/if}
</OutputSection>

<OutputSection title="What a price is worth, as a function of what you believe">
	<FormRow>
		<FormGroup label="Price" hint="American">
			<input type="text" inputmode="numeric" bind:value={priceInput} placeholder="-110" />
		</FormGroup>
	</FormRow>

	{#if (evCurve.current ?? []).length > 0}
		{#if breakeven !== null}
			<ResultRow
				label="Break-even probability"
				value={pct(breakeven)}
				color="highlight"
				hint="Below this the bet loses money, however confident you are"
			/>
		{/if}
		<LineChart
			series={evSeries}
			markers={[{ value: 0, label: 'break even', color: 'var(--text-muted)', axis: 'y' }]}
			height={240}
			formatX={(v) => pct(v, 0)}
			formatY={(v) => pct(v, 0)}
			labelX="True probability you assume"
		/>
		<p class="note">
			The slope of this line is the decimal price. A longshot's expected value swings much harder
			on the same change of mind — the same fact the variance module reports as a longer
			confirmation horizon.
		</p>
	{:else}
		<EmptyState icon="/" message="Enter a price" />
	{/if}
</OutputSection>

<InfoSection title="Two lines, four hidden parameters">
	<p>
		A book posts a spread and a total. Those are statements about two different things — how far
		apart the teams finish, and how much they score together — and their standard deviations differ:
		around 13.5 and 10.2 points in the NFL. Together they pin down two quantities nobody posts:
	</p>
	<p>
		<code>σ_team = √((σ_margin² + σ_total²) / 4)</code> and
		<code>ρ = (σ_total² − σ_margin²) / (σ_total² + σ_margin²)</code>.
	</p>
	<p>
		Football lands at ρ ≈ −0.27: the two teams' scores are <em>anti</em>-correlated, which is game
		script — one side runs the clock out while the other throws. Basketball lands at +0.31, where
		pace is shared and both totals rise together. Neither number is an input; both fall out of two
		lines a book already publishes.
	</p>
	<p>
		<strong>Everything is evaluated on integers.</strong> A spread of exactly 3 is graded against a
		margin of exactly 3, so a push is a real event with a real probability rather than something a
		continuous model rounds away. That single choice is what makes whole-number and half-point lines
		price differently here, as they should.
	</p>
	<p>
		<strong>What the key-number table is.</strong> Football margins pile up on 3 and 7 and a normal
		cannot see it, so the margin distribution is reweighted and renormalised. With it on, a 3-point
		margin carries 12.7% and a 7 carries 8.4%, against the roughly 15% and 9% usually quoted for the
		NFL — much closer than a smooth normal manages, and still not the observed rates. It is a shape
		correction, not a fitted distribution, and it is one sport's. Applying it to basketball would be
		nonsense, which is why the toggle only appears meaningful for football.
	</p>
	<p>
		<strong>A caveat the weighting introduces.</strong> The weights are applied to the absolute
		margin and are heaviest near the middle, so on a distribution that is not centred on zero they
		pull the mean in: a game set up at -3.5 prices out around -3.2 once key numbers are on. The
		mean margin above is measured from the distribution rather than echoed back from your input,
		so you can see it happen.
	</p>
	<p>
		Low-scoring sports are not offered here. Hockey and soccer are Poisson-shaped, and a normal
		misprices them badly at those rates — the
		<a href="/calculators/poisson-match">Poisson match predictor</a> is the right screen for those.
	</p>
</InfoSection>

<style>
	.check {
		display: flex;
		align-items: flex-start;
		gap: 0.6rem;
		margin-top: 1rem;
		font-size: 0.8rem;
		color: var(--text-secondary);
		cursor: pointer;
	}

	.check input {
		width: auto;
		margin-top: 0.2rem;
	}

	.check em {
		display: block;
		font-style: normal;
		font-size: 0.72rem;
		color: var(--text-muted);
		margin-top: 0.15rem;
	}

	.block {
		margin-top: 1.25rem;
		padding-top: 1.25rem;
		border-top: 1px solid var(--border);
	}

	.block-title,
	.chart-head {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		margin-bottom: 0.75rem;
	}

	.chart-head {
		margin-top: 1.75rem;
	}

	.note {
		margin-top: 0.75rem;
		font-size: 0.78rem;
		color: var(--text-muted);
		line-height: 1.65;
	}
</style>
