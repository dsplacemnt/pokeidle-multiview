//! Tray icon: the app keeps running there (with its background streams) when the window is
//! closed or minimized. Left click or "Show" brings the window back; "Quit" really exits.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

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

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show PokeIdle Multiview", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &separator, &quit_item])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("PokeIdle Multiview")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main(app),
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
pub fn set_tooltip(app: &AppHandle, panels: &HashMap<u8, PanelState>) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let streams: usize = panels.values().map(|p| p.running.len()).sum();
    let text = match streams {
        0 => "PokeIdle Multiview".to_string(),
        1 => "PokeIdle Multiview — 1 stream running".to_string(),
        n => format!("PokeIdle Multiview — {n} streams running"),
    };
    let _ = tray.set_tooltip(Some(text));
}

pub fn update_tooltip(app: &AppHandle) {
    let state = app.state::<Mutex<AppState>>();
    let panels = state.lock().unwrap().panels.clone();
    set_tooltip(app, &panels);
}
