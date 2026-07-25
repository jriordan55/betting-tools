<script lang="ts">
	import { commands, type MathError, type OddsFormat, type Parlay } from '$lib/bindings';
	import { Async, positive } from '$lib/async.svelte';
	import { parseAll, SIMPLE_FORMAT_OPTIONS, ODDS_PLACEHOLDER } from '$lib/odds';
	import { american, money, moneySigned, num, pct } from '$lib/format';
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

	const MAX_LEGS = 10;

	let format = $state<OddsFormat>('american');
	let stake = $state('100');
	let legs = $state<string[]>(['', '']);

	interface ParlayView {
		parlay: Parlay;
		combinedAmerican: number;
	}

	const result = new Async<
		{ legs: string[]; stake: string; format: OddsFormat },
		{ data: ParlayView | null; error: MathError | string | null }
	>(
		() => ({ legs: [...legs], stake, format }),
		async ({ legs, stake, format }) => {
			const stakeAmount = positive(stake);
			if (stakeAmount === null) {
				// `if (!stake)` in the web app let a negative stake through, since
				// `!(-500)` is false. A negative parlay stake is not a bet.
				return { data: null, error: stake.trim() === '' ? null : 'Stake must be greater than 0.' };
			}

			if (legs.some((leg) => leg.trim() === '')) return { data: null, error: null };

			// All-or-nothing: the web app dropped unparseable legs and priced the
			// parlay from the survivors, so one typo returned a four-leg price
			// presented as your five-leg parlay.
			const parsed = await parseAll(legs, format);
			if (parsed.error) {
				return { data: null, error: parsed.error };
			}
			if (!parsed.decimals) return { data: null, error: null };

			const parlay = await commands.parlay(parsed.decimals, stakeAmount);
			if (parlay.status === 'error') return { data: null, error: parlay.error };

			const view = await commands.fromDecimal(parlay.data.decimal);
			return {
				data: {
					parlay: parlay.data,
					combinedAmerican: view.status === 'ok' ? view.data.american : Number.NaN
				},
				error: null
			};
		}
	);

	function addLeg() {
		if (legs.length < MAX_LEGS) legs = [...legs, ''];
	}

	function removeLeg(index: number) {
		if (legs.length > 2) legs = legs.filter((_, i) => i !== index);
	}
</script>

<InputCard title="Settings">
	<FormRow>
		<FormGroup label="Stake ($)">
			<input type="number" bind:value={stake} placeholder="100" min="0" step="any" />
		</FormGroup>
		<FormGroup label="Odds Format">
			<ToggleGroup options={SIMPLE_FORMAT_OPTIONS} bind:value={format} />
		</FormGroup>
	</FormRow>
</InputCard>

<InputCard title="Parlay Legs ({legs.length})">
	{#each legs as _, index (index)}
		<div class="leg">
			<span class="leg-number">#{index + 1}</span>
			<input type="text" bind:value={legs[index]} placeholder={ODDS_PLACEHOLDER[format]} />
			<button
				type="button"
				class="remove"
				onclick={() => removeLeg(index)}
				disabled={legs.length <= 2}
				aria-label="Remove leg {index + 1}"
			>
				Remove
			</button>
		</div>
	{/each}
	{#if legs.length < MAX_LEGS}
		<button type="button" class="add" onclick={addLeg}>+ Add Leg</button>
	{/if}
</InputCard>

<OutputSection title="Parlay Results">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data}
		{@const { parlay, combinedAmerican } = result.current.data}
		<ResultLarge value={money(parlay.payout)} label="Total Payout" />
		<ResultRow label="Profit" value={moneySigned(parlay.profit)} color="positive" />
		<ResultRow label="Combined Odds (American)" value={american(combinedAmerican)} color="highlight" />
		<ResultRow label="Combined Odds (Decimal)" value={num(parlay.decimal, 3)} />
		<ResultRow label="Implied Probability" value={pct(parlay.impliedProb)} />
		<ResultRow label="Number of Legs" value={String(parlay.legCount)} />
	{:else if !result.current?.error}
		<EmptyState icon="+" message="Enter odds for every leg to price the parlay" />
	{/if}
</OutputSection>

<InfoSection title="About Parlays">
	<p>
		A parlay combines multiple bets into one; every leg must win. Payouts are higher, but so is the
		book's edge — the vig on each leg compounds.
	</p>
	<p>
		<strong>This price assumes the legs are independent.</strong> Same-game legs almost never are: a
		quarterback going over on passing yards makes his receiver's over more likely, so this
		understates a positively-correlated parlay's true win probability. Treat the number as a
		straight-parlay price, and treat a same-game parlay as unpriced — this app does not claim to
		price one, because doing it honestly needs correlation data it does not have.
	</p>
</InfoSection>

<style>
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

	.leg input {
		flex: 1;
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
</style>
