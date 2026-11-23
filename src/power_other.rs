//! Outside Windows: keeps the PC from going to sleep while background streams run (when
//! enabled) with `systemd-inhibit`, which most Linux desktops have. Without it, nothing happens.

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use tauri::AppHandle;

static INHIBIT: Mutex<Option<Child>> = Mutex::new(None);

pub fn set_keep_awake(_app: &AppHandle, wanted: bool) {
    let mut held = INHIBIT.lock().unwrap();
    if held.is_some() == wanted {
        return;
    }
    if wanted {
        let child = Command::new("systemd-inhibit")
            .args(["--what=sleep:idle", "--who=PokeIdle Multiview", "--why=Streams are running", "sleep", "infinity"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match child {
            Ok(child) => {
                println!("[power] keep awake: true (systemd-inhibit)");
                *held = Some(child);
            }
            Err(e) => eprintln!("[power] could not keep the PC awake (systemd-inhibit: {e})"),
        }
    } else if let Some(mut child) = held.take() {
        let _ = child.kill();
        let _ = child.wait();
        println!("[power] keep awake: false");
    }
}
