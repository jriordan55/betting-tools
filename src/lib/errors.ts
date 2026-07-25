import type { MathError, Result } from './bindings';

/**
 * Render a `MathError` as a sentence a bettor can act on.
 *
 * The frontend matches on `kind` — it never parses a message string. Adding a
 * variant in Rust makes this a compile error, which is the point.
 */
export function describeError(error: MathError): string {
	switch (error.kind) {
		case 'parseOdds': {
			const { value, format } = error.detail;
			const shown = value.trim() === '' ? '(empty)' : value;
			return `"${shown}" is not valid ${format} odds.`;
		}
		case 'probabilityOutOfRange': {
			const { value, reason } = error.detail;
			return `Probability ${formatNumber(value, 4)} is out of range — ${reason}.`;
		}
		case 'domainError': {
			const { param, constraint, value } = error.detail;
			return `${param} must be ${constraint}; got ${formatNumber(value, 4)}.`;
		}
		case 'shapeError': {
			const { what, expected, got } = error.detail;
			return `Expected ${expected} ${what}; got ${got}.`;
		}
		case 'noConvergence': {
			const { solver, iterations, residual } = error.detail;
			return `The ${solver} solver did not converge in ${iterations} iterations (residual ${residual.toExponential(2)}). The market may be degenerate.`;
		}
	}
}

/** Narrow a command `Result`, returning `null` on error. */
export function ok<T>(result: Result<T, MathError>): T | null {
	return result.status === 'ok' ? result.data : null;
}

/** The error side of a command `Result`, or `null` when it succeeded. */
export function err<T>(result: Result<T, MathError>): MathError | null {
	return result.status === 'error' ? result.error : null;
}

function formatNumber(value: number, digits: number): string {
	return Number.isFinite(value) ? value.toFixed(digits).replace(/\.?0+$/, '') : String(value);
}
