/**
 * Every calculation is an IPC round trip, so every result is asynchronous.
 *
 * `Async` gives the `$derived` ergonomics back: declare the reactive inputs in
 * `deps`, do the awaiting in `run`. `deps` runs inside the effect so its reads
 * are tracked; `run` is free to await, because nothing it reads needs tracking.
 * Stale responses are discarded, so a fast keystroke can never be overwritten
 * by a slow earlier one.
 */
export class Async<D, T> {
	current = $state<T | null>(null);
	pending = $state(false);

	constructor(deps: () => D, run: (deps: D) => Promise<T | null>) {
		$effect(() => {
			const d = deps();
			let cancelled = false;
			this.pending = true;

			run(d)
				.then((value) => {
					if (cancelled) return;
					this.current = value;
					this.pending = false;
				})
				.catch(() => {
					if (cancelled) return;
					this.current = null;
					this.pending = false;
				});

			return () => {
				cancelled = true;
			};
		});
	}
}

/** Parse a required positive number from an input field. */
export function positive(value: string): number | null {
	const n = Number.parseFloat(value);
	return Number.isFinite(n) && n > 0 ? n : null;
}

/** Parse a number from an input field, allowing zero and negatives. */
export function numeric(value: string): number | null {
	const n = Number.parseFloat(value);
	return Number.isFinite(n) ? n : null;
}
