<script lang="ts">
	import type { Snippet } from 'svelte';

	let { title, children }: { title: string; children: Snippet } = $props();
</script>

<div class="container">
	<div class="title">{title}</div>
	<div class="content">{@render children()}</div>
</div>

<style>
	.container {
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.85rem 0.95rem;
		margin-top: 0.85rem;
		background: var(--bg-secondary);
		box-shadow: var(--shadow-sm);
	}

	.title {
		font-family: var(--font-sans);
		font-size: 0.78rem;
		font-weight: 600;
		letter-spacing: 0.02em;
		color: var(--text-secondary);
		margin-bottom: 0.75rem;
	}

	.content {
		font-size: 0.9rem;
		color: var(--text-secondary);
		line-height: 1.65;
	}

	.content :global(p + p) {
		margin-top: 0.75rem;
	}

	.content :global(code) {
		font-family: var(--font-mono);
		font-size: 0.85em;
		color: var(--accent-cyan);
	}
</style>
