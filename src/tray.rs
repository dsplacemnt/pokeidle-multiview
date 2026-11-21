//! Tray icon: the app keeps running there (with its background streams) when the window is
//! closed or minimized. Left click or "Show" brings the window back; "Quit" really exits.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::config::Language;
use crate::{remember_panels, AppState, PanelState};

const TRAY_ID: &str = "main";

/// Set by "Quit", so closing the window then exits instead of hiding to the tray.
static QUITTING: AtomicBool = AtomicBool::new(false);

pub struct ToTray {
    pub close: bool,
    pub minimize: bool,
}

/// Current tray options from the config, and whether the app is quitting.
pub fn settings(app: &AppHandle) -> (ToTray, bool) {
    let state = app.state::<Mutex<AppState>>();
    let state = state.lock().unwrap();
    let to_tray = ToTray {
        close: state.config.close_to_tray,
        minimize: state.config.minimize_to_tray,
    };
    (to_tray, QUITTING.load(Ordering::SeqCst))
}

/// Shows the main window again, from the tray or from a second copy of the app.
pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_window("main") {
        if crate::away::bring_back(&window) {
            return;
        }
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn quit(app: &AppHandle) {
    QUITTING.store(true, Ordering::SeqCst);
    remember_panels(app);
    // The window-state plugin saves the window geometry on exit.
    app.exit(0);
}

fn menu(app: &AppHandle, lang: Language) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", lang.pick("Show PokeIdle Multiview", "Mostrar o PokeIdle Multiview", "Mostrar PokeIdle Multiview"), true, None::<&str>)?;
    let support = MenuItem::with_id(app, "support", lang.pick("♥ Support the dev", "♥ Apoiar o dev", "♥ Apoyar al dev"), true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", lang.pick("Quit", "Sair", "Salir"), true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    Menu::with_items(app, &[&show, &support, &separator, &quit_item])
}

/// The language changed: the menu is rebuilt and the tooltip rewritten.
pub fn set_language(app: &AppHandle) {
    let lang = app.state::<Mutex<AppState>>().lock().unwrap().config.language;
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), menu(app, lang)) {
        let _ = tray.set_menu(Some(menu));
    }
    update_tooltip(app);
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let lang = app.state::<Mutex<AppState>>().lock().unwrap().config.language;
    let menu = menu(app, lang)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("PokeIdle Multiview")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main(app),
            "support" => {
                show_main(app);
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = crate::support::open_support(app).await {
                        eprintln!("[support] {e}");
                    }
                });
            }
            "quit" => quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// "PokeIdle Multiview — 8 streams running", so it's clear the app is still working.
/// Callers often hold the app state, and changing the tray waits for the UI thread: the change
/// is posted to that thread instead of waited for (waiting there froze the app).
pub fn set_tooltip(app: &AppHandle, panels: &HashMap<u8, PanelState>, lang: Language) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let streams: usize = panels.values().map(|p| p.pages.len()).sum();
    let text = match streams {
        0 => "PokeIdle Multiview".to_string(),
        1 => lang.pick("PokeIdle Multiview — 1 stream running", "PokeIdle Multiview — 1 stream rodando", "PokeIdle Multiview — 1 stream activo").to_string(),
        n => format!("PokeIdle Multiview — {n} {}", lang.pick("streams running", "streams rodando", "streams activos")),
    };
    let _ = app.run_on_main_thread(move || {
        let _ = tray.set_tooltip(Some(text));
    });
}

pub fn update_tooltip(app: &AppHandle) {
    let state = app.state::<Mutex<AppState>>();
    let (panels, lang) = {
        let state = state.lock().unwrap();
        (state.panels.clone(), state.config.language)
    };
    set_tooltip(app, &panels, lang);
}
