//! Pages that swell over a long session get reloaded.
//!
//! Twitch and Kick players (and sometimes the game) keep growing over many hours. Each page
//! tells the app how much JavaScript memory it uses (`performance.memory`, in the stream health
//! report every 30 s and the game's stats): the reading 10 minutes after a page first reports is
//! its normal size, and when it later reaches twice that and at least 150 MB more, the page is
//! reloaded:
//! - streams: never the one on screen;
//! - the game: never while its slot is fullscreen (the player may be using it).
//! At most one reload per account every 2 minutes; a reloaded page measures its normal size again.
//!
//! Pages of the same site in one account share a renderer (--process-per-site), and the number
//! is that renderer's heap: all of an account's Twitch pages report the same value. When it
//! swells, they reload one at a time, two minutes apart, until it's back down.
//!
//! (WebView2's per-process frame info would also tell which page is heavy, but reading it
//! crashed the app, so each page reports itself instead. Pages without that number — WebKit on
//! Linux — are simply left alone.)

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

const SETTLE: Duration = Duration::from_secs(10 * 60);
const GROWTH: f64 = 2.0;
const MIN_EXTRA_MB: u64 = 150;
const PER_ACCOUNT: Duration = Duration::from_secs(2 * 60);

#[derive(Default)]
struct Page {
    first: Option<Instant>,
    normal: Option<u64>,
}

#[derive(Default)]
struct Track {
    pages: HashMap<String, Page>,
    last_reload: HashMap<u8, Instant>,
}

static TRACK: Mutex<Option<Track>> = Mutex::new(None);

/// MULTIVIEW_HEAVY_SOON=1 (testing): pages settle after a minute and every reading is logged.
fn testing() -> bool {
    std::env::var_os("MULTIVIEW_HEAVY_SOON").is_some()
}

/// What to do with a reading: the normal size once the page settled, and whether it swelled.
fn judge(page: &mut Page, mb: u64, now: Instant) -> bool {
    let first = *page.first.get_or_insert(now);
    let settle = if testing() { Duration::from_secs(60) } else { SETTLE };
    match page.normal {
        None => {
            if now.duration_since(first) >= settle {
                page.normal = Some(mb.max(1));
            }
            false
        }
        Some(normal) => mb as f64 >= normal as f64 * GROWTH && mb >= normal + MIN_EXTRA_MB,
    }
}

/// A page of slot `id` reported its JavaScript memory (MB).
pub fn reading(app: &AppHandle, id: u8, label: &str, mb: u64) {
    let now = Instant::now();
    let normal = {
        let mut track = TRACK.lock().unwrap();
        let track = track.get_or_insert_with(Track::default);
        let page = track.pages.entry(label.to_string()).or_default();
        let swollen = judge(page, mb, now);
        if testing() {
            println!("[memory] {label}: {mb} MB (normal {:?})", page.normal);
        }
        if !swollen {
            return;
        }
        if track.last_reload.get(&id).is_some_and(|t| now.duration_since(*t) < PER_ACCOUNT) {
            return;
        }
        let normal = page.normal.unwrap_or(0);
        track.pages.remove(label);
        track.last_reload.insert(id, now);
        normal
    };
    // Not the page the player is looking at.
    let skip = {
        let state = app.state::<Mutex<crate::AppState>>();
        let state = state.lock().unwrap();
        if label == crate::slot_label(id) {
            state.config.focus == Some(id) && !state.config.economy
        } else {
            state.panels.get(&id).is_some_and(|p| p.front.as_deref() == Some(label))
        }
    };
    if skip {
        return;
    }
    let what = if label == crate::slot_label(id) { "The game" } else { "A stream" };
    let text = format!("{what} grew to {mb} MB of memory (normal {normal} MB): reloaded");
    println!("[memory] slot {id} {label}: {text}");
    {
        let state = app.state::<Mutex<crate::AppState>>();
        let mut state = state.lock().unwrap();
        crate::log_event(&mut state, id, text);
    }
    if let Some(page) = app.get_webview(label) {
        crate::page_activity();
        let _ = page.reload();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swelling_needs_double_and_150_mb_after_settling() {
        let t0 = Instant::now();
        let mut p = Page::default();
        // Still settling: never.
        assert!(!judge(&mut p, 900, t0));
        assert!(!judge(&mut p, 900, t0 + Duration::from_secs(60)));
        // Normal size taken at 10 min.
        assert!(!judge(&mut p, 100, t0 + SETTLE));
        assert_eq!(p.normal, Some(100));
        assert!(!judge(&mut p, 190, t0 + SETTLE * 2)); // not double
        assert!(!judge(&mut p, 240, t0 + SETTLE * 2)); // double but only +140
        assert!(judge(&mut p, 260, t0 + SETTLE * 2));
        // A small page: double isn't enough without +150 MB.
        let mut small = Page { first: Some(t0), normal: Some(20) };
        assert!(!judge(&mut small, 60, t0 + SETTLE));
        assert!(judge(&mut small, 180, t0 + SETTLE));
    }
}
