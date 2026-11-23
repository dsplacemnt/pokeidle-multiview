//! What the game pages report about themselves besides the hunt numbers (see game-watch.js):
//! whether each account's game is still connected, and the Pokémon sprites the panel draws.
//!
//! An account can drop out of the game without anything on screen saying so: it logged in on
//! another device or browser (the game closes the connection with code 4001 and doesn't come
//! back), the connection fell and didn't come back, or the page has no game session at all.
//! Each of those becomes a warning on the slot's header, in economy mode and in the panel,
//! with a button that reloads the game.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use serde::Deserialize;
use tauri::{AppHandle, Manager};

/// The game closes the connection with this code when the account logs in somewhere else.
const CLOSED_ELSEWHERE: u16 = 4001;
/// A connection that stays down this long (the game reconnects by itself after short drops).
const DOWN_FOR: i64 = 60_000;

/// The last report of a slot's game page.
#[derive(Deserialize, Clone, Copy, Default, Debug)]
pub struct GameConn {
    /// The game's socket is connected (None until it first tried).
    pub open: Option<bool>,
    /// How the socket last closed.
    pub code: Option<u16>,
    /// The page holds a game session.
    pub session: Option<bool>,
    /// When this report came (PC clock, ms).
    #[serde(skip)]
    pub at: i64,
}

/// What is wrong with an account's game, if anything. The texts are English keys that the
/// pages translate (ui/i18n.js).
pub fn issue(conn: &GameConn, now: i64) -> Option<&'static str> {
    if conn.session == Some(false) {
        return Some("Not logged in to the game");
    }
    if conn.open == Some(false) {
        if conn.code == Some(CLOSED_ELSEWHERE) {
            return Some("Logged in somewhere else: this game was disconnected");
        }
        if now - conn.at >= DOWN_FOR {
            return Some("The game lost its connection");
        }
    }
    None
}

/// Each account's game problem, for the pages: slot → text.
pub fn issues(state: &crate::AppState) -> HashMap<u8, String> {
    let now = crate::now_ms();
    state
        .game_conn
        .iter()
        .filter_map(|(id, c)| issue(c, now).map(|text| (*id, text.to_string())))
        .collect()
}

/// "[multiview-conn] {…}" from a game page.
pub fn conn_reported(app: &AppHandle, id: u8, json: &str) {
    let Ok(mut conn) = serde_json::from_str::<GameConn>(json) else { return };
    conn.at = crate::now_ms();
    let state = app.state::<Mutex<crate::AppState>>();
    let mut state = state.lock().unwrap();
    let before = state.game_conn.get(&id).and_then(|c| issue(c, conn.at + DOWN_FOR));
    // A closed socket that was already closed keeps its first time (the 60 s count from then).
    if let Some(old) = state.game_conn.get(&id) {
        if old.open == Some(false) && conn.open == Some(false) {
            conn.at = old.at;
        }
    }
    state.game_conn.insert(id, conn);
    let after = issue(&conn, conn.at + DOWN_FOR);
    if before != after {
        let text = after.unwrap_or("Game connected again");
        println!("[game] slot {id}: {text}");
        crate::log_event(&mut state, id, text);
    }
    crate::discord::check_games(&mut state);
    crate::publish(app, &state);
    if conn.open == Some(false) && conn.code != Some(CLOSED_ELSEWHERE) {
        // Shown once it has been down long enough, unless it reconnects first.
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(DOWN_FOR as u64 + 1_000));
            let state = app.state::<Mutex<crate::AppState>>();
            let mut state = state.lock().unwrap();
            crate::discord::check_games(&mut state);
            crate::publish(&app, &state);
        });
    }
}

/// A game page was reloaded or closed: its old report no longer says anything.
pub fn forget(state: &mut crate::AppState, id: u8) {
    state.game_conn.remove(&id);
}

/// "[multiview-sprite] {"look":…,"src":…,"x":…,"y":…,"w":…,"h":…}": where a Pokémon's first
/// frame is on the game's sprite sheet. Kept for all accounts (looks are the same everywhere).
pub fn sprite_reported(app: &AppHandle, json: &str) {
    #[derive(Deserialize)]
    struct Report {
        look: u32,
        src: String,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    }
    let Ok(r) = serde_json::from_str::<Report>(json) else { return };
    // Only the game's own images.
    if !r.src.starts_with("https://pokeidle.io/") || r.w == 0 || r.h == 0 || r.w > 512 || r.h > 512 {
        return;
    }
    let state = app.state::<Mutex<crate::AppState>>();
    let mut state = state.lock().unwrap();
    state.sprites.insert(r.look, serde_json::json!({ "src": r.src, "x": r.x, "y": r.y, "w": r.w, "h": r.h }));
    crate::publish(app, &state);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn(open: Option<bool>, code: Option<u16>, session: Option<bool>, at: i64) -> GameConn {
        GameConn { open, code, session, at }
    }

    #[test]
    fn issues_need_a_real_problem() {
        assert_eq!(issue(&conn(Some(true), None, Some(true), 0), 1_000_000), None);
        assert_eq!(issue(&conn(None, None, None, 0), 1_000_000), None);
        // A short drop: the game reconnects by itself.
        assert_eq!(issue(&conn(Some(false), Some(1006), Some(true), 0), 30_000), None);
        assert_eq!(issue(&conn(Some(false), Some(1006), Some(true), 0), 61_000), Some("The game lost its connection"));
        // Logged in elsewhere: right away.
        assert!(issue(&conn(Some(false), Some(4001), Some(true), 0), 1).unwrap().starts_with("Logged in somewhere else"));
        // No session wins over the rest.
        assert_eq!(issue(&conn(Some(false), Some(4001), Some(false), 0), 1), Some("Not logged in to the game"));
    }
}
