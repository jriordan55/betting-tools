<script lang="ts">
	let { message }: { message: string } = $props();
</script>

<p class="from-log">{message}</p>

<style>
	.from-log {
		margin: 0 0 1rem;
		padding: 0.65rem 0.85rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: color-mix(in srgb, var(--accent-cyan) 8%, transparent);
		color: var(--text-secondary);
		font-size: 0.8rem;
		line-height: 1.5;
	}
</style>
