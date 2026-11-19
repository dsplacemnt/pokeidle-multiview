#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod channels;
mod config;
mod hunt;
mod layout;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::webview::{NewWindowResponse, WebviewBuilder};
use tauri::window::WindowBuilder;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Url, WebviewUrl, WindowEvent,
};

use config::{Config, Platform, SavedPanel, SlotConfig, MAX_SLOTS};
use layout::Rect;

const TOOLBAR_HEIGHT: f64 = 40.0;
const GAME_URL: &str = "https://pokeidle.io/";
const WEBAUTHN_BLOCK: &str = include_str!("../scripts/webauthn-block.js");
const DESKTOP_LAYOUT: &str = include_str!("../scripts/desktop-layout.js");

struct AppState {
    config: Config,
    config_path: PathBuf,
    data_root: PathBuf,
    hunt_dir: Option<PathBuf>,
    /// Twitch/Kick pages per slot (not saved yet: they don't come back after a restart).
    panels: HashMap<u8, PanelState>,
}

#[derive(Serialize, Clone)]
struct StateView {
    slots: Vec<SlotConfig>,
    focus: Option<u8>,
    max_slots: u8,
    hunt_dir: Option<String>,
    panels: HashMap<u8, PanelState>,
}

fn slot_label(id: u8) -> String {
    format!("slot-{id}")
}

// ---------------------------------------------------------------- navigation policy

fn is_pokeidle(host: &str) -> bool {
    host == "pokeidle.io" || host == "www.pokeidle.io"
}

fn is_platform(host: &str) -> bool {
    ["twitch.tv", "kick.com"]
        .iter()
        .any(|d| host == *d || host.ends_with(&format!(".{d}")))
}

/// Google sign-in can hop through country domains (accounts.google.com.br, google.de).
fn is_google_auth(host: &str) -> bool {
    if host == "accounts.youtube.com" {
        return true;
    }
    let labels: Vec<&str> = host.split('.').collect();
    let Some(i) = labels.iter().rposition(|l| *l == "google") else { return false };
    match &labels[i + 1..] {
        [tld] => tld.len() == 2 || *tld == "com",
        ["com" | "co", cc] => cc.len() == 2,
        _ => false,
    }
}

/// The game's "Login with Discord" redirects here.
fn is_discord_auth(host: &str) -> bool {
    ["discord.com", "discordapp.com"]
        .iter()
        .any(|d| host == *d || host.ends_with(&format!(".{d}")))
}

/// Slots stay on the game and on the sign-in pages its logins redirect to (Google, Discord,
/// Twitch, Kick), so each flow runs inside the slot and comes back to the game. WebView2 runs
/// this for server redirects too.
fn slot_may_navigate(url: &Url) -> bool {
    match url.scheme() {
        "about" | "data" | "blob" => true,
        "https" => url.host_str().is_some_and(|h| {
            is_pokeidle(h) || is_platform(h) || is_google_auth(h) || is_discord_auth(h)
        }),
        _ => false,
    }
}

#[cfg(test)]
mod nav_tests {
    use super::*;

    fn ok(url: &str) -> bool {
        slot_may_navigate(&url.parse().unwrap())
    }

    #[test]
    fn login_redirects_are_allowed() {
        for url in [
            "https://pokeidle.io/auth/discord",
            "https://discord.com/oauth2/authorize?client_id=1",
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://accounts.google.com.br/accounts/SetSID",
            "https://www.google.de/",
            "https://id.twitch.tv/oauth2/authorize",
            "https://www.twitch.tv/login",
            "https://id.kick.com/oauth/authorize",
            "about:blank",
        ] {
            assert!(ok(url), "{url} should be allowed");
        }
    }

    #[test]
    fn other_sites_are_blocked() {
        for url in [
            "https://example.com/",
            "https://google.evil.com/",
            "https://notdiscord.com/",
            "https://pokeidle.io.evil.com/",
            "https://twitch.tv.evil.com/",
            "http://pokeidle.io/",
            "file:///C:/Windows/win.ini",
        ] {
            assert!(!ok(url), "{url} should be blocked");
        }
    }
}

// ---------------------------------------------------------------- state + layout

fn state_view(state: &AppState) -> StateView {
    StateView {
        slots: state.config.slots.clone(),
        focus: state.config.focus,
        max_slots: MAX_SLOTS,
        hunt_dir: state.hunt_dir.as_ref().map(|p| p.display().to_string()),
        panels: state.panels.clone(),
    }
}

