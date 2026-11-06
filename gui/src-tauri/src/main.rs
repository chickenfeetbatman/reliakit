//! ReliaKit desktop host (Tauri 2).
//!
//! The window loads the web front end from gui/ (index.html, src/,
//! styles, assets). All application state and computation live in
//! the reliakit-engine crate; this binary only owns one Work and
//! forwards JSON requests to the engine dispatcher through the
//! single `dispatch` command. Project and CSV file IO happen in
//! the engine against the real filesystem, using the paths the
//! front end sends as fields.

use reliakit_engine::handle::{handle_str, new_work};
use reliakit_engine::Work;
use std::sync::Mutex;
use tauri::Manager;

struct AppState {
    work: Mutex<Work>,
}

/// One command for everything. The request arrives either as a
/// JSON string in `req`, or as a parsed JSON value (or string) in
/// `request`; both spellings are accepted so the front end can
/// send whichever its bridge provides. The return value is the
/// full UI state as a JSON string, as documented in engine/API.md.
#[tauri::command]
fn dispatch(state: tauri::State<'_, AppState>, req: Option<String>, request: Option<serde_json::Value>) -> String {
    let text = match request {
        Some(serde_json::Value::String(s)) => s,
        Some(v) => v.to_string(),
        None => req.unwrap_or_else(|| "{}".to_string()),
    };
    match state.work.lock() {
        Ok(mut w) => handle_str(&mut w, &text),
        Err(_) => "{\"status\":\"Internal error: state lock poisoned.\",\"screen\":\"start\"}".to_string(),
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            work: Mutex::new(new_work()),
        })
        .setup(|app| {
            // Window icon from the bundled asset (icons/icon.png,
            // generated from the original logo artwork). On Windows
            // the executable icon comes from bundle.icon instead;
            // this covers the live window on every platform.
            if let Some(window) = app.get_webview_window("main") {
                let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))?;
                window.set_icon(icon)?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![dispatch])
        .run(tauri::generate_context!())
        .expect("error while running the ReliaKit application");
}
