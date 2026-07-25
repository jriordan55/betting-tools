<script lang="ts">
	import { DOCS, allTags } from '$lib/docs';

	let filter = $state('');
	let activeTag = $state('');

	const tags = allTags();

	const shown = $derived.by(() => {
		const needle = filter.trim().toLowerCase();
		return DOCS.filter((doc) => {
			if (activeTag !== '' && !doc.tags.includes(activeTag)) return false;
			if (needle === '') return true;
			return (
				doc.title.toLowerCase().includes(needle) ||
				doc.excerpt.toLowerCase().includes(needle) ||
				doc.tags.some((t) => t.includes(needle))
			);
		});
	});

</script>

<svelte:head><title>Reference — Bettor Desktop</title></svelte:head>

<div class="page container-wide">
	<header class="head">
		<h1>Reference</h1>
		<p>
			{DOCS.length} explainers, bundled with the app. No network needed, and each one links to
			the calculators it explains.
		</p>
	</header>

	<div class="controls">
		<input type="text" bind:value={filter} placeholder="Filter articles" aria-label="Filter articles" />
		<div class="tags">
			<button
				type="button"
				class:active={activeTag === ''}
				onclick={() => (activeTag = '')}
			>
				all
			</button>
			{#each tags as tag (tag)}
				<button
					type="button"
					class:active={activeTag === tag}
					onclick={() => (activeTag = activeTag === tag ? '' : tag)}
				>
					{tag}
				</button>
			{/each}
		</div>
	</div>

	{#if shown.length > 0}
		<ul class="list">
			{#each shown as doc (doc.slug)}
				<li>
					<a href="/docs/{doc.slug}">
						<h2>{doc.title}</h2>
						<p>{doc.excerpt}</p>
						<div class="meta">
							<span>{doc.date}</span>
							{#each doc.tags as tag (tag)}<span class="tag">{tag}</span>{/each}
						</div>
					</a>
				</li>
			{/each}
		</ul>
	{:else}
		<p class="empty">Nothing matches that.</p>
	{/if}
</div>

<style>
	.page {
		padding: 2rem 0 4rem;
	}

	.head {
		margin-bottom: 1.5rem;
	}

	h1 {
		font-size: 1.6rem;
		font-weight: 700;
		letter-spacing: -0.01em;
	}

	.head p {
		margin-top: 0.4rem;
		color: var(--text-secondary);
		font-size: 0.9rem;
		max-width: 62ch;
	}

	.controls {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-bottom: 1.5rem;
	}

	.controls input {
		max-width: 22rem;
		text-align: left;
	}

	.tags {
		display: flex;
		flex-wrap: wrap;
		gap: 0.35rem;
	}

	.tags button {
		padding: 0.2rem 0.55rem;
		font-size: 0.7rem;
		color: var(--text-muted);
	}

	.tags button.active {
		color: var(--accent-cyan);
		border-color: var(--accent-cyan);
	}

	.list {
		list-style: none;
		display: grid;
		gap: 0.75rem;
	}

	.list a {
		display: block;
		padding: 1rem 1.1rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--bg-secondary);
		color: inherit;
		transition: border-color 0.15s ease;
	}

	.list a:hover {
		border-color: var(--accent-cyan);
		opacity: 1;
	}

	h2 {
		font-size: 0.95rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.list p {
		margin-top: 0.35rem;
		font-size: 0.82rem;
		color: var(--text-secondary);
		line-height: 1.55;
	}

	.meta {
		margin-top: 0.5rem;
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		font-size: 0.68rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}

	.tag::before {
		content: '#';
		opacity: 0.6;
	}

	.empty {
		color: var(--text-muted);
		font-size: 0.85rem;
	}
</style>
