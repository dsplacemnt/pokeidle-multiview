# PokeIdle Multiview (Rust)

Up to **4 isolated [pokeidle.io](https://pokeidle.io) sessions in one window**, with Twitch and
Kick streams running muted in the background for the game's XP bonus.

Built with **Rust + [Tauri v2](https://v2.tauri.app)**. Instead of shipping its own Chromium,
it uses the WebView2 (Edge) runtime that comes with Windows, so the release executable is
under 10 MB.

> **Status: early (0.1).** The core works: slots, logins, layouts, the /hunt extension and
> background streams. More is planned; see the [roadmap](#roadmap).

## Features

- **Up to 4 slots.** Each slot has its own WebView2 data folder, so cookies, storage and logins
  stay separate and are kept between runs.
- **Layouts.** One slot fills the window, two sit side by side, and three or four use a 2×2 grid.
  Click a slot's name (or press `Ctrl+1`…`Ctrl+4`) to focus it, and click it again or press
  **Grid** to see all slots.
- **Twitch and Kick per slot.**
  - The **Twitch** or **Kick** button on a slot opens that site in place of the game, using the
    slot's own session. Log in there; the login belongs to that slot only.
  - Clicking the button again takes you back to the game. The page **keeps running, muted, in
    the background**, and the button gets a green dot.
  - Clicking a button with a dot brings the stream back exactly where it was, for example to
    claim channel points. **⏹** stops it.
  - **Official channel links clicked inside the game** go to that slot's Twitch/Kick page
    instead of a separate window. The first time, the page opens visible so you can check you
    are logged in. After that, a new channel replaces the one running for that platform and
    stays in the background.
  - Running streams and the channel they are on are saved, and come back muted when the app
    starts.
- **"/hunt for PokeIdle" extension.** If
  [the extension](https://chromewebstore.google.com/detail/hunt-for-pokeidle/mhnplloljgchpfpcpbhmcokmnefodjnh)
  is installed in a Chromium browser on the PC (Chrome, Edge, Brave, Vivaldi, Opera, Comet,
  including Beta/Nightly/Dev channels), the newest copy is loaded into every slot at startup.
- **Desktop layout of the game.** pokeidle.io switches to its phone layout when the page is
  720 px wide or less, which a 2×2 grid often is. Slots default the game's own setting to the
  desktop layout, and the game then scales itself down to fit. If you picked a layout in the
  game, your choice is kept.
- **Per-slot controls.** Zoom from 50% to 200%, reload, and close. Closing asks for a second click
  and keeps the slot's login.
- **Remembers** the open slots, zoom, focused slot, background streams, and the window's size,
  position and maximized state.

## Install and run

Requirements:

- Windows 10 or 11 (x64).
- [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/).
  It is already installed on Windows 11 and on most up-to-date Windows 10 PCs.

There are no prebuilt releases yet; build it from source (below).

## Build from source

You need:

- [Rust](https://rustup.rs) with the MSVC toolchain (`stable-x86_64-pc-windows-msvc`).
- Visual Studio Build Tools with the **Desktop development with C++** workload.

```bash
cargo run                # debug build, opens the app
cargo test               # unit tests
cargo build --release    # optimized build: target/release/pokeidle-multiview.exe
```

The release build is a single executable with no console window.

### Development helpers

These environment variables only apply to **debug** builds, except
`MULTIVIEW_ALLOW_WEBAUTHN`:

| Variable | Effect |
| --- | --- |
| `MULTIVIEW_DEVTOOLS=1` | Slot N (and its Twitch/Kick pages) answers the Chrome DevTools Protocol on `127.0.0.1:930N`. Open `edge://inspect` to debug it. |
| `MULTIVIEW_EXTENSION_DIR=<path>` | Loads this unpacked extension instead of /hunt, for example `tests/fixtures/test-extension`. |
| `MULTIVIEW_ALLOW_WEBAUTHN=1` | Keeps WebAuthn (passkeys) on in slots. By default it is off; see [Privacy and security](#privacy-and-security). |

## Where data is kept

| What | Where |
| --- | --- |
| Settings (slots, zoom, focus, background streams) | `%LOCALAPPDATA%\io.pokeidle-multiview.app\config.json` |
| Each slot's browser data (cookies, logins, storage) | `%LOCALAPPDATA%\io.pokeidle-multiview.app\slots\slot-N\` |
| Copy of the /hunt extension | `%LOCALAPPDATA%\io.pokeidle-multiview.app\extensions\` |
| Window size and position | `%APPDATA%\io.pokeidle-multiview.app\.window-state.json` |

To reset a slot's login, close the app and delete its `slot-N` folder.

## How it works

```
src/main.rs      window, slot and stream webviews, navigation guard, toolbar commands
src/layout.rs    grid math (pure, unit-tested)
src/config.rs    saved settings: load, normalize, atomic save
src/hunt.rs      finds the installed /hunt extension and copies it for WebView2
src/channels.rs  recognizes Twitch/Kick channel links
ui/index.html    the toolbar; the only page allowed to call into Rust
scripts/         scripts injected into the game pages
tests/fixtures/  a tiny extension used to test extension loading
```

Some details that are easy to miss:

- **Slots are separate WebView2 environments**, one data folder each. A slot's Twitch/Kick pages
  live in that same environment, so they share its logins.
- **Background streams are parked outside the window, not hidden.** A page that loads while
  hidden doesn't start its video player, so streams stay "visible" but off-screen. They are
  muted with WebView2's own switch, which the page cannot undo.
- **The navigation guard runs on every navigation, server redirects included.** Slots may only
  reach pokeidle.io, twitch.tv, kick.com, and the Google and Discord sign-in pages. Popups on
  those hosts open in the slot's session; everything else is blocked.

## Privacy and security

- Only the toolbar (a local page) can call the Rust side. Game, Twitch and Kick pages get no
  access to it.
- WebAuthn (passkeys) is turned off inside the app's pages, so Twitch, Kick and Google sign-in
  use password and code instead of the Windows passkey prompt. This only affects pages inside
  the app, not your browsers.
- The app reads Chromium browser profile folders **only** to find an installed /hunt extension.
  It has no telemetry and sends nothing anywhere; the only network traffic comes from the pages
  you open.
- Known limitation: popup windows opened from inside a page are not covered by the navigation
  guard.

## Roadmap

Planned, roughly in priority order:

- [ ] Several channels per platform in each slot (up to 8), shown as pills you can bring back
      or close.
- [ ] Lighter background players: Twitch's popout player and Kick's embed player instead of the
      full site, plus lowest-quality selection.
- [ ] Stream health: a ⚠ on the slot when a stream crashes or stalls, with automatic restarts.
- [ ] Recovering a game page that crashed or stopped responding.
- [ ] `Ctrl+1`…`Ctrl+4` while a game page has focus (today they only work from the toolbar).
- [ ] Allow only one copy of the app at a time.
- [ ] Permission allowlist (notifications, fullscreen, pointer lock) instead of WebView2's prompts.
- [ ] Close to tray, and keep Windows awake while streams run.
- [ ] Per-slot mute, renaming slots, and automatic reload of all game pages on a timer.
- [ ] Focus layout with the other slots in a side column, and draggable splitters.
- [ ] Settings menu (⚙).
- [ ] Light mode: Kick via its HLS feed and Twitch via authenticated chat, without video players.
- [ ] Installer and release builds from CI.

## Disclaimer

Unofficial, community-made tool. **Not affiliated with or endorsed by pokeidle.io, Twitch, Kick,
Google or Discord.** Keeping streams open in the background and using several accounts may be
regulated by the terms of service of the game and the streaming platforms. Check the current
rules and use it at your own risk.

## License

[MIT](LICENSE).
