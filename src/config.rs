//! Persisted settings: slots (name, mute, streams, bonus), layout, tray, stream automation and
//! comfort options.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub const MAX_SLOTS: u8 = 4;
/// Twitch (or Kick) pages one slot may keep running at once.
pub const MAX_PAGES_PER_PLATFORM: usize = 8;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Twitch,
    Kick,
}

/// A Twitch/Kick page that was running in a slot, reopened (hidden, muted) at startup.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SavedPanel {
    pub platform: Platform,
    pub url: String,
}

/// A PokeIdle bonus bought with channel points on Kick and Twitch (the channels' "Bônus +15% …"
/// rewards).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KickBonus {
    /// "Bônus +15% XP PokeIdle"
    Xp,
    /// "Bônus de +15% Capture Boost"
    Capture,
    /// "Bônus de +15% Secret Lure"
    Lure,
}

/// One redeem goal: keep at least `hours` of this bonus running.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Goal {
    pub bonus: KickBonus,
    pub hours: u16,
}

/// What an account does with its points once every goal is met.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WhenMet {
    /// Keep the points for when a goal runs low again.
    #[default]
    Save,
    /// Keep buying the last goal's bonus.
    KeepLast,
}

/// An account's redeem goals, in order: the first one not met is what gets bought.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct Goals {
    #[serde(default)]
    pub list: Vec<Goal>,
    #[serde(default)]
    pub when_met: WhenMet,
}

/// At most this many channels in the Kick priority list.
pub const MAX_KICK_PRIORITY: usize = 30;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SlotConfig {
    pub id: u8,
    /// The game page's zoom. No longer adjustable (the controls were removed to give the
    /// header room) and not applied: kept so files from older versions still load.
    #[serde(default = "full_zoom")]
    pub zoom: f64,
    /// Shown on the slot's chip; None = "Slot N".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The game page's sound is off (its Twitch/Kick pages are muted anyway when hidden).
    #[serde(default)]
    pub muted: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub panels: Vec<SavedPanel>,
    /// The bonus this account redeems automatically with its channel points, on Kick and
    /// Twitch; None = off. ("kick_bonus" is the name it had while it was Kick only.)
    #[serde(default, alias = "kick_bonus", skip_serializing_if = "Option::is_none")]
    pub bonus: Option<KickBonus>,
    /// Redeem goals instead of the fixed `bonus` (Some = goals mode); see goals.rs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goals: Option<Goals>,
    /// "Streams only": this account's game runs on another computer (one game connection per
    /// account: opening it here would log the other one out). The slot keeps its session and
    /// runs only its Twitch/Kick streams, chests and redeems; a light page of the app's own
    /// takes the game's place.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub streams_only: bool,
    /// "Game only": the account plays here but runs no Twitch/Kick streams (no lives, chests or
    /// redeems); the game's own bonuses still show. Never set together with `streams_only`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub game_only: bool,
    /// Streams only: when this account's bonuses end, as the app reckons them from its own
    /// redeems (the game isn't here to tell): platform ("kick"/"twitch") → [xp, capture, lure]
    /// end times, ms since 1970 (0 = not running).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bonus_estimate: BTreeMap<String, [i64; 3]>,
}

fn full_zoom() -> f64 {
    1.0
}

impl SlotConfig {
    pub fn new(id: u8) -> SlotConfig {
        SlotConfig {
            id,
            zoom: 1.0,
            name: None,
            muted: false,
            panels: Vec::new(),
            bonus: None,
            goals: None,
            streams_only: false,
            game_only: false,
            bonus_estimate: BTreeMap::new(),
        }
    }

    pub fn mode(&self) -> SlotMode {
        if self.streams_only {
            SlotMode::Streams
        } else if self.game_only {
            SlotMode::Game
        } else {
            SlotMode::Both
        }
    }

    pub fn set_mode(&mut self, mode: SlotMode) {
        self.streams_only = mode == SlotMode::Streams;
        self.game_only = mode == SlotMode::Game;
    }

    /// The slot runs Twitch/Kick streams (every mode but "game only").
    pub fn runs_streams(&self) -> bool {
        self.mode() != SlotMode::Game
    }
}

