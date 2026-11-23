//! Keyboard shortcuts, configurable in ⚙ → Shortcuts. They work while the app has the focus,
//! in any of its pages: the app's own pages (toolbar, slot headers, stats panel) through
//! ui/shortcuts.js, and the game/Twitch/Kick pages through WebView2's accelerator keys
//! (native.rs), so a page can't swallow them.
//!
//! A shortcut is written "Ctrl+Shift+R": modifiers in the order Ctrl, Alt, Shift, then one key
//! (A–Z, 0–9 or F1–F12). A letter or digit needs Ctrl or Alt (typing must still work); a
//! function key may go alone. An empty one turns the action off.

use std::collections::BTreeMap;

use tauri::{AppHandle, Emitter, Manager};

/// Every action, in the order the settings list shows them, with its default keys.
pub const ACTIONS: [(&str, &str); 8] = [
    ("focus1", "Ctrl+1"),
    ("focus2", "Ctrl+2"),
    ("focus3", "Ctrl+3"),
    ("focus4", "Ctrl+4"),
    ("grid", "Ctrl+G"),
    ("reload_all", "Ctrl+Shift+R"),
    ("new_slot", "Ctrl+T"),
    ("economy", "Ctrl+E"),
];

pub fn defaults() -> BTreeMap<String, String> {
    ACTIONS.iter().map(|(a, k)| (a.to_string(), k.to_string())).collect()
}

/// The saved shortcuts made whole: unknown actions dropped, missing ones back to their
/// default, every combo written the canonical way (or "" when it isn't a valid one).
pub fn normalized(saved: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    ACTIONS
        .iter()
        .map(|(action, default)| {
            let keys = match saved.get(*action) {
                Some(keys) => canonical(keys).unwrap_or_default(),
                None => default.to_string(),
            };
            (action.to_string(), keys)
        })
        .collect()
}

/// "shift+ctrl+r" → "Ctrl+Shift+R"; None if it isn't a usable shortcut.
pub fn canonical(text: &str) -> Option<String> {
    let (mut ctrl, mut alt, mut shift) = (false, false, false);
    let mut key: Option<String> = None;
    for part in text.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "alt" => alt = true,
            "shift" => shift = true,
            other => {
                if key.is_some() {
                    return None;
                }
                key = Some(key_name(other)?);
            }
        }
    }
    let key = key?;
    let function = key.starts_with('F') && key.len() > 1;
    if !function && !ctrl && !alt {
        return None;
    }
    let mut out = String::new();
    for (on, name) in [(ctrl, "Ctrl+"), (alt, "Alt+"), (shift, "Shift+")] {
        if on {
            out.push_str(name);
        }
    }
    out.push_str(&key);
    Some(out)
}

/// A key's canonical name: "A"–"Z", "0"–"9", "F1"–"F12".
fn key_name(key: &str) -> Option<String> {
    let upper = key.to_ascii_uppercase();
    let mut chars = upper.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphanumeric() => Some(upper),
        (Some('F'), Some(_)) => {
            let n: u8 = upper[1..].parse().ok()?;
            (1..=12).contains(&n).then_some(upper)
        }
        _ => None,
    }
}

/// The shortcut for a Windows virtual key and the modifiers held, as `canonical` writes it.
// Used by native.rs (WebView2); elsewhere the pages report the combination as text.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn from_virtual_key(vk: u32, ctrl: bool, alt: bool, shift: bool) -> Option<String> {
    let key = match vk {
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(vk)?.to_string(),
        0x70..=0x7B => format!("F{}", vk - 0x70 + 1),
        _ => return None,
    };
    let mut text = String::new();
    for (on, name) in [(ctrl, "Ctrl+"), (alt, "Alt+"), (shift, "Shift+")] {
        if on {
            text.push_str(name);
        }
    }
    text.push_str(&key);
    canonical(&text)
}

/// The action bound to `keys`, if any.
pub fn action_for(shortcuts: &BTreeMap<String, String>, keys: &str) -> Option<String> {
    shortcuts.iter().find(|(_, k)| !k.is_empty() && k.as_str() == keys).map(|(a, _)| a.clone())
}

/// Runs an action. Layout changes create and move webviews, so this is never called from
/// inside a WebView2 event: callers spawn it.
pub fn run(app: &AppHandle, action: &str) {
    match action {
        "focus1" | "focus2" | "focus3" | "focus4" => {
            let id = action[5..].parse::<u8>().unwrap_or(1);
            crate::toggle_focus(app, Some(id));
        }
        "grid" => crate::toggle_grid(app),
        "economy" => crate::toggle_economy(app),
        "reload_all" => crate::reload_all_games(app),
        "new_slot" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = crate::add_slot(app.clone()).await {
                    let _ = app.emit("error", e);
                }
            });
        }
        _ => {}
    }
}

