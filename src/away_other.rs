//! Outside Windows: minimizing and the tray do what they normally do. (On Windows, away.rs
//! moves the window off-screen instead, because WebView2 stops drawing a minimized window;
//! whether WebKitGTK needs the same is still to be seen.)

use tauri::Window;

pub fn is_away() -> bool {
    false
}

pub fn moving() -> bool {
    false
}

pub fn in_tray() -> bool {
    false
}

/// To the tray (hidden) or minimized.
pub fn send(window: &Window, tray: bool) {
    let _ = if tray { window.hide() } else { window.minimize() };
}

/// Shows the window again; always true (there's nothing "away" to undo).
pub fn bring_back(window: &Window) -> bool {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
    true
}

pub fn saved_maximized() -> Option<bool> {
    None
}
