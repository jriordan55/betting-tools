<script lang="ts">
	import {
		commands,
		type MatchMarkets,
		type MatchModel,
		type MatchSport,
		type MathError
	} from '$lib/bindings';
	import { Async, positive } from '$lib/async.svelte';
	import { sportConfig } from '$lib/config';
	import { fairOdds, type FairOdds } from '$lib/odds';
	import { american, line as fmtLine, num, pct } from '$lib/format';
	import {
		InputCard,
		OutputSection,
		FormRow,
		FormGroup,
		ResultRow,
		EmptyState,
		InfoSection,
		ErrorNote
	} from '$lib/ui';

	let { overdispersed = false }: { overdispersed?: boolean } = $props();

	let sports = $state<MatchSport[]>([]);
	let sportKey = $state('soccer');
	let homeRate = $state('');
	let awayRate = $state('');
	let rHome = $state('');
	let rAway = $state('');
	let maxScore = $state('');

	$effect(() => {
		sportConfig().then((config) => {
			sports = config.matches;
		});
	});

	const sport = $derived(sports.find((s) => s.key === sportKey) ?? null);

	let lastSport = $state('');
	$effect(() => {
		if (!sport || sport.key === lastSport) return;
		lastSport = sport.key;
		homeRate = String(sport.defaultHome);
		awayRate = String(sport.defaultAway);
		rHome = String(sport.defaultR);
		rAway = String(sport.defaultR);
		maxScore = String(sport.maxScore);
	});

	interface MatchView {
		markets: MatchMarkets;
		homeFair: FairOdds;
		drawFair: FairOdds | null;
		awayFair: FairOdds;
	}

	const result = new Async<
		{
			sport: MatchSport | null;
			homeRate: string;
			awayRate: string;
			rHome: string;
			rAway: string;
			maxScore: string;
			overdispersed: boolean;
		},
		{ data: MatchView | null; error: MathError | string | null }
	>(
		() => ({ sport, homeRate, awayRate, rHome, rAway, maxScore, overdispersed }),
		async ({ sport, homeRate, awayRate, rHome, rAway, maxScore, overdispersed }) => {
			if (!sport) return { data: null, error: null };

			const home = positive(homeRate);
			const away = positive(awayRate);
			const grid = positive(maxScore);
			if (home === null || away === null || grid === null) return { data: null, error: null };

			let model: MatchModel;
			if (overdispersed) {
				const dispersionHome = positive(rHome);
				const dispersionAway = positive(rAway);
				if (dispersionHome === null || dispersionAway === null) {
					return { data: null, error: 'The dispersion parameter r must be greater than 0.' };
				}
				model = {
					kind: 'negativeBinomial',
					mean_home: home,
					mean_away: away,
					r_home: dispersionHome,
					r_away: dispersionAway
				};
			} else {
				model = { kind: 'poisson', lambda_home: home, lambda_away: away };
			}

			const markets = await commands.deriveMarkets(
				model,
				grid,
				sport.spreadLines,
				sport.totalLines,
				sport.allowDraw
			);
			if (markets.status === 'error') return { data: null, error: markets.error };

			const probs = markets.data.markets;
			const [homeFair, awayFair] = await Promise.all([
				fairOdds(probs.homeWin),
				fairOdds(probs.awayWin)
			]);
			const drawFair = sport.allowDraw && probs.draw > 0 ? await fairOdds(probs.draw) : null;

			return { data: { markets: markets.data, homeFair, drawFair, awayFair }, error: null };
		}
	);

	function fairText(fair: FairOdds | null) {
		if (!fair) return '—';
		return fair.decimal === null
			? american(fair.american)
			: `${american(fair.american)} · ${num(fair.decimal, 2)}`;
	}
</script>

