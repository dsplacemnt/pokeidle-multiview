#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod channels;
mod config;
mod external;
mod hunt;
mod kick;
mod twitch;
mod layout;
mod native;
mod power;
mod tray;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use serde::Serialize;
use tauri::webview::{NewWindowResponse, PageLoadEvent, PageLoadPayload, WebviewBuilder};
use tauri::window::WindowBuilder;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Url, WebviewUrl, WindowEvent,
};

use config::{
    Config, KickBonus, Platform, SavedPanel, SlotConfig, KICK_LIVES_PER_SLOT, MAX_PAGES_PER_PLATFORM,
    MAX_SLOTS,
};
use layout::Rect;

/// The toolbar across the top of the window (app-wide buttons).
const BAR_HEIGHT: f64 = 40.0;
/// Each slot's header, on top of its tile: the slot's controls and its stream pills, side by side.
const HEADER_HEIGHT: f64 = 32.0;
const GAME_URL: &str = "https://pokeidle.io/";
const WEBAUTHN_BLOCK: &str = include_str!("../scripts/webauthn-block.js");
const DESKTOP_LAYOUT: &str = include_str!("../scripts/desktop-layout.js");
const LOWEST_QUALITY: &str = include_str!("../scripts/lowest-quality.js");
const KICK_LIGHT: &str = include_str!("../scripts/kick-light.js");
const GAME_WATCH: &str = include_str!("../scripts/game-watch.js");
/// A Kick page running in the background is parked at this size (logical px) instead of its
/// tile's: a small page lays out in Kick's compact layout and its player has fewer pixels to
/// draw. It gets the tile's size back when shown.
const KICK_BACKGROUND_SIZE: (f64, f64) = (480.0, 360.0);

struct AppState {
    config: Config,
    config_path: PathBuf,
    data_root: PathBuf,
    hunt_dir: Option<PathBuf>,
    /// Twitch/Kick pages per slot (their URLs are saved in the config by `remember_panels`).
    panels: HashMap<u8, PanelState>,
    /// Last known live status of the channels in each platform's priority list.
    live: HashMap<Platform, HashMap<String, bool>>,
    /// When each background Twitch stream was last visited (see start_twitch_visits).
    visits: HashMap<String, std::time::Instant>,
    /// Twitch streams showing their full page for a visit right now.
    visiting: std::collections::HashSet<String>,
    /// The game's official partner channels per platform, as its pages last reported them.
    partners: HashMap<Platform, Vec<String>>,
    /// Whether each account (slot, platform) has that platform linked in the game.
    linked: HashMap<(u8, Platform), bool>,
    /// When each account's bonuses from each platform end (PC clock).
    bonus_ends: HashMap<(u8, Platform), BonusEnds>,
    /// What the health check has seen of each stream page, by label.
    health: HashMap<String, HealthTrack>,
    /// Kick Daily Reward tries per slot today: (UTC day, tries).
    daily_tries: HashMap<u8, (i64, u8)>,
    /// The last IdleHub vote status of each reminded account: (status, when it was asked).
    vote: HashMap<u8, (VoteStatus, std::time::Instant)>,
}

#[derive(Serialize, Clone)]
struct StateView {
    slots: Vec<SlotConfig>,
    focus: Option<u8>,
    max_slots: u8,
    hunt_dir: Option<String>,
    panels: HashMap<u8, PanelState>,
    /// The top toolbar is hidden (only while there are slots: with none it always shows).
    toolbar_hidden: bool,
    /// The slot in the top-left corner, whose header shows the button that brings the toolbar
    /// back.
    corner: Option<u8>,
    /// Each account's running PokeIdle bonuses, as the game reports them.
    bonuses: HashMap<u8, SlotBonuses>,
    /// The IdleHub vote status of the reminded accounts, by slot.
    vote: HashMap<u8, VoteStatus>,
}

fn slot_bonuses(state: &AppState) -> HashMap<u8, SlotBonuses> {
    let mut out: HashMap<u8, SlotBonuses> = HashMap::new();
    for ((id, platform), ends) in &state.bonus_ends {
        let entry = out.entry(*id).or_default();
        match platform {
            Platform::Kick => entry.kick = Some(*ends),
            Platform::Twitch => entry.twitch = Some(*ends),
        }
    }
    out
}

fn header_label(id: u8) -> String {
    format!("head-{id}")
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

/// Navigation in a slot page: allowed sites load in the slot; any other web page is opened in
/// the default browser instead, and the slot stays where it is.
fn guard_navigation(url: &Url) -> bool {
    if slot_may_navigate(url) {
        return true;
    }
    eprintln!("[nav] not in slot: {url}");
    external::open_in_browser(url);
    false
}

/// `window.open` / `target=_blank` from a slot page, same rule as `guard_navigation`.
fn guard_popup(url: &Url) -> NewWindowResponse<tauri::Wry> {
    if slot_may_navigate(url) {
        return NewWindowResponse::Allow;
    }
    eprintln!("[popup] not in slot: {url}");
    external::open_in_browser(url);
    NewWindowResponse::Deny
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
        toolbar_hidden: toolbar_hidden(state),
        // The layout always puts the focused slot (or the first one) in the top-left corner.
        corner: state
            .config
            .focus
            .filter(|f| state.config.slots.iter().any(|s| s.id == *f))
            .or_else(|| state.config.slots.first().map(|s| s.id)),
        bonuses: slot_bonuses(state),
        vote: vote_view(state),
    }
}

fn toolbar_hidden(state: &AppState) -> bool {
    state.config.toolbar_hidden && !state.config.slots.is_empty()
}

fn save_and_publish(app: &AppHandle, state: &AppState) {
    if let Err(e) = state.config.save(&state.config_path) {
        eprintln!("[config] could not save: {e}");
    }
    publish(app, state);
    tray::set_tooltip(app, &state.panels);
    power::set_keep_awake(app, state.config.keep_awake && any_pages(state));
}

/// Sends the state to the app's own pages: the toolbar and every slot header.
fn publish(app: &AppHandle, state: &AppState) {
    let view = state_view(state);
    let _ = app.emit_to("toolbar", "state", view.clone());
    for id in 1..=MAX_SLOTS {
        let _ = app.emit_to(header_label(id).as_str(), "state", view.clone());
    }
}

fn any_pages(state: &AppState) -> bool {
    state.panels.values().any(|p| !p.pages.is_empty())
}

fn window_logical_size(app: &AppHandle) -> tauri::Result<LogicalSize<f64>> {
    let window = app.get_window("main").expect("main window exists");
    let scale = window.scale_factor()?;
    Ok(window.inner_size()?.to_logical::<f64>(scale))
}

/// The part of the window below the toolbar (the whole window while it is hidden), in logical
/// pixels. Must not be called with the state locked.
fn game_area(app: &AppHandle) -> tauri::Result<Rect> {
    let size = window_logical_size(app)?;
    let hidden = toolbar_hidden(&app.state::<Mutex<AppState>>().lock().unwrap());
    let top = if hidden { 0.0 } else { BAR_HEIGHT };
    Ok(Rect { x: 0.0, y: top, w: size.width, h: (size.height - top).max(0.0) })
}

/// Splits a slot's tile into its header strip (top) and the rest, where its pages go.
fn split_tile(tile: Rect) -> (Rect, Rect) {
    let header = HEADER_HEIGHT.min(tile.h);
    (
        Rect { h: header, ..tile },
        Rect { y: tile.y + header, h: tile.h - header, ..tile },
    )
}

fn current_layout(app: &AppHandle) -> tauri::Result<layout::Layout> {
    let area = game_area(app)?;
    let config = app.state::<Mutex<AppState>>().lock().unwrap().config.clone();
    let ids: Vec<u8> = config.slots.iter().map(|s| s.id).collect();
    Ok(layout::compute(area, &ids, config.focus, config.grid_split))
}

/// Where each open slot goes for the current window size and layout (None = hidden behind
/// a fullscreen slot).
fn slot_tiles(app: &AppHandle) -> tauri::Result<Vec<(u8, Option<Rect>)>> {
    let ids: Vec<u8> = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        state.config.slots.iter().map(|s| s.id).collect()
    };
    Ok(tiles_for(&ids, current_layout(app)?.tiles))
}

/// Every id in `ids`, with its tile from the layout or None when the layout leaves it out.
fn tiles_for(ids: &[u8], tiles: Vec<(u8, Rect)>) -> Vec<(u8, Option<Rect>)> {
    ids.iter()
        .map(|id| (*id, tiles.iter().find(|(t, _)| t == id).map(|(_, r)| *r)))
        .collect()
}

const SPLIT_V: &str = "split-v";
const SPLIT_H: &str = "split-h";

/// Same size as the tile, but entirely left of the window's client area.
fn parked(r: Rect) -> Rect {
    Rect { x: -(r.w + 100.0), ..r }
}

/// Where one webview goes in a layout.
enum Placement {
    /// Visible in the window at this rect.
    Show(Rect),
    /// Running but parked off-screen (background streams; see `parked`).
    Park(Rect),
    /// Hidden; moved to the rect first if there is one, so it reappears in the right place.
    Hide(Option<Rect>),
}

/// Bumped by every layout, so an animation still running stops when a newer layout arrives.
static LAYOUT_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
const ANIMATION_MS: u64 = 180;
const FRAME_MS: u64 = 15;

/// Places every webview for the current layout at once (dragging a splitter, window resizes,
/// showing a stream).
fn apply_layout(app: &AppHandle) -> tauri::Result<()> {
    run_layout(app, false)
}

/// Same, but slots that move or change size slide there (fullscreen, grid reset, toolbar,
/// adding or closing a slot), unless smooth transitions are off in the settings.
fn apply_layout_smooth(app: &AppHandle) -> tauri::Result<()> {
    let smooth = app.state::<Mutex<AppState>>().lock().unwrap().config.smooth_transitions;
    run_layout(app, smooth)
}

