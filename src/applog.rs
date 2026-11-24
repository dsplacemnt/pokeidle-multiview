//! The app's log file, for finding problems on players' PCs: everything the app prints
//! (`println!`/`eprintln!` are redirected here by the macros at the top of main.rs) also goes,
//! with the time, to `<app data>/logs/app.log`. It rotates at 2 MB (the previous one is kept as
//! app.old.log).
//!
//! Each start writes a header (app, Windows and WebView2 versions) and notes when the previous
//! session didn't close normally (a crash, a forced close or the PC shutting down); a panic is
//! written before the app goes. ⚙ → About opens the folder or copies a diagnostic to paste in an
//! issue.
//!
//! Nothing secret goes in: the app never prints passwords, cookies or tokens, and every line is
//! still passed through `scrub` (a Discord webhook address or anything after "token=" is hidden).

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_BYTES: u64 = 2 * 1024 * 1024;
/// Lines in the copied diagnostic.
const DIAGNOSTIC_LINES: usize = 200;

struct Log {
    dir: PathBuf,
    file: File,
    written: u64,
}

static LOG: Mutex<Option<Log>> = Mutex::new(None);
/// Lines printed before the file is open (the start), written once it is.
static EARLY: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// The start header, repeated at the top of a copied diagnostic.
static HEADER: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The local time, "2026-11-24 09:23:50".
fn now() -> String {
    #[cfg(windows)]
    {
        #[repr(C)]
        #[derive(Default)]
        struct SystemTime {
            year: u16,
            month: u16,
            weekday: u16,
            day: u16,
            hour: u16,
            minute: u16,
            second: u16,
            ms: u16,
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetLocalTime(t: *mut SystemTime);
        }
        let mut t = SystemTime::default();
        unsafe { GetLocalTime(&mut t) };
        format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", t.year, t.month, t.day, t.hour, t.minute, t.second)
    }
    #[cfg(not(windows))]
    {
        let secs = crate::now_ms().div_euclid(1000);
        let (d, s) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
        format!("day {d} {:02}:{:02}:{:02} UTC", s / 3600, (s / 60) % 60, s % 60)
    }
}

/// Hides what must never be in a log, just in case: a Discord webhook address and token values.
pub fn scrub(text: &str) -> String {
    let mut out = text.to_string();
    // The user's folder carries their Windows/Linux user name.
    let home = std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default();
    if home.len() > 3 {
        out = out.replace(&home, if cfg!(windows) { "%USERPROFILE%" } else { "~" });
    }
    for marker in ["api/webhooks/", "token=", "Bearer "] {
        let mut from = 0;
        while let Some(i) = out[from..].find(marker) {
            let start = from + i + marker.len();
            let end = out[start..].find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '&').map_or(out.len(), |e| start + e);
            if end > start {
                out.replace_range(start..end, "[hidden]");
            }
            from = start + "[hidden]".len().min(out.len() - start);
        }
    }
    out
}

/// One printed line: to the terminal (debug builds) and to the log file.
pub fn out(text: String, error: bool) {
    if error {
        std::eprintln!("{text}");
    } else {
        std::println!("{text}");
    }
    let line = format!("{} {}{}", now(), if error { "ERROR " } else { "" }, scrub(&text));
    let mut log = LOG.lock().unwrap();
    match log.as_mut() {
        Some(log) => write(log, &line),
        None => EARLY.lock().unwrap().push(line),
    }
}

fn write(log: &mut Log, line: &str) {
    if log.written > MAX_BYTES {
        rotate(log);
    }
    if writeln!(log.file, "{line}").is_ok() {
        log.written += line.len() as u64 + 1;
    }
}

fn rotate(log: &mut Log) {
    let current = log.dir.join("app.log");
    let old = log.dir.join("app.old.log");
    let _ = std::fs::remove_file(&old);
    let _ = std::fs::rename(&current, &old);
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(&current) {
        log.file = file;
        log.written = 0;
    }
}

