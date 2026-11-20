//! Updates: checks GitHub for a newer release (its latest.json) a little after startup and
//! every 6 hours. The top bar then shows "⬆ <version>"; one click downloads the installer,
//! checks its signature against the public key in tauri.conf.json, installs it and reopens the
//! app. Copies that weren't installed (the portable exe, debug builds) can't replace
//! themselves: they only show that a new version is out and open its release page.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

const FIRST_CHECK: Duration = Duration::from_secs(20);
const EVERY: Duration = Duration::from_secs(6 * 3600);
const RELEASES: &str = "https://github.com/dsplacemnt/pokeidle-multiview/releases/latest";

#[derive(Serialize, Clone, Default)]
pub struct UpdateView {
    current: &'static str,
    /// The newer version on GitHub, if any.
    available: Option<String>,
    /// This copy can install it by itself (an installed copy, not the portable exe).
    can_install: bool,
    checking: bool,
    /// Download progress while installing, 0–100.
    progress: Option<u8>,
    error: Option<String>,
    /// When the last check finished (ms since 1970).
    checked_at: Option<i64>,
}

#[derive(Default)]
pub struct UpdateState(Mutex<UpdateView>);

/// Installed by the MSI (Program Files) or the setup exe (its own folder in LocalAppData).
fn installed() -> bool {
    if cfg!(debug_assertions) {
        return false;
    }
    let Ok(exe) = std::env::current_exe() else { return false };
    let exe = exe.to_string_lossy().to_lowercase();
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default().to_lowercase();
    exe.contains("\\program files") || (!local.is_empty() && exe.starts_with(&local))
}

fn view(app: &AppHandle) -> UpdateView {
    app.state::<UpdateState>().0.lock().unwrap().clone()
}

fn change(app: &AppHandle, f: impl FnOnce(&mut UpdateView)) {
    let snapshot = {
        let state = app.state::<UpdateState>();
        let mut v = state.0.lock().unwrap();
        f(&mut v);
        v.clone()
    };
    for page in ["toolbar", "settings"] {
        let _ = app.emit_to(page, "update", snapshot.clone());
    }
}

pub fn start(app: AppHandle) {
    change(&app, |v| {
        v.current = env!("CARGO_PKG_VERSION");
        v.can_install = installed();
    });
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            tauri::async_runtime::block_on(check(&app));
            std::thread::sleep(EVERY);
        }
    });
}

async fn check(app: &AppHandle) {
    change(app, |v| {
        v.checking = true;
        v.error = None;
    });
    let result = match app.updater() {
        Ok(updater) => match updater.check().await {
            // The latest release has no latest.json (it predates the updater): nothing newer.
            Err(tauri_plugin_updater::Error::ReleaseNotFound) => Ok(None),
            other => other.map_err(|e| e.to_string()),
        },
        Err(e) => Err(e.to_string()),
    };
    let now = crate::now_ms();
    match result {
        Ok(found) => {
            let version = found.map(|u| u.version);
            if let Some(v) = &version {
                println!("[update] version {v} is available");
            }
            change(app, |v| {
                v.available = version;
                v.checking = false;
                v.checked_at = Some(now);
            });
        }
        Err(e) => {
            eprintln!("[update] check failed: {e}");
            change(app, |v| {
                v.checking = false;
                v.error = Some(e);
                v.checked_at = Some(now);
            });
        }
    }
}

#[tauri::command]
pub fn get_update(app: AppHandle) -> UpdateView {
    view(&app)
}

/// ⚙ → About → "Check now".
#[tauri::command]
pub async fn check_update(app: AppHandle) -> UpdateView {
    check(&app).await;
    view(&app)
}

/// "⬆ <version>": downloads, verifies and installs the update, then the app reopens. A copy
/// that can't install opens the release page instead.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    if !view(&app).can_install {
        if let Ok(url) = RELEASES.parse() {
            crate::external::open_in_browser(&url);
        }
        return Ok(());
    }
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("No update available")?;
    // Saves the streams running in each slot, as quitting does, so they come back.
    crate::remember_panels(&app);
    change(&app, |v| {
        v.progress = Some(0);
        v.error = None;
    });
    let mut got: u64 = 0;
    let result = update
        .download_and_install(
            |chunk, total| {
                got += chunk as u64;
                if let Some(total) = total.filter(|t| *t > 0) {
                    let pct = ((got * 100) / total).min(100) as u8;
                    change(&app, |v| v.progress = Some(pct));
                }
            },
            || println!("[update] downloaded, installing"),
        )
        .await;
    match result {
        // On Windows the installer takes over and closes the app; this only runs if it didn't.
        Ok(()) => app.restart(),
        Err(e) => {
            change(&app, |v| {
                v.progress = None;
                v.error = Some(e.to_string());
            });
            Err(e.to_string())
        }
    }
}
