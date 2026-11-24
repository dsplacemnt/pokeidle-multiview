#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Everything the app prints also goes to its log file (see applog.rs). Defined before the
// modules so they all use these.
macro_rules! println {
    () => { $crate::applog::out(String::new(), false) };
    ($($arg:tt)*) => { $crate::applog::out(format!($($arg)*), false) };
}
macro_rules! eprintln {
    () => { $crate::applog::out(String::new(), true) };
    ($($arg:tt)*) => { $crate::applog::out(format!($($arg)*), true) };
}

mod applog;

#[cfg(windows)]
mod away;
#[cfg(not(windows))]
#[path = "away_other.rs"]
mod away;
#[cfg(not(windows))]
mod compat;
mod channels;
mod config;
mod discord;
mod external;
mod game;
mod goals;
mod hunt;
mod kick;
mod twitch;
mod layout;
#[cfg(windows)]
mod native;
#[cfg(not(windows))]
#[path = "native_other.rs"]
mod native;
mod net;
#[cfg(windows)]
mod power;
#[cfg(not(windows))]
#[path = "power_other.rs"]
mod power;
mod splash;
mod heavy;
mod logins;
mod prey;
mod resources;
mod runtime;
mod shortcuts;
mod support;
mod whatsnew;
mod tray;
mod updater;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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
#[cfg(not(windows))]
use compat::{HwndCompat, OwnerCompat};

/// The toolbar across the top of the window (app-wide buttons).
const BAR_HEIGHT: f64 = 40.0;
/// Each slot's header, on top of its tile: the slot's controls and its stream pills, side by side.
const HEADER_HEIGHT: f64 = 32.0;
const GAME_URL: &str = "https://pokeidle.io/";
const WEBAUTHN_BLOCK: &str = include_str!("../scripts/webauthn-block.js");
const DESKTOP_LAYOUT: &str = include_str!("../scripts/desktop-layout.js");
const LOWEST_QUALITY: &str = include_str!("../scripts/lowest-quality.js");
const ALWAYS_VISIBLE: &str = include_str!("../scripts/always-visible.js");
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
    /// Channels a page found offline while the platform still said live, per account: kept
    /// out of that account's picks for a while (see `OFFLINE_REPORTS_TO_SWAP`).
    page_offline: HashMap<(u8, Platform, String), std::time::Instant>,
    /// Kick Daily Reward tries per slot today: (UTC day, tries).
    daily_tries: HashMap<u8, (i64, u8)>,
    /// The last IdleHub vote status of each reminded account: (status, when it was asked).
    vote: HashMap<u8, (VoteStatus, std::time::Instant)>,
    /// Recent automation events per account (newest last), for the single-focus panel.
    log: HashMap<u8, std::collections::VecDeque<(i64, String)>>,
    /// The game's session numbers per account (see game-watch.js), for the panel.
    game_stats: HashMap<u8, serde_json::Value>,
    /// A redeem just went through for this account and platform: in goals mode it waits for
    /// the game to report the new time before buying again (see `bonus_now`).
    redeemed: HashMap<(u8, Platform), std::time::Instant>,
    /// The slot that was fullscreen last, so the grid shortcut can go back to it.
    last_focus: Option<u8>,
    /// Whether each account is logged in on each platform (from its cookies; see logins.rs).
    logins: HashMap<(u8, Platform), bool>,
    /// Pages opened by a "Log in" button (see logins.rs): they close once the account is in.
    login_pages: std::collections::HashSet<String>,
    /// Each account's game connection and session, as its page last reported (see game.rs).
    game_conn: HashMap<u8, game::GameConn>,
    /// Where each Pokémon look's first frame is on the game's sprite sheets (see game.rs).
    sprites: HashMap<u32, serde_json::Value>,
    /// What the Discord alerts already sent (see discord.rs).
    discord_sent: discord::Sent,
    /// Each account's daily Prey cards, as its game page reported them (see prey.rs).
    prey: HashMap<u8, (prey::Report, serde_json::Value)>,
    /// Platforms whose live checks all failed last time: why, as a kind (see `CheckDown`).
    checks_down: HashMap<Platform, CheckDown>,
}

/// Why none of a platform's channels could be checked, so the pages can tell the player what to
/// look at.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
enum CheckDown {
    /// The HTTPS certificate isn't trusted: something on the network or PC inspects HTTPS.
    Certificate,
    /// Anything else: no internet, a firewall, a blocked site, a page that isn't the platform's.
    Network,
}

impl CheckDown {
    fn of(error: &str) -> Self {
        if error.to_ascii_lowercase().contains("certificate") { CheckDown::Certificate } else { CheckDown::Network }
    }
}

#[cfg(test)]
mod check_down_tests {
    use super::*;

    #[test]
    fn certificate_errors_are_told_apart() {
        // As ureq/rustls word it in app.log on a network that inspects HTTPS.
        assert_eq!(CheckDown::of("io: invalid peer certificate: UnknownIssuer"), CheckDown::Certificate);
        assert_eq!(CheckDown::of("io: timed out"), CheckDown::Network);
        assert_eq!(CheckDown::of("unexpected page"), CheckDown::Network);
        assert_eq!(serde_json::to_value(CheckDown::Certificate).unwrap(), "certificate");
    }
}

/// Events kept per account in the automation log.
const LOG_KEEP: usize = 60;

/// Adds an event to an account's automation log (shown in its single-focus panel).
fn log_event(state: &mut AppState, id: u8, text: impl Into<String>) {
    let log = state.log.entry(id).or_default();
    log.push_back((now_ms(), text.into()));
    while log.len() > LOG_KEEP {
        log.pop_front();
    }
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
    /// A fullscreen slot shows its stats panel (the ▤ button in its header toggles it).
    focus_panel: bool,
    /// Economy mode: the slots are hidden and the rest screen shows (see `set_economy`).
    economy: bool,
    /// Which accounts are logged in on Twitch and Kick: slot → platform → logged in.
    logins: HashMap<u8, HashMap<Platform, bool>>,
    /// Accounts whose game dropped out (logged in elsewhere, disconnected, no session): slot → text.
    game_issues: HashMap<u8, String>,
    /// Accounts whose daily Prey is due: slot → "roll" (free roll ready) or "pick".
    prey_pending: HashMap<u8, String>,
    /// Platforms whose live checks all fail: nothing is opened or swapped there until they work.
    checks_down: HashMap<Platform, CheckDown>,
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
    // WebKitGTK also asks about every frame inside the page (ads, Twitch's anti-bot frame…),
    // and the callback can't tell them from the page itself: there, other sites load in place
    // (as frames do on Windows) and only new windows go to the browser (`guard_popup`).
    #[cfg(not(windows))]
    {
        let _ = url;
        true
    }
    #[cfg(windows)]
    {
        eprintln!("[nav] not in slot: {url}");
        external::open_in_browser(url);
        false
    }
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
        focus_panel: state.config.focus_panel,
        economy: state.config.economy,
        logins: logins::view(state),
        game_issues: game::issues(state),
        prey_pending: prey::pendings(state),
        checks_down: state.checks_down.clone(),
    }
}

fn toolbar_hidden(state: &AppState) -> bool {
    state.config.toolbar_hidden && !state.config.slots.is_empty() && !state.config.economy
}

fn save_and_publish(app: &AppHandle, state: &AppState) {
    if let Err(e) = state.config.save(&state.config_path) {
        eprintln!("[config] could not save: {e}");
    }
    publish(app, state);
    tray::set_tooltip(app, &state.panels, state.config.language);
    power::set_keep_awake(app, state.config.keep_awake && any_pages(state));
}

/// A state update is on its way to the pages (see `publish`).
static PUBLISH_PENDING: AtomicBool = AtomicBool::new(false);
/// Changes that come close together (pages loading at startup, a manager pass) go out as one
/// update this long after the first.
const PUBLISH_AFTER: std::time::Duration = std::time::Duration::from_millis(100);

/// Sends the state to the app's own pages: the toolbar, every slot header and the stats panel.
/// Coalesced: the first call schedules one send with the state as it is then, and calls until
/// it goes out join it, so a burst of changes redraws the pages once instead of every time.
fn publish(app: &AppHandle, _state: &AppState) {
    if PUBLISH_PENDING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(PUBLISH_AFTER);
        // Cleared before reading the state: a change after this point schedules another send.
        PUBLISH_PENDING.store(false, Ordering::SeqCst);
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        publish_now(&app, &state);
    });
}

fn publish_now(app: &AppHandle, state: &AppState) {
    let view = state_view(state);
    let _ = app.emit_to("toolbar", "state", view.clone());
    for id in 1..=MAX_SLOTS {
        let _ = app.emit_to(header_label(id).as_str(), "state", view.clone());
    }
    let _ = app.emit_to(REST_PAGE, "state", view.clone());
    let _ = app.emit_to(FOCUS_PANEL, "focus", focus_view(state));
}

