//! "One browser for all accounts" (⚙ → General → Memory, off by default).
//!
//! Normally each slot has its own WebView2 user data folder, which means its own browser: its
//! own browser process, GPU process, network service and so on (about 400–500 MB each, with
//! the streams on top). With this option the app creates a single WebView2 environment (one
//! browser, data folder `slots/shared`) and gives each slot its own **profile** in it
//! (`slot-1`, `slot-2`…): cookies, storage and logins stay separate per account, and the
//! browser's own processes are shared. The profile name reaches wry through the patched
//! tauri-runtime-wry in vendor/ (see PATCHED.md).
//!
//! The profiles start empty, so every account has to log in again (game, Twitch, Kick); the
//! per-slot folders are left alone, so turning the option off brings the old logins back.
//! The environment is made once, at startup: switching needs a restart.
//!
//! Windows only (WebKitGTK on Linux has no such thing here).

#[cfg(windows)]
mod imp {
    use std::path::Path;
    use std::sync::{mpsc, Mutex};

    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use webview2_com::{CoreWebView2EnvironmentOptions, CreateCoreWebView2EnvironmentCompletedHandler};
    use windows_core::{HSTRING, PCWSTR};

    /// The environment is a COM object of the main thread; it's only handed to Tauri, which
    /// uses it on the main thread to create the webviews.
    struct Env(ICoreWebView2Environment);
    unsafe impl Send for Env {}

    static ENV: Mutex<Option<Env>> = Mutex::new(None);

    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoInitializeEx(reserved: *const std::ffi::c_void, coinit: u32) -> i32;
    }
    /// COINIT_APARTMENTTHREADED: WebView2 needs a single-threaded apartment.
    const COINIT_APARTMENTTHREADED: u32 = 0x2;

    /// Creates the shared browser. Call it on the main thread before any slot opens (it pumps
    /// window messages while WebView2 starts).
    pub fn start(data_dir: &Path, args: &str, extensions: bool) -> Result<(), String> {
        std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::channel();
        let options = CoreWebView2EnvironmentOptions::default();
        unsafe {
            // This runs at startup, before the first webview has set COM up on this thread (it's
            // harmless when it already is: same apartment, it just counts once more).
            let _ = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED);
            options.set_additional_browser_arguments(args.to_string());
            options.set_are_browser_extensions_enabled(extensions);
            CreateCoreWebView2EnvironmentWithOptions(
                PCWSTR::null(),
                &HSTRING::from(data_dir.to_string_lossy().as_ref()),
                &ICoreWebView2EnvironmentOptions::from(options),
                &CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(move |code, env| {
                    let _ = tx.send(code.map(|_| env));
                    Ok(())
                })),
            )
            .map_err(|e| e.to_string())?;
        }
        let env = webview2_com::wait_with_pump(rx)
            .map_err(|e| format!("{e:?}"))?
            .map_err(|e| e.to_string())?
            .ok_or("WebView2 gave no environment")?;
        *ENV.lock().unwrap() = Some(Env(env));
        Ok(())
    }

    /// The shared browser, if the option is on and it started.
    pub fn environment() -> Option<ICoreWebView2Environment> {
        ENV.lock().unwrap().as_ref().map(|e| e.0.clone())
    }
}

#[cfg(windows)]
pub use imp::{environment, start};

/// Whether this platform has the option.
pub const AVAILABLE: bool = cfg!(windows);

/// Where the shared browser keeps its data, and the folder that names a slot's profile (the
/// patched runtime reads the last folder as the profile name; nothing is written there).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn data_dir(data_root: &std::path::Path) -> std::path::PathBuf {
    data_root.join("shared")
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn profile_dir(data_root: &std::path::Path, slot_label: &str) -> std::path::PathBuf {
    data_dir(data_root).join("profiles").join(slot_label)
}
