<script lang="ts">
	import type { MathError } from '$lib/bindings';
	import { describeError } from '$lib/errors';

	let { error }: { error: MathError | string | null } = $props();

	let message = $derived(
		error === null ? '' : typeof error === 'string' ? error : describeError(error)
	);
</script>

{#if message}
	<div class="note" role="alert">{message}</div>
{/if}

<style>
	.note {
		border: 1px solid var(--accent-red);
		border-radius: var(--radius);
		background: color-mix(in srgb, var(--accent-red) 10%, transparent);
		color: var(--accent-red);
		font-size: 0.85rem;
		line-height: 1.5;
		padding: 0.75rem 1rem;
		margin-bottom: 1rem;
	}
</style>