fn save_and_publish(app: &AppHandle, state: &AppState) {
    if let Err(e) = state.config.save(&state.config_path) {
        eprintln!("[config] could not save: {e}");
    }
    let _ = app.emit_to("toolbar", "state", state_view(state));
}

fn window_logical_size(app: &AppHandle) -> tauri::Result<LogicalSize<f64>> {
    let window = app.get_window("main").expect("main window exists");
    let scale = window.scale_factor()?;
    Ok(window.inner_size()?.to_logical::<f64>(scale))
}

/// Where each open slot goes for the current window size and focus (None = hidden).
fn slot_tiles(app: &AppHandle) -> tauri::Result<Vec<(u8, Option<Rect>)>> {
    let size = window_logical_size(app)?;
    let config = app.state::<Mutex<AppState>>().lock().unwrap().config.clone();
    let ids: Vec<u8> = config.slots.iter().map(|s| s.id).collect();
    let visible: Vec<u8> = match config.focus {
        Some(f) if ids.contains(&f) => vec![f],
        _ => ids.clone(),
    };
    let area = Rect {
        x: 0.0,
        y: TOOLBAR_HEIGHT,
        w: size.width,
        h: (size.height - TOOLBAR_HEIGHT).max(0.0),
    };
    let tiles = layout::grid(area, visible.len());
    Ok(ids
        .into_iter()
        .map(|id| (id, visible.iter().position(|v| *v == id).map(|i| tiles[i])))
        .collect())
}

/// Same size as the tile, but entirely left of the window's client area.
fn parked(r: Rect) -> Rect {
    Rect { x: -(r.w + 100.0), ..r }
}

fn apply_layout(app: &AppHandle) -> tauri::Result<()> {
    if app.get_window("main").is_none() {
        return Ok(());
    }
    let size = window_logical_size(app)?;
    if let Some(toolbar) = app.get_webview("toolbar") {
        toolbar.set_position(LogicalPosition::new(0.0, 0.0))?;
        toolbar.set_size(LogicalSize::new(size.width, TOOLBAR_HEIGHT))?;
    }
    let fronts: HashMap<u8, Platform> = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        state.panels.iter().filter_map(|(id, p)| Some((*id, p.front?))).collect()
    };
    for (id, tile) in slot_tiles(app)? {
        // The shown panel (or the game) takes the slot's tile. Everything else in the slot
        // is hidden but keeps running, and keeps the tile's size so pages don't reflow.
        let front_label = match fronts.get(&id) {
            Some(p) => panel_label(id, *p),
            None => slot_label(id),
        };
        let size = window_logical_size(app)?;
        let tile_or_full = tile.unwrap_or(Rect {
            x: 0.0,
            y: TOOLBAR_HEIGHT,
            w: size.width,
            h: (size.height - TOOLBAR_HEIGHT).max(0.0),
        });
        let labels = std::iter::once(slot_label(id)).chain(PLATFORMS.map(|p| panel_label(id, p)));
        for label in labels {
            let Some(webview) = app.get_webview(&label) else { continue };
            let is_panel = label != slot_label(id);
            if label == front_label && tile.is_some() {
                let r = tile_or_full;
                webview.set_position(LogicalPosition::new(r.x, r.y))?;
                webview.set_size(LogicalSize::new(r.w, r.h))?;
                webview.show()?;
            } else if is_panel {
                // Background streams are parked outside the window instead of hidden: the
                // page still counts as visible, so players start (and keep) playing.
                let r = parked(tile_or_full);
                webview.set_position(LogicalPosition::new(r.x, r.y))?;
                webview.set_size(LogicalSize::new(r.w, r.h))?;
                webview.show()?;
            } else {
                // The game behind a shown panel, or a slot behind a focused one.
                if let Some(r) = tile {
                    webview.set_position(LogicalPosition::new(r.x, r.y))?;
                    webview.set_size(LogicalSize::new(r.w, r.h))?;
                }
                webview.hide()?;
            }
        }
    }
    Ok(())
}

