/**
 * DraftKings My Bets → Bettor Desktop JSON export.
 *
 * Run this in DevTools on https://sportsbook.draftkings.com/mybets while logged in.
 * It reads bet cards already rendered on the page, downloads a JSON file, and
 * copies the same payload to your clipboard.
 *
 * Usage:
 *   1. Open My Bets, set filters (All / Settled), scroll to load every card.
 *   2. F12 → Console → paste this whole file → Enter.
 *   3. In Bettor Desktop → Bet Log → Import JSON → pick the downloaded file.
 *
 * Closing lines are not on this page, so imported bets show profit but not CLV
 * until you add both closing prices later.
 */
(() => {
	const OUTCOME = {
		won: 'won',
		win: 'won',
		lost: 'lost',
		loss: 'lost',
		push: 'push',
		void: 'void',
		cash: 'void',
		'cash out': 'void',
		open: 'pending',
		live: 'pending',
		pending: 'pending'
	};

	function parseAmerican(text) {
		const match = text.match(/([+-]\d{3,4})\b/);
		return match ? Number.parseInt(match[1], 10) : null;
	}

	function parseMoney(text) {
		const match = text.match(/\$([\d,]+(?:\.\d{2})?)/);
		return match ? Number.parseFloat(match[1].replace(/,/g, '')) : null;
	}

	function parsePlacedAt(text) {
		const match = text.match(
			/([A-Za-z]{3}\s+\d{1,2},\s+\d{4})(?:,\s+(\d{1,2}:\d{2}:\d{2}\s*[AP]M))?/i
		);
		if (!match) return new Date().toISOString().slice(0, 10);
		const stamp = match[2] ? `${match[1]}, ${match[2]}` : match[1];
		const parsed = new Date(stamp);
		return Number.isNaN(parsed.getTime())
			? new Date().toISOString().slice(0, 10)
			: parsed.toISOString().slice(0, 10);
	}

	function parseOutcome(text) {
		const head = text.slice(0, 120).toLowerCase();
		for (const [needle, value] of Object.entries(OUTCOME)) {
			if (head.includes(needle)) return value;
		}
		return 'pending';
	}

	function inferMarket(selection, event) {
		const blob = `${selection} ${event}`.toLowerCase();
		if (/\bo\/u\b|over|under|total/.test(blob)) return 'total';
		if (/spread|[+-]\d+(?:\.\d+)?(?!\d)/.test(blob)) return 'spread';
		if (/moneyline|ml\b|to win/.test(blob)) return 'moneyline';
		if (/birdie|prop|yards|touchdown|points|assists|rebounds|strikeouts/.test(blob)) {
			return 'prop';
		}
		return 'other';
	}

	function inferSport(event) {
		const blob = event.toLowerCase();
		if (/round \d|birdie|golf|pga|lpga|masters|open championship/.test(blob)) return 'Golf';
		if (/nfl|touchdown|quarterback|rushing yards/.test(blob)) return 'NFL';
		if (/nba|rebounds|assists/.test(blob)) return 'NBA';
		if (/mlb|strikeouts|home run/.test(blob)) return 'MLB';
		if (/nhl|puck/.test(blob)) return 'NHL';
		if (/premier league|la liga|bundesliga|serie a|mls|soccer/.test(blob)) return 'Soccer';
		if (/college football|cfb/.test(blob)) return 'College Football';
		if (/college basketball|ncaa/.test(blob)) return 'College Basketball';
		if (/tennis|open \(/.test(blob)) return 'Tennis';
		return '';
	}

	function cardLooksLikeBet(text) {
		return /wager:/i.test(text) && /(won|lost|open|live|push|cash out)/i.test(text);
	}

	function extractFromCard(node) {
		const text = node.innerText.replace(/\s+/g, ' ').trim();
		if (!cardLooksLikeBet(text)) return null;

		const wager = text.match(/Wager:\s*\$[\d,.]+/i)?.[0] ?? text;
		const stake = parseMoney(wager);
		if (stake === null) return null;

		const lines = node.innerText
			.split('\n')
			.map((line) => line.trim())
			.filter(Boolean);

		const statusLine = lines.find((line) => /^(WON|LOST|OPEN|LIVE|PUSH|CASH OUT)/i.test(line)) ?? '';
		const oddsLine =
			lines.find((line) => /^(OVER|UNDER|[+-]?\d)/i.test(line) && /[+-]\d{3,4}/.test(line)) ??
			lines.find((line) => /[+-]\d{3,4}/.test(line)) ??
			'';
		const eventLine =
			lines.find(
				(line) =>
					line.length > 12 &&
					!/^(WON|LOST|OPEN|LIVE|PUSH|CASH OUT|Wager:|Paid:)/i.test(line) &&
					!/[+-]\d{3,4}/.test(line)
			) ?? 'DraftKings bet';

		const footer = lines.find((line) => /DK\d+/.test(line) || /\d{4}/.test(line)) ?? text;
		const betId = footer.match(/DK\d+/)?.[0] ?? '';
		const priceTaken = parseAmerican(oddsLine) ?? parseAmerican(text);
		if (priceTaken === null) return null;

		const selection = oddsLine || eventLine;
		const sport = inferSport(eventLine);
		const market = inferMarket(selection, eventLine);
		const paid = parseMoney(text.match(/Paid:\s*\$[\d,.]+/i)?.[0] ?? '');

		return {
			placedAt: parsePlacedAt(footer),
			sport,
			market,
			selection,
			book: 'DraftKings',
			priceTaken,
			closingPrice: null,
			opposingClosingPrice: null,
			stake,
			outcome: parseOutcome(statusLine || text),
			notes: [
				eventLine !== selection ? eventLine : '',
				betId,
				paid !== null ? `paid ${paid.toFixed(2)}` : ''
			]
				.filter(Boolean)
				.join(' · ')
		};
	}

	function findBetCards() {
		const seen = new Set();
		const cards = [];

		for (const node of document.querySelectorAll('article, li, div')) {
			if (!(node instanceof HTMLElement)) continue;
			if (node.children.length === 0) continue;
			const text = node.innerText;
			if (!cardLooksLikeBet(text)) continue;
			if (text.length > 4000) continue;

			// Prefer the smallest wrapper that still looks like one bet card.
			const parentAlsoBet = [...node.parentElement?.querySelectorAll('div, article, li') ?? []]
				.filter((other) => other !== node && other.contains(node))
				.some((other) => cardLooksLikeBet(other.innerText) && other.innerText.length < text.length);
			if (parentAlsoBet) continue;

			const key = text.slice(0, 180);
			if (seen.has(key)) continue;
			seen.add(key);

			const draft = extractFromCard(node);
			if (draft) cards.push(draft);
		}

		return cards;
	}

	const drafts = findBetCards();
	if (drafts.length === 0) {
		console.warn(
			'No bet cards found. Scroll My Bets to load history, then run again.'
		);
		return;
	}

	const payload = JSON.stringify(drafts, null, 2);
	const blob = new Blob([payload], { type: 'application/json' });
	const url = URL.createObjectURL(blob);
	const anchor = document.createElement('a');
	const stamp = new Date().toISOString().slice(0, 10);
	anchor.href = url;
	anchor.download = `draftkings-my-bets-${stamp}.json`;
	anchor.click();
	URL.revokeObjectURL(url);

	navigator.clipboard?.writeText(payload).catch(() => {});

	console.log(`Exported ${drafts.length} bets → draftkings-my-bets-${stamp}.json`);
	console.log('Sample:', drafts[0]);
	return drafts;
})();
