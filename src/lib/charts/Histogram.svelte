<script lang="ts">
	import { scaleLinear } from 'd3';
	import type { HistogramBin } from '$lib/bindings';

	export interface BandMarker {
		value: number;
		label: string;
		color: string;
	}

	let {
		bins,
		markers = [],
		height = 280,
		formatX = (v: number) => String(v),
		formatY = (v: number) => String(v),
		labelX = '',
		colorFor = () => 'var(--accent-cyan)'
	}: {
		bins: HistogramBin[];
		markers?: BandMarker[];
		height?: number;
		formatX?: (v: number) => string;
		formatY?: (v: number) => string;
		labelX?: string;
		colorFor?: (bin: HistogramBin) => string;
	} = $props();

	let width = $state(640);
	let hovered = $state<HistogramBin | null>(null);

	const margin = { top: 16, right: 16, bottom: 34, left: 52 };

	const inner = $derived({
		width: Math.max(10, width - margin.left - margin.right),
		height: Math.max(10, height - margin.top - margin.bottom)
	});

	const domain = $derived.by(() => {
		if (bins.length === 0) return { x: [0, 1] as [number, number], yMax: 1 };
		return {
			x: [bins[0].rangeStart, bins[bins.length - 1].rangeEnd] as [number, number],
			yMax: Math.max(...bins.map((b) => b.freq))
		};
	});

	const xScale = $derived(scaleLinear().domain(domain.x).range([0, inner.width]));
	const yScale = $derived(scaleLinear().domain([0, domain.yMax]).range([inner.height, 0]));

	const xTicks = $derived(xScale.ticks(7));
	const yTicks = $derived(yScale.ticks(4));
</script>

<div class="chart" bind:clientWidth={width}>
	<svg {width} {height} role="img" aria-label="Distribution of simulated outcomes">
		<g transform="translate({margin.left},{margin.top})">
			{#each yTicks as tick (tick)}
				<line class="grid" x1="0" x2={inner.width} y1={yScale(tick)} y2={yScale(tick)} />
				<text class="tick" x="-8" y={yScale(tick)} dy="0.32em" text-anchor="end">
					{formatY(tick)}
				</text>
			{/each}

			{#each bins as bin (bin.rangeStart)}
				{@const x = xScale(bin.rangeStart)}
				{@const w = Math.max(1, xScale(bin.rangeEnd) - x - 1)}
				<rect
					{x}
					y={yScale(bin.freq)}
					width={w}
					height={Math.max(0, inner.height - yScale(bin.freq))}
					style="fill: {colorFor(bin)}"
					class="bar"
					class:active={hovered === bin}
					role="presentation"
					onmouseenter={() => (hovered = bin)}
					onmouseleave={() => (hovered = null)}
				/>
			{/each}

			{#each markers as marker (marker.label)}
				<line
					class="marker"
					x1={xScale(marker.value)}
					x2={xScale(marker.value)}
					y1="0"
					y2={inner.height}
					style="stroke: {marker.color}"
				/>
				<text
					class="marker-label"
					x={xScale(marker.value)}
					y="-4"
					text-anchor="middle"
					style="fill: {marker.color}"
				>
					{marker.label}
				</text>
			{/each}

			{#each xTicks as tick (tick)}
				<text class="tick" x={xScale(tick)} y={inner.height + 18} text-anchor="middle">
					{formatX(tick)}
				</text>
			{/each}

			<line class="axis" x1="0" x2={inner.width} y1={inner.height} y2={inner.height} />
		</g>
	</svg>

	<div class="footer">
		<span>{labelX}</span>
		{#if hovered}
			<span class="readout">
				{formatX(hovered.rangeStart)}–{formatX(hovered.rangeEnd)} →
				<strong>{formatY(hovered.freq)}</strong>
			</span>
		{/if}
	</div>
</div>

<style>
	.chart {
		width: 100%;
		margin-top: 1rem;
	}

	svg {
		display: block;
		overflow: visible;
	}

	.grid {
		stroke: var(--border);
		stroke-dasharray: 3 3;
		opacity: 0.55;
	}

	.axis {
		stroke: var(--border);
	}

	.tick {
		fill: var(--text-muted);
		font-size: 10px;
		font-family: var(--font-mono);
	}

	.bar {
		opacity: 0.75;
		transition: opacity 0.1s ease;
	}

	.bar.active {
		opacity: 1;
	}

	.marker {
		stroke-width: 1.5;
		stroke-dasharray: 6 3;
	}

	.marker-label {
		font-size: 10px;
		font-family: var(--font-mono);
	}

	.footer {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		margin-top: 0.35rem;
		font-size: 0.72rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}

	.readout strong {
		color: var(--accent-cyan);
	}
</style>