/// What a slot runs: the game and its streams (the usual), only the game, or only the streams
/// (the game runs on another computer; see streams_only.rs).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SlotMode {
    #[default]
    Both,
    Game,
    Streams,
}

/// Cleans the Twitch priority list: logins or links become lowercase logins, invalid and
/// repeated entries are dropped, at most MAX_KICK_PRIORITY are kept.
pub fn clean_twitch_priority(raw: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for entry in raw {
        let entry = entry.trim();
        let login = entry
            .rsplit_once("twitch.tv/")
            .map_or(entry, |(_, rest)| rest)
            .trim_start_matches('@')
            .split(['/', '?', '#'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let valid = (4..=25).contains(&login.len())
            && login.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if valid && !out.contains(&login) {
            out.push(login);
        }
        if out.len() == MAX_KICK_PRIORITY {
            break;
        }
    }
    out
}

/// Twitch lives each account keeps open while the Twitch priority list is in use: 1 to 8.
pub const DEFAULT_TWITCH_LIVES: u8 = 4;

/// Cleans the Kick priority list typed by the user: channel names or links become lowercase
/// slugs, invalid and repeated entries are dropped, at most MAX_KICK_PRIORITY are kept.
pub fn clean_kick_priority(raw: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for entry in raw {
        let entry = entry.trim();
        let slug = entry
            .rsplit_once("kick.com/")
            .map_or(entry, |(_, rest)| rest)
            .trim_start_matches('@')
            .split(['/', '?', '#'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let valid = (3..=25).contains(&slug.len())
            && slug.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
        if valid && !out.contains(&slug) {
            out.push(slug);
        }
        if out.len() == MAX_KICK_PRIORITY {
            break;
        }
    }
    out
}

pub const MAX_NAME_CHARS: usize = 40;

/// Normalizes a slot name typed by the user: trimmed, at most 40 characters, empty = default.
pub fn clean_name(raw: &str) -> Option<String> {
    let name: String = raw.trim().chars().take(MAX_NAME_CHARS).collect();
    let name = name.trim().to_string();
    (!name.is_empty()).then_some(name)
}

/// Where the splitters sit, as fractions of the game area.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct GridSplit {
    /// Left column width.
    pub v: f64,
    /// Top row height.
    pub h: f64,
}

pub const MIN_SPLIT: f64 = 0.15;
pub const MAX_SPLIT: f64 = 0.85;

pub fn clamp_split(raw: f64, fallback: f64) -> f64 {
    if raw.is_finite() {
        raw.clamp(MIN_SPLIT, MAX_SPLIT)
    } else {
        fallback
    }
}

/// Allowed auto-reload intervals in minutes; 0 = off.
pub const AUTO_RELOAD_OPTIONS: [u32; 8] = [0, 15, 30, 60, 180, 360, 720, 1440];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub slots: Vec<SlotConfig>,
    /// The slot shown fullscreen (the others hidden); None = grid.
    #[serde(default)]
    pub focus: Option<u8>,
    #[serde(default = "default_grid_split")]
    pub grid_split: GridSplit,
    /// Keep Windows from sleeping while background streams run.
    #[serde(default = "enabled")]
    pub keep_awake: bool,
    /// Reload every game page this often (minutes); 0 = off.
    #[serde(default)]
    pub auto_reload_minutes: u32,
    /// × hides the window to the tray (streams keep running); "Quit" in the tray menu exits.
    #[serde(default = "enabled")]
    pub close_to_tray: bool,
    /// Minimizing hides the window to the tray instead of the taskbar (off by default: only
    /// closing goes to the tray).
    #[serde(default)]
    pub minimize_to_tray: bool,
    /// The top toolbar is hidden to give the slots more room; a button in the top-left slot's
    /// header brings it back.
    #[serde(default)]
    pub toolbar_hidden: bool,
    /// Kick pages running in the background are trimmed (no chat or sidebars, lowest quality,
    /// small size), like Twitch's light player. Can be turned off if a channel stops counting.
    #[serde(default = "enabled")]
    pub light_kick_background: bool,
    /// Kick channels in priority order. While not empty, every slot keeps the
    /// KICK_LIVES_PER_SLOT highest ones that are live open (Kick only pays channel points for
    /// 2 lives at a time per account) and closes its other Kick channels.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kick_priority: Vec<String>,
    /// Twitch channels in priority order. While not empty, every slot keeps the
    /// `twitch_lives_per_slot` highest ones that are live open and closes its other Twitch
    /// channels (Twitch has no 2-stream limit on channel points).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub twitch_priority: Vec<String>,
    #[serde(default = "default_twitch_lives")]
    pub twitch_lives_per_slot: u8,
    /// Keep each priority list updated from the game's partner channels: new partners are
    /// added at the end as the game announces them.
    #[serde(default)]
    pub kick_sync: bool,
    #[serde(default)]
    pub twitch_sync: bool,
    /// Last seen channel points of each account on each channel, keyed
    /// "<slot>:<platform>:<channel>" (see `balance_key`). Each account prefers, among the live
    /// channels of the priority lists, the ones where it has the most points.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub balances: std::collections::HashMap<String, u64>,
    /// Whether each account follows each channel, as its pages last showed (same keys as
    /// `balances`). Only read from the pages: the app never follows anyone.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub follows: std::collections::HashMap<String, bool>,
    /// Claim Kick's Daily Reward once a day on every account (off by default).
    #[serde(default)]
    pub kick_daily: bool,
    /// The last day (days since 1970, UTC: Kick's daily rewards reset at 00:00 UTC) each slot's
    /// Kick Daily Reward was claimed or found already claimed.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub kick_daily_done: std::collections::HashMap<u8, i64>,
    /// The accounts reminded to vote on IdleHub (their header shows when the vote is ready).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vote_slots: Vec<u8>,
    /// Read from files of the single-account version, then moved into `vote_slots`.
    #[serde(default, skip_serializing)]
    pub vote_slot: Option<u8>,
    /// A fullscreen slot shows its stats panel at the right ("single focus").
    #[serde(default = "enabled")]
    pub focus_panel: bool,
    /// Economy mode: every slot is hidden (they keep running) and a light rest screen takes the
    /// game area, which saves the CPU spent drawing the games.
    #[serde(default)]
    pub economy: bool,
    /// Keyboard shortcuts by action ("" = off); see shortcuts.rs. Missing ones use defaults.
    #[serde(default)]
    pub shortcuts: std::collections::BTreeMap<String, String>,
    /// The app version whose patch notes were last shown; a newer app shows what changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seen_version: Option<String>,
    /// The app's language (the flags on the top bar). Portuguese unless changed.
    #[serde(default)]
    pub language: Language,
    /// Discord alerts (⚙ → Discord); off until a webhook is set.
    #[serde(default)]
    pub discord: crate::discord::DiscordConfig,
    /// Accounts whose Prey reminder the player dismissed, and for which game day (see prey.rs).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub prey_dismissed: std::collections::BTreeMap<u8, String>,
    /// The main window's last size and maximized state (it always opens centered).
    #[serde(default)]
    pub window: Option<WindowGeom>,
}

/// The main window's size (logical pixels) and whether it was maximized.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct WindowGeom {
    pub width: f64,
    pub height: f64,
    pub maximized: bool,
}

impl Default for WindowGeom {
    fn default() -> Self {
        WindowGeom { width: 1280.0, height: 800.0, maximized: false }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    #[serde(rename = "pt")]
    Pt,
    #[serde(rename = "en")]
    En,
    #[serde(rename = "es")]
    Es,
}

impl Language {
    /// Picks the text for this language.
    pub fn pick<'a>(self, en: &'a str, pt: &'a str, es: &'a str) -> &'a str {
        match self {
            Language::En => en,
            Language::Pt => pt,
            Language::Es => es,
        }
    }
}

