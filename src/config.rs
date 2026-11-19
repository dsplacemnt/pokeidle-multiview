//! Persisted settings: slots (name, zoom, mute, streams), layout, tray and comfort options.

use std::path::Path;

use serde::{Deserialize, Serialize};

pub const MAX_SLOTS: u8 = 4;

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SlotConfig {
    pub id: u8,
    pub zoom: f64,
    /// Shown on the slot's chip; None = "Slot N".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The game page's sound is off (its Twitch/Kick pages are muted anyway when hidden).
    #[serde(default)]
    pub muted: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub panels: Vec<SavedPanel>,
}

impl SlotConfig {
    pub fn new(id: u8) -> SlotConfig {
        SlotConfig { id, zoom: 1.0, name: None, muted: false, panels: Vec::new() }
    }
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
}

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
        if !AUTO_RELOAD_OPTIONS.contains(&self.auto_reload_minutes) {
            self.auto_reload_minutes = 0;
        }
        for slot in &mut self.slots {
            slot.zoom = if slot.zoom.is_finite() { slot.zoom.clamp(0.5, 2.0) } else { 1.0 };
            slot.name = slot.name.as_deref().and_then(clean_name);
            // One page per platform; the URL is checked against the nav policy at restore.
            let mut seen = Vec::new();
            slot.panels.retain(|p| {
                let fresh = !seen.contains(&p.platform);
                seen.push(p.platform);
                fresh
            });
        }
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
    fn keeps_one_saved_panel_per_platform_and_reads_old_files() {
        let panel = |platform, url: &str| SavedPanel { platform, url: url.into() };
        let mut slot = s(1, 1.0);
        slot.panels = vec![
            panel(Platform::Twitch, "https://www.twitch.tv/a"),
            panel(Platform::Kick, "https://kick.com/b"),
            panel(Platform::Twitch, "https://www.twitch.tv/c"),
        ];
        let cfg = Config { slots: vec![slot], ..Config::default() }.normalized();
        assert_eq!(
            cfg.slots[0].panels,
            vec![
                panel(Platform::Twitch, "https://www.twitch.tv/a"),
                panel(Platform::Kick, "https://kick.com/b"),
            ]
        );

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
        assert_eq!(cfg.auto_reload_minutes, 0);
        assert_eq!(cfg.slots[0].name, None);
        assert!(!cfg.slots[0].muted);
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
