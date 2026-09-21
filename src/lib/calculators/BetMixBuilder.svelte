<script lang="ts">
	import { onMount } from 'svelte';
	import { commands, type BetMix, type MathError, type MixLeg } from '$lib/bindings';
	import { Async, numeric, positive } from '$lib/async.svelte';
	import { betLog } from '$lib/betlog-store.svelte';
	import { mixLegsToBuckets, mixSourceLabel } from '$lib/betlog-prefill';
	import { takeMix } from '$lib/handoff';
	import { american, count, money, moneySigned, num, pct, pctSigned, signColor } from '$lib/format';
	import {
		InputCard,
		OutputSection,
		ResultRow,
		ResultLarge,
		EmptyState,
		InfoSection,
		ErrorNote,
		BetLogNote
	} from '$lib/ui';

	const MAX_BUCKETS = 8;

	interface Bucket {
		price: string;
		stake: string;
		edge: string;
		count: string;
	}

	function bucket(price: string, stake: string, edge: string, n: string): Bucket {
		return { price, stake, edge, count: n };
	}

	let buckets = $state<Bucket[]>([
		bucket('-110', '100', '2', '300'),
		bucket('150', '100', '3', '100'),
		bucket('600', '100', '5', '40')
	]);

	let fromBetLog = $state(false);
	let logNote = $state('');

	onMount(async () => {
		const handed = takeMix();
		if (handed && handed.length > 0) {
			buckets = mixLegsToBuckets(handed);
			fromBetLog = true;
			logNote = 'Loaded from a handoff on the bet log page.';
			return;
		}
		await betLog.ensureLoaded();
		const legs = betLog.mixLegs();
		if (legs.length > 0 && betLog.snapshot) {
			buckets = mixLegsToBuckets(legs);
			fromBetLog = true;
			logNote = `Loaded from your bet log (${mixSourceLabel(betLog.snapshot)}).`;
		}
	});

	let error = $state<MathError | string | null>(null);

	const mix = new Async(
		() => ({ buckets: buckets.map((b) => ({ ...b })) }),
		async ({ buckets }): Promise<BetMix | null> => {
			error = null;

			const legs: MixLeg[] = [];
			for (const b of buckets) {
				if (b.price.trim() === '') continue;

				const price = numeric(b.price);
				const stake = positive(b.stake);
				const edge = numeric(b.edge);
				const n = positive(b.count);
				if (price === null || stake === null || edge === null || n === null) {
					// All or nothing: pricing the buckets that happened to parse
					// and presenting the total as the user's book is the bug
					// this app exists to stop repeating.
					error = 'Every bucket needs a price, a stake, an edge and a bet count.';
					return null;
				}
				// A typed percentage into the 0–1 fraction the core works in.
				legs.push({ american: price, stake, edge: edge / 100, count: Math.round(n) });
			}

			if (legs.length === 0) return null;

			const response = await commands.betMix(legs);
			if (response.status === 'error') {
				error = response.error;
				return null;
			}
			return response.data;
		}
	);

	const result = $derived(mix.current);

	function addBucket() {
		if (buckets.length < MAX_BUCKETS) buckets.push(bucket('', '100', '2', '50'));
	}

	function removeBucket(index: number) {
		if (buckets.length > 1) buckets.splice(index, 1);
	}

	/** How much more of the swing a bucket carries than of the money. */
	function skew(varianceShare: number, stakeShare: number): number {
		return stakeShare > 0 ? varianceShare / stakeShare : Number.NaN;
	}
</script>

