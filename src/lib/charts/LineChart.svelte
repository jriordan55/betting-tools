<script lang="ts">
	import { scaleLinear, line as d3Line, curveMonotoneX } from 'd3';

	export interface Point {
		x: number;
		y: number;
	}

	export interface Series {
		data: Point[];
		label: string;
		color?: string;
		fill?: boolean;
		dashed?: boolean;
	}

	export interface Marker {
		value: number;
		label: string;
		color: string;
		/** `y` draws a horizontal rule, `x` a vertical one. */
		axis: 'x' | 'y';
	}

	let {
		data = [],
		series,
		markers = [],
		height = 300,
		color = 'var(--accent-cyan)',
		fill = false,
		formatX = (v: number) => String(v),
		formatY = (v: number) => String(v),
		labelX = '',
		labelY = ''
	}: {
		/** Shorthand for a single series. Ignored when `series` is given. */
		data?: Point[];
		/** Two or more lines on shared axes. */
		series?: Series[];
		markers?: Marker[];
		height?: number;
		color?: string;
		fill?: boolean;
		formatX?: (v: number) => string;
		formatY?: (v: number) => string;
		labelX?: string;
		labelY?: string;
	} = $props();

	/** One code path for both shapes: the single-series props become a series. */
	const lines = $derived<Series[]>(
		series ?? [{ data, label: labelY, color, fill, dashed: false }]
	);

	let width = $state(640);
	/** The x the pointer is nearest, or null. Each series reads its own y from it. */
	let hoveredX = $state<number | null>(null);

	const margin = { top: 16, right: 68, bottom: 34, left: 56 };

	const inner = $derived({
		width: Math.max(10, width - margin.left - margin.right),
		height: Math.max(10, height - margin.top - margin.bottom)
	});

	const domain = $derived.by(() => {
		const xs: number[] = [];
		const ys: number[] = [];
		for (const s of lines) {
			for (const p of s.data) {
				xs.push(p.x);
				ys.push(p.y);
			}
		}
		if (xs.length === 0) return { x: [0, 1] as [number, number], y: [0, 1] as [number, number] };

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

	const shapes = $derived(
		lines.map((s) => {
			const path =
				d3Line<Point>()
					.x((d) => xScale(d.x))
					.y((d) => yScale(d.y))
					.curve(curveMonotoneX)(s.data) ?? '';

			const first = s.data[0];
			const last = s.data[s.data.length - 1];
			const area =
				s.fill && s.data.length > 1 && first && last
					? `${path}L${xScale(last.x)},${yScale(domain.y[0])}L${xScale(first.x)},${yScale(domain.y[0])}Z`
					: '';

			return { ...s, color: s.color ?? color, path, area };
		})
	);

	/** The point of each series nearest the hovered x. */
	const readout = $derived.by(() => {
		const at = hoveredX;
		if (at === null) return [];
		return shapes
			.map((s) => {
				let closest: Point | null = null;
				for (const point of s.data) {
					if (closest === null || Math.abs(point.x - at) < Math.abs(closest.x - at)) {
						closest = point;
					}
				}
				return closest ? { label: s.label, color: s.color, point: closest } : null;
			})
			.filter((entry) => entry !== null);
	});

	function onMove(event: MouseEvent) {
		const bounds = (event.currentTarget as SVGRectElement).getBoundingClientRect();
		hoveredX = xScale.invert(event.clientX - bounds.left);
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

			{#each shapes as s (s.label)}
				{#if s.area}
					<path d={s.area} style="fill: {s.color}" class="area" />
				{/if}
				<path
					d={s.path}
					style="stroke: {s.color}"
					class="series"
					class:dashed={s.dashed}
				/>
			{/each}

			{#each readout as entry (entry.label)}
				<circle
					cx={xScale(entry.point.x)}
					cy={yScale(entry.point.y)}
					r="4"
					style="fill: {entry.color}"
					class="dot"
				/>
			{/each}

			<rect
				class="overlay"
				width={inner.width}
				height={inner.height}
				onmousemove={onMove}
				onmouseleave={() => (hoveredX = null)}
				role="presentation"
			/>
		</g>
	</svg>

	<div class="footer">
		{#if lines.length > 1}
			<span class="legend">
				{#each shapes as s (s.label)}
					<span class="key">
						<span class="swatch" style="background: {s.color}"></span>{s.label}
					</span>
				{/each}
			</span>
		{:else}
			<span class="axis-label">{labelX}</span>
		{/if}

		{#if readout.length > 0}
			<span class="readout">
				{formatX(readout[0].point.x)} →
				{#each readout as entry (entry.label)}
					<strong style="color: {entry.color}">{formatY(entry.point.y)}</strong>
				{/each}
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

	.series.dashed {
		stroke-dasharray: 5 4;
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
		width: 9px;
		height: 2px;
		border-radius: 1px;
	}

	.readout {
		display: inline-flex;
		gap: 0.5rem;
		white-space: nowrap;
	}

	.readout strong {
		color: var(--accent-cyan);
	}
</style>
