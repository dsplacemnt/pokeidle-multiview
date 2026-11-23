//! Patch notes ("What's new"): the first time a newer version starts, a small window lists what
//! changed since the version used before. It can be opened again from ⚙ → About. The notes
//! themselves live in ui/changelog.js (one entry per release, in the app's languages).

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewUrl};

#[cfg(not(windows))]
use crate::compat::{HwndCompat, OwnerCompat};

const WINDOW: &str = "whatsnew";

/// The version shown "since" when the window opens by itself (None = every entry).
static SINCE: Mutex<Option<String>> = Mutex::new(None);

pub fn title(lang: crate::config::Language) -> &'static str {
    lang.pick("What's new — PokeIdle Multiview", "Novidades — PokeIdle Multiview", "Novedades — PokeIdle Multiview")
}

#[derive(Serialize)]
pub struct WhatsNew {
    /// This app's version.
    current: &'static str,
    /// Show the entries newer than this one (the version used before the update); None = all.
    since: Option<String>,
}

#[tauri::command]
pub fn get_whats_new() -> WhatsNew {
    WhatsNew { current: env!("CARGO_PKG_VERSION"), since: SINCE.lock().unwrap().clone() }
}

/// Opens (or brings back) the window with every entry (⚙ → About).
#[tauri::command]
pub async fn open_whats_new(app: AppHandle) -> Result<(), String> {
    *SINCE.lock().unwrap() = None;
    open(&app)
}

fn open(app: &AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(WINDOW) {
        let _ = existing.unminimize();
        return existing.set_focus().map_err(|e| e.to_string());
    }
    let main = app.get_window("main").ok_or("No window")?;
    let (w, h) = (520.0, 640.0);
    let lang = app.state::<Mutex<crate::AppState>>().lock().unwrap().config.language;
    let builder = tauri::WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::App("whatsnew.html".into()))
        .title(title(lang))
        .inner_size(w, h);
    let builder = match crate::centered_over(&main, w, h) {
        Some((x, y)) => builder.position(x, y),
        None => builder.center(),
    };
    let builder = match crate::local_args() {
        Some(args) => builder.additional_browser_args(args),
        None => builder,
    };
    builder
        .minimizable(false)
        .maximizable(false)
        .owner_raw(main.hwnd().map_err(|e| e.to_string())?)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// What to do at startup: the version used before (None = nothing to show). A first run of
/// the app shows nothing; a config from before 0.15.0 (which didn't record it) counts as 0.14.0.
pub fn previous_version(seen: Option<&str>, used_before: bool, current: &str) -> Option<String> {
    match seen {
        Some(v) if v == current => None,
        Some(v) => Some(v.to_string()),
        None if used_before => Some("0.14.0".to_string()),
        None => None,
    }
}

/// Called once the main window is shown: records this version and, after an update, opens
/// the notes since the version used before.
pub fn after_start(app: &AppHandle) {
    let current = env!("CARGO_PKG_VERSION");
    let since = {
        let state = app.state::<Mutex<crate::AppState>>();
        let mut state = state.lock().unwrap();
        if state.config.seen_version.as_deref() == Some(current) {
            return;
        }
        // A config that already saved its window or seen points was used by an older version.
        let used_before = state.config.window.is_some() || !state.config.balances.is_empty();
        let since = previous_version(state.config.seen_version.as_deref(), used_before, current);
        state.config.seen_version = Some(current.to_string());
        if let Err(e) = state.config.save(&state.config_path) {
            eprintln!("[config] could not save: {e}");
        }
        since
    };
    let Some(since) = since else { return };
    println!("[whatsnew] updated from {since} to {current}: showing the notes");
    *SINCE.lock().unwrap() = Some(since);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // A moment after the main window, so it opens on top of it.
        std::thread::sleep(std::time::Duration::from_millis(1500));
        if let Err(e) = open(&app) {
            eprintln!("[whatsnew] could not open: {e}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::previous_version;

    #[test]
    fn shows_notes_only_after_an_update() {
        assert_eq!(previous_version(Some("0.15.0"), true, "0.15.0"), None);
        assert_eq!(previous_version(Some("0.14.0"), true, "0.15.0").as_deref(), Some("0.14.0"));
        // Configs from before 0.15.0 don't say; a first run shows nothing.
        assert_eq!(previous_version(None, true, "0.15.0").as_deref(), Some("0.14.0"));
        assert_eq!(previous_version(None, false, "0.15.0"), None);
    }
}
