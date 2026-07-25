<script lang="ts">
	import { commands, type LineSport, type MathError, type Teaser } from '$lib/bindings';
	import { Async, numeric } from '$lib/async.svelte';
	import { sportConfig } from '$lib/config';
	import { american, line as fmtLine, pct, pctSigned, points, signColor } from '$lib/format';
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

	const MIN_POINTS = -5;
	const MAX_POINTS = 10;
	const STEP = 0.5;
	const MAX_LEGS = 6;

	interface Leg {
		spread: string;
		odds: string;
		opposing: string;
	}

	let sports = $state<LineSport[]>([]);
	let sportKey = $state('nfl');
	let teaserPoints = $state(6);
	let bookPayout = $state('');
	let legs = $state<Leg[]>([
		{ spread: '', odds: '-110', opposing: '-110' },
		{ spread: '', odds: '-110', opposing: '-110' }
	]);

	$effect(() => {
		sportConfig().then((config) => {
			sports = config.lines;
		});
	});

	// Teasers move a spread, so only sports that post spreads apply.
	const spreadSports = $derived(sports.filter((s) => s.markets.includes('spread')));
	const sport = $derived(spreadSports.find((s) => s.key === sportKey) ?? null);
	const stdDev = $derived(sport?.spreadStd ?? null);

	$effect(() => {
		if (spreadSports.length > 0 && !spreadSports.some((s) => s.key === sportKey)) {
			sportKey = spreadSports[0].key;
		}
	});

	const result = new Async<
		{ legs: Leg[]; teaserPoints: number; bookPayout: string; stdDev: number | null },
		{ data: Teaser | null; error: MathError | string | null }
	>(
		() => ({ legs: legs.map((l) => ({ ...l })), teaserPoints, bookPayout, stdDev }),
		async ({ legs, teaserPoints, bookPayout, stdDev }) => {
			if (stdDev === null) return { data: null, error: null };
			if (bookPayout.trim() === '') return { data: null, error: null };

			const payout = await commands.toDecimal(bookPayout, 'american');
			if (payout.status === 'error') return { data: null, error: payout.error };

			const parsed = [];
			for (const leg of legs) {
				const spread = numeric(leg.spread);
				const price = numeric(leg.odds);
				const opposing = numeric(leg.opposing);
				if (spread === null || price === null) return { data: null, error: null };
				if (opposing === null) {
					return {
						data: null,
						error: 'Each leg needs the opposing price so the vig can be removed before teasing.'
					};
				}

				const fair = await commands.fairCoverProb(price, opposing);
				if (fair.status === 'error') return { data: null, error: fair.error };
				parsed.push({ spread, fairCoverProb: fair.data });
			}

			const teaser = await commands.analyzeTeaser(parsed, teaserPoints, payout.data, stdDev);
			return teaser.status === 'ok'
				? { data: teaser.data, error: null }
				: { data: null, error: teaser.error };
		}
	);

	function step(direction: -1 | 1) {
		const next = teaserPoints + direction * STEP;
		teaserPoints = Math.min(MAX_POINTS, Math.max(MIN_POINTS, next));
	}

	function addLeg() {
		if (legs.length < MAX_LEGS) legs = [...legs, { spread: '', odds: '-110', opposing: '-110' }];
	}

	function removeLeg(index: number) {
		if (legs.length > 2) legs = legs.filter((_, i) => i !== index);
	}
</script>

