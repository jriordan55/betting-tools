"""Load the seeded book and keep one-at-a-time additions.

`seed/transactions.csv` ships with the app. New bets are appended to
`seed/additions.json` so they reload with the book.
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SEED_DIR = ROOT / "seed"
DATA_DIR = ROOT / "data"
SEED_CSV = SEED_DIR / "transactions.csv"
ADDITIONS_PATH = SEED_DIR / "additions.json"
BOOK_PATH = DATA_DIR / "book.json"


def ensure_dirs() -> None:
    SEED_DIR.mkdir(parents=True, exist_ok=True)
    DATA_DIR.mkdir(parents=True, exist_ok=True)


def load_seed_csv() -> str | None:
    if not SEED_CSV.is_file():
        return None
    return SEED_CSV.read_text(encoding="utf-8-sig")


def load_additions() -> list[dict]:
    if not ADDITIONS_PATH.is_file():
        return []
    try:
        data = json.loads(ADDITIONS_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return []
    return data if isinstance(data, list) else []


def save_additions(bets: list[dict]) -> None:
    ensure_dirs()
    ADDITIONS_PATH.write_text(json.dumps(bets, indent=2), encoding="utf-8")


def append_addition(bet: dict) -> None:
    bets = load_additions()
    bets.append(bet)
    save_additions(bets)


def save_book(book: dict) -> None:
    ensure_dirs()
    BOOK_PATH.write_text(json.dumps(book), encoding="utf-8")


def load_book() -> dict | None:
    if not BOOK_PATH.is_file():
        return None
    try:
        return json.loads(BOOK_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None