fn layout_plan(app: &AppHandle) -> tauri::Result<Vec<(String, Placement)>> {
    let mut plan = Vec::new();
    let area = game_area(app)?;
    // The toolbar never animates: it shows or hides, and the slots slide into its place.
    plan.push((
        "toolbar".to_string(),
        if area.y > 0.0 {
            Placement::Show(Rect { x: 0.0, y: 0.0, w: area.w, h: area.y })
        } else {
            Placement::Hide(None)
        },
    ));
    let (panels, ids, light_kick) = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        (
            state.panels.clone(),
            state.config.slots.iter().map(|s| s.id).collect::<Vec<u8>>(),
            state.config.light_kick_background,
        )
    };
    let layout = current_layout(app)?;
    for (label, vertical) in [(SPLIT_V, true), (SPLIT_H, false)] {
        let placement = match layout.splitters.iter().find(|(kind, _)| kind.is_vertical() == vertical) {
            Some((_, r)) => Placement::Show(*r),
            None => Placement::Hide(None),
        };
        plan.push((label.to_string(), placement));
    }
    for (id, full_tile) in tiles_for(&ids, layout.tiles) {
        let slot_panels = panels.get(&id);
        // The header sits on top of the tile; the slot's pages get the rest.
        plan.push((
            header_label(id),
            match full_tile {
                Some(t) => Placement::Show(split_tile(t).0),
                None => Placement::Hide(None),
            },
        ));
        let tile = full_tile.map(|t| split_tile(t).1);
        // The shown page (or the game) takes the slot's tile. Everything else in the slot
        // is hidden but keeps running, and keeps the tile's size so pages don't reflow.
        let front_label =
            slot_panels.and_then(|p| p.front.clone()).unwrap_or_else(|| slot_label(id));
        let tile_or_full = tile.unwrap_or_else(|| split_tile(area).1);
        let labels = std::iter::once(slot_label(id))
            .chain(slot_panels.into_iter().flat_map(|p| p.pages.iter().map(|pg| pg.label.clone())));
        for label in labels {
            let is_panel = label != slot_label(id);
            let placement = if label == front_label && tile.is_some() {
                Placement::Show(tile_or_full)
            } else if is_panel {
                // Background streams are parked outside the window instead of hidden: the
                // page still counts as visible, so players start (and keep) playing.
                let kick = slot_panels
                    .and_then(|p| p.page(&label))
                    .is_some_and(|p| p.platform == Platform::Kick);
                let size = if kick && light_kick {
                    let (w, h) = KICK_BACKGROUND_SIZE;
                    Rect { w, h, ..tile_or_full }
                } else {
                    tile_or_full
                };
                Placement::Park(parked(size))
            } else {
                // The game behind a shown panel, or a slot behind a focused one.
                Placement::Hide(tile)
            };
            plan.push((label, placement));
        }
    }
    Ok(plan)
}

fn set_rect(webview: &tauri::Webview, r: Rect) -> tauri::Result<()> {
    webview.set_position(LogicalPosition::new(r.x, r.y))?;
    webview.set_size(LogicalSize::new(r.w, r.h))
}

/// Where a webview is now, in logical pixels.
fn current_rect(webview: &tauri::Webview, scale: f64) -> Option<Rect> {
    let pos = webview.position().ok()?.to_logical::<f64>(scale);
    let size = webview.size().ok()?.to_logical::<f64>(scale);
    Some(Rect { x: pos.x, y: pos.y, w: size.width, h: size.height })
}

fn run_layout(app: &AppHandle, animate: bool) -> tauri::Result<()> {
    let Some(window) = app.get_window("main") else { return Ok(()) };
    let generation = LAYOUT_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let scale = window.scale_factor()?;
    let mut moves = Vec::new();
    for (label, placement) in layout_plan(app)? {
        let Some(webview) = app.get_webview(&label) else { continue };
        match placement {
            Placement::Show(to) => {
                // Slide only webviews that are already in the window somewhere else; anything
                // coming from off-screen or a 1×1 placeholder just appears.
                let from = current_rect(&webview, scale)
                    .filter(|r| animate && r.x >= 0.0 && r.w > 2.0 && r.h > 2.0 && *r != to);
                match from {
                    Some(from) => moves.push((webview.clone(), from, to)),
                    None => set_rect(&webview, to)?,
                }
                webview.show()?;
            }
            Placement::Park(r) => {
                set_rect(&webview, r)?;
                webview.show()?;
            }
            Placement::Hide(r) => {
                if let Some(r) = r {
                    set_rect(&webview, r)?;
                }
                webview.hide()?;
            }
        }
    }
    if !moves.is_empty() {
        std::thread::spawn(move || animate_moves(moves, generation));
    }
    Ok(())
}

/// Moves webviews from `from` to `to` with an ease-out curve; stops early (leaving the newer
/// layout in charge) if another layout starts meanwhile.
fn animate_moves(moves: Vec<(tauri::Webview, Rect, Rect)>, generation: u64) {
    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let start = std::time::Instant::now();
    loop {
        if LAYOUT_GEN.load(Ordering::SeqCst) != generation {
            return;
        }
        let p = (start.elapsed().as_millis() as f64 / ANIMATION_MS as f64).min(1.0);
        let t = 1.0 - (1.0 - p).powi(3);
        for (webview, from, to) in &moves {
            let r = Rect {
                x: lerp(from.x, to.x, t),
                y: lerp(from.y, to.y, t),
                w: lerp(from.w, to.w, t),
                h: lerp(from.h, to.h, t),
            };
            let _ = set_rect(webview, r);
        }
        if p >= 1.0 {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(FRAME_MS));
    }
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
    .on_page_load(|webview, payload| lower_player_quality(&webview, &payload))
    .on_navigation(|url| guard_navigation(url))
    // Popups (e.g. a sign-in window) open with WebView2's default window, which shares the
    // slot's environment and therefore its login. Other sites open in the default browser.
    .on_new_window(|url, _features| guard_popup(&url));

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

/// Light background players drop to their lowest quality once loaded.
fn lower_player_quality(webview: &tauri::Webview, payload: &PageLoadPayload<'_>) {
    if payload.event() == PageLoadEvent::Finished
        && payload.url().host_str() == Some("player.twitch.tv")
    {
        if let Err(e) = webview.eval(LOWEST_QUALITY) {
            eprintln!("[panel] could not lower quality: {e}");
        }
    }
}

/// Where a slot's pages go: its tile below the header. None = the slot is hidden behind a
/// fullscreen one.
fn tile_of(app: &AppHandle, slot_id: u8) -> tauri::Result<Option<Rect>> {
    Ok(slot_tiles(app)?
        .into_iter()
        .find(|(id, _)| *id == slot_id)
        .and_then(|(_, tile)| tile)
        .map(|t| split_tile(t).1))
}

/// Pages of a slot hidden behind a fullscreen one still lay out at full size.
fn hidden_tile_of(app: &AppHandle) -> tauri::Result<Rect> {
    Ok(split_tile(game_area(app)?).1)
}

/// The slot's header: a local page with the slot's controls and its stream pills. It is created
/// after the slot's pages exist, so it reads the right state when it loads.
fn create_slot_header(app: &AppHandle, id: u8) -> tauri::Result<()> {
    if app.get_webview(&header_label(id)).is_some() {
        return Ok(());
    }
    let window = app.get_window("main").expect("main window exists");
    window.add_child(
        WebviewBuilder::new(header_label(id), WebviewUrl::App(format!("slot.html?id={id}").into())),
        LogicalPosition::new(0.0, BAR_HEIGHT),
        LogicalSize::new(1.0, 1.0),
    )?;
    Ok(())
}

fn create_slot_webview(app: &AppHandle, slot: &SlotConfig) -> tauri::Result<()> {
    let window = app.get_window("main").expect("main window exists");
    // The game's handlers replace the session defaults: Twitch/Kick channel links (the game
    // opens its official channels in a new tab) go to the slot's own stream panel instead of a
    // loose popup window.
    let (nav_app, popup_app, title_app, id) = (app.clone(), app.clone(), app.clone(), slot.id);
    let builder = slot_session_builder(app, slot.id, slot_label(slot.id), GAME_URL)
        .initialization_script(DESKTOP_LAYOUT)
        // Listens to the game's own state messages for its Kick/Twitch partner channels and the
        // account's link status; it reports through a brief title change.
        .initialization_script(GAME_WATCH)
        .on_document_title_changed(move |_, title| {
            if let Some(json) = title.strip_prefix("[multiview-game] ") {
                game_reported(&title_app, id, json);
            } else if let Some(json) = title.strip_prefix("[multiview-vote] ") {
                vote_reported(&title_app, id, json);
            }
        })
        .on_navigation(move |url| {
            if let Some(channel) = channels::channel_from_url(url) {
                spawn_route_channel(&nav_app, id, channel);
                return false;
            }
            guard_navigation(url)
        })
        .on_new_window(move |url, _features| {
            if let Some(channel) = channels::channel_from_url(&url) {
                spawn_route_channel(&popup_app, id, channel);
                return NewWindowResponse::Deny;
            }
            guard_popup(&url)
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
            let r = hidden_tile_of(app)?;
            let webview = window.add_child(
                builder,
                LogicalPosition::new(r.x, r.y),
                LogicalSize::new(r.w, r.h),
            )?;
            webview.hide()?;
            webview
        }
    };
    if slot.muted {
        set_muted(&webview, true);
    }
    native::attach(&webview, app.clone());
    Ok(())
}

// ---------------------------------------------------------------- Twitch/Kick pages

impl Platform {
    /// Where a new page opens: the sign-in page on Twitch, the home page on Kick (its
    /// "Log In" button opens a dialog there).
    fn start_url(self) -> &'static str {
        match self {
            Platform::Twitch => "https://www.twitch.tv/login",
            Platform::Kick => "https://kick.com/",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Platform::Twitch => "twitch",
            Platform::Kick => "kick",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Platform::Twitch => "Twitch",
            Platform::Kick => "Kick",
        }
    }
}

/// Numbers page labels, so a page closed and opened again never reuses a webview label.
static NEXT_PAGE: AtomicU32 = AtomicU32::new(1);

/// A Twitch/Kick page running in a slot, in the slot's session. It is either shown in place of
/// the game or running hidden and muted (so its stream keeps counting).
#[derive(Serialize, Clone, Debug)]
struct Page {
    /// Webview label, also how the toolbar names the page.
    label: String,
    platform: Platform,
    /// The channel it is on; None on other pages (login, home).
    login: Option<String>,
    /// A problem the health check found and the app couldn't fix by itself (shown as ⚠ on the
    /// page's pill): stalled, logged out, offline.
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
}

/// Per slot: its Twitch/Kick pages, in the order they were opened, and the one (if any) shown
/// in place of the game.
#[derive(Serialize, Clone, Default, Debug)]
struct PanelState {
    front: Option<String>,
    pages: Vec<Page>,
    /// Per platform, the page that was shown last: the platform button brings it back.
    #[serde(skip)]
    last_shown: HashMap<Platform, String>,
}

impl PanelState {
    fn page(&self, label: &str) -> Option<&Page> {
        self.pages.iter().find(|p| p.label == label)
    }

    fn front_page(&self) -> Option<&Page> {
        self.page(self.front.as_deref()?)
    }

