import type { MixLeg } from './bindings';

/**
 * A one-shot handoff between two calculators.
 *
 * The bet log can derive a real bet mix, and the season simulator takes one.
 * Rather than couple the two components or push the mix through the URL — where
 * a dozen floats would be unreadable and truncated on the way — the sender
 * parks it here and the receiver consumes it once on mount.
 *
 * Deliberately not reactive and deliberately not persisted. It is a baton, not
 * a store: if the receiver never runs, the value is simply dropped when the
 * window closes, which is the right outcome for a handoff nobody caught.
 */
let pending: MixLeg[] | null = null;

/** Park a mix for whichever page picks it up next. */
export function offerMix(legs: MixLeg[]): void {
	pending = legs;
}

/** Take the parked mix, if there is one. Clears it. */
export function takeMix(): MixLeg[] | null {
	const legs = pending;
	pending = null;
	return legs;
}
