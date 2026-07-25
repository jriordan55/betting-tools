<script lang="ts">
	import { commands, type BookRow, type MathError, type OddsFormat } from '$lib/bindings';
	import { Async } from '$lib/async.svelte';
	import { parseOdds, fairOdds, SIMPLE_FORMAT_OPTIONS, ODDS_PLACEHOLDER, type FairOdds } from '$lib/odds';
	import { describeError } from '$lib/errors';
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

	const MAX_BOOKS = 8;

	interface BookEntry {
		name: string;
		oddsA: string;
		oddsB: string;
	}

	let format = $state<OddsFormat>('american');
	let books = $state<BookEntry[]>([
		{ name: 'Book 1', oddsA: '', oddsB: '' },
		{ name: 'Book 2', oddsA: '', oddsB: '' },
		{ name: 'Book 3', oddsA: '', oddsB: '' }
	]);

	interface VigView {
		rows: BookRow[];
		sharpest: BookRow | null;
		fairA: FairOdds | null;
		fairB: FairOdds | null;
		/** Hold at the worst priced book that could still be read. */
		worstHold: number | null;
		/** Books whose quotes could not be parsed, kept so they do not vanish. */
		unreadable: { name: string; error: MathError }[];
	}

	const result = new Async<
		{ books: BookEntry[]; format: OddsFormat },
		{ data: VigView | null; error: MathError | null }
	>(
		() => ({ books: books.map((b) => ({ ...b })), format }),
		async ({ books, format }) => {
			const inputs = [];
			const unreadable: { name: string; error: MathError }[] = [];

			for (const [index, book] of books.entries()) {
				const name = book.name.trim() || `Book ${index + 1}`;
				if (book.oddsA.trim() === '' || book.oddsB.trim() === '') continue;

				// One bad quote must not blank the whole comparison. The book
				// stays in the table with its reason — a book that silently
				// disappears looks like a book you never entered.
				const a = await parseOdds(book.oddsA, format);
				const b = await parseOdds(book.oddsB, format);
				if (a.error || b.error) {
					unreadable.push({ name, error: (a.error ?? b.error)! });
					continue;
				}
				if (a.decimal === null || b.decimal === null) continue;

				const [viewA, viewB] = await Promise.all([
					commands.fromDecimal(a.decimal),
					commands.fromDecimal(b.decimal)
				]);
				if (viewA.status === 'error' || viewB.status === 'error') {
					unreadable.push({
						name,
						error: viewA.status === 'error' ? viewA.error : (viewB as { error: MathError }).error
					});
					continue;
				}

				inputs.push({
					name,
					impliedA: viewA.data.probability,
					impliedB: viewB.data.probability
				});
			}

			if (inputs.length < 2) return { data: null, error: null };

			// The core returns these sorted by ascending hold, failures last.
			const rows = await commands.compareVig(inputs);
			const priced = rows.filter((r) => r.hold !== null);
			const sharpest = priced[0] ?? null;
			const worst = priced[priced.length - 1] ?? null;

			let fairA: FairOdds | null = null;
			let fairB: FairOdds | null = null;
			if (sharpest?.hold) {
				[fairA, fairB] = await Promise.all([
					fairOdds(sharpest.hold.noVigProbA),
					fairOdds(sharpest.hold.noVigProbB)
				]);
			}

			return {
				data: { rows, sharpest, fairA, fairB, worstHold: worst?.hold?.hold ?? null, unreadable },
				error: null
			};
		}
	);

	function addBook() {
		if (books.length < MAX_BOOKS) {
			books = [...books, { name: `Book ${books.length + 1}`, oddsA: '', oddsB: '' }];
		}
	}

	function removeBook(index: number) {
		if (books.length > 2) books = books.filter((_, i) => i !== index);
	}

	function fairText(fair: FairOdds | null, prob: number) {
		if (!fair) return pct(prob, 1);
		return fair.decimal === null
			? `${american(fair.american)} (${pct(prob, 1)})`
			: `${american(fair.american)} · ${num(fair.decimal, 3)} (${pct(prob, 1)})`;
	}

	function rankColor(index: number, total: number) {
		if (index === 0) return 'positive' as const;
		if (index === total - 1) return 'negative' as const;
		return 'default' as const;
	}
</script>

