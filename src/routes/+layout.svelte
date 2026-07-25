<script lang="ts">
	import 'katex/dist/katex.min.css';
	import '../app.css';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { byCategory, CATEGORY_LABELS } from '$lib/calculators';
	import { theme } from '$lib/theme.svelte';

	let { children } = $props();

	let filter = $state('');
	let collapsed = $state(false);

	onMount(() => theme.load());

	const groups = $derived.by(() => {
		const needle = filter.trim().toLowerCase();
		return byCategory()
			.map((group) => ({
				...group,
				items: needle
					? group.items.filter(
							(c) =>
								c.title.toLowerCase().includes(needle) ||
								c.description.toLowerCase().includes(needle)
						)
					: group.items
			}))
			.filter((group) => group.items.length > 0);
	});

	let currentSlug = $derived(page.params.slug ?? '');
</script>

<div class="shell" class:collapsed>
	<aside class="sidebar">
		<div class="top">
			<a href="/" class="logo">
				<span class="logo-mark">B</span>
				<span class="logo-text">BETTOR<span class="logo-dim">·DESKTOP</span></span>
			</a>
			<button
				class="collapse"
				type="button"
				onclick={() => (collapsed = true)}
				aria-label="Hide the calculator list"
				title="Hide the calculator list"
			>
				«
			</button>
		</div>

		<div class="search">
			<input
				type="text"
				placeholder="Filter calculators"
				bind:value={filter}
				aria-label="Filter calculators"
			/>
		</div>

		<nav>
			<!-- Neither of these is a calculator, and both belong above them. -->
			<div class="group">
				<div class="group-title">Your book</div>
				<a
					class="nav-link"
					class:active={page.url.pathname === '/bet-log'}
					href="/bet-log"
					title="Every bet you record, priced against its closing line"
				>
					<span class="nav-icon">$</span>
					<span class="nav-label">Bet Log</span>
				</a>
				<a
					class="nav-link"
					class:active={page.url.pathname.startsWith('/docs')}
					href="/docs"
					title="Explainers for every calculator, bundled with the app"
				>
					<span class="nav-icon">?</span>
					<span class="nav-label">Reference</span>
				</a>
			</div>

			{#each groups as group (group.category)}
				<div class="group">
					<div class="group-title">{CATEGORY_LABELS[group.category]}</div>
					{#each group.items as calc (calc.slug)}
						<a
							class="nav-link"
							class:active={currentSlug === calc.slug}
							href="/calculators/{calc.slug}"
							title={calc.description}
						>
							<span class="nav-icon">{calc.icon}</span>
							<span class="nav-label">{calc.title}</span>
						</a>
					{/each}
				</div>
			{:else}
				<p class="no-match">No calculator matches “{filter}”.</p>
			{/each}
		</nav>

		<button class="theme-toggle" type="button" onclick={() => theme.toggle()}>
			{theme.current === 'dark' ? '☾ Dark' : '☀ Light'}
		</button>
	</aside>

	<main>
		{#if collapsed}
			<button
				class="reveal"
				type="button"
				onclick={() => (collapsed = false)}
				aria-label="Show the calculator list"
			>
				» Calculators
			</button>
		{/if}
		{@render children()}
	</main>
</div>

<style>
	/* A desktop window is never a phone. The sidebar stays a sidebar at every
	   width — it narrows, and it can be collapsed outright, but it never
	   stacks above the content, which pushed every calculator below the fold. */
	.shell {
		display: grid;
		grid-template-columns: clamp(178px, 20vw, 250px) minmax(0, 1fr);
		min-height: 100vh;
	}

	/* `display: none` takes the sidebar out of the grid, so `main` becomes the
	   first item. The column list must shrink to match — leaving a `0` column
	   in front puts every calculator inside it. */
	.shell.collapsed {
		grid-template-columns: minmax(0, 1fr);
	}

	.shell.collapsed .sidebar {
		display: none;
	}

	.sidebar {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		border-right: 1px solid var(--border);
		background: var(--bg-secondary);
		padding: 1rem 0.75rem;
		height: 100vh;
		position: sticky;
		top: 0;
		overflow-y: auto;
	}

	.top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.4rem;
		padding-bottom: 0.25rem;
	}

	.collapse {
		flex: 0 0 auto;
		padding: 2px 6px;
		font-size: 0.75rem;
		line-height: 1;
		color: var(--text-muted);
		background: none;
		border-color: transparent;
	}

	.collapse:hover {
		background: var(--bg-tertiary);
		color: var(--accent-cyan);
	}

	.reveal {
		margin-bottom: 1rem;
		font-size: 0.75rem;
		color: var(--accent-cyan);
		padding: 4px 10px;
	}

	.logo {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-width: 0;
		padding-left: 0.25rem;
		color: var(--text-primary);
	}

	.logo:hover {
		opacity: 1;
	}

	.logo-mark {
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		border-radius: 3px;
		background: var(--accent-cyan);
		color: var(--bg-primary);
		font-weight: 700;
		font-size: 0.8rem;
	}

	.logo-text {
		font-size: 0.85rem;
		font-weight: 700;
		letter-spacing: 0.08em;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.logo-dim {
		color: var(--text-muted);
		font-weight: 500;
	}

	.search input {
		text-align: left;
		font-size: 0.78rem;
		padding: 6px 10px;
	}

	nav {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.9rem;
	}

	.group-title {
		font-size: 0.65rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.12em;
		color: var(--text-muted);
		padding: 0 0.5rem;
		margin-bottom: 0.3rem;
	}

	.nav-link {
		display: flex;
		align-items: center;
		gap: 0.55rem;
		padding: 0.3rem 0.5rem;
		border-radius: var(--radius-sm);
		color: var(--text-secondary);
		font-size: 0.8rem;
		line-height: 1.35;
		transition: background 0.12s ease, color 0.12s ease;
	}

	.nav-link:hover {
		opacity: 1;
		background: var(--bg-tertiary);
		color: var(--text-primary);
	}

	.nav-link.active {
		background: var(--bg-tertiary);
		color: var(--accent-cyan);
		font-weight: 600;
	}

	.nav-icon {
		flex: 0 0 1.9rem;
		font-size: 0.68rem;
		font-weight: 700;
		color: var(--text-muted);
		text-align: center;
	}

	.nav-link.active .nav-icon {
		color: var(--accent-cyan);
	}

	.nav-label {
		flex: 1;
		min-width: 0;
	}

	.no-match {
		font-size: 0.8rem;
		color: var(--text-muted);
		padding: 0.5rem;
	}

	.theme-toggle {
		font-size: 0.72rem;
		padding: 5px 10px;
		color: var(--text-secondary);
	}

	main {
		padding-top: 1.75rem;
		padding-bottom: 4rem;
		min-width: 0;
	}

</style>
