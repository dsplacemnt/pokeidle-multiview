//! Finds the "/hunt for PokeIdle" extension already installed by a Chromium browser on this PC.
//! The app does not ship the extension: it loads the newest installed copy at startup, so
//! browser updates reach the app on the next launch.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

pub const HUNT_EXTENSION_ID: &str = "mhnplloljgchpfpcpbhmcokmnefodjnh";

fn browser_roots(local_app_data: &Path) -> Vec<PathBuf> {
    [
        "Google/Chrome/User Data",
        "Microsoft/Edge/User Data",
        "BraveSoftware/Brave-Browser/User Data",
        "BraveSoftware/Brave-Browser-Beta/User Data",
        "BraveSoftware/Brave-Browser-Nightly/User Data",
        "Microsoft/Edge Beta/User Data",
        "Microsoft/Edge Dev/User Data",
        "Microsoft/Edge SxS/User Data",
        "Vivaldi/User Data",
        "Perplexity/Comet/User Data",
        "Chromium/User Data",
        "Google/Chrome SxS/User Data",
        "Google/Chrome Beta/User Data",
        "Google/Chrome Dev/User Data",
        // Opera keeps the profile straight in the browser folder, without "User Data".
        "Opera Software/Opera Stable",
        "Opera Software/Opera GX Stable",
    ]
    .iter()
    .map(|rel| local_app_data.join(rel))
    .collect()
}

fn candidate_profile_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![root.to_path_buf(), root.join("Default")];
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_profile = name
                .strip_prefix("Profile ")
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
            if is_profile {
                dirs.push(entry.path());
            }
        }
    }
    dirs
}

/// Installed versions live in "1.2.3_0" folders (the suffix is Chromium's reinstall counter).
/// Sorts newest first, comparing segments numerically and breaking ties by the counter.
pub fn sort_version_dirs(names: &[String]) -> Vec<String> {
    let mut parsed: Vec<(Vec<u64>, u64, &String)> = names
        .iter()
        .filter_map(|name| {
            let mut parts = name.splitn(2, '_');
            let version = parts.next()?;
            let counter = parts.next().map_or(Some(0), |c| c.parse().ok())?;
            let segments = version
                .split('.')
                .map(|s| s.parse::<u64>().ok())
                .collect::<Option<Vec<_>>>()?;
            Some((segments, counter, name))
        })
        .collect();
    parsed.sort_by(|a, b| {
        let len = a.0.len().max(b.0.len());
        for i in 0..len {
            let (x, y) = (a.0.get(i).unwrap_or(&0), b.0.get(i).unwrap_or(&0));
            match y.cmp(x) {
                Ordering::Equal => continue,
                other => return other,
            }
        }
        b.1.cmp(&a.1)
    });
    parsed.into_iter().map(|(_, _, n)| n.clone()).collect()
}

/// Returns the folder of the newest valid installed copy (one that has a manifest.json).
pub fn resolve_hunt_extension_dir() -> Option<PathBuf> {
    let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)?;
    for root in browser_roots(&local_app_data) {
        for profile in candidate_profile_dirs(&root) {
            let id_dir = profile.join("Extensions").join(HUNT_EXTENSION_ID);
            let Ok(entries) = std::fs::read_dir(&id_dir) else { continue };
            let names: Vec<String> = entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            // Newest to oldest: skip folders caught mid-update (no manifest yet).
            for version in sort_version_dirs(&names) {
                let dir = id_dir.join(&version);
                if dir.join("manifest.json").is_file() {
                    return Some(dir);
                }
            }
        }
    }
    None
}

/// Name of the folder the extension is copied into, inside the app's extensions folder.
pub const STAGED_NAME: &str = "hunt-for-pokeidle";

/// WebView2 (through wry) loads every sub-folder of the folder it is given as one unpacked
/// extension, and fails the whole webview if any entry is not an extension. So the chosen
/// version is copied into `<extensions_root>/hunt-for-pokeidle` and `extensions_root` is what
/// the slots get. Copying (instead of pointing at the browser's folder) also means a browser
/// update mid-session can't change files under a running slot.
pub fn stage_extension(source: &Path, extensions_root: &Path) -> std::io::Result<PathBuf> {
    if extensions_root.exists() {
        std::fs::remove_dir_all(extensions_root)?;
    }
    copy_dir(source, &extensions_root.join(STAGED_NAME))?;
    Ok(extensions_root.to_path_buf())
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn stages_one_extension_folder() {
        let tmp = std::env::temp_dir().join(format!("multiview-stage-test-{}", std::process::id()));
        let src = tmp.join("src");
        std::fs::create_dir_all(src.join("js")).unwrap();
        std::fs::write(src.join("manifest.json"), "{}").unwrap();
        std::fs::write(src.join("js").join("a.js"), "1").unwrap();
        let root = tmp.join("extensions");
        std::fs::create_dir_all(root.join("stale")).unwrap();

        let staged = stage_extension(&src, &root).unwrap();

        let names: Vec<_> = std::fs::read_dir(&staged)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, vec![STAGED_NAME.to_string()]);
        assert!(staged.join(STAGED_NAME).join("js").join("a.js").is_file());
        std::fs::remove_dir_all(tmp).unwrap();
    }

    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn sorts_numerically_and_by_counter() {
        let sorted = sort_version_dirs(&s(&["0.9.0_0", "0.10.0_0", "0.10.0_1", "junk", "1.0_x"]));
        assert_eq!(sorted, s(&["0.10.0_1", "0.10.0_0", "0.9.0_0"]));
    }

    #[test]
    fn missing_segments_count_as_zero() {
        assert_eq!(sort_version_dirs(&s(&["1.0", "1.0.1"])), s(&["1.0.1", "1.0"]));
    }
}
