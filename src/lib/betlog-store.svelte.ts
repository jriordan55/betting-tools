import { commands, type BetFilter, type BetLogSnapshot } from './bindings';
import { snapshotMixLegs } from './betlog-prefill';

const EMPTY_FILTER: BetFilter = {
	outcome: null,
	sport: null,
	book: null,
	market: null,
	year: null,
	minStake: null,
	maxStake: null,
	kind: null,
	fromDate: null,
	toDate: null
};

/**
 * Cached views of the bet log for pre-filling calculators and the home page.
 *
 * Refreshed on app load and after the log changes. Deliberately holds the
 * whole book — calculator pre-fills describe the record, not the bet-log
 * table's current filter.
 */
class BetLogStore {
	snapshot = $state<BetLogSnapshot | null>(null);
	loading = $state(false);

	async refresh(filter: BetFilter = EMPTY_FILTER) {
		this.loading = true;
		try {
			const response = await commands.betLogSnapshot(filter);
			if (response.status === 'ok') {
				this.snapshot = response.data;
			}
		} finally {
			this.loading = false;
		}
	}

	async ensureLoaded(filter: BetFilter = EMPTY_FILTER) {
		if (!this.snapshot) await this.refresh(filter);
	}

	hasBets(): boolean {
		return (this.snapshot?.summary.bets ?? 0) > 0;
	}

	mixLegs() {
		return this.snapshot ? snapshotMixLegs(this.snapshot) : [];
	}
}

export const betLog = new BetLogStore();
