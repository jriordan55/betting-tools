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
		display: flex;
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		overflow: hidden;
	}

	button {
		flex: 1;
		padding: 0.4rem 0.75rem;
		font-family: var(--font-mono);
		font-size: 0.85rem;
		font-weight: 500;
		letter-spacing: 0.02em;
		color: var(--text-muted);
		background: var(--bg-tertiary);
		border: none;
		border-radius: 0;
		cursor: pointer;
		transition: all 0.15s;
		white-space: nowrap;
	}

	button:not(:last-child) {
		border-right: 1px solid var(--border);
	}

	button:hover {
		color: var(--text-secondary);
		background: var(--bg-secondary);
	}

	button.active {
		color: var(--text-primary);
		background: var(--bg-secondary);
		font-weight: 600;
	}

	@media (max-width: 640px) {
		button {
			font-size: 0.75rem;
			padding: 0.4rem 0.5rem;
		}
	}
</style>
