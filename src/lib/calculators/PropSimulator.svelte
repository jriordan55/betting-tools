<script lang="ts">
	import {
		commands,
		type MathError,
		type PropSimulation,
		type PropSport,
		type PropStat
	} from '$lib/bindings';
	import { numeric, positive } from '$lib/async.svelte';
	import { sportConfig } from '$lib/config';
	import { fairOdds, type FairOdds } from '$lib/odds';
	import { american, count, num, pct } from '$lib/format';
	import Histogram from '$lib/charts/Histogram.svelte';
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

	const SIM_OPTIONS = [
		{ value: '1000', label: '1,000' },
		{ value: '10000', label: '10,000' },
		{ value: '50000', label: '50,000' }
	];

	const DISTRIBUTION_LABELS: Record<string, string> = {
		poisson: 'Poisson',
		nbinom: 'Negative Binomial',
		gamma: 'Gamma',
		lognormal: 'Lognormal'
	};

	/** Rungs either side of the posted line in the fair-odds ladder. */
	const LADDER_RUNGS = 8;

	let sports = $state<PropSport[]>([]);
	let sportKey = $state('nfl');
	let positionKey = $state('qb');
	let statKey = $state('passing_yards');

	let projection = $state('');
	let lineValue = $state('');
	let varMultiplier = $state('2');
	let simCount = $state('10000');
	let ladderStep = $state('0.5');
	let seedInput = $state('');

	let simulation = $state<PropSimulation | null>(null);
	let overFair = $state<FairOdds | null>(null);
	let underFair = $state<FairOdds | null>(null);
	interface LadderRung {
		line: number;
		over: number;
		under: number;
		overAmerican: number;
		underAmerican: number;
	}

	let ladder = $state<LadderRung[]>([]);
	let error = $state<MathError | string | null>(null);
	let running = $state(false);

	$effect(() => {
		sportConfig().then((config) => {
			sports = config.props;
		});
	});

	const sport = $derived(sports.find((s) => s.key === sportKey) ?? null);
	const position = $derived(
		sport?.positions.find((p) => p.key === positionKey) ?? sport?.positions[0] ?? null
	);
	const stat = $derived<PropStat | null>(
		position?.stats.find((s) => s.key === statKey) ?? position?.stats[0] ?? null
	);

	let lastStatId = $state('');
	$effect(() => {
		if (!sport || !position || !stat) return;
		const id = `${sport.key}/${position.key}/${stat.key}`;
		if (id === lastStatId) return;
		lastStatId = id;
		positionKey = position.key;
		statKey = stat.key;
		projection = String(stat.defaultProjection);
		lineValue = String(stat.defaultLine);

		// Results belong to the stat that produced them. The labels, units and
		// line all follow the dropdown immediately, so leaving the previous
		// run on screen relabels a yardage histogram as touchdowns and puts a
		// mean of 250 under "Passing TDs".
		simulation = null;
		ladder = [];
		overFair = null;
		underFair = null;
		error = null;
	});

	async function run() {
		if (!stat) return;
		running = true;
		error = null;

		try {
			const mu = positive(projection);
			const line = numeric(lineValue);
			const sims = positive(simCount);
			const step = positive(ladderStep);
			const variance = positive(varMultiplier) ?? 2;
			if (mu === null || line === null || sims === null || step === null) {
				error = 'Projection, simulation count and ladder step must be greater than 0.';
				return;
			}

			const seed = seedInput.trim() === '' ? null : seedInput.trim();
			const response = await commands.simulateProp(
				sims,
				stat.distribution,
				mu,
				variance,
				line,
				seed
			);
			if (response.status === 'error') {
				error = response.error;
				simulation = null;
				return;
			}

			simulation = response.data;
			seedInput = response.data.seed;

			[overFair, underFair] = await Promise.all([
				fairOdds(response.data.overProb),
				fairOdds(response.data.underProb)
			]);

			// Every rung re-runs on the same seed, so the ladder is drawn from one
			// consistent sample rather than a fresh draw per line.
			const rungs: LadderRung[] = [];
			for (let i = -LADDER_RUNGS; i <= LADDER_RUNGS; i++) {
				const rung = line + i * step;
				if (rung < 0) continue;
				const at = await commands.simulateProp(
					sims,
					stat.distribution,
					mu,
					variance,
					rung,
					response.data.seed
				);
				if (at.status !== 'ok') continue;

				rungs.push({
					line: rung,
					over: at.data.overProb,
					under: at.data.underProb,
					// The core already converted both sides for this rung.
					overAmerican: at.data.overFairOdds,
					underAmerican: at.data.underFairOdds
				});
			}
			ladder = rungs;
		} finally {
			running = false;
		}
	}

	function reroll() {
		seedInput = '';
		run();
	}

	const posted = $derived(numeric(lineValue));
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
		<FormGroup label="Position">
			<select bind:value={positionKey}>
				{#each sport?.positions ?? [] as p (p.key)}
					<option value={p.key}>{p.label}</option>
				{/each}
			</select>
		</FormGroup>
		<FormGroup label="Stat">
			<select bind:value={statKey}>
				{#each position?.stats ?? [] as s (s.key)}
					<option value={s.key}>{s.label}</option>
				{/each}
			</select>
		</FormGroup>
	</FormRow>
</InputCard>

{#if stat}
	<InputCard title="Inputs">
		<FormRow>
			<FormGroup label="Projection ({stat.unit})" hint="Your mean expectation">
				<input type="text" inputmode="decimal" bind:value={projection} />
			</FormGroup>
			<FormGroup label="Posted Line" hint="The number the book is offering">
				<input type="text" inputmode="decimal" bind:value={lineValue} />
			</FormGroup>
		</FormRow>
		<FormRow>
			<FormGroup
				label="Variance Multiplier"
				hint="Variance as a multiple of the mean. Ignored by Poisson, which fixes it at 1."
			>
				<input type="text" inputmode="decimal" bind:value={varMultiplier} />
			</FormGroup>
			<FormGroup label="Ladder Step" hint="Spacing of the fair-odds table">
				<input type="text" inputmode="decimal" bind:value={ladderStep} />
			</FormGroup>
		</FormRow>
		<FormRow>
			<FormGroup label="Simulations">
				<ToggleGroup options={SIM_OPTIONS} bind:value={simCount} />
			</FormGroup>
			<FormGroup label="Seed" hint="Blank for a fresh run; paste one to reproduce it">
				<input type="text" inputmode="numeric" bind:value={seedInput} placeholder="(random)" />
			</FormGroup>
		</FormRow>
		<div class="actions">
			<button type="button" class="run" onclick={run} disabled={running}>
				{running ? 'Simulating…' : 'Run simulation'}
			</button>
			<button type="button" onclick={reroll} disabled={running}>New seed</button>
		</div>
		<p class="dist-note">
			{stat.label} is simulated as <strong>{DISTRIBUTION_LABELS[stat.distribution]}</strong> —
			{stat.distribution === 'poisson'
				? 'counts whose variance equals their mean.'
				: stat.distribution === 'nbinom'
					? 'counts that are overdispersed relative to Poisson.'
					: stat.distribution === 'gamma'
						? 'a continuous right-skewed quantity.'
						: 'a continuous quantity with a heavy right tail.'}
		</p>
	</InputCard>
{/if}

<OutputSection title="Simulation">
	<ErrorNote {error} />
	{#if simulation && stat}
		<div class="split">
			<ResultLarge
				value={pct(simulation.overProb, 1)}
				label="Over {lineValue}"
				color="positive"
			/>
			<ResultLarge value={pct(simulation.underProb, 1)} label="Under {lineValue}" color="negative" />
		</div>

		<ResultRow
			label="Fair odds — over"
			value={overFair ? `${american(overFair.american)}` : '—'}
			color="highlight"
		/>
		<ResultRow
			label="Fair odds — under"
			value={underFair ? `${american(underFair.american)}` : '—'}
			color="highlight"
		/>
		<ResultRow label="Simulated mean" value="{num(simulation.stats.mean, 2)} {stat.unit}" />
		<ResultRow
			label="Simulated standard deviation"
			value="{num(simulation.stats.stdDev, 2)} {stat.unit}"
		/>

		<Histogram
			bins={simulation.histogram}
			markers={posted !== null
				? [{ value: posted, label: `line ${posted}`, color: 'var(--accent-amber)' }]
				: []}
			formatX={(v) => num(v, 0)}
			formatY={(v) => pct(v, 0)}
			labelX={stat.unit}
			colorFor={(bin) =>
				posted !== null && bin.rangeStart >= posted ? 'var(--accent-green)' : 'var(--accent-blue)'}
		/>

		<p class="seed">
			{count(Number(simCount))} samples · seed <code data-selectable>{simulation.seed}</code>
		</p>
	{:else if !error}
		<EmptyState icon="~" message="Choose a stat and run the simulation" />
	{/if}
</OutputSection>

{#if ladder.length > 0 && stat}
	<OutputSection title="Fair Odds Ladder">
		<div class="table-wrap">
			<table>
				<thead>
					<tr>
						<th>Line</th>
						<th>Over</th>
						<th>Under</th>
						<th>Over odds</th>
						<th>Under odds</th>
					</tr>
				</thead>
				<tbody>
					{#each ladder as rung (rung.line)}
						<tr class:current={posted !== null && Math.abs(rung.line - posted) < 1e-9}>
							<td class="line-cell">{num(rung.line, 1)}</td>
							<td>{pct(rung.over, 1)}</td>
							<td>{pct(rung.under, 1)}</td>
							<td class="odds">{american(rung.overAmerican)}</td>
							<td class="odds">{american(rung.underAmerican)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		<p class="note">Every rung is drawn from the same seeded sample, so the ladder is internally consistent.</p>
	</OutputSection>
{/if}

<InfoSection title="About Prop Simulation">
	<p>
		A projection is a mean, not an outcome. The distribution around it is what decides whether an
		over at a given line is worth taking — and the right distribution depends on the stat. Counts
		with variance equal to their mean are Poisson; counts that cluster and burst are negative
		binomial; yardage is continuous and right-skewed.
	</p>
	<p>
		<strong>The web version's Poisson sampler was a rounded normal above λ=30</strong>, while its
		doc comment claimed Ahrens-Dieter rejection sampling. A normal is symmetric and a Poisson is
		right-skewed, so the approximation flattened exactly the tail a prop line sits in. There is now a
		regression test asserting skew ≈ 1/√λ at λ=50.
	</p>
	<p>
		A lognormal with a non-positive μ used to return an array of zeros, which charts as certainty
		at zero. It is an error now.
	</p>
</InfoSection>

<style>
	.actions {
		display: flex;
		gap: 0.5rem;
		margin-top: 1rem;
	}

	.run {
		flex: 1;
		color: var(--accent-cyan);
		font-weight: 600;
	}

	.dist-note {
		margin-top: 0.85rem;
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.split {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
	}

	.table-wrap {
		overflow-x: auto;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-family: var(--font-mono);
		font-size: 0.8rem;
	}

	th {
		text-align: right;
		font-weight: 600;
		font-size: 0.68rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		padding: 0.4rem 0.5rem;
		border-bottom: 1px solid var(--border);
	}

	td {
		text-align: right;
		padding: 0.35rem 0.5rem;
		border-bottom: 1px solid var(--border);
	}

	tbody tr:last-child td {
		border-bottom: none;
	}

	tr.current {
		background: var(--bg-tertiary);
	}

	tr.current .line-cell {
		color: var(--accent-cyan);
		font-weight: 700;
	}

	.odds {
		color: var(--text-secondary);
	}

	.seed,
	.note {
		margin-top: 0.75rem;
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.seed code {
		font-family: var(--font-mono);
		color: var(--accent-cyan);
	}
</style>