/// Opens the log (at startup, once the app's data folder is known) and writes the header.
pub fn start(data_dir: &std::path::Path) {
    let dir = data_dir.join("logs");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("app.log");
    let written = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) else { return };
    let mut log = Log { dir: dir.clone(), file, written };
    if log.written > MAX_BYTES {
        rotate(&mut log);
    }
    let header = vec![
        String::new(),
        format!("==== PokeIdle Multiview {} — started {}", env!("CARGO_PKG_VERSION"), now()),
        format!("system: {}", os_version()),
        format!("WebView2: {}", tauri::webview_version().unwrap_or_else(|e| format!("unknown ({e})"))),
    ];
    for line in &header {
        let _ = writeln!(log.file, "{line}");
    }
    // A session that didn't reach `closing` left its mark.
    let running = dir.join("running");
    if running.exists() {
        let _ = writeln!(
            log.file,
            "{} the previous session didn't close normally (a crash, a forced close or the PC shutting down)",
            now()
        );
    }
    let _ = std::fs::write(&running, env!("CARGO_PKG_VERSION"));
    for line in EARLY.lock().unwrap().drain(..) {
        let _ = writeln!(log.file, "{line}");
    }
    *HEADER.lock().unwrap() = header.into_iter().skip(1).collect();
    *LOG.lock().unwrap() = Some(log);
}

/// The app is about to close on purpose (Quit, a restart, an update): not a crash next time.
pub fn closing(why: &str) {
    let dir = LOG.lock().unwrap().as_ref().map(|l| l.dir.clone());
    out(format!("[app] closing: {why}"), false);
    if let Some(dir) = dir {
        let _ = std::fs::remove_file(dir.join("running"));
    }
}

/// A panic goes in the log before the app does.
pub fn catch_panics() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        out(format!("[panic] {info}"), true);
        default(info);
    }));
}

fn os_version() -> String {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct OsVersion {
            size: u32,
            major: u32,
            minor: u32,
            build: u32,
            platform: u32,
            csd: [u16; 128],
        }
        #[link(name = "ntdll")]
        unsafe extern "system" {
            fn RtlGetVersion(v: *mut OsVersion) -> i32;
        }
        let mut v = OsVersion { size: std::mem::size_of::<OsVersion>() as u32, major: 0, minor: 0, build: 0, platform: 0, csd: [0; 128] };
        unsafe { RtlGetVersion(&mut v) };
        // Windows 11 still says 10.0; its builds start at 22000.
        let name = if v.major == 10 && v.build >= 22000 { "Windows 11" } else if v.major == 10 { "Windows 10" } else { "Windows" };
        format!("{name} ({}.{}.{})", v.major, v.minor, v.build)
    }
    #[cfg(not(windows))]
    {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|t| t.lines().find(|l| l.starts_with("PRETTY_NAME=")).map(|l| l[12..].trim_matches('"').to_string()))
            .unwrap_or_else(|| std::env::consts::OS.to_string())
    }
}

/// ⚙ → About → "Open the logs folder".
#[tauri::command]
pub fn open_logs_folder() -> Result<(), String> {
    let dir = LOG.lock().unwrap().as_ref().map(|l| l.dir.clone()).ok_or("The log isn't open")?;
    tauri_plugin_opener::open_path(dir.to_string_lossy().to_string(), None::<&str>).map_err(|e| e.to_string())
}

/// ⚙ → About → "Copy diagnostics": the header and the last lines of the log, to paste in an
/// issue or on Discord.
#[tauri::command]
pub fn diagnostics() -> String {
    let path = LOG.lock().unwrap().as_ref().map(|l| l.dir.join("app.log"));
    let text = path.and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let tail = &lines[lines.len().saturating_sub(DIAGNOSTIC_LINES)..];
    let mut out = String::from("```\n");
    for h in HEADER.lock().unwrap().iter() {
        out.push_str(h);
        out.push('\n');
    }
    out.push_str(&format!("--- last {} lines of the log ---\n", tail.len()));
    for l in tail {
        out.push_str(&scrub(l));
        out.push('\n');
    }
    out.push_str("```\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_hidden() {
        assert_eq!(scrub("posting to https://discord.com/api/webhooks/123/abcDEF now"), "posting to https://discord.com/api/webhooks/[hidden] now");
        assert_eq!(scrub("url?token=abc123&x=1"), "url?token=[hidden]&x=1");
        assert_eq!(scrub("Authorization: Bearer abc.def"), "Authorization: Bearer [hidden]");
        assert_eq!(scrub("[health] slot-1-twitch-2: ok"), "[health] slot-1-twitch-2: ok");
        assert_eq!(scrub("token= "), "token= ");
        let home = std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap();
        assert!(!scrub(&format!("loading {home}/x")).contains(&home));
    }
}
