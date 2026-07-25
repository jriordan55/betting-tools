<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		label,
		hint,
		children
	}: { label: string; hint?: string; children: Snippet } = $props();
</script>

<div class="form-group">
	<!-- svelte-ignore a11y_label_has_associated_control -->
	<label>
		<span class="label">{label}</span>
		{@render children()}
	</label>
	{#if hint}
		<span class="hint">{hint}</span>
	{/if}
</div>

<style>
	.form-group {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.form-group label {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.label {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--text-secondary);
	}

	.hint {
		font-size: 0.8rem;
		color: var(--text-muted);
	}
</style>
