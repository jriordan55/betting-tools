<script lang="ts">
	import { page } from '$app/state';
	import { getCalculator } from '$lib/calculators';
	import { getDoc, renderDoc } from '$lib/docs';

	const doc = $derived(getDoc(page.params.slug ?? ''));
	const html = $derived(doc ? renderDoc(doc) : '');
	const related = $derived((doc?.related ?? []).map(getCalculator).filter((c) => c !== undefined));
</script>

<svelte:head>
	<title>{doc?.title ?? 'Not found'} — Bettor Desktop</title>
</svelte:head>

<div class="page container">
	{#if doc}
		<a class="back" href="/docs">← Reference</a>

		<header class="head">
			<h1>{doc.title}</h1>
			<div class="meta">
				<span>{doc.date}</span>
				{#each doc.tags as tag (tag)}<span class="tag">{tag}</span>{/each}
			</div>
		</header>

		<!--
			The markdown is ours: fifteen files bundled at build time from a repo
			we control. It is not user input and never crosses a network, which is
			what makes rendering it as HTML the right call rather than a risk.
		-->
		<article class="prose">{@html html}</article>

		{#if related.length > 0}
			<footer class="related">
				<div class="related-title">Calculators this explains</div>
				<ul>
					{#each related as calc (calc.slug)}
						<li>
							<a href="/calculators/{calc.slug}">
								<span class="icon">{calc.icon}</span>{calc.title}
							</a>
						</li>
					{/each}
				</ul>
			</footer>
		{/if}
	{:else}
		<p class="missing">No article with that name. <a href="/docs">Back to the reference</a>.</p>
	{/if}
</div>

<style>
	.page {
		padding: 2rem 0 4rem;
	}

	.back {
		font-size: 0.78rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}

	.back:hover {
		color: var(--accent-cyan);
	}

	.head {
		margin: 1rem 0 2rem;
	}

	h1 {
		font-size: 1.7rem;
		font-weight: 700;
		line-height: 1.25;
		letter-spacing: -0.01em;
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

	.related {
		margin-top: 3rem;
		padding-top: 1.25rem;
		border-top: 1px solid var(--border);
	}

	.related-title {
		font-family: var(--font-mono);
		font-size: 0.7rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		margin-bottom: 0.6rem;
	}

	.related ul {
		list-style: none;
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.related a {
		display: inline-flex;
		align-items: center;
		gap: 0.45rem;
		padding: 0.4rem 0.7rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		font-size: 0.8rem;
		color: var(--text-primary);
	}

	.related a:hover {
		border-color: var(--accent-cyan);
		opacity: 1;
	}

	.icon {
		font-family: var(--font-mono);
		font-size: 0.72rem;
		color: var(--accent-cyan);
	}

	.missing {
		color: var(--text-muted);
	}
</style>
