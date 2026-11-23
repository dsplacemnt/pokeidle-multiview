//! Discord alerts: the app posts to a channel webhook the player sets in ⚙ → Discord, so they
//! can follow their accounts from the phone. Only what the app already sees goes out: a shiny
//! seen, caught or fled; an account's game that dropped out; a stream the app couldn't fix by
//! itself; an IdleHub vote ready; and, once a day at the hour chosen, a short report. Nothing
//! is sent until a webhook is set, and each kind can be turned off.
//!
//! The webhook address is a secret of the player's channel: it's kept in the config file only,
//! never logged, and only sent to Discord.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::config::Language;

fn on() -> bool {
    true
}

/// ⚙ → Discord.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DiscordConfig {
    /// The channel's webhook ("" = alerts off).
    #[serde(default)]
    pub webhook: String,
    #[serde(default = "on")]
    pub shinies: bool,
    /// An account's game dropped out (logged in elsewhere, disconnected, no session).
    #[serde(default = "on")]
    pub game: bool,
    /// A stream with a problem the app couldn't fix.
    #[serde(default = "on")]
    pub streams: bool,
    /// An IdleHub vote is ready (reminded accounts only).
    #[serde(default = "on")]
    pub vote: bool,
    /// The daily report's local hour (None = no report).
    #[serde(default)]
    pub report_hour: Option<u8>,
    /// The PC's offset from UTC in minutes (east positive), sent by the settings page.
    #[serde(default)]
    pub utc_offset_min: i32,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        DiscordConfig { webhook: String::new(), shinies: true, game: true, streams: true, vote: true, report_hour: None, utc_offset_min: 0 }
    }
}

/// A Discord webhook address (and nothing else: the app never posts anywhere else).
pub fn valid_webhook(url: &str) -> bool {
    let url = url.trim();
    ["https://discord.com/api/webhooks/", "https://discordapp.com/api/webhooks/", "https://ptb.discord.com/api/webhooks/", "https://canary.discord.com/api/webhooks/"]
        .iter()
        .any(|p| url.starts_with(p) && url.len() > p.len() + 10)
        && !url.contains(char::is_whitespace)
}

/// What was already sent, so each thing goes out once.
#[derive(Default)]
pub struct Sent {
    /// (slot, when the shiny was met, result).
    shinies: HashSet<(u8, i64, String)>,
    /// The game problem each account was last reported with.
    issues: HashMap<u8, String>,
    /// The vote time each account was last reported ready for.
    votes: HashMap<u8, i64>,
    /// The last local day a report went out.
    report_day: i64,
}

#[derive(Clone, Copy)]
pub enum Kind {
    Shiny,
    Game,
    Stream,
    Vote,
    Report,
}

const COLOR_SHINY: u32 = 0xF2C14E;
const COLOR_BAD: u32 = 0xFF6B6B;
const COLOR_OK: u32 = 0x3DDC84;
const COLOR_INFO: u32 = 0x5AAAFF;

fn tr(lang: Language, en: &str, pt: &str, es: &str) -> String {
    match lang {
        Language::En => en,
        Language::Pt => pt,
        Language::Es => es,
    }
    .to_string()
}

fn account(state: &crate::AppState, id: u8) -> String {
    state
        .config
        .slots
        .iter()
        .find(|s| s.id == id)
        .and_then(|s| s.name.clone())
        .unwrap_or_else(|| format!("Slot {id}"))
}

/// Posts an alert of this kind if the player wants it. Takes the state already locked by the
/// caller; the post itself runs on its own thread.
fn post(state: &crate::AppState, kind: Kind, title: String, text: String, color: u32) {
    let d = &state.config.discord;
    let wanted = match kind {
        Kind::Shiny => d.shinies,
        Kind::Game => d.game,
        Kind::Stream => d.streams,
        Kind::Vote => d.vote,
        Kind::Report => d.report_hour.is_some(),
    };
    if !wanted || !valid_webhook(&d.webhook) {
        return;
    }
    let url = d.webhook.trim().to_string();
    std::thread::spawn(move || {
        if let Err(e) = send(&url, &title, &text, color) {
            eprintln!("[discord] could not post an alert: {e}");
        }
    });
}