pub fn balance_key(slot: u8, platform: Platform, channel: &str) -> String {
    let platform = match platform {
        Platform::Kick => "kick",
        Platform::Twitch => "twitch",
    };
    format!("{slot}:{platform}:{channel}")
}

fn default_twitch_lives() -> u8 {
    DEFAULT_TWITCH_LIVES
}

/// Kick channels each account keeps open while the priority list is in use.
pub const KICK_LIVES_PER_SLOT: usize = 2;

fn enabled() -> bool {
    true
}

/// Splitters in the middle: every slot the same size.
pub fn default_grid_split() -> GridSplit {
    GridSplit { v: 0.5, h: 0.5 }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            slots: Vec::new(),
            focus: None,
            grid_split: default_grid_split(),
            keep_awake: true,
            auto_reload_minutes: 0,
            close_to_tray: true,
            minimize_to_tray: false,
            toolbar_hidden: false,
            light_kick_background: true,
            kick_priority: Vec::new(),
            twitch_priority: Vec::new(),
            twitch_lives_per_slot: DEFAULT_TWITCH_LIVES,
            kick_sync: false,
            twitch_sync: false,
            balances: std::collections::HashMap::new(),
            follows: std::collections::HashMap::new(),
            kick_daily: false,
            kick_daily_done: std::collections::HashMap::new(),
            vote_slots: Vec::new(),
            vote_slot: None,
            focus_panel: true,
            economy: false,
            shortcuts: crate::shortcuts::defaults(),
            seen_version: None,
            language: Language::Pt,
            discord: Default::default(),
            prey_dismissed: Default::default(),
            window: None,
        }
    }
}

