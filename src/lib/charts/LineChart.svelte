<script lang="ts">
	import { scaleLinear, line as d3Line, curveMonotoneX } from 'd3';

	export interface Point {
		x: number;
		y: number;
	}

	export interface Marker {
		value: number;
		label: string;
		color: string;
		/** `y` draws a horizontal rule, `x` a vertical one. */
		axis: 'x' | 'y';
	}

	let {
		data,
		markers = [],
		height = 300,
		color = 'var(--accent-cyan)',
		fill = false,
		formatX = (v: number) => String(v),
		formatY = (v: number) => String(v),
		labelX = '',
		labelY = ''
	}: {
		data: Point[];
		markers?: Marker[];
		height?: number;
		color?: string;
		fill?: boolean;
		formatX?: (v: number) => string;
		formatY?: (v: number) => string;
		labelX?: string;
		labelY?: string;
	} = $props();

	let width = $state(640);
	let hovered = $state<Point | null>(null);

	const margin = { top: 16, right: 68, bottom: 34, left: 56 };

	const inner = $derived({
		width: Math.max(10, width - margin.left - margin.right),
		height: Math.max(10, height - margin.top - margin.bottom)
	});

	const domain = $derived.by(() => {
		if (data.length === 0) return { x: [0, 1] as [number, number], y: [0, 1] as [number, number] };

		const xs = data.map((d) => d.x);
		const ys = data.map((d) => d.y);
		for (const marker of markers) {
			if (marker.axis === 'y') ys.push(marker.value);
			else xs.push(marker.value);
		}

		const yMin = Math.min(...ys);
		const yMax = Math.max(...ys);
		const pad = (yMax - yMin) * 0.1 || Math.abs(yMax) * 0.1 || 0.01;

		return {
			x: [Math.min(...xs), Math.max(...xs)] as [number, number],
			y: [yMin - pad, yMax + pad] as [number, number]
		};
	});

	const xScale = $derived(scaleLinear().domain(domain.x).range([0, inner.width]));
	const yScale = $derived(scaleLinear().domain(domain.y).range([inner.height, 0]));

	const xTicks = $derived(xScale.ticks(6));
	const yTicks = $derived(yScale.ticks(5));

	const path = $derived(
		d3Line<Point>()
			.x((d) => xScale(d.x))
			.y((d) => yScale(d.y))
			.curve(curveMonotoneX)(data) ?? ''
	);

	const areaPath = $derived(
		fill && data.length > 1
			? `${path}L${xScale(data[data.length - 1].x)},${yScale(domain.y[0])}L${xScale(data[0].x)},${yScale(domain.y[0])}Z`
			: ''
	);

	function onMove(event: MouseEvent) {
		if (data.length === 0) return;
		const bounds = (event.currentTarget as SVGRectElement).getBoundingClientRect();
		const value = xScale.invert(event.clientX - bounds.left);
		let closest = data[0];
		for (const point of data) {
			if (Math.abs(point.x - value) < Math.abs(closest.x - value)) closest = point;
		}
		hovered = closest;
	}
</script>

<div class="chart" bind:clientWidth={width}>
	<svg {width} {height} role="img" aria-label="{labelY} against {labelX}">
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

			{#each markers as marker (marker.label)}
				{#if marker.axis === 'y'}
					<line
						class="marker"
						x1="0"
						x2={inner.width}
						y1={yScale(marker.value)}
						y2={yScale(marker.value)}
						style="stroke: {marker.color}"
					/>
					<text
						class="marker-label"
						x={inner.width + 6}
						y={yScale(marker.value)}
						dy="0.32em"
						style="fill: {marker.color}"
					>
						{marker.label}
					</text>
				{:else}
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
				{/if}
			{/each}

			{#if areaPath}
				<path d={areaPath} style="fill: {color}" class="area" />
			{/if}
			<path d={path} style="stroke: {color}" class="series" />

			{#if hovered}
				<circle
					cx={xScale(hovered.x)}
					cy={yScale(hovered.y)}
					r="4"
					style="fill: {color}"
					class="dot"
				/>
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
		<span class="axis-label">{labelX}</span>
		{#if hovered}
			<span class="readout">
				{formatX(hovered.x)} → <strong>{formatY(hovered.y)}</strong>
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

	.marker {
		stroke-width: 1.5;
		stroke-dasharray: 6 3;
	}

	.marker-label {
		font-size: 10px;
		font-family: var(--font-mono);
	}

	.series {
		fill: none;
		stroke-width: 2;
	}

	.area {
		opacity: 0.12;
		stroke: none;
	}

	.dot {
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
		margin-top: 0.35rem;
		font-size: 0.72rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}

	.readout strong {
		color: var(--accent-cyan);
	}
</style>
