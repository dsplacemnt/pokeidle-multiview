//! Whether each account is logged in on Twitch and Kick, read from its browser session's
//! cookies: Twitch keeps `auth-token` and Kick `session_token` only while logged in. Only their
//! presence is looked at; the values are never read out, logged or sent anywhere.
//!
//! An account that isn't logged in on a platform earns nothing there, so the stream manager
//! opens no lives for it (they would only cost memory and CPU) and every place that shows the
//! account offers a "log in" button instead (`open_login`). As soon as the login shows up, the
//! login page closes by itself, the account's lives open and everything updates.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::config::Platform;

/// How often the cookies are checked when nothing asks for it sooner.
const EVERY: Duration = Duration::from_secs(20);
/// Set by `check_soon` (a Twitch/Kick page loaded, a login page opened): checks right away.
static NOW: AtomicBool = AtomicBool::new(false);

fn cookie_name(platform: Platform) -> (&'static str, &'static str) {
    match platform {
        Platform::Twitch => ("https://www.twitch.tv/", "auth-token"),
        Platform::Kick => ("https://kick.com/", "session_token"),
    }
}

/// Whether the session of `webview` holds the platform's login cookie; None if it can't be read.
/// Never call it on the UI thread or while holding the app state (WebView2 deadlocks).
fn logged_in(webview: &tauri::Webview, platform: Platform) -> Option<bool> {
    let (url, name) = cookie_name(platform);
    let cookies = webview.cookies_for_url(url.parse().ok()?).ok()?;
    Some(cookies.iter().any(|c| c.name() == name && !c.value().is_empty()))
}

/// Asks for a check within a second or two.
pub fn check_soon() {
    NOW.store(true, Ordering::SeqCst);
}

/// Checks every account now and then (and right away when asked).
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        // The games' sessions are ready once the slots have loaded.
        std::thread::sleep(Duration::from_secs(8));
        loop {
            check(&app);
            let since = Instant::now();
            while since.elapsed() < EVERY && !NOW.swap(false, Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(700));
            }
        }
    });
}

fn check(app: &AppHandle) {
    let ids: Vec<u8> = {
        let state = app.state::<Mutex<crate::AppState>>();
        let state = state.lock().unwrap();
        state.config.slots.iter().map(|s| s.id).collect()
    };
    // Read with the state unlocked: the cookie calls wait for the UI thread.
    let mut seen: Vec<(u8, Platform, bool)> = Vec::new();
    for id in ids {
        let Some(game) = app.get_webview(&crate::slot_label(id)) else { continue };
        for platform in [Platform::Twitch, Platform::Kick] {
            if let Some(on) = logged_in(&game, platform) {
                seen.push((id, platform, on));
            }
        }
    }
    let mut changed = false;
    let mut to_close: Vec<(u8, String)> = Vec::new();
    {
        let state = app.state::<Mutex<crate::AppState>>();
        let mut state = state.lock().unwrap();
        for (id, platform, on) in seen {
            let before = state.logins.insert((id, platform), on);
            if before == Some(on) {
                continue;
            }
            changed = true;
            // The first reading after startup only fills in what's known; changes get logged.
            if before.is_some() {
                let text = if on {
                    format!("{}: logged in", platform.name())
                } else {
                    format!("{}: logged out, its streams wait for a login", platform.name())
                };
                println!("[login] slot {id}: {text}");
                crate::log_event(&mut state, id, text);
            }
            if !on {
                // Logged out: every channel page shows "Follow", which says nothing.
                state.config.follows.retain(|key, _| !key.starts_with(&crate::config::balance_key(id, platform, "")));
                // Pills that already got a ♡ before the login was known lose it too.
                if let Some(panel) = state.panels.get_mut(&id) {
                    for page in panel.pages.iter_mut().filter(|p| p.platform == platform) {
                        page.not_following = false;
                    }
                }
            }
        }
        // Pages a "Log in" button opened are done once their account is in (also when it
        // already was: the button may have been used on a logged-in account).
        let done: Vec<(u8, String)> = state
            .panels
            .iter()
            .flat_map(|(id, panel)| panel.pages.iter().map(move |p| (*id, p)))
            .filter(|(id, p)| state.login_pages.contains(&p.label) && state.logins.get(&(*id, p.platform)) == Some(&true))
            .map(|(id, p)| (id, p.label.clone()))
            .collect();
        for (_, label) in &done {
            state.login_pages.remove(label);
        }
        to_close.extend(done);
        // Labels of pages that no longer exist.
        let alive: std::collections::HashSet<String> =
            state.panels.values().flat_map(|p| p.pages.iter().map(|pg| pg.label.clone())).collect();
        state.login_pages.retain(|l| alive.contains(l));
    }
    if !changed && to_close.is_empty() {
        return;
    }
    // Closing pages and opening lives create/close webviews: not on this thread's lock.
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        for (id, label) in to_close {
            if let Err(e) = crate::stop_pages_inner(&app2, id, Some(&[label])) {
                eprintln!("[login] could not close the login page: {e}");
            }
        }
        let state = app2.state::<Mutex<crate::AppState>>();
        crate::save_and_publish(&app2, &state.lock().unwrap());
    });
    // The manager opens (or stops opening) the account's lives now.
    crate::AUTO_WAKE.store(true, Ordering::SeqCst);
    let _ = app.emit_to("settings", "automation", crate::automation_view(app));
}

/// Each account's known login state, for the pages: slot → platform → logged in.
pub fn view(state: &crate::AppState) -> HashMap<u8, HashMap<Platform, bool>> {
    let mut out: HashMap<u8, HashMap<Platform, bool>> = HashMap::new();
    // Game-only accounts open no streams: their Twitch/Kick logins don't matter there.
    let game_only = |id: &u8| state.config.slots.iter().any(|s| s.id == *id && !s.runs_streams());
    for ((id, platform), on) in state.logins.iter().filter(|((id, _), _)| !game_only(id)) {
        out.entry(*id).or_default().insert(*platform, *on);
    }
    out
}

/// "Log in" buttons: leaves economy mode, shows the account fullscreen with the platform's
/// login page in front (reusing one that's already open), and checks again soon.
#[tauri::command]
pub async fn open_login(app: AppHandle, id: u8, platform: Platform) -> Result<(), String> {
    let existing = {
        let state = app.state::<Mutex<crate::AppState>>();
        let mut state = state.lock().unwrap();
        if !state.config.slots.iter().any(|s| s.id == id) {
            return Err("No such slot".into());
        }
        state.config.economy = false;
        if state.config.focus != Some(id) {
            if state.config.focus.is_some() {
                state.last_focus = state.config.focus;
            }
            state.config.focus = Some(id);
        }
        state
            .panels
            .get(&id)
            .and_then(|p| p.pages.iter().find(|pg| pg.platform == platform && pg.login.is_none()))
            .map(|pg| pg.label.clone())
    };
    let label = match existing {
        Some(label) => label,
        None => crate::create_page(&app, id, platform, platform.start_url(), false).map_err(|e| e.to_string())?,
    };
    app.state::<Mutex<crate::AppState>>().lock().unwrap().login_pages.insert(label.clone());
    crate::set_front(&app, id, Some(label)).map_err(|e| e.to_string())?;
    crate::apply_layout(&app).map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<crate::AppState>>();
    crate::save_and_publish(&app, &state.lock().unwrap());
    if let Some(main) = app.get_window("main") {
        let _ = main.set_focus();
    }
    check_soon();
    Ok(())
}
