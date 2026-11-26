//! Kick automation: which channels of the priority list are live (Kick's public channel API),
//! which ones each account should keep open, and the script that redeems the account's chosen
//! PokeIdle bonus with its channel points.

use std::collections::HashMap;
use std::time::Duration;

use crate::config::Bonus;
use crate::redeem::{self, Redeemer};

/// How often the live status is checked (and slots adjusted, bonuses redeemed).
pub const CHECK_EVERY: Duration = Duration::from_secs(120);

const REDEEM: &str = include_str!("../scripts/kick-redeem.js");
const POINTS: &str = include_str!("../scripts/kick-points.js");
/// How the app's scripts in Kick/Twitch pages report back (a brief title change).
pub const REPORT: &str = include_str!("../scripts/report.js");

/// The script that reports this account's channel points on a live Kick page.
pub fn points_script() -> String {
    format!("{REPORT}\n{POINTS}")
}

/// Whether `slug` is live right now. Uses the same JSON the kick.com site reads
/// (`/api/v2/channels/<slug>`): `livestream` is null while offline.
pub fn is_live(slug: &str) -> Result<bool, String> {
    let url = format!("https://kick.com/api/v2/channels/{slug}");
    let body = crate::net::agent()
        .get(&url)
        .header("User-Agent", crate::net::USER_AGENT)
        .header("Accept", "application/json")
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    live_from_json(&body)
}

/// Only the part of the channel JSON that matters (the rest is skipped, not built).
#[derive(serde::Deserialize)]
struct Channel {
    livestream: Option<Livestream>,
}

#[derive(serde::Deserialize)]
struct Livestream {
    is_live: Option<bool>,
}

fn live_from_json(body: &str) -> Result<bool, String> {
    let channel: Channel = serde_json::from_str(body).map_err(|e| e.to_string())?;
    Ok(channel.livestream.is_some_and(|l| l.is_live.unwrap_or(true)))
}

/// The first `n` channels of `priority` that are live (unknown status counts as offline).
pub fn pick(priority: &[String], live: &HashMap<String, bool>, n: usize) -> Vec<String> {
    priority.iter().filter(|c| live.get(*c) == Some(&true)).take(n).cloned().collect()
}

/// Points a channel that is already open gets on top of its balance when an account picks its
/// lives, so streams aren't swapped over small differences (a reload costs watch time).
pub const KEEP_OPEN_MARGIN: u64 = 50;

/// The `n` live channels of `priority` one account should watch: the ones where it has the most
/// points first (closest to its next reward, so fewer points stay stranded on channels it isn't
/// watching), with the priority order deciding between equal balances and channels never seen.
/// `open` = channels this account has open now (they get KEEP_OPEN_MARGIN).
pub fn pick_for_account(
    priority: &[String],
    live: &HashMap<String, bool>,
    n: usize,
    points: impl Fn(&str) -> u64,
    open: &[String],
) -> Vec<String> {
    let mut candidates: Vec<(usize, &String, u64)> = priority
        .iter()
        .enumerate()
        .filter(|(_, c)| live.get(*c) == Some(&true))
        .map(|(i, c)| (i, c, points(c) + if open.contains(c) { KEEP_OPEN_MARGIN } else { 0 }))
        .collect();
    candidates.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    candidates.into_iter().take(n).map(|(_, c, _)| c.clone()).collect()
}

/// Kick's side of redeeming: the script that buys a bonus in a live Kick channel page if its
/// points cover the cheapest reward of that kind (see scripts/kick-redeem.js). Kick pages show
/// their points even in the background, so it runs at every check.
pub struct Redeem;

impl Redeemer for Redeem {
    fn script(&self, bonus: Option<Bonus>) -> Option<String> {
        let kind = redeem::kind(bonus?);
        Some(format!("{REPORT}\n{}\n{REDEEM}\nwindow.__multiviewRedeem({kind:?});", redeem::COMMON))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_live_status() {
        assert_eq!(live_from_json(r#"{"slug":"a","livestream":null}"#), Ok(false));
        assert_eq!(live_from_json(r#"{"slug":"a","livestream":{"is_live":true,"viewer_count":3}}"#), Ok(true));
        assert_eq!(live_from_json(r#"{"slug":"a","livestream":{"is_live":false}}"#), Ok(false));
        assert_eq!(live_from_json(r#"{"slug":"a"}"#), Ok(false));
        assert!(live_from_json("<html>blocked</html>").is_err());
    }

    /// Talks to kick.com: run with `cargo test -- --ignored kick_api`.
    #[test]
    #[ignore]
    fn kick_api_answers() {
        let result = is_live("axt");
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn each_account_prefers_its_own_points() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<String>>();
        let priority = s(&["a", "b", "c", "d"]);
        let live: HashMap<String, bool> =
            [("a", true), ("b", true), ("c", true), ("d", false)].iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let points = |balances: &'static [(&'static str, u64)]| move |c: &str| balances.iter().find(|(k, _)| *k == c).map_or(0, |(_, v)| *v);
        // Most points first; offline "d" is never picked, however rich.
        let rich_c: &'static [(&str, u64)] = &[("c", 180), ("b", 40), ("d", 900)];
        assert_eq!(pick_for_account(&priority, &live, 2, points(rich_c), &[]), s(&["c", "b"]));
        // No balances known: the priority order, like `pick`.
        assert_eq!(pick_for_account(&priority, &live, 2, |_| 0, &[]), s(&["a", "b"]));
        // An open channel keeps its place against a slightly richer one...
        let close: &'static [(&str, u64)] = &[("a", 100), ("b", 120), ("c", 130)];
        assert_eq!(pick_for_account(&priority, &live, 1, points(close), &s(&["a"])), s(&["a"]));
        // ...but not against a clearly richer one.
        let far: &'static [(&str, u64)] = &[("a", 20), ("c", 140)];
        assert_eq!(pick_for_account(&priority, &live, 1, points(far), &s(&["a"])), s(&["c"]));
    }

    #[test]
    fn picks_the_highest_live_channels() {
        let priority: Vec<String> = ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect();
        let live: HashMap<String, bool> =
            [("a", false), ("b", true), ("d", true)].iter().map(|(k, v)| (k.to_string(), *v)).collect();
        assert_eq!(pick(&priority, &live, 2), vec!["b", "d"]);
        assert_eq!(pick(&priority, &live, 1), vec!["b"]);
        assert!(pick(&priority, &HashMap::new(), 2).is_empty());
    }
}