/// One message with one embed. Errors say what Discord answered, never the address.
fn send(url: &str, title: &str, text: &str, color: u32) -> Result<(), String> {
    let body = serde_json::json!({
        "username": "PokeIdle Multiview",
        "embeds": [{ "title": title, "description": text, "color": color }],
        "allowed_mentions": { "parse": [] },
    });
    let response = crate::net::agent()
        .post(url)
        .header("User-Agent", crate::net::USER_AGENT)
        .header("Content-Type", "application/json")
        .config()
        .http_status_as_error(false)
        .build()
        .send(body.to_string())
        .map_err(|e| format!("network error: {}", e.to_string().replace(url, "<webhook>")))?;
    match response.status().as_u16() {
        200..=299 => Ok(()),
        401 | 403 | 404 => Err("Discord didn't accept this webhook (deleted, or a wrong address?)".into()),
        429 => Err("Discord asked to slow down; try again in a minute".into()),
        s => Err(format!("Discord answered {s}")),
    }
}

// ------------------------------------------------------------------ the alerts

/// A game page sent its hunt numbers: new shinies in its list go out.
pub fn shinies_reported(state: &mut crate::AppState, id: u8, stats: &serde_json::Value) {
    let Some(list) = stats.get("shinyList").and_then(|l| l.as_array()) else { return };
    let now = crate::now_ms();
    let lang = state.config.language;
    let mut fresh = Vec::new();
    for s in list {
        let at = s.get("at").and_then(|v| v.as_i64()).unwrap_or(0);
        let result = s.get("result").and_then(|v| v.as_str()).unwrap_or("seen").to_string();
        if !state.discord_sent.shinies.insert((id, at, result.clone())) {
            continue;
        }
        // Only what just happened (a page reload doesn't resend older ones it kept).
        if now - at > 10 * 60_000 {
            continue;
        }
        let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        let level = s.get("level").and_then(|v| v.as_i64());
        fresh.push((name, level, result));
    }
    for (name, level, result) in fresh {
        let who = account(state, id);
        let lv = level.map(|l| format!(" Lv {l}")).unwrap_or_default();
        let (title, color) = match result.as_str() {
            "caught" => (tr(lang, "✨ Shiny caught!", "✨ Shiny capturado!", "✨ ¡Shiny capturado!"), COLOR_OK),
            "fled" => (tr(lang, "Shiny fled", "O shiny fugiu", "El shiny huyó"), COLOR_BAD),
            _ => (tr(lang, "✨ Shiny spotted!", "✨ Shiny à vista!", "✨ ¡Shiny a la vista!"), COLOR_SHINY),
        };
        post(state, Kind::Shiny, title, format!("**{who}**: {name}{lv}"), color);
    }
}

/// The game problems of every account: a new one goes out once; one that clears is forgotten
/// (and said to be fixed).
pub fn check_games(state: &mut crate::AppState) {
    let now = crate::now_ms();
    let lang = state.config.language;
    let ids: Vec<u8> = state.config.slots.iter().map(|s| s.id).collect();
    for id in ids {
        let issue = state.game_conn.get(&id).and_then(|c| crate::game::issue(c, now)).map(str::to_string);
        let before = state.discord_sent.issues.get(&id).cloned();
        if issue == before {
            continue;
        }
        match &issue {
            Some(text) => {
                state.discord_sent.issues.insert(id, text.clone());
                let who = account(state, id);
                let title = tr(lang, "⚠ The game needs you", "⚠ O jogo precisa de você", "⚠ El juego te necesita");
                post(state, Kind::Game, title, format!("**{who}**: {}", game_text(lang, text)), COLOR_BAD);
            }
            None => {
                state.discord_sent.issues.remove(&id);
                let who = account(state, id);
                let title = tr(lang, "Game back", "Jogo de volta", "Juego de vuelta");
                let text = tr(lang, "is connected again", "está conectado de novo", "está conectado de nuevo");
                post(state, Kind::Game, title, format!("**{who}** {text}"), COLOR_OK);
            }
        }
    }
}

fn game_text(lang: Language, en: &str) -> String {
    match en {
        "Not logged in to the game" => tr(lang, en, "Sem login no jogo", "Sin sesión en el juego"),
        "Logged in somewhere else: this game was disconnected" => tr(
            lang,
            en,
            "Conta logada em outro lugar: este jogo foi desconectado",
            "Cuenta iniciada en otro lugar: este juego se desconectó",
        ),
        "The game lost its connection" => tr(lang, en, "O jogo perdeu a conexão", "El juego perdió la conexión"),
        other => other.to_string(),
    }
}

/// A stream got a warning the app couldn't fix by itself.
pub fn stream_warning(state: &crate::AppState, id: u8, channel: &str, warning: &str) {
    let lang = state.config.language;
    let who = account(state, id);
    let title = tr(lang, "⚠ A stream needs you", "⚠ Uma live precisa de você", "⚠ Un stream te necesita");
    post(state, Kind::Stream, title, format!("**{who}** · {channel}: {warning}"), COLOR_BAD);
}

