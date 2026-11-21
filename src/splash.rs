//! Loading screen: a small window with the logo, each account's game and the saved streams
//! reopening. The main window stays hidden meanwhile (no white panels), and shows once every
//! game page has finished its first load and the streams are open again, or after `GIVE_UP`
//! whatever happens.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl};

use crate::config::SlotConfig;

const WINDOW: &str = "splash";
/// When everything is ready: a moment for the last page to paint before the window shows.
const SETTLE: Duration = Duration::from_millis(600);
/// The main window shows by then even if a game never reports.
const GIVE_UP: Duration = Duration::from_secs(45);

#[derive(Serialize, Clone, Default)]
pub struct SplashView {
    /// The accounts being loaded: [id, name].
    slots: Vec<(u8, String)>,
    /// The ones whose game finished loading.
    loaded: Vec<u8>,
    /// Saved streams reopened so far, of how many (None until known).
    streams: Option<(usize, usize)>,
    done: bool,
}

impl SplashView {
    fn ready(&self) -> bool {
        let games = self.slots.iter().all(|(id, _)| self.loaded.contains(id));
        let streams = self.streams.is_some_and(|(done, total)| done >= total);
        games && streams
    }
}

static VIEW: Mutex<Option<SplashView>> = Mutex::new(None);
static DONE: AtomicBool = AtomicBool::new(false);

fn snapshot() -> SplashView {
    VIEW.lock().unwrap().clone().unwrap_or_default()
}

/// Opens the loading screen for these accounts (none = the main window shows right away).
pub fn open(app: &AppHandle, slots: &[SlotConfig]) {
    let view = SplashView {
        slots: slots
            .iter()
            .map(|s| (s.id, s.name.clone().unwrap_or_else(|| format!("Slot {}", s.id))))
            .collect(),
        ..Default::default()
    };
    *VIEW.lock().unwrap() = Some(view);
    if slots.is_empty() {
        finish(app);
        return;
    }
    let (w, h) = (440.0, 320.0);
    let builder = tauri::WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::App("splash.html".into()))
        .title("PokeIdle Multiview")
        .inner_size(w, h)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .center()
        .background_color(tauri::window::Color(0, 0, 0, 0));
    let builder = match crate::local_args() {
        Some(args) => builder.additional_browser_args(args),
        None => builder,
    };
    if let Err(e) = builder.build() {
        eprintln!("[splash] could not open: {e}");
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(GIVE_UP);
        if !DONE.load(Ordering::SeqCst) {
            println!("[splash] still loading after {}s, showing the window anyway", GIVE_UP.as_secs());
            finish(&app);
        }
    });
}

/// Records a change and, once everything is ready, ends the loading screen.
fn update(app: &AppHandle, f: impl FnOnce(&mut SplashView)) {
    if DONE.load(Ordering::SeqCst) {
        return;
    }
    let ready = {
        let mut guard = VIEW.lock().unwrap();
        let Some(view) = guard.as_mut() else { return };
        f(view);
        view.ready()
    };
    let _ = app.emit_to(WINDOW, "splash", snapshot());
    if ready {
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(SETTLE);
            finish(&app);
        });
    }
}

/// A game page finished loading (the first time counts).
pub fn game_loaded(app: &AppHandle, id: u8) {
    update(app, |view| {
        if !view.loaded.contains(&id) {
            view.loaded.push(id);
        }
    });
}

/// Saved streams reopened so far (see restore_panels).
pub fn streams_progress(app: &AppHandle, done: usize, total: usize) {
    update(app, |view| view.streams = Some((done, total)));
}

/// Shows the main window and closes the loading screen (once).
fn finish(app: &AppHandle) {
    if DONE.swap(true, Ordering::SeqCst) {
        return;
    }
    println!("[splash] done, showing the main window");
    if let Some(view) = VIEW.lock().unwrap().as_mut() {
        view.done = true;
    }
    let _ = app.emit_to(WINDOW, "splash", snapshot());
    let main = app.clone();
    // Window calls belong on the main thread.
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = main.get_window("main") {
            // A window saved minimized would come back minimized: after loading, it's in view.
            let _ = window.unminimize();
            // Opens centered on the screen at its last size, then maximized if it was (so
            // restoring it lands in the middle too).
            let maximized = main
                .state::<Mutex<crate::AppState>>()
                .lock()
                .unwrap()
                .config
                .window
                .is_some_and(|g| g.maximized);
            let _ = window.center();
            let _ = window.show();
            if maximized {
                let _ = window.maximize();
            }
            let _ = window.set_focus();
        }
    });
    // Streams created while the window was hidden didn't start playing (the browser doesn't
    // autoplay in a hidden window): now that it's shown, they're started.
    let resume = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(1500));
        crate::resume_paused_streams(&resume);
    });
    // The splash fades out (splash.html), then closes.
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        let splash = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(window) = splash.get_webview_window(WINDOW) {
                let _ = window.close();
            }
        });
    });
}

#[tauri::command]
pub fn get_splash() -> SplashView {
    snapshot()
}
