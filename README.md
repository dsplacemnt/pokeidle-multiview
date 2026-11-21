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
- **Updates in the app.** A little after it starts and every 6 hours the app looks for a newer
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
  back.
- **Redeem goals.** Instead of one fixed bonus, an account can switch to **Goals**: an ordered
  list such as XP ≥ 10h, Capture ≥ 5h, Lure ≥ 3h. Its points buy the first goal not met yet,
  each platform judged by its own clock (Kick points buy Kick bonuses); once every goal is met
  it saves the points or keeps buying the last one. After a redeem it waits for the game's new
  time before buying again, and on Kick only the channel with the most points redeems per pass,
  so goals aren't overshot. Each goal shows its time left on Twitch and Kick (green when met,
  outlined while being bought); the stats panel marks the goals on its bars. "Copy to all
  accounts" applies one account's setup to the others.
- **Running bonuses at a glance.** Each slot's header shows the account's PokeIdle bonuses that
  are running and the time left (e.g. `XP 6h12`, `Cap 1h40`; Kick's and Twitch's times on
  hover), read from the game's own state. The same appears in ⚙ → Bonus per account.
- **Stream health.** Every 30 seconds each background stream checks that its video is moving.
  A stalled stream is reloaded once quietly; if it still doesn't play, its pill gets a ⚠ and it
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

- [ ] Light mode: Kick via its HLS feed and Twitch via authenticated chat, without video players.

## Apoie o projeto ♥

If the app helps you and you'd like to chip in, any amount helps keep it going. In the app:
**♥ Apoiar** on the top bar, ⚙ → About, or the tray menu.

<img src="docs/pix-qr.svg" alt="Pix QR code" width="200">

**Pix copia e cola** (random key, you pick the amount):

```
00020126580014BR.GOV.BCB.PIX0136384baaaf-07f9-495b-96d0-f365b7ac99e55204000053039865802BR5924Thainan Henrique Feistel6009SAO PAULO62140510TlU51tMQRA6304856D
```

**Free ways to help:** give this repository a ⭐ (it helps more players find the app), and
invite friends who don't play PokeIdle yet with the referral link
<https://pokeidle.io/?ref=DBCHW36>.

Ideas, bugs or questions: Discord **dsplacemnt**.

## Disclaimer

Unofficial, community-made tool. **Not affiliated with or endorsed by pokeidle.io, Twitch, Kick,
Google or Discord.** Keeping streams open in the background and using several accounts may be
regulated by the terms of service of the game and the streaming platforms. Check the current
rules and use it at your own risk.

## License

[MIT](LICENSE).