<InputCard title="Settings">
	<FormRow>
		<FormGroup label="Sport" hint={stdDev !== null ? `σ = ${stdDev}` : undefined}>
			<select bind:value={sportKey}>
				{#each spreadSports as s (s.key)}
					<option value={s.key}>{s.label}</option>
				{/each}
			</select>
		</FormGroup>
		<FormGroup label="Teaser Points">
			<div class="stepper">
				<button type="button" onclick={() => step(-1)} disabled={teaserPoints <= MIN_POINTS}>−</button>
				<span class="step-value">{fmtLine(teaserPoints)}</span>
				<button type="button" onclick={() => step(1)} disabled={teaserPoints >= MAX_POINTS}>+</button>
			</div>
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Book's Payout" hint="American price the book offers for the teaser">
			<input type="text" bind:value={bookPayout} placeholder="-120" />
		</FormGroup>
	</FormRow>
</InputCard>

<InputCard title="Teaser Legs ({legs.length})">
	{#each legs as leg, index (index)}
		<div class="leg">
			<span class="leg-number">#{index + 1}</span>
			<div class="leg-inputs">
				<input type="text" bind:value={leg.spread} placeholder="Spread, e.g. -7.5" />
				<input type="text" bind:value={leg.odds} placeholder="Price, e.g. -110" />
				<input type="text" bind:value={leg.opposing} placeholder="Opposing, e.g. -110" />
			</div>
			<button
				type="button"
				class="remove"
				onclick={() => removeLeg(index)}
				disabled={legs.length <= 2}
			>
				Remove
			</button>
		</div>
	{/each}
	{#if legs.length < MAX_LEGS}
		<button type="button" class="add" onclick={addLeg}>+ Add Leg</button>
	{/if}
</InputCard>

<OutputSection title="Leg Analysis">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const teaser = result.current.data}
		<div class="table-wrap">
			<table>
				<thead>
					<tr>
						<th class="left">Spread</th>
						<th class="left">Key numbers crossed</th>
						<th>Cover</th>
						<th>Fair odds</th>
					</tr>
				</thead>
				<tbody>
					{#each teaser.legs as leg, index (index)}
						<tr>
							<td class="left">
								{fmtLine(leg.spread)} <span class="arrow">→</span>
								<strong>{fmtLine(leg.teasedSpread)}</strong>
							</td>
							<td class="left">
								{#if leg.keyNumbersCrossed.length > 0}
									{#each leg.keyNumbersCrossed as key (key)}
										<span class="key-badge">{key}</span>
									{/each}
								{:else}
									<span class="none">—</span>
								{/if}
							</td>
							<td>{pct(leg.coverProb, 1)}</td>
							<td>{american(leg.fairOdds)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{:else if !result.current?.error}
		<EmptyState icon="T" message="Enter each leg's spread and both sides of its price" />
	{/if}
</OutputSection>

<OutputSection title="Fair vs Book">
	{#if result.current?.data}
		{@const teaser = result.current.data}
		<ResultLarge
			value={pctSigned(teaser.evFraction)}
			label="Expected Value"
			color={signColor(teaser.evFraction)}
		/>
		<ResultRow label="Fair teaser odds" value={american(teaser.fairOdds)} color="highlight" />
		<ResultRow label="Combined cover probability" value={pct(teaser.combinedProb)} />
		<ResultRow
			label="Break-even probability"
			value={pct(teaser.breakeven)}
			hint="What the book's price requires you to hit"
		/>

		{#if teaser.modelIgnoresKeyNumbers}
			<div class="caveat">
				<strong>This model cannot see key numbers.</strong> Cover probabilities come from a normal
				distribution, which has no idea that football margins pile up on 3 and 7. Roughly 15% of NFL
				games end with a margin of exactly 3 and about 9% with exactly 7; a normal at σ≈13.9 puts under
				3% on each. So the textbook two-team Wong teaser (-7.5 and -8.5, six points) prices here at
				about 44.6% against a 52.4% break-even and reads as -EV, while those legs are commonly reported
				hitting in the low seventies.
				<br /><br />
				The web app displayed the crossed key numbers <em>beside</em> a probability that ignored them,
				which reads as though the crossing were priced in. It is not. Pricing the discrete mass needs an
				empirical margin distribution per sport — a data problem, and its own change.
			</div>
		{/if}
	{:else if !result.current?.error}
		<EmptyState icon="T" message="Complete the inputs above to compare fair odds against the book" />
	{/if}
</OutputSection>

<InfoSection title="How It Works">
	<p>
		Each leg's posted spread and price imply a true line once the vig is removed. Move that line by
		the teaser points, read the cover probability off the normal at the new number, and multiply
		across legs for the teaser's fair price. Compare against what the book offers.
	</p>
	<p>
		Key numbers (3, 7, 10, 14) are margins that occur disproportionately often in football. Legs
		that cross them gain more real cover probability per teaser point than the normal model
		credits — which is exactly why the caveat above matters.
	</p>
	<p>
		A teaser with fewer than two legs is an error here. The web app reduced over an empty leg list
		with an initial value of 1, so an empty teaser reported a certainty and an enormous positive EV.
	</p>
</InfoSection>

<style>
	.stepper {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.stepper button {
		width: 2rem;
		flex: 0 0 auto;
		font-size: 0.9rem;
	}

	.step-value {
		flex: 1;
		text-align: center;
		font-family: var(--font-mono);
		font-size: 1rem;
		font-weight: 700;
		color: var(--accent-cyan);
	}

	.leg {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.75rem;
	}

	.leg:last-of-type {
		margin-bottom: 0;
	}

	.leg-number {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		color: var(--text-muted);
		min-width: 1.75rem;
	}

	.leg-inputs {
		flex: 1;
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
		gap: 0.5rem;
	}

	.remove {
		font-size: 0.75rem;
		color: var(--accent-red);
		background: none;
		border: 1px solid transparent;
		padding: 4px 8px;
	}

	.remove:hover:not(:disabled) {
		background: none;
		border-color: var(--accent-red);
	}

	.add {
		margin-top: 0.85rem;
		width: 100%;
		color: var(--accent-cyan);
		font-size: 0.85rem;
	}

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
		padding: 0.45rem 0.5rem;
		border-bottom: 1px solid var(--border);
	}

	tbody tr:last-child td {
		border-bottom: none;
	}

	.left {
		text-align: left;
	}

	.arrow {
		color: var(--text-muted);
	}

	.key-badge {
		display: inline-block;
		background: var(--bg-tertiary);
		border: 1px solid var(--accent-amber);
		color: var(--accent-amber);
		border-radius: 3px;
		padding: 0 5px;
		margin-right: 3px;
		font-size: 0.7rem;
		font-weight: 700;
	}

	.none {
		color: var(--text-muted);
	}

	.caveat {
		margin-top: 1.25rem;
		padding: 1rem;
		border: 1px solid var(--accent-amber);
		border-radius: var(--radius);
		background: color-mix(in srgb, var(--accent-amber) 8%, transparent);
		font-size: 0.82rem;
		line-height: 1.6;
		color: var(--text-secondary);
	}
</style>
