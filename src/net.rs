//! The app's HTTP client for the live checks: one shared agent, so checks reuse connections
//! instead of opening (and securing) a new one for every channel, and run side by side.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

/// The app says what it is: Kick's Cloudflare front refuses (403) clients that claim to be a
/// browser but don't look like one, and accepts a plain app user agent.
pub const USER_AGENT: &str = concat!("pokeidle-multiview/", env!("CARGO_PKG_VERSION"));

/// How many channels are checked at the same time.
const PARALLEL: usize = 8;

pub fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(20)))
            .build()
            .new_agent()
    })
}

/// Runs `check` on every item, up to `PARALLEL` at a time; results keep the items' order.
pub fn check_all<T: Sync, R: Send>(items: &[T], check: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let mut results: Vec<Option<R>> = (0..items.len()).map(|_| None).collect();
    let slots: Vec<std::sync::Mutex<&mut Option<R>>> = results.iter_mut().map(std::sync::Mutex::new).collect();
    std::thread::scope(|scope| {
        for _ in 0..PARALLEL.min(items.len()) {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(item) = items.get(i) else { break };
                let result = check(item);
                **slots[i].lock().unwrap() = Some(result);
            });
        }
    });
    drop(slots);
    results.into_iter().map(|r| r.expect("every item was checked")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_all_keeps_order_and_runs_everything() {
        let items: Vec<u32> = (0..50).collect();
        let out = check_all(&items, |n| {
            std::thread::sleep(Duration::from_millis(u64::from(50 - n) % 7));
            n * 2
        });
        assert_eq!(out, items.iter().map(|n| n * 2).collect::<Vec<_>>());
        assert!(check_all(&Vec::<u32>::new(), |n| *n).is_empty());
    }

    /// Talks to twitch.tv and kick.com: `cargo test --release -- --ignored --nocapture live_checks_timing`.
    /// Checks a list the new way and prints how long it took and what it found.
    #[test]
    #[ignore]
    fn live_checks_timing() {
        let twitch: Vec<String> = ["rodil", "kennzy", "faturetosl", "ookina", "patopapao", "techzz", "dersin2k", "denissnidertv", "surskity11", "tftoddy", "daniels"]
            .map(String::from)
            .to_vec();
        let kick: Vec<String> = ["axt", "surskity", "rodil", "clovaobengalalol", "yetz", "patopapao2", "techzz", "guaxilol1", "dersin2k", "denissnider", "yoda", "kennzy", "ookina8", "matta"]
            .map(String::from)
            .to_vec();
        let start = std::time::Instant::now();
        let t = check_all(&twitch, |c| crate::twitch::is_live(c));
        let twitch_time = start.elapsed();
        let start = std::time::Instant::now();
        let k = check_all(&kick, |c| crate::kick::is_live(c));
        let kick_time = start.elapsed();
        println!("twitch {} channels in {:?}: {:?}", twitch.len(), twitch_time, twitch.iter().zip(&t).map(|(c, r)| format!("{c}={r:?}")).collect::<Vec<_>>());
        println!("kick {} channels in {:?}: {:?}", kick.len(), kick_time, kick.iter().zip(&k).map(|(c, r)| format!("{c}={r:?}")).collect::<Vec<_>>());
        assert!(t.iter().all(Result::is_ok) && k.iter().all(Result::is_ok));
    }
}