    fn forget(&mut self, label: &str) {
        self.pages.retain(|p| p.label != label);
        self.last_shown.retain(|_, l| l != label);
        if self.front.as_deref() == Some(label) {
            self.front = None;
        }
    }
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

/// Shows page `label` in place of the slot's game, or the game again with None. The page shown
/// before keeps running hidden and muted, on its light player.
fn set_front(app: &AppHandle, id: u8, label: Option<String>) -> tauri::Result<()> {
    let state = app.state::<Mutex<AppState>>();
    let old = state.lock().unwrap().panels.get(&id).and_then(|p| p.front.clone());
    if old != label {
        let light_kick = state.lock().unwrap().config.light_kick_background;
        if let Some(page) = old.as_deref().and_then(|l| app.get_webview(l)) {
            set_muted(&page, true);
            switch_page(&page, false);
            if light_kick {
                set_kick_light(&page, true);
            }
        }
        if let Some(page) = label.as_deref().and_then(|l| app.get_webview(l)) {
            set_muted(&page, false);
            switch_page(&page, true);
            set_kick_light(&page, false);
        }
    }
    {
        let mut state = state.lock().unwrap();
        let panels = state.panels.entry(id).or_default();
        panels.front = label.clone().filter(|l| panels.page(l).is_some());
        if let Some(page) = panels.front_page().cloned() {
            panels.last_shown.insert(page.platform, page.label);
        }
    }
    remember_panels(app);
    apply_layout(app)
}

/// 📺/🟩 button. Its platform shown → back to the game (the page keeps running hidden and
/// muted). Otherwise it shows the platform's page that was shown last (or its first one), and
/// with no page yet it opens the site over the slot, to log in.
fn toggle_panel_inner(app: &AppHandle, id: u8, platform: Platform) -> tauri::Result<()> {
    let (shown, target) = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        let panels = state.panels.get(&id);
        let shown = panels.and_then(|p| p.front_page()).map(|p| p.platform);
        let target = panels.and_then(|p| {
            p.last_shown
                .get(&platform)
                .filter(|l| p.page(l).is_some())
                .cloned()
                .or_else(|| p.pages.iter().find(|pg| pg.platform == platform).map(|pg| pg.label.clone()))
        });
        (shown, target)
    };
    if shown == Some(platform) {
        return set_front(app, id, None);
    }
    let label = match target {
        Some(label) => label,
        None => create_page(app, id, platform, platform.start_url(), false)?,
    };
    set_front(app, id, Some(label))
}

/// A pill: shows that page in place of the game, or (if it is the one shown) goes back to it.
fn show_page_inner(app: &AppHandle, id: u8, label: &str) -> tauri::Result<()> {
    let (exists, shown) = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        let panels = state.panels.get(&id);
        (
            panels.is_some_and(|p| p.page(label).is_some()),
            panels.and_then(|p| p.front.as_deref()) == Some(label),
        )
    };
    match (exists, shown) {
        (false, _) => Ok(()),
        (true, true) => set_front(app, id, None),
        (true, false) => set_front(app, id, Some(label.to_string())),
    }
}

/// A page on a channel moves between the full channel page (shown: chat, channel points) and
/// the light player (hidden: just the video). Pages elsewhere (login, home) stay put.
fn switch_page(panel: &tauri::Webview, shown: bool) {
    let Ok(current) = panel.url() else { return };
    let Some(channel) = channels::channel_from_url(&current) else { return };
    if channels::is_on(&current, &channel, shown) {
        return;
    }
    let target = channel.url_for(shown);
    if let Err(e) = panel.navigate(target.parse().unwrap()) {
        eprintln!("[panel] could not switch to {target}: {e}");
    }
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
            Err(e) => {
                eprintln!("[route] slot {id}: could not open {}: {e}", channel.page_url());
                let _ = app.emit_to(header_label(id).as_str(), "error", e);
            }
        }
    });
}

/// A channel link clicked in the game. Each channel gets its own page, so several streams per
/// platform run in the background at once (up to MAX_PAGES_PER_PLATFORM):
/// - already running in this slot → nothing to do;
/// - first page of the platform → it opens visible, so the player can check the slot is
///   logged in, then sends it to the background;
/// - a page of the platform that isn't on a channel (login, home) is reused;
/// - otherwise a new page starts in the background, muted.
fn route_channel(app: &AppHandle, id: u8, channel: &channels::Channel) -> Result<(), String> {
    let (same_platform, front) = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        let panels = state.panels.get(&id).cloned().unwrap_or_default();
        let same: Vec<Page> =
            panels.pages.into_iter().filter(|p| p.platform == channel.platform).collect();
        (same, panels.front)
    };
    if same_platform.iter().any(|p| p.login.as_deref() == Some(channel.login.as_str())) {
        return Ok(());
    }
    if same_platform.is_empty() {
        let label = create_page(app, id, channel.platform, &channel.page_url(), false)
            .map_err(|e| e.to_string())?;
        return set_front(app, id, Some(label)).map_err(|e| e.to_string());
    }
    // The shown page first, if it is one of them.
    let reuse = same_platform
        .iter()
        .filter(|p| p.login.is_none())
        .max_by_key(|p| front.as_deref() == Some(p.label.as_str()));
    if let Some(page) = reuse {
        let shown = front.as_deref() == Some(page.label.as_str());
        if let Some(webview) = app.get_webview(&page.label) {
            webview.navigate(channel.url_for(shown).parse().unwrap()).map_err(|e| e.to_string())?;
        }
        set_page_login(app, &page.label, Some(channel.login.clone()));
    } else if same_platform.len() >= MAX_PAGES_PER_PLATFORM {
        return Err(format!(
            "Slot {id} already runs {MAX_PAGES_PER_PLATFORM} {} streams; close one first",
            channel.platform.name()
        ));
    } else {
        create_page(app, id, channel.platform, &channel.url_for(false), true)
            .map_err(|e| e.to_string())?;
    }
    remember_panels(app);
    apply_layout(app).map_err(|e| e.to_string())
}

/// Records which channel a page is on; true if it changed.
fn set_page_login(app: &AppHandle, label: &str, login: Option<String>) -> bool {
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    let page = state.panels.values_mut().flat_map(|p| p.pages.iter_mut()).find(|p| p.label == label);
    match page {
        Some(page) if page.login != login => {
            page.login = login;
            true
        }
        _ => false,
    }
}

/// A page finished loading: its pill follows the channel it is on now (the player may have
/// clicked to another channel inside it), and a Kick channel loading in the background gets
/// its light page.
fn page_loaded(app: &AppHandle, webview: &tauri::Webview, url: &Url) {
    let label = webview.label();
    let login = channels::channel_from_url(url).map(|c| c.login);
    if set_page_login(app, label, login) {
        let state = app.state::<Mutex<AppState>>();
        publish(app, &state.lock().unwrap());
    }
    let in_background = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        state.config.light_kick_background
            && !state.panels.values().any(|p| p.front.as_deref() == Some(label))
    };
    if in_background {
        set_kick_light(webview, true);
    }
    twitch_visit_loaded(app, webview, url);
}

/// Turns the light Kick page (scripts/kick-light.js) on or off in `webview`. Does nothing on
/// pages that aren't a Kick channel (Twitch has its own light player, login pages stay as
/// they are).
fn set_kick_light(webview: &tauri::Webview, on: bool) {
    let on_kick_channel = webview
        .url()
        .ok()
        .and_then(|u| channels::channel_from_url(&u))
        .is_some_and(|c| c.platform == Platform::Kick);
    if !on_kick_channel {
        return;
    }
    // Turning it off only matters where it was turned on (the script is then loaded).
    let script = if on {
        format!("{KICK_LIGHT}\nwindow.__multiviewLight(true);")
    } else {
        "window.__multiviewLight && window.__multiviewLight(false);".to_string()
    };
    if let Err(e) = webview.eval(script) {
        eprintln!("[panel] could not set the light Kick page ({on}): {e}");
    }
}

/// Creates a Twitch/Kick page for a slot and returns its label. `background` = hidden and muted
/// from the start (used for extra streams and when streams come back at startup).
fn create_page(
    app: &AppHandle,
    id: u8,
    platform: Platform,
    url: &str,
    background: bool,
) -> tauri::Result<String> {
    let window = app.get_window("main").expect("main window exists");
    let label = format!("slot-{id}-{}-{}", platform.key(), NEXT_PAGE.fetch_add(1, Ordering::SeqCst));
    let (load_app, title_app) = (app.clone(), app.clone());
    let builder = slot_session_builder(app, id, label.clone(), url)
        .on_page_load(move |webview, payload| {
            lower_player_quality(&webview, &payload);
            if payload.event() == PageLoadEvent::Finished {
                page_loaded(&load_app, &webview, payload.url());
            }
        })
        // The app's scripts in the page (scripts/report.js) report by briefly setting the title:
        // "[multiview] …" = what they did (logged), "[multiview-points] {…}" = the account's
        // channel points there. Those pages have no other way to talk.
        .on_document_title_changed(move |webview, title| {
            if let Some(message) = title.strip_prefix("[multiview] ") {
                println!("[auto] {}: {message}", webview.label());
            } else if let Some(json) = title.strip_prefix("[multiview-points] ") {
                points_reported(&title_app, id, json);
            } else if let Some(json) = title.strip_prefix("[multiview-health] ") {
                health_reported(&title_app, webview.label(), json);
            } else if let Some(json) = title.strip_prefix("[multiview-daily] ") {
                daily_reported(&title_app, id, json);
            }
        });
    let mut r = match tile_of(app, id)? {
        Some(r) => r,
        // Slot hidden behind a focused one: use the whole area so the page lays out normally.
        None => hidden_tile_of(app)?,
    };
    if background {
        // Born off-screen (not hidden) so the player autoplays; see apply_layout.
        r = parked(r);
    }
    let page =
        window.add_child(builder, LogicalPosition::new(r.x, r.y), LogicalSize::new(r.w, r.h))?;
    if background {
        set_muted(&page, true);
    }
    native::attach(&page, app.clone());
    let login = url.parse::<Url>().ok().and_then(|u| channels::channel_from_url(&u)).map(|c| c.login);
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    state.panels.entry(id).or_default().pages.push(Page { label: label.clone(), platform, login, warning: None });
    Ok(label)
}

