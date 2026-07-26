<script lang="ts">
	import type { Snippet } from 'svelte';

	let { title, children }: { title: string; children: Snippet } = $props();
</script>

<div class="section">
	<div class="header">{title}</div>
	<div class="body">{@render children()}</div>
</div>

<style>
	.section {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		overflow: hidden;
		margin-bottom: 1rem;
		box-shadow: var(--shadow-sm);
	}

	.header {
		font-family: var(--font-sans);
		font-size: 0.78rem;
		font-weight: 600;
		letter-spacing: 0.02em;
		color: var(--text-secondary);
		padding: 0.55rem 0.9rem;
		border-bottom: 1px solid var(--border);
		background: var(--bg-tertiary);
	}

	.body {
		padding: 0.85rem 0.9rem 0.95rem;
	}
</style>
