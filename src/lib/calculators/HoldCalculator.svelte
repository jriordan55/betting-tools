<script lang="ts">
	import { commands, type Hold, type MathError, type OddsFormat } from '$lib/bindings';
	import { Async } from '$lib/async.svelte';
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
	let sideA = $state('');
	let sideB = $state('');

	interface HoldView {
		hold: Hold;
		decimalA: number;
		decimalB: number;
		fairA: FairOdds;
		fairB: FairOdds;
	}

	const result = new Async<
		{ sideA: string; sideB: string; format: OddsFormat },
		{ data: HoldView | null; error: MathError | null }
	>(
		() => ({ sideA, sideB, format }),
		async ({ sideA, sideB, format }) => {
			const a = await parseOdds(sideA, format);
			if (a.error) return { data: null, error: a.error };
			const b = await parseOdds(sideB, format);
			if (b.error) return { data: null, error: b.error };
			if (a.decimal === null || b.decimal === null) return { data: null, error: null };

			const [viewA, viewB] = await Promise.all([
				commands.fromDecimal(a.decimal),
				commands.fromDecimal(b.decimal)
			]);
			if (viewA.status === 'error') return { data: null, error: viewA.error };
			if (viewB.status === 'error') return { data: null, error: viewB.error };

			const hold = await commands.calculateHold(viewA.data.probability, viewB.data.probability);
			if (hold.status === 'error') return { data: null, error: hold.error };

			const [fairA, fairB] = await Promise.all([
				fairOdds(hold.data.noVigProbA),
				fairOdds(hold.data.noVigProbB)
			]);

			return {
				data: { hold: hold.data, decimalA: a.decimal, decimalB: b.decimal, fairA, fairB },
				error: null
			};
		}
	);

	function holdColor(hold: number) {
		if (hold > 0.05) return 'negative' as const;
		if (hold > 0.03) return 'amber' as const;
		return 'positive' as const;
	}

	function fairText(fair: FairOdds) {
		return fair.decimal === null
			? american(fair.american)
			: `${american(fair.american)} (${num(fair.decimal, 3)})`;
	}
</script>

<InputCard title="Market Odds">
	<FormRow>
		<FormGroup label="Odds Format">
			<ToggleGroup options={SIMPLE_FORMAT_OPTIONS} bind:value={format} />
		</FormGroup>
	</FormRow>
	<FormRow>
		<FormGroup label="Side A" hint="e.g. favorite or Over">
			<input type="text" bind:value={sideA} placeholder={ODDS_PLACEHOLDER[format]} />
		</FormGroup>
		<FormGroup label="Side B" hint="e.g. underdog or Under">
			<input type="text" bind:value={sideB} placeholder={ODDS_PLACEHOLDER[format]} />
		</FormGroup>
	</FormRow>
</InputCard>

<OutputSection title="Hold Analysis">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const { hold, decimalA, decimalB, fairA, fairB } = result.current.data}
		<ResultLarge value={pct(hold.hold)} label="Book Hold (Vig)" color={holdColor(hold.hold)} />
		<ResultRow label="Total Implied Probability" value={pct(hold.totalImplied)} color="highlight" />

		<div class="side">
			<div class="side-title">Side A</div>
			<ResultRow label="Decimal Price" value={num(decimalA, 3)} />
			<ResultRow label="Implied Probability" value={pct(hold.impliedA)} />
			<ResultRow label="No-Vig Fair Probability" value={pct(hold.noVigProbA)} color="highlight" />
			<ResultRow label="No-Vig Fair Odds" value={fairText(fairA)} color="highlight" />
		</div>

		<div class="side">
			<div class="side-title">Side B</div>
			<ResultRow label="Decimal Price" value={num(decimalB, 3)} />
			<ResultRow label="Implied Probability" value={pct(hold.impliedB)} />
			<ResultRow label="No-Vig Fair Probability" value={pct(hold.noVigProbB)} color="highlight" />
			<ResultRow label="No-Vig Fair Odds" value={fairText(fairB)} color="highlight" />
		</div>
	{:else if !result.current?.error}
		<EmptyState icon="%" message="Enter odds for both sides to calculate the hold" />
	{/if}
</OutputSection>

<InfoSection title="About Hold / Vig">
	<p>
		The hold (or vig, or juice) is the margin the book builds into the odds. In a fair market the
		implied probabilities of all outcomes sum to exactly 100%; the amount above 100% is the book's
		edge. A standard -110/-110 market holds <strong>4.76%</strong> by that definition. Sharp books
		often run 2–3%; retail books can run 6–10% on props.
	</p>
	<p>
		You will also see -110/-110 quoted as 4.55%. That is the same market measured as a share of
		total handle rather than of the fair book — <code>hold / (1 + hold)</code>. Both are called
		"hold". This calculator reports the first, which is the one that compares directly against the
		no-vig line below it.
	</p>
	<p>
		This calculator uses proportional (multiplicative) devigging. For markets with a heavy
		favorite that assumption is questionable — see the Devig Calculator, which compares five
		methods side by side.
	</p>
</InfoSection>

<style>
	.side {
		margin-top: 1.25rem;
		padding-top: 1.25rem;
		border-top: 1px solid var(--border);
	}

	.side-title {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		margin-bottom: 0.75rem;
	}
</style>