/// Every minute: votes that became ready, and the daily report at its hour.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(60));
        let state = app.state::<Mutex<crate::AppState>>();
        let mut state = state.lock().unwrap();
        tick(&mut state);
    });
}

fn tick(state: &mut crate::AppState) {
    let now = crate::now_ms();
    let lang = state.config.language;
    // Votes ready (reminded accounts only), once per vote.
    let ready: Vec<(u8, i64)> = state
        .config
        .vote_slots
        .iter()
        .filter_map(|id| state.vote.get(id).map(|(v, _)| (*id, v.ready_at)))
        .filter(|(_, at)| *at > 0 && *at <= now)
        .collect();
    for (id, at) in ready {
        if state.discord_sent.votes.get(&id) == Some(&at) {
            continue;
        }
        state.discord_sent.votes.insert(id, at);
        let who = account(state, id);
        let title = tr(lang, "★ IdleHub vote ready", "★ Voto do IdleHub liberado", "★ Voto de IdleHub disponible");
        let text = tr(lang, "can vote now", "já pode votar", "ya puede votar");
        post(state, Kind::Vote, title, format!("**{who}** {text}"), COLOR_INFO);
    }
    // The daily report, once a day at the chosen local hour.
    if let Some(hour) = state.config.discord.report_hour {
        let local = now + i64::from(state.config.discord.utc_offset_min) * 60_000;
        let day = local.div_euclid(86_400_000);
        let local_hour = (local.rem_euclid(86_400_000) / 3_600_000) as u8;
        if local_hour == hour && state.discord_sent.report_day != day {
            state.discord_sent.report_day = day;
            let text = report(state);
            let title = tr(lang, "📋 Daily report", "📋 Relatório do dia", "📋 Informe del día");
            post(state, Kind::Report, title, text, COLOR_INFO);
        }
    }
}

/// One line per account: game state, level, pace, bonuses running and shinies.
fn report(state: &crate::AppState) -> String {
    let lang = state.config.language;
    let now = crate::now_ms();
    let issues = crate::game::issues(state);
    let mut lines = Vec::new();
    for slot in &state.config.slots {
        let id = slot.id;
        let mut parts = Vec::new();
        if let Some(issue) = issues.get(&id) {
            parts.push(format!("⚠ {}", game_text(lang, issue)));
        }
        if let Some(g) = state.game_stats.get(&id) {
            if let Some(t) = g.get("trainer") {
                let level = t.get("level").and_then(|v| v.as_i64()).unwrap_or(0);
                let pct = t.get("pct").and_then(|v| v.as_f64());
                parts.push(match pct {
                    Some(p) => format!("Lv {level} ({p:.1}%)"),
                    None => format!("Lv {level}"),
                });
            }
            if let Some(xp) = g.get("xpPerHour").and_then(|v| v.as_f64()) {
                parts.push(format!("{} XP/h", compact(xp)));
            }
            if let Some(gold) = g.get("netPerHour").and_then(|v| v.as_f64()) {
                parts.push(format!("{}{} gold/h", if gold >= 0.0 { "+" } else { "" }, compact(gold)));
            }
            let shinies = g.get("shinyList").and_then(|l| l.as_array()).map_or(0, |l| l.len());
            if shinies > 0 {
                parts.push(format!("✨ {shinies}"));
            }
        }
        // Bonuses still running, the longest of each kind across Twitch and Kick.
        let mut bonus = Vec::new();
        for (label, pick) in [("XP", 0usize), ("CAP", 1), ("LURE", 2)] {
            let end = [crate::config::Platform::Twitch, crate::config::Platform::Kick]
                .iter()
                .filter_map(|p| state.bonus_ends.get(&(id, *p)))
                .map(|b| [b.xp, b.capture, b.lure][pick])
                .max()
                .unwrap_or(0);
            if end > now {
                bonus.push(format!("{label} {}h", (end - now) / 3_600_000));
            }
        }
        if !bonus.is_empty() {
            parts.push(bonus.join(" "));
        }
        let name = account(state, id);
        lines.push(if parts.is_empty() {
            format!("**{name}**: {}", tr(lang, "no numbers yet", "ainda sem números", "aún sin números"))
        } else {
            format!("**{name}**: {}", parts.join(" · "))
        });
    }
    if lines.is_empty() {
        tr(lang, "No accounts open.", "Nenhuma conta aberta.", "Ninguna cuenta abierta.")
    } else {
        lines.join("\n")
    }
}

fn compact(n: f64) -> String {
    let a = n.abs();
    if a >= 1e9 {
        format!("{:.1}B", n / 1e9)
    } else if a >= 1e6 {
        format!("{:.1}M", n / 1e6)
    } else if a >= 1e3 {
        format!("{:.1}K", n / 1e3)
    } else {
        format!("{}", n.round() as i64)
    }
}