#[derive(Serialize, Clone)]
struct FocusStream {
    platform: Platform,
    login: Option<String>,
    warning: Option<String>,
    shown: bool,
    /// The account doesn't follow this channel (the ♡ tip).
    not_following: bool,
}

/// Everything the single-focus stats panel shows about the fullscreen account.
#[derive(Serialize, Clone)]
struct FocusView {
    slot: u8,
    name: String,
    bonus: Option<KickBonus>,
    goals: Option<config::Goals>,
    /// What the next redeem buys on each platform (None = nothing now).
    next: NextRedeem,
    running: SlotBonuses,
    streams: Vec<FocusStream>,
    /// Last seen channel points, most first: [platform, channel, points].
    points: Vec<(Platform, String, u64)>,
    kick_linked: Option<bool>,
    twitch_linked: Option<bool>,
    /// Logged in on Kick / Twitch in this slot (None = not checked yet).
    kick_login: Option<bool>,
    twitch_login: Option<bool>,
    vote_reminded: bool,
    vote: Option<VoteStatus>,
    daily_enabled: bool,
    daily_done_today: bool,
    /// Newest first: [time (ms), text].
    log: Vec<(i64, String)>,
    /// The game's session numbers, as its own state reports them (see game-watch.js).
    game: Option<serde_json::Value>,
    /// What is wrong with this account's game, if anything (see game.rs).
    game_issue: Option<String>,
    /// Pokémon sprites by look, for the hunt analysis.
    sprites: HashMap<u32, serde_json::Value>,
    /// The account's daily Prey as its page reported it, and what's left to do today.
    prey: Option<serde_json::Value>,
    prey_pending: Option<String>,
}

fn focus_view(state: &AppState) -> Option<FocusView> {
    if !focus_panel_on(&state.config) {
        return None;
    }
    let id = state.config.focus?;
    let slot = state.config.slots.iter().find(|s| s.id == id)?;
    let panels = state.panels.get(&id);
    let streams = panels
        .map(|p| {
            p.pages
                .iter()
                .map(|pg| FocusStream {
                    platform: pg.platform,
                    login: pg.login.clone(),
                    warning: pg.warning.clone(),
                    shown: p.front.as_deref() == Some(pg.label.as_str()),
                    not_following: pg.not_following,
                })
                .collect()
        })
        .unwrap_or_default();
    Some(FocusView {
        slot: id,
        name: slot.name.clone().unwrap_or_else(|| format!("Slot {id}")),
        bonus: slot.bonus,
        goals: slot.goals.clone(),
        next: next_redeem(state, slot),
        running: SlotBonuses {
            kick: state.bonus_ends.get(&(id, Platform::Kick)).copied(),
            twitch: state.bonus_ends.get(&(id, Platform::Twitch)).copied(),
        },
        streams,
        points: slot_points(&state.config, id),
        kick_linked: state.linked.get(&(id, Platform::Kick)).copied(),
        twitch_linked: state.linked.get(&(id, Platform::Twitch)).copied(),
        kick_login: state.logins.get(&(id, Platform::Kick)).copied(),
        twitch_login: state.logins.get(&(id, Platform::Twitch)).copied(),
        vote_reminded: state.config.vote_slots.contains(&id),
        vote: state.vote.get(&id).map(|(s, _)| s.clone()),
        daily_enabled: state.config.kick_daily,
        daily_done_today: state.config.kick_daily_done.get(&id) == Some(&utc_day()),
        log: state.log.get(&id).map(|l| l.iter().rev().cloned().collect()).unwrap_or_default(),
        game: state.game_stats.get(&id).cloned(),
        game_issue: state.game_conn.get(&id).and_then(|c| game::issue(c, now_ms())).map(str::to_string),
        sprites: state.sprites.clone(),
        prey: state.prey.get(&id).map(|(_, v)| v.clone()),
        prey_pending: prey::pending_for(state, id, now_ms()).map(str::to_string),
    })
}

#[tauri::command]
fn get_focus(state: tauri::State<'_, Mutex<AppState>>) -> Option<FocusView> {
    focus_view(&state.lock().unwrap())
}

#[tauri::command]
fn get_language(state: tauri::State<'_, Mutex<AppState>>) -> config::Language {
    state.lock().unwrap().config.language
}

/// The flags on the top bar (or ⚙): saved, and every open page, the tray and the window titles
/// switch right away.
#[tauri::command]
fn set_language(app: AppHandle, lang: config::Language) {
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        state.config.language = lang;
        if let Err(e) = state.config.save(&state.config_path) {
            eprintln!("[config] could not save: {e}");
        }
    }
    let mut pages = vec![
        "toolbar".to_string(),
        FOCUS_PANEL.to_string(),
        REST_PAGE.to_string(),
        "settings".to_string(),
        "support".to_string(),
        "whatsnew".to_string(),
    ];
    pages.extend((1..=MAX_SLOTS).map(header_label));
    for page in pages {
        let _ = app.emit_to(page.as_str(), "language", lang);
    }
    tray::set_language(&app);
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.set_title(settings_title(lang));
    }
    if let Some(w) = app.get_webview_window("support") {
        let _ = w.set_title(support::title(lang));
    }
}

fn settings_title(lang: config::Language) -> &'static str {
    lang.pick("Settings — PokeIdle Multiview", "Configurações — PokeIdle Multiview", "Configuración — PokeIdle Multiview")
}

