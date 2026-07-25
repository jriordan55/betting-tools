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
		padding: 1.25rem;
		margin-top: 1rem;
		background: var(--bg-secondary);
	}

	.title {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--text-muted);
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
