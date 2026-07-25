<script lang="ts">
	import {
		commands,
		type ConvergencePoint,
		type MathError,
		type Regressed,
		type RegressionSport,
		type RegressionStat
	} from '$lib/bindings';
	import { Async, numeric, positive } from '$lib/async.svelte';
	import { sportConfig } from '$lib/config';
	import { count, num, pct } from '$lib/format';
	import LineChart from '$lib/charts/LineChart.svelte';
	import {
		InputCard,
		OutputSection,
		FormRow,
		FormGroup,
		ResultRow,
		ResultLarge,
		EmptyState,
		InfoSection,
		ErrorNote
	} from '$lib/ui';

	let sports = $state<RegressionSport[]>([]);
	let sportKey = $state('mlb');
	let statKey = $state('avg');

	let observed = $state('');
	let sampleSize = $state('');
	let baseline = $state('');
	let customConstant = $state('');
	let useCustomConstant = $state(false);

	$effect(() => {
		sportConfig().then((config) => {
			sports = config.regression;
		});
	});

	const sport = $derived(sports.find((s) => s.key === sportKey) ?? null);
	const stat = $derived<RegressionStat | null>(
		sport?.stats.find((s) => s.key === statKey) ?? sport?.stats[0] ?? null
	);

	// Switching sport or stat reloads that preset's illustrative numbers.
	let lastStatId = $state('');
	$effect(() => {
		if (!sport || !stat) return;
		const id = `${sport.key}/${stat.key}`;
		if (id === lastStatId) return;
		lastStatId = id;
		statKey = stat.key;
		observed = String(stat.defaultObserved);
		sampleSize = String(stat.defaultSampleSize);
		baseline = String(stat.leagueAverage);
		customConstant = String(stat.regressionConstant);
	});

	const constant = $derived(
		useCustomConstant ? positive(customConstant) : (stat?.regressionConstant ?? null)
	);

	/** Rates below 1.0 read as percentages; counting stats do not. */
	const isRate = $derived((stat?.leagueAverage ?? 0) < 1);

	function fmt(value: number) {
		return isRate ? pct(value, 1) : num(value, 3);
	}

	interface RegressionView {
		regressed: Regressed;
		convergence: ConvergencePoint[];
		observed: number;
		baseline: number;
		sampleSize: number;
	}

	const result = new Async<
		{ observed: string; sampleSize: string; baseline: string; constant: number | null },
		{ data: RegressionView | null; error: MathError | string | null }
	>(
		() => ({ observed, sampleSize, baseline, constant }),
		async ({ observed, sampleSize, baseline, constant }) => {
			if (constant === null) {
				return { data: null, error: 'The regression constant must be greater than 0.' };
			}
			const obs = numeric(observed);
			const base = numeric(baseline);
			const n = positive(sampleSize);
			if (obs === null || base === null) return { data: null, error: null };
			if (n === null) {
				return {
					data: null,
					error: sampleSize.trim() === '' ? null : 'Sample size must be greater than 0.'
				};
			}

			const regressed = await commands.regress(obs, base, n, constant);
			if (regressed.status === 'error') return { data: null, error: regressed.error };

			// Out to twice the constant, which is where the curve has flattened.
			const series = await commands.convergenceSeries(obs, base, constant, constant * 2, 100);
			if (series.status === 'error') return { data: null, error: series.error };

			return {
				data: {
					regressed: regressed.data,
					convergence: series.data,
					observed: obs,
					baseline: base,
					sampleSize: n
				},
				error: null
			};
		}
	);
</script>

