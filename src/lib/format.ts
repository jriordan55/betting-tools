/**
 * Display formatting only.
 *
 * The core computes on 0–1 fractions and leaves presentation alone (CLAUDE.md:
 * "Formatting is the frontend's job"). Rendering a fraction as a percent is
 * formatting. Deriving a new quantity is not — that belongs in `bettor-core`.
 */

/** A 0–1 fraction as a percentage string. */
export function pct(fraction: number, digits = 2): string {
	if (!Number.isFinite(fraction)) return '—';
	return `${(fraction * 100).toFixed(digits)}%`;
}

/** A 0–1 fraction as a signed percentage — for EV, edge, ROI. */
export function pctSigned(fraction: number, digits = 2): string {
	if (!Number.isFinite(fraction)) return '—';
	const sign = fraction > 0 ? '+' : '';
	return `${sign}${(fraction * 100).toFixed(digits)}%`;
}

/** Probability points, the unit CLV should actually be quoted in. */
export function points(fraction: number, digits = 2): string {
	if (!Number.isFinite(fraction)) return '—';
	const sign = fraction > 0 ? '+' : '';
	return `${sign}${(fraction * 100).toFixed(digits)} pts`;
}

/** Currency, unsigned. */
export function money(amount: number, digits = 2): string {
	if (!Number.isFinite(amount)) return '—';
	return `$${amount.toFixed(digits)}`;
}

/** Currency with an explicit sign — for profit and loss. */
export function moneySigned(amount: number, digits = 2): string {
	if (!Number.isFinite(amount)) return '—';
	const sign = amount > 0 ? '+' : amount < 0 ? '-' : '';
	return `${sign}$${Math.abs(amount).toFixed(digits)}`;
}

/** American odds with the leading sign books print. */
export function american(value: number): string {
	if (!Number.isFinite(value)) return '—';
	const rounded = Math.round(value);
	return rounded > 0 ? `+${rounded}` : String(rounded);
}

/** A plain number at fixed precision. */
export function num(value: number, digits = 2): string {
	if (!Number.isFinite(value)) return '—';
	return value.toFixed(digits);
}

/** A line as a book posts it: `-3.5`, `+7`, `48.5`. */
export function line(value: number, digits = 1): string {
	if (!Number.isFinite(value)) return '—';
	const sign = value > 0 ? '+' : '';
	return `${sign}${value.toFixed(digits)}`;
}

/** A count with thousands separators. */
export function count(value: number): string {
	if (!Number.isFinite(value)) return '—';
	return value.toLocaleString('en-US');
}

/** Colour a value by its sign, for `ResultRow`. */
export function signColor(value: number): 'positive' | 'negative' | 'default' {
	if (value > 0) return 'positive';
	if (value < 0) return 'negative';
	return 'default';
}