/// The shortcuts as they are now (what the pages and the settings list use).
#[tauri::command]
pub fn get_shortcuts(app: AppHandle) -> BTreeMap<String, String> {
    let state = app.state::<std::sync::Mutex<crate::AppState>>();
    let state = state.lock().unwrap();
    normalized(&state.config.shortcuts)
}

/// A page of the app saw `action`'s keys pressed.
#[tauri::command]
pub fn run_shortcut(app: AppHandle, action: String) {
    run(&app, &action);
}

/// Sets one action's keys ("" = off). Keys already used by another action move here (that
/// one is left without). Errors when the keys aren't a usable shortcut.
#[tauri::command]
pub fn set_shortcut(app: AppHandle, action: String, keys: String) -> Result<BTreeMap<String, String>, String> {
    if !ACTIONS.iter().any(|(a, _)| *a == action) {
        return Err("Unknown action".into());
    }
    let keys = if keys.trim().is_empty() { String::new() } else { canonical(&keys).ok_or("Not a usable shortcut")? };
    let map = {
        let state = app.state::<std::sync::Mutex<crate::AppState>>();
        let mut state = state.lock().unwrap();
        let mut map = normalized(&state.config.shortcuts);
        if !keys.is_empty() {
            for (_, other) in map.iter_mut().filter(|(a, k)| **a != action && **k == keys) {
                other.clear();
            }
        }
        map.insert(action, keys);
        state.config.shortcuts = map.clone();
        if let Err(e) = state.config.save(&state.config_path) {
            eprintln!("[config] could not save: {e}");
        }
        map
    };
    let _ = app.emit("shortcuts", map.clone());
    Ok(map)
}

/// Every shortcut back to its default.
#[tauri::command]
pub fn reset_shortcuts(app: AppHandle) -> BTreeMap<String, String> {
    let map = defaults();
    {
        let state = app.state::<std::sync::Mutex<crate::AppState>>();
        let mut state = state.lock().unwrap();
        state.config.shortcuts = map.clone();
        if let Err(e) = state.config.save(&state.config_path) {
            eprintln!("[config] could not save: {e}");
        }
    }
    let _ = app.emit("shortcuts", map.clone());
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_orders_modifiers_and_rejects_bad_keys() {
        assert_eq!(canonical("shift+ctrl+r").as_deref(), Some("Ctrl+Shift+R"));
        assert_eq!(canonical("Alt+5").as_deref(), Some("Alt+5"));
        assert_eq!(canonical("f5").as_deref(), Some("F5"));
        assert_eq!(canonical("Shift+F12").as_deref(), Some("Shift+F12"));
        // Typing must keep working: a bare letter or Shift+letter isn't a shortcut.
        assert_eq!(canonical("R"), None);
        assert_eq!(canonical("Shift+R"), None);
        assert_eq!(canonical("Ctrl+F13"), None);
        assert_eq!(canonical("Ctrl+A+B"), None);
        assert_eq!(canonical("Ctrl+Esc"), None);
        assert_eq!(canonical("Ctrl"), None);
    }

    #[test]
    fn virtual_keys_map_like_the_pages() {
        assert_eq!(from_virtual_key(0x31, true, false, false).as_deref(), Some("Ctrl+1"));
        assert_eq!(from_virtual_key(0x52, true, false, true).as_deref(), Some("Ctrl+Shift+R"));
        assert_eq!(from_virtual_key(0x74, false, false, false).as_deref(), Some("F5"));
        assert_eq!(from_virtual_key(0x52, false, false, false), None);
        assert_eq!(from_virtual_key(0x1B, true, false, false), None);
    }

    #[test]
    fn saved_shortcuts_are_completed_and_cleaned() {
        let mut saved = BTreeMap::new();
        saved.insert("grid".to_string(), "".to_string());
        saved.insert("reload_all".to_string(), "shift+ctrl+l".to_string());
        saved.insert("gone".to_string(), "Ctrl+Q".to_string());
        let map = normalized(&saved);
        assert_eq!(map.len(), ACTIONS.len());
        assert_eq!(map["grid"], "");
        assert_eq!(map["reload_all"], "Ctrl+Shift+L");
        assert_eq!(map["focus1"], "Ctrl+1");
        assert!(!map.contains_key("gone"));
        assert_eq!(action_for(&map, "Ctrl+Shift+L").as_deref(), Some("reload_all"));
        assert_eq!(action_for(&map, ""), None);
    }
}
