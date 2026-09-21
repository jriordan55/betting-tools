<script lang="ts">
	import { goto } from '$app/navigation';
	import {
		commands,
		type Bet,
		type BetDraft,
		type BetFilter,
		type BetKind,
		type BetLogError,
		type BetLogFacets,
		type BetLogView,
		type LedgerMix,
		type LogError,
		type BetOutcome
	} from '$lib/bindings';
	import { describeBetLogError, describeLogError } from '$lib/errors';
	import { offerMix } from '$lib/handoff';
	import { betLog } from '$lib/betlog-store.svelte';
	import { american, count, money, moneySigned, pct, pctSigned, points, signColor } from '$lib/format';
	import { InputCard, OutputSection, FormRow, FormGroup, ResultRow, EmptyState, InfoSection } from '$lib/ui';

	const OUTCOMES: { value: BetOutcome; label: string }[] = [
		{ value: 'pending', label: 'Pending' },
		{ value: 'won', label: 'Won' },
		{ value: 'lost', label: 'Lost' },
		{ value: 'push', label: 'Push' },
		{ value: 'void', label: 'Void' }
	];

	function blankDraft(): BetDraft {
		return {
			placedAt: new Date().toISOString().slice(0, 10),
			sport: '',
			market: '',
			selection: '',
			book: '',
			priceTaken: -110,
			closingPrice: null,
			opposingClosingPrice: null,
			stake: 100,
			outcome: 'pending',
			notes: ''
		};
	}

	/** Text fields, because a partly-typed number is not a number yet. */
	let form = $state({
		placedAt: blankDraft().placedAt,
		sport: '',
		market: '',
		selection: '',
		book: '',
		priceTaken: '-110',
		closingPrice: '',
		opposingClosingPrice: '',
		stake: '100',
		outcome: 'pending' as BetOutcome,
		notes: ''
	});

	let editingId = $state<number | null>(null);
	let filterOutcome = $state<BetOutcome | ''>('');
	let filterSport = $state('');
	let filterBook = $state('');
	let filterMarket = $state('');
	let filterYear = $state('');
	let filterKind = $state<BetKind | ''>('');
	let filterMinStake = $state('');
	let filterMaxStake = $state('');

	type SortColumn =
		| 'placedAt'
		| 'selection'
		| 'book'
		| 'sport'
		| 'market'
		| 'priceTaken'
		| 'stake'
		| 'outcome'
		| 'profit'
		| 'clvPoints'
		| 'ev';
	type SortDir = 'asc' | 'desc';

	let sortColumn = $state<SortColumn>('placedAt');
	let sortDir = $state<SortDir>('desc');

	let view = $state<BetLogView | null>(null);
	let facets = $state<BetLogFacets | null>(null);
	let mix = $state<LedgerMix | null>(null);
	let error = $state<string | null>(null);
	let saving = $state(false);
	let ephemeral = $state<string | null>(null);
	/** Armed by a first click on delete, so nothing is destroyed by one tap. */
	let confirmingDelete = $state<number | null>(null);

	/**
	 * Which refresh is the current one.
	 *
	 * Three commands go out per refresh and a filter change can fire another
	 * before they land. Without this, a slow earlier response overwrites a fast
	 * later one and the table shows a filter the controls no longer say — the
	 * same staleness `Async` guards against, which this page cannot use because
	 * it also refreshes imperatively after a write.
	 */
	let generation = 0;

	function optionalStake(value: string): number | null {
		const trimmed = value.trim();
		if (trimmed === '') return null;
		const n = Number.parseFloat(trimmed);
		return Number.isFinite(n) ? n : null;
	}

	const filter = $derived<BetFilter>({
		outcome: filterOutcome === '' ? null : filterOutcome,
		sport: filterSport === '' ? null : filterSport,
		book: filterBook === '' ? null : filterBook,
		market: filterMarket === '' ? null : filterMarket,
		year: filterYear === '' ? null : Number.parseInt(filterYear, 10),
		minStake: optionalStake(filterMinStake),
		maxStake: optionalStake(filterMaxStake),
		kind: filterKind === '' ? null : filterKind,
		fromDate: null,
		toDate: null
	});

	const sports = $derived(facets?.sports ?? []);

	function fail(e: LogError | BetLogError | string) {
		error = typeof e === 'string' ? e : 'source' in e ? describeBetLogError(e) : describeLogError(e);
	}

	async function refresh() {
		const mine = ++generation;
		const [analysis, facetList, derived, status] = await Promise.all([
			commands.analyzeBetLog(filter),
			commands.betLogFacets(),
			// A mix needs bets with both closing prices, so a young log has
			// none. That is a normal state, not a failure worth showing.
			commands.betLogMix(filter, 100),
			commands.betLogStatus()
		]);
		if (mine !== generation) return;

		if (analysis.status === 'error') {
			fail(analysis.error);
			view = null;
		} else {
			error = null;
			view = analysis.data;
		}
		if (facetList.status === 'ok') facets = facetList.data;
		mix = derived.status === 'ok' ? derived.data : null;
		ephemeral = status ?? null;
		await betLog.refresh();
	}

	// Runs once on mount and again whenever the filter changes. `filter` is read
	// synchronously here, before the await, so the effect actually tracks it.
	$effect(() => {
		void filter;
		void refresh();
	});

	function optionalPrice(value: string): number | null {
		const trimmed = value.trim();
		if (trimmed === '') return null;
		const n = Number.parseFloat(trimmed);
		return Number.isFinite(n) ? n : Number.NaN;
	}

	function toDraft(): BetDraft | null {
		const price = Number.parseFloat(form.priceTaken);
		const stake = Number.parseFloat(form.stake);
		const close = optionalPrice(form.closingPrice);
		const opposing = optionalPrice(form.opposingClosingPrice);

		if (!Number.isFinite(price) || !Number.isFinite(stake)) {
			error = 'A bet needs a price and a stake.';
			return null;
		}
		if (Number.isNaN(close) || Number.isNaN(opposing)) {
			error = 'A closing price must be a number, or left blank.';
			return null;
		}
		if (form.selection.trim() === '') {
			error = 'Say what you backed — the log is unreadable a month later without it.';
			return null;
		}

		return {
			placedAt: form.placedAt,
			sport: form.sport.trim(),
			market: form.market.trim(),
			selection: form.selection.trim(),
			book: form.book.trim(),
			priceTaken: price,
			closingPrice: close,
			opposingClosingPrice: opposing,
			stake,
			outcome: form.outcome,
			realizedProfit: null,
			notes: form.notes.trim()
		};
	}

	async function save() {
		const draft = toDraft();
		if (!draft) return;

		saving = true;
		try {
			const response =
				editingId === null
					? await commands.addBet(draft)
					: await commands.updateBet(editingId, draft);

			if (response.status === 'error') {
				fail(response.error);
				return;
			}
			resetForm();
			await refresh();
		} finally {
			saving = false;
		}
	}

	function edit(bet: Bet) {
		editingId = bet.id;
		form = {
			placedAt: bet.placedAt,
			sport: bet.sport,
			market: bet.market,
			selection: bet.selection,
			book: bet.book,
			priceTaken: String(bet.priceTaken),
			closingPrice: bet.closingPrice === null ? '' : String(bet.closingPrice),
			opposingClosingPrice:
				bet.opposingClosingPrice === null ? '' : String(bet.opposingClosingPrice),
			stake: String(bet.stake),
			outcome: bet.outcome,
			notes: bet.notes
		};
	}

	function resetForm() {
		const blank = blankDraft();
		editingId = null;
		error = null;
		form = {
			placedAt: blank.placedAt,
			sport: form.sport,
			market: form.market,
			selection: '',
			book: form.book,
			priceTaken: '-110',
			closingPrice: '',
			opposingClosingPrice: '',
			stake: form.stake,
			outcome: 'pending',
			notes: ''
		};
	}

	async function remove(bet: Bet) {
		// A logged bet is not recoverable, and the button is two pixels from
		// "edit". The first click arms; the second commits.
		if (confirmingDelete !== bet.id) {
			confirmingDelete = bet.id;
			return;
		}
		confirmingDelete = null;

		const response = await commands.deleteBet(bet.id);
		if (response.status === 'error') {
			fail(response.error);
			return;
		}
		if (editingId === bet.id) resetForm();
		await refresh();
	}

	async function sendToSimulator() {
		if (!mix) return;
		offerMix(mix.legs);
		await goto('/calculators/season-simulator');
	}

	const summary = $derived(view?.ledger.summary ?? null);

	/** Bets and their analysis travel together, so index alignment is the API's. */
	const rows = $derived(
		(view?.bets ?? []).map((bet, i) => ({ bet, analysis: view?.ledger.bets[i] ?? null }))
	);

	function compareRows(
		a: { bet: Bet; analysis: BetLogView['ledger']['bets'][number] | null },
		b: { bet: Bet; analysis: BetLogView['ledger']['bets'][number] | null }
	): number {
		const dir = sortDir === 'asc' ? 1 : -1;

		const text = (left: string, right: string) => left.localeCompare(right) * dir;
		const number = (left: number | null | undefined, right: number | null | undefined) => {
			const l = left ?? Number.NEGATIVE_INFINITY;
			const r = right ?? Number.NEGATIVE_INFINITY;
			return (l - r) * dir;
		};

		switch (sortColumn) {
			case 'selection':
				return text(a.bet.selection, b.bet.selection);
			case 'book':
				return text(a.bet.book, b.bet.book);
			case 'sport':
				return text(a.bet.sport, b.bet.sport);
			case 'market':
				return text(a.bet.market, b.bet.market);
			case 'priceTaken':
				return number(a.bet.priceTaken, b.bet.priceTaken);
			case 'stake':
				return number(a.bet.stake, b.bet.stake);
			case 'outcome':
				return text(a.bet.outcome, b.bet.outcome);
			case 'profit':
				return number(a.analysis?.profit, b.analysis?.profit);
			case 'clvPoints':
				return number(a.analysis?.clvPoints, b.analysis?.clvPoints);
			case 'ev':
				return number(a.analysis?.ev, b.analysis?.ev);
			default:
				return text(a.bet.placedAt, b.bet.placedAt) || (a.bet.id - b.bet.id) * dir;
		}
	}

	const displayRows = $derived([...rows].sort(compareRows));

	const activeFilterCount = $derived(
		[
			filterOutcome,
			filterSport,
			filterBook,
			filterMarket,
			filterYear,
			filterKind,
			filterMinStake,
			filterMaxStake
		].filter((value) => value !== '').length
	);

	function toggleSort(column: SortColumn) {
		if (sortColumn === column) {
			sortDir = sortDir === 'asc' ? 'desc' : 'asc';
			return;
		}
		sortColumn = column;
		sortDir =
			column === 'selection' || column === 'book' || column === 'sport' || column === 'market'
				? 'asc'
				: 'desc';
	}

	function sortLabel(column: SortColumn): string {
		if (sortColumn !== column) return '';
		return sortDir === 'asc' ? ' ↑' : ' ↓';
	}

	function clearFilters() {
		filterOutcome = '';
		filterSport = '';
		filterBook = '';
		filterMarket = '';
		filterYear = '';
		filterKind = '';
		filterMinStake = '';
		filterMaxStake = '';
	}

	function outcomeClass(outcome: BetOutcome): string {
		if (outcome === 'won') return 'won';
		if (outcome === 'lost') return 'lost';
		if (outcome === 'pending') return 'pending';
		return 'push';
	}