/// A webview in slot `slot_id`'s browser session. The game and its Twitch/Kick panel both
/// use this, so they share one WebView2 environment: same cookies, so a Twitch login made
/// in the panel is the slot's Twitch login. WebView2 requires every webview on one data
/// folder to use identical browser options, which is why they all come from here.
fn slot_session_builder(
    app: &AppHandle,
    slot_id: u8,
    label: String,
    url: &str,
) -> WebviewBuilder<tauri::Wry> {
    let (data_dir, hunt_dir) = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        (state.data_root.join(slot_label(slot_id)), state.hunt_dir.clone())
    };

    // Each slot gets its own WebView2 user data folder: separate cookies, storage and
    // logins that survive restarts.
    let mut builder = WebviewBuilder::new(label, WebviewUrl::External(url.parse().unwrap()))
    .data_directory(data_dir)
    .zoom_hotkeys_enabled(true)
    .on_navigation(|url| {
        let ok = slot_may_navigate(url);
        if !ok {
            eprintln!("[nav] blocked {url}");
        }
        ok
    })
    // Popups (e.g. a sign-in window) open with WebView2's default window, which shares the
    // slot's environment and therefore its login. Anything off the allow list is dropped.
    .on_new_window(|url, _features| {
        if slot_may_navigate(&url) {
            NewWindowResponse::Allow
        } else {
            eprintln!("[popup] blocked {url}");
            NewWindowResponse::Deny
        }
    });

    if std::env::var_os("MULTIVIEW_ALLOW_WEBAUTHN").is_none() {
        builder = builder.initialization_script_for_all_frames(WEBAUTHN_BLOCK);
    }
    // Debug builds only: MULTIVIEW_DEVTOOLS=1 exposes slot N to Chrome DevTools Protocol on
    // 127.0.0.1:930N (open edge://inspect or http://127.0.0.1:930N/json). Setting browser args
    // replaces wry's defaults, so they are repeated here.
    #[cfg(debug_assertions)]
    if std::env::var_os("MULTIVIEW_DEVTOOLS").is_some() {
        builder = builder.additional_browser_args(&format!(
            "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
             --autoplay-policy=no-user-gesture-required \
             --remote-debugging-address=127.0.0.1 --remote-debugging-port={}",
            9300 + u16::from(slot_id)
        ));
    }
    if let Some(dir) = hunt_dir {
        builder = builder.browser_extensions_enabled(true).extensions_path(dir);
    }
    builder
}

fn tile_of(app: &AppHandle, slot_id: u8) -> tauri::Result<Option<Rect>> {
    Ok(slot_tiles(app)?
        .into_iter()
        .find(|(id, _)| *id == slot_id)
        .and_then(|(_, tile)| tile))
}

fn create_slot_webview(app: &AppHandle, slot: &SlotConfig) -> tauri::Result<()> {
    let window = app.get_window("main").expect("main window exists");
    // The game's handlers replace the session defaults: Twitch/Kick channel links (the game
    // opens its official channels in a new tab) go to the slot's own stream panel instead of a
    // loose popup window.
    let (nav_app, popup_app, id) = (app.clone(), app.clone(), slot.id);
    let builder = slot_session_builder(app, slot.id, slot_label(slot.id), GAME_URL)
        .initialization_script(DESKTOP_LAYOUT)
        .on_navigation(move |url| {
            if let Some(channel) = channels::channel_from_url(url) {
                spawn_route_channel(&nav_app, id, channel);
                return false;
            }
            let ok = slot_may_navigate(url);
            if !ok {
                eprintln!("[nav] blocked {url}");
            }
            ok
        })
        .on_new_window(move |url, _features| {
            if let Some(channel) = channels::channel_from_url(&url) {
                spawn_route_channel(&popup_app, id, channel);
                return NewWindowResponse::Deny;
            }
            if slot_may_navigate(&url) {
                NewWindowResponse::Allow
            } else {
                eprintln!("[popup] blocked {url}");
                NewWindowResponse::Deny
            }
        });

    // Start at the final tile size: the game picks its layout from the size it loads at.
    let tile = tile_of(app, slot.id)?;
    let webview = match tile {
        Some(r) => window.add_child(
            builder,
            LogicalPosition::new(r.x, r.y),
            LogicalSize::new(r.w, r.h),
        )?,
        // Hidden behind a focused slot: give it the full area so it still loads as desktop.
        None => {
            let size = window_logical_size(app)?;
            let webview = window.add_child(
                builder,
                LogicalPosition::new(0.0, TOOLBAR_HEIGHT),
                LogicalSize::new(size.width, (size.height - TOOLBAR_HEIGHT).max(0.0)),
            )?;
            webview.hide()?;
            webview
        }
    };
    webview.set_zoom(slot.zoom)?;
    Ok(())
}

// ---------------------------------------------------------------- Twitch/Kick panel