/// Writes every running page and where it is into the config, so the streams come back after
/// a restart. Called on each page change and when the window closes (to catch channels the
/// user switched to inside a page).
fn remember_panels(app: &AppHandle) {
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    let running = state.panels.clone();
    for slot in &mut state.config.slots {
        let pages = running.get(&slot.id).map(|p| p.pages.clone()).unwrap_or_default();
        slot.panels = pages
            .into_iter()
            .map(|page| {
                let url = app
                    .get_webview(&page.label)
                    .and_then(|w| w.url().ok())
                    // Not loaded yet (about:blank): fall back to what we know below.
                    .filter(|u| u.scheme() == "https")
                    // Channels are saved as their full page, whichever player is running.
                    .map(|u| channels::channel_from_url(&u).map_or_else(|| u.to_string(), |c| c.page_url()))
                    .or_else(|| {
                        page.login.clone().map(|login| {
                            channels::Channel { platform: page.platform, login }.page_url()
                        })
                    })
                    .unwrap_or_else(|| page.platform.start_url().to_string());
                SavedPanel { platform: page.platform, url }
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
            let parsed = saved.url.parse::<Url>().ok().filter(slot_may_navigate);
            // A channel comes back on its light player; other pages as they were.
            let url = match parsed.as_ref().and_then(channels::channel_from_url) {
                Some(channel) => channel.url_for(false),
                None if parsed.is_some() => saved.url.clone(),
                None => saved.platform.start_url().to_string(),
            };
            let url = url.as_str();
            match create_page(app, slot.id, saved.platform, url, true) {
                Ok(_) => println!("[panel] slot {}: resumed {}", slot.id, url),
                Err(e) => eprintln!("[panel] slot {}: could not resume {url}: {e}", slot.id),
            }
        }
    }
}

/// Closes pages for good (stops their streams). `labels` = None closes all of the slot's.
fn stop_pages_inner(app: &AppHandle, id: u8, labels: Option<&[String]>) -> tauri::Result<()> {
    let closing: Vec<String> = {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        let panels = state.panels.entry(id).or_default();
        let closing: Vec<String> = panels
            .pages
            .iter()
            .map(|p| p.label.clone())
            .filter(|l| labels.is_none_or(|wanted| wanted.contains(l)))
            .collect();
        for label in &closing {
            panels.forget(label);
        }
        for label in &closing {
            state.health.remove(label);
        }
        closing
    };
    for label in &closing {
        if let Some(page) = app.get_webview(label) {
            if let Err(e) = page.close() {
                eprintln!("[{label}] close failed: {e}");
            }
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
    create_slot_header(&app, slot.id).map_err(|e| e.to_string())?;
    apply_layout_smooth(&app).map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
    Ok(slot.id)
}

#[tauri::command]
async fn close_slot(app: AppHandle, id: u8) -> Result<(), String> {
    // Forget the slot first, so a failed close never leaves it half-open in the config.
    let pages = {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        state.config.slots.retain(|s| s.id != id);
        if state.config.focus == Some(id) {
            state.config.focus = None;
        }
        state.panels.remove(&id).map(|p| p.pages).unwrap_or_default()
    };
    for label in pages.into_iter().map(|p| p.label).chain([slot_label(id), header_label(id)]) {
        if let Some(webview) = app.get_webview(&label) {
            if let Err(e) = webview.close() {
                eprintln!("[{label}] close failed: {e}");
            }
        }
    }
    // Its IdleHub vote window, if open (it uses the slot's session).
    if let Some(vote) = app.get_window(&vote_window_label(id)) {
        let _ = vote.close();
    }
    apply_layout_smooth(&app).map_err(|e| e.to_string())?;
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
async fn show_page(app: AppHandle, id: u8, label: String) -> Result<(), String> {
    show_page_inner(&app, id, &label).map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
    Ok(())
}

/// ⏹ or a pill's ×: closes that page for good (stops its stream).
#[tauri::command]
async fn stop_page(app: AppHandle, id: u8, label: String) -> Result<(), String> {
    stop_pages_inner(&app, id, Some(&[label])).map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
    Ok(())
}

/// The header's "+": a channel typed by name (on `platform`) or pasted as a Twitch/Kick link.
/// It opens like a channel link clicked in the game.
#[tauri::command]
async fn add_channel(app: AppHandle, id: u8, text: String, platform: Platform) -> Result<(), String> {
    let channel = channels::parse_input(&text, platform)
        .ok_or_else(|| format!("\"{}\" is not a Twitch or Kick channel", text.trim()))?;
    route_channel(&app, id, &channel)?;
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
    Ok(())
}

/// Pills dragged into a new order: `labels` lists the slot's pages in the order wanted (pages it
/// leaves out keep their place at the end). The order is saved and comes back at startup.
#[tauri::command]
async fn reorder_pages(app: AppHandle, id: u8, labels: Vec<String>) -> Result<(), String> {
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        let panels = state.panels.entry(id).or_default();
        let mut rest = std::mem::take(&mut panels.pages);
        let mut ordered = Vec::with_capacity(rest.len());
        for label in &labels {
            if let Some(i) = rest.iter().position(|p| &p.label == label) {
                ordered.push(rest.remove(i));
            }
        }
        ordered.extend(rest);
        panels.pages = ordered;
    }
    remember_panels(&app);
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
    Ok(())
}

/// "Close all": stops every Twitch/Kick page of the slot.
#[tauri::command]
async fn stop_all_pages(app: AppHandle, id: u8) -> Result<(), String> {
    stop_pages_inner(&app, id, None).map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
    Ok(())
}

#[tauri::command]
fn reload_slot(app: AppHandle, id: u8) -> Result<(), String> {
    let webview = app.get_webview(&slot_label(id)).ok_or("No such slot")?;
    webview.reload().map_err(|e| e.to_string())
}

/// Shows `id` alone, filling the whole game area (the other slots hide and keep running);
/// the same slot again (or None) goes back to the grid as it was. Used by the slot chips,
/// Grid button and Ctrl+1…Ctrl+4.
fn toggle_focus(app: &AppHandle, id: Option<u8>) {
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        let exists = id.is_some_and(|id| state.config.slots.iter().any(|s| s.id == id));
        state.config.focus = if !exists || state.config.focus == id { None } else { id };
    }
    if let Err(e) = apply_layout_smooth(app) {
        eprintln!("[layout] {e}");
    }
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(app, &state.lock().unwrap());
}

#[tauri::command]
fn focus_slot(app: AppHandle, id: Option<u8>) {
    toggle_focus(&app, id);
}

#[tauri::command]
fn toggle_mute(app: AppHandle, id: u8) -> Result<(), String> {
    let webview = app.get_webview(&slot_label(id)).ok_or("No such slot")?;
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    let slot = state.config.slots.iter_mut().find(|s| s.id == id).ok_or("No such slot")?;
    slot.muted = !slot.muted;
    set_muted(&webview, slot.muted);
    save_and_publish(&app, &state);
    Ok(())
}

#[tauri::command]
fn rename_slot(app: AppHandle, id: u8, name: String) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    let slot = state.config.slots.iter_mut().find(|s| s.id == id).ok_or("No such slot")?;
    slot.name = config::clean_name(&name);
    save_and_publish(&app, &state);
    Ok(())
}

/// A splitter is being dragged; (x, y) is the pointer in screen coordinates (CSS pixels).
/// Moves the boundary live; `drag_split_end` saves it.
#[tauri::command]
fn drag_split(app: AppHandle, vertical: bool, x: f64, y: f64) -> Result<(), String> {
    let window = app.get_window("main").ok_or("No window")?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let origin = window.inner_position().map_err(|e| e.to_string())?.to_logical::<f64>(scale);
    let area = game_area(&app).map_err(|e| e.to_string())?;
    let (px, py) = (x - origin.x, y - origin.y);
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        let config = &mut state.config;
        let kind = if vertical { layout::SplitKind::GridV } else { layout::SplitKind::GridH };
        let fraction = config::clamp_split(layout::fraction_at(area, kind, px, py), 0.5);
        match kind {
            layout::SplitKind::GridV => config.grid_split.v = fraction,
            layout::SplitKind::GridH => config.grid_split.h = fraction,
        }
    }
    apply_layout(&app).map_err(|e| e.to_string())
}

#[tauri::command]
fn drag_split_end(app: AppHandle) {
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
}

/// Hides the top toolbar (the slots take its room) or brings it back. Saved.
#[tauri::command]
fn set_toolbar_hidden(app: AppHandle, hidden: bool) {
    app.state::<Mutex<AppState>>().lock().unwrap().config.toolbar_hidden = hidden;
    if let Err(e) = apply_layout_smooth(&app) {
        eprintln!("[layout] {e}");
    }
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
}

/// Back to the starting layout: leaves fullscreen and puts the splitters in the middle, so
/// every slot gets the same size.
#[tauri::command]
fn reset_layout(app: AppHandle) {
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        state.config.focus = None;
        state.config.grid_split = config::default_grid_split();
    }
    if let Err(e) = apply_layout_smooth(&app) {
        eprintln!("[layout] {e}");
    }
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
}

// ---------------------------------------------------------------- settings window

#[derive(Serialize, serde::Deserialize, Clone)]
struct Settings {
    close_to_tray: bool,
    minimize_to_tray: bool,
    keep_awake: bool,
    auto_reload_minutes: u32,
    smooth_transitions: bool,
    light_kick_background: bool,
    kick_daily: bool,
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, Mutex<AppState>>) -> Settings {
    let config = &state.lock().unwrap().config;
    Settings {
        close_to_tray: config.close_to_tray,
        minimize_to_tray: config.minimize_to_tray,
        keep_awake: config.keep_awake,
        auto_reload_minutes: config.auto_reload_minutes,
        smooth_transitions: config.smooth_transitions,
        light_kick_background: config.light_kick_background,
        kick_daily: config.kick_daily,
    }
}

#[tauri::command]
fn set_settings(app: AppHandle, settings: Settings) {
    let background_kick = {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        let light_kick_changed = state.config.light_kick_background != settings.light_kick_background;
        state.config.close_to_tray = settings.close_to_tray;
        state.config.minimize_to_tray = settings.minimize_to_tray;
        state.config.keep_awake = settings.keep_awake;
        state.config.auto_reload_minutes = settings.auto_reload_minutes;
        state.config.smooth_transitions = settings.smooth_transitions;
        state.config.light_kick_background = settings.light_kick_background;
        state.config.kick_daily = settings.kick_daily;
        state.config = state.config.clone().normalized();
        save_and_publish(&app, &state);
        // The Kick pages running in the background, if the light page was just turned on/off.
        light_kick_changed.then(|| {
            state
                .panels
                .values()
                .flat_map(|p| {
                    p.pages
                        .iter()
                        .filter(|pg| pg.platform == Platform::Kick && p.front.as_deref() != Some(&pg.label))
                        .map(|pg| pg.label.clone())
                })
                .collect::<Vec<String>>()
        })
    };
    // Running streams switch right away: trimmed (or back to the full page) and resized.
    if let Some(labels) = background_kick {
        for label in labels {
            if let Some(page) = app.get_webview(&label) {
                set_kick_light(&page, settings.light_kick_background);
            }
        }
        if let Err(e) = apply_layout(&app) {
            eprintln!("[layout] {e}");
        }
    }
}

/// Where a `w`×`h` window goes to be centered over the main window (logical px); None when the
/// main window is minimized, hidden or can't be read (then the screen's center is used).
fn centered_over(main: &tauri::Window, w: f64, h: f64) -> Option<(f64, f64)> {
    (|| -> tauri::Result<Option<(f64, f64)>> {
        if main.is_minimized()? || !main.is_visible()? {
            return Ok(None);
        }
        let scale = main.scale_factor()?;
        let pos = main.outer_position()?.to_logical::<f64>(scale);
        let size = main.outer_size()?.to_logical::<f64>(scale);
        Ok(Some((pos.x + (size.width - w) / 2.0, pos.y + (size.height - h) / 2.0)))
    })()
    .ok()
    .flatten()
}

// Async: creates a window.
#[tauri::command]
async fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window("settings") {
        let _ = existing.unminimize();
        return existing.set_focus().map_err(|e| e.to_string());
    }
    let main = app.get_window("main").ok_or("No window")?;
    // Wide: general · Kick · Twitch · bonus per account side by side, each column scrolling on
    // its own (the page falls back to two columns if the window is made narrower).
    let (w, h) = (1360.0, 820.0);
    let builder =
        tauri::WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("settings.html".into()))
            .title("Settings — PokeIdle Multiview")
            .inner_size(w, h);
    let builder = match centered_over(&main, w, h) {
        Some((x, y)) => builder.position(x, y),
        None => builder.center(),
    };
    builder
        .resizable(true)
        .min_inner_size(720.0, 520.0)
        .minimizable(false)
        .maximizable(false)
        // Owned by the main window: stays in front of it and closes with it.
        .owner_raw(main.hwnd().map_err(|e| e.to_string())?)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------- Kick/Twitch priority + bonuses

