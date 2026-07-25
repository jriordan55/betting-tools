<script lang="ts">
	import type { Snippet } from 'svelte';

	let { children, min = 200 }: { children: Snippet; min?: number } = $props();
</script>

<div class="form-row" style="--min-col: {min}px">{@render children()}</div>

<style>
	.form-row {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(var(--min-col), 1fr));
		gap: 1rem;
		margin-bottom: 1rem;
	}

	.form-row:last-child {
		margin-bottom: 0;
	}
</style>
