//! Recognizes Twitch/Kick channel links (as opposed to login, settings or other site pages).
//! Login formats and reserved paths follow the sites' own rules.

use tauri::Url;

use crate::config::Platform;

const TWITCH_RESERVED: &[&str] = &[
    "directory", "videos", "settings", "search", "downloads", "p", "team", "store", "turbo",
    "jobs", "security", "broadcast", "dashboard", "moderator", "u", "chat", "embed", "popout",
    "signup", "login", "logout", "subscriptions", "wallet", "inventory", "drops", "terms",
    "privacy", "dmca", "legal", "help", "about", "sitemap", "redeem", "payments", "campfires",
];

const KICK_RESERVED: &[&str] = &[
    "browse", "categories", "search", "dashboard", "terms-of-service", "privacy-policy",
    "support", "community-guidelines", "dmca", "newsroom", "careers", "apps", "help", "login",
    "signup", "security", "api", "videos", "clips", "leaderboard", "faq", "rules",
    "transparency", "report", "welcome", "store", "merch", "account",
];

/// Twitch logins: 4–25 of [a-z0-9_].
fn valid_twitch_login(s: &str) -> bool {
    (4..=25).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Kick slugs: 3–25 of [a-z0-9_-].
fn valid_kick_slug(s: &str) -> bool {
    (3..=25).contains(&s.len())
        && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// A channel the link points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub platform: Platform,
    pub login: String,
}

impl Channel {
    /// The channel's full page, used for the visible panel.
    pub fn page_url(&self) -> String {
        match self.platform {
            Platform::Twitch => format!("https://www.twitch.tv/{}", self.login),
            Platform::Kick => format!("https://kick.com/{}", self.login),
        }
    }
}

/// `https://www.twitch.tv/somechannel` → Twitch "somechannel"; `https://kick.com/foo` → Kick
/// "foo". Popout and embed players count too. Anything else (login pages, OAuth, settings,
/// other sites) → None.
pub fn channel_from_url(url: &Url) -> Option<Channel> {
    if url.scheme() != "https" {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    let segments: Vec<String> = url
        .path_segments()?
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase())
        .collect();
    match host.as_str() {
        "twitch.tv" | "www.twitch.tv" | "m.twitch.tv" => {
            // Popout player: /popout/<channel>/player.
            let first = if segments.first().map(String::as_str) == Some("popout") {
                segments.get(1)?
            } else {
                segments.first()?
            };
            (!TWITCH_RESERVED.contains(&first.as_str()) && valid_twitch_login(first))
                .then(|| Channel { platform: Platform::Twitch, login: first.clone() })
        }
        "kick.com" | "www.kick.com" | "player.kick.com" => {
            let first = segments.first()?;
            (!KICK_RESERVED.contains(&first.as_str()) && valid_kick_slug(first))
                .then(|| Channel { platform: Platform::Kick, login: first.clone() })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(url: &str) -> Option<(Platform, String)> {
        channel_from_url(&url.parse().unwrap()).map(|c| (c.platform, c.login))
    }

    #[test]
    fn finds_channels() {
        assert_eq!(ch("https://www.twitch.tv/SomeStreamer"), Some((Platform::Twitch, "somestreamer".into())));
        assert_eq!(ch("https://twitch.tv/abcd?ref=game"), Some((Platform::Twitch, "abcd".into())));
        assert_eq!(ch("https://www.twitch.tv/popout/abcd/player"), Some((Platform::Twitch, "abcd".into())));
        assert_eq!(ch("https://kick.com/some-one"), Some((Platform::Kick, "some-one".into())));
        assert_eq!(ch("https://player.kick.com/xyz"), Some((Platform::Kick, "xyz".into())));
    }

    #[test]
    fn ignores_other_pages() {
        for url in [
            "https://www.twitch.tv/",
            "https://www.twitch.tv/login",
            "https://www.twitch.tv/directory/category/x",
            "https://id.twitch.tv/oauth2/authorize",
            "https://www.twitch.tv/passport-callback",
            "https://www.twitch.tv/abc",
            "https://kick.com/",
            "https://kick.com/browse",
            "https://id.kick.com/oauth/authorize",
            "https://kick.com/ab",
            "http://www.twitch.tv/abcd",
            "https://pokeidle.io/app",
        ] {
            assert_eq!(ch(url), None, "{url}");
        }
    }

    #[test]
    fn page_urls() {
        let c = Channel { platform: Platform::Kick, login: "foo".into() };
        assert_eq!(c.page_url(), "https://kick.com/foo");
    }
}
