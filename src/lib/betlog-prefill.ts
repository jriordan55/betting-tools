import type { BetLogSnapshot, MixLeg } from './bindings';

export interface MixBucket {
	price: string;
	stake: string;
	edge: string;
	count: string;
}

export function mixLegsToBuckets(legs: MixLeg[]): MixBucket[] {
	return legs.map((leg) => ({
		price: String(Math.round(leg.american)),
		stake: leg.stake.toFixed(0),
		edge: (leg.edge * 100).toFixed(2),
		count: String(leg.count)
	}));
}

export function snapshotMixLegs(snapshot: BetLogSnapshot): MixLeg[] {
	if (snapshot.clvMix && snapshot.clvMix.legs.length > 0) {
		return snapshot.clvMix.legs;
	}
	return snapshot.bookMix.legs;
}

export function mixSourceLabel(snapshot: BetLogSnapshot): string {
	if (snapshot.clvMix && snapshot.clvMix.legs.length > 0) {
		return 'closing-line edge';
	}
	return 'realised return';
}
