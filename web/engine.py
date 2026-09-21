"""Run the Rust math engine from the WebAssembly build of `bettor-cli`."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

WASM_PATH = Path(__file__).resolve().parent / "bettor.wasm"

_ENGINE = None
_MODULE = None


class EngineError(RuntimeError):
    pass


def call(payload: dict) -> dict:
    """Send one JSON command to bettor-core and return the `ok` payload."""
    raw = json.dumps(payload)
    stdout = _invoke_wasm(raw)
    try:
        parsed = json.loads(stdout)
    except json.JSONDecodeError as error:
        raise EngineError(f"engine returned something that is not JSON: {stdout[:400]}") from error
    if "error" in parsed:
        raise EngineError(str(parsed["error"]))
    if "ok" not in parsed:
        raise EngineError("engine response had neither ok nor error")
    return parsed["ok"]


def _invoke_wasm(raw: str) -> str:
    try:
        import wasmtime
    except ImportError as error:
        raise EngineError(
            "wasmtime is not installed. From the repo root: pip install -r requirements.txt"
        ) from error

    if not WASM_PATH.is_file():
        raise EngineError(
            f"missing {WASM_PATH.name}. Build it with: cargo build -p bettor-cli --release "
            "--target wasm32-wasip1 --no-default-features"
        )

    engine, module = _load(wasmtime)
    with tempfile.TemporaryDirectory() as tmp:
        folder = Path(tmp)
        stdin_path = folder / "stdin.txt"
        stdout_path = folder / "stdout.txt"
        stderr_path = folder / "stderr.txt"
        stdin_path.write_text(raw, encoding="utf-8")
        stdout_path.write_bytes(b"")
        stderr_path.write_bytes(b"")

        linker = wasmtime.Linker(engine)
        linker.define_wasi()
        store = wasmtime.Store(engine)
        wasi = wasmtime.WasiConfig()
        wasi.argv = ["bettor-cli"]
        wasi.stdin_file = str(stdin_path)
        wasi.stdout_file = str(stdout_path)
        wasi.stderr_file = str(stderr_path)
        store.set_wasi(wasi)
        instance = linker.instantiate(store, module)
        start = instance.exports(store)["_start"]
        try:
            start(store)
        except Exception as error:
            detail = stderr_path.read_text(encoding="utf-8", errors="replace").strip()
            text = f"{detail} {error}".strip()
            if "exit status 0" not in text and "Exited with i32(0)" not in text:
                raise EngineError(text or str(error)) from error
        return stdout_path.read_text(encoding="utf-8")


def _load(wasmtime):
    global _ENGINE, _MODULE
    if _ENGINE is None or _MODULE is None:
        _ENGINE = wasmtime.Engine()
        _MODULE = wasmtime.Module.from_file(_ENGINE, str(WASM_PATH))
    return _ENGINE, _MODULE