impl Platform {
    /// Where the panel opens: the sign-in page on Twitch, the home page on Kick (its
    /// "Log In" button opens a dialog there).
    fn start_url(self) -> &'static str {
        match self {
            Platform::Twitch => "https://www.twitch.tv/login",
            Platform::Kick => "https://kick.com/",
        }
    }
}

const PLATFORMS: [Platform; 2] = [Platform::Twitch, Platform::Kick];

impl Platform {
    fn key(self) -> &'static str {
        match self {
            Platform::Twitch => "twitch",
            Platform::Kick => "kick",
        }
    }
}

fn panel_label(id: u8, platform: Platform) -> String {
    format!("slot-{id}-{}", platform.key())
}

/// Per slot: which platform panels exist (their page keeps running) and which one, if any,
/// is shown in place of the game.
#[derive(Serialize, Clone, Default, Debug)]
struct PanelState {
    front: Option<Platform>,
    running: Vec<Platform>,
}

/// Mutes or unmutes a webview with WebView2's own switch, so the page can't override it.
fn set_muted(webview: &tauri::Webview, muted: bool) {
    let result = webview.with_webview(move |platform_webview| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_8;
        use windows_core::Interface;
        let muted_ok = platform_webview
            .controller()
            .CoreWebView2()
            .and_then(|core| core.cast::<ICoreWebView2_8>())
            .and_then(|core| core.SetIsMuted(muted));
        if let Err(e) = muted_ok {
            eprintln!("[panel] could not set muted={muted}: {e}");
        }
    });
    if let Err(e) = result {
        eprintln!("[panel] could not reach webview to mute: {e}");
    }
}

/// 📺/🟩 button. Not running → opens the site over the slot. Shown → back to the game, the
/// page keeps running hidden and muted (so a stream keeps counting). Hidden → shown again,
/// right where it was.
fn toggle_panel_inner(app: &AppHandle, id: u8, platform: Platform) -> tauri::Result<()> {
    let state = app.state::<Mutex<AppState>>();
    let label = panel_label(id, platform);
    let front = state.lock().unwrap().panels.get(&id).and_then(|p| p.front);

    if front == Some(platform) {
        if let Some(panel) = app.get_webview(&label) {
            set_muted(&panel, true);
        }
        state.lock().unwrap().panels.entry(id).or_default().front = None;
    } else {
        // The other platform, if it was shown, goes to the background muted too.
        if let Some(other) = front.and_then(|p| app.get_webview(&panel_label(id, p))) {
            set_muted(&other, true);
        }
        match app.get_webview(&label) {
            Some(panel) => set_muted(&panel, false),
            None => create_panel(app, id, platform, platform.start_url(), false)?,
        }
        state.lock().unwrap().panels.entry(id).or_default().front = Some(platform);
    }
    remember_panels(app);
    apply_layout(app)
}

/// Webview event handlers run on the UI thread, where creating another webview can deadlock,
/// so routing happens on a worker (like the async commands).
fn spawn_route_channel(app: &AppHandle, id: u8, channel: channels::Channel) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        println!("[route] slot {id}: {:?} channel {}", channel.platform, channel.login);
        match route_channel(&app, id, &channel) {
            Ok(()) => {
                let state = app.state::<Mutex<AppState>>();
                save_and_publish(&app, &state.lock().unwrap());
            }
            Err(e) => eprintln!("[route] slot {id}: could not open {}: {e}", channel.page_url()),
        }
    });
}

/// A channel link clicked in the game. The platform's panel switches to that channel and stays
/// where it was (shown or in the background). The first time, there is no panel yet: it opens
/// visible, so the player can check the slot is logged in, then sends it to the background.
fn route_channel(app: &AppHandle, id: u8, channel: &channels::Channel) -> tauri::Result<()> {
    let url = channel.page_url();
    match app.get_webview(&panel_label(id, channel.platform)) {
        Some(panel) => {
            let already_there = panel
                .url()
                .ok()
                .and_then(|u| channels::channel_from_url(&u))
                .is_some_and(|c| &c == channel);
            if !already_there {
                panel.navigate(url.parse().unwrap())?;
            }
            remember_panels(app);
            apply_layout(app)
        }
        None => {
            create_panel(app, id, channel.platform, &url, false)?;
            // Now running but not shown: toggling shows it (and mutes whatever was shown).
            toggle_panel_inner(app, id, channel.platform)
        }
    }
}

