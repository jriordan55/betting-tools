import { call } from "./engine.js";

const ADDITIONS_KEY = "bettor-additions-v1";

const money = (value) => `${value > 0 ? "+" : ""}$${value.toFixed(2)}`;
const pct = (value) => `${(value * 100).toFixed(2)}%`;
const pctSigned = (value) => `${value > 0 ? "+" : ""}${(value * 100).toFixed(2)}%`;
const american = (value) => {
	const rounded = Math.round(value);
	return rounded > 0 ? `+${rounded}` : String(rounded);
};
const signClass = (value) => (value > 0 ? "pos" : value < 0 ? "neg" : "");

let book = null;

function loadAdditions() {
	try {
		const raw = localStorage.getItem(ADDITIONS_KEY);
		const parsed = raw ? JSON.parse(raw) : [];
		return Array.isArray(parsed) ? parsed : [];
	} catch {
		return [];
	}
}

function saveAdditions(bets) {
	localStorage.setItem(ADDITIONS_KEY, JSON.stringify(bets));
}

function loggedBets(current) {
	return current.bets.map((bet) => ({
		priceTaken: bet.priceTaken,
		closingPrice: bet.closingPrice ?? null,
		opposingClosingPrice: bet.opposingClosingPrice ?? null,
		stake: bet.stake,
		outcome: bet.outcome,
		realizedProfit: bet.profit ?? null,
	}));
}

async function refreshSnapshot(current) {
	current.snapshot = await call({ cmd: "snapshot", bets: loggedBets(current) });
}

function renderSummary() {
	const summaryEl = document.getElementById("summary");
	const caption = document.getElementById("caption");
	const status = document.getElementById("status");
	if (!book?.snapshot) return;

	const summary = book.snapshot.summary;
	status.hidden = true;
	summaryEl.hidden = false;
	caption.hidden = false;
	summaryEl.innerHTML = `
		<div class="metric"><span>Profit</span><strong class="${signClass(summary.profit)}">${money(summary.profit)}</strong></div>
		<div class="metric"><span>ROI</span><strong class="${signClass(summary.roi)}">${pctSigned(summary.roi)}</strong></div>
		<div class="metric"><span>Record</span><strong>${summary.won}–${summary.lost}–${summary.pushed}</strong></div>
		<div class="metric"><span>Bets</span><strong>${summary.bets}</strong></div>
	`;
	caption.textContent =
		`Settled ${summary.settled} · win rate ${pct(summary.winRate)} of decided bets · ` +
		`avg stake $${book.snapshot.avgStake.toFixed(0)} · avg price ${american(book.snapshot.avgAmerican)}`;
}

function renderLog() {
	const outcome = document.getElementById("filter-outcome").value;
	const root = document.getElementById("log");
	const rows = (book?.bets || [])
		.filter((bet) => !outcome || bet.outcome === outcome)
		.slice()
		.reverse()
		.slice(0, 200);

	root.innerHTML = rows
		.map(
			(bet) => `
		<article class="card">
			<div class="title">${escapeHtml(bet.selection || "Bet")}</div>
			<div class="meta">
				<span>${escapeHtml(bet.placedAt || "")}</span>
				<span>${escapeHtml(bet.book || "")}</span>
				<span>${american(bet.priceTaken)}</span>
				<span>$${Number(bet.stake).toFixed(0)}</span>
				<span class="${signClass(bet.profit ?? 0)}">${bet.outcome}${
				bet.profit == null ? "" : ` · ${money(bet.profit)}`
			}</span>
			</div>
		</article>`
		)
		.join("");
}

function escapeHtml(value) {
	return String(value)
		.replaceAll("&", "&amp;")
		.replaceAll("<", "&lt;")
		.replaceAll(">", "&gt;")
		.replaceAll('"', "&quot;");
}

function showPage(name) {
	document.querySelectorAll(".page").forEach((page) => page.classList.remove("active"));
	document.querySelectorAll("nav button").forEach((button) => button.classList.remove("active"));
	document.getElementById(`page-${name}`).classList.add("active");
	document.querySelector(`nav button[data-page="${name}"]`).classList.add("active");
	if (name === "log") renderLog();
}

async function boot() {
	const status = document.getElementById("status");
	try {
		const response = await fetch("./seed-book.json");
		if (!response.ok) throw new Error("could not load seed-book.json");
		book = await response.json();
		const additions = loadAdditions();
		if (additions.length) {
			book.bets.push(...additions);
			status.textContent = "Updating the book with bets you added on this phone…";
			await refreshSnapshot(book);
		}
		renderSummary();
		renderLog();
	} catch (error) {
		status.textContent = String(error.message || error);
	}
}

document.querySelectorAll("nav button").forEach((button) => {
	button.addEventListener("click", () => showPage(button.dataset.page));
});

document.getElementById("filter-outcome").addEventListener("change", renderLog);

document.querySelector('[name="placedAt"]').value = new Date().toISOString().slice(0, 10);

document.getElementById("add-form").addEventListener("submit", async (event) => {
	event.preventDefault();
	const form = event.currentTarget;
	const status = document.getElementById("add-status");
	const data = new FormData(form);
	status.textContent = "Adding…";

	try {
		const price = await call({
			cmd: "convertOdds",
			value: String(data.get("odds")).trim(),
			format: "american",
		});
		const header =
			"bet_id,sportsbook,type,status,odds,closing_line,amount,profit,time_placed_iso,bet_info,sports,leagues";
		const outcomeMap = {
			won: "SETTLED_WIN",
			lost: "SETTLED_LOSS",
			push: "SETTLED_PUSH",
			void: "SETTLED_VOID",
			pending: "PENDING",
		};
		const escapeCsv = (value) => `"${String(value).replaceAll('"', '""')}"`;
		const csv = [
			header,
			[
				"",
				escapeCsv(data.get("book") || "Unknown"),
				String(data.get("market")).toLowerCase() === "parlay" ? "parlay" : "single",
				outcomeMap[String(data.get("outcome"))] || "PENDING",
				price.decimal,
				"",
				data.get("stake"),
				"",
				`${data.get("placedAt")}T12:00:00Z`,
				escapeCsv(data.get("selection")),
				escapeCsv(data.get("sport") || ""),
				"",
			].join(","),
			"",
		].join("\n");

		const parsed = await call({ cmd: "importCsv", csv });
		if (!parsed.bets?.length) throw new Error(parsed.skipped?.[0]?.message || "bet was skipped");
		const bet = parsed.bets[0];
		bet.market = String(data.get("market") || bet.market || "other");

		book.bets.push(bet);
		const additions = loadAdditions();
		additions.push(bet);
		saveAdditions(additions);
		await refreshSnapshot(book);
		renderSummary();
		renderLog();
		form.reset();
		document.querySelector('[name="placedAt"]').value = new Date().toISOString().slice(0, 10);
		document.querySelector('[name="sport"]').value = "NBA";
		document.querySelector('[name="market"]').value = "moneyline";
		document.querySelector('[name="book"]').value = "DraftKings";
		document.querySelector('[name="odds"]').value = "-110";
		document.querySelector('[name="stake"]').value = "25";
		status.textContent = `Added ${bet.selection} at ${american(bet.priceTaken)}.`;
		showPage("book");
	} catch (error) {
		status.textContent = String(error.message || error);
	}
});

boot();
