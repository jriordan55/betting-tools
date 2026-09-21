<script lang="ts">
	import '@fontsource/ibm-plex-sans/400.css';
	import '@fontsource/ibm-plex-sans/500.css';
	import '@fontsource/ibm-plex-sans/600.css';
	import '@fontsource/ibm-plex-sans/700.css';
	import '@fontsource/jetbrains-mono/400.css';
	import '@fontsource/jetbrains-mono/500.css';
	import '@fontsource/jetbrains-mono/600.css';
	import '@fontsource/jetbrains-mono/700.css';
	import 'katex/dist/katex.min.css';
	import '../app.css';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { byCategory, CATEGORY_LABELS } from '$lib/calculators';
	import { betLog } from '$lib/betlog-store.svelte';
	import { theme } from '$lib/theme.svelte';

	let { children } = $props();

	let filter = $state('');
	let collapsed = $state(false);

	onMount(() => {
		theme.load();
		void betLog.refresh();
	});

	const summary = $derived(betLog.snapshot?.summary ?? null);

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
				<span class="logo-text">Bettor<span class="logo-dim"> Desktop</span></span>
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
				{#if summary && summary.bets > 0}
					<div class="nav-record">
						<span class={summary.profit >= 0 ? 'won' : 'lost'}>
							{summary.profit >= 0 ? '+' : ''}{summary.profit.toFixed(0)}
						</span>
						<span>{summary.won}–{summary.lost}</span>
						<span>{summary.bets} bets</span>
					</div>
				{/if}
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
		grid-template-columns: clamp(190px, 20vw, 260px) minmax(0, 1fr);
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
		gap: 0.85rem;
		border-right: 1px solid var(--border);
		background: var(--bg-secondary);
		padding: 1rem 0.7rem 0.85rem;
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
		padding: 0.15rem 0.25rem 0.35rem;
	}

	.collapse {
		flex: 0 0 auto;
		padding: 0.2rem 0.4rem;
		font-size: 0.8rem;
		line-height: 1;
		color: var(--text-muted);
		background: none;
		border-color: transparent;
	}

	.collapse:hover {
		background: var(--bg-tertiary);
		color: var(--accent-cyan);
		border-color: transparent;
	}

	.reveal {
		margin-bottom: 1rem;
		font-size: 0.8rem;
		color: var(--accent-cyan);
		padding: 0.35rem 0.7rem;
		background: var(--accent-cyan-soft);
		border-color: transparent;
	}

	.reveal:hover {
		border-color: var(--accent-cyan);
	}

	.logo {
		display: flex;
		align-items: center;
		gap: 0.55rem;
		min-width: 0;
		color: var(--text-primary);
	}

	.logo:hover {
		color: var(--text-primary);
		opacity: 1;
	}

	.logo-mark {
		display: grid;
		place-items: center;
		width: 24px;
		height: 24px;
		border-radius: 7px;
		background: var(--accent-cyan);
		color: var(--bg-primary);
		font-weight: 700;
		font-size: 0.82rem;
		box-shadow: var(--shadow-sm);
	}

	.logo-text {
		font-size: 0.92rem;
		font-weight: 600;
		letter-spacing: -0.02em;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.logo-dim {
		color: var(--text-muted);
		font-weight: 500;
	}

	.search input {
		font-family: var(--font-sans);
		font-size: 0.8rem;
		height: 1.9rem;
		max-width: none;
		padding: 0.3rem 0.55rem;
		background: var(--bg-primary);
	}

	nav {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.group-title {
		font-size: 0.68rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		padding: 0 0.55rem;
		margin-bottom: 0.35rem;
	}

	.nav-link {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.38rem 0.55rem;
		border-radius: var(--radius-sm);
		color: var(--text-secondary);
		font-size: 0.82rem;
		line-height: 1.3;
		transition: background 0.12s ease, color 0.12s ease;
	}

	.nav-link:hover {
		background: var(--bg-tertiary);
		color: var(--text-primary);
	}

	.nav-link.active {
		background: var(--accent-cyan-soft);
		color: var(--accent-cyan);
		font-weight: 600;
	}

	.nav-icon {
		flex: 0 0 1.6rem;
		font-family: var(--font-mono);
		font-size: 0.68rem;
		font-weight: 600;
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

	.nav-record {
		display: flex;
		flex-wrap: wrap;
		gap: 0.35rem 0.55rem;
		padding: 0.35rem 0.55rem 0.15rem 2rem;
		font-family: var(--font-mono);
		font-size: 0.68rem;
		color: var(--text-muted);
	}

	.nav-record .won {
		color: var(--accent-green);
	}

	.nav-record .lost {
		color: var(--accent-red);
	}

	.no-match {
		font-size: 0.8rem;
		color: var(--text-muted);
		padding: 0.5rem;
	}

	.theme-toggle {
		font-size: 0.78rem;
		padding: 0.45rem 0.7rem;
		color: var(--text-secondary);
		background: var(--bg-primary);
	}

	main {
		padding-top: 1.75rem;
		padding-bottom: 4rem;
		min-width: 0;
	}
</style>
