//! Leaving economy mode one game at a time. In economy mode every hidden game runs in the game's
//! ULTRA Battery Saver (see `__multiviewEco` in game-watch.js): nothing on its page is drawn.
//! Bringing them all back at once makes every game rebuild its whole screen and reload its maps
//! together, a CPU spike that freezes the PC for a moment. So the rest screen stays up with a
//! loading card while the games come back one by one, each when the one before reports it's
//! done ("[multiview-eco] {…}") or after `STEP_TIMEOUT`; then the grid (or the chosen slot)
//! shows.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::AppState;

/// How long one game may take to come back before the next one starts anyway (a page that's
/// still loading, or one whose script never answers).
const STEP_TIMEOUT: Duration = Duration::from_secs(20);

pub struct Waking {
    /// Games still to bring back, in order.
    queue: VecDeque<u8>,
    /// The game coming back now.
    current: Option<u8>,
    /// Games already back.
    done: Vec<u8>,
    /// Where the player goes once every game is back: None keeps the layout as it was (the ECO
    /// button), Some(focus) shows that slot (Some(None) = the grid).
    focus: Option<Option<u8>>,
    /// Bumped on every step, so a step's timeout knows whether it's stale.
    step: u64,
}

/// What the rest screen shows while the games come back.
#[derive(Clone, Serialize)]
pub struct View {
    done: Vec<u8>,
    current: Option<u8>,
    waiting: Vec<u8>,
}

pub fn view(state: &AppState) -> Option<View> {
    state.waking.as_ref().map(|w| View {
        done: w.done.clone(),
        current: w.current,
        waiting: w.queue.iter().copied().collect(),
    })
}

/// The games already back or coming back: they draw off-screen (see `layout_plan`).
pub fn warm(state: &AppState) -> Vec<u8> {
    state.waking.as_ref().map(|w| w.done.iter().copied().chain(w.current).collect()).unwrap_or_default()
}

/// Leaves economy mode: starts bringing the games back (or only changes where the player lands,
/// if they're already coming back). `focus` as in `Waking::focus`.
pub fn leave(app: &AppHandle, focus: Option<Option<u8>>) {
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        if !state.config.economy {
            return;
        }
        if let Some(w) = state.waking.as_mut() {
            if focus.is_some() {
                w.focus = focus;
            }
            return;
        }
        let queue: VecDeque<u8> = state.config.slots.iter().filter(|s| !s.streams_only).map(|s| s.id).collect();
        println!("[economy] leaving: bringing {} game(s) back one at a time", queue.len());
        state.waking = Some(Waking { queue, current: None, done: Vec::new(), focus, step: 0 });
        crate::publish(app, &state);
    }
    next(app);
}

/// Economy mode came back on while the games were coming back: stop (the caller turns the
/// ULTRA Battery Saver back on everywhere).
pub fn cancel(state: &mut AppState) -> bool {
    state.waking.take().is_some()
}

/// The game page of slot `id` reported its Battery Saver ("[multiview-eco] {…}"): when it's the
/// one coming back, the next one starts.
pub fn reported(app: &AppHandle, id: u8, json: &str) {
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    let on = v["on"].as_bool().unwrap_or(false);
    println!(
        "[economy] slot {id}: {} (scene: {}, ultra: {})",
        if on { "economy on" } else { "economy off" },
        v["eco"].as_str().unwrap_or("?"),
        v["ultra"].as_str().unwrap_or("?")
    );
    if on {
        return;
    }
    let ours = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        state.waking.as_ref().is_some_and(|w| w.current == Some(id))
    };
    if ours {
        next(app);
    }
}

/// Marks the current game as back and starts the next one, or finishes when none is left.
fn next(app: &AppHandle) {
    let (id, step) = {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        let Some(w) = state.waking.as_mut() else { return };
        if let Some(done) = w.current.take() {
            w.done.push(done);
        }
        w.step += 1;
        // Slots closed meanwhile are skipped.
        let open: Vec<u8> = state.config.slots.iter().filter(|s| !s.streams_only).map(|s| s.id).collect();
        let w = state.waking.as_mut().unwrap();
        w.queue.retain(|id| open.contains(id));
        match w.queue.pop_front() {
            Some(id) => {
                w.current = Some(id);
                let step = w.step;
                crate::publish(app, &state);
                (id, step)
            }
            None => {
                drop(state);
                finish(app);
                return;
            }
        }
    };
    if let Err(e) = crate::apply_layout(app) {
        eprintln!("[layout] {e}");
    }
    crate::sync_game_economy(app, id, false);
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(STEP_TIMEOUT);
        let stale = {
            let state = app.state::<Mutex<AppState>>();
            let state = state.lock().unwrap();
            state.waking.as_ref().is_none_or(|w| w.step != step)
        };
        if !stale {
            eprintln!("[economy] slot {id}: no answer after {}s, going on", STEP_TIMEOUT.as_secs());
            next(&app);
        }
    });
}

/// Every game is back: economy mode ends and the slots show.
fn finish(app: &AppHandle) {
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        let Some(w) = state.waking.take() else { return };
        state.config.economy = false;
        if let Some(focus) = w.focus {
            if state.config.focus.is_some() {
                state.last_focus = state.config.focus;
            }
            state.config.focus = focus.filter(|f| state.config.slots.iter().any(|s| s.id == *f));
        }
        println!("[economy] off: every game is back");
    }
    if let Err(e) = crate::apply_layout(app) {
        eprintln!("[layout] {e}");
    }
    let state = app.state::<Mutex<AppState>>();
    crate::save_and_publish(app, &state.lock().unwrap());
}

/// Economy mode ended without the loading card (a "Log in" button shows the account at once):
/// the games still come back one at a time, in the background.
pub fn wake_quietly(app: &AppHandle) {
    let ids: Vec<u8> = {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().unwrap();
        state.waking = None;
        state.config.slots.iter().filter(|s| !s.streams_only).map(|s| s.id).collect()
    };
    let app = app.clone();
    std::thread::spawn(move || {
        for (i, id) in ids.into_iter().enumerate() {
            if i > 0 {
                std::thread::sleep(Duration::from_secs(3));
            }
            if app.state::<Mutex<AppState>>().lock().unwrap().config.economy {
                return;
            }
            crate::sync_game_economy(&app, id, false);
        }
    });
}
