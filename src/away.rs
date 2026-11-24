//! "Away" instead of minimized or hidden in the tray: the main window stays shown but moves
//! off-screen. WebView2 stops rendering the pages of a minimized or hidden window, and a Twitch
//! player that loads or restarts then never starts again (Kick's does), so the health checks
//! reloaded those pages all night long. Off-screen, every page keeps running as if watched.
//!
//! Minimizing keeps the taskbar button (clicking it brings the window back); the tray removes
//! it (the tray icon brings the window back). Either way the window returns where it was, with
//! the same size and maximized state.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Mutex;

use tauri::Window;

// Filled in by Windows (a Win32 layout): not every field is read.
#[allow(dead_code)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Point {
    x: i32,
    y: i32,
}

// Filled in by Windows (a Win32 layout): not every field is read.
#[allow(dead_code)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct WinRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct WindowPlacement {
    length: u32,
    flags: u32,
    show_cmd: u32,
    min_position: Point,
    max_position: Point,
    normal_position: WinRect,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetWindowPlacement(hwnd: *mut c_void, placement: *mut WindowPlacement) -> i32;
    fn SetWindowPlacement(hwnd: *mut c_void, placement: *const WindowPlacement) -> i32;
    fn ShowWindow(hwnd: *mut c_void, cmd: i32) -> i32;
    fn IsIconic(hwnd: *mut c_void) -> i32;
    fn SetWindowPos(
        hwnd: *mut c_void,
        after: *mut c_void,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32;
    fn SetWinEventHook(
        event_min: u32,
        event_max: u32,
        module: *mut c_void,
        callback: WinEventProc,
        process: u32,
        thread: u32,
        flags: u32,
    ) -> *mut c_void;
    fn GetForegroundWindow() -> *mut c_void;
    fn SetForegroundWindow(hwnd: *mut c_void) -> i32;
    fn GetShellWindow() -> *mut c_void;
}

type WinEventProc = unsafe extern "system" fn(*mut c_void, u32, *mut c_void, i32, i32, u32, u32);

const EVENT_SYSTEM_MINIMIZESTART: u32 = 0x0016;
const WINEVENT_OUTOFCONTEXT: u32 = 0;
const WINEVENT_SKIPOWNPROCESS: u32 = 2;
/// SetWindowPos "insert after": the bottom of the window stack.
const HWND_BOTTOM: *mut c_void = 1 as *mut c_void;

const SW_SHOWNORMAL: u32 = 1;
const SW_SHOWMINIMIZED: u32 = 2;
const SW_SHOWMAXIMIZED: u32 = 3;
const SW_SHOWNOACTIVATE: u32 = 4;
const SW_MINIMIZE: i32 = 6;
const WPF_RESTORETOMAXIMIZED: u32 = 2;
const SWP_NOSIZE: u32 = 0x0001;
const SWP_NOMOVE: u32 = 0x0002;
const SWP_NOACTIVATE: u32 = 0x0010;

/// Far left of any screen setup (Windows parks minimized windows at -32000).
const AWAY_X: i32 = -30000;

struct Away {
    /// Where the window was (its normal rect) and whether it was maximized.
    placement: WindowPlacement,
    maximized: bool,
    /// Sent to the tray (no taskbar button) rather than minimized.
    tray: bool,
}

static AWAY: Mutex<Option<Away>> = Mutex::new(None);
/// Set while the window is being moved, so the resize events this causes are ignored.
static MOVING: AtomicBool = AtomicBool::new(false);

fn hwnd(window: &Window) -> Option<*mut c_void> {
    window.hwnd().ok().map(|h| h.0)
}

fn placement(hwnd: *mut c_void) -> Option<WindowPlacement> {
    let mut wp = WindowPlacement {
        length: size_of::<WindowPlacement>() as u32,
        ..Default::default()
    };
    (unsafe { GetWindowPlacement(hwnd, &mut wp) } != 0).then_some(wp)
}

/// Whether the window is away (off-screen, standing for minimized or in the tray).
pub fn is_away() -> bool {
    AWAY.lock().unwrap().is_some()
}

/// Whether the window is being moved away or back right now.
pub fn moving() -> bool {
    MOVING.load(Ordering::SeqCst)
}

/// Whether the window is away in the tray (no taskbar button).
pub fn in_tray() -> bool {
    AWAY.lock().unwrap().as_ref().is_some_and(|a| a.tray)
}

