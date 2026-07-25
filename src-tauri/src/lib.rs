//! Desktop shell for the bettor calculators.
//!
//! This crate owns windowing, IPC, and (from Phase 6) the SQLite bet log.
//! It deliberately owns no math: every command here is a thin adapter that
//! deserializes input, calls `bettor_core`, and serializes the result.

/// Build and version information for the math engine.
///
/// Rendered in the UI so any result a user reports can be tied to the exact
/// engine that produced it.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    /// Version of the `bettor-core` math crate.
    pub core_version: &'static str,
    /// Version of this desktop shell.
    pub shell_version: &'static str,
    /// Whether the engine was compiled with optimizations. Monte Carlo results
    /// from an unoptimized build are correct but far slower.
    pub optimized: bool,
}

/// Reports the math engine's version. Phase 0 smoke test for the IPC path.
#[tauri::command]
fn engine_info() -> EngineInfo {
    EngineInfo {
        core_version: bettor_core::VERSION,
        shell_version: env!("CARGO_PKG_VERSION"),
        optimized: !cfg!(debug_assertions),
    }
}

/// Starts the desktop application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![engine_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
