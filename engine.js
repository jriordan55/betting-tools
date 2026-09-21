/**
 * Run bettor-cli (wasm32-wasip1) in the browser via a WASI shim.
 * Same Rust engine as the desktop app.
 */
import { WASI, File, OpenFile } from "https://cdn.jsdelivr.net/npm/@bjorn3/browser_wasi_shim@0.4.2/+esm";

let compiled = null;

async function loadModule() {
	if (!compiled) {
		compiled = await WebAssembly.compileStreaming(fetch("./bettor.wasm"));
	}
	return compiled;
}

export async function call(payload) {
	const stdin = new OpenFile(new File(new TextEncoder().encode(JSON.stringify(payload))));
	const stdout = new OpenFile(new File(new Uint8Array()));
	const stderr = new OpenFile(new File(new Uint8Array()));
	const wasi = new WASI(["bettor-cli"], [], [stdin, stdout, stderr], { debug: false });
	const module = await loadModule();
	const instance = await WebAssembly.instantiate(module, {
		wasi_snapshot_preview1: wasi.wasiImport,
	});

	try {
		wasi.start(instance);
	} catch (error) {
		const message = String(error?.message || error);
		if (!/exit.*0|Exited with i32\(0\)/i.test(message)) {
			const detail = new TextDecoder().decode(stderr.file.data);
			throw new Error(detail || message);
		}
	}

	const raw = new TextDecoder().decode(stdout.file.data).trim();
	const parsed = JSON.parse(raw);
	if (parsed.error) throw new Error(parsed.error);
	if (!("ok" in parsed)) throw new Error("engine response had neither ok nor error");
	return parsed.ok;
}