<InputCard title="Sport & Stat">
	<FormRow>
		<FormGroup label="Sport">
			<select bind:value={sportKey}>
				{#each sports as s (s.key)}
					<option value={s.key}>{s.label}</option>
				{/each}
			</select>
		</FormGroup>
		<FormGroup label="Stat">
			<select bind:value={statKey}>
				{#each sport?.stats ?? [] as s (s.key)}
					<option value={s.key}>{s.label}</option>
				{/each}
			</select>
		</FormGroup>
	</FormRow>
</InputCard>

{#if stat}
	<InputCard title="Inputs">
		<FormRow min={160}>
			<FormGroup
				label="Observed {stat.unit}"
				hint="The player's rate over the sample so far"
			>
				<input type="text" inputmode="decimal" bind:value={observed} />
			</FormGroup>
			<FormGroup label="Sample size ({stat.sampleUnit})" hint="Opportunities observed">
				<input type="text" inputmode="numeric" bind:value={sampleSize} />
			</FormGroup>
			<FormGroup label="League average" hint="The population baseline">
				<input type="text" inputmode="decimal" bind:value={baseline} />
			</FormGroup>
		</FormRow>
		<div class="custom">
			<label class="toggle">
				<input type="checkbox" bind:checked={useCustomConstant} />
				<span>Override the regression constant</span>
			</label>
			{#if useCustomConstant}
				<FormGroup
					label="Regression constant"
					hint="Default for {stat.label}: {stat.regressionConstant} {stat.sampleUnit}"
				>
					<input type="text" inputmode="numeric" bind:value={customConstant} />
				</FormGroup>
			{/if}
		</div>
	</InputCard>
{/if}

<OutputSection title="Regressed Estimate">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data && stat}
		{@const view = result.current.data}
		<ResultLarge
			value={fmt(view.regressed.estimate)}
			label="True talent estimate ({stat.unit})"
			color="highlight"
		/>

		<div class="weight">
			<div class="weight-labels">
				<span>League average {pct(1 - view.regressed.weight, 0)}</span>
				<span>Observed {pct(view.regressed.weight, 0)}</span>
			</div>
			<div class="weight-bar">
				<div class="weight-baseline" style="width: {(1 - view.regressed.weight) * 100}%"></div>
				<div class="weight-observed" style="width: {view.regressed.weight * 100}%"></div>
			</div>
		</div>

		<ResultRow
			label="Weight on the observed rate"
			value={pct(view.regressed.weight)}
			color="amber"
			hint="n / (n + k) — the rest goes to the baseline"
		/>
		<ResultRow
			label="90% confidence interval"
			value="{fmt(view.regressed.lower)} — {fmt(view.regressed.upper)}"
			color="highlight"
		/>
		<ResultRow
			label="Regression constant"
			value="{count(stat.regressionConstant)} {stat.sampleUnit}"
			color="blue"
			hint="The sample size at which observed and baseline carry equal weight"
		/>
		<ResultRow label="Observed" value={fmt(view.observed)} color="amber" />
		<ResultRow label="League average" value={fmt(view.baseline)} color="blue" />

		<div class="chart-head">Convergence curve</div>
		<LineChart
			data={view.convergence.map((p) => ({ x: p.sampleSize, y: p.regressed }))}
			markers={[
				{ value: view.observed, label: `obs ${fmt(view.observed)}`, color: 'var(--accent-amber)', axis: 'y' },
				{ value: view.baseline, label: `avg ${fmt(view.baseline)}`, color: 'var(--accent-blue)', axis: 'y' },
				{ value: view.sampleSize, label: `n=${count(view.sampleSize)}`, color: 'var(--accent-red)', axis: 'x' }
			]}
			formatX={(v) => count(Math.round(v))}
			formatY={fmt}
			labelX="Sample size ({stat.sampleUnit})"
			labelY={stat.unit}
		/>
	{:else if !result.current?.error}
		<EmptyState icon="R" message="Enter an observed rate and a sample size" />
	{/if}
</OutputSection>

<InfoSection title="About Regression to the Mean">
	<p>
		Small samples are noisy. A hitter batting .400 through 50 plate appearances is not a .400
		hitter; most of that is luck. Regression blends the observed rate toward the league average,
		weighted by how much data there is:
		<code>regressed = baseline + (observed − baseline) × n / (n + k)</code>.
	</p>
	<p>
		<strong>k is the sample size at which the two carry equal weight</strong>, and it varies enormously
		by stat. BABIP (k=820) and batting average (k=910) take most of a season to say anything; strikeout
		rate (k=60) stabilises in weeks. Early-season props are mispriced precisely because the market
		overreacts to samples smaller than k.
	</p>
	<p>
		A note on the curve's x-axis: the web version accumulated a floating-point step and rounded it,
		so whenever <code>maxSample/100</code> was not a whole number the points came out unevenly
		spaced — a max of 150 gave 0, 2, 3, 5, 6, 8, 9, 11, alternating gaps of 2 and 1. The spacing is
		even here.
	</p>
</InfoSection>

<style>
	.custom {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border);
	}

	.toggle {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.8rem;
		color: var(--text-secondary);
		cursor: pointer;
		margin-bottom: 0.75rem;
	}

	.toggle input {
		width: auto;
	}

	.weight {
		margin: 1rem 0 1.25rem;
	}

	.weight-labels {
		display: flex;
		justify-content: space-between;
		font-size: 0.72rem;
		color: var(--text-muted);
		margin-bottom: 0.3rem;
	}

	.weight-bar {
		display: flex;
		height: 10px;
		border-radius: 5px;
		overflow: hidden;
		background: var(--bg-tertiary);
	}

	.weight-baseline {
		background: var(--accent-blue);
	}

	.weight-observed {
		background: var(--accent-amber);
	}

	.chart-head {
		margin-top: 1.5rem;
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
	}
</style>
