//! DropAgentX desktop shell — Tauri v2 skeleton (Phase 7 Step 6, audit §9/D2).
//!
//! Loads the ONE combined static root (`front/dist` — same artifact the
//! server's ServeDir and the PWA use, D3) in a system webview: no Electron,
//! no bundled browser, ~10 MB binaries. No custom commands are exposed yet —
//! the window is a pure viewer over the frozen-API frontend until Step 9
//! decides otherwise (fail-closed by default).
//
// Release builds are windowed (no console flash); debug keeps the console.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running DropAgentX desktop shell");
}
