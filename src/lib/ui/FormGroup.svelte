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
		gap: 0.28rem;
		min-width: 0;
	}

	.form-group label {
		display: flex;
		flex-direction: column;
		gap: 0.28rem;
	}

	.label {
		font-family: var(--font-sans);
		font-size: 0.8rem;
		font-weight: 500;
		color: var(--text-secondary);
	}

	.hint {
		font-size: 0.75rem;
		color: var(--text-muted);
		line-height: 1.35;
		max-width: 36rem;
	}

	/* Keep segment controls the same footprint as text inputs. */
	.form-group :global(.group) {
		max-width: var(--control-max);
	}
</style>
