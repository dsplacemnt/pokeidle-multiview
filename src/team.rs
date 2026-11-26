//! Team analysis window (the Pokéball button left of ⚙): every account's Pokémon list, as the
//! game pages last reported it (game::mons_reported keeps it in `pokemon/slot-N.json`), ranked
//! with the game's own formulas (ui/team-analysis.js). The game's public data those formulas
//! need (species, move sheets, Megas, branching evolutions) is downloaded from pokeidle.io and
//! kept in `gamedata/`, refreshed once a day; it's handed to the page as text and only ever
//! parsed there, never run.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl};

#[cfg(not(windows))]
use crate::compat::{HwndCompat, OwnerCompat};

pub const WINDOW: &str = "team";

/// The game's public files the analysis reads.
const GAME_FILES: [&str; 7] = [
    "assets/creatures.json",
    "assets/creatures-novos.json",
    "assets/creatures-formas.json",
    "assets/creatures-audit-overrides.json",
    "shared/golpes-planilhas.mjs",
    "shared/megas.mjs",
    "shared/evolucoes-ramificadas.mjs",
];
const REFRESH_AFTER: Duration = Duration::from_secs(24 * 3600);
const MAX_FILE: u64 = 32 * 1024 * 1024;

pub fn title(lang: crate::config::Language) -> &'static str {
    lang.pick("Team analysis — PokeIdle Multiview", "Análise do time — PokeIdle Multiview", "Análisis del equipo — PokeIdle Multiview")
}

/// Opens (or brings back) the window.
#[tauri::command]
pub async fn open_team(app: AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(WINDOW) {
        let _ = existing.unminimize();
        return existing.set_focus().map_err(|e| e.to_string());
    }
    let main = app.get_window("main").ok_or("No window")?;
    let (w, h) = (1080.0, 720.0);
    let lang = app.state::<Mutex<crate::AppState>>().lock().unwrap().config.language;
    let builder = tauri::WebviewWindowBuilder::new(&app, WINDOW, WebviewUrl::App("team.html".into()))
        .title(title(lang))
        .inner_size(w, h)
        .min_inner_size(900.0, 520.0);
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
        .owner_raw(main.hwnd().map_err(|e| e.to_string())?)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize)]
pub struct TeamSlot {
    id: u8,
    name: String,
    /// "both" | "game" | "streams"
    mode: &'static str,
    /// The last list the game reported (None: none yet).
    list: Option<serde_json::Value>,
}

fn data_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_local_data_dir().ok()
}

/// Every slot with its saved Pokémon list.
#[tauri::command]
pub async fn get_team_lists(app: AppHandle) -> Vec<TeamSlot> {
    let slots: Vec<(u8, String, &'static str)> = {
        let state = app.state::<Mutex<crate::AppState>>();
        let state = state.lock().unwrap();
        state
            .config
            .slots
            .iter()
            .map(|s| {
                let mode = match s.mode() {
                    crate::config::SlotMode::Both => "both",
                    crate::config::SlotMode::Game => "game",
                    crate::config::SlotMode::Streams => "streams",
                };
                (s.id, s.name.clone().unwrap_or_else(|| format!("Slot {}", s.id)), mode)
            })
            .collect()
    };
    let dir = data_dir(&app).map(|d| d.join("pokemon"));
    slots
        .into_iter()
        .map(|(id, name, mode)| {
            let list = dir
                .as_ref()
                .and_then(|d| std::fs::read_to_string(d.join(format!("slot-{id}.json"))).ok())
                .and_then(|t| serde_json::from_str(&t).ok());
            TeamSlot { id, name, mode, list }
        })
        .collect()
}

#[derive(Serialize)]
pub struct GameData {
    /// Path (as in GAME_FILES) → the file's text.
    files: std::collections::HashMap<String, String>,
    /// Files that couldn't be downloaded and have no saved copy.
    missing: Vec<String>,
    /// Some file is an old copy (no internet when it was due).
    stale: bool,
}

/// The game's public data for the analysis: the saved copy when it's less than a day old,
/// otherwise downloaded again (an older copy is used when the download fails).
#[tauri::command]
pub async fn get_game_data(app: AppHandle, refresh: Option<bool>) -> Result<GameData, String> {
    let dir = data_dir(&app).ok_or("No data folder")?.join("gamedata");
    let force = refresh.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || load_game_data(&dir, force)).await.map_err(|e| e.to_string())
}

fn load_game_data(dir: &PathBuf, force: bool) -> GameData {
    let _ = std::fs::create_dir_all(dir);
    let results = crate::net::check_all(&GAME_FILES, |rel| {
        let file = dir.join(rel.replace('/', "_"));
        let age = std::fs::metadata(&file).and_then(|m| m.modified()).ok().and_then(|t| SystemTime::now().duration_since(t).ok());
        let fresh = !force && age.is_some_and(|a| a < REFRESH_AFTER);
        if !fresh {
            match download(rel) {
                Ok(text) => {
                    let _ = std::fs::write(&file, &text);
                    return (rel.to_string(), Some(text), false);
                }
                Err(e) => println!("[team] couldn't download {rel}: {e}"),
            }
        }
        let saved = std::fs::read_to_string(&file).ok();
        let stale = !fresh && saved.is_some();
        (rel.to_string(), saved, stale)
    });
    let mut data = GameData { files: Default::default(), missing: Vec::new(), stale: false };
    for (rel, text, stale) in results {
        data.stale |= stale;
        match text {
            Some(t) => {
                data.files.insert(rel, t);
            }
            None => data.missing.push(rel),
        }
    }
    data
}

fn download(rel: &str) -> Result<String, String> {
    crate::net::agent()
        .get(&format!("https://pokeidle.io/{rel}"))
        .header("User-Agent", crate::net::USER_AGENT)
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .with_config()
        .limit(MAX_FILE)
        .read_to_string()
        .map_err(|e| e.to_string())
}

/// A slot's list was just saved: the open window reloads it.
pub fn list_saved(app: &AppHandle, id: u8) {
    let _ = app.emit_to(WINDOW, "team-list", id);
}