/// Set to run the stream manager now instead of at its next turn (after a settings change).
static AUTO_WAKE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Every 2 minutes: checks which channels of the Kick and Twitch priority lists are live, makes
/// every slot keep the highest live ones open (2 on Kick, which only pays channel points for 2
/// lives at a time per account; the chosen number on Twitch), redeems each account's chosen
/// bonus on Kick, and starts the due Twitch visits (bonus chest + bonus).
fn start_stream_manager(app: AppHandle) {
    std::thread::spawn(move || {
        // Let the restored streams load first.
        std::thread::sleep(std::time::Duration::from_secs(20));
        loop {
            for platform in PLATFORMS {
                manage_platform(&app, platform);
            }
            {
                let state = app.state::<Mutex<AppState>>();
                save_and_publish(&app, &state.lock().unwrap());
            }
            let _ = app.emit_to("settings", "automation", automation_view(&app));
            redeem_kick_bonuses(&app);
            request_kick_points(&app);
            check_stream_health(&app);
            claim_kick_daily(&app);
            check_vote(&app);
            start_twitch_visits(&app);
            let started = std::time::Instant::now();
            while started.elapsed() < kick::CHECK_EVERY && !AUTO_WAKE.swap(false, Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        }
    });
}

const PLATFORMS: [Platform; 2] = [Platform::Kick, Platform::Twitch];

fn is_live(platform: Platform, channel: &str) -> Result<bool, String> {
    match platform {
        Platform::Kick => kick::is_live(channel),
        Platform::Twitch => twitch::is_live(channel),
    }
}

/// One platform's turn: live checks, then each slot's streams of that platform.
fn manage_platform(app: &AppHandle, platform: Platform) {
    let (priority, per_slot, slots, previous, balances) = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        let (priority, per_slot) = match platform {
            Platform::Kick => (state.config.kick_priority.clone(), KICK_LIVES_PER_SLOT),
            Platform::Twitch => (state.config.twitch_priority.clone(), usize::from(state.config.twitch_lives_per_slot)),
        };
        let previous = state.live.get(&platform).cloned().unwrap_or_default();
        (priority, per_slot, state.config.slots.clone(), previous, state.config.balances.clone())
    };
    if priority.is_empty() {
        return;
    }
    // A channel whose check fails keeps its last known status; with none known, the slots are
    // left as they are this time rather than closing streams on a network hiccup.
    let mut live = HashMap::new();
    let mut unknown = false;
    for channel in &priority {
        match is_live(platform, channel) {
            Ok(is_live) => {
                live.insert(channel.clone(), is_live);
            }
            Err(e) => {
                eprintln!("[auto] could not check {} {channel}: {e}", platform.name());
                match previous.get(channel) {
                    Some(v) => {
                        live.insert(channel.clone(), *v);
                    }
                    None => unknown = true,
                }
            }
        }
    }
    {
        let state = app.state::<Mutex<AppState>>();
        state.lock().unwrap().live.insert(platform, live.clone());
    }
    if unknown {
        return;
    }
    println!("[auto] {} live in priority order: {:?}", platform.name(), kick::pick(&priority, &live, usize::MAX));
    // Each account picks among the live channels by its own points (closest to its next reward
    // first), the priority order breaking ties.
    for slot in &slots {
        let open = open_channels(app, slot.id, platform);
        let points = |c: &str| balances.get(&config::balance_key(slot.id, platform, c)).copied().unwrap_or(0);
        let wanted = kick::pick_for_account(&priority, &live, per_slot, points, &open);
        if let Err(e) = manage_slot_streams(app, slot.id, platform, &wanted) {
            eprintln!("[auto] slot {}: {e}", slot.id);
        }
    }
}

/// The channels of `platform` slot `id` has open now.
fn open_channels(app: &AppHandle, id: u8, platform: Platform) -> Vec<String> {
    let state = app.state::<Mutex<AppState>>();
    let state = state.lock().unwrap();
    state
        .panels
        .get(&id)
        .map(|p| p.pages.iter().filter(|pg| pg.platform == platform).filter_map(|pg| pg.login.clone()).collect())
        .unwrap_or_default()
}

#[derive(serde::Deserialize)]
struct PointsReport {
    platform: Platform,
    channel: String,
    points: u64,
}

/// A Kick page or a Twitch visit told this account's points on its channel: remembered (and
/// saved) for picking the account's lives.
fn points_reported(app: &AppHandle, id: u8, json: &str) {
    let Ok(report) = serde_json::from_str::<PointsReport>(json) else { return };
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    let key = config::balance_key(id, report.platform, &report.channel.to_ascii_lowercase());
    if state.config.balances.insert(key, report.points) != Some(report.points) {
        if let Err(e) = state.config.save(&state.config_path) {
            eprintln!("[config] could not save: {e}");
        }
    }
}

/// Asks every live Kick page (all accounts) for its channel points; they answer through
/// `points_reported`, and the next check uses them.
const GAME_VOTE: &str = include_str!("../scripts/game-vote.js");

/// An account's IdleHub vote status, as its game reports it.
#[derive(serde::Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct VoteStatus {
    #[serde(default)]
    votes: u64,
    #[serde(default)]
    diamonds: u64,
    /// Votes per diamond, and how many of the current pair are in.
    #[serde(default)]
    per_diamond: u64,
    #[serde(default)]
    in_pair: u64,
    /// When the next vote counts again (PC clock, ms); in the past = it's time to vote.
    #[serde(default, skip_deserializing)]
    ready_at: i64,
    #[serde(default)]
    wait_ms: i64,
    #[serde(default)]
    nick: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    error: Option<String>,
}

/// How often a reminded account's vote status is asked: rarely while the vote is far off,
/// often while it's due (so the header updates soon after the player votes).
const VOTE_CHECK_WAITING: std::time::Duration = std::time::Duration::from_secs(30 * 60);
const VOTE_CHECK_READY: std::time::Duration = std::time::Duration::from_secs(4 * 60);

fn vote_window_label(id: u8) -> String {
    format!("vote-{id}")
}

/// Asks each reminded account's game page for its IdleHub vote status when it's due
/// (scripts/game-vote.js); the answers come to `vote_reported`.
fn check_vote(app: &AppHandle) {
    let due: Vec<u8> = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        state
            .config
            .vote_slots
            .iter()
            .copied()
            .filter(|id| match state.vote.get(id) {
                Some((status, at)) => {
                    let ready = status.ready_at <= now_ms();
                    at.elapsed() >= if ready { VOTE_CHECK_READY } else { VOTE_CHECK_WAITING }
                        || (!ready && status.ready_at <= now_ms() + 60_000)
                }
                None => true,
            })
            .collect()
    };
    let script = format!("{}\n{GAME_VOTE}", kick::REPORT);
    for id in due {
        if let Some(game) = app.get_webview(&slot_label(id)) {
            let _ = game.eval(&script);
        }
    }
}

/// A game page answered with its account's vote status: remembered, and shown in its header
/// and in ⚙.
fn vote_reported(app: &AppHandle, id: u8, json: &str) {
    let Ok(mut status) = serde_json::from_str::<VoteStatus>(json) else { return };
    status.ready_at = now_ms() + status.wait_ms;
    println!(
        "[vote] slot {id}: {} vote(s), {} diamond(s), next in {} min{}",
        status.votes,
        status.diamonds,
        status.wait_ms / 60_000,
        status.error.as_deref().map(|e| format!(" ({e})")).unwrap_or_default()
    );
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    state.vote.insert(id, (status, std::time::Instant::now()));
    publish(app, &state);
    drop(state);
    let _ = app.emit_to("settings", "automation", automation_view(app));
}

/// The vote status of the reminded accounts, by slot.
fn vote_view(state: &AppState) -> HashMap<u8, VoteStatus> {
    state
        .config
        .vote_slots
        .iter()
        .filter_map(|id| Some((*id, state.vote.get(id)?.0.clone())))
        .collect()
}

/// "Remind me to vote" in ⚙: whether account `id` is reminded.
#[tauri::command]
fn set_vote_slot(app: AppHandle, id: u8, on: bool) -> AutomationView {
    automation_changed(&app, |state| {
        state.config.vote_slots.retain(|v| *v != id);
        if on && state.config.slots.iter().any(|s| s.id == id) {
            state.config.vote_slots.push(id);
            state.config.vote_slots.sort_unstable();
        }
        state.vote.remove(&id);
    })
}

fn is_idlehub(host: &str) -> bool {
    host == "idlehub.io" || host.ends_with(".idlehub.io")
}

/// The vote chip: opens account `id`'s IdleHub vote page in a window of its own that uses the
/// slot's browser session, so each account keeps its own IdleHub sign-in and votes with its
/// own nick. The player signs in and confirms there; the app never touches the page. When the
/// window closes, the account's vote status is asked again.
#[tauri::command]
async fn open_vote(app: AppHandle, id: u8) -> Result<(), String> {
    let label = vote_window_label(id);
    if let Some(existing) = app.get_window(&label) {
        let _ = existing.unminimize();
        return existing.set_focus().map_err(|e| e.to_string());
    }
    let (url, nick) = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        let status = state.vote.get(&id).map(|(s, _)| s.clone()).ok_or("No vote status yet")?;
        (status.url, status.nick)
    };
    let parsed: Url = url.parse().map_err(|_| "Bad vote link")?;
    if parsed.scheme() != "https" || !parsed.host_str().is_some_and(is_idlehub) {
        return Err("The vote link isn't an idlehub.io page".into());
    }
    let main = app.get_window("main").ok_or("No window")?;
    let (w, h) = (560.0, 820.0);
    let builder = WindowBuilder::new(&app, &label)
        .title(format!("Vote on IdleHub — {nick}"))
        .inner_size(w, h);
    // Centered over the main window, like the settings.
    let builder = match centered_over(&main, w, h) {
        Some((x, y)) => builder.position(x, y),
        None => builder.center(),
    };
    let window = builder
        .min_inner_size(400.0, 500.0)
        .owner_raw(main.hwnd().map_err(|e| e.to_string())?)
        .build()
        .map_err(|e| e.to_string())?;
    // The slot's session (cookies, sign-ins). IdleHub and its sign-ins (Google, Discord) open
    // here; anything else goes to the default browser.
    let builder = slot_session_builder(&app, id, format!("{label}-page"), parsed.as_str())
        .on_navigation(|url| {
            let in_window = url.scheme() == "https" && url.host_str().is_some_and(is_idlehub);
            in_window || guard_navigation(url)
        })
        .on_new_window(|url, _features| {
            if url.scheme() == "https" && url.host_str().is_some_and(is_idlehub) {
                return NewWindowResponse::Allow;
            }
            guard_popup(&url)
        });
    let page = window
        .add_child(builder, LogicalPosition::new(0.0, 0.0), LogicalSize::new(w, h))
        .map_err(|e| e.to_string())?;
    native::attach(&page, app.clone());
    let event_app = app.clone();
    let event_window = window.clone();
    window.on_window_event(move |event| match event {
        // The page fills the window.
        WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
            if let (Ok(size), Ok(scale)) = (event_window.inner_size(), event_window.scale_factor()) {
                let size = size.to_logical::<f64>(scale);
                let _ = page.set_size(LogicalSize::new(size.width, size.height));
            }
        }
        // Closed: ask the game for the account's status again at the next check.
        WindowEvent::Destroyed => {
            let state = event_app.state::<Mutex<AppState>>();
            state.lock().unwrap().vote.remove(&id);
            AUTO_WAKE.store(true, Ordering::SeqCst);
        }
        _ => {}
    });
    Ok(())
}

