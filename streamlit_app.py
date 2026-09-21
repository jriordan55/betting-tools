"""Bettor Desktop in the browser.

Run locally with `streamlit run streamlit_app.py`.
Streamlit Community Cloud uses this file as the entry point.
Numbers come from bettor-core via web/bettor.wasm — this file only formats them.
"""

from __future__ import annotations

import sys
from pathlib import Path

import streamlit as st

ROOT = Path(__file__).resolve().parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from web.engine import EngineError, call

st.set_page_config(page_title="Bettor Desktop", page_icon="B", layout="wide")


def money(value: float) -> str:
    sign = "+" if value > 0 else ""
    return f"{sign}${value:,.2f}"


def pct(value: float) -> str:
    return f"{value * 100:.2f}%"


def pct_signed(value: float) -> str:
    sign = "+" if value > 0 else ""
    return f"{sign}{value * 100:.2f}%"


def american(value: float) -> str:
    rounded = int(round(value))
    return f"+{rounded}" if rounded > 0 else str(rounded)


def require_book() -> dict | None:
    book = st.session_state.get("book")
    if not book:
        st.info("Upload a transactions CSV on **Your book** first.")
        return None
    return book


def logged_bets(book: dict) -> list[dict]:
    rows = []
    for bet in book["bets"]:
        rows.append(
            {
                "priceTaken": bet["priceTaken"],
                "closingPrice": bet.get("closingPrice"),
                "opposingClosingPrice": bet.get("opposingClosingPrice"),
                "stake": bet["stake"],
                "outcome": bet["outcome"],
            }
        )
    return rows


def page_book() -> None:
    st.header("Your book")
    st.write(
        "Upload the same `transactions.csv` Pikkit export the desktop app imports. "
        "Nothing is stored on the server after you close the tab."
    )
    upload = st.file_uploader("transactions.csv", type=["csv"])
    if upload is not None and st.button("Load bets", type="primary"):
        text = upload.getvalue().decode("utf-8-sig", errors="replace")
        with st.spinner("Reading the file in the Rust engine…"):
            try:
                result = call({"cmd": "importCsv", "csv": text})
            except EngineError as error:
                st.error(str(error))
                return
        st.session_state["book"] = result
        skipped = len(result.get("skipped") or [])
        loaded = len(result.get("bets") or [])
        st.success(f"Loaded {loaded} bets. Skipped {skipped}.")

    book = st.session_state.get("book")
    if not book:
        return

    summary = book["snapshot"]["summary"]
    c1, c2, c3, c4 = st.columns(4)
    c1.metric("Profit", money(summary["profit"]))
    c2.metric("ROI", pct_signed(summary["roi"]))
    c3.metric("Record", f"{summary['won']}–{summary['lost']}")
    c4.metric("Bets", f"{summary['bets']}")
    st.caption(
        f"Settled {summary['settled']} · win rate {pct(summary['winRate'])} of decided bets · "
        f"average stake ${book['snapshot']['avgStake']:,.0f} · "
        f"average price {american(book['snapshot']['avgAmerican'])}"
    )
    if book.get("skipped"):
        with st.expander(f"Skipped rows ({len(book['skipped'])})"):
            st.dataframe(book["skipped"], hide_index=True, width="stretch")


def page_log() -> None:
    st.header("Bet log")
    book = require_book()
    if not book:
        return
    bets = book["bets"]
    sports = sorted({b["sport"] for b in bets if b.get("sport")})
    books = sorted({b["book"] for b in bets if b.get("book")})
    outcomes = sorted({b["outcome"] for b in bets})
    c1, c2, c3 = st.columns(3)
    sport = c1.selectbox("Sport", ["All", *sports])
    book_name = c2.selectbox("Book", ["All", *books])
    outcome = c3.selectbox("Result", ["All", *outcomes])
    rows = [
        bet
        for bet in bets
        if (sport == "All" or bet.get("sport") == sport)
        and (book_name == "All" or bet.get("book") == book_name)
        and (outcome == "All" or bet.get("outcome") == outcome)
    ]
    st.caption(f"{len(rows)} of {len(bets)} bets")
    st.dataframe(rows, hide_index=True, width="stretch")


