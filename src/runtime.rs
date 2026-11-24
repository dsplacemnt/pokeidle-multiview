//! When Windows updates the WebView2 runtime (Edge updates it on its own, often at night), the
//! browsers already running keep the old version, but new pages can't be created in them any
//! more: every new webview fails with 0x80010108 ("the object invoked has disconnected from
//! its clients"). Streams that go offline are then never replaced and Settings won't open.
//!
//! So the app restarts itself on the new runtime: when WebView2 says a new version is available
//! (NewBrowserVersionAvailable), or when creating a page fails with that error. The open
//! streams are saved first, and a window that was away (minimized or in the tray) comes back
//! away, so a restart at night doesn't put it on screen.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Manager};

/// The error WebView2 gives once the runtime was updated under the running browsers.
const DISCONNECTED: &str = "80010108";
/// A little wait, so a runtime update that is still installing has finished.
const DELAY: Duration = Duration::from_secs(20);

static SCHEDULED: AtomicBool = AtomicBool::new(false);

/// The file that tells the next start how the window was (deleted once read).
fn marker(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_local_data_dir().ok().map(|d| d.join("restart-away"))
}

/// Was the app restarted while its window was away? ("tray" or "minimized"); reads it once.
pub fn restarted_away(app: &AppHandle) -> Option<bool> {
    let path = marker(app)?;
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    Some(text.trim() == "tray")
}

/// A page couldn't be created: if it's the runtime-update error, restart.
pub fn page_failed(app: &AppHandle, error: &str) {
    if error.contains(DISCONNECTED) {
        restart_soon(app, "a new page couldn't be created (WebView2 was updated)");
    }
}

/// Restarts the app in a little while, once (saves the streams, remembers the window).
pub fn restart_soon(app: &AppHandle, why: &str) {
    if SCHEDULED.swap(true, Ordering::SeqCst) {
        return;
    }
    println!("[runtime] {why}: restarting in {} s", DELAY.as_secs());
    {
        let state = app.state::<Mutex<crate::AppState>>();
        let mut state = state.lock().unwrap();
        for id in state.config.slots.iter().map(|s| s.id).collect::<Vec<_>>() {
            crate::log_event(&mut state, id, "WebView2 was updated: the app restarts to use it");
        }
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(DELAY);
        // The open streams, so the new start opens them again.
        crate::remember_panels_known(&app);
        if crate::away::is_away() {
            if let Some(path) = marker(&app) {
                let _ = std::fs::write(path, if crate::away::in_tray() { "tray" } else { "minimized" });
            }
        }
        println!("[runtime] restarting now");
        // The test switch must not make the new start restart again.
        std::env::remove_var("MULTIVIEW_FAKE_RUNTIME_UPDATE");
        app.restart();
    });
}

/// Listens for a new WebView2 runtime in this page's browser (called for each slot's game).
#[cfg(windows)]
pub fn watch(webview: &tauri::Webview, app: AppHandle) {
    // MULTIVIEW_FAKE_RUNTIME_UPDATE=1 (testing): acts as if WebView2 said so 90 s after start.
    if std::env::var_os("MULTIVIEW_FAKE_RUNTIME_UPDATE").is_some() && !SCHEDULED.load(Ordering::SeqCst) {
        let test = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(90));
            restart_soon(&test, "test: pretending WebView2 has a new version");
        });
    }
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use webview2_com::NewBrowserVersionAvailableEventHandler;
    use windows_core::Interface;
    let _ = webview.with_webview(move |platform| unsafe {
        let Ok(core) = platform.controller().CoreWebView2() else { return };
        let Ok(env) = core.cast::<ICoreWebView2_2>().and_then(|c| c.Environment()) else { return };
        let handler = NewBrowserVersionAvailableEventHandler::create(Box::new(move |_, _| {
            restart_soon(&app, "WebView2 has a new version");
            Ok(())
        }));
        let mut token = 0i64;
        let _ = env.add_NewBrowserVersionAvailable(&handler, &mut token);
    });
}

#[cfg(not(windows))]
pub fn watch(_webview: &tauri::Webview, _app: AppHandle) {}