/// Moves the window off-screen. `tray`: also removes its taskbar button. Already away: only
/// the taskbar button changes.
pub fn send(window: &Window, tray: bool) {
    let Some(hwnd) = hwnd(window) else { return };
    MOVING.store(true, Ordering::SeqCst);
    // No lock is held across the window calls below: they send events that read this state.
    let already = {
        let mut away = AWAY.lock().unwrap();
        away.as_mut().map(|a| a.tray = tray).is_some()
    };
    if !already {
        if let Some(wp) = placement(hwnd) {
            let iconic = unsafe { IsIconic(hwnd) } != 0;
            let maximized = if iconic || wp.show_cmd == SW_SHOWMINIMIZED {
                wp.flags & WPF_RESTORETOMAXIMIZED != 0
            } else {
                window.is_maximized().unwrap_or(false)
            };
            *AWAY.lock().unwrap() = Some(Away {
                placement: wp,
                maximized,
                tray,
            });
            if !iconic && window.is_visible().unwrap_or(false) {
                // Minimizing first hands the focus to the window behind, like a real minimize
                // (otherwise the keyboard would still type into the window nobody sees).
                unsafe { ShowWindow(hwnd, SW_MINIMIZE) };
            }
            // Restored (not maximized, not activated), then moved: Windows keeps a placement's
            // rect on a screen, a plain move it doesn't.
            let mut normal = wp;
            normal.flags = 0;
            normal.show_cmd = SW_SHOWNOACTIVATE;
            unsafe {
                SetWindowPlacement(hwnd, &normal);
                // At the bottom of the window stack: when another window minimizes or closes,
                // Windows activates the next one down, and an away window it picks pops back.
                SetWindowPos(
                    hwnd,
                    HWND_BOTTOM,
                    AWAY_X,
                    wp.normal_position.top,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }
    let _ = window.set_skip_taskbar(tray);
    MOVING.store(false, Ordering::SeqCst);
    println!("[window] away (tray: {tray})");
}

/// Brings the window back where it was (no-op when it isn't away). Returns whether it was.
pub fn bring_back(window: &Window) -> bool {
    let taken = AWAY.lock().unwrap().take();
    let Some(a) = taken else { return false };
    MOVING.store(true, Ordering::SeqCst);
    if let Some(hwnd) = hwnd(window) {
        let mut wp = a.placement;
        wp.flags = 0;
        wp.show_cmd = if a.maximized {
            SW_SHOWMAXIMIZED
        } else {
            SW_SHOWNORMAL
        };
        unsafe { SetWindowPlacement(hwnd, &wp) };
    }
    let _ = window.set_skip_taskbar(false);
    let _ = window.set_focus();
    MOVING.store(false, Ordering::SeqCst);
    println!("[window] back");
    true
}

/// When another app's window last started minimizing (ms since the epoch, PC clock).
static OTHER_MINIMIZED_AT: AtomicI64 = AtomicI64::new(0);
/// Focus this soon after another window started minimizing is Windows handing it on, not the
/// player picking this window.
const HANDOFF_WINDOW_MS: i64 = 1500;

unsafe extern "system" fn other_window_event(_: *mut c_void, _: u32, _: *mut c_void, _: i32, _: i32, _: u32, _: u32) {
    OTHER_MINIMIZED_AT.store(crate::now_ms(), Ordering::SeqCst);
}

/// Starts noticing other apps' windows being minimized (on the UI thread, whose message loop
/// delivers the events). The away window is a normal window off-screen, so when one is
/// minimized Windows may hand it the focus, and focus while away otherwise means "bring it
/// back" (its taskbar button, Alt+Tab).
pub fn watch_other_windows() {
    let hook = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_MINIMIZESTART,
            EVENT_SYSTEM_MINIMIZESTART,
            std::ptr::null_mut(),
            other_window_event,
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        )
    };
    if hook.is_null() {
        eprintln!("[window] could not watch other windows being minimized");
    }
}

/// The focus the away window just got came from another window being minimized.
pub fn focus_is_handoff() -> bool {
    crate::now_ms() - OTHER_MINIMIZED_AT.load(Ordering::SeqCst) < HANDOFF_WINDOW_MS
}

/// Staying away after a handoff: the focus goes to the desktop, as if the window were really
/// minimized (otherwise typing would go into a window nobody sees), and the window back to the
/// bottom of the stack.
pub fn refuse_focus(window: &Window) {
    let Some(hwnd) = hwnd(window) else { return };
    unsafe {
        if GetForegroundWindow() == hwnd {
            let desktop = GetShellWindow();
            if !desktop.is_null() {
                SetForegroundWindow(desktop);
            }
        }
        SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE);
    }
}

/// The maximized state to save while away (the off-screen window itself isn't maximized).
pub fn saved_maximized() -> Option<bool> {
    AWAY.lock().unwrap().as_ref().map(|a| a.maximized)
}