/// The ▤ button: shows or hides the stats panel of a fullscreen slot (saved).
#[tauri::command]
fn set_focus_panel(app: AppHandle, on: bool) {
    app.state::<Mutex<AppState>>().lock().unwrap().config.focus_panel = on;
    if let Err(e) = apply_layout(&app) {
        eprintln!("[layout] {e}");
    }
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(&app, &state.lock().unwrap());
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

/// Width of the stats panel at the right of a fullscreen slot ("single focus"), logical px.
const FOCUS_PANEL_W: f64 = 360.0;
const FOCUS_PANEL: &str = "focus";

/// The stats panel shows while a slot is fullscreen, unless the player hid it.
fn focus_panel_on(config: &Config) -> bool {
    !config.economy && config.focus_panel && config.focus.is_some_and(|f| config.slots.iter().any(|s| s.id == f))
}

/// The rest screen of economy mode (local page), over the whole game area.
const REST_PAGE: &str = "rest";

fn current_layout(app: &AppHandle) -> tauri::Result<layout::Layout> {
    let mut area = game_area(app)?;
    let config = app.state::<Mutex<AppState>>().lock().unwrap().config.clone();
    // Single focus: the slot leaves room for the stats panel at the right, right next to it
    // (no gap), so the header's accent line runs straight into the panel's accent border.
    if focus_panel_on(&config) {
        area.w = (area.w - FOCUS_PANEL_W).max(0.0);
    }
    // Economy mode: no tiles at all, so every slot is hidden (and keeps running).
    let ids: Vec<u8> = if config.economy { Vec::new() } else { config.slots.iter().map(|s| s.id).collect() };
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


fn layout_plan(app: &AppHandle) -> tauri::Result<Vec<(String, Placement)>> {
    let mut plan = Vec::new();
    let area = game_area(app)?;
    // The toolbar shows or hides; the slots take its place.
    plan.push((
        "toolbar".to_string(),
        if area.y > 0.0 {
            Placement::Show(Rect { x: 0.0, y: 0.0, w: area.w, h: area.y })
        } else {
            Placement::Hide(None)
        },
    ));
    // The stats panel of a fullscreen slot, at the right of the game area.
    let panel_on = focus_panel_on(&app.state::<Mutex<AppState>>().lock().unwrap().config);
    plan.push((
        FOCUS_PANEL.to_string(),
        if panel_on {
            let w = FOCUS_PANEL_W.min(area.w);
            Placement::Show(Rect { x: area.x + area.w - w, y: area.y, w, h: area.h })
        } else {
            Placement::Hide(None)
        },
    ));
    let economy = app.state::<Mutex<AppState>>().lock().unwrap().config.economy;
    plan.push((REST_PAGE.to_string(), if economy { Placement::Show(area) } else { Placement::Hide(None) }));
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

/// A Twitch/Kick page of a slot (not a game, header or the app's own pages).
fn is_stream_label(label: &str) -> bool {
    label.contains("-twitch-") || label.contains("-kick-")
}

fn set_rect(webview: &tauri::Webview, r: Rect) -> tauri::Result<()> {
    // Linux: Tauri can't place child webviews there, so they go in our own container.
    #[cfg(not(windows))]
    native::place(webview, r);
    webview.set_position(LogicalPosition::new(r.x, r.y))?;
    webview.set_size(LogicalSize::new(r.w, r.h))
}

/// Places every webview for the current layout (window resizes, splitters, fullscreen, the
/// toolbar, slots and streams coming and going).
fn apply_layout(app: &AppHandle) -> tauri::Result<()> {
    if app.get_window("main").is_none() {
        return Ok(());
    }
    for (label, placement) in layout_plan(app)? {
        let Some(webview) = app.get_webview(&label) else { continue };
        match placement {
            Placement::Show(to) => {
                set_rect(&webview, to)?;
                webview.show()?;
                if is_stream_label(&label) {
                    native::set_low_memory(&webview, false);
                }
            }
            Placement::Park(r) => {
                set_rect(&webview, r)?;
                webview.show()?;
                // Background streams ask WebView2 for low memory: measured with 32 streams, about
                // 1 GB less, still playing and earning (MULTIVIEW_NORMAL_MEMORY=1 turns it off).
                native::set_low_memory(&webview, std::env::var_os("MULTIVIEW_NORMAL_MEMORY").is_none());
            }
            Placement::Hide(r) => {
                if let Some(r) = r {
                    set_rect(&webview, r)?;
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
    .data_directory(data_dir.clone())
    // Dark until the page paints (no white flash while it loads).
    .background_color(BACKGROUND)
    .zoom_hotkeys_enabled(true)
    .on_page_load(|webview, payload| lower_player_quality(&webview, &payload))
    .on_navigation(|url| guard_navigation(url))
    // Popups (e.g. a sign-in window) open with WebView2's default window, which shares the
    // slot's environment and therefore its login. Other sites open in the default browser.
    .on_new_window(|url, _features| guard_popup(&url));

    if std::env::var_os("MULTIVIEW_ALLOW_WEBAUTHN").is_none() {
        builder = builder.initialization_script_for_all_frames(WEBAUTHN_BLOCK);
    }
    builder = builder.additional_browser_args(&slot_browser_args(slot_id));
    if let Some(dir) = hunt_dir {
        builder = builder.browser_extensions_enabled(true).extensions_path(dir);
    }
    // Outside Windows (WebKit): the page reports shortcuts itself (no accelerator hook there),
    // and asks not to be slowed down when it isn't seen.
    #[cfg(not(windows))]
    {
        builder = builder
            .initialization_script(native::KEYS_SCRIPT)
            .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled);
    }
    builder
}

/// The browser options of a slot's session (and of the shared browser, with slot 1's port).
fn slot_browser_args(slot_id: u8) -> String {
    // The streams must keep playing when Windows thinks nobody sees them: the window minimized
    // or in the tray, covered, or the screen off or locked. By default the browser then marks
    // the pages hidden and slows them down, and Twitch's player pauses itself (whole nights of
    // paused streams and reloads). The browser also pauses muted videos in a hidden page on its
    // own (Twitch's player is muted, Kick's isn't, so only Twitch stopped);
    // --disable-background-media-suspend turns that off. Setting browser args replaces wry's
    // defaults, so those are repeated here.
    // --process-per-site: pages of the same site share one renderer process instead of one
    // each, so a slot's Twitch players run in one process and its Kick pages in another (the
    // game is another site and keeps its own). Measured with 8 Twitch players: 1129 → 706 MB.
    #[allow(unused_mut)]
    let mut args = String::from(
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,CalculateNativeWinOcclusion,\
         BackgroundVideoPauseOptimization,BackgroundVideoTrackOptimization \
         --disable-background-media-suspend \
         --autoplay-policy=no-user-gesture-required \
         --disable-background-timer-throttling \
         --disable-renderer-backgrounding \
         --disable-backgrounding-occluded-windows \
         --process-per-site",
    );
    // Debug builds only: MULTIVIEW_DEVTOOLS=1 exposes slot N to Chrome DevTools Protocol on
    // 127.0.0.1:930N (open edge://inspect or http://127.0.0.1:930N/json).
    #[cfg(debug_assertions)]
    if std::env::var_os("MULTIVIEW_DEVTOOLS").is_some() {
        args.push_str(&format!(
            " --remote-debugging-address=127.0.0.1 --remote-debugging-port={}",
            9300 + u16::from(slot_id)
        ));
    }
    args
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
/// Debug builds only: with MULTIVIEW_DEVTOOLS=1 the app's own pages (toolbar, headers, panel,
/// settings) share CDP port 9300. They share one WebView2 environment, so every one of them must
/// get the same arguments.
#[cfg(debug_assertions)]
fn local_args() -> Option<&'static str> {
    std::env::var_os("MULTIVIEW_DEVTOOLS").map(|_| {
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
         --autoplay-policy=no-user-gesture-required \
         --remote-debugging-address=127.0.0.1 --remote-debugging-port=9300"
    })
}
#[cfg(not(debug_assertions))]
fn local_args() -> Option<&'static str> {
    None
}

/// Behind every page until it paints, and behind the window itself: the app's dark gray.
const BACKGROUND: tauri::window::Color = tauri::window::Color(22, 24, 29, 255);

/// A webview for one of the app's own pages in ui/.
fn local_page(label: impl Into<String>, page: String) -> WebviewBuilder<tauri::Wry> {
    let builder = WebviewBuilder::new(label, WebviewUrl::App(page.into())).background_color(BACKGROUND);
    match local_args() {
        Some(args) => builder.additional_browser_args(args),
        None => builder,
    }
}

fn create_slot_header(app: &AppHandle, id: u8) -> tauri::Result<()> {
    if app.get_webview(&header_label(id)).is_some() {
        return Ok(());
    }
    let window = app.get_window("main").expect("main window exists");
    window.add_child(
        local_page(header_label(id), format!("slot.html?id={id}")),
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
    let load_app = app.clone();
    let builder = slot_session_builder(app, slot.id, slot_label(slot.id), GAME_URL)
        // The loading screen waits for each game's first load.
        .on_page_load(move |_, payload| {
            if payload.event() == PageLoadEvent::Finished {
                splash::game_loaded(&load_app, id);
                // A game (re)loaded: its Battery Saver as the economy mode wants it (the script
                // waits until the game is ready).
                if payload.url().host_str() == Some("pokeidle.io") {
                    let on = load_app.state::<Mutex<AppState>>().lock().unwrap().config.economy;
                    sync_game_economy(&load_app, id, on);
                }
            }
        })
        .initialization_script(DESKTOP_LAYOUT)
        // Listens to the game's own state messages for its Kick/Twitch partner channels and the
        // account's link status; it reports through a brief title change.
        .initialization_script(GAME_WATCH)
        .on_document_title_changed(move |_, title| {
            if let Some(keys) = title.strip_prefix("[multiview-key] ") {
                shortcut_from_page(&title_app, keys);
            } else if let Some(json) = title.strip_prefix("[multiview-game] ") {
                game_reported(&title_app, id, json);
            } else if let Some(json) = title.strip_prefix("[multiview-vote] ") {
                vote_reported(&title_app, id, json);
            } else if let Some(json) = title.strip_prefix("[multiview-stats] ") {
                stats_reported(&title_app, id, json);
            } else if let Some(json) = title.strip_prefix("[multiview-conn] ") {
                game::conn_reported(&title_app, id, json);
            } else if let Some(json) = title.strip_prefix("[multiview-sprite] ") {
                game::sprite_reported(&title_app, json);
            } else if let Some(json) = title.strip_prefix("[multiview-prey] ") {
                prey::reported(&title_app, id, json);
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
    // A WebView2 runtime update under this browser: restart onto it (runtime.rs).
    runtime::watch(&webview, app.clone());
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
    /// The account doesn't follow this channel (as the page last showed it): the pill gets a
    /// tip that following is possible. The app never follows by itself.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    not_following: bool,
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
#[cfg(not(windows))]
fn set_muted(webview: &tauri::Webview, muted: bool) {
    native::set_muted(webview, muted);
}

#[cfg(windows)]
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
    page_activity();
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
            page_activity();
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
    if matches!(url.host_str(), Some(h) if h.ends_with("twitch.tv") || h.ends_with("kick.com")) {
        logins::check_soon();
    }
    // Whether the account follows the channel, now and whenever it changes (the ♡ tip).
    if channels::channel_from_url(url).is_some() {
        if let Err(e) = webview.eval(format!("{}\n{FOLLOW_WATCH}", kick::REPORT)) {
            eprintln!("[panel] could not start the follow watch: {e}");
        }
    }
    twitch_visit_loaded(app, webview, url);
}

/// Reports whether the account follows the page's channel (see `follow_reported`).
const FOLLOW_WATCH: &str = include_str!("../scripts/follow-watch.js");

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
        // Keeps playing when the app is minimized or in the tray (scripts/always-visible.js).
        .initialization_script_for_all_frames(ALWAYS_VISIBLE)
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
            if let Some(keys) = title.strip_prefix("[multiview-key] ") {
                shortcut_from_page(&title_app, keys);
            } else if let Some(message) = title.strip_prefix("[multiview] ") {
                println!("[auto] {}: {message}", webview.label());
                // Into the account's log, leaving out routine "not enough points" answers.
                let worth = ["redeemed", "chest", "error", "no "].iter().any(|w| message.contains(w));
                if worth {
                    let channel = channels::channel_from_url(&webview.url().unwrap_or_else(|_| "about:blank".parse().unwrap()))
                        .map(|c| format!("{} {}: ", c.platform.name(), c.login))
                        .unwrap_or_default();
                    // "Twitch x: Twitch visit: chest claimed" reads better as "Twitch x: chest claimed".
                    let message = message.split_once("visit: ").map_or(message, |(_, rest)| rest);
                    let state = title_app.state::<Mutex<AppState>>();
                    let mut state = state.lock().unwrap();
                    log_event(&mut state, id, format!("{channel}{message}"));
                    // Goals mode waits for the game's new time before buying again.
                    if message.contains("redeemed ") {
                        state.redeemed.insert((id, platform), std::time::Instant::now());
                    }
                }
            } else if let Some(json) = title.strip_prefix("[multiview-points] ") {
                points_reported(&title_app, id, json);
            } else if let Some(json) = title.strip_prefix("[multiview-follow] ") {
                follow_reported(&title_app, id, webview.label(), json);
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
    // A page the app opens by itself (a background stream) doesn't take the focus: that would
    // bring a minimized or tray window back on screen.
    let builder = builder.focused(!background);
    page_activity();
    let page = match window.add_child(builder, LogicalPosition::new(r.x, r.y), LogicalSize::new(r.w, r.h)) {
        Ok(page) => page,
        Err(e) => {
            // After a WebView2 runtime update no page can be created any more: restart.
            runtime::page_failed(app, &e.to_string());
            return Err(e);
        }
    };
    if background {
        set_muted(&page, true);
    }
    native::attach(&page, app.clone());
    let login = url.parse::<Url>().ok().and_then(|u| channels::channel_from_url(&u)).map(|c| c.login);
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    let not_following =
        login.as_ref().is_some_and(|l| state.config.follows.get(&config::balance_key(id, platform, l)) == Some(&false));
    state.panels.entry(id).or_default().pages.push(Page { label: label.clone(), platform, login, warning: None, not_following });
    Ok(label)
}

/// Writes every running page and where it is into the config, so the streams come back after
/// a restart. Called on each page change and when the window closes (to catch channels the
/// user switched to inside a page).
///
/// Asking a page for its URL or the window for its size waits for the UI thread, so that's
/// done with the state unlocked: the manager calls this from its own thread, and holding the
/// lock there while the UI thread waits for it in a command froze the app.
fn remember_panels(app: &AppHandle) {
    remember_panels_with(app, true);
}

/// Like `remember_panels`, for the stream manager's frequent saves: a page on a known channel
/// is saved as that channel without asking the page (each question waits for the UI thread);
/// only pages off a channel (login, home) are asked. Channels switched inside a page are
/// caught by the full save when the window closes.
fn remember_panels_known(app: &AppHandle) {
    remember_panels_with(app, false);
}

fn remember_panels_with(app: &AppHandle, ask_pages: bool) {
    let running = app.state::<Mutex<AppState>>().lock().unwrap().panels.clone();
    let saved: HashMap<u8, Vec<SavedPanel>> = running
        .iter()
        .map(|(id, panels)| {
            let pages = panels
                .pages
                .iter()
                .map(|page| {
                    let url = (ask_pages || page.login.is_none())
                        .then(|| app.get_webview(&page.label))
                        .flatten()
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
            (*id, pages)
        })
        .collect();
    // The main window's size and maximized state, for the next start (it opens centered).
    // A maximized or minimized window's size isn't the one to come back to.
    let window = app.get_window("main").map(|window| {
        // Away (off-screen, see away.rs): the state it had before, and its size is kept as saved.
        let away = away::saved_maximized();
        let maximized = away.unwrap_or_else(|| window.is_maximized().unwrap_or(false));
        let minimized = window.is_minimized().unwrap_or(false);
        let size = (away.is_none() && !maximized && !minimized && window.is_visible().unwrap_or(false))
            .then(|| Some(window.inner_size().ok()?.to_logical::<f64>(window.scale_factor().ok()?)))
            .flatten()
            .filter(|s| s.width >= 640.0 && s.height >= 400.0);
        (maximized, size)
    });

    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    for slot in &mut state.config.slots {
        // Pages opened or closed meanwhile are caught by the next call.
        slot.panels = saved.get(&slot.id).cloned().unwrap_or_default();
    }
    if let Some((maximized, size)) = window {
        let mut geom = state.config.window.unwrap_or_default();
        geom.maximized = maximized;
        if let Some(size) = size {
            geom.width = size.width;
            geom.height = size.height;
        }
        state.config.window = Some(geom);
    }
    if let Err(e) = state.config.save(&state.config_path) {
        eprintln!("[config] could not save: {e}");
    }
}

/// Set once the streams saved at the last exit are open again (see `restore_panels`).
static RESTORED: AtomicBool = AtomicBool::new(false);

/// Reopens the streams saved at the last exit, in the background. One page at a time on the
/// main thread with a pause between them, so the window (and the loading screen's progress)
/// keeps updating instead of freezing until all of them exist.
fn restore_panels(app: &AppHandle, slots: &[SlotConfig]) {
    let mut pages: Vec<(u8, Platform, String)> = Vec::new();
    for slot in slots {
        for saved in &slot.panels {
            let parsed = saved.url.parse::<Url>().ok().filter(slot_may_navigate);
            // A channel comes back on its light player; other pages as they were.
            let url = match parsed.as_ref().and_then(channels::channel_from_url) {
                Some(channel) => channel.url_for(false),
                None if parsed.is_some() => saved.url.clone(),
                None => saved.platform.start_url().to_string(),
            };
            pages.push((slot.id, saved.platform, url));
        }
    }
    let total = pages.len();
    splash::streams_progress(app, 0, total);
    let app = app.clone();
    std::thread::spawn(move || {
        for (done, (id, platform, url)) in pages.into_iter().enumerate() {
            let (tx, rx) = std::sync::mpsc::channel();
            let main = app.clone();
            let _ = app.run_on_main_thread(move || {
                match create_page(&main, id, platform, &url, true) {
                    Ok(_) => println!("[panel] slot {id}: resumed {url}"),
                    Err(e) => eprintln!("[panel] slot {id}: could not resume {url}: {e}"),
                }
                let _ = tx.send(());
            });
            let _ = rx.recv();
            splash::streams_progress(&app, done + 1, total);
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
        // The headers and the toolbar learn about the reopened streams; pages go in place.
        let main = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Err(e) = apply_layout(&main) {
                eprintln!("[layout] {e}");
            }
            let state = main.state::<Mutex<AppState>>();
            let state = state.lock().unwrap();
            publish(&main, &state);
            tray::set_tooltip(&main, &state.panels, state.config.language);
            power::set_keep_awake(&main, state.config.keep_awake && any_pages(&state));
        });
        RESTORED.store(true, Ordering::SeqCst);
    });
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
    remember_panels_known(app);
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
    apply_layout(&app).map_err(|e| e.to_string())?;
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
        game::forget(&mut state, id);
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
    // The new page reports its connection again.
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        game::forget(&mut state, id);
        publish(&app, &state);
    }
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
        if state.config.focus.is_some() {
            state.last_focus = state.config.focus;
        }
        let was_economy = std::mem::replace(&mut state.config.economy, false);
        state.config.focus = if !exists || (state.config.focus == id && !was_economy) { None } else { id };
    }
    if let Err(e) = apply_layout(app) {
        eprintln!("[layout] {e}");
    }
    let state = app.state::<Mutex<AppState>>();
    save_and_publish(app, &state.lock().unwrap());
}

/// Closes the window of the page that asks (settings, patch notes): the page's own
/// window.close() only blanks the webview and leaves the window open.
#[tauri::command]
async fn close_window(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[tauri::command]
fn focus_slot(app: AppHandle, id: Option<u8>) {
    toggle_focus(&app, id);
}

/// A game or stream page reported a key combination (outside Windows, from `KEYS_SCRIPT`):
/// runs the shortcut bound to it, if any.
fn shortcut_from_page(app: &AppHandle, keys: &str) {
    let action = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        shortcuts::action_for(&state.config.shortcuts, keys.trim())
    };
    if let Some(action) = action {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { shortcuts::run(&app, &action) });
    }
}

/// Economy mode on or off (toolbar ECO button, the rest screen's button and the shortcut).
fn set_economy_mode(app: &AppHandle, on: bool) {
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        if state.config.economy == on {
            return;
        }
        state.config.economy = on;
        println!("[economy] {}", if on { "on: slots hidden, rest screen shown" } else { "off" });
    }
    if let Err(e) = apply_layout(app) {
        eprintln!("[layout] {e}");
    }
    let ids: Vec<u8> = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        save_and_publish(app, &state);
        state.config.slots.iter().map(|s| s.id).collect()
    };
    for id in ids {
        sync_game_economy(app, id, on);
    }
}

/// The game's own Battery Saver follows the app's economy mode in every hidden account (see
/// game-watch.js: it only undoes what the app turned on).
fn sync_game_economy(app: &AppHandle, id: u8, on: bool) {
    if let Some(game) = app.get_webview(&slot_label(id)) {
        let js = format!("window.__multiviewEco && window.__multiviewEco({on})");
        if let Err(e) = game.eval(&js) {
            eprintln!("[economy] slot {id}: could not reach the game's battery saver: {e}");
        }
    }
}

fn toggle_economy(app: &AppHandle) {
    let on = app.state::<Mutex<AppState>>().lock().unwrap().config.economy;
    set_economy_mode(app, !on);
}

#[tauri::command]
fn set_economy(app: AppHandle, on: bool) {
    set_economy_mode(&app, on);
}

/// The grid shortcut: from a fullscreen slot back to the grid, and from the grid to the slot
/// that was fullscreen last (or the first one).
fn toggle_grid(app: &AppHandle) {
    let target = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        if state.config.focus.is_some() {
            None
        } else {
            state
                .last_focus
                .filter(|id| state.config.slots.iter().any(|s| s.id == *id))
                .or_else(|| state.config.slots.first().map(|s| s.id))
        }
    };
    toggle_focus(app, target);
}

/// Reloads every slot's game page at once (the reload shortcut; the timed option in ⚙ does
/// the same on its own).
fn reload_all_games(app: &AppHandle) {
    let ids: Vec<u8> = app.state::<Mutex<AppState>>().lock().unwrap().config.slots.iter().map(|s| s.id).collect();
    println!("[reload] reloading {} game page(s) (shortcut)", ids.len());
    for id in ids {
        if let Some(webview) = app.get_webview(&slot_label(id)) {
            page_activity();
            let _ = webview.reload();
        }
    }
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
    if let Err(e) = apply_layout(&app) {
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
    if let Err(e) = apply_layout(&app) {
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
        if away::is_away() || main.is_minimized()? || !main.is_visible()? {
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
    // Tabs (General · Channels · Accounts · About): room for two account cards side by side.
    let (w, h) = (1180.0, 820.0);
    let lang = app.state::<Mutex<AppState>>().lock().unwrap().config.language;
    let builder =
        tauri::WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("settings.html".into()))
            .title(settings_title(lang))
            .inner_size(w, h);
    let builder = match centered_over(&main, w, h) {
        Some((x, y)) => builder.position(x, y),
        None => builder.center(),
    };
    let builder = match local_args() {
        Some(args) => builder.additional_browser_args(args),
        None => builder,
    };
    builder
        .resizable(true)
        .min_inner_size(720.0, 520.0)
        .minimizable(false)
        .maximizable(false)
        // Owned by the main window: stays in front of it and closes with it.
        .owner_raw(main.hwnd().map_err(|e| e.to_string())?)
        .build()
        .map_err(|e| {
            let e = e.to_string();
            runtime::page_failed(&app, &e);
            if e.contains("80010108") {
                "WebView2 was updated: the app restarts in a few seconds".to_string()
            } else {
                e
            }
        })?;
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
        // Let the restored streams open and load first.
        while !RESTORED.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
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

/// Records whether all of a platform's live checks failed; a change goes in every account's
/// log and on the pages (top bar, rest screen). A first round that works says nothing.
fn set_checks_down(app: &AppHandle, platform: Platform, down: Option<CheckDown>) {
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    let before = match down {
        Some(d) => state.checks_down.insert(platform, d),
        None => state.checks_down.remove(&platform),
    };
    if before == down {
        return;
    }
    if down.is_some() != before.is_some() {
        let text = match down {
            Some(_) => format!("{}: can't check which channels are live", platform.name()),
            None => format!("{}: live checks working again", platform.name()),
        };
        println!("[auto] {text}");
        let ids: Vec<u8> = state.config.slots.iter().map(|s| s.id).collect();
        for id in ids {
            log_event(&mut state, id, text.clone());
        }
    }
    publish(app, &state);
}

/// "Check again" on a live-checks warning: runs the stream manager now.
#[tauri::command]
fn check_streams_now() {
    AUTO_WAKE.store(true, Ordering::SeqCst);
}

/// One platform's turn: live checks, then each slot's streams of that platform.
fn manage_platform(app: &AppHandle, platform: Platform) {
    let (priority, per_slot, slots, previous, balances, logged_out) = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        // Accounts known not to be logged in on this platform: their lives would earn nothing.
        let logged_out: Vec<u8> =
            state.logins.iter().filter(|((_, p), on)| *p == platform && !**on).map(|((id, _), _)| *id).collect();
        let (priority, per_slot) = match platform {
            Platform::Kick => (state.config.kick_priority.clone(), KICK_LIVES_PER_SLOT),
            Platform::Twitch => (state.config.twitch_priority.clone(), usize::from(state.config.twitch_lives_per_slot)),
        };
        let previous = state.live.get(&platform).cloned().unwrap_or_default();
        (priority, per_slot, state.config.slots.clone(), previous, state.config.balances.clone(), logged_out)
    };
    if priority.is_empty() {
        set_checks_down(app, platform, None);
        return;
    }
    // A channel whose check fails keeps its last known status; with none known, the slots are
    // left as they are this time rather than closing streams on a network hiccup.
    let mut live = HashMap::new();
    let mut unknown = false;
    // Side by side over one shared client (see net.rs): ~1-2 s for the whole list instead of
    // one channel after another.
    let checks = net::check_all(&priority, |channel| is_live(platform, channel));
    // Every check failed (not one channel's hiccup): nothing gets opened or swapped on this
    // platform meanwhile, so it's said where the player looks, not only in app.log.
    let down = if checks.iter().all(Result::is_err) {
        checks.iter().find_map(|r| r.as_ref().err()).map(|e| CheckDown::of(e))
    } else {
        None
    };
    set_checks_down(app, platform, down);
    for (channel, result) in priority.iter().zip(checks) {
        match result {
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
    // Channels a page found offline before the platform did: dropped once the platform agrees
    // (or after a while); until then they are offline for that account.
    let page_offline: Vec<(u8, String)> = {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        state.page_offline.retain(|(_, p, channel), since| {
            *p != platform || (live.get(channel) == Some(&true) && since.elapsed() < PAGE_OFFLINE_FOR)
        });
        state
            .page_offline
            .keys()
            .filter(|(_, p, _)| *p == platform)
            .map(|(id, _, channel)| (*id, channel.clone()))
            .collect()
    };
    // Each account picks among the live channels by its own points (closest to its next reward
    // first), the priority order breaking ties.
    for slot in &slots {
        let open = open_channels(app, slot.id, platform);
        let points = |c: &str| balances.get(&config::balance_key(slot.id, platform, c)).copied().unwrap_or(0);
        let mut slot_live = live.clone();
        for (_, channel) in page_offline.iter().filter(|(id, _)| *id == slot.id) {
            println!("[auto] slot {}: {} {channel} looks offline on its page; picking another", slot.id, platform.name());
            slot_live.insert(channel.clone(), false);
        }
        let wanted = if logged_out.contains(&slot.id) {
            Vec::new()
        } else {
            kick::pick_for_account(&priority, &slot_live, per_slot, points, &open)
        };
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

#[derive(serde::Deserialize)]
struct FollowReport {
    following: bool,
}

/// A Twitch or Kick channel page (scripts/follow-watch.js) told whether this account follows
/// its channel: remembered (and saved) per account and channel, every pill of it updated, and
/// the settings lists too.
fn follow_reported(app: &AppHandle, id: u8, label: &str, json: &str) {
    let Ok(report) = serde_json::from_str::<FollowReport>(json) else { return };
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        let Some((platform, login)) = state
            .panels
            .get(&id)
            .and_then(|p| p.page(label))
            .and_then(|p| Some((p.platform, p.login.clone()?)))
        else {
            return;
        };
        if state.logins.get(&(id, platform)) == Some(&false) {
            return;
        }
        let key = config::balance_key(id, platform, &login);
        if state.config.follows.insert(key, report.following) == Some(report.following) {
            return;
        }
        if let Err(e) = state.config.save(&state.config_path) {
            eprintln!("[config] could not save: {e}");
        }
        if let Some(panel) = state.panels.get_mut(&id) {
            for page in panel.pages.iter_mut().filter(|p| p.platform == platform && p.login.as_deref() == Some(&login)) {
                page.not_following = !report.following;
            }
        }
        publish(app, &state);
    }
    // automation_view locks the state itself, so after the block above.
    let _ = app.emit_to("settings", "automation", automation_view(app));
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

/// A game page sent its hunt analysis (see game-watch.js): kept as-is for the stats panel,
/// which is refreshed if it shows this account.
fn stats_reported(app: &AppHandle, id: u8, json: &str) {
    let Ok(stats) = serde_json::from_str::<serde_json::Value>(json) else { return };
    if !stats.is_object() {
        return;
    }
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    discord::shinies_reported(&mut state, id, &stats);
    let heap = stats.get("heap").and_then(|h| h.as_u64());
    state.game_stats.insert(id, stats);
    if state.config.focus == Some(id) {
        let _ = app.emit_to(FOCUS_PANEL, "focus", focus_view(&state));
    }
    // The game page's memory (heavy.rs locks the state itself).
    drop(state);
    if let Some(mb) = heap {
        heavy::reading(app, id, &slot_label(id), mb);
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
    // A new vote counted since the last check goes into the account's log.
    let before = state.vote.get(&id).map(|(s, _)| (s.votes, s.diamonds));
    if let Some((votes, diamonds)) = before {
        if status.votes > votes {
            let diamond = if status.diamonds > diamonds { " (+1 diamond)" } else { "" };
            log_event(&mut state, id, format!("IdleHub vote counted{diamond}"));
        }
    }
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
    let (url, nick, lang) = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        let status = state.vote.get(&id).map(|(s, _)| s.clone()).ok_or("No vote status yet")?;
        (status.url, status.nick, state.config.language)
    };
    let parsed: Url = url.parse().map_err(|_| "Bad vote link")?;
    if parsed.scheme() != "https" || !parsed.host_str().is_some_and(is_idlehub) {
        return Err("The vote link isn't an idlehub.io page".into());
    }
    let main = app.get_window("main").ok_or("No window")?;
    let (w, h) = (560.0, 820.0);
    let builder = WindowBuilder::new(&app, &label)
        .title(format!("{} — {nick}", lang.pick("Vote on IdleHub", "Votar no IdleHub", "Votar en IdleHub")))
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
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().unwrap();
    let text = match report.result.as_str() {
        "claimed" => "Kick Daily Reward claimed".to_string(),
        "not available" => "Kick Daily Reward already claimed today".to_string(),
        other => format!("Kick Daily Reward: {other}"),
    };
    log_event(&mut state, id, text);
    if report.result == "claimed" || report.result == "not available" {
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
    /// Checks in a row where the page said its channel is offline.
    offline_reports: u8,
    /// The page was reloaded once because it had no player while the platform said live.
    offline_reloaded: bool,
}

/// What stream-health.js says when Twitch's player shows its "offline" screen.
const TWITCH_OFFLINE_SCREEN: &str = "The Twitch player says the channel is offline";

/// After this many "offline" checks in a row, the page is believed over the platform's
/// live status (which can lag for minutes) and its channel is swapped.
const OFFLINE_REPORTS_TO_SWAP: u8 = 2;
/// How long a channel a page found offline is kept out of that account's picks, unless the
/// platform also says offline or the page plays again first.
const PAGE_OFFLINE_FOR: std::time::Duration = std::time::Duration::from_secs(15 * 60);

/// Asks every background stream page whether it is playing (scripts/stream-health.js); the
/// answers come to `health_reported`. Pages in the middle of a Twitch visit are skipped.
/// How often every background stream is asked whether it's playing (stream-health.js).
const HEALTH_EVERY: std::time::Duration = std::time::Duration::from_secs(30);
/// After a page says "offline", it's asked again this soon, to confirm and swap it quickly.
const OFFLINE_RECHECK: std::time::Duration = std::time::Duration::from_secs(10);

/// Health checks on their own clock, more often than the manager's 2-minute pass: an ended
/// stream is noticed within ~30 s instead of minutes.
fn start_health_checks(app: AppHandle) {
    std::thread::spawn(move || {
        while !RESTORED.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        loop {
            std::thread::sleep(HEALTH_EVERY);
            check_stream_health(&app);
        }
    });
}

fn check_stream_health(app: &AppHandle) {
    let visiting = app.state::<Mutex<AppState>>().lock().unwrap().visiting.clone();
    for platform in PLATFORMS {
        for (_, label, _, _) in background_channel_pages(app, platform, true) {
            if !visiting.contains(&label) {
                check_page_health(app, &label);
            }
        }
    }
}

/// Starts every background stream whose video is paused (muted, so the browser allows it).
pub(crate) fn resume_paused_streams(app: &AppHandle) {
    // Kick's stream is #video-player (its first <video> is a decoration); Twitch's player has one.
    const PLAY: &str = "(() => { const v = document.getElementById('video-player') || document.querySelector('video'); if (v && v.paused) v.play().catch(() => {}); })();";
    let pages: Vec<String> = app
        .state::<Mutex<AppState>>()
        .lock()
        .unwrap()
        .panels
        .values()
        .flat_map(|p| p.pages.iter().map(|pg| pg.label.clone()))
        .collect();
    for label in pages {
        if let Some(page) = app.get_webview(&label) {
            let _ = page.eval(PLAY);
        }
    }
}

fn check_page_health(app: &AppHandle, label: &str) {
    if let Some(page) = app.get_webview(label) {
        let _ = page.eval(format!("{}\n{STREAM_HEALTH}", kick::REPORT));
    }
}

#[derive(serde::Deserialize)]
struct HealthReport {
    state: String,
    #[serde(default)]
    detail: String,
    /// The page's JavaScript memory in MB (see heavy.rs).
    #[serde(default)]
    heap: Option<u64>,
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
            track.offline_reports = 0;
            track.offline_reloaded = false;
        }
        "logged-in" => track.logged_out = None,
        "logged-out" => track.logged_out = Some(format!("Logged out: {detail}")),
        "offline" => {
            track.offline = Some(format!("Offline: {detail}"));
            track.offline_reports = track.offline_reports.saturating_add(1);
        }
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
    // "slot-3-twitch-12" → slot 3: its memory reading (pages that swell get reloaded).
    if let (Some(mb), Some(id)) = (report.heap, label.split('-').nth(1).and_then(|n| n.parse::<u8>().ok())) {
        heavy::reading(app, id, label, mb);
    }
    let (reload, changed, wake, offline_count) = {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        let track = state.health.entry(label.to_string()).or_default();
        let Some((reload, warning)) =
            next_health(track, &report.state, &report.detail, std::time::Instant::now())
        else {
            return;
        };
        let offline_reports = track.offline_reports;
        let offline_reloaded = track.offline_reloaded;
        let mut reload = reload;
        // Which account and channel this page is.
        let owner = state.panels.iter().find_map(|(id, p)| {
            let page = p.page(label)?;
            Some((*id, page.platform, page.login.clone()?))
        });
        // An offline channel is swapped sooner: the first report wakes the manager (the
        // platform may already agree); from the second one on, the page is believed even if
        // the platform still says live, and that account picks another channel. Playing
        // again lets it back in.
        let mut wake = false;
        // Only channels the priority lists manage (a stream opened by hand is just warned).
        let managed = |platform: Platform, login: &str| match platform {
            Platform::Kick => state.config.kick_priority.iter().any(|c| c == login),
            Platform::Twitch => state.config.twitch_priority.iter().any(|c| c == login),
        };
        let who = owner.clone();
        let owner = owner.filter(|(_, platform, login)| managed(*platform, login));
        if let Some((id, platform, login)) = owner {
            let key = (id, platform, login);
            match report.state.as_str() {
                "offline" => {
                    // Only the first reports wake the manager (and get the quick recheck): a
                    // page that stays offline is handled by then, and mustn't wake it forever.
                    wake = offline_reports <= OFFLINE_REPORTS_TO_SWAP;
                    if offline_reports >= OFFLINE_REPORTS_TO_SWAP {
                        // The player's own "offline" screen is a sure sign. A page that just
                        // has no player while the platform still says live may be stuck
                        // instead (a Kick page that loaded in the hidden window never mounts
                        // it): it's reloaded once first, and believed if it's still empty.
                        let sure = report.detail == TWITCH_OFFLINE_SCREEN;
                        let platform_live = state.live.get(&key.1).and_then(|m| m.get(&key.2)) == Some(&true);
                        if !sure && platform_live && !offline_reloaded {
                            println!("[health] {label}: no player but {} says {} is live; reloading it once", key.1.name(), key.2);
                            if let Some(track) = state.health.get_mut(label) {
                                track.offline_reloaded = true;
                                track.offline_reports = 0;
                            }
                            reload = true;
                        } else {
                            state.page_offline.entry(key).or_insert_with(std::time::Instant::now);
                        }
                    }
                }
                "ok" => {
                    state.page_offline.remove(&key);
                }
                _ => {}
            }
        }
        let page = state.panels.values_mut().flat_map(|p| p.pages.iter_mut()).find(|p| p.label == label);
        let changed = match page {
            Some(page) if page.warning != warning => {
                page.warning = warning.clone();
                true
            }
            _ => false,
        };
        // Into the account's log: reloads and warnings coming and going.
        if let Some((id, platform, login)) = who {
            let channel = format!("{} {login}", platform.name());
            if reload {
                let why = if report.state == "offline" { "no player, reloaded" } else { "stalled, reloaded" };
                log_event(&mut state, id, format!("{channel}: {why}"));
            }
            if changed {
                if let Some(w) = &warning {
                    discord::stream_warning(&state, id, &channel, w);
                }
                let text = match &warning {
                    Some(w) => format!("{channel}: ⚠ {w}"),
                    None => format!("{channel}: playing normally again"),
                };
                log_event(&mut state, id, text);
            }
        }
        (reload, changed, wake, offline_reports)
    };
    if wake {
        println!("[health] {label}: offline ({}); checking the priority lists now", report.detail);
        AUTO_WAKE.store(true, Ordering::SeqCst);
        // After the first "offline", asked again shortly: a second one swaps the channel even if
        // the platform still says live, without waiting for the next round of checks.
        if offline_count < OFFLINE_REPORTS_TO_SWAP {
            let app = app.clone();
            let label = label.to_string();
            std::thread::spawn(move || {
                std::thread::sleep(OFFLINE_RECHECK);
                check_page_health(&app, &label);
            });
        }
    }
    if reload {
        println!("[health] {label}: {} ({}); reloading", report.state, report.detail);
        if let Some(page) = app.get_webview(label) {
            page_activity();
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
    fn offline_reports_are_counted_until_the_stream_plays() {
        let mut t = HealthTrack::default();
        let now = Instant::now();
        next_health(&mut t, "offline", "x", now);
        assert_eq!(t.offline_reports, 1);
        next_health(&mut t, "offline", "x", now);
        assert!(t.offline_reports >= OFFLINE_REPORTS_TO_SWAP);
        // Other signals don't reset it; playing again does.
        next_health(&mut t, "logged-in", "", now);
        assert_eq!(t.offline_reports, 2);
        next_health(&mut t, "ok", "", now);
        assert_eq!(t.offline_reports, 0);
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
        let closed: Vec<String> = pages
            .iter()
            .filter(|p| close.contains(&p.label))
            .filter_map(|p| p.login.clone())
            .collect();
        {
            let state = app.state::<Mutex<AppState>>();
            log_event(&mut state.lock().unwrap(), id, format!("{} closed: {}", platform.name(), closed.join(", ")));
        }
        stop_pages_inner(app, id, Some(&close))?;
    }
    for login in missing {
        let channel = channels::Channel { platform, login: login.clone() };
        println!("[auto] slot {id}: opening {} {login}", platform.name());
        {
            let state = app.state::<Mutex<AppState>>();
            log_event(&mut state.lock().unwrap(), id, format!("{} opened: {login}", platform.name()));
        }
        create_page(app, id, platform, &channel.url_for(false), true)?;
    }
    remember_panels_known(app);
    apply_layout(app)
}

/// Background pages of `platform` on a channel, per account with a bonus chosen (or every
/// account with `all`), leaving out the page being shown: (slot, label, login, bonus).
fn background_channel_pages(app: &AppHandle, platform: Platform, all: bool) -> Vec<(u8, String, String, Option<KickBonus>)> {
    let state = app.state::<Mutex<AppState>>();
    let state = state.lock().unwrap();
    let mut out = Vec::new();
    for slot in &state.config.slots {
        if !all && slot.bonus.is_none() && slot.goals.is_none() {
            continue;
        }
        let bonus = bonus_now(&state, slot, platform);
        let Some(panels) = state.panels.get(&slot.id) else { continue };
        for page in &panels.pages {
            let Some(login) = &page.login else { continue };
            if page.platform == platform && panels.front.as_deref() != Some(page.label.as_str()) {
                out.push((slot.id, page.label.clone(), login.clone(), bonus));
            }
        }
    }
    out
}

/// After a redeem, goals mode waits this long at most for the game to report the new time.
const REDEEM_SETTLE: std::time::Duration = std::time::Duration::from_secs(180);

/// The bonus an account's points buy now on `platform`: its fixed bonus, or in goals mode
/// the first goal not met there (None = nothing to buy: every goal met and points saved, the
/// game hasn't reported the times yet, or a redeem is still being counted).
fn bonus_now(state: &AppState, slot: &SlotConfig, platform: Platform) -> Option<KickBonus> {
    let Some(goals) = &slot.goals else { return slot.bonus };
    let ends = state.bonus_ends.get(&(slot.id, platform))?;
    if state.redeemed.get(&(slot.id, platform)).is_some_and(|at| at.elapsed() < REDEEM_SETTLE) {
        return None;
    }
    let now = now_ms();
    goals::pick(goals, |bonus| {
        let end = match bonus {
            KickBonus::Xp => ends.xp,
            KickBonus::Capture => ends.capture,
            KickBonus::Lure => ends.lure,
        };
        goals::hours_left(end, now)
    })
}

/// Asks the live Kick pages of accounts that redeem to buy their bonus if the points are
/// enough (scripts/kick-redeem.js does nothing otherwise; Kick pages show their points even in
/// the background). In goals mode only the account's channel with the most points tries, so
/// one pass buys one package and the next decision sees the new time.
fn redeem_kick_bonuses(app: &AppHandle) {
    let pages = background_channel_pages(app, Platform::Kick, false);
    let (goal_slots, balances): (Vec<u8>, HashMap<String, u64>) = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        (
            state.config.slots.iter().filter(|s| s.goals.is_some()).map(|s| s.id).collect(),
            state.config.balances.clone(),
        )
    };
    let points = |id: u8, login: &str| balances.get(&config::balance_key(id, Platform::Kick, login)).copied().unwrap_or(0);
    let mut chosen: Vec<&(u8, String, String, Option<KickBonus>)> = Vec::new();
    for page in &pages {
        let (id, _, login, _) = page;
        if !goal_slots.contains(id) {
            chosen.push(page);
            continue;
        }
        match chosen.iter().position(|p| p.0 == *id) {
            Some(i) if points(*id, login) > points(*id, &chosen[i].2) => chosen[i] = page,
            Some(_) => {}
            None => chosen.push(page),
        }
    }
    for (_, label, _, bonus) in chosen {
        let (Some(page), Some(bonus)) = (app.get_webview(label), bonus) else { continue };
        if let Err(e) = page.eval(kick::redeem_script(*bonus)) {
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
        page_activity();
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
        slot.and_then(|id| state.config.slots.iter().find(|s| s.id == id))
            .and_then(|s| bonus_now(&state, s, Platform::Twitch))
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
    goals: Option<config::Goals>,
    /// What the next redeem buys on each platform (None = nothing now).
    next: NextRedeem,
    /// Whether the account has Kick / Twitch linked in the game (None = not reported yet).
    kick_linked: Option<bool>,
    twitch_linked: Option<bool>,
    /// Logged in on Kick / Twitch in this slot (None = not checked yet).
    kick_login: Option<bool>,
    twitch_login: Option<bool>,
    /// Last seen channel points per channel, most first: [platform, channel, points].
    points: Vec<(Platform, String, u64)>,
    /// The account's running bonuses per platform.
    running: SlotBonuses,
}

#[derive(Serialize, Clone, Copy, Default, Debug)]
struct NextRedeem {
    kick: Option<KickBonus>,
    twitch: Option<KickBonus>,
}

fn next_redeem(state: &AppState, slot: &SlotConfig) -> NextRedeem {
    NextRedeem { kick: bonus_now(state, slot, Platform::Kick), twitch: bonus_now(state, slot, Platform::Twitch) }
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
    /// Per channel, the accounts known not to follow it.
    not_following: HashMap<String, Vec<String>>,
    /// Per channel, the accounts that haven't opened it yet (whether they follow is unknown).
    unseen: HashMap<String, Vec<String>>,
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
    // Who doesn't follow (or hasn't opened yet) each channel of a priority list.
    let follows = |p: Platform, list: &[String]| {
        let mut not: HashMap<String, Vec<String>> = HashMap::new();
        let mut unseen: HashMap<String, Vec<String>> = HashMap::new();
        for channel in list {
            for slot in &state.config.slots {
                match state.config.follows.get(&config::balance_key(slot.id, p, &channel.to_ascii_lowercase())) {
                    Some(true) => {}
                    Some(false) => not.entry(channel.clone()).or_default().push(slot_name(slot.id)),
                    None => unseen.entry(channel.clone()).or_default().push(slot_name(slot.id)),
                }
            }
        }
        (not, unseen)
    };
    let (kick_not, kick_unseen) = follows(Platform::Kick, &state.config.kick_priority);
    let (twitch_not, twitch_unseen) = follows(Platform::Twitch, &state.config.twitch_priority);
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
            not_following: kick_not,
            unseen: kick_unseen,
        },
        twitch: PriorityView {
            priority: state.config.twitch_priority.clone(),
            live: live(Platform::Twitch),
            lives_per_slot: usize::from(state.config.twitch_lives_per_slot),
            sync: state.config.twitch_sync,
            partners: partners(Platform::Twitch),
            watching: watching(Platform::Twitch),
            not_following: twitch_not,
            unseen: twitch_unseen,
        },
        slots: state
            .config
            .slots
            .iter()
            .map(|s| BonusSlotView {
                id: s.id,
                name: s.name.clone().unwrap_or_else(|| format!("Slot {}", s.id)),
                bonus: s.bonus,
                goals: s.goals.clone(),
                next: next_redeem(&state, s),
                kick_linked: state.linked.get(&(s.id, Platform::Kick)).copied(),
                twitch_linked: state.linked.get(&(s.id, Platform::Twitch)).copied(),
                kick_login: state.logins.get(&(s.id, Platform::Kick)).copied(),
                twitch_login: state.logins.get(&(s.id, Platform::Twitch)).copied(),
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

/// When the app last opened, reloaded or navigated a page by itself. WebView2 can hand the
/// window focus when that happens; while the window is away (minimized or in the tray), focus
/// that close to it isn't the player asking for the window, so it doesn't bring it back.
static PAGE_ACTIVITY: Mutex<Option<std::time::Instant>> = Mutex::new(None);
const PAGE_ACTIVITY_QUIET: std::time::Duration = std::time::Duration::from_secs(3);

fn page_activity() {
    *PAGE_ACTIVITY.lock().unwrap() = Some(std::time::Instant::now());
}

fn after_page_activity() -> bool {
    PAGE_ACTIVITY.lock().unwrap().is_some_and(|t| t.elapsed() < PAGE_ACTIVITY_QUIET)
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
        // The game counted a redeem: goals mode can decide again with the new time.
        if bonus_changed {
            state.redeemed.remove(&(id, report.platform));
        }
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

/// At most this many goals per account.
const MAX_GOALS: usize = 6;

/// An account's redeem goals (None = back to its fixed bonus). Hours stay within 1–999.
#[tauri::command]
fn set_goals(app: AppHandle, id: u8, goals: Option<config::Goals>) -> AutomationView {
    let goals = goals.map(|mut g| {
        g.list.truncate(MAX_GOALS);
        for goal in &mut g.list {
            goal.hours = goal.hours.clamp(1, 999);
        }
        g
    });
    automation_changed(&app, |state| {
        if let Some(slot) = state.config.slots.iter_mut().find(|s| s.id == id) {
            slot.goals = goals;
        }
    })
}

/// "Copy to all accounts": every account gets this one's way of redeeming (goals or fixed).
#[tauri::command]
fn copy_redeem_to_all(app: AppHandle, id: u8) -> AutomationView {
    automation_changed(&app, |state| {
        let Some(from) = state.config.slots.iter().find(|s| s.id == id).cloned() else { return };
        for slot in &mut state.config.slots {
            slot.bonus = from.bonus;
            slot.goals = from.goals.clone();
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
                    page_activity();
                    let _ = webview.reload();
                }
            }
        }
    });
}

// ---------------------------------------------------------------- startup

fn main() {
    applog::catch_panics();
    tauri::Builder::default()
        // Must be the first plugin. Two copies would share the slots' data folders and run every
        // background stream twice, so a second launch exits and brings this window forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            println!("[app] another copy was started; showing this one instead");
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(updater::UpdateState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            let base = app.path().app_local_data_dir()?;
            applog::start(&base);
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
            // "One browser for all accounts" (0.19.0) was removed: its folder isn't read anymore.
            let old_shared = base.join("slots").join("shared");
            if old_shared.is_dir() {
                std::thread::spawn(move || {
                    if std::fs::remove_dir_all(&old_shared).is_ok() {
                        println!("[app] removed the unused shared browser folder");
                    }
                });
            }
            let slots = config.slots.clone();
            let geom = config.window.unwrap_or_default();
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
                page_offline: HashMap::new(),
                daily_tries: HashMap::new(),
                vote: HashMap::new(),
                log: HashMap::new(),
                game_stats: HashMap::new(),
                redeemed: HashMap::new(),
                last_focus: None,
                logins: HashMap::new(),
                login_pages: std::collections::HashSet::new(),
                game_conn: HashMap::new(),
                sprites: HashMap::new(),
                discord_sent: discord::Sent::default(),
                prey: HashMap::new(),
                checks_down: HashMap::new(),
            }));

            // Hidden until the games have loaded (splash.rs shows it); the splash window is
            // on screen meanwhile.
            splash::open(&handle, &slots);
            let window = WindowBuilder::new(app, "main")
                .title("PokeIdle Multiview")
                .visible(false)
                .background_color(BACKGROUND)
                // The last size; centered (and maximized if it was) when the loading ends.
                .inner_size(geom.width.max(640.0), geom.height.max(400.0))
                .min_inner_size(640.0, 400.0)
                .build()?;
            for slot in &slots {
                create_slot_webview(&handle, slot)?;
            }
            for slot in &slots {
                create_slot_header(&handle, slot.id)?;
            }
            // The two splitter strips (local page, same environment as the toolbar); placed
            // over the gaps between slots by apply_layout.
            for (label, axis) in [(SPLIT_V, "v"), (SPLIT_H, "h")] {
                let splitter = window.add_child(
                    local_page(label, format!("splitter.html?axis={axis}")),
                    LogicalPosition::new(0.0, BAR_HEIGHT),
                    LogicalSize::new(1.0, 1.0),
                )?;
                splitter.hide()?;
            }
            // The stats panel of a fullscreen slot (local page); placed by apply_layout.
            window
                .add_child(
                    local_page(FOCUS_PANEL, "focus.html".into()),
                    LogicalPosition::new(0.0, BAR_HEIGHT),
                    LogicalSize::new(1.0, 1.0),
                )?
                .hide()?;
            // The rest screen of economy mode (local page); placed by apply_layout.
            window
                .add_child(
                    local_page(REST_PAGE, "rest.html".into()),
                    LogicalPosition::new(0.0, BAR_HEIGHT),
                    LogicalSize::new(1.0, 1.0),
                )?
                .hide()?;
            // Toolbar last, over the other pages; it gets the streams once they reopen.
            window.add_child(
                local_page("toolbar", "index.html".into()),
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(1280.0, BAR_HEIGHT),
            )?;
            apply_layout(&handle)?;
            // The saved streams reopen after this, in the background (see restore_panels).
            restore_panels(&handle, &slots);

            tray::create(&handle)?;
            updater::start(handle.clone());
            tray::update_tooltip(&handle);
            {
                let state = handle.state::<Mutex<AppState>>();
                let state = state.lock().unwrap();
                power::set_keep_awake(&handle, state.config.keep_awake && any_pages(&state));
            }
            away::watch_other_windows();
            start_auto_reload(handle.clone());
            resources::start(handle.clone());
            logins::start(handle.clone());
            discord::start(handle.clone());
            start_stream_manager(handle.clone());
            start_health_checks(handle.clone());

            let event_handle = handle.clone();
            let event_window = window.clone();
            // Minimizing or hiding to the tray sends the window away (off-screen) instead, so the
            // streams keep playing: see away.rs.
            window.on_window_event(move |event| match event {
                WindowEvent::Resized(_) => {
                    let minimized = event_window.is_minimized().unwrap_or(false);
                    if minimized && !away::moving() {
                        let (to_tray, quitting) = tray::settings(&event_handle);
                        if !quitting {
                            if away::is_away() && !away::in_tray() {
                                // Its taskbar button was clicked while it was the active window
                                // (Windows then minimizes it): it comes back.
                                away::bring_back(&event_window);
                            } else {
                                away::send(&event_window, to_tray.minimize || away::in_tray());
                            }
                            return;
                        }
                    }
                    if !minimized {
                        let _ = apply_layout(&event_handle);
                    }
                }
                // Its taskbar button clicked, or picked with Alt+Tab, while away. Focus that
                // comes right after the app opened, reloaded or navigated a page by itself is
                // WebView2's, not the player's: the window stays away.
                // Focus that Windows hands on because another app's window was minimized isn't
                // the player's either. The minimize event can arrive just after the focus, so
                // the choice is made a moment later.
                WindowEvent::Focused(true) if away::is_away() && !away::moving() => {
                    if after_page_activity() {
                        println!("[window] focus right after a page change while away: staying away");
                        return;
                    }
                    let app = event_handle.clone();
                    let window = event_window.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(200));
                        let _ = app.run_on_main_thread(move || {
                            if !away::is_away() {
                                return;
                            }
                            if away::focus_is_handoff() {
                                println!("[window] focus handed on after another window was minimized: staying away");
                                away::refuse_focus(&window);
                            } else {
                                away::bring_back(&window);
                            }
                        });
                    });
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
                        away::send(&event_window, true);
                    }
                }
                _ => {}
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            check_streams_now,
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
            close_window,
            set_economy,
            logins::open_login,
            discord::get_discord,
            applog::open_logs_folder,
            applog::diagnostics,
            prey::dismiss_prey,
            discord::set_discord,
            discord::test_discord,
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
            set_goals,
            copy_redeem_to_all,
            set_sync,
            set_vote_slot,
            open_vote,
            get_focus,
            set_focus_panel,
            get_language,
            set_language,
            support::support_info,
            support::open_support,
            shortcuts::get_shortcuts,
            shortcuts::run_shortcut,
            shortcuts::set_shortcut,
            shortcuts::reset_shortcuts,
            whatsnew::get_whats_new,
            whatsnew::open_whats_new,
            support::open_github,
            support::open_referral,
            support::open_discord,
            updater::get_update,
            updater::check_update,
            updater::install_update,
            splash::get_splash
        ])
        .run(tauri::generate_context!())
        .expect("error while running PokeIdle Multiview");
}
