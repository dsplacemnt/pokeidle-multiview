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
- **Layouts.** Grid: one slot fills the window, two sit side by side, three put two on top and
  one wide below, four make a 2×2 grid. Focus: click a slot's name (or press `Ctrl+1`…`Ctrl+4`,
  which also works while a game page has focus) to show it large with the others in a column
  on the right; click it again or press **Grid** to go back. **Drag the bars between slots** to
  resize them; the sizes are remembered.
- **Per-slot names and sound.** Double-click a slot's name to rename it (Enter saves, Esc
  cancels, empty goes back to "Slot N"). 🔊/🔇 mutes that slot's game.
- **Settings (⚙).** Close or minimize to the tray, keep the PC awake while streams run (on by
  default), and reload all game pages every 15 min to 24 h (off by default).
- **Recovers by itself.** A game or stream page that crashes or stops responding reloads
  itself (up to 3 times in 5 minutes).
- **Asks for nothing.** Pages may only use notifications and autoplay; requests for the camera,
  microphone, location, clipboard and so on are denied without a prompt.
- **Twitch and Kick per slot.**
  - The **Twitch** or **Kick** button on a slot opens that site in place of the game, using the
    slot's own session. Log in there; the login belongs to that slot only.
  - Clicking the button again takes you back to the game. The page **keeps running, muted, in
    the background**, and the button gets a green dot.
  - Clicking a button with a dot brings the stream back on its full channel page, for example to
    claim channel points. **⏹** stops it.
  - **Light players in the background.** A hidden Twitch stream runs on Twitch's own embed
    player at its lowest quality instead of the full site (no chat, sidebars or site
    around it). It still earns channel points: in a side-by-side test the embed and the
    full page both earned +20 points in 12 minutes. With 4 Twitch + 4 Kick streams this
    took the app from ~8.3 GB of RAM and ~4.6 CPU cores to ~5.4 GB and ~0.7 cores. Kick
    keeps its full page, as its embed player shows live channels as offline.
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
- **Tray.** Closing (×) hides the app to the tray (the Poké Ball near the clock), and streams
  keep running; its tooltip shows how many. Click it to bring the window back, or right-click →
  **Quit** to exit. Minimizing works as usual (taskbar); both can be changed in ⚙.
- **One copy at a time.** Starting the app again brings the open window to the front instead
  of running a second copy, which would play every background stream twice.
- **Remembers** the open slots, zoom, focused slot, background streams, and the window's size,
  position and maximized state.

## Download

Get the latest version from the [Releases](../../releases/latest) page. Pick one:

| File | What it is |
| --- | --- |
| `PokeIdleMultiview_<version>_x64.msi` | Windows Installer package |
| `PokeIdleMultiview_<version>_x64-setup.exe` | Installer; no admin rights needed |
| `PokeIdleMultiview_<version>_x64_portable.exe` | Portable: no install, just run it |

Requirements: Windows 10 or 11 (x64) and the
[WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/). WebView2 is
already on Windows 11 and on most up-to-date Windows 10 PCs, and both installers add it if it is
missing.

The files are not code-signed, so Windows SmartScreen may show "Windows protected your PC" the
first time. Choose **More info → Run anyway**.

Settings and logins live in your user folder (see [Where data is kept](#where-data-is-kept)), so
updating or reinstalling keeps them.

## Build from source

You need:

- [Rust](https://rustup.rs) with the MSVC toolchain (`stable-x86_64-pc-windows-msvc`).
- Visual Studio Build Tools with the **Desktop development with C++** workload.

```bash
cargo run                # debug build, opens the app
cargo test               # unit tests
cargo build --release    # optimized build: target/release/pokeidle-multiview.exe
```

The release build is a single executable with no console window. To also build the MSI and
setup installers (into `target/release/bundle/`), use the Tauri CLI:

```bash
npx @tauri-apps/cli@2 build
```

### Making a release

1. Set the same new version in `Cargo.toml` and `tauri.conf.json`, and commit it.
2. Tag the commit and push the tag:

   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

The [Release workflow](.github/workflows/release.yml) checks that the tag matches both
versions, runs the tests, builds the portable exe and both installers, and publishes them as a
GitHub release. The [CI workflow](.github/workflows/ci.yml) runs the tests on every push and
pull request.

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
src/tray.rs      tray icon, close/minimize to tray, quit
src/native.rs    WebView2 events: permissions, Ctrl+1…4 inside pages, crash recovery
src/power.rs     keeps Windows awake while streams run
src/hunt.rs      finds the installed /hunt extension and copies it for WebView2
src/channels.rs  recognizes Twitch/Kick channel links
ui/              toolbar, splitter strips and settings window (the only pages that can call Rust)
scripts/         scripts injected into the game pages
tests/fixtures/  a tiny extension used to test extension loading
icons/           app icon; icon.svg is the source (regenerate with `npx @tauri-apps/cli@2 icon icons/icon.png`)
```

Some details that are easy to miss:

- **Slots are separate WebView2 environments**, one data folder each. A slot's Twitch/Kick pages
  live in that same environment, so they share its logins.
- **Background streams are parked outside the window, not hidden.** A page that loads while
  hidden doesn't start its video player, so streams stay "visible" but off-screen. They are
  muted with WebView2's own switch, which the page cannot undo.
- **The navigation guard runs on every navigation, server redirects included.** Slots may only
  reach pokeidle.io, twitch.tv, kick.com, and the Google and Discord sign-in pages. Popups on
  those hosts open in the slot's session. Any other web link (a vote page, a streamer's
  socials…) opens in your default browser instead, never inside the app; the same link isn't
  reopened within 10 s, and at most 3 links open per 30 s, so a page can't flood your browser.

## Privacy and security

- Only the app's own local pages (toolbar, splitters, settings) can call the Rust side. Game,
  Twitch and Kick pages get no access to it.
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
- [ ] A lighter background page for Kick (Twitch already uses its embed player).
- [ ] Stream health: a ⚠ on the slot when a stream stalls (crashed pages already reload).
- [ ] Light mode: Kick via its HLS feed and Twitch via authenticated chat, without video players.

## Disclaimer

Unofficial, community-made tool. **Not affiliated with or endorsed by pokeidle.io, Twitch, Kick,
Google or Discord.** Keeping streams open in the background and using several accounts may be
regulated by the terms of service of the game and the streaming platforms. Check the current
rules and use it at your own risk.

## License

[MIT](LICENSE).