const KICK_DAILY: &str = include_str!("../scripts/kick-daily.js");
/// Tries per account per day to claim Kick's Daily Reward before giving up until tomorrow.
const DAILY_TRIES: u8 = 3;

/// Days since 1970 in UTC: Kick's daily rewards reset at 00:00 UTC.
fn utc_day() -> i64 {
    now_ms().div_euclid(86_400_000)
}

/// With "Claim Kick's Daily Reward" on: every account that hasn't claimed it today gets one try
/// per check, in one of its live background Kick pages (scripts/kick-daily.js), up to
/// DAILY_TRIES a day. The answer comes to `daily_reported`.
fn claim_kick_daily(app: &AppHandle) {
    let today = utc_day();
    let due: Vec<u8> = {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        if !state.config.kick_daily {
            return;
        }
        state.daily_tries.retain(|_, (day, _)| *day == today);
        let ids: Vec<u8> = state.config.slots.iter().map(|s| s.id).collect();
        ids.into_iter()
            .filter(|id| state.config.kick_daily_done.get(id) != Some(&today))
            .filter(|id| state.daily_tries.get(id).is_none_or(|(_, n)| *n < DAILY_TRIES))
            .collect()
    };
    if due.is_empty() {
        return;
    }
    let pages = background_channel_pages(app, Platform::Kick, true);
    let script = format!("{}\n{KICK_DAILY}\nwindow.__multiviewDaily();", kick::REPORT);
    for id in due {
        let Some((_, label, _, _)) = pages.iter().find(|p| p.0 == id) else { continue };
        let Some(page) = app.get_webview(label) else { continue };
        {
            let state = app.state::<Mutex<AppState>>();
            let mut state = state.lock().unwrap();
            let tries = state.daily_tries.entry(id).or_insert((today, 0));
            tries.1 += 1;
        }
        println!("[daily] slot {id}: claiming Kick's Daily Reward in {label}");
        let _ = page.eval(&script);
    }
}

#[derive(serde::Deserialize)]
struct DailyReport {
    result: String,
    #[serde(default)]
    detail: String,
}

/// A Kick page answered the Daily Reward try: claimed (or already claimed today) marks the
/// account done until tomorrow; anything else is retried on a later check.
fn daily_reported(app: &AppHandle, id: u8, json: &str) {
    let Ok(report) = serde_json::from_str::<DailyReport>(json) else { return };
    println!("[daily] slot {id}: {} {}", report.result, report.detail);
    if report.result == "claimed" || report.result == "not available" {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        state.config.kick_daily_done.insert(id, utc_day());
        if let Err(e) = state.config.save(&state.config_path) {
            eprintln!("[config] could not save: {e}");
        }
    }
}

const STREAM_HEALTH: &str = include_str!("../scripts/stream-health.js");
/// A stalled stream that a reload didn't fix is reloaded again at most this often.
const STALLED_RETRY: std::time::Duration = std::time::Duration::from_secs(10 * 60);

/// What the health check has seen of one page (see `health_reported`).
#[derive(Default)]
struct HealthTrack {
    /// Checks in a row that found the video not moving.
    stalls: u8,
    /// Reloads since the stream last played normally.
    reloads: u8,
    last_reload: Option<std::time::Instant>,
    /// The current problems, each cleared by its own signal: a playing video clears
    /// `stalled` and `offline`; only seeing the channel points again clears `logged_out`
    /// (Twitch's light player can't tell; its visits can).
    stalled: Option<String>,
    offline: Option<String>,
    logged_out: Option<String>,
}

/// Asks every background stream page whether it is playing (scripts/stream-health.js); the
/// answers come to `health_reported`. Pages in the middle of a Twitch visit are skipped.
fn check_stream_health(app: &AppHandle) {
    let script = format!("{}\n{STREAM_HEALTH}", kick::REPORT);
    let visiting = app.state::<Mutex<AppState>>().lock().unwrap().visiting.clone();
    for platform in PLATFORMS {
        for (_, label, _, _) in background_channel_pages(app, platform, true) {
            if visiting.contains(&label) {
                continue;
            }
            if let Some(page) = app.get_webview(&label) {
                let _ = page.eval(&script);
            }
        }
    }
}

#[derive(serde::Deserialize)]
struct HealthReport {
    state: String,
    #[serde(default)]
    detail: String,
}

/// A stream page answered the health check. A video that stopped moving in two checks in a row
/// gets its page reloaded; if that doesn't fix it, its pill shows ⚠ and it is reloaded again
/// every 10 minutes. Logged-out and offline pages get ⚠ (they need the player, or the priority
/// lists close them). A stream playing again clears it all.
/// What to do with one health answer: whether to reload the page, and the warning to show
/// (`None` inside = clear it). `None` overall = nothing changes this time.
fn next_health(
    track: &mut HealthTrack,
    state: &str,
    detail: &str,
    now: std::time::Instant,
) -> Option<(bool, Option<String>)> {
    let mut reload = false;
    match state {
        "ok" => {
            track.stalls = 0;
            track.reloads = 0;
            track.last_reload = None;
            track.stalled = None;
            track.offline = None;
        }
        "logged-in" => track.logged_out = None,
        "logged-out" => track.logged_out = Some(format!("Logged out: {detail}")),
        "offline" => track.offline = Some(format!("Offline: {detail}")),
        "stalled" => {
            track.stalls += 1;
            if track.stalls < 2 {
                return None;
            }
            track.stalls = 0;
            let due = track.last_reload.is_none_or(|t| now.duration_since(t) >= STALLED_RETRY);
            reload = track.reloads == 0 || due;
            if reload {
                track.reloads = track.reloads.saturating_add(1);
                track.last_reload = Some(now);
            }
            // The first reload is silent; if the stream still stalls after it, it's shown.
            if track.reloads > 1 || !reload {
                track.stalled = Some(format!("Stalled: {detail}. Reloaded, still not playing."));
            }
        }
        // "first" (a page just loaded) and anything unknown change nothing.
        _ => return None,
    }
    let warning = track.logged_out.clone().or_else(|| track.offline.clone()).or_else(|| track.stalled.clone());
    Some((reload, warning))
}

fn health_reported(app: &AppHandle, label: &str, json: &str) {
    let Ok(report) = serde_json::from_str::<HealthReport>(json) else { return };
    let (reload, changed) = {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        let track = state.health.entry(label.to_string()).or_default();
        let Some((reload, warning)) =
            next_health(track, &report.state, &report.detail, std::time::Instant::now())
        else {
            return;
        };
        let page = state.panels.values_mut().flat_map(|p| p.pages.iter_mut()).find(|p| p.label == label);
        let changed = match page {
            Some(page) if page.warning != warning => {
                page.warning = warning;
                true
            }
            _ => false,
        };
        (reload, changed)
    };
    if reload {
        println!("[health] {label}: {} ({}); reloading", report.state, report.detail);
        if let Some(page) = app.get_webview(label) {
            let _ = page.reload();
        }
    }
    if changed {
        let state = app.state::<Mutex<AppState>>();
        publish(app, &state.lock().unwrap());
    }
}

#[cfg(test)]
mod health_tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn a_stall_reloads_once_quietly_then_warns_and_retries_every_10_minutes() {
        let mut t = HealthTrack::default();
        let start = Instant::now();
        // One stalled check is not enough.
        assert_eq!(next_health(&mut t, "stalled", "x", start), None);
        // Two in a row: reload, no warning yet.
        assert_eq!(next_health(&mut t, "stalled", "x", start), Some((true, None)));
        // The reloaded page's first check changes nothing (the reload is remembered).
        assert_eq!(next_health(&mut t, "first", "", start), None);
        // Still stalled: warning, no reload before 10 minutes.
        next_health(&mut t, "stalled", "x", start + Duration::from_secs(120));
        let (reload, warning) = next_health(&mut t, "stalled", "x", start + Duration::from_secs(240)).unwrap();
        assert!(!reload && warning.is_some());
        // After 10 minutes: reloads again, warning kept.
        next_health(&mut t, "stalled", "x", start + Duration::from_secs(660));
        let (reload, warning) = next_health(&mut t, "stalled", "x", start + Duration::from_secs(780)).unwrap();
        assert!(reload && warning.is_some());
        // Playing again clears everything.
        assert_eq!(next_health(&mut t, "ok", "", start + Duration::from_secs(900)), Some((false, None)));
        assert_eq!(t.reloads, 0);
    }

    #[test]
    fn logged_out_and_offline_warn_without_reloading() {
        let mut t = HealthTrack::default();
        let now = Instant::now();
        assert!(matches!(next_health(&mut t, "logged-out", "y", now), Some((false, Some(w))) if w.starts_with("Logged out")));
        // A playing video doesn't clear "logged out" (Twitch's light player can't see logins)...
        assert!(matches!(next_health(&mut t, "ok", "", now), Some((false, Some(w))) if w.starts_with("Logged out")));
        // ...seeing the points again does.
        assert_eq!(next_health(&mut t, "logged-in", "", now), Some((false, None)));
        assert!(matches!(next_health(&mut t, "offline", "z", now), Some((false, Some(w))) if w.starts_with("Offline")));
        assert_eq!(next_health(&mut t, "ok", "", now), Some((false, None)));
    }
}

fn request_kick_points(app: &AppHandle) {
    let script = kick::points_script();
    for (_, label, _, _) in background_channel_pages(app, Platform::Kick, true) {
        if let Some(page) = app.get_webview(&label) {
            let _ = page.eval(&script);
        }
    }
}

/// Makes slot `id` run exactly the `wanted` channels of `platform` in the background: the others
/// are closed (except the page being shown, which is never touched), the missing ones are
/// opened. Pages of that platform that aren't on a channel (login, home) are left alone.
fn manage_slot_streams(app: &AppHandle, id: u8, platform: Platform, wanted: &[String]) -> tauri::Result<()> {
    let (pages, front) = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        let panels = state.panels.get(&id).cloned().unwrap_or_default();
        let pages: Vec<Page> = panels.pages.into_iter().filter(|p| p.platform == platform).collect();
        (pages, panels.front)
    };
    let mut kept: Vec<String> = Vec::new();
    let mut close: Vec<String> = Vec::new();
    for page in &pages {
        let Some(login) = &page.login else { continue };
        let shown = front.as_deref() == Some(page.label.as_str());
        if shown || (wanted.contains(login) && !kept.contains(login)) {
            kept.push(login.clone());
        } else {
            close.push(page.label.clone());
        }
    }
    let missing: Vec<&String> = wanted.iter().filter(|l| !kept.contains(l)).collect();
    if close.is_empty() && missing.is_empty() {
        return Ok(());
    }
    if !close.is_empty() {
        println!("[auto] slot {id}: closing {} {} page(s) outside the priority picks", close.len(), platform.name());
        stop_pages_inner(app, id, Some(&close))?;
    }
    for login in missing {
        let channel = channels::Channel { platform, login: login.clone() };
        println!("[auto] slot {id}: opening {} {login}", platform.name());
        create_page(app, id, platform, &channel.url_for(false), true)?;
    }
    remember_panels(app);
    apply_layout(app)
}

