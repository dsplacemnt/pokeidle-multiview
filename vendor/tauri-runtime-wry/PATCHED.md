# tauri-runtime-wry 2.12.5, patched for PokeIdle Multiview

A copy of the crate from crates.io (MIT / Apache-2.0, see the LICENSE files), used through
`[patch.crates-io]` in the app's Cargo.toml. One change, in `src/lib.rs` (search for
"PokeIdle Multiview"):

When a webview is given a ready-made WebView2 environment (`WebviewBuilder::environment`), the
last folder of its `data_directory` becomes its **WebView2 profile name** (wry's
`with_profile_name`; Tauri 2.12 doesn't expose it). That lets the app run the four accounts in
one shared browser (one environment, one profile each) for the "One browser for all accounts"
option. Without an environment nothing changes.

To update Tauri: copy the new tauri-runtime-wry over this folder and apply the same change.