<InputCard title="Settings">
	<FormRow>
		<FormGroup label="Odds Format">
			<ToggleGroup options={SIMPLE_FORMAT_OPTIONS} bind:value={format} />
		</FormGroup>
	</FormRow>
</InputCard>

<InputCard title="Books">
	{#each books as book, index (index)}
		<div class="book">
			<div class="book-head">
				<span class="book-title">Book {index + 1}</span>
				<button
					type="button"
					class="remove"
					onclick={() => removeBook(index)}
					disabled={books.length <= 2}
				>
					remove
				</button>
			</div>
			<FormRow min={140}>
				<FormGroup label="Name">
					<input type="text" bind:value={book.name} placeholder="Book {index + 1}" />
				</FormGroup>
				<FormGroup label="Side A">
					<input type="text" bind:value={book.oddsA} placeholder={ODDS_PLACEHOLDER[format]} />
				</FormGroup>
				<FormGroup label="Side B">
					<input type="text" bind:value={book.oddsB} placeholder={ODDS_PLACEHOLDER[format]} />
				</FormGroup>
			</FormRow>
		</div>
	{/each}
	{#if books.length < MAX_BOOKS}
		<button type="button" class="add" onclick={addBook}>+ Add Book</button>
	{/if}
</InputCard>

<OutputSection title="Vig Comparison">
	<ErrorNote error={result.current?.error ?? null} />
	{#if result.current?.data?.sharpest?.hold}
		{@const view = result.current.data}
		{@const best = view.sharpest!}
		{@const bestHold = best.hold!}
		<ResultLarge value={pct(bestHold.hold)} label="Lowest vig — {best.name}" color="positive" />

		{#if view.worstHold !== null && view.worstHold !== bestHold.hold}
			<ResultRow
				label="Worst book on this market"
				value={pct(view.worstHold)}
				color="negative"
				hint="Every bet placed there gives up the difference before the game starts"
			/>
		{/if}

		<div class="block">
			<div class="block-title">Fair line, from the sharpest book</div>
			<ResultRow
				label="Side A"
				value={fairText(view.fairA, bestHold.noVigProbA)}
				color="highlight"
			/>
			<ResultRow
				label="Side B"
				value={fairText(view.fairB, bestHold.noVigProbB)}
				color="highlight"
			/>
		</div>

		<div class="block">
			<div class="block-title">Ranking — lowest vig first</div>
			{#each view.rows as row, index (row.name + index)}
				{#if row.hold}
					<ResultRow
						label="{index + 1}. {row.name}"
						value="{pct(row.hold.hold)} vig"
						color={rankColor(index, view.rows.length)}
					/>
				{:else if row.error}
					<ResultRow
						label={row.name}
						value="unreadable"
						color="muted"
						hint={describeError(row.error)}
					/>
				{/if}
			{/each}
			{#each view.unreadable as book (book.name)}
				<ResultRow
					label={book.name}
					value="unreadable"
					color="muted"
					hint={describeError(book.error)}
				/>
			{/each}
		</div>
	{:else if !result.current?.error}
		<EmptyState icon="V" message="Enter both sides at two or more books to compare the vig" />
	{/if}
</OutputSection>

<InfoSection title="About Vig Comparison">
	<p>
		Books charge different margins on the same market, and the difference compounds over hundreds of
		bets. The sharpest book is also the best estimate of the true price — its de-vigged line is what
		the other books' prices should be measured against.
	</p>
	<p>
		A book whose prices cannot be read stays in the table with the reason, rather than vanishing
		from it. A book that silently disappears looks like a book you never entered.
	</p>
</InfoSection>

<style>
	.book {
		margin-bottom: 1rem;
	}

	.book:last-of-type {
		margin-bottom: 0;
	}

	.book-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.5rem;
	}

	.book-title,
	.block-title {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
	}

	.remove {
		background: none;
		border: 1px solid transparent;
		color: var(--text-muted);
		font-size: 0.72rem;
		padding: 2px 6px;
	}

	.remove:hover:not(:disabled) {
		background: none;
		color: var(--accent-red);
		border-color: var(--accent-red);
	}

	.add {
		margin-top: 0.85rem;
		width: 100%;
		color: var(--accent-cyan);
		font-size: 0.85rem;
	}

	.block {
		margin-top: 1.25rem;
		padding-top: 1.25rem;
		border-top: 1px solid var(--border);
	}

	.block-title {
		display: block;
		margin-bottom: 0.75rem;
	}
</style>