/// Background pages of `platform` on a channel, per account with a bonus chosen (or every
/// account with `all`), leaving out the page being shown: (slot, label, login, bonus).
fn background_channel_pages(app: &AppHandle, platform: Platform, all: bool) -> Vec<(u8, String, String, Option<KickBonus>)> {
    let state = app.state::<Mutex<AppState>>();
    let state = state.lock().unwrap();
    let mut out = Vec::new();
    for slot in &state.config.slots {
        if !all && slot.bonus.is_none() {
            continue;
        }
        let Some(panels) = state.panels.get(&slot.id) else { continue };
        for page in &panels.pages {
            let Some(login) = &page.login else { continue };
            if page.platform == platform && panels.front.as_deref() != Some(page.label.as_str()) {
                out.push((slot.id, page.label.clone(), login.clone(), slot.bonus));
            }
        }
    }
    out
}

/// Asks each live Kick page of an account with a bonus chosen to redeem it if the points are
/// enough (scripts/kick-redeem.js does nothing otherwise; Kick pages show their points even in
/// the background).
fn redeem_kick_bonuses(app: &AppHandle) {
    for (_, label, _, bonus) in background_channel_pages(app, Platform::Kick, false) {
        let (Some(page), Some(bonus)) = (app.get_webview(&label), bonus) else { continue };
        if let Err(e) = page.eval(kick::redeem_script(bonus)) {
            eprintln!("[auto] {label}: could not run the redeem script: {e}");
        }
    }
}

/// Background Twitch streams run the embed player, which has no channel points, so every
/// ~15 minutes each one is "visited": its full channel page loads for ~30 s, where
/// scripts/twitch-visit.js claims the bonus chest and redeems the account's bonus, then it goes
/// back to the light player. A few visits start per check, the longest-waiting first, so they
/// are spread out. Runs while the Twitch list is in use or the account has a bonus chosen.
fn start_twitch_visits(app: &AppHandle) {
    let twitch_list = !app.state::<Mutex<AppState>>().lock().unwrap().config.twitch_priority.is_empty();
    let pages = background_channel_pages(app, Platform::Twitch, twitch_list);
    let now = std::time::Instant::now();
    let due: Vec<(String, String)> = {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        let labels: Vec<String> = pages.iter().map(|p| p.1.clone()).collect();
        state.visits.retain(|label, _| labels.contains(label));
        let mut due: Vec<(std::time::Instant, String, String)> = Vec::new();
        for (i, (_, label, login, _)) in pages.iter().enumerate() {
            if state.visiting.contains(label) {
                continue;
            }
            // A stream seen for the first time gets a spread-out first visit, not all at once.
            let last = *state.visits.entry(label.clone()).or_insert_with(|| {
                let spread = twitch::VISIT_EVERY.as_secs() * (i as u64 % 8) / 8;
                now.checked_sub(twitch::VISIT_EVERY - std::time::Duration::from_secs(spread)).unwrap_or(now)
            });
            if now.duration_since(last) >= twitch::VISIT_EVERY {
                due.push((last, label.clone(), login.clone()));
            }
        }
        due.sort_by_key(|d| d.0);
        due.into_iter()
            .take(twitch::VISITS_PER_CHECK)
            .map(|(_, label, login)| {
                state.visiting.insert(label.clone());
                state.visits.insert(label.clone(), now);
                (label, login)
            })
            .collect()
    };
    for (label, login) in due {
        let Some(page) = app.get_webview(&label) else { continue };
        let channel = channels::Channel { platform: Platform::Twitch, login: login.clone() };
        println!("[auto] visiting Twitch {login} ({label})");
        if let Err(e) = page.navigate(channel.page_url().parse().unwrap()) {
            eprintln!("[auto] {label}: could not open the full page: {e}");
            app.state::<Mutex<AppState>>().lock().unwrap().visiting.remove(&label);
            continue;
        }
        // Back to the light player after the visit, unless the stream was shown meanwhile.
        let back_app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(twitch::VISIT_LENGTH);
            let shown = {
                let state = back_app.state::<Mutex<AppState>>();
                let mut state = state.lock().unwrap();
                state.visiting.remove(&label);
                state.panels.values().any(|p| p.front.as_deref() == Some(label.as_str()))
            };
            if let (false, Some(page)) = (shown, back_app.get_webview(&label)) {
                switch_page(&page, false);
            }
        });
    }
}

/// A visited Twitch page finished loading its full page: run the visit script with the
/// account's bonus.
fn twitch_visit_loaded(app: &AppHandle, webview: &tauri::Webview, url: &Url) {
    let bonus = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        if !state.visiting.contains(webview.label()) {
            return;
        }
        let slot = state.panels.iter().find(|(_, p)| p.page(webview.label()).is_some()).map(|(id, _)| *id);
        slot.and_then(|id| state.config.slots.iter().find(|s| s.id == id)).and_then(|s| s.bonus)
    };
    let on_full_page = url.host_str().is_some_and(|h| h.ends_with("twitch.tv") && h != "player.twitch.tv")
        && channels::channel_from_url(url).is_some();
    if on_full_page {
        if let Err(e) = webview.eval(twitch::visit_script(bonus)) {
            eprintln!("[auto] {}: could not run the visit script: {e}", webview.label());
        }
    }
}

#[derive(Serialize, Clone)]
struct BonusSlotView {
    id: u8,
    name: String,
    bonus: Option<KickBonus>,
    /// Whether the account has Kick / Twitch linked in the game (None = not reported yet).
    kick_linked: Option<bool>,
    twitch_linked: Option<bool>,
    /// Last seen channel points per channel, most first: [platform, channel, points].
    points: Vec<(Platform, String, u64)>,
    /// The account's running bonuses per platform.
    running: SlotBonuses,
}

/// Slot `id`'s last seen balances, most first.
fn slot_points(config: &Config, id: u8) -> Vec<(Platform, String, u64)> {
    let mut out: Vec<(Platform, String, u64)> = config
        .balances
        .iter()
        .filter_map(|(key, points)| {
            let mut parts = key.splitn(3, ':');
            let slot: u8 = parts.next()?.parse().ok()?;
            let platform = match parts.next()? {
                "kick" => Platform::Kick,
                "twitch" => Platform::Twitch,
                _ => return None,
            };
            let channel = parts.next()?.to_string();
            (slot == id).then_some((platform, channel, *points))
        })
        .collect();
    out.sort_by(|a, b| b.2.cmp(&a.2).then(a.1.cmp(&b.1)));
    out
}

#[derive(Serialize, Clone)]
struct PriorityView {
    priority: Vec<String>,
    live: HashMap<String, bool>,
    lives_per_slot: usize,
    /// "Keep updated from the game" is on.
    sync: bool,
    /// How many partner channels the game has announced (0 = not reported yet).
    partners: usize,
    /// How many accounts have each channel open right now.
    watching: HashMap<String, usize>,
}

/// What the settings window shows in its Kick, Twitch and bonus sections.
#[derive(Serialize, Clone)]
struct AutomationView {
    kick: PriorityView,
    twitch: PriorityView,
    slots: Vec<BonusSlotView>,
    /// Streams with a ⚠ (stalled, logged out, offline): [slot name, channel, warning].
    problems: Vec<(String, String, String)>,
    /// The account reminded to vote on IdleHub, and its status.
    vote_slots: Vec<u8>,
    vote: HashMap<u8, VoteStatus>,
}

fn automation_view(app: &AppHandle) -> AutomationView {
    let state = app.state::<Mutex<AppState>>();
    let state = state.lock().unwrap();
    let live = |p: Platform| state.live.get(&p).cloned().unwrap_or_default();
    let partners = |p: Platform| state.partners.get(&p).map_or(0, Vec::len);
    let watching = |p: Platform| {
        let mut count: HashMap<String, usize> = HashMap::new();
        for panels in state.panels.values() {
            let mut seen: Vec<&String> = Vec::new();
            for page in panels.pages.iter().filter(|pg| pg.platform == p) {
                if let Some(login) = page.login.as_ref().filter(|l| !seen.contains(l)) {
                    seen.push(login);
                    *count.entry(login.clone()).or_default() += 1;
                }
            }
        }
        count
    };
    let slot_name = |id: u8| {
        state.config.slots.iter().find(|s| s.id == id).and_then(|s| s.name.clone()).unwrap_or_else(|| format!("Slot {id}"))
    };
    let mut problems: Vec<(String, String, String)> = state
        .panels
        .iter()
        .flat_map(|(id, p)| {
            p.pages.iter().filter_map(move |pg| {
                let warning = pg.warning.clone()?;
                Some((*id, format!("{} {}", pg.platform.name(), pg.login.clone().unwrap_or_default()), warning))
            })
        })
        .map(|(id, channel, warning)| (slot_name(id), channel, warning))
        .collect();
    problems.sort();
    AutomationView {
        problems,
        vote_slots: state.config.vote_slots.clone(),
        vote: vote_view(&state),
        kick: PriorityView {
            priority: state.config.kick_priority.clone(),
            live: live(Platform::Kick),
            lives_per_slot: KICK_LIVES_PER_SLOT,
            sync: state.config.kick_sync,
            partners: partners(Platform::Kick),
            watching: watching(Platform::Kick),
        },
        twitch: PriorityView {
            priority: state.config.twitch_priority.clone(),
            live: live(Platform::Twitch),
            lives_per_slot: usize::from(state.config.twitch_lives_per_slot),
            sync: state.config.twitch_sync,
            partners: partners(Platform::Twitch),
            watching: watching(Platform::Twitch),
        },
        slots: state
            .config
            .slots
            .iter()
            .map(|s| BonusSlotView {
                id: s.id,
                name: s.name.clone().unwrap_or_else(|| format!("Slot {}", s.id)),
                bonus: s.bonus,
                kick_linked: state.linked.get(&(s.id, Platform::Kick)).copied(),
                twitch_linked: state.linked.get(&(s.id, Platform::Twitch)).copied(),
                points: slot_points(&state.config, s.id),
                running: SlotBonuses {
                    kick: state.bonus_ends.get(&(s.id, Platform::Kick)).copied(),
                    twitch: state.bonus_ends.get(&(s.id, Platform::Twitch)).copied(),
                },
            })
            .collect(),
    }
}

/// Saves a settings change and runs the manager right away.
fn automation_changed(app: &AppHandle, change: impl FnOnce(&mut AppState)) -> AutomationView {
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        change(&mut state);
        save_and_publish(app, &state);
    }
    AUTO_WAKE.store(true, Ordering::SeqCst);
    automation_view(app)
}

#[tauri::command]
fn get_automation(app: AppHandle) -> AutomationView {
    automation_view(&app)
}