<InputCard title="Model">
	<FormRow>
		<FormGroup label="Sport">
			<select bind:value={sportKey}>
				{#each sports as s (s.key)}
					<option value={s.key}>{s.label}</option>
				{/each}
			</select>
		</FormGroup>
		<FormGroup label="Grid Size" hint="Largest score the matrix runs to">
			<input type="text" inputmode="numeric" bind:value={maxScore} />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Home {sport?.scoreLabel ?? 'Score'}" hint="Expected, not a whole number">
			<input type="text" inputmode="decimal" bind:value={homeRate} />
		</FormGroup>
		<FormGroup label="Away {sport?.scoreLabel ?? 'Score'}" hint="Expected">
			<input type="text" inputmode="decimal" bind:value={awayRate} />
		</FormGroup>
	</FormRow>
	{#if overdispersed}
		<FormRow>
			<FormGroup label="Home dispersion (r)" hint={sport?.rHint}>
				<input type="text" inputmode="decimal" bind:value={rHome} />
			</FormGroup>
			<FormGroup label="Away dispersion (r)" hint="Smaller r means more variance">
				<input type="text" inputmode="decimal" bind:value={rAway} />
			</FormGroup>
		</FormRow>
	{/if}
</InputCard>

<OutputSection title="Match Result">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const view = result.current.data}
		{@const probs = view.markets.markets}
		<ResultRow
			label="Home win"
			value="{pct(probs.homeWin)} · {fairText(view.homeFair)}"
			color="highlight"
		/>
		{#if sport?.allowDraw}
			<ResultRow label="Draw" value="{pct(probs.draw)} · {fairText(view.drawFair)}" color="amber" />
		{/if}
		<ResultRow
			label="Away win"
			value="{pct(probs.awayWin)} · {fairText(view.awayFair)}"
			color="highlight"
		/>

		<ResultRow
			label="Probability mass off the grid"
			value={pct(view.markets.truncationMass, 3)}
			color={view.markets.truncationMass > 0.005 ? 'amber' : 'muted'}
			hint="Scores above the grid size. Raise it until this is negligible."
		/>
	{:else if !result.current?.error}
		<EmptyState icon="P" message="Set the scoring rates to derive the markets" />
	{/if}
</OutputSection>

{#if result.current?.data}
	{@const probs = result.current.data.markets.markets}

	<OutputSection title="Most Likely Scorelines">
		<div class="table-wrap">
			<table>
				<thead>
					<tr>
						<th class="left">Scoreline</th>
						<th>Probability</th>
					</tr>
				</thead>
				<tbody>
					{#each probs.topScorelines as score (`${score.home}-${score.away}`)}
						<tr>
							<td class="left">{score.home} – {score.away}</td>
							<td>{pct(score.prob, 2)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</OutputSection>

	<OutputSection title="Spreads">
		<div class="table-wrap">
			<table>
				<thead>
					<tr>
						<th class="left">Line</th>
						<th>Home covers</th>
						<th>Away covers</th>
						<th>Push</th>
					</tr>
				</thead>
				<tbody>
					{#each probs.spreads as spread (spread.spread)}
						<tr>
							<td class="left">{fmtLine(spread.spread)}</td>
							<td>{pct(spread.homeCovers)}</td>
							<td>{pct(spread.awayCovers)}</td>
							<td class="push">{spread.push > 0 ? pct(spread.push) : '—'}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</OutputSection>

	<OutputSection title="Totals">
		<div class="table-wrap">
			<table>
				<thead>
					<tr>
						<th class="left">Line</th>
						<th>Over</th>
						<th>Under</th>
						<th>Push</th>
					</tr>
				</thead>
				<tbody>
					{#each probs.totals as total (total.line)}
						<tr>
							<td class="left">{num(total.line, 1)}</td>
							<td>{pct(total.over)}</td>
							<td>{pct(total.under)}</td>
							<td class="push">{total.push > 0 ? pct(total.push) : '—'}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</OutputSection>
{/if}

<InfoSection title={overdispersed ? 'About the Negative Binomial Model' : 'About the Poisson Model'}>
	{#if overdispersed}
		<p>
			Poisson fixes variance equal to the mean. Real scoring is often more volatile than that —
			baseball has big innings, hockey has power plays, and a blowout-prone matchup produces a wider
			spread of results than Poisson allows. The negative binomial adds a dispersion parameter
			<code>r</code>: as <code>r</code> grows it converges on Poisson, and as it shrinks the tails
			fatten.
		</p>
		<p>
			This matters most on the alternate lines and the big-total overs, which is exactly where a
			Poisson model prices too confidently.
		</p>
	{:else}
		<p>
			Independent Poisson scoring for each side gives a full matrix of scorelines, and every
			moneyline, spread and total follows by summing the cells that win. It is the standard soccer
			model and works well where scoring events are roughly independent.
		</p>
		<p>
			If the sport has bursty scoring, use the negative binomial predictor instead — Poisson's
			variance is locked to its mean and will understate the tails.
		</p>
	{/if}
	<p>
		<strong>Two fixes carried over from the port.</strong> The web version never renormalised the
		matrix after truncating it at the grid size, so whatever mass fell off the edge biased every
		derived price low. How much that is depends entirely on the grid: at baseball rates (4.5 and
		4.2) a grid of 10 loses about 1.1%, while the default of 16 loses under 0.001%. The figure is
		reported directly above so it can be checked rather than assumed — if it climbs above a
		fraction of a percent, raise the grid size.
	</p>
	<p>
		And pushes were silently dropped: <code>margin &gt; spread</code> went to home,
		<code>margin &lt; spread</code> to away, and an exact tie went nowhere, so on whole-number lines
		the two sides did not sum to 1. Pushes have their own column here.
	</p>
</InfoSection>

<style>
	.table-wrap {
		overflow-x: auto;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-family: var(--font-mono);
		font-size: 0.82rem;
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

	.left {
		text-align: left;
	}

	.push {
		color: var(--text-muted);
	}
</style>
