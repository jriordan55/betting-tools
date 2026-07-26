<script lang="ts" generics="T extends string">
	interface ToggleOption {
		value: T;
		label: string;
	}

	let {
		options,
		value = $bindable(),
		onchange
	}: { options: readonly ToggleOption[]; value: T; onchange?: (value: T) => void } = $props();

	function select(next: T) {
		value = next;
		onchange?.(next);
	}
</script>

<div class="group">
	{#each options as option (option.value)}
		<button
			type="button"
			aria-pressed={value === option.value}
			class:active={value === option.value}
			onclick={() => select(option.value)}
		>
			{option.label}
		</button>
	{/each}
</div>

<style>
	.group {
		display: inline-flex;
		width: 100%;
		max-width: var(--control-max);
		height: var(--control-height);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		overflow: hidden;
		background: var(--bg-primary);
	}

	button {
		flex: 1;
		padding: 0 0.65rem;
		height: 100%;
		font-family: var(--font-sans);
		font-size: 0.8rem;
		font-weight: 500;
		line-height: 1;
		color: var(--text-muted);
		background: transparent;
		border: none;
		border-radius: 0;
		cursor: pointer;
		transition: background 0.12s ease, color 0.12s ease;
		white-space: nowrap;
	}

	button:not(:last-child) {
		border-right: 1px solid var(--border);
	}

	button:hover {
		color: var(--text-secondary);
		background: var(--bg-tertiary);
	}

	button.active {
		color: var(--accent-cyan);
		background: var(--accent-cyan-soft);
		font-weight: 600;
	}
</style>