// ------------------------------------------------------------------ settings

/// ⚙ → Discord: what is set (the webhook only as "set or not" plus its end, never whole).
#[derive(Serialize, Clone)]
pub struct DiscordView {
    webhook_set: bool,
    /// The last characters, so the player can tell which webhook it is.
    webhook_hint: Option<String>,
    shinies: bool,
    game: bool,
    streams: bool,
    vote: bool,
    report_hour: Option<u8>,
}

fn view(d: &DiscordConfig) -> DiscordView {
    let set = valid_webhook(&d.webhook);
    DiscordView {
        webhook_set: set,
        webhook_hint: set.then(|| {
            let w = d.webhook.trim();
            format!("…{}", &w[w.len().saturating_sub(6)..])
        }),
        shinies: d.shinies,
        game: d.game,
        streams: d.streams,
        vote: d.vote,
        report_hour: d.report_hour,
    }
}

#[tauri::command]
pub fn get_discord(state: tauri::State<'_, Mutex<crate::AppState>>) -> DiscordView {
    view(&state.lock().unwrap().config.discord)
}

/// What ⚙ → Discord sends. `webhook`: None = keep the current one, "" = remove it.
#[derive(Deserialize)]
pub struct DiscordChange {
    webhook: Option<String>,
    shinies: bool,
    game: bool,
    streams: bool,
    vote: bool,
    report_hour: Option<u8>,
    utc_offset_min: i32,
}

#[tauri::command]
pub fn set_discord(app: AppHandle, change: DiscordChange) -> Result<DiscordView, String> {
    if let Some(w) = &change.webhook {
        if !w.trim().is_empty() && !valid_webhook(w) {
            return Err("That isn't a Discord webhook address (it starts with https://discord.com/api/webhooks/)".into());
        }
    }
    let state = app.state::<Mutex<crate::AppState>>();
    let mut state = state.lock().unwrap();
    let d = &mut state.config.discord;
    if let Some(w) = change.webhook {
        d.webhook = w.trim().to_string();
    }
    d.shinies = change.shinies;
    d.game = change.game;
    d.streams = change.streams;
    d.vote = change.vote;
    d.report_hour = change.report_hour.filter(|h| *h < 24);
    d.utc_offset_min = change.utc_offset_min.clamp(-14 * 60, 14 * 60);
    let out = view(&state.config.discord);
    if let Err(e) = state.config.save(&state.config_path) {
        eprintln!("[config] could not save: {e}");
    }
    Ok(out)
}

/// "Send a test": posts a test message to the given webhook (or the saved one).
#[tauri::command]
pub async fn test_discord(app: AppHandle, webhook: Option<String>) -> Result<(), String> {
    let (url, lang) = {
        let state = app.state::<Mutex<crate::AppState>>();
        let state = state.lock().unwrap();
        let url = webhook.filter(|w| !w.trim().is_empty()).unwrap_or_else(|| state.config.discord.webhook.clone());
        (url.trim().to_string(), state.config.language)
    };
    if !valid_webhook(&url) {
        return Err("That isn't a Discord webhook address (it starts with https://discord.com/api/webhooks/)".into());
    }
    let title = tr(lang, "PokeIdle Multiview is connected", "PokeIdle Multiview conectado", "PokeIdle Multiview conectado");
    let text = tr(
        lang,
        "Alerts from the app will show up in this channel.",
        "Os alertas do app vão aparecer neste canal.",
        "Las alertas de la app aparecerán en este canal.",
    );
    tauri::async_runtime::spawn_blocking(move || send(&url, &title, &text, COLOR_OK))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_discord_webhooks() {
        assert!(valid_webhook("https://discord.com/api/webhooks/123456789/abcdefghijk"));
        assert!(valid_webhook(" https://discordapp.com/api/webhooks/123456789/abcdefghijk "));
        assert!(!valid_webhook("https://discord.com/api/webhooks/"));
        assert!(!valid_webhook("https://example.com/api/webhooks/123456789/abcdefghijk"));
        assert!(!valid_webhook("http://discord.com/api/webhooks/123456789/abcdefghijk"));
        assert!(!valid_webhook("https://discord.com.evil.io/api/webhooks/123456789/abc"));
    }

    #[test]
    fn compact_numbers() {
        assert_eq!(compact(950.0), "950");
        assert_eq!(compact(89_183_861.0), "89.2M");
        assert_eq!(compact(-2_723_270.0), "-2.7M");
    }
}