def page_mix() -> None:
    st.header("Bet mix")
    book = require_book()
    if not book:
        return
    legs = book["snapshot"]["bookMix"]["legs"]
    if not legs:
        st.warning("No settled bets to bucket.")
        return
    st.caption("Price buckets use realised return, because this export has almost no closing lines.")
    st.dataframe(
        [
            {
                "price": american(leg["american"]),
                "stake": round(leg["stake"], 2),
                "edge": pct_signed(leg["edge"]),
                "bets": leg["count"],
            }
            for leg in legs
        ],
        hide_index=True,
        width="stretch",
    )
    if st.button("Price the mix", type="primary"):
        with st.spinner("Pricing…"):
            try:
                mix = call({"cmd": "betMix", "legs": legs})
            except EngineError as error:
                st.error(str(error))
                return
        st.session_state["mix"] = mix
    mix = st.session_state.get("mix")
    if not mix:
        return
    a, b, c = st.columns(3)
    a.metric("Expected profit", money(mix["ev"]))
    b.metric("ROI", pct_signed(mix["roi"]))
    c.metric("Bets", f"{mix['bets']}")
    st.dataframe(mix["legs"], hide_index=True, width="stretch")


def page_season() -> None:
    st.header("Season simulator")
    book = require_book()
    if not book:
        return
    legs = book["snapshot"]["bookMix"]["legs"]
    if not legs:
        st.warning("No settled bets to simulate.")
        return
    bankroll = st.number_input("Starting bankroll", min_value=1.0, value=10000.0, step=100.0)
    sims = st.number_input("Seasons", min_value=1, max_value=5000, value=500, step=100)
    seed = st.text_input("Seed (blank = new run)", value="")
    if st.button("Simulate", type="primary"):
        with st.spinner("Simulating… this stays in the Rust engine, so a long book takes a moment."):
            try:
                result = call(
                    {
                        "cmd": "simulateSeason",
                        "seed": seed,
                        "input": {
                            "legs": legs,
                            "bankroll": bankroll,
                            "numSims": int(sims),
                            "stopAtRuin": False,
                        },
                    }
                )
            except EngineError as error:
                st.error(str(error))
                return
        st.session_state["season"] = result
    result = st.session_state.get("season")
    if not result:
        return
    a, b, c, d = st.columns(4)
    a.metric("Median ending", money(result["endingMedian"]))
    b.metric("Mean ending", money(result["endingMean"]))
    c.metric("Losing season", pct(result["losingSeasonProb"]))
    d.metric("Ruin", pct(result["ruinProb"]))
    st.caption(f"Seed {result['seed']} · {result['bets']} bets per season")
    if result.get("fan"):
        import pandas as pd

        frame = pd.DataFrame(result["fan"])
        st.line_chart(frame, x="bet", y=["p05", "p25", "median", "p75", "p95"])


