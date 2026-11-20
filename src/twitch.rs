//! Twitch automation: which channels of the Twitch priority list are live (the channel's public
//! page), and the "visit" that briefly shows a background stream's full page to grab the bonus
//! chest and redeem the account's chosen PokeIdle bonus (the light embed player has neither).

use std::time::Duration;

use crate::config::KickBonus;

/// How often each background Twitch stream is visited (the bonus chest comes every ~15 min).
pub const VISIT_EVERY: Duration = Duration::from_secs(15 * 60);
/// How long a visit keeps the full page before going back to the light player.
pub const VISIT_LENGTH: Duration = Duration::from_secs(30);
/// At most this many visits start per check, so they are spread out instead of all at once.
pub const VISITS_PER_CHECK: usize = 3;

const VISIT: &str = include_str!("../scripts/twitch-visit.js");

/// Whether `login` is live. The channel's public page carries schema.org data with
/// `"isLiveBroadcast":true` only while it is live.
pub fn is_live(login: &str) -> Result<bool, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .new_agent();
    let page = agent
        .get(&format!("https://www.twitch.tv/{login}"))
        .header("User-Agent", concat!("pokeidle-multiview/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .with_config()
        .limit(5 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    live_from_page(&page)
}

fn live_from_page(page: &str) -> Result<bool, String> {
    // A page without Twitch's usual markup (an error or a block page) is not an answer.
    if !page.contains("twitch") {
        return Err("unexpected page".into());
    }
    let compact: String = page.split_whitespace().collect();
    Ok(compact.contains("\"isLiveBroadcast\":true"))
}

/// The script run in a visited full page: claims the bonus chest, then redeems `bonus` if the
/// points cover its cheapest reward (scripts/twitch-visit.js).
pub fn visit_script(bonus: Option<KickBonus>) -> String {
    let kind = match bonus {
        Some(KickBonus::Xp) => "\"xp\"",
        Some(KickBonus::Capture) => "\"capture\"",
        Some(KickBonus::Lure) => "\"lure\"",
        None => "null",
    };
    format!("{}\n{VISIT}\nwindow.__multiviewTwitchVisit({kind});", crate::kick::REPORT)
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
    fn visit_script_names_the_bonus() {
        assert!(visit_script(Some(KickBonus::Capture)).ends_with("__multiviewTwitchVisit(\"capture\");"));
        assert!(visit_script(None).ends_with("__multiviewTwitchVisit(null);"));
    }

    /// Talks to twitch.tv: run with `cargo test -- --ignored twitch_page`.
    #[test]
    #[ignore]
    fn twitch_page_answers() {
        let result = is_live("rodil");
        assert!(result.is_ok(), "{result:?}");
    }
}
