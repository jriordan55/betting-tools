<script lang="ts">
	import type { ResultColor } from './types';

	let {
		value,
		label,
		color = 'default'
	}: { value: string; label: string; color?: ResultColor } = $props();
</script>

<div class="container">
	<div class="value {color}" data-selectable>{value}</div>
	<div class="label">{label}</div>
</div>

<style>
	.container {
		text-align: center;
		padding: 0.85rem 0;
		margin-bottom: 0.35rem;
	}

	.value {
		font-family: var(--font-mono);
		font-size: 1.85rem;
		font-weight: 700;
		font-variant-numeric: tabular-nums;
		line-height: 1.15;
		letter-spacing: -0.02em;
		color: var(--text-primary);
	}

	.label {
		font-size: 0.8rem;
		font-weight: 500;
		color: var(--text-muted);
		margin-top: 0.4rem;
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

	@media (max-width: 640px) {
		.value {
			font-size: 1.75rem;
		}
	}
</style>
