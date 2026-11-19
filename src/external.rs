//! Links the slots may not open (votes, socials, help pages…) go to the user's default
//! browser instead of being dropped. Only http(s), and throttled so a page that keeps
//! redirecting or popping up ads can't flood the browser with tabs.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::Url;

/// The same link again within this window is ignored (double clicks, redirect loops).
const SAME_URL_COOLDOWN: Duration = Duration::from_secs(10);
/// At most this many links per window, whatever they are.
const BURST_LIMIT: usize = 3;
const BURST_WINDOW: Duration = Duration::from_secs(30);

/// Remembers recent openings to apply the limits above.
#[derive(Default)]
pub struct Throttle {
    recent: VecDeque<(Instant, String)>,
}

impl Throttle {
    /// Whether `url` may be opened at `now`; if so, it is recorded.
    pub fn allow(&mut self, url: &str, now: Instant) -> bool {
        while self
            .recent
            .front()
            .is_some_and(|(t, _)| now.duration_since(*t) > BURST_WINDOW)
        {
            self.recent.pop_front();
        }
        let repeated = self
            .recent
            .iter()
            .any(|(t, u)| u == url && now.duration_since(*t) < SAME_URL_COOLDOWN);
        if repeated || self.recent.len() >= BURST_LIMIT {
            return false;
        }
        self.recent.push_back((now, url.to_string()));
        true
    }
}

static THROTTLE: Mutex<Throttle> = Mutex::new(Throttle { recent: VecDeque::new() });

/// Opens `url` in the default browser if it is a web link and the limits allow it.
pub fn open_in_browser(url: &Url) {
    if !matches!(url.scheme(), "http" | "https") {
        eprintln!("[external] ignored non-web link {url}");
        return;
    }
    if !THROTTLE.lock().unwrap().allow(url.as_str(), Instant::now()) {
        eprintln!("[external] throttled {url}");
        return;
    }
    match tauri_plugin_opener::open_url(url.as_str(), None::<&str>) {
        Ok(()) => println!("[external] opened in browser: {url}"),
        Err(e) => eprintln!("[external] could not open {url}: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_repeats_and_bursts() {
        let mut t = Throttle::default();
        let t0 = Instant::now();
        assert!(t.allow("https://a.example/", t0));
        assert!(!t.allow("https://a.example/", t0 + Duration::from_secs(2)), "same link again");
        assert!(t.allow("https://b.example/", t0 + Duration::from_secs(3)));
        assert!(t.allow("https://c.example/", t0 + Duration::from_secs(4)));
        assert!(!t.allow("https://d.example/", t0 + Duration::from_secs(5)), "4th within 30 s");
        // Once the window has passed, links open again.
        assert!(t.allow("https://d.example/", t0 + Duration::from_secs(40)));
        assert!(t.allow("https://a.example/", t0 + Duration::from_secs(41)));
    }
}
