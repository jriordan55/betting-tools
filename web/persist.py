"""Persist the uploaded book between Streamlit reloads.

The CSV and a JSON snapshot live under `data/`, which is gitignored so a
betting record never lands in the repository.
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA_DIR = ROOT / "data"
CSV_PATH = DATA_DIR / "transactions.csv"
BOOK_PATH = DATA_DIR / "book.json"


def ensure_data_dir() -> None:
    DATA_DIR.mkdir(parents=True, exist_ok=True)


def save_csv(text: str) -> None:
    ensure_data_dir()
    CSV_PATH.write_text(text, encoding="utf-8")


def load_csv() -> str | None:
    if not CSV_PATH.is_file():
        return None
    return CSV_PATH.read_text(encoding="utf-8")


def save_book(book: dict) -> None:
    ensure_data_dir()
    BOOK_PATH.write_text(json.dumps(book), encoding="utf-8")


def load_book() -> dict | None:
    if not BOOK_PATH.is_file():
        return None
    try:
        return json.loads(BOOK_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None


def clear_saved() -> None:
    for path in (CSV_PATH, BOOK_PATH):
        if path.is_file():
            path.unlink()


def has_saved_csv() -> bool:
    return CSV_PATH.is_file()
