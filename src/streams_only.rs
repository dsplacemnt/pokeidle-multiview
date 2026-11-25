//! "Streams only" accounts: the account's game runs on another computer, and this app runs
//! only its Twitch and Kick streams here (points, chests, redeems). The bonuses follow the
//! Twitch/Kick login linked in the game, not the computer the game runs on, so they still
//! reach the account. The game itself can't open here too: the game keeps one connection per
//! account and would log the other computer out.
//!
//! Without the game, nobody tells the app when the bonuses end, so it reckons them from its
//! own redeems: each PokeIdle reward says its hours in its title ("Bônus +15% XP PokeIdle" is
//! 1 hour, "… (3h)" is 3), and the game adds the hours up. The estimate is saved with the
//! slot, so it survives restarts. A redeem made by hand elsewhere isn't counted.

use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use crate::config::{Language, Platform, SlotConfig};
use crate::{AppState, BonusEnds};

const HOUR_MS: i64 = 3_600_000;

/// What a redeem report bought: (bonus 0 = XP, 1 = Capture, 2 = Lure; hours). The scripts
/// report "redeemed <reward title>…" (kick-redeem.js, twitch-visit.js).
pub fn redeemed(message: &str) -> Option<(usize, i64)> {
    let what = message.split("redeemed ").nth(1)?;
    let lower = what.to_lowercase();
    let kind = if lower.contains("capture") {
        1
    } else if lower.contains("lure") {
        2
    } else if lower.contains("xp") {
        0
    } else {
        return None;
    };
    Some((kind, hours_in(&lower).unwrap_or(1)))
}

/// "(3h)" / "(3 h)" in a reward title (the price, "(600)", has no h).
fn hours_in(text: &str) -> Option<i64> {
    let mut rest = text;
    while let Some(i) = rest.find('(') {
        rest = &rest[i + 1..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() && rest[digits.len()..].trim_start().starts_with('h') {
            return digits.parse().ok().filter(|h| (1..=168).contains(h));
        }
    }
    None
}

/// The ends after buying `hours` of bonus `kind` at `now`: hours add up from what's left.
pub fn add(mut ends: [i64; 3], kind: usize, hours: i64, now: i64) -> [i64; 3] {
    ends[kind] = ends[kind].max(now) + hours * HOUR_MS;
    ends
}

/// The streams-only slots' saved estimates as the running bonuses the rest of the app reads
/// (slot headers, economy mode, redeem goals). Both platforms are always there: "no bonus"
/// rather than "the game hasn't said yet".
pub fn sync_ends(state: &mut AppState) {
    let slots: Vec<(u8, [i64; 3], [i64; 3])> = state
        .config
        .slots
        .iter()
        .filter(|s| s.streams_only)
        .map(|s| (s.id, estimate(s, Platform::Kick), estimate(s, Platform::Twitch)))
        .collect();
    for (id, kick, twitch) in slots {
        for (platform, e) in [(Platform::Kick, kick), (Platform::Twitch, twitch)] {
            state.bonus_ends.insert((id, platform), BonusEnds { xp: e[0], capture: e[1], lure: e[2] });
        }
    }
}

fn estimate(slot: &SlotConfig, platform: Platform) -> [i64; 3] {
    slot.bonus_estimate.get(platform_key(platform)).copied().unwrap_or([0; 3])
}

fn platform_key(platform: Platform) -> &'static str {
    match platform {
        Platform::Kick => "kick",
        Platform::Twitch => "twitch",
    }
}

/// A Twitch/Kick page of slot `id` reported "redeemed …": if the slot is streams only, the
/// bonus's estimated end moves on. Returns the line for the account's activity log.
pub fn redeem_reported(state: &mut AppState, id: u8, platform: Platform, message: &str, now: i64) -> Option<String> {
    let (kind, hours) = redeemed(message)?;
    let slot = state.config.slots.iter_mut().find(|s| s.id == id && s.streams_only)?;
    let before = estimate(slot, platform);
    let after = add(before, kind, hours, now);
    slot.bonus_estimate.insert(platform_key(platform).to_string(), after);
    sync_ends(state);
    if let Err(e) = state.config.save(&state.config_path) {
        eprintln!("[config] could not save: {e}");
    }
    let name = ["XP", "Capture", "Lure"][kind];
    let left = (after[kind] - now) / 60_000;
    Some(format!("≈ {name} +{hours}h (estimated: {}h{:02} left)", left / 60, left % 60))
}

/// Turning streams only on: the estimate starts from what the game last said, when that's
/// longer than what the app reckoned before.
pub fn seed(state: &mut AppState, id: u8) {
    let known: Vec<(Platform, BonusEnds)> = [Platform::Kick, Platform::Twitch]
        .into_iter()
        .filter_map(|p| state.bonus_ends.get(&(id, p)).map(|e| (p, *e)))
        .collect();
    let Some(slot) = state.config.slots.iter_mut().find(|s| s.id == id) else { return };
    for (platform, e) in known {
        let saved = estimate(slot, platform);
        let merged = [saved[0].max(e.xp), saved[1].max(e.capture), saved[2].max(e.lure)];
        slot.bonus_estimate.insert(platform_key(platform).to_string(), merged);
    }
}

