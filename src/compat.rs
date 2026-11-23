//! Outside Windows: stand-ins for the two Windows-only window calls the app makes, so the same
//! code builds everywhere. An "owned" window (settings, support, patch notes, vote) stays in
//! front of the main one on Windows; elsewhere it is just a normal window.

use tauri::{Manager, Runtime};

pub trait HwndCompat {
    fn hwnd(&self) -> tauri::Result<()>;
}

impl<R: Runtime> HwndCompat for tauri::Window<R> {
    fn hwnd(&self) -> tauri::Result<()> {
        Ok(())
    }
}

pub trait OwnerCompat {
    fn owner_raw(self, owner: ()) -> Self;
}

impl<'a, R: Runtime, M: Manager<R>> OwnerCompat for tauri::webview::WebviewWindowBuilder<'a, R, M> {
    fn owner_raw(self, _owner: ()) -> Self {
        self
    }
}

impl<'a, R: Runtime, M: Manager<R>> OwnerCompat for tauri::window::WindowBuilder<'a, R, M> {
    fn owner_raw(self, _owner: ()) -> Self {
        self
    }
}
