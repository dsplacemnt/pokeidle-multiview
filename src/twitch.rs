//! Twitch automation: which channels of the Twitch priority list are live (the channel's public
//! page), and the "visit" that briefly shows a background stream's full page to grab the bonus
//! chest and redeem the account's chosen PokeIdle bonus (the light embed player has neither).

use std::time::Duration;

use crate::config::Bonus;
use crate::redeem::{self, Redeemer};

/// How often each background Twitch stream is visited (the bonus chest comes every ~15 min).
pub const VISIT_EVERY: Duration = Duration::from_secs(15 * 60);
/// How long a visit keeps the full page before going back to the light player.
pub const VISIT_LENGTH: Duration = Duration::from_secs(30);
/// At most this many visits start per check, so they are spread out instead of all at once.
pub const VISITS_PER_CHECK: usize = 3;

const VISIT: &str = include_str!("../scripts/twitch-visit.js");

/// Whether `login` is live. The channel's public page carries schema.org data with
/// `"isLiveBroadcast":true` only while it is live, in its `<head>` (~6 KB in): only the head
/// is read (~18 KB of a ~230 KB page).
pub fn is_live(login: &str) -> Result<bool, String> {
    let mut response = crate::net::agent()
        .get(&format!("https://www.twitch.tv/{login}"))
        .header("User-Agent", crate::net::USER_AGENT)
        .call()
        .map_err(|e| e.to_string())?;
    let head = read_head(&mut response.body_mut().as_reader()).map_err(|e| e.to_string())?;
    live_from_page(&String::from_utf8_lossy(&head))
}

/// The start of an HTML page up to its `</head>` (or `HEAD_LIMIT` bytes if there's none).
const HEAD_LIMIT: usize = 256 * 1024;
fn read_head(reader: &mut impl std::io::Read) -> std::io::Result<Vec<u8>> {
    const END: &[u8] = b"</head>";
    let mut page = Vec::with_capacity(32 * 1024);
    let mut chunk = [0u8; 8 * 1024];
    loop {
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        // Only the new bytes (and a few before them) can complete "</head>".
        let from = page.len().saturating_sub(END.len());
        page.extend_from_slice(&chunk[..n]);
        if page[from..].windows(END.len()).any(|w| w == END) || page.len() >= HEAD_LIMIT {
            break;
        }
    }
    Ok(page)
}

fn live_from_page(page: &str) -> Result<bool, String> {
    // A page without Twitch's usual markup (an error or a block page) is not an answer.
    if !page.contains("twitch") {
        return Err("unexpected page".into());
    }
    let compact: String = page.split_whitespace().collect();
    Ok(compact.contains("\"isLiveBroadcast\":true"))
}

/// Twitch's side of redeeming: the script run in a visited full page, which claims the bonus
/// chest, then buys the bonus if the points cover its cheapest reward (scripts/twitch-visit.js).
/// It runs even with no bonus to buy, for the chest.
pub struct Redeem;

impl Redeemer for Redeem {
    fn script(&self, bonus: Option<Bonus>) -> Option<String> {
        let kind = bonus.map_or("null".to_string(), |b| format!("{:?}", redeem::kind(b)));
        Some(format!("{}\n{}\n{VISIT}\nwindow.__multiviewTwitchVisit({kind});", crate::kick::REPORT, redeem::COMMON))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_live_flag_from_the_page() {
        let live = r#"<html>twitch <script type="application/ld+json">{"publication":{"@type":"BroadcastEvent","isLiveBroadcast": true}}</script>"#;
        let offline = r#"<html>twitch <script type="application/ld+json">{"@type":"WebPage"}</script>"#;
        assert_eq!(live_from_page(live), Ok(true));
        assert_eq!(live_from_page(offline), Ok(false));
        assert!(live_from_page("<html>blocked</html>").is_err());
    }

    #[test]
    fn reads_only_up_to_the_end_of_the_head() {
        let mut body = b"<html><head><title>twitch</title></head><body>".to_vec();
        body.extend(std::iter::repeat_n(b'x', 100_000));
        let head = read_head(&mut &body[..]).unwrap();
        assert!(head.ends_with(b"</head><body>") || head.len() < 9000);
        assert!(head.len() < 9000, "read {} bytes", head.len());
        // No head at all: stops at the limit.
        let endless = vec![b'y'; HEAD_LIMIT * 2];
        assert!(read_head(&mut &endless[..]).unwrap().len() < HEAD_LIMIT + 8 * 1024);
    }

    /// Talks to twitch.tv: run with `cargo test -- --ignored twitch_page`.
    #[test]
    #[ignore]
    fn twitch_page_answers() {
        let result = is_live("rodil");
        assert!(result.is_ok(), "{result:?}");
    }
}