/// Creates the platform page for a slot. `background` = hidden and muted from the start
/// (used when streams come back at startup).
fn create_panel(
    app: &AppHandle,
    id: u8,
    platform: Platform,
    url: &str,
    background: bool,
) -> tauri::Result<()> {
    let window = app.get_window("main").expect("main window exists");
    let builder = slot_session_builder(app, id, panel_label(id, platform), url);
    let mut r = tile_of(app, id)?.unwrap_or_else(|| {
        // Slot hidden behind a focused one: use the whole area so the page lays out normally.
        let size = window_logical_size(app).unwrap_or(LogicalSize::new(1280.0, 800.0));
        Rect { x: 0.0, y: TOOLBAR_HEIGHT, w: size.width, h: (size.height - TOOLBAR_HEIGHT).max(0.0) }
    });
    if background {
        // Born off-screen (not hidden) so the player autoplays; see apply_layout.
        r = parked(r);
    }
    let panel =
        window.add_child(builder, LogicalPosition::new(r.x, r.y), LogicalSize::new(r.w, r.h))?;
    if background {
        set_muted(&panel, true);
    }
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    let panels = state.panels.entry(id).or_default();
    if !panels.running.contains(&platform) {
        panels.running.push(platform);
    }
    Ok(())
}

/// Writes every running panel and the page it is on into the config, so the streams come
/// back after a restart. Called on each panel change and when the window closes (to catch
/// channels the user switched to inside a panel).
fn remember_panels(app: &AppHandle) {
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    let running = state.panels.clone();
    for slot in &mut state.config.slots {
        let platforms = running.get(&slot.id).map(|p| p.running.clone()).unwrap_or_default();
        let previous = std::mem::take(&mut slot.panels);
        slot.panels = platforms
            .into_iter()
            .filter_map(|platform| {
                let url = app
                    .get_webview(&panel_label(slot.id, platform))
                    .and_then(|w| w.url().ok())
                    .map(|u| u.to_string())
                    // Not readable right now (e.g. still being created): keep what we had.
                    .or_else(|| previous.iter().find(|p| p.platform == platform).map(|p| p.url.clone()))?;
                Some(SavedPanel { platform, url })
            })
            .collect();
    }
    if let Err(e) = state.config.save(&state.config_path) {
        eprintln!("[config] could not save: {e}");
    }
}

/// Startup: reopen the pages that were running, in the background and muted.
fn restore_panels(app: &AppHandle, slots: &[SlotConfig]) {
    for slot in slots {
        for saved in &slot.panels {
            let allowed = saved.url.parse::<Url>().is_ok_and(|u| slot_may_navigate(&u));
            let url = if allowed { saved.url.as_str() } else { saved.platform.start_url() };
            match create_panel(app, slot.id, saved.platform, url, true) {
                Ok(()) => println!("[panel] slot {}: resumed {}", slot.id, url),
                Err(e) => eprintln!("[panel] slot {}: could not resume {url}: {e}", slot.id),
            }
        }
    }
}

/// ⏹: closes the platform's page for good (stops its stream).
fn stop_panel_inner(app: &AppHandle, id: u8, platform: Platform) -> tauri::Result<()> {
    if let Some(panel) = app.get_webview(&panel_label(id, platform)) {
        panel.close()?;
    }
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        let panels = state.panels.entry(id).or_default();
        panels.running.retain(|p| *p != platform);
        if panels.front == Some(platform) {
            panels.front = None;
        }
    }
    remember_panels(app);
    apply_layout(app)
}

// ---------------------------------------------------------------- commands

#[tauri::command]
fn get_state(state: tauri::State<'_, Mutex<AppState>>) -> StateView {
    state_view(&state.lock().unwrap())
}

// Async on purpose: creating a webview from a sync command deadlocks on Windows.
#[tauri::command]
async fn add_slot(app: AppHandle) -> Result<u8, String> {
    let slot = {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        let id = state.config.next_free_id().ok_or("All 4 slots are open")?;
        let slot = SlotConfig::new(id);
        state.config.slots.push(slot.clone());
        state.config.slots.sort_by_key(|s| s.id);
        slot
    };
    create_slot_webview(&app, &slot).map_err(|e| e.to_string())?;
    apply_layout(&app).map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
    Ok(slot.id)
}

#[tauri::command]
async fn close_slot(app: AppHandle, id: u8) -> Result<(), String> {
    // Forget the slot first, so a failed close never leaves it half-open in the config.
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        state.config.slots.retain(|s| s.id != id);
        if state.config.focus == Some(id) {
            state.config.focus = None;
        }
        state.panels.remove(&id);
    }
    for label in PLATFORMS.map(|p| panel_label(id, p)).into_iter().chain([slot_label(id)]) {
        if let Some(webview) = app.get_webview(&label) {
            if let Err(e) = webview.close() {
                eprintln!("[{label}] close failed: {e}");
            }
        }
    }
    apply_layout(&app).map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
    Ok(())
}

