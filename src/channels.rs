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

    /// A lighter page for a stream running hidden: just the video, without chat, sidebars or
    /// the site around it. None = keep the full page in the background too.
    ///
    /// - Twitch: the official embed player. It runs on twitch.tv, so the slot stays logged in.
    /// - Kick: its embed player (player.kick.com) reports live channels as offline, so Kick
    ///   keeps the full page.
    pub fn background_url(&self) -> Option<String> {
        match self.platform {
            Platform::Twitch => Some(format!(
                "https://player.twitch.tv/?channel={}&parent=twitch.tv&muted=true",
                self.login
            )),
            Platform::Kick => None,
        }
    }

    /// The URL a panel should be on: the full page when shown, the light player when hidden.
    pub fn url_for(&self, shown: bool) -> String {
        if shown {
            self.page_url()
        } else {
            self.background_url().unwrap_or_else(|| self.page_url())
        }
    }
}

/// Whether `url` is already the page `url_for` would pick (ignoring query details).
pub fn is_on(url: &Url, channel: &Channel, shown: bool) -> bool {
    let wanted: Url = match channel.url_for(shown).parse() {
        Ok(u) => u,
        Err(_) => return false,
    };
    channel_from_url(url).as_ref() == Some(channel)
        && url.host_str() == wanted.host_str()
        && url.path().trim_end_matches('/') == wanted.path().trim_end_matches('/')
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
        // Embed player: https://player.twitch.tv/?channel=<login>&parent=…
        "player.twitch.tv" => {
            let login = url
                .query_pairs()
                .find(|(k, _)| k == "channel")
                .map(|(_, v)| v.to_ascii_lowercase())?;
            valid_twitch_login(&login).then_some(Channel { platform: Platform::Twitch, login })
        }
        "kick.com" | "www.kick.com" | "player.kick.com" => {
            let first = segments.first()?;
            (!KICK_RESERVED.contains(&first.as_str()) && valid_kick_slug(first))
                .then(|| Channel { platform: Platform::Kick, login: first.clone() })
        }
        _ => None,
    }
}

/// What the player typed in a slot's "+" box: a Twitch/Kick link (with or without https://),
/// or a channel name (an optional leading @ is fine) on `platform`. None = not a channel.
pub fn parse_input(text: &str, platform: Platform) -> Option<Channel> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let looks_like_link = text.contains('/') || text.contains("twitch.tv") || text.contains("kick.com");
    let url = if looks_like_link {
        if text.starts_with("http://") || text.starts_with("https://") {
            text.replacen("http://", "https://", 1)
        } else {
            format!("https://{text}")
        }
    } else {
        let login = text.trim_start_matches('@').to_ascii_lowercase();
        Channel { platform, login }.page_url()
    };
    channel_from_url(&url.parse().ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typed_channels() {
        let p = |text: &str, platform| parse_input(text, platform).map(|c| (c.platform, c.login));
        assert_eq!(p("Gaules", Platform::Twitch), Some((Platform::Twitch, "gaules".into())));
        assert_eq!(p(" @xqc ", Platform::Kick), Some((Platform::Kick, "xqc".into())));
        // A link picks its own platform, whatever was selected.
        assert_eq!(p("https://kick.com/some-one", Platform::Twitch), Some((Platform::Kick, "some-one".into())));
        assert_eq!(p("twitch.tv/abcd", Platform::Kick), Some((Platform::Twitch, "abcd".into())));
        assert_eq!(p("http://www.twitch.tv/abcd", Platform::Kick), Some((Platform::Twitch, "abcd".into())));
        for bad in ["", "ab", "has space", "https://example.com/abcd", "twitch.tv/login", "a/b"] {
            assert_eq!(p(bad, Platform::Twitch), None, "{bad}");
        }
    }

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
    fn background_and_shown_urls() {
        let twitch = Channel { platform: Platform::Twitch, login: "abcd".into() };
        let bg = twitch.url_for(false);
        assert_eq!(bg, "https://player.twitch.tv/?channel=abcd&parent=twitch.tv&muted=true");
        assert_eq!(ch(&bg), Some((Platform::Twitch, "abcd".into())));
        assert_eq!(twitch.url_for(true), "https://www.twitch.tv/abcd");

        let on = |url: &str, shown| is_on(&url.parse().unwrap(), &twitch, shown);
        assert!(on("https://player.twitch.tv/?channel=abcd&muted=true&parent=twitch.tv", false));
        assert!(!on("https://player.twitch.tv/?channel=abcd", true));
        assert!(on("https://www.twitch.tv/abcd", true));
        assert!(!on("https://www.twitch.tv/other", true));

        // Kick keeps its full page in the background.
        let kick = Channel { platform: Platform::Kick, login: "foo".into() };
        assert_eq!(kick.url_for(false), "https://kick.com/foo");
        assert!(is_on(&"https://kick.com/foo".parse().unwrap(), &kick, false));
    }

    #[test]
    fn page_urls() {
        let c = Channel { platform: Platform::Kick, login: "foo".into() };
        assert_eq!(c.page_url(), "https://kick.com/foo");
    }
}