/// A priority list from the settings window, in order (names or links). Empty = that
/// platform's streams are managed by hand again.
#[tauri::command]
fn set_priority(app: AppHandle, platform: Platform, priority: Vec<String>) -> AutomationView {
    automation_changed(&app, |state| {
        let cleaned = match platform {
            Platform::Kick => config::clean_kick_priority(&priority),
            Platform::Twitch => config::clean_twitch_priority(&priority),
        };
        if let Some(live) = state.live.get_mut(&platform) {
            live.retain(|c, _| cleaned.contains(c));
        }
        match platform {
            Platform::Kick => state.config.kick_priority = cleaned,
            Platform::Twitch => state.config.twitch_priority = cleaned,
        }
    })
}

/// What a game page reports about one platform (scripts/game-watch.js): the game's official
/// partner channels in its order, and whether this account has the platform linked.
#[derive(serde::Deserialize)]
struct GameReport {
    platform: Platform,
    #[serde(default)]
    channels: Vec<String>,
    #[serde(default)]
    linked: bool,
    /// When each bonus from this platform ends, in the server's clock (ms; 0 = none).
    #[serde(default)]
    bonus: BonusEnds,
    /// The server's clock when it sent this (ms; 0 = unknown).
    #[serde(default)]
    now: i64,
}

/// When an account's bonuses from one platform end, in ms since 1970 (0 = not running). Kept
/// in the PC's clock (corrected for the difference to the game server's).
#[derive(serde::Deserialize, Serialize, Clone, Copy, Default, Debug, PartialEq)]
struct BonusEnds {
    #[serde(default)]
    xp: i64,
    #[serde(default)]
    capture: i64,
    #[serde(default)]
    lure: i64,
}

impl BonusEnds {
    /// Moves server-clock times to the PC's clock: `server_now` was the server's time when the
    /// PC's clock read `local_now`.
    fn to_local(self, server_now: i64, local_now: i64) -> BonusEnds {
        let offset = if server_now > 0 { server_now - local_now } else { 0 };
        let shift = |t: i64| if t > 0 { t - offset } else { 0 };
        BonusEnds { xp: shift(self.xp), capture: shift(self.capture), lure: shift(self.lure) }
    }
}

/// An account's running bonuses per platform, for the slot header and the settings.
#[derive(Serialize, Clone, Copy, Default, Debug)]
struct SlotBonuses {
    kick: Option<BonusEnds>,
    twitch: Option<BonusEnds>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

#[cfg(test)]
mod bonus_tests {
    use super::*;

    #[test]
    fn bonus_ends_follow_the_pc_clock() {
        // The server is 5 s ahead of the PC: its "ends at" times are 5 s later than the PC's.
        let server = BonusEnds { xp: 1_000_005_000, capture: 0, lure: 2_000_005_000 };
        let local = server.to_local(500_005_000, 500_000_000);
        assert_eq!(local, BonusEnds { xp: 1_000_000_000, capture: 0, lure: 2_000_000_000 });
        // Unknown server time: kept as is.
        assert_eq!(server.to_local(0, 500_000_000), server);
    }
}

/// Adds the channels of `partners` missing from `platform`'s priority list at its end, in the
/// game's order; the user's order is kept. Returns how many were added.
fn merge_partners(config: &mut Config, platform: Platform, partners: &[String]) -> usize {
    let list = match platform {
        Platform::Kick => &mut config.kick_priority,
        Platform::Twitch => &mut config.twitch_priority,
    };
    let before = list.len();
    let mut merged = list.clone();
    merged.extend(partners.iter().cloned());
    *list = match platform {
        Platform::Kick => config::clean_kick_priority(&merged),
        Platform::Twitch => config::clean_twitch_priority(&merged),
    };
    list.len() - before
}

fn sync_on(config: &Config, platform: Platform) -> bool {
    match platform {
        Platform::Kick => config.kick_sync,
        Platform::Twitch => config.twitch_sync,
    }
}

/// A game page (slot `id`) reported a platform's partner list and link status. Remembers both,
/// and with "Keep updated from the game" on, adds new partners to the priority list.
fn game_reported(app: &AppHandle, id: u8, json: &str) {
    let Ok(report) = serde_json::from_str::<GameReport>(json) else { return };
    println!(
        "[game] slot {id}: {} partners: {}, linked: {}",
        report.platform.name(),
        report.channels.len(),
        report.linked
    );
    let added = {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        state.linked.insert((id, report.platform), report.linked);
        if !report.channels.is_empty() {
            state.partners.insert(report.platform, report.channels.clone());
        }
        let ends = report.bonus.to_local(report.now, now_ms());
        let bonus_changed = state.bonus_ends.insert((id, report.platform), ends) != Some(ends);
        let added = if sync_on(&state.config, report.platform) && !report.channels.is_empty() {
            merge_partners(&mut state.config, report.platform, &report.channels)
        } else {
            0
        };
        if added > 0 {
            println!("[sync] {}: {added} new partner channel(s) from the game", report.platform.name());
            save_and_publish(app, &state);
        } else if bonus_changed {
            // The slot headers show the running bonuses.
            publish(app, &state);
        }
        added
    };
    if added > 0 {
        AUTO_WAKE.store(true, Ordering::SeqCst);
    }
    let _ = app.emit_to("settings", "automation", automation_view(app));
}

#[cfg(test)]
mod sync_tests {
    use super::*;

    #[test]
    fn new_partners_go_to_the_end_and_the_users_order_stays() {
        let mut config = Config {
            kick_priority: vec!["matta".into(), "axt".into()],
            ..Config::default()
        };
        let partners: Vec<String> = ["axt", "cogu", "matta", "yetz"].iter().map(|s| s.to_string()).collect();
        assert_eq!(merge_partners(&mut config, Platform::Kick, &partners), 2);
        assert_eq!(config.kick_priority, vec!["matta", "axt", "cogu", "yetz"]);
        assert_eq!(merge_partners(&mut config, Platform::Kick, &partners), 0);
        assert!(config.twitch_priority.is_empty());
    }
}

/// "Keep updated from the game" for a platform's priority list. Turning it on adds the
/// partners the game has already announced right away.
#[tauri::command]
fn set_sync(app: AppHandle, platform: Platform, on: bool) -> AutomationView {
    automation_changed(&app, |state| {
        match platform {
            Platform::Kick => state.config.kick_sync = on,
            Platform::Twitch => state.config.twitch_sync = on,
        }
        if on {
            let partners = state.partners.get(&platform).cloned().unwrap_or_default();
            merge_partners(&mut state.config, platform, &partners);
        }
    })
}

#[tauri::command]
fn set_twitch_lives(app: AppHandle, count: u8) -> AutomationView {
    automation_changed(&app, |state| {
        state.config.twitch_lives_per_slot = count.clamp(1, MAX_PAGES_PER_PLATFORM as u8);
    })
}

#[tauri::command]
fn set_bonus(app: AppHandle, id: u8, bonus: Option<KickBonus>) -> AutomationView {
    automation_changed(&app, |state| {
        if let Some(slot) = state.config.slots.iter_mut().find(|s| s.id == id) {
            slot.bonus = bonus;
        }
    })
}

// ---------------------------------------------------------------- auto-reload

/// Reloads every game page every `auto_reload_minutes` (checked twice a minute). Turning the
/// option on or changing it restarts the countdown.
fn start_auto_reload(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last = std::time::Instant::now();
        let mut minutes = 0u32;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(30));
            let (wanted, ids) = {
                let state = app.state::<Mutex<AppState>>();
                let state = state.lock().unwrap();
                let ids: Vec<u8> = state.config.slots.iter().map(|s| s.id).collect();
                (state.config.auto_reload_minutes, ids)
            };
            if wanted != minutes {
                minutes = wanted;
                last = std::time::Instant::now();
                continue;
            }
            if minutes == 0 || last.elapsed() < std::time::Duration::from_secs(u64::from(minutes) * 60) {
                continue;
            }
            last = std::time::Instant::now();
            println!("[reload] auto-reloading {} game page(s) (every {minutes} min)", ids.len());
            for id in ids {
                if let Some(webview) = app.get_webview(&slot_label(id)) {
                    let _ = webview.reload();
                }
            }
        }
    });
}

// ---------------------------------------------------------------- startup

fn main() {
    tauri::Builder::default()
        // Must be the first plugin. Two copies would share the slots' data folders and run every
        // background stream twice, so a second launch exits and brings this window forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            println!("[app] another copy was started; showing this one instead");
            tray::show_main(app);
        }))
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
                live: HashMap::new(),
                visits: HashMap::new(),
                visiting: std::collections::HashSet::new(),
                partners: HashMap::new(),
                linked: HashMap::new(),
                bonus_ends: HashMap::new(),
                health: HashMap::new(),
                daily_tries: HashMap::new(),
                vote: HashMap::new(),
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
            for slot in &slots {
                create_slot_header(&handle, slot.id)?;
            }
            // The two splitter strips (local page, same environment as the toolbar); placed
            // over the gaps between slots by apply_layout.
            for (label, axis) in [(SPLIT_V, "v"), (SPLIT_H, "h")] {
                let splitter = window.add_child(
                    WebviewBuilder::new(label, WebviewUrl::App(format!("splitter.html?axis={axis}").into())),
                    LogicalPosition::new(0.0, BAR_HEIGHT),
                    LogicalSize::new(1.0, 1.0),
                )?;
                splitter.hide()?;
            }
            // Toolbar last: creating a WebView2 pumps window messages, so a toolbar created
            // first would load and read the state before the streams above were restored.
            window.add_child(
                WebviewBuilder::new("toolbar", WebviewUrl::App("index.html".into())),
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(1280.0, BAR_HEIGHT),
            )?;
            apply_layout(&handle)?;

            tray::create(&handle)?;
            tray::update_tooltip(&handle);
            {
                let state = handle.state::<Mutex<AppState>>();
                let state = state.lock().unwrap();
                power::set_keep_awake(&handle, state.config.keep_awake && any_pages(&state));
            }
            start_auto_reload(handle.clone());
            start_stream_manager(handle.clone());

            let event_handle = handle.clone();
            let event_window = window.clone();
            window.on_window_event(move |event| match event {
                WindowEvent::Resized(_) => {
                    let (to_tray, quitting) = tray::settings(&event_handle);
                    if to_tray.minimize && !quitting && event_window.is_minimized().unwrap_or(false) {
                        let _ = event_window.hide();
                        return;
                    }
                    let _ = apply_layout(&event_handle);
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    let _ = apply_layout(&event_handle);
                }
                WindowEvent::CloseRequested { api, .. } => {
                    // Record which channel each panel ended up on, whether we hide or quit.
                    remember_panels(&event_handle);
                    let (to_tray, quitting) = tray::settings(&event_handle);
                    if to_tray.close && !quitting {
                        api.prevent_close();
                        let _ = event_window.hide();
                    }
                }
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
            show_page,
            stop_page,
            stop_all_pages,
            add_channel,
            reorder_pages,
            focus_slot,
            toggle_mute,
            rename_slot,
            drag_split,
            drag_split_end,
            reset_layout,
            set_toolbar_hidden,
            get_settings,
            set_settings,
            open_settings,
            get_automation,
            set_priority,
            set_twitch_lives,
            set_bonus,
            set_sync,
            set_vote_slot,
            open_vote
        ])
        .run(tauri::generate_context!())
        .expect("error while running PokeIdle Multiview");
}
