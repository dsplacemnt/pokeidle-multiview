# Security and your accounts

PokeIdle Multiview runs your PokeIdle, Twitch and Kick accounts inside the app, so it's fair to
ask what it does with them. The whole source is in this repository; this page says where to
look.

## What the app does with your accounts

- **Logins stay on your PC.** Each slot is a WebView2 (Edge) browser profile in
  `%LOCALAPPDATA%\io.pokeidle-multiview.app\slots\slot-N\`. The app never reads your passwords;
  you type them into the real Twitch, Kick, Google or PokeIdle pages.
- **It never sends your sessions anywhere.** To know whether an account is logged in, the app
  only checks that the login cookie *exists* (`src/logins.rs`); the game's session is never
  read out (`scripts/game-watch.js`, which only listens to what the game already sends the
  page).
- **What goes out over the network:** the pages you open (PokeIdle, Twitch, Kick), GitHub (to
  look for updates), and — only if you set one — your own Discord webhook, with the alerts you
  chose in ⚙ → Discord. No telemetry, no analytics, no servers of ours.
- **Game, Twitch and Kick pages can't talk to the app.** Only the app's own pages (top bar,
  slot headers, Settings) can call it; the game pages report through brief title changes the
  app reads, and the app decides what to do with them.
- **Links stay contained.** Slot pages may only go to pokeidle.io, Twitch, Kick and the Google
  and Discord sign-in pages; any other link opens in your normal browser.
- **The app plays like you do.** It watches streams, redeems channel-point rewards you chose,
  claims the Kick daily reward if you turned it on, and reminds you of votes and the daily
  Prey. It never follows channels, never votes for you, and never spends Coins or items on its
  own. The only thing it does in the game is press the game's own Battery Saver button while
  the app's economy mode hides the accounts (and press it back afterwards).

## The log

The app writes what it does to `%LOCALAPPDATA%\io.pokeidle-multiview.app\logs\app.log` (on your PC
only; it is never sent anywhere). It has the app, Windows and WebView2 versions, the streams
opened and closed, reloads, errors and crashes. It never has passwords, cookies or tokens, and
each line is still filtered for anything that looks like a token or a Discord webhook. ⚙ →
About → **Copy diagnostic** copies its last lines to paste in an issue or on Discord.

## Checking what you download

- Every release is built by GitHub Actions from this repository (`.github/workflows/release.yml`),
  not on a developer's PC. You can read the build log of each release in the Actions tab.
- Each release has a `SHA256SUMS.txt`. Compare it with your file:
  - Windows (PowerShell): `Get-FileHash .\PokeIdleMultiview_<version>_x64-setup.exe`
  - Linux: `sha256sum PokeIdleMultiview_<version>_amd64.AppImage`
- In-app updates are **signed**: the app only installs an update whose signature matches the
  public key built into it (`tauri.conf.json`), so a tampered download is refused.
- The installers aren't code-signed (that costs money), so Windows SmartScreen may warn the
  first time.

## Reporting a problem

If you find a security issue, please report it privately through
[GitHub's private vulnerability reporting](https://github.com/dsplacemnt/pokeidle-multiview/security/advisories/new)
instead of opening a public issue. Say what you found and how to reproduce it.

**Never post** passwords, cookies, session tokens, your Discord webhook address, or screenshots
that show them, in issues or on Discord. Nobody from this project will ever ask for them.