{#if fromBetLog}
	<BetLogNote message={logNote} />
{/if}

<InputCard title="Your bet mix">
	<div class="grid head">
		<span>Price</span>
		<span>Stake</span>
		<span>Edge %</span>
		<span>Bets</span>
		<span></span>
	</div>
	{#each buckets as b, i (i)}
		<div class="grid">
			<input type="text" inputmode="numeric" bind:value={buckets[i].price} placeholder="-110" />
			<input type="text" inputmode="decimal" bind:value={buckets[i].stake} placeholder="100" />
			<input type="text" inputmode="decimal" bind:value={buckets[i].edge} placeholder="2" />
			<input type="text" inputmode="numeric" bind:value={buckets[i].count} placeholder="100" />
			<button
				type="button"
				class="remove"
				onclick={() => removeBucket(i)}
				disabled={buckets.length === 1}
				aria-label="Remove bucket {i + 1}">×</button
			>
		</div>
	{/each}
	<button type="button" class="add" onclick={addBucket} disabled={buckets.length >= MAX_BUCKETS}>
		Add a price bucket
	</button>
</InputCard>

<OutputSection title="The whole book">
	<ErrorNote {error} />
	{#if result}
		<ResultLarge
			value={moneySigned(result.ev)}
			label="Expected profit over {count(result.bets)} bets"
			color={signColor(result.ev)}
		/>

		<ResultRow label="Total risked" value={money(result.totalStake)} />
		<ResultRow label="ROI" value={pctSigned(result.roi)} color={signColor(result.roi)} />
		<ResultRow
			label="Standard deviation"
			value={money(result.sd)}
			color="amber"
			hint="One season's swing, assuming the bets are independent"
		/>
		<ResultRow
			label="Edge as a share of the swing"
			value={num(result.tStat, 2)}
			color={result.tStat > 0.5 ? 'positive' : 'amber'}
			hint="Expected profit divided by its standard deviation. Below about 0.5 the year is mostly noise."
		/>
		<ResultRow
			label="Bets to confirm the edge"
			value={result.betsToDetect === null
				? 'never — this mix loses'
				: `${count(Math.round(result.betsToDetect))} bets`}
			color={result.betsToDetect === null ? 'negative' : 'blue'}
			hint="At two standard errors, holding this shape of book"
		/>

		<div class="block">
			<div class="block-title">Win rate</div>
			<ResultRow
				label="Blended break even"
				value={pct(result.blendedBreakeven)}
				hint="What you would need if every bet here hit at the same rate — a summary, not a prediction"
			/>
			<ResultRow
				label="Blended actual"
				value={pct(result.blendedWinRate)}
				color="positive"
				hint="Stake-weighted average of what each bucket is assumed to hit"
			/>
		</div>

		<div class="block">
			<div class="block-title">Where the swing comes from</div>
			<div class="table-wrap">
				<table>
					<thead>
						<tr>
							<th>Price</th>
							<th>Bets</th>
							<th>Risked</th>
							<th>EV</th>
							<th>Share of money</th>
							<th>Share of swing</th>
							<th></th>
						</tr>
					</thead>
					<tbody>
						{#each result.legs as leg, i (i)}
							<tr>
								<td class="price">{american(leg.american)}</td>
								<td>{count(leg.count)}</td>
								<td>{money(leg.totalStake, 0)}</td>
								<td class={leg.ev >= 0 ? 'good' : 'bad'}>{moneySigned(leg.ev, 0)}</td>
								<td>{pct(leg.stakeShare, 1)}</td>
								<td class="warn">{pct(leg.varianceShare, 1)}</td>
								<td class="skew">
									{Number.isFinite(skew(leg.varianceShare, leg.stakeShare))
										? `${num(skew(leg.varianceShare, leg.stakeShare), 1)}×`
										: '—'}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			<p class="note">
				The last column is share of swing divided by share of money. Anything above 1 is a bucket
				that costs you more variance than it costs you bankroll.
			</p>
		</div>
	{:else if !error}
		<EmptyState icon="#" message="Describe the bets you actually place" />
	{/if}
</OutputSection>

<InfoSection title="Why the money and the swing are not the same split">
	<p>
		Expected value adds across bets. Variance adds too, but stake enters it <em>squared</em> and the
		per-unit variance rises with the price — at the break-even rate it is exactly the amount you
		stand to win. So a bucket of longshots at the same stake and the same edge as your bread and
		butter can supply the large majority of the year's swing while supplying a small minority of the
		expected profit.
	</p>
	<p>
		<strong>The blended break-even figure is a summary, not a forecast.</strong> It answers "what
		would I need to hit if every bet in this book hit at the same rate", which is a question with a
		clean stake-weighted answer and no claim to describe reality — the buckets do not hit at the same
		rate, and that is the point of splitting them.
	</p>
	<p>
		<strong>Independence is assumed.</strong> Every figure here treats each bet as unrelated to the
		others. Same-game legs, the same side across correlated markets, and a whole slate moving on one
		weather report all push real variance above what this reports. Nothing in the app tries to fit a
		correlation between legs, for reasons written up in <code>tasks/todo.md</code>.
	</p>
</InfoSection>

<style>
	.grid {
		display: grid;
		grid-template-columns: 1.1fr 1fr 0.9fr 0.9fr 2rem;
		gap: 0.5rem;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.grid.head {
		font-size: 0.68rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		margin-bottom: 0.35rem;
	}

	.grid.head span {
		text-align: center;
	}

	.remove {
		padding: 0.35rem 0;
		line-height: 1;
		color: var(--text-muted);
	}

	.remove:hover:not(:disabled) {
		color: var(--accent-red);
		border-color: var(--accent-red);
	}

	.add {
		width: 100%;
		margin-top: 0.5rem;
		color: var(--accent-cyan);
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

	.table-wrap {
		overflow-x: auto;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.8rem;
	}

	th {
		text-align: right;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-size: 0.7rem;
		padding: 0.5rem 0.6rem;
		border-bottom: 1px solid var(--border-strong);
		white-space: nowrap;
	}

	th:first-child {
		text-align: left;
	}

	td {
		text-align: right;
		padding: 0.4rem 0.6rem;
		border-bottom: 1px solid var(--border);
		font-family: var(--font-mono);
		white-space: nowrap;
	}

	td.price {
		text-align: left;
		font-weight: 600;
	}

	td.good {
		color: var(--accent-green);
	}

	td.bad {
		color: var(--accent-red);
	}

	td.warn {
		color: var(--accent-amber);
	}

	td.skew {
		color: var(--text-secondary);
	}

	.note {
		margin-top: 0.75rem;
		font-size: 0.75rem;
		color: var(--text-muted);
		line-height: 1.6;
	}
</style>
