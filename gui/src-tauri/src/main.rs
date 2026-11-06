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
fn dispatch(
    state: tauri::State<'_, AppState>,
    req: Option<String>,
    request: Option<serde_json::Value>,
) -> String {
    let text = match request {
        Some(serde_json::Value::String(s)) => s,
        Some(v) => v.to_string(),
        None => req.unwrap_or_else(|| "{}".to_string()),
    };
    match state.work.lock() {
        Ok(mut w) => handle_str(&mut w, &text),
        Err(_) => {
            "{\"status\":\"Internal error: state lock poisoned.\",\"screen\":\"start\"}".to_string()
        }
    }
}

/// Headless smoke check, run when argv contains --headless and
/// before any window is built. It creates the engine work state,
/// loads Example 1 through the dispatcher, runs the Life Data fit,
/// and checks the headline value. Returns the process exit code:
/// 0 when the headline matches, 1 on any failure.
fn headless_check() -> i32 {
    let mut work = new_work();
    let loaded = handle_str(
        &mut work,
        "{\"action\":\"card_action\",\"fields\":{\"id\":\"example:1\"}}",
    );
    if loaded.contains("\"error\":{") {
        eprintln!("reliakit-app headless check FAILED: Example 1 load returned an error");
        return 1;
    }
    let fitted = handle_str(&mut work, "{\"action\":\"life-fit\",\"fields\":{}}");
    let state: serde_json::Value = match serde_json::from_str(&fitted) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("reliakit-app headless check FAILED: fit response is not JSON: {e}");
            return 1;
        }
    };
    if !state["error"].is_null() {
        eprintln!(
            "reliakit-app headless check FAILED: fit returned an error: {}",
            state["error"]
        );
        return 1;
    }
    // Canonical Example 1 headline, also asserted by the browser
    // tours: R(1000) = 0.9172 for the bearing life fit.
    let headline = state["headline"].as_str().unwrap_or("");
    if headline.contains("0.9172") {
        println!("reliakit-app headless check OK: Example 1 fit headline {headline}");
        0
    } else {
        eprintln!(
            "reliakit-app headless check FAILED: headline {headline:?} does not contain 0.9172"
        );
        1
    }
}

fn main() {
    if std::env::args().any(|a| a == "--headless") {
        std::process::exit(headless_check());
    }
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