// Async: may create a webview (see add_slot).
#[tauri::command]
async fn toggle_panel(app: AppHandle, id: u8, platform: Platform) -> Result<(), String> {
    toggle_panel_inner(&app, id, platform).map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
    Ok(())
}

#[tauri::command]
async fn stop_panel(app: AppHandle, id: u8, platform: Platform) -> Result<(), String> {
    stop_panel_inner(&app, id, platform).map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
    Ok(())
}

#[tauri::command]
fn reload_slot(app: AppHandle, id: u8) -> Result<(), String> {
    let webview = app.get_webview(&slot_label(id)).ok_or("No such slot")?;
    webview.reload().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_zoom(app: AppHandle, id: u8, zoom: f64) -> Result<(), String> {
    // 50%–200% in 10% steps.
    let zoom = ((zoom * 10.0).round() / 10.0).clamp(0.5, 2.0);
    let webview = app.get_webview(&slot_label(id)).ok_or("No such slot")?;
    webview.set_zoom(zoom).map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    if let Some(slot) = state.config.slots.iter_mut().find(|s| s.id == id) {
        slot.zoom = zoom;
    }
    save_and_publish(&app, &state);
    Ok(())
}

#[tauri::command]
fn focus_slot(app: AppHandle, id: Option<u8>) -> Result<(), String> {
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        // Clicking the focused slot again goes back to the grid.
        state.config.focus = if state.config.focus == id { None } else { id };
    }
    apply_layout(&app).map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
    Ok(())
}

// ---------------------------------------------------------------- startup

fn main() {
    tauri::Builder::default()
        // Remembers the main window's size, position and maximized state between runs.
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let handle = app.handle().clone();
            let base = app.path().app_local_data_dir()?;
            let config_path = base.join("config.json");
            let config = Config::load(&config_path);
            // MULTIVIEW_EXTENSION_DIR points at any unpacked extension instead of /hunt, which
            // lets the extension path be tested on a PC that doesn't have /hunt installed.
            let source = std::env::var_os("MULTIVIEW_EXTENSION_DIR")
                .map(PathBuf::from)
                .or_else(hunt::resolve_hunt_extension_dir);
            let hunt_dir = match source {
                None => {
                    println!("[hunt] \"/hunt for PokeIdle\" not found in any installed browser.");
                    // Drop any earlier copy. Slots also start with extensions turned off, so an
                    // extension WebView2 remembered in a slot's profile stays inactive.
                    let _ = std::fs::remove_dir_all(base.join("extensions"));
                    None
                }
                Some(src) => match hunt::stage_extension(&src, &base.join("extensions")) {
                    Ok(root) => {
                        println!("[hunt] loading extension from {}", src.display());
                        Some(root)
                    }
                    Err(e) => {
                        eprintln!("[hunt] could not copy {}: {e}", src.display());
                        None
                    }
                },
            };
            let slots = config.slots.clone();
            app.manage(Mutex::new(AppState {
                config,
                config_path,
                data_root: base.join("slots"),
                hunt_dir,
                panels: HashMap::new(),
            }));

            let window = WindowBuilder::new(app, "main")
                .title("PokeIdle Multiview")
                .inner_size(1280.0, 800.0)
                .min_inner_size(640.0, 400.0)
                .build()?;
            for slot in &slots {
                create_slot_webview(&handle, slot)?;
            }
            restore_panels(&handle, &slots);
            // Toolbar last: creating a WebView2 pumps window messages, so a toolbar created
            // first would load and read the state before the streams above were restored.
            window.add_child(
                WebviewBuilder::new("toolbar", WebviewUrl::App("index.html".into())),
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(1280.0, TOOLBAR_HEIGHT),
            )?;
            apply_layout(&handle)?;

            let event_handle = handle.clone();
            window.on_window_event(move |event| match event {
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    let _ = apply_layout(&event_handle);
                }
                // Last chance to record which channel each panel ended up on.
                WindowEvent::CloseRequested { .. } => remember_panels(&event_handle),
                _ => {}
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            add_slot,
            close_slot,
            reload_slot,
            toggle_panel,
            stop_panel,
            set_zoom,
            focus_slot
        ])
        .run(tauri::generate_context!())
        .expect("error while running PokeIdle Multiview");
}