/// Streams-only accounts redeem a fixed bonus: without the game the times are only estimates,
/// too rough for goals. An account with goals keeps its first goal's bonus.
pub fn fixed_only(slot: &mut SlotConfig) {
    if !slot.streams_only {
        return;
    }
    if let Some(goals) = slot.goals.take() {
        slot.bonus = goals.list.first().map(|g| g.bonus).or(slot.bonus);
    }
}

/// The page in the game's place: what this slot is, in the app's language. A data: page (no
/// IPC, nothing loaded from anywhere), in the slot's own session so its logins stay readable.
pub fn page_url(lang: Language) -> String {
    let (title, body, hint, note) = match lang {
        Language::Pt => (
            "Só lives",
            "O jogo desta conta roda em outro computador. Aqui ela só assiste às lives da Twitch e da Kick, abre os baús e resgata os bônus, que chegam à conta do jogo pelo login vinculado.",
            "As pílulas no cabeçalho acima mostram uma live aqui.",
            "Sem o jogo aqui, os tempos de bônus são estimados pelos resgates que este app faz.",
        ),
        Language::Es => (
            "Solo streams",
            "El juego de esta cuenta corre en otra computadora. Aquí solo mira los streams de Twitch y Kick, abre los cofres y canjea los bonos, que llegan a la cuenta del juego por el login vinculado.",
            "Las píldoras del encabezado de arriba muestran un stream aquí.",
            "Sin el juego aquí, los tiempos de bono se estiman por los canjes que hace esta app.",
        ),
        Language::En => (
            "Streams only",
            "This account's game runs on another computer. Here it only watches its Twitch and Kick streams, opens the chests and redeems the bonuses, which reach the game account through its linked login.",
            "The pills in the header above show a stream here.",
            "With no game here, the bonus times are estimated from the redeems this app makes.",
        ),
    };
    let html = format!(
        r##"<!doctype html><html><head><meta charset="utf-8"><title>{title}</title><style>
html,body{{margin:0;height:100%;background:#16181d;color:#e6e8ee;font:14px/1.5 "Segoe UI",system-ui,sans-serif;user-select:none}}
body{{display:grid;place-items:center;padding:24px;box-sizing:border-box;
background:radial-gradient(ellipse at 50% 40%,#1c2029 0%,#16181d 65%)}}
.card{{max-width:460px;text-align:center;display:grid;gap:10px;justify-items:center}}
.icon{{width:64px;height:64px;border-radius:16px;display:grid;place-items:center;background:#1d2027;
box-shadow:0 6px 18px rgba(0,0,0,.25)}}
h1{{margin:4px 0 0;font-size:18px;font-weight:600;color:#f2c14e}}
p{{margin:0;color:#b9bfcc}} .hint{{color:#8b91a1;font-size:13px}} .note{{color:#6f7686;font-size:12px}}
</style></head><body><div class="card">
<div class="icon"><svg width="34" height="34" viewBox="0 0 24 24" fill="none" stroke="#f2c14e" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="6" width="18" height="12" rx="2.5"/><path d="M10 9.5v5l4.2-2.5z" fill="#f2c14e" stroke="none"/><path d="M8 3l4 3 4-3"/></svg></div>
<h1>{title}</h1><p>{body}</p><p class="hint">{hint}</p><p class="note">{note}</p>
</div></body></html>"##
    );
    format!("data:text/html;charset=utf-8,{}", percent_encode(&html))
}

fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The language changed: the streams-only pages follow.
pub fn set_language(app: &AppHandle, lang: Language) {
    let ids: Vec<u8> = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();
        state.config.slots.iter().filter(|s| s.streams_only).map(|s| s.id).collect()
    };
    for id in ids {
        if let Some(page) = app.get_webview(&crate::slot_label(id)) {
            if let Ok(url) = page_url(lang).parse() {
                let _ = page.navigate(url);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_a_redeem_bought() {
        assert_eq!(redeemed("Kick redeem: redeemed Bônus +15% XP PokeIdle 200"), Some((0, 1)));
        assert_eq!(redeemed("Kick redeem: redeemed Bônus +15% XP PokeIdle (3h) 600"), Some((0, 3)));
        assert_eq!(redeemed("Twitch visit: chest claimed, redeemed Bônus de +15% Capture Boost (500)"), Some((1, 1)));
        assert_eq!(redeemed("Twitch visit: redeemed Bônus de +15% Secret Lure (2h) (2000)"), Some((2, 2)));
        assert_eq!(redeemed("Kick redeem: not enough points (10/200)"), None);
        assert_eq!(redeemed("Twitch visit: chest claimed"), None);
    }

    #[test]
    fn hours_add_up_from_what_is_left() {
        let now = 10 * HOUR_MS;
        // Nothing running: starts now.
        assert_eq!(add([0, 0, 0], 0, 1, now), [11 * HOUR_MS, 0, 0]);
        // Running: added at the end.
        assert_eq!(add([12 * HOUR_MS, 0, 0], 0, 3, now), [15 * HOUR_MS, 0, 0]);
        // Ended in the past: starts now.
        assert_eq!(add([0, 5 * HOUR_MS, 0], 1, 2, now), [0, 12 * HOUR_MS, 0]);
    }

    #[test]
    fn the_page_is_a_data_url() {
        let url = page_url(Language::Pt);
        assert!(url.starts_with("data:text/html;charset=utf-8,"));
        assert!(url.parse::<tauri::Url>().is_ok());
        assert!(!url.contains(' ') && !url.contains('#'));
    }
}
