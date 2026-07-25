<script lang="ts">
	let { icon, message }: { icon: string; message: string } = $props();
</script>

<div class="container">
	<div class="icon">{icon}</div>
	<p class="message">{message}</p>
</div>

<style>
	.container {
		text-align: center;
		padding: 2.5rem 1rem;
	}

	.icon {
		font-family: var(--font-mono);
		font-size: 1.65rem;
		font-weight: 700;
		color: var(--text-muted);
		margin-bottom: 0.75rem;
		opacity: 0.5;
	}

	.message {
		font-size: 0.9rem;
		color: var(--text-muted);
		line-height: 1.5;
	}
</style>
