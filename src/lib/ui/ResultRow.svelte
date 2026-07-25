<script lang="ts">
	import type { ResultColor } from './types';

	let {
		label,
		value,
		color = 'default',
		hint
	}: { label: string; value: string; color?: ResultColor; hint?: string } = $props();
</script>

<div class="row">
	<span class="label">
		{label}
		{#if hint}<span class="hint">{hint}</span>{/if}
	</span>
	<span class="value {color}" data-selectable>{value}</span>
</div>

<style>
	.row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
		padding: 0.625rem 0;
		border-bottom: 1px solid var(--border);
	}

	.row:last-child {
		border-bottom: none;
	}

	.label {
		font-size: 1rem;
		color: var(--text-secondary);
	}

	.hint {
		display: block;
		font-size: 0.78rem;
		color: var(--text-muted);
		line-height: 1.4;
	}

	.value {
		font-family: var(--font-mono);
		font-size: 1rem;
		font-weight: 600;
		color: var(--text-primary);
		text-align: right;
		white-space: nowrap;
	}

	.positive {
		color: var(--accent-green);
	}
	.negative {
		color: var(--accent-red);
	}
	.highlight {
		color: var(--accent-cyan);
	}
	.amber {
		color: var(--accent-amber);
	}
	.blue {
		color: var(--accent-blue);
	}
	.muted {
		color: var(--text-muted);
	}
</style>