impl Config {
    /// Reads the file and normalizes it; a missing or broken file gives the defaults.
    pub fn load(path: &Path) -> Config {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Config>(&text).ok())
            .unwrap_or_default()
            .normalized()
    }

    /// Writes to a temp file first, then renames, so a crash never leaves half a file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(tmp, path)
    }

    pub fn normalized(mut self) -> Config {
        self.slots.retain(|s| (1..=MAX_SLOTS).contains(&s.id));
        self.slots.sort_by_key(|s| s.id);
        self.slots.dedup_by_key(|s| s.id);
        self.grid_split = GridSplit {
            v: clamp_split(self.grid_split.v, 0.5),
            h: clamp_split(self.grid_split.h, 0.5),
        };
        self.shortcuts = crate::shortcuts::normalized(&self.shortcuts);
        if !AUTO_RELOAD_OPTIONS.contains(&self.auto_reload_minutes) {
            self.auto_reload_minutes = 0;
        }
        for slot in &mut self.slots {
            slot.zoom = if slot.zoom.is_finite() { slot.zoom.clamp(0.5, 2.0) } else { 1.0 };
            slot.name = slot.name.as_deref().and_then(clean_name);
            // One mode at a time (a hand-edited file could tick both).
            if slot.streams_only {
                slot.game_only = false;
            }
            // Game only runs no streams: nothing to reopen.
            if slot.game_only {
                slot.panels.clear();
            }
            // No duplicate pages and at most MAX_PAGES_PER_PLATFORM per platform; the URL is
            // checked against the nav policy at restore.
            let mut seen: Vec<SavedPanel> = Vec::new();
            slot.panels.retain(|p| {
                let count = seen.iter().filter(|s| s.platform == p.platform).count();
                let keep = !seen.contains(p) && count < MAX_PAGES_PER_PLATFORM;
                if keep {
                    seen.push(p.clone());
                }
                keep
            });
        }
        if let Some(old) = self.vote_slot.take() {
            self.vote_slots.push(old);
        }
        let open: Vec<u8> = self.slots.iter().map(|s| s.id).collect();
        self.vote_slots.retain(|v| open.contains(v));
        self.vote_slots.sort_unstable();
        self.vote_slots.dedup();
        self.kick_priority = clean_kick_priority(&self.kick_priority);
        self.twitch_priority = clean_twitch_priority(&self.twitch_priority);
        self.twitch_lives_per_slot = self.twitch_lives_per_slot.clamp(1, MAX_PAGES_PER_PLATFORM as u8);
        // Balances of slots that no longer exist are dropped.
        let slots: Vec<String> = self.slots.iter().map(|s| format!("{}:", s.id)).collect();
        self.balances.retain(|key, _| slots.iter().any(|s| key.starts_with(s.as_str())));
        self.follows.retain(|key, _| slots.iter().any(|s| key.starts_with(s.as_str())));
        if !self.focus.is_some_and(|f| self.slots.iter().any(|s| s.id == f)) {
            self.focus = None;
        }
        self
    }

    pub fn next_free_id(&self) -> Option<u8> {
        (1..=MAX_SLOTS).find(|id| !self.slots.iter().any(|s| s.id == *id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(id: u8, zoom: f64) -> SlotConfig {
        SlotConfig { zoom, ..SlotConfig::new(id) }
    }

    #[test]
    fn keeps_several_saved_pages_without_duplicates_and_reads_old_files() {
        let panel = |platform, url: &str| SavedPanel { platform, url: url.into() };
        let mut slot = s(1, 1.0);
        slot.panels = vec![
            panel(Platform::Twitch, "https://www.twitch.tv/a"),
            panel(Platform::Kick, "https://kick.com/b"),
            panel(Platform::Twitch, "https://www.twitch.tv/c"),
            panel(Platform::Twitch, "https://www.twitch.tv/a"),
        ];
        let cfg = Config { slots: vec![slot], ..Config::default() }.normalized();
        assert_eq!(
            cfg.slots[0].panels,
            vec![
                panel(Platform::Twitch, "https://www.twitch.tv/a"),
                panel(Platform::Kick, "https://kick.com/b"),
                panel(Platform::Twitch, "https://www.twitch.tv/c"),
            ]
        );

        // At most MAX_PAGES_PER_PLATFORM per platform.
        let mut slot = s(1, 1.0);
        slot.panels = (0..12)
            .map(|i| panel(Platform::Kick, &format!("https://kick.com/channel{i}")))
            .collect();
        let cfg = Config { slots: vec![slot], ..Config::default() }.normalized();
        assert_eq!(cfg.slots[0].panels.len(), MAX_PAGES_PER_PLATFORM);

        // Files written before panels were saved still load.
        let old: Config = serde_json::from_str(r#"{"slots":[{"id":2,"zoom":0.9}]}"#).unwrap();
        assert!(old.slots[0].panels.is_empty());
    }

    #[test]
    fn normalizes_bad_values() {
        let cfg = Config {
            slots: vec![
                s(3, 9.0),
                s(7, 1.0),
                s(1, f64::NAN),
                s(3, 1.0),
            ],
            focus: Some(7),
            ..Config::default()
        }
        .normalized();
        assert_eq!(
            cfg.slots,
            vec![s(1, 1.0), s(3, 2.0)]
        );
        assert_eq!(cfg.focus, None);
    }

    #[test]
    fn slot_modes_load_and_stay_exclusive() {
        let cfg: Config = serde_json::from_str(
            r#"{"slots":[{"id":1,"zoom":1.0},{"id":2,"zoom":1.0,"game_only":true,"panels":[{"platform":"kick","url":"https://kick.com/a"}]},
                {"id":3,"zoom":1.0,"streams_only":true},{"id":4,"zoom":1.0,"streams_only":true,"game_only":true}]}"#,
        )
        .unwrap();
        let cfg = cfg.normalized();
        let modes: Vec<SlotMode> = cfg.slots.iter().map(SlotConfig::mode).collect();
        assert_eq!(modes, vec![SlotMode::Both, SlotMode::Game, SlotMode::Streams, SlotMode::Streams]);
        assert!(cfg.slots[1].panels.is_empty() && !cfg.slots[1].runs_streams());
        assert!(!cfg.slots[3].game_only);
        let mut slot = SlotConfig::new(1);
        slot.set_mode(SlotMode::Game);
        assert!(slot.game_only && !slot.streams_only);
        // The usual mode writes nothing new to the file.
        assert!(!serde_json::to_string(&SlotConfig::new(1)).unwrap().contains("_only"));
    }

    #[test]
    fn next_free_id_fills_gaps() {
        let cfg = Config {
            slots: vec![s(1, 1.0), s(3, 1.0)],
            ..Config::default()
        };
        assert_eq!(cfg.next_free_id(), Some(2));
    }
}

#[cfg(test)]
mod comfort_tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_capped() {
        assert_eq!(clean_name("  Main  "), Some("Main".into()));
        assert_eq!(clean_name("   "), None);
        assert_eq!(clean_name(&"x".repeat(60)).unwrap().chars().count(), MAX_NAME_CHARS);
    }

    #[test]
    fn normalizes_splits_and_reload() {
        let cfg: Config = serde_json::from_str(
            r#"{"grid_split":{"v":0.01,"h":2},"focus_split":-1,"auto_reload_minutes":7}"#,
        )
        .unwrap();
        let cfg = cfg.normalized();
        assert_eq!(cfg.grid_split, GridSplit { v: MIN_SPLIT, h: MAX_SPLIT });
        assert_eq!(cfg.auto_reload_minutes, 0);
    }

    #[test]
    fn old_files_get_comfort_defaults() {
        let cfg: Config = serde_json::from_str(r#"{"slots":[{"id":1,"zoom":1.0}]}"#).unwrap();
        assert!(cfg.keep_awake);
        assert!(!cfg.toolbar_hidden);
        assert!(cfg.light_kick_background);
        assert_eq!(cfg.auto_reload_minutes, 0);
        assert_eq!(cfg.slots[0].name, None);
        assert!(!cfg.slots[0].muted);
    }
}

#[cfg(test)]
mod kick_tests {
    use super::*;

    #[test]
    fn priority_list_is_cleaned() {
        let raw: Vec<String> = ["  AxT ", "https://kick.com/tftoddy?ref=x", "@Matta", "axt", "ab", "bad name", "kick.com/rodil/videos"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(clean_kick_priority(&raw), vec!["axt", "tftoddy", "matta", "rodil"]);
        let many: Vec<String> = (0..40).map(|i| format!("channel{i}")).collect();
        assert_eq!(clean_kick_priority(&many).len(), MAX_KICK_PRIORITY);
    }

    #[test]
    fn bonus_and_priority_load_and_default_off() {
        let cfg: Config = serde_json::from_str(
            r#"{"slots":[{"id":1,"zoom":1.0,"kick_bonus":"capture"},{"id":2,"zoom":1.0,"bonus":"lure"},{"id":3,"zoom":1.0}],
                "kick_priority":["AXT","axt","matta"],"twitch_priority":["Rodil","twitch.tv/kennzy","abc"],"twitch_lives_per_slot":20}"#,
        )
        .unwrap();
        let cfg = cfg.normalized();
        // Files from the Kick-only version keep their choice.
        assert_eq!(cfg.slots[0].bonus, Some(KickBonus::Capture));
        assert_eq!(cfg.slots[1].bonus, Some(KickBonus::Lure));
        assert_eq!(cfg.slots[2].bonus, None);
        assert_eq!(cfg.kick_priority, vec!["axt", "matta"]);
        assert_eq!(cfg.twitch_priority, vec!["rodil", "kennzy"]);
        assert_eq!(cfg.twitch_lives_per_slot, MAX_PAGES_PER_PLATFORM as u8);
        let saved = serde_json::to_string(&cfg).unwrap();
        assert!(saved.contains("\"bonus\":\"capture\"") && !saved.contains("kick_bonus"));
        let fresh = Config::default();
        assert!(fresh.kick_priority.is_empty() && fresh.twitch_priority.is_empty());
        assert_eq!(fresh.twitch_lives_per_slot, DEFAULT_TWITCH_LIVES);
        let old: Config = serde_json::from_str(r#"{"slots":[]}"#).unwrap();
        assert_eq!(old.twitch_lives_per_slot, DEFAULT_TWITCH_LIVES);
    }
}

#[cfg(test)]
mod tray_tests {
    use super::*;

    #[test]
    fn only_close_goes_to_tray_by_default() {
        let old: Config = serde_json::from_str(r#"{"slots":[]}"#).unwrap();
        assert!(old.close_to_tray && !old.minimize_to_tray);
        let off: Config =
            serde_json::from_str(r#"{"close_to_tray":false,"minimize_to_tray":false}"#).unwrap();
        assert!(!off.close_to_tray && !off.minimize_to_tray);
    }
}
