<script lang="ts">
	import { onMount } from 'svelte';
	import { commands, type EngineInfo } from '$lib/bindings';
	import { byCategory, CATEGORY_LABELS, CALCULATORS } from '$lib/calculators';

	let info = $state<EngineInfo | null>(null);

	onMount(async () => {
		info = await commands.engineInfo();
	});

	const groups = byCategory();
</script>

<div class="container-wide">
	<header class="hero">
		<h1>Bettor Desktop</h1>
		<p class="tagline">
			{CALCULATORS.length} calculators. Every number computed in Rust — the same engine the parity
			suite tests, not a second implementation in the UI.
		</p>
		{#if info}
			<p class="engine">
				<span class="dot" class:warn={!info.optimized}></span>
				bettor-core {info.coreVersion} · shell {info.shellVersion} ·
				{info.optimized ? 'optimized' : 'debug build — simulations will be slow'}
			</p>
		{/if}
	</header>

	{#each groups as group (group.category)}
		<section>
			<h2>{CATEGORY_LABELS[group.category]}</h2>
			<div class="grid">
				{#each group.items as calc (calc.slug)}
					<a class="card" href="/calculators/{calc.slug}">
						<div class="card-icon">{calc.icon}</div>
						<div class="card-body">
							<div class="card-title">{calc.title}</div>
							<div class="card-desc">{calc.description}</div>
						</div>
					</a>
				{/each}
			</div>
		</section>
	{/each}
</div>

<style>
	.hero {
		margin-bottom: 2.5rem;
	}

	h1 {
		font-size: 1.9rem;
		font-weight: 700;
		letter-spacing: -0.01em;
	}

	.tagline {
		color: var(--text-secondary);
		font-size: 0.9rem;
		max-width: 62ch;
		margin-top: 0.4rem;
	}

	.engine {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		margin-top: 0.85rem;
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--accent-green);
	}

	.dot.warn {
		background: var(--accent-amber);
	}

	section {
		margin-bottom: 2rem;
	}

	h2 {
		font-size: 0.72rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.12em;
		color: var(--text-muted);
		margin-bottom: 0.75rem;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 0.75rem;
	}

	.card {
		display: flex;
		gap: 0.85rem;
		align-items: flex-start;
		padding: 0.9rem 1rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--bg-secondary);
		color: var(--text-primary);
		transition: border-color 0.15s ease, background 0.15s ease;
	}

	.card:hover {
		opacity: 1;
		border-color: var(--accent-cyan);
		background: var(--bg-tertiary);
	}

	.card-icon {
		flex: 0 0 2.1rem;
		height: 2.1rem;
		display: grid;
		place-items: center;
		border-radius: var(--radius-sm);
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		font-size: 0.72rem;
		font-weight: 700;
		color: var(--accent-cyan);
	}

	.card:hover .card-icon {
		background: var(--bg-secondary);
	}

	.card-body {
		min-width: 0;
	}

	.card-title {
		font-size: 0.88rem;
		font-weight: 600;
		margin-bottom: 0.2rem;
	}

	.card-desc {
		font-size: 0.76rem;
		color: var(--text-muted);
		line-height: 1.5;
	}
</style>
