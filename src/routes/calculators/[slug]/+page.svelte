<script lang="ts">
	import { page } from '$app/state';
	import type { Component } from 'svelte';
	import { getCalculator } from '$lib/calculators';

	const calc = $derived(getCalculator(page.params.slug ?? ''));

	// Keyed on the slug so navigating between calculators re-runs the import
	// rather than leaving the previous component mounted.
	const loader = $derived(
		calc ? calc.load().then((m) => m.default) : Promise.resolve<Component | null>(null)
	);
</script>

<div class="container">
	{#if !calc}
		<div class="missing">
			<h1>Not found</h1>
			<p>There is no calculator at <code>/calculators/{page.params.slug}</code>.</p>
			<p><a href="/">← Back to all calculators</a></p>
		</div>
	{:else}
		<header class="head">
			<a class="back" href="/">← All calculators</a>
			<h1>{calc.title}</h1>
			<p class="subtitle">{calc.description}</p>
		</header>

		{#await loader}
			<p class="loading">Loading…</p>
		{:then Calculator}
			{#if Calculator}
				<Calculator />
			{/if}
		{:catch error}
			<p class="failed">This calculator failed to load: {error.message}</p>
		{/await}
	{/if}
</div>

<style>
	.head {
		margin-bottom: 1.5rem;
	}

	.back {
		display: inline-block;
		font-size: 0.75rem;
		color: var(--text-muted);
		margin-bottom: 0.6rem;
	}

	.back:hover {
		color: var(--accent-cyan);
		opacity: 1;
	}

	h1 {
		font-size: 1.5rem;
		font-weight: 700;
		letter-spacing: -0.01em;
	}

	.subtitle {
		color: var(--text-secondary);
		font-size: 0.85rem;
		max-width: 68ch;
		margin-top: 0.3rem;
	}

	.loading,
	.failed {
		color: var(--text-muted);
		font-size: 0.85rem;
		padding: 2rem 0;
	}

	.failed {
		color: var(--accent-red);
	}

	.missing {
		padding: 3rem 0;
	}

	.missing h1 {
		margin-bottom: 0.5rem;
	}

	.missing p {
		color: var(--text-secondary);
		font-size: 0.9rem;
		margin-bottom: 0.4rem;
	}

	code {
		font-family: var(--font-mono);
		color: var(--accent-cyan);
	}
</style>
