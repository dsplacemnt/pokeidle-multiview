//! The game's daily Prey: each account gets a free roll of its Prey card every day at 18:00
//! (Brasília), which reveals 9 Pokémon, a bonus and its stars; the player then picks one of
//! the 9, and the bonus works while hunting that Pokémon. The game page reports the account's
//! cards (see game-watch.js: "[multiview-prey] {…}"); the app reminds the player when today's
//! free roll or the pick is still to do, and shows the active Prey in the stats panel.
//!
//! The app only reads: it never rolls or picks.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Deserialize;
use tauri::{AppHandle, Manager};

/// One card as the page reports it.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct Card {
    #[serde(default)]
    pub rolled: bool,
    #[serde(default)]
    pub pk: Option<i64>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Report {
    /// The game day of the cards ("2026-11-24"); None = nothing rolled yet.
    #[serde(default)]
    pub day: Option<String>,
    /// Card 1, and card 2 if the account unlocked it (None = locked).
    #[serde(default)]
    pub cards: Vec<Option<Card>>,
}

/// The game's day: it turns at 18:00 Brasília (21:00 UTC).
pub fn game_day(now_ms: i64) -> String {
    let days = (now_ms + 3 * 3_600_000).div_euclid(86_400_000);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Days since 1970-01-01 → (year, month, day) (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { yoe + era * 400 + 1 } else { yoe + era * 400 }, m, d)
}

/// What's left to do today: "roll" (the free roll), "pick" (choose one of the 9), or None.
pub fn pending(report: &Report, now_ms: i64) -> Option<&'static str> {
    let today = game_day(now_ms);
    let unlocked = report.cards.iter().enumerate().filter(|(i, c)| *i == 0 || c.is_some()).count().max(1);
    if report.day.as_deref() != Some(today.as_str()) {
        return Some("roll");
    }
    let mut pick = false;
    for i in 0..unlocked {
        match report.cards.get(i).and_then(|c| c.as_ref()) {
            None => return Some("roll"),
            Some(c) if !c.rolled => return Some("roll"),
            Some(c) if c.pk.is_none() => pick = true,
            _ => {}
        }
    }
    pick.then_some("pick")
}

/// An account's pending Prey step, unless the player dismissed today's Prey for it (they may
/// not want any of the 9: picking is free, but another roll costs Coins).
pub fn pending_for(state: &crate::AppState, id: u8, now_ms: i64) -> Option<&'static str> {
    if state.config.prey_dismissed.get(&id).is_some_and(|d| *d == game_day(now_ms)) {
        return None;
    }
    state.prey.get(&id).and_then(|(report, _)| pending(report, now_ms))
}

/// Each account's pending Prey step, for the pages: slot → "roll" / "pick".
pub fn pendings(state: &crate::AppState) -> HashMap<u8, String> {
    let now = crate::now_ms();
    state.prey.keys().filter_map(|id| pending_for(state, *id, now).map(|p| (*id, p.to_string()))).collect()
}

/// "Not today": hides the account's Prey reminder until the game day turns (18:00 Brasília).
#[tauri::command]
pub fn dismiss_prey(app: AppHandle, id: u8) {
    let state = app.state::<Mutex<crate::AppState>>();
    let mut state = state.lock().unwrap();
    let day = game_day(crate::now_ms());
    state.config.prey_dismissed.retain(|_, d| *d == day);
    state.config.prey_dismissed.insert(id, day);
    println!("[prey] slot {id}: dismissed for today");
    crate::log_event(&mut state, id, "Prey: dismissed for today");
    crate::save_and_publish(&app, &state);
}

/// "[multiview-prey] {…}" from a game page: kept (as parsed and as sent, for the panel).
pub fn reported(app: &AppHandle, id: u8, json: &str) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else { return };
    let Ok(report) = serde_json::from_value::<Report>(value.clone()) else { return };
    let state = app.state::<Mutex<crate::AppState>>();
    let mut state = state.lock().unwrap();
    let before = state.prey.get(&id).and_then(|(r, _)| pending(r, crate::now_ms()));
    let after = pending(&report, crate::now_ms());
    if before != after {
        let text = match after {
            Some("roll") => "Prey: today's free roll is ready",
            Some(_) => "Prey: pick one of the 9 Pokémon",
            None => "Prey: done for today",
        };
        println!("[prey] slot {id}: {text}");
        crate::log_event(&mut state, id, text);
    }
    state.prey.insert(id, (report, value));
    crate::discord::check_prey(&mut state);
    crate::publish(app, &state);
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-11-23 20:59:59 UTC (17:59:59 Brasília) and one second later.
    const BEFORE_SIX: i64 = 1_795_467_599_000;
    const AT_SIX: i64 = 1_795_467_600_000;

    #[test]
    fn the_day_turns_at_six_pm_in_brasilia() {
        assert_eq!(game_day(BEFORE_SIX), "2026-11-23");
        assert_eq!(game_day(AT_SIX), "2026-11-24");
        assert_eq!(game_day(0), "1970-01-01");
    }

    fn card(rolled: bool, pk: Option<i64>) -> Option<Card> {
        Some(Card { rolled, pk })
    }

    #[test]
    fn what_is_left_to_do() {
        let done = Report { day: Some("2026-11-24".into()), cards: vec![card(true, Some(13)), None] };
        assert_eq!(pending(&done, AT_SIX), None);
        // The next day: rolled yesterday doesn't count.
        assert_eq!(pending(&done, AT_SIX + 86_400_000), Some("roll"));
        let unpicked = Report { day: Some("2026-11-24".into()), cards: vec![card(true, None), None] };
        assert_eq!(pending(&unpicked, AT_SIX), Some("pick"));
        let never = Report { day: None, cards: vec![card(false, None), None] };
        assert_eq!(pending(&never, AT_SIX), Some("roll"));
        // Card 2 unlocked and not rolled yet.
        let two = Report { day: Some("2026-11-24".into()), cards: vec![card(true, Some(13)), card(false, None)] };
        assert_eq!(pending(&two, AT_SIX), Some("roll"));
    }
}
