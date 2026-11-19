//! Persisted settings: which slots are open, their zoom and the focused slot.

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub panels: Vec<SavedPanel>,
}

impl SlotConfig {
    pub fn new(id: u8) -> SlotConfig {
        SlotConfig { id, zoom: 1.0, panels: Vec::new() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub slots: Vec<SlotConfig>,
    #[serde(default)]
    pub focus: Option<u8>,
    /// × hides the window to the tray (streams keep running); "Quit" in the tray menu exits.
    #[serde(default = "enabled")]
    pub close_to_tray: bool,
    /// Minimizing hides the window to the tray instead of the taskbar.
    #[serde(default = "enabled")]
    pub minimize_to_tray: bool,
}

fn enabled() -> bool {
    true
}

impl Default for Config {
    fn default() -> Config {
        Config { slots: Vec::new(), focus: None, close_to_tray: true, minimize_to_tray: true }
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
        for slot in &mut self.slots {
            slot.zoom = if slot.zoom.is_finite() { slot.zoom.clamp(0.5, 2.0) } else { 1.0 };
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
mod tray_tests {
    use super::*;

    #[test]
    fn tray_options_default_on_even_for_old_files() {
        let old: Config = serde_json::from_str(r#"{"slots":[]}"#).unwrap();
        assert!(old.close_to_tray && old.minimize_to_tray);
        let off: Config =
            serde_json::from_str(r#"{"close_to_tray":false,"minimize_to_tray":false}"#).unwrap();
        assert!(!off.close_to_tray && !off.minimize_to_tray);
    }
}
