//! Keeps Windows from going to sleep while background streams run (when enabled), so the
//! streams keep counting. The display may still turn off.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::AppHandle;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetThreadExecutionState(flags: u32) -> u32;
}

const ES_CONTINUOUS: u32 = 0x8000_0000;
const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;

static AWAKE: AtomicBool = AtomicBool::new(false);

/// Applies the wanted state. The request belongs to the calling thread, so it is always made
/// from the main (UI) thread, which lives as long as the app.
pub fn set_keep_awake(app: &AppHandle, wanted: bool) {
    if AWAKE.swap(wanted, Ordering::SeqCst) == wanted {
        return;
    }
    let flags = if wanted { ES_CONTINUOUS | ES_SYSTEM_REQUIRED } else { ES_CONTINUOUS };
    let result = app.run_on_main_thread(move || unsafe {
        SetThreadExecutionState(flags);
    });
    match result {
        Ok(()) => println!("[power] keep awake: {wanted}"),
        Err(e) => eprintln!("[power] could not change sleep setting: {e}"),
    }
}
