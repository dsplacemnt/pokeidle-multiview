//! "Apoiar o dev": the developer's Discord and a Pix code (with its QR, drawn here so nothing
//! is fetched) for players who like the app and want to chip in. Only shown when asked for:
//! the ♥ button on the toolbar, ⚙ → About, or the tray menu.

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewUrl};

pub const DISCORD: &str = "dsplacemnt";
/// The developer's Discord profile (opens Discord's page or app).
pub const DISCORD_URL: &str = "https://discord.com/users/1191794452147208262";
pub const GITHUB: &str = "https://github.com/dsplacemnt/pokeidle-multiview";
/// The developer's PokeIdle referral link, for players to share with friends who don't play yet
/// (free way to support). Only copied or opened when asked; slots never load it.
pub const REFERRAL: &str = "https://pokeidle.io/?ref=DBCHW36";
/// Static Pix "copia e cola" (BR Code): random key, no fixed amount (the payer picks it).
pub const PIX: &str = "00020126580014BR.GOV.BCB.PIX0136384baaaf-07f9-495b-96d0-f365b7ac99e55204000053039865802BR5924Thainan Henrique Feistel6009SAO PAULO62140510TlU51tMQRA6304856D";

const WINDOW: &str = "support";

pub fn title(lang: crate::config::Language) -> &'static str {
    lang.pick("Support the dev — PokeIdle Multiview", "Apoiar o dev — PokeIdle Multiview", "Apoyar al dev — PokeIdle Multiview")
}

#[derive(Serialize)]
pub struct SupportInfo {
    discord: &'static str,
    github: &'static str,
    referral: &'static str,
    pix: &'static str,
    /// The Pix code as a QR image (SVG markup).
    qr: String,
    version: &'static str,
}

fn qr_svg(text: &str) -> String {
    use qrcode::render::svg;
    // Medium error correction: still scans from a phone pointed at a screen.
    match qrcode::QrCode::with_error_correction_level(text, qrcode::EcLevel::M) {
        Ok(code) => code
            .render::<svg::Color>()
            .min_dimensions(240, 240)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .quiet_zone(true)
            .build(),
        Err(_) => String::new(),
    }
}

#[tauri::command]
pub fn support_info() -> SupportInfo {
    SupportInfo {
        discord: DISCORD,
        github: GITHUB,
        referral: REFERRAL,
        pix: PIX,
        qr: qr_svg(PIX),
        version: env!("CARGO_PKG_VERSION"),
    }
}

/// Opens (or brings back) the support window, centered on the app.
#[tauri::command]
pub async fn open_support(app: AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(WINDOW) {
        let _ = existing.unminimize();
        return existing.set_focus().map_err(|e| e.to_string());
    }
    let main = app.get_window("main").ok_or("No window")?;
    let (w, h) = (440.0, 840.0);
    let lang = app.state::<std::sync::Mutex<crate::AppState>>().lock().unwrap().config.language;
    let builder = tauri::WebviewWindowBuilder::new(&app, WINDOW, WebviewUrl::App("support.html".into()))
        .title(title(lang))
        .inner_size(w, h);
    let builder = match crate::centered_over(&main, w, h) {
        Some((x, y)) => builder.position(x, y),
        None => builder.center(),
    };
    let builder = match crate::local_args() {
        Some(args) => builder.additional_browser_args(args),
        None => builder,
    };
    builder
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .owner_raw(main.hwnd().map_err(|e| e.to_string())?)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// The project page, in the default browser.
#[tauri::command]
pub fn open_github() {
    if let Ok(url) = GITHUB.parse() {
        crate::external::open_in_browser(&url);
    }
}

/// The developer's Discord profile, in the default browser.
#[tauri::command]
pub fn open_discord() {
    if let Ok(url) = DISCORD_URL.parse() {
        crate::external::open_in_browser(&url);
    }
}

/// The referral link, in the default browser (to see it or sign up a new account there).
#[tauri::command]
pub fn open_referral() {
    if let Ok(url) = REFERRAL.parse() {
        crate::external::open_in_browser(&url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CRC16-CCITT (0x1021, start 0xFFFF), as the BR Code spec uses for its last field.
    fn crc16(data: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;
        for &b in data {
            crc ^= u16::from(b) << 8;
            for _ in 0..8 {
                crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
            }
        }
        crc
    }

    #[test]
    fn pix_code_checksum_is_valid() {
        let (body, crc) = PIX.split_at(PIX.len() - 4);
        assert!(body.ends_with("6304"));
        assert_eq!(format!("{:04X}", crc16(body.as_bytes())), crc);
    }

    /// Regenerates the QR shown in the README: `cargo test -- --ignored write_readme_qr`.
    #[test]
    #[ignore]
    fn write_readme_qr() {
        std::fs::create_dir_all("docs").unwrap();
        std::fs::write("docs/pix-qr.svg", qr_svg(PIX)).unwrap();
    }

    #[test]
    fn pix_qr_renders() {
        assert!(qr_svg(PIX).starts_with("<?xml") || qr_svg(PIX).contains("<svg"));
    }
}