def page_ruin() -> None:
    st.header("Risk of ruin")
    book = st.session_state.get("book")
    snap = book["snapshot"] if book else None
    win = st.number_input(
        "Win rate (%)",
        min_value=0.1,
        max_value=99.9,
        value=(snap["summary"]["winRate"] * 100) if snap else 55.0,
        step=0.1,
    )
    odds = st.text_input("Average American price", value=american(snap["avgAmerican"]) if snap else "-110")
    bankroll = st.number_input("Bankroll", min_value=1.0, value=max((snap or {}).get("avgStake", 20) * 50, 1000) if snap else 1000.0)
    stake = st.number_input("Stake", min_value=0.01, value=float(snap["avgStake"]) if snap else 20.0)
    bets = st.number_input("Bets", min_value=1, value=int(snap["summary"]["settled"]) if snap else 500)
    sims = st.number_input("Simulations", min_value=1, max_value=20000, value=2000)
    seed = st.text_input("Seed", value="", key="ruin-seed")
    if st.button("Run", type="primary"):
        try:
            price = call({"cmd": "convertOdds", "value": odds, "format": "american"})
        except EngineError as error:
            st.error(str(error))
            return
        with st.spinner("Simulating ruin…"):
            try:
                result = call(
                    {
                        "cmd": "simulateRuin",
                        "seed": seed,
                        "input": {
                            "winProb": win / 100,
                            "decimalOdds": price["decimal"],
                            "betSize": stake,
                            "bankroll": bankroll,
                            "numBets": int(bets),
                            "numSims": int(sims),
                        },
                    }
                )
            except EngineError as error:
                st.error(str(error))
                return
        st.session_state["ruin"] = result
    result = st.session_state.get("ruin")
    if not result:
        return
    a, b, c = st.columns(3)
    a.metric("Ruin probability", pct(result["ruinProb"]))
    b.metric("Edge", pct_signed(result["edge"]))
    c.metric("Median ending (all paths)", money(result["medianEndingAll"]))
    st.caption(f"Seed {result['seed']} · longest losing streak {result['longestLosingStreak']}")
    if result.get("survivalCurve"):
        import pandas as pd

        st.line_chart(pd.DataFrame(result["survivalCurve"]), x="bet", y="survival")


def page_ladder() -> None:
    st.header("Breakeven ladder")
    book = st.session_state.get("book")
    snap = book["snapshot"] if book else None
    start = st.text_input("From", value=american(snap["priceMin"]) if snap else "-400")
    end = st.text_input("To", value=american(snap["priceMax"]) if snap else "+600")
    step = st.number_input("Step in cents", min_value=1.0, value=100.0)
    edge = st.number_input(
        "Target edge (%)",
        value=(snap["summary"]["roi"] * 100) if snap else 2.0,
        step=0.1,
    )
    if st.button("Build ladder", type="primary"):
        try:
            low = call({"cmd": "convertOdds", "value": start, "format": "american"})["american"]
            high = call({"cmd": "convertOdds", "value": end, "format": "american"})["american"]
            prices = call(
                {"cmd": "priceLadder", "from": low, "to": high, "stepCents": float(step)}
            )
            rungs = call(
                {"cmd": "breakevenLadder", "prices": prices, "targetEdge": edge / 100}
            )
        except EngineError as error:
            st.error(str(error))
            return
        st.dataframe(rungs, hide_index=True, width="stretch")


def page_kelly() -> None:
    st.header("Kelly criterion")
    book = st.session_state.get("book")
    snap = book["snapshot"] if book else None
    odds = st.text_input("Odds", value=american(snap["avgAmerican"]) if snap else "-110", key="kelly-odds")
    win = st.number_input(
        "True win probability (%)",
        min_value=0.1,
        max_value=99.9,
        value=(snap["summary"]["winRate"] * 100) if snap else 55.0,
        key="kelly-win",
    )
    bankroll = st.number_input("Bankroll", min_value=1.0, value=10000.0, key="kelly-bank")
    fraction = st.select_slider("Kelly fraction", options=[1.0, 0.75, 0.5, 0.25], value=0.5)
    if st.button("Size the bet", type="primary"):
        try:
            price = call({"cmd": "convertOdds", "value": odds, "format": "american"})
            kelly = call(
                {
                    "cmd": "kelly",
                    "decimal": price["decimal"],
                    "trueProb": win / 100,
                    "bankroll": bankroll,
                    "multiplier": fraction,
                }
            )
        except EngineError as error:
            st.error(str(error))
            return
        st.metric("Suggested stake", money(kelly["adjustedStake"]))
        st.write(
            f"Full Kelly {money(kelly['fullStake'])} ({pct(kelly['fullFraction'])} of bankroll). "
            f"Edge {pct_signed(kelly['evFraction'])}."
        )


