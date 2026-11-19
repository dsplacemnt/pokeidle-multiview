//! WebView2 events the slot pages need that Tauri doesn't expose:
//! - permission requests: only notifications and autoplay are allowed (no camera,
//!   microphone, location, clipboard…);
//! - Ctrl+1…Ctrl+4 pressed inside a page focuses that slot, like on the toolbar;
//! - a page whose renderer crashed or stopped responding reloads itself (at most 3 times in
//!   5 minutes, so a page that keeps crashing doesn't loop forever).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::AppHandle;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{AcceleratorKeyPressedEventHandler, PermissionRequestedEventHandler, ProcessFailedEventHandler};

#[link(name = "user32")]
unsafe extern "system" {
    fn GetKeyState(virtual_key: i32) -> i16;
}

const VK_CONTROL: i32 = 0x11;
const VK_SHIFT: i32 = 0x10;
const VK_MENU: i32 = 0x12;
const VK_1: u32 = 0x31;
const VK_4: u32 = 0x34;

const CRASH_RELOAD_LIMIT: u32 = 3;
const CRASH_WINDOW: Duration = Duration::from_secs(300);

static CRASHES: Mutex<Option<HashMap<String, (Instant, u32)>>> = Mutex::new(None);

/// Whether a page that just crashed may be reloaded now (records the attempt).
fn may_reload(label: &str) -> bool {
    let mut guard = CRASHES.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    let now = Instant::now();
    let entry = map.entry(label.to_string()).or_insert((now, 0));
    if now.duration_since(entry.0) > CRASH_WINDOW {
        *entry = (now, 0);
    }
    entry.1 += 1;
    entry.1 <= CRASH_RELOAD_LIMIT
}

fn key_down(vk: i32) -> bool {
    unsafe { GetKeyState(vk) < 0 }
}

/// Hooks the events above onto a slot webview (game or Twitch/Kick page).
pub fn attach(webview: &tauri::Webview, app: AppHandle) {
    let label = webview.label().to_string();
    let result = webview.with_webview(move |platform_webview| unsafe {
        let controller = platform_webview.controller();
        let Ok(core) = controller.CoreWebView2() else { return };
        let mut token = 0i64;

        let permissions = PermissionRequestedEventHandler::create(Box::new(|_, args| {
            let Some(args) = args else { return Ok(()) };
            let mut kind = COREWEBVIEW2_PERMISSION_KIND::default();
            args.PermissionKind(&mut kind)?;
            let allow = kind == COREWEBVIEW2_PERMISSION_KIND_NOTIFICATIONS
                || kind == COREWEBVIEW2_PERMISSION_KIND_AUTOPLAY;
            if !allow {
                eprintln!("[permission] denied kind {}", kind.0);
            }
            args.SetState(if allow {
                COREWEBVIEW2_PERMISSION_STATE_ALLOW
            } else {
                COREWEBVIEW2_PERMISSION_STATE_DENY
            })
        }));
        if let Err(e) = core.add_PermissionRequested(&permissions, &mut token) {
            eprintln!("[{label}] could not watch permissions: {e}");
        }

        let crash_label = label.clone();
        let crashes = ProcessFailedEventHandler::create(Box::new(move |sender, args| {
            let (Some(sender), Some(args)) = (sender, args) else { return Ok(()) };
            let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
            args.ProcessFailedKind(&mut kind)?;
            let page_died = kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED
                || kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE;
            if !page_died {
                eprintln!("[{crash_label}] WebView2 process failed (kind {})", kind.0);
                return Ok(());
            }
            if may_reload(&crash_label) {
                eprintln!("[{crash_label}] page crashed or froze (kind {}); reloading", kind.0);
                sender.Reload()?;
            } else {
                eprintln!("[{crash_label}] page keeps crashing; not reloading again for now");
            }
            Ok(())
        }));
        if let Err(e) = core.add_ProcessFailed(&crashes, &mut token) {
            eprintln!("[{label}] could not watch crashes: {e}");
        }

        let keys_app = app.clone();
        let keys = AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else { return Ok(()) };
            let mut kind = COREWEBVIEW2_KEY_EVENT_KIND::default();
            args.KeyEventKind(&mut kind)?;
            if kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN {
                return Ok(());
            }
            let mut vk = 0u32;
            args.VirtualKey(&mut vk)?;
            if (VK_1..=VK_4).contains(&vk)
                && key_down(VK_CONTROL)
                && !key_down(VK_SHIFT)
                && !key_down(VK_MENU)
            {
                args.SetHandled(true)?;
                let id = (vk - VK_1 + 1) as u8;
                let app = keys_app.clone();
                // Layout changes create/move webviews: not from inside this event.
                tauri::async_runtime::spawn(async move { crate::toggle_focus(&app, Some(id)) });
            }
            Ok(())
        }));
        if let Err(e) = controller.add_AcceleratorKeyPressed(&keys, &mut token) {
            eprintln!("[{label}] could not watch shortcuts: {e}");
        }
    });
    if let Err(e) = result {
        eprintln!("[native] could not reach webview: {e}");
    }
}