</script>

<svelte:head><title>Bet Log — Bettor Desktop</title></svelte:head>

<div class="page container-wide">
	<header class="head">
		<h1>Bet Log</h1>
		<p>
			Your book opens with every bet already recorded. Add the next one below — it stays in the
			list on this machine.
		</p>
	</header>

	{#if ephemeral}
		<div class="note warn" role="alert">
			<strong>Nothing is being saved.</strong> The log file could not be opened, so this session is
			running in memory and will be lost when the window closes. {ephemeral}
		</div>
	{/if}

	{#if error}
		<div class="note" role="alert">{error}</div>
	{/if}

	<div class="split">
		<div>
			<InputCard title={editingId === null ? 'Record a bet' : `Editing bet #${editingId}`}>
				<FormRow>
					<FormGroup label="Date">
						<input type="date" bind:value={form.placedAt} />
					</FormGroup>
					<FormGroup label="Sport">
						<input type="text" bind:value={form.sport} placeholder="NFL" list="sports" />
					</FormGroup>
					<FormGroup label="Market">
						<input type="text" bind:value={form.market} placeholder="spread" />
					</FormGroup>
				</FormRow>
				<FormRow>
					<FormGroup label="Selection" hint="What you actually backed">
						<input type="text" bind:value={form.selection} placeholder="Bears -3.5" />
					</FormGroup>
					<FormGroup label="Book">
						<input type="text" bind:value={form.book} placeholder="Pinnacle" />
					</FormGroup>
				</FormRow>
				<FormRow>
					<FormGroup label="Price taken">
						<input type="text" inputmode="numeric" bind:value={form.priceTaken} placeholder="-110" />
					</FormGroup>
					<FormGroup label="Stake ($)">
						<input type="text" inputmode="decimal" bind:value={form.stake} placeholder="100" />
					</FormGroup>
					<FormGroup label="Result">
						<select bind:value={form.outcome}>
							{#each OUTCOMES as o (o.value)}
								<option value={o.value}>{o.label}</option>
							{/each}
						</select>
					</FormGroup>
				</FormRow>
				<FormRow>
					<FormGroup label="Closing price" hint="Your side at the close — optional">
						<input type="text" inputmode="numeric" bind:value={form.closingPrice} placeholder="—" />
					</FormGroup>
					<FormGroup
						label="Opposing close"
						hint="The other side. Without it the vig cannot be removed and no edge can be measured."
					>
						<input
							type="text"
							inputmode="numeric"
							bind:value={form.opposingClosingPrice}
							placeholder="—"
						/>
					</FormGroup>
				</FormRow>
				<FormRow>
					<FormGroup label="Notes">
						<input type="text" bind:value={form.notes} placeholder="optional" />
					</FormGroup>
				</FormRow>
				<div class="actions">
					<button type="button" class="save" onclick={save} disabled={saving}>
						{editingId === null ? 'Record it' : 'Save changes'}
					</button>
					{#if editingId !== null}
						<button type="button" onclick={resetForm}>Cancel</button>
					{/if}
				</div>
			</InputCard>

			<datalist id="sports">
				{#each sports as s (s)}<option value={s}></option>{/each}
			</datalist>
		</div>

		<div>
			<OutputSection title="The record">
				{#if summary && summary.bets > 0}
					<ResultRow
						label="Profit"
						value={moneySigned(summary.profit)}
						color={signColor(summary.profit)}
						hint="{count(summary.settled)} settled bets, {money(summary.staked)} risked"
					/>
					<ResultRow label="ROI" value={pctSigned(summary.roi)} color={signColor(summary.roi)} />
					<ResultRow
						label="Record"
						value="{summary.won}–{summary.lost}{summary.pushed > 0 ? `–${summary.pushed}` : ''}"
						hint="{pct(summary.winRate)} of decided bets. Pushes are settled but not decided."
					/>
					{#if summary.pending > 0}
						<ResultRow label="Still open" value="{count(summary.pending)} bets" color="muted" />
					{/if}

					{#if summary.meanClvPoints !== null}
						<div class="block">
							<div class="block-title">Against the close</div>
							<ResultRow
								label="Mean closing line value"
								value={points(summary.meanClvPoints)}
								color={signColor(summary.meanClvPoints)}
								hint="Across {count(summary.withClose)} bets with a closing price"
							/>
							{#if summary.beatCloseRate !== null}
								<ResultRow label="Beat the close" value={pct(summary.beatCloseRate)} />
							{/if}
						</div>
					{/if}

					{#if summary.expectedProfit !== null && summary.luck !== null}
						<div class="block">
							<div class="block-title">What should have happened</div>
							<ResultRow
								label="Expected profit"
								value={moneySigned(summary.expectedProfit)}
								color={signColor(summary.expectedProfit)}
								hint="Against the devigged close, over {count(summary.withFairClose)} bets that have both sides recorded"
							/>
							{#if summary.expectedRoi !== null}
								<ResultRow
									label="Expected ROI"
									value={pctSigned(summary.expectedRoi)}
									color={signColor(summary.expectedRoi)}
								/>
							{/if}
							<ResultRow
								label="Luck"
								value={moneySigned(summary.luck)}
								color={signColor(summary.luck)}
								hint="Realised minus expected. Normally the largest number here, and it is not skill in either direction."
							/>
						</div>
					{/if}
				{:else}
					<EmptyState icon="$" message="Record a bet and the record builds itself" />
				{/if}
			</OutputSection>

			{#if mix}
				<OutputSection title="The season this record implies">
					<p class="lead">
						{count(mix.betsUsed)} bets bucketed by price, each carrying its edge against the fair
						close.
						{#if mix.betsSkipped > 0}
							<strong>{count(mix.betsSkipped)}</strong> skipped for want of a closing market on both
							sides.
						{/if}
					</p>
					<div class="table-wrap">
						<table>
							<thead>
								<tr><th>Price</th><th>Bets</th><th>Stake</th><th>Edge</th></tr>
							</thead>
							<tbody>
								{#each mix.legs as leg, i (i)}
									<tr>
										<td class="price">{american(leg.american)}</td>
										<td>{count(leg.count)}</td>
										<td>{money(leg.stake, 0)}</td>
										<td class={leg.edge >= 0 ? 'won' : 'lost'}>{pctSigned(leg.edge)}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
					<button type="button" class="send" onclick={sendToSimulator}>
						Simulate a season of this →
					</button>
				</OutputSection>
			{/if}
		</div>
	</div>

	<OutputSection title="Bets">
		<div class="filters">
			<select bind:value={filterOutcome} aria-label="Filter by result">
				<option value="">All results</option>
				{#each OUTCOMES as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
			</select>
			<select bind:value={filterKind} aria-label="Filter by ticket type">
				<option value="">Singles & parlays</option>
				<option value="single">Singles only</option>
				<option value="parlay">Parlays only</option>
			</select>
			<select bind:value={filterYear} aria-label="Filter by year">
				<option value="">All years</option>
				{#each facets?.years ?? [] as year (year)}<option value={String(year)}>{year}</option>{/each}
			</select>
			<select bind:value={filterSport} aria-label="Filter by sport">
				<option value="">All sports</option>
				{#each facets?.sports ?? [] as s (s)}<option value={s}>{s}</option>{/each}
			</select>
			<select bind:value={filterBook} aria-label="Filter by sportsbook">
				<option value="">All sportsbooks</option>
				{#each facets?.books ?? [] as book (book)}<option value={book}>{book}</option>{/each}
			</select>
			<select bind:value={filterMarket} aria-label="Filter by market type">
				<option value="">All market types</option>
				{#each facets?.markets ?? [] as market (market)}<option value={market}>{market}</option>{/each}
			</select>
			<label class="stake-filter">
				<span>Min stake</span>
				<input
					type="text"
					inputmode="decimal"
					bind:value={filterMinStake}
					placeholder="any"
					aria-label="Minimum stake"
				/>
			</label>
			<label class="stake-filter">
				<span>Max stake</span>
				<input
					type="text"
					inputmode="decimal"
					bind:value={filterMaxStake}
					placeholder="any"
					aria-label="Maximum stake"
				/>
			</label>
			{#if activeFilterCount > 0}
				<button type="button" class="clear-filters" onclick={clearFilters}>
					Clear filters ({activeFilterCount})
				</button>
			{/if}
		</div>

		{#if displayRows.length > 0}
			<p class="table-meta">{count(displayRows.length)} bets shown</p>
			<div class="table-wrap">
				<table class="bets-table">
					<thead>
						<tr>
							<th class="sortable left" onclick={() => toggleSort('placedAt')}>
								Date{sortLabel('placedAt')}
							</th>
							<th class="sortable left" onclick={() => toggleSort('selection')}>
								Selection{sortLabel('selection')}
							</th>
							<th class="sortable left" onclick={() => toggleSort('book')}>
								Book{sortLabel('book')}
							</th>
							<th class="sortable left" onclick={() => toggleSort('sport')}>
								Sport{sortLabel('sport')}
							</th>
							<th class="sortable left" onclick={() => toggleSort('market')}>
								Market{sortLabel('market')}
							</th>
							<th class="sortable" onclick={() => toggleSort('priceTaken')}>
								Price{sortLabel('priceTaken')}
							</th>
							<th class="sortable" onclick={() => toggleSort('stake')}>
								Stake{sortLabel('stake')}
							</th>
							<th class="sortable" onclick={() => toggleSort('outcome')}>
								Result{sortLabel('outcome')}
							</th>
							<th class="sortable" onclick={() => toggleSort('profit')}>
								Profit{sortLabel('profit')}
							</th>
							<th class="sortable" onclick={() => toggleSort('clvPoints')}>
								CLV{sortLabel('clvPoints')}
							</th>
							<th class="sortable" onclick={() => toggleSort('ev')}>
								Edge{sortLabel('ev')}
							</th>
							<th></th>
						</tr>
					</thead>
					<tbody>
						{#each displayRows as { bet, analysis } (bet.id)}
							<tr class:editing={editingId === bet.id}>
								<td class="left">{bet.placedAt}</td>
								<td class="left">{bet.selection}</td>
								<td class="left meta-cell">{bet.book || '—'}</td>
								<td class="left meta-cell">{bet.sport || '—'}</td>
								<td class="left meta-cell">{bet.market || '—'}</td>
								<td class="price">{american(bet.priceTaken)}</td>
								<td>{money(bet.stake, 0)}</td>
								<td class={outcomeClass(bet.outcome)}>{bet.outcome}</td>
								<td class={analysis?.profit ? (analysis.profit > 0 ? 'won' : 'lost') : ''}>
									{analysis?.profit === null || analysis?.profit === undefined
										? '—'
										: moneySigned(analysis.profit, 0)}
								</td>
								<td class={analysis?.clvPoints ? (analysis.clvPoints > 0 ? 'won' : 'lost') : ''}>
									{analysis?.clvPoints === null || analysis?.clvPoints === undefined
										? '—'
										: points(analysis.clvPoints)}
								</td>
								<td class={analysis?.ev ? (analysis.ev > 0 ? 'won' : 'lost') : ''}>
									{analysis?.ev === null || analysis?.ev === undefined
										? '—'
										: pctSigned(analysis.ev)}
								</td>
								<td class="row-actions">
									<button type="button" onclick={() => edit(bet)} aria-label="Edit">edit</button>
									<button
										type="button"
										class:armed={confirmingDelete === bet.id}
										onclick={() => remove(bet)}
										aria-label={confirmingDelete === bet.id ? 'Confirm delete' : 'Delete'}
									>
										{confirmingDelete === bet.id ? 'sure?' : '×'}
									</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{:else}
			<EmptyState icon="·" message="No bets match this filter" />
		{/if}
	</OutputSection>

	<InfoSection title="Why the closing line matters more than the result">
		<p>
			<strong>Profit is a small sample of a noisy process.</strong> Over a few hundred bets the gap
			between what a record returned and what its prices deserved is routinely larger than the edge
			itself, in either direction. The <em>Luck</em> row is that gap, stated rather than left for
			you to infer from two numbers that look unrelated.
		</p>
		<p>
			<strong>The opposing closing price is not optional if you want an edge figure.</strong> A
			closing price on its own still carries the book's margin, so any edge computed from it is
			overstated by roughly half the hold — enough to turn a break-even bettor into a winning one on
			paper. Record both sides and the vig can be removed; record one and the log will show closing
			line value but decline to claim an edge.
		</p>
		<p>
			<strong>The bucketed mix is the honest input to a variance model.</strong> It carries the edge
			your closing lines say you had, not the return you happened to get — because "given bets this
			good at prices this long, what does a season look like" is a different question from "what
			did last season do", and only the first one generalises.
		</p>
		<p>
			The log lives in SQLite in the app's own data directory, in WAL mode. Nothing leaves the
			machine.
		</p>
	</InfoSection>
</div>

<style>
	.page {
		padding: 2rem 0 4rem;
	}

	.head {
		margin-bottom: 1.75rem;
	}

	h1 {
		font-size: 1.6rem;
		font-weight: 700;
		letter-spacing: -0.025em;
	}

	.head p {
		margin-top: 0.4rem;
		color: var(--text-secondary);
		font-size: 0.9rem;
		max-width: 62ch;
	}

	.note.warn {
		border-color: var(--accent-amber);
		background: color-mix(in srgb, var(--accent-amber) 10%, transparent);
		color: var(--accent-amber);
	}

	.note {
		border: 1px solid var(--accent-red);
		border-radius: var(--radius);
		background: color-mix(in srgb, var(--accent-red) 10%, transparent);
		color: var(--accent-red);
		font-size: 0.85rem;
		padding: 0.75rem 1rem;
		margin-bottom: 1rem;
	}

	.note.ok {
		border-color: var(--accent-green);
		background: color-mix(in srgb, var(--accent-green) 10%, transparent);
		color: var(--accent-green);
	}

	.import-bar {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 1.5rem;
		padding: 1rem 1.1rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface-raised);
	}

	.import-bar p {
		margin-top: 0.35rem;
		color: var(--text-secondary);
		font-size: 0.85rem;
		max-width: 62ch;
	}

	.import-bar code {
		font-family: var(--font-mono);
		font-size: 0.82rem;
	}

	.import-button {
		flex-shrink: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 8.5rem;
		padding: 0.55rem 0.9rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface);
		font-size: 0.85rem;
		font-weight: 600;
		cursor: pointer;
	}

	.import-button input {
		display: none;
	}

	.import-button:has(input:disabled) {
		opacity: 0.6;
		cursor: wait;
	}

	.split {
		display: grid;
		grid-template-columns: minmax(0, 1.1fr) minmax(0, 1fr);
		gap: 1.5rem;
		align-items: start;
	}

	.actions {
		display: flex;
		gap: 0.5rem;
		margin-top: 1rem;
	}

	.save {
		flex: 1;
		color: var(--accent-cyan);
		font-weight: 600;
	}

	.filters {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		margin-bottom: 0.75rem;
		align-items: flex-end;
	}

	.filters select,
	.stake-filter input {
		width: auto;
		min-width: 9rem;
		max-width: none;
	}

	.stake-filter {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.72rem;
		color: var(--text-muted);
	}

	.stake-filter input {
		min-width: 6rem;
	}

	.clear-filters {
		color: var(--accent-cyan);
		font-size: 0.8rem;
		white-space: nowrap;
	}

	.table-meta {
		margin-bottom: 0.5rem;
		font-size: 0.78rem;
		color: var(--text-muted);
	}

	.block {
		margin-top: 1.25rem;
		padding-top: 1.25rem;
		border-top: 1px solid var(--border);
	}

	.block-title {
		font-family: var(--font-sans);
		font-size: 0.78rem;
		font-weight: 600;
		letter-spacing: 0.02em;
		color: var(--text-secondary);
		margin-bottom: 0.75rem;
	}

	.lead {
		font-size: 0.8rem;
		color: var(--text-secondary);
		line-height: 1.6;
		margin-bottom: 0.75rem;
	}

	.send {
		width: 100%;
		margin-top: 1rem;
		color: var(--accent-cyan);
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
		letter-spacing: 0.02em;
		font-size: 0.72rem;
		padding: 0.5rem 0.6rem;
		border-bottom: 1px solid var(--border-strong);
		white-space: nowrap;
	}

	th:first-child,
	th:nth-child(2),
	th:nth-child(3),
	th:nth-child(4),
	th:nth-child(5) {
		text-align: left;
	}

	th.sortable {
		cursor: pointer;
		user-select: none;
	}

	th.sortable:hover {
		color: var(--text-secondary);
	}

	td {
		text-align: right;
		padding: 0.45rem 0.6rem;
		border-bottom: 1px solid var(--border);
		font-family: var(--font-mono);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}

	td.left {
		text-align: left;
		white-space: normal;
	}

	td.price {
		font-weight: 600;
	}

	tr.editing {
		background: color-mix(in srgb, var(--accent-cyan) 8%, transparent);
	}

	.meta {
		display: block;
		font-size: 0.68rem;
		color: var(--text-muted);
	}

	.meta-cell {
		font-family: var(--font-sans);
		font-size: 0.72rem;
		color: var(--text-secondary);
		max-width: 10rem;
		white-space: normal;
	}

	.won {
		color: var(--accent-green);
	}

	.lost {
		color: var(--accent-red);
	}

	.pending {
		color: var(--accent-amber);
	}

	.push {
		color: var(--text-muted);
	}

	.row-actions {
		display: flex;
		gap: 0.3rem;
		justify-content: flex-end;
	}

	.row-actions button {
		padding: 0.15rem 0.45rem;
		font-size: 0.7rem;
		color: var(--text-muted);
	}

	.row-actions button:hover {
		color: var(--accent-cyan);
	}

	.row-actions button.armed {
		color: var(--accent-red);
		border-color: var(--accent-red);
	}

	@media (max-width: 768px) {
		.page {
			padding: 0.5rem 0 2.5rem;
		}

		h1 {
			font-size: 1.35rem;
		}

		.head {
			margin-bottom: 1.15rem;
		}

		.head p {
			font-size: 0.82rem;
		}

		.split {
			grid-template-columns: 1fr;
			gap: 1rem;
		}

		.filters {
			display: grid;
			grid-template-columns: 1fr 1fr;
			gap: 0.55rem;
			align-items: stretch;
		}

		.filters select,
		.stake-filter,
		.stake-filter input,
		.clear-filters {
			width: 100%;
			min-width: 0;
			max-width: none;
		}

		.clear-filters {
			grid-column: 1 / -1;
			justify-self: start;
			white-space: normal;
		}

		.table-wrap {
			margin-inline: -0.15rem;
			padding-bottom: 0.25rem;
			-webkit-overflow-scrolling: touch;
		}

		.bets-table {
			font-size: 0.74rem;
			min-width: 36rem;
		}

		/* Drop secondary columns so the useful ones fit without endless scroll. */
		.bets-table th:nth-child(3),
		.bets-table th:nth-child(4),
		.bets-table th:nth-child(5),
		.bets-table th:nth-child(10),
		.bets-table th:nth-child(11),
		.bets-table td:nth-child(3),
		.bets-table td:nth-child(4),
		.bets-table td:nth-child(5),
		.bets-table td:nth-child(10),
		.bets-table td:nth-child(11) {
			display: none;
		}

		.bets-table th,
		.bets-table td {
			padding: 0.4rem 0.45rem;
		}

		.bets-table td.left:nth-child(2) {
			max-width: 9.5rem;
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
		}

		.meta-cell {
			max-width: 6rem;
		}

		.actions {
			flex-wrap: wrap;
		}

		.save {
			flex: 1 1 100%;
		}
	}

	@media (max-width: 420px) {
		.filters {
			grid-template-columns: 1fr;
		}

		.bets-table {
			min-width: 28rem;
		}
	}
</style>
