//! Outside Windows, the WebKitGTK side of what native.rs does with WebView2:
//! - permission requests: only notifications are allowed (no camera, microphone, location…);
//! - muting a page through WebKit itself, so the page can't override it;
//! - shortcuts pressed inside a page: WebKitGTK has no accelerator hook, so an init script in
//!   the slot pages reports them through a brief title change (see `KEYS_SCRIPT`);
//! - a page whose web process crashed reloads itself.
//! The low memory level is a WebView2 setting with no WebKit counterpart.

use tauri::AppHandle;

#[cfg(target_os = "linux")]
pub fn attach(webview: &tauri::Webview, _app: AppHandle) {
    let label = webview.label().to_string();
    let _ = webview.with_webview(move |platform| {
        use webkit2gtk::glib::ObjectExt;
        use webkit2gtk::{PermissionRequestExt, WebViewExt};
        let view = platform.inner();
        view.connect_permission_request(|_, request| {
            let allow = request.is::<webkit2gtk::NotificationPermissionRequest>();
            if allow {
                request.allow();
            } else {
                request.deny();
            }
            true
        });
        let crash_label = label.clone();
        view.connect_web_process_terminated(move |view, reason| {
            eprintln!("[{crash_label}] web process ended ({reason:?}); reloading");
            view.reload();
        });
    });
}

#[cfg(not(target_os = "linux"))]
pub fn attach(_webview: &tauri::Webview, _app: AppHandle) {}

/// Places a webview at `r` (logical px) inside the main window.
///
/// Tauri packs a window's child webviews into a vertical GtkBox on Linux, which stacks them
/// and ignores their position and size. So the first time a webview is placed, it moves into a
/// GtkFixed (created once, filling that box) where every page is put at its own spot.
#[cfg(target_os = "linux")]
pub fn place(webview: &tauri::Webview, r: crate::layout::Rect) {
    let _ = webview.with_webview(move |platform| {
        use gtk::prelude::*;
        thread_local! {
            static FIXED: std::cell::RefCell<Option<gtk::Fixed>> = const { std::cell::RefCell::new(None) };
        }
        let view = platform.inner();
        let widget: gtk::Widget = view.clone().upcast();
        FIXED.with(|cell| {
            let mut cell = cell.borrow_mut();
            let in_fixed = widget.parent().is_some_and(|p| p.is::<gtk::Fixed>());
            if !in_fixed {
                let Some(parent) = widget.parent() else { return };
                let fixed = match cell.as_ref() {
                    Some(f) => f.clone(),
                    None => {
                        let fixed = gtk::Fixed::new();
                        if let Some(vbox) = parent.downcast_ref::<gtk::Box>() {
                            vbox.pack_start(&fixed, true, true, 0);
                        }
                        fixed.show();
                        *cell = Some(fixed.clone());
                        fixed
                    }
                };
                if let Some(container) = parent.downcast_ref::<gtk::Container>() {
                    container.remove(&widget);
                }
                fixed.put(&widget, 0, 0);
            }
            if let Some(fixed) = cell.as_ref() {
                fixed.move_(&widget, r.x.round() as i32, r.y.round() as i32);
            }
            widget.set_size_request(r.w.round().max(1.0) as i32, r.h.round().max(1.0) as i32);
        });
    });
}

#[cfg(not(target_os = "linux"))]
pub fn place(_webview: &tauri::Webview, _r: crate::layout::Rect) {}

/// WebView2 only.
pub fn set_low_memory(_webview: &tauri::Webview, _low: bool) {}

/// Mutes or unmutes a page through WebKit.
#[cfg(target_os = "linux")]
pub fn set_muted(webview: &tauri::Webview, muted: bool) {
    let _ = webview.with_webview(move |platform| {
        use webkit2gtk::WebViewExt;
        platform.inner().set_is_muted(muted);
    });
}

#[cfg(not(target_os = "linux"))]
pub fn set_muted(_webview: &tauri::Webview, _muted: bool) {}

/// Injected into the game and stream pages outside Windows: a pressed combination that could
/// be a shortcut ("Ctrl+G", written like shortcuts.rs writes them) is reported through a brief
/// title change ("[multiview-key] Ctrl+G"); the app checks it against the configured shortcuts.
pub const KEYS_SCRIPT: &str = r#"
(() => {
  if (window.top !== window || window.__multiviewKeys) return;
  window.__multiviewKeys = true;
  document.addEventListener('keydown', (e) => {
    let key = null;
    if (/^Key[A-Z]$/.test(e.code)) key = e.code.slice(3);
    else if (/^Digit[0-9]$/.test(e.code)) key = e.code.slice(5);
    else if (/^F([1-9]|1[0-2])$/.test(e.code)) key = e.code;
    if (!key) return;
    if (!(key.startsWith('F') && key.length > 1) && !e.ctrlKey && !e.altKey) return;
    const combo = (e.ctrlKey ? 'Ctrl+' : '') + (e.altKey ? 'Alt+' : '') + (e.shiftKey ? 'Shift+' : '') + key;
    const title = document.title;
    document.title = '[multiview-key] ' + combo;
    setTimeout(() => { if (document.title.startsWith('[multiview-key]')) document.title = title; }, 120);
  }, true);
})();
"#;
