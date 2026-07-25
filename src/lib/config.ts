import { commands, type SportConfig } from './bindings';

/**
 * Sport presets, fetched once from the core.
 *
 * The tables live in `bettor-core::config` — the frontend reads them rather
 * than keeping a second copy, because a σ that disagrees between the two
 * silently prices every line in this app differently from the tests.
 */
let cached: Promise<SportConfig> | null = null;

export function sportConfig(): Promise<SportConfig> {
	cached ??= commands.sportConfig();
	return cached;
}
