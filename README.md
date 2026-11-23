# PokeIdle Multiview (Rust)

Up to **4 isolated [pokeidle.io](https://pokeidle.io) sessions in one window**, with Twitch and
Kick streams running muted in the background for the game's XP bonus.

Built with **Rust + [Tauri v2](https://v2.tauri.app)**. Instead of shipping its own Chromium,
it uses the WebView2 (Edge) runtime that comes with Windows, so the release executable is
under 10 MB. An **experimental Linux** build (WebKitGTK) comes with every release too; see
[Linux (experimental)](#linux-experimental).

## Features

- **Up to 4 slots.** Each slot has its own WebView2 data folder, so cookies, storage and logins
  stay separate and are kept between runs.
- **A header on each slot.** Every slot has its own strip on top: its controls (name, sound,
  Twitch/Kick, running bonuses, reload, close) on the left and its stream pills on the right, past a
  divider. Everything about one account sits next to its game. The toolbar at the top keeps
  the app-wide buttons and a quick switch between slots; **▴** in its top-left corner hides it
  for more room, and **▾** in the same corner (on the top-left slot) brings it back.
- **Layouts.** Grid: one slot fills the window, two sit side by side, three put two on top and
  one wide below, four make a 2×2 grid. Fullscreen: click a slot's name (in its header or the
  toolbar, or press `Ctrl+1`…`Ctrl+4`, which also works while a game page has focus) to show
  only that slot, filling the window (the others keep running hidden); click it again or press
  **GRID** to go back. **Drag the bars between slots** to resize them; the sizes are remembered. **Reset**
  in ⚙ → Layout leaves fullscreen and makes every slot the same size again.
- **Economy mode.** **ECO** on the top bar (or `Ctrl+E`) hides every slot, which keeps running
  in the background, and shows a light screen instead: a little animation, what needs you
  (streams with a ⚠, IdleHub votes that are ready, each with a button) and each account's
  running bonuses. Drawing four games is what costs the most: with 4 accounts and 32 streams
  the app went from about 50 % CPU in the grid to about 22 % in economy mode. It also turns on
  the game's own Battery Saver in each account (and turns it back off when you leave; one you
  turned on yourself stays). Picking a slot or GRID leaves it.
- **Logins.** The app checks each account's Twitch and Kick login from its own session cookies
  (only whether the login cookie is there; nothing is read out or sent). An account that isn't
  logged in on a platform earns nothing there, so no lives are opened for it; instead a **Log in**
  button shows on the slot bar, the side panel, ⚙ → Accounts and economy mode. It opens the
  login in that slot; once you're in, the login page closes and the lives start.
- **Lighter background streams.** Streams nobody is looking at ask WebView2 for its low memory
  level: with 32 streams the app used about 1 GB less, and they keep playing and earning.
- **Pages that swell get reloaded.** Each game and stream page reports its JavaScript memory;
  the reading 10 minutes after it opens is its normal size, and when it later reaches twice that
  (and at least 150 MB more), the page reloads — never the stream on screen, never the game of a
  slot you have fullscreen, at most one per account every 2 minutes.
- **Resource meter.** The top bar shows the CPU and memory of the whole app (its own process
  and every WebView2 process it started), like Task Manager adds them up.
- **Keyboard shortcuts.** ⚙ → Shortcuts lists them and lets you change each one (click it,
  press the new keys; Delete turns it off). Defaults: `Ctrl+1`…`Ctrl+4` show a slot fullscreen,
  `Ctrl+G` switches between the grid and the last fullscreen slot, `Ctrl+Shift+R` reloads every
  game page and `Ctrl+T` opens a new slot. They work in every page of the app, games and
  streams included. Keys taken by another action move to the new one (the other turns off).
- **What's new.** After an update, a small window lists what changed since the version you had
  (ui/changelog.js); ⚙ → About opens it again with every version.
- **Single focus stats panel.** A fullscreen slot gets a panel on its right with that account
  at a glance, starting with a **hunt analysis** that adds what the game doesn't show: for each
  ball, potion and revive, how many are left, how many go per hour and **how long they last**
  (warned in orange/red when low); kills, XP and gold per hour, what the supplies cost per hour
  and the net gold; the time to the trainer's and the active Pokémon's next level; catches and
  catch rate. It counts since the game loaded and starts over when the hunt changes. Then its
  Twitch and Kick bonuses (time left, linked in game or not, the Kick redeem target), daily
  chores (IdleHub vote countdown with a vote button, Kick Daily Reward), its open streams and
  their warnings, last seen channel points, and an activity log of what the app did for it
  (swaps, redeems, chests, reloads, claims). **▤** in the slot's header hides or shows it; the
  choice is remembered.
- **Daily Prey.** When an account's free Prey roll is ready (it renews at 18:00 Brasília) or
  it rolled without picking the Pokémon, its header shows **🎯 Prey** (also in economy mode, the
  stats panel's Daily chores and, if wanted, Discord). The stats panel shows each card's picked
  Pokémon, bonus and stars, and whether the account is hunting it now. The app only reads it.
- **Discord alerts.** ⚙ → Discord: paste a webhook of a channel in your server and the app
  posts there when a shiny shows up (seen, caught or fled), an account's game drops out, a
  stream needs you, an IdleHub vote is ready, and once a day a short report. Each kind can be
  turned off; nothing is sent until a webhook is set.
- **Game drop-out warning.** When an account's game is logged in somewhere else, loses its
  connection for over a minute, or has no game session, its header shows **⚠ Game** (also in
  economy mode and the stats panel) with a button to reload it.
- **Updates in the app.** A little after it starts and every 30 minutes the app looks for a newer
  release; **⬆ &lt;version&gt;** then shows on the top bar. Click it twice: it downloads the
  installer, checks its signature, installs it and opens again with the same streams. The
  portable exe can't replace itself: its ⬆ opens the release page. ⚙ → About checks on demand.
- **Loading screen.** While the games load, a small window shows each account's progress;
  the main window appears once they're ready (or after 45 s), centered on the screen with its
  last size (or maximized), with no white panels. Every page has a dark background while it
  loads.
- **Português, English or Español.** The app opens in Brazilian Portuguese; the 🇧🇷 / 🇺🇸 / 🇪🇸
  flags on the top bar (or ⚙ → Language) switch every window, the tray menu and the app's
  messages at once.
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
    the background**, and the button gets a green dot (or how many streams it has running).
  - Clicking a button with a dot brings the stream back on its full channel page, for example to
    claim channel points. **⏹** stops it.
  - **Several channels per slot.** Each slot can keep up to 8 Twitch and 8 Kick streams running
    at once. The slot's header shows a **pill per stream**: click a pill to show that channel
    in the slot (again to go back to the game) and **×** stops it. **Hold and drag a pill**
    sideways to put the streams in the order you like (saved). When they don't fit, the row
    fades on the hidden side and shows an arrow that scrolls it (the mouse wheel works too).
  - **Add any channel.** The header's **+** opens a box: type a channel name (pick Twitch or
    Kick next to it) or paste a Twitch/Kick link, and press Enter. It opens like a channel
    link clicked in the game.
  - **Light players in the background.** A hidden Twitch stream runs on Twitch's own embed
    player at its lowest quality instead of the full site (no chat, sidebars or site
    around it). It still earns channel points: in a side-by-side test the embed and the
    full page both earned +20 points in 12 minutes. With 4 Twitch + 4 Kick streams this
    took the app from ~8.3 GB of RAM and ~4.6 CPU cores to ~5.4 GB and ~0.7 cores.
  - **Lighter Kick pages in the background.** Kick's embed player shows live channels as
    offline, so a hidden Kick stream keeps its channel page, trimmed instead: chat, sidebars,
    navigation and animations are hidden, the player drops to its lowest quality, and the page
    is parked at 480×360. The video itself is never hidden, so it keeps playing. Showing the
    stream brings back the full page and "Auto" quality. Can be turned off in ⚙ → Streams.
  - **Official channel links clicked inside the game** go to that slot's Twitch/Kick page
    instead of a separate window. The first time, the page opens visible so you can check you
    are logged in. After that, each new channel starts its own stream in the background (a
    page of that platform that isn't on a channel, like the login page, is reused first).
  - Running streams and the channel they are on are saved, and come back muted when the app
    starts.
- **Kick priority list (⚙ → Kick).** Kick pays channel points for 2 lives at a time per account
  (measured: with 4 Kick lives open, each account earned on exactly 2). List your Kick
  channels in order and every account keeps the **2 highest ones that are live** open, closing
  its other Kick streams. Kick's public channel API is checked every 2 minutes: when a channel
  goes offline the next live one takes its place, and a higher one that goes live takes over.
  The stream you are looking at is never closed. An empty list leaves Kick streams to you.
- **Twitch priority list (⚙ → Twitch).** Same idea for Twitch, which has no 2-stream limit:
  choose how many lives each account keeps (1–8, default 4). Live status comes from the
  channel's public page.
- **Keep the lists updated from the game.** The game announces its official Kick and Twitch
  partner channels by itself (in its own state messages, which the app only listens to). Tick
  *Keep updated from the game* and new partners are added at the end of the list.
- **Automatic bonuses (⚙ → Bonus per account).** Pick +15% XP, Capture Boost or Secret Lure per
  account, and whenever its channel points cover the cheapest reward of that kind the app
  redeems it (the game adds the hours up, and every size costs the same per hour). On Kick this
  happens in the background page; on Twitch, whose light player has no points, each background
  stream is **visited** every ~15 minutes: its full page loads for ~30 s, the **bonus chest**
  (+50) is claimed and the bonus redeemed, then it goes back to the light player. Each account
  shows whether the game has its Kick and Twitch linked; without that, the points are given
  back. A channel new to the account may open its points on a welcome screen first; the visit
  steps past it, and waits for the confirm button, which Twitch keeps disabled for a moment.
- **Follow tips.** A stream whose channel the account doesn't follow gets a ♡ on its pill, with a
  tip on hover (on Twitch a follow gives +300 points once). Read only: the app never follows by
  itself. Every Twitch and Kick channel page watches it, so the tip goes away within seconds.
  The ⚙ priority lists show it per channel too: ♡ when some account doesn't follow it, ? when
  some account hasn't opened it yet (the app only learns it from an open channel page), with
  the accounts on hover. It is saved, so it survives restarts.
- **Redeem goals.** Instead of one fixed bonus, an account can switch to **Goals**: an ordered
  list such as XP ≥ 10h, Capture ≥ 5h, Lure ≥ 3h. Its points buy the first goal not met yet,
  each platform judged by its own clock (Kick points buy Kick bonuses); once every goal is met
  it saves the points or keeps buying the last one. After a redeem it waits for the game's new
  time before buying again, and on Kick only the channel with the most points redeems per pass,
  so goals aren't overshot. Each goal shows its time left on Twitch and Kick (green when met,
  outlined while being bought); the stats panel marks the goals on its bars. "Copy to all
  accounts" applies one account's setup to the others.
- **Running bonuses at a glance.** Each slot's header shows the account's PokeIdle bonuses that
  are running and the time left (e.g. `XP 6h12`, `CAP 1h40`, `LURE 3h00`; Kick's and Twitch's times on
  hover), read from the game's own state. The same appears in ⚙ → Bonus per account.
- **Streams keep playing when nobody watches.** Minimizing or sending the app to the tray moves
  the window off-screen instead (the taskbar button or the tray icon brings it back where it
  was), and behind other windows or with the screen off the pages still count as visible. A
  really minimized window stops rendering, and a Twitch player that restarts then never plays
  again: that spent whole nights paused and reloading.
- **Stream health.** Every 30 seconds each background stream checks that its video is moving.
  A player that paused itself gets play pressed (no reload, no lost watch time); only one that
  stays paused or frozen counts as stalled. A stalled stream is reloaded once quietly; if it
  still doesn't play, its pill gets a ⚠ and it
  is reloaded again every 10 minutes. Logged-out accounts and offline channels get a ⚠ too (a
  page that is still loading isn't flagged). The settings window shows how many streams need
  attention. A live that ends is caught right away (the player's "offline" screen over a frozen
  video), not taken for a stall. A channel of a priority list that its page finds offline is
  swapped within seconds: the first report runs the priority check and asks the page again 10 s
  later, and a second "offline" is believed even if Kick/Twitch still say "live" (their status
  can lag for minutes), so that account opens its next live channel.
- **IdleHub vote reminder (⚙ → IdleHub vote).** Tick the accounts to be reminded. Each one's
  header shows when its vote is ready (asked from the game, like its own "Vote no IdleHub.io"
  window does) and its votes/diamonds; clicking opens the vote page in a window with that slot's
  session, so each account keeps its own IdleHub sign-in and votes with its own nick. You sign in
  and confirm there; the app never votes by itself.
- **Kick Daily Reward (⚙ → Kick, off by default).** Once a day per account, the app opens Kick's
  gift dialog in one of the account's background Kick pages, moves its carousel to the Daily
  Reward card and claims it (up to 3 tries a day; Kick's day resets at 00:00 UTC).
- **"/hunt for PokeIdle" extension.** If
  [the extension](https://chromewebstore.google.com/detail/hunt-for-pokeidle/mhnplloljgchpfpcpbhmcokmnefodjnh)
  is installed in a Chromium browser on the PC (Chrome, Edge, Brave, Vivaldi, Opera, Comet,
  including Beta/Nightly/Dev channels), the newest copy is loaded into every slot at startup.
- **Desktop layout of the game.** pokeidle.io switches to its phone layout when the page is
  720 px wide or less, which a 2×2 grid often is. Slots default the game's own setting to the
  desktop layout, and the game then scales itself down to fit. If you picked a layout in the
  game, your choice is kept.
- **Per-slot controls.** Reload and close. Closing asks for a second click and keeps the slot's
  login. (The game scales itself to the slot; `Ctrl` + mouse wheel still zooms a page for the
  moment.)
- **Tray.** Closing (×) hides the app to the tray (the Poké Ball near the clock), and streams
  keep running; its tooltip shows how many. Click it to bring the window back, or right-click →
  **Quit** to exit. Minimizing works as usual (taskbar); both can be changed in ⚙.
- **One copy at a time.** Starting the app again brings the open window to the front instead
  of running a second copy, which would play every background stream twice.
- **Remembers** the open slots, focused slot, background streams, and the window's size,
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

### Linux (experimental)

| File | What it is |
| --- | --- |
| `PokeIdleMultiview_<version>_amd64.deb` | Debian, Ubuntu, Mint…: `sudo apt install ./PokeIdleMultiview_<version>_amd64.deb` |
| `PokeIdleMultiview_<version>_amd64.AppImage` | Any distribution: make it executable and run it; it updates itself |

On Linux the pages run in WebKitGTK instead of Chromium, so a few things differ:

- **No /hunt.** It is a Chromium extension; WebKit can't load it.
- **Video needs GStreamer.** Twitch and Kick play through the GStreamer plugins
  (`gstreamer1.0-plugins-good`, `gstreamer1.0-plugins-bad`, `gstreamer1.0-libav`); the .deb
  installs them, with the AppImage install them yourself.
- **Shortcuts** work in the app's own pages and, inside a game or stream page, are read from
  the page itself (the page may still act on the keys too).
- **Minimizing** really minimizes (Windows moves the window off-screen instead, so the streams
  keep playing); if streams stop while minimized, keep the window open behind others.
- Keeping the PC awake uses `systemd-inhibit`; the resource meter reads `/proc`.

Tested on Ubuntu 24.04: games, the grid, headers, the Log in buttons, Twitch and Kick pages, and
a Twitch stream that kept playing in the background. Not tested yet: Wayland desktops, long
sessions, earning channel points while logged in.

## Build from source

You need:

- [Rust](https://rustup.rs) with the MSVC toolchain (`stable-x86_64-pc-windows-msvc`).
- Visual Studio Build Tools with the **Desktop development with C++** workload.

On Linux (Debian/Ubuntu): Rust, plus
`sudo apt install libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`
and the GStreamer plugins above; `npx @tauri-apps/cli@2 build --bundles deb,appimage` makes the
packages.

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
GitHub release, with a `latest.json` that the in-app updater reads. The installers are signed
for the updater with the private key in the `TAURI_SIGNING_PRIVATE_KEY` repository secret (no
password); its public half is `plugins.updater.pubkey` in `tauri.conf.json`. Keep the private
key safe: without it, installed copies can't be updated anymore. The [CI workflow](.github/workflows/ci.yml) runs the tests on every push and
pull request.

### Development helpers

These environment variables only apply to **debug** builds, except
`MULTIVIEW_ALLOW_WEBAUTHN`:

| Variable | Effect |
| --- | --- |
| `MULTIVIEW_DEVTOOLS=1` | Slot N (and its Twitch/Kick pages) answers the Chrome DevTools Protocol on `127.0.0.1:930N`. Open `edge://inspect` to debug it. The app's own pages (toolbar, headers, stats panel, settings) answer on `127.0.0.1:9300`. |
| `MULTIVIEW_EXTENSION_DIR=<path>` | Loads this unpacked extension instead of /hunt, for example `tests/fixtures/test-extension`. |
| `MULTIVIEW_ALLOW_WEBAUTHN=1` | Keeps WebAuthn (passkeys) on in slots. By default it is off; see [Privacy and security](#privacy-and-security). |

## Where data is kept

| What | Where |
| --- | --- |
| Settings (slots, focus, streams, lists, bonuses) | `%LOCALAPPDATA%\io.pokeidle-multiview.app\config.json` |
| Each slot's browser data (cookies, logins, storage) | `%LOCALAPPDATA%\io.pokeidle-multiview.app\slots\slot-N\` |
| Copy of the /hunt extension | `%LOCALAPPDATA%\io.pokeidle-multiview.app\extensions\` |
| Window size and position | `%APPDATA%\io.pokeidle-multiview.app\.window-state.json` |

On Linux the same files are under `~/.local/share/io.pokeidle-multiview.app/`.

To reset a slot's login, close the app and delete its `slot-N` folder.

## How it works

```
src/main.rs      window, slot and stream webviews, navigation guard, toolbar commands
src/layout.rs    grid math (pure, unit-tested)
src/config.rs    saved settings: load, normalize, atomic save
src/tray.rs      tray icon, close/minimize to tray, quit
src/native.rs    WebView2 events: permissions, shortcuts inside pages, crash recovery
src/*_other.rs   Linux versions of native.rs, away.rs and power.rs (WebKitGTK, systemd-inhibit)
src/shortcuts.rs keyboard shortcuts: defaults, parsing, actions, settings commands
src/whatsnew.rs  patch notes window after an update
src/resources.rs CPU/memory meter of the app and its WebView2 processes
src/power.rs     keeps Windows awake while streams run
src/hunt.rs      finds the installed /hunt extension and copies it for WebView2
src/channels.rs  recognizes Twitch/Kick channel links
src/game.rs      game connection warnings and the Pokémon sprites for the stats panel
src/discord.rs   Discord alerts (webhook, what to send, daily report)
src/prey.rs      the game's daily Prey: what's left to do today, for the reminders
src/heavy.rs     reloads pages that swell over a long session
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
  It has no telemetry. Besides the pages you open, it only contacts GitHub (to look for updates)
  and, if you set one, your own Discord webhook (the alerts you chose). The webhook address
  stays in your config file and is never shown back or logged.
- Known limitation: popup windows opened from inside a page are not covered by the navigation
  guard.
- More, and how to check a download (SHA256SUMS.txt in every release), in
  [SECURITY.md](SECURITY.md).

## Disclaimer

Unofficial, community-made tool. **Not affiliated with or endorsed by pokeidle.io, Twitch, Kick,
Google or Discord.** Keeping streams open in the background and using several accounts may be
regulated by the terms of service of the game and the streaming platforms. Check the current
rules and use it at your own risk.

## License

[MIT](LICENSE).
