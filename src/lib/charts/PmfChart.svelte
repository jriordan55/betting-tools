<script lang="ts">
	import { scaleLinear } from 'd3';
	import type { PmfPoint } from '$lib/bindings';

	export interface Band {
		/** Everything at or beyond this value is shaded. */
		from: number;
		to: number;
		color: string;
		label: string;
	}

	let {
		data,
		bands = [],
		line = null,
		height = 240,
		color = 'var(--accent-cyan)',
		highlight = () => false,
		formatX = (v: number) => String(v),
		labelX = ''
	}: {
		data: PmfPoint[];
		/** Shaded regions, for grading a line against the distribution. */
		bands?: Band[];
		/** A vertical rule — the line being graded. */
		line?: number | null;
		height?: number;
		color?: string;
		/** Bars to draw in the accent colour, e.g. football's key numbers. */
		highlight?: (k: number) => boolean;
		formatX?: (v: number) => string;
		labelX?: string;
	} = $props();

	let width = $state(640);
	let hovered = $state<PmfPoint | null>(null);

	const margin = { top: 12, right: 16, bottom: 30, left: 46 };

	const inner = $derived({
		width: Math.max(10, width - margin.left - margin.right),
		height: Math.max(10, height - margin.top - margin.bottom)
	});

	/**
	 * Trim the flat tails before scaling. A normal over ±4σ spends most of its
	 * width at visually zero height, which squashes the part anyone is looking
	 * at into the middle third of the chart.
	 */
	const shown = $derived.by(() => {
		if (data.length === 0) return [];
		const peak = Math.max(...data.map((d) => d.p));
		const floor = peak * 0.002;
		const first = data.findIndex((d) => d.p >= floor);
		if (first === -1) return data;
		let last = data.length - 1;
		while (last > first && data[last].p < floor) last -= 1;
		return data.slice(first, last + 1);
	});

	const domain = $derived.by(() => {
		if (shown.length === 0) return { x: [0, 1] as [number, number], y: 1 };
		return {
			x: [shown[0].k - 0.5, shown[shown.length - 1].k + 0.5] as [number, number],
			y: Math.max(...shown.map((d) => d.p))
		};
	});

	const xScale = $derived(scaleLinear().domain(domain.x).range([0, inner.width]));
	const yScale = $derived(scaleLinear().domain([0, domain.y * 1.08]).range([inner.height, 0]));

	const barWidth = $derived(
		shown.length > 1 ? Math.max(1, inner.width / shown.length - 1) : inner.width / 2
	);

	const xTicks = $derived(xScale.ticks(Math.min(10, Math.max(2, shown.length))));

	function onMove(event: MouseEvent) {
		if (shown.length === 0) return;
		const bounds = (event.currentTarget as SVGRectElement).getBoundingClientRect();
		const value = xScale.invert(event.clientX - bounds.left);
		let closest = shown[0];
		for (const point of shown) {
			if (Math.abs(point.k - value) < Math.abs(closest.k - value)) closest = point;
		}
		hovered = closest;
	}
</script>

<div class="chart" bind:clientWidth={width}>
	<svg {width} {height} role="img" aria-label="Probability by outcome">
		<g transform="translate({margin.left},{margin.top})">
			{#each bands as band (band.label)}
				{@const x0 = xScale(Math.max(band.from, domain.x[0]))}
				{@const x1 = xScale(Math.min(band.to, domain.x[1]))}
				{#if x1 > x0}
					<rect
						x={x0}
						y="0"
						width={x1 - x0}
						height={inner.height}
						style="fill: {band.color}"
						class="band"
					/>
				{/if}
			{/each}

			{#each shown as point (point.k)}
				<rect
					x={xScale(point.k) - barWidth / 2}
					y={yScale(point.p)}
					width={barWidth}
					height={Math.max(0, inner.height - yScale(point.p))}
					style="fill: {highlight(point.k) ? 'var(--accent-amber)' : color}"
					class="bar"
					class:hovered={hovered?.k === point.k}
				/>
			{/each}

			{#if line !== null}
				<line
					class="line"
					x1={xScale(line)}
					x2={xScale(line)}
					y1="0"
					y2={inner.height}
				/>
			{/if}

			<line class="axis" x1="0" x2={inner.width} y1={inner.height} y2={inner.height} />

			{#each xTicks as tick (tick)}
				<text class="tick" x={xScale(tick)} y={inner.height + 16} text-anchor="middle">
					{formatX(tick)}
				</text>
			{/each}
		</g>

		<rect
			class="overlay"
			x={margin.left}
			y={margin.top}
			width={inner.width}
			height={inner.height}
			onmousemove={onMove}
			onmouseleave={() => (hovered = null)}
			role="presentation"
		/>
	</svg>

	<div class="footer">
		<span class="legend">
			{#each bands as band (band.label)}
				<span class="key">
					<span class="swatch" style="background: {band.color}"></span>{band.label}
				</span>
			{:else}
				<span>{labelX}</span>
			{/each}
		</span>
		{#if hovered}
			<span class="readout">
				{formatX(hovered.k)} → <strong>{(hovered.p * 100).toFixed(2)}%</strong>
			</span>
		{/if}
	</div>
</div>

<style>
	.chart {
		width: 100%;
		margin-top: 0.75rem;
	}

	svg {
		display: block;
		overflow: visible;
	}

	.band {
		opacity: 0.1;
	}

	.bar {
		opacity: 0.85;
	}

	.bar.hovered {
		opacity: 1;
	}

	.line {
		stroke: var(--text-primary);
		stroke-width: 1.5;
		stroke-dasharray: 4 3;
	}

	.axis {
		stroke: var(--border);
	}

	.tick {
		fill: var(--text-muted);
		font-size: 10px;
		font-family: var(--font-mono);
	}

	.overlay {
		fill: transparent;
		cursor: crosshair;
	}

	.footer {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: 1rem;
		margin-top: 0.3rem;
		font-size: 0.72rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}

	.legend {
		display: flex;
		gap: 0.85rem;
		flex-wrap: wrap;
	}

	.key {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
	}

	.swatch {
		width: 10px;
		height: 8px;
		border-radius: 1px;
		opacity: 0.35;
	}

	.readout strong {
		color: var(--accent-cyan);
	}
</style>