def page_odds() -> None:
    st.header("Odds converter")
    value = st.text_input("Price", value="-110")
    fmt = st.selectbox("Format", ["american", "decimal", "fractional"])
    if value.strip():
        try:
            view = call({"cmd": "convertOdds", "value": value.strip(), "format": fmt})
        except EngineError as error:
            st.error(str(error))
            return
        c1, c2, c3, c4 = st.columns(4)
        c1.metric("American", american(view["american"]))
        c2.metric("Decimal", f"{view['decimal']:.4f}")
        c3.metric("Fractional", f"{view['fractionalNum']}/{view['fractionalDen']}")
        c4.metric("Implied", pct(view["probability"]))


def page_ev() -> None:
    st.header("Expected value")
    odds = st.text_input("Odds", value="-110", key="ev-odds")
    win = st.number_input("True win probability (%)", min_value=0.1, max_value=99.9, value=55.0, key="ev-win")
    stake = st.number_input("Stake", min_value=0.01, value=100.0, key="ev-stake")
    if odds.strip():
        try:
            price = call({"cmd": "convertOdds", "value": odds.strip(), "format": "american"})
            ev = call(
                {
                    "cmd": "expectedValue",
                    "decimal": price["decimal"],
                    "trueProb": win / 100,
                    "stake": stake,
                }
            )
        except EngineError as error:
            st.error(str(error))
            return
        st.metric("Expected value", money(ev["evAmount"]))
        st.caption(f"Edge {pct_signed(ev['evFraction'])} at {american(price['american'])}")


def page_parlay() -> None:
    st.header("Parlay")
    raw = st.text_area("Leg prices, one per line", value="-110\n-110\n+150")
    stake = st.number_input("Stake", min_value=0.01, value=10.0, key="parlay-stake")
    prices = [line.strip() for line in raw.splitlines() if line.strip()]
    if len(prices) < 2:
        st.info("Enter at least two prices.")
        return
    try:
        american_prices = [
            call({"cmd": "convertOdds", "value": price, "format": "american"})["american"] for price in prices
        ]
        parlay = call({"cmd": "parlay", "american": american_prices, "stake": stake})
        priced = call(
            {"cmd": "convertOdds", "value": str(parlay["decimal"]), "format": "decimal"}
        )
    except EngineError as error:
        st.error(str(error))
        return
    c1, c2, c3 = st.columns(3)
    c1.metric("Decimal", f"{parlay['decimal']:.3f}")
    c2.metric("American", american(priced["american"]))
    c3.metric("Payout", money(parlay["payout"]))


def page_hold() -> None:
    st.header("Hold")
    a = st.text_input("Side A", value="-110")
    b = st.text_input("Side B", value="-110")
    if a.strip() and b.strip():
        try:
            side_a = call({"cmd": "convertOdds", "value": a.strip(), "format": "american"})["american"]
            side_b = call({"cmd": "convertOdds", "value": b.strip(), "format": "american"})["american"]
            hold = call({"cmd": "hold", "americanA": side_a, "americanB": side_b})
        except EngineError as error:
            st.error(str(error))
            return
        st.metric("Hold", pct(hold["hold"]))
        st.caption(f"No-vig prices sit at {pct(hold['noVigProbA'])} / {pct(hold['noVigProbB'])}")


PAGES = {
    "Your book": page_book,
    "Bet log": page_log,
    "Bet mix": page_mix,
    "Season simulator": page_season,
    "Risk of ruin": page_ruin,
    "Breakeven ladder": page_ladder,
    "Kelly criterion": page_kelly,
    "Odds converter": page_odds,
    "Expected value": page_ev,
    "Parlay": page_parlay,
    "Hold": page_hold,
}

st.sidebar.title("Bettor Desktop")
st.sidebar.caption("Same Rust engine as the desktop app, running in the browser tab's server.")
choice = st.sidebar.radio("Tool", list(PAGES))
PAGES[choice]()
