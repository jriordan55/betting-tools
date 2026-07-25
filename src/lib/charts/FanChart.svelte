<script lang="ts">
	import { scaleLinear, area as d3Area, line as d3Line, curveMonotoneX } from 'd3';
	import type { FanPoint } from '$lib/bindings';

	let {
		data,
		start,
		height = 320,
		formatX = (v: number) => String(v),
		formatY = (v: number) => String(v),
		labelX = ''
	}: {
		data: FanPoint[];
		/** Starting bankroll. Drawn as the line between a winning year and a losing one. */
		start: number;
		height?: number;
		formatX?: (v: number) => string;
		formatY?: (v: number) => string;
		labelX?: string;
	} = $props();

	let width = $state(640);
	let hovered = $state<FanPoint | null>(null);

	const margin = { top: 16, right: 72, bottom: 34, left: 68 };

	const inner = $derived({
		width: Math.max(10, width - margin.left - margin.right),
		height: Math.max(10, height - margin.top - margin.bottom)
	});

	const domain = $derived.by(() => {
		if (data.length === 0) return { x: [0, 1] as [number, number], y: [0, 1] as [number, number] };
		const lo = Math.min(start, ...data.map((d) => d.p05));
		const hi = Math.max(start, ...data.map((d) => d.p95));
		const pad = (hi - lo) * 0.08 || Math.abs(hi) * 0.08 || 1;
		return {
			x: [0, Math.max(...data.map((d) => d.bet))] as [number, number],
			y: [lo - pad, hi + pad] as [number, number]
		};
	});

	const xScale = $derived(scaleLinear().domain(domain.x).range([0, inner.width]));
	const yScale = $derived(scaleLinear().domain(domain.y).range([inner.height, 0]));

	const xTicks = $derived(xScale.ticks(6));
	const yTicks = $derived(yScale.ticks(5));

	function band(lo: (d: FanPoint) => number, hi: (d: FanPoint) => number): string {
		return (
			d3Area<FanPoint>()
				.x((d) => xScale(d.bet))
				.y0((d) => yScale(lo(d)))
				.y1((d) => yScale(hi(d)))
				.curve(curveMonotoneX)(data) ?? ''
		);
	}

	const outer = $derived(band((d) => d.p05, (d) => d.p95));
	const middle = $derived(band((d) => d.p25, (d) => d.p75));

	const medianPath = $derived(
		d3Line<FanPoint>()
			.x((d) => xScale(d.bet))
			.y((d) => yScale(d.median))
			.curve(curveMonotoneX)(data) ?? ''
	);

	function onMove(event: MouseEvent) {
		if (data.length === 0) return;
		const bounds = (event.currentTarget as SVGRectElement).getBoundingClientRect();
		const value = xScale.invert(event.clientX - bounds.left);
		let closest = data[0];
		for (const point of data) {
			if (Math.abs(point.bet - value) < Math.abs(closest.bet - value)) closest = point;
		}
		hovered = closest;
	}
</script>

<div class="chart" bind:clientWidth={width}>
	<svg {width} {height} role="img" aria-label="Bankroll percentiles against bets placed">
		<g transform="translate({margin.left},{margin.top})">
			{#each yTicks as tick (tick)}
				<line class="grid" x1="0" x2={inner.width} y1={yScale(tick)} y2={yScale(tick)} />
				<text class="tick" x="-8" y={yScale(tick)} dy="0.32em" text-anchor="end">
					{formatY(tick)}
				</text>
			{/each}

			{#each xTicks as tick (tick)}
				<text class="tick" x={xScale(tick)} y={inner.height + 18} text-anchor="middle">
					{formatX(tick)}
				</text>
			{/each}

			<line class="axis" x1="0" x2={inner.width} y1={inner.height} y2={inner.height} />
			<line class="axis" x1="0" x2="0" y1="0" y2={inner.height} />

			<path d={outer} class="band outer" />
			<path d={middle} class="band middle" />
			<path d={medianPath} class="median" />

			<!-- Everything under this line is a losing season. -->
			<line
				class="breakeven"
				x1="0"
				x2={inner.width}
				y1={yScale(start)}
				y2={yScale(start)}
			/>
			<text class="breakeven-label" x={inner.width + 6} y={yScale(start)} dy="0.32em">
				break even
			</text>

			{#if hovered}
				<line
					class="cursor"
					x1={xScale(hovered.bet)}
					x2={xScale(hovered.bet)}
					y1={yScale(hovered.p95)}
					y2={yScale(hovered.p05)}
				/>
				<circle cx={xScale(hovered.bet)} cy={yScale(hovered.median)} r="4" class="dot" />
			{/if}

			<rect
				class="overlay"
				width={inner.width}
				height={inner.height}
				onmousemove={onMove}
				onmouseleave={() => (hovered = null)}
				role="presentation"
			/>
		</g>
	</svg>

	<div class="footer">
		<span class="legend">
			<span class="key"><span class="swatch outer"></span>5–95%</span>
			<span class="key"><span class="swatch middle"></span>25–75%</span>
			<span class="key"><span class="swatch line"></span>median</span>
		</span>
		{#if hovered}
			<span class="readout">
				{formatX(hovered.bet)}{labelX ? ` ${labelX}` : ''} →
				<strong>{formatY(hovered.p05)}</strong> …
				<strong>{formatY(hovered.median)}</strong> …
				<strong>{formatY(hovered.p95)}</strong>
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

	.band {
		stroke: none;
		fill: var(--accent-cyan);
	}

	.band.outer {
		opacity: 0.12;
	}

	.band.middle {
		opacity: 0.2;
	}

	.median {
		fill: none;
		stroke: var(--accent-cyan);
		stroke-width: 2;
	}

	.breakeven {
		stroke: var(--text-muted);
		stroke-width: 1.5;
		stroke-dasharray: 6 3;
	}

	.breakeven-label {
		fill: var(--text-muted);
		font-size: 10px;
		font-family: var(--font-mono);
	}

	.cursor {
		stroke: var(--border-strong);
		stroke-width: 1;
	}

	.dot {
		fill: var(--accent-cyan);
		stroke: var(--bg-secondary);
		stroke-width: 2;
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
		margin-top: 0.35rem;
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
		background: var(--accent-cyan);
	}

	.swatch.outer {
		opacity: 0.25;
	}

	.swatch.middle {
		opacity: 0.45;
	}

	.swatch.line {
		height: 2px;
	}

	.readout {
		white-space: nowrap;
	}

	.readout strong {
		color: var(--accent-cyan);
	}
</style>
