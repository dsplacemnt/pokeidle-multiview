# Working on PokeIdle Multiview

Rules for anyone changing this repo (people and AI assistants alike).

## UI/UX comes first

The player's experience is the most important thing in this app. Every change that touches a
screen (ui/*.html, the slot headers, the stats panel, Settings, economy mode, any window) must
be **checked visually before it's done**:

- Look at the real result, not just the code: a screenshot of the page at the size it really
  has (the stats panel is 360 px wide; Settings and the slot headers change with the window).
- Check every place the change shows up, in **pt, en and es** (texts grow in Portuguese and
  Spanish), with long names and numbers, empty states and full lists.
- Nothing may overlap, touch, get cut off or wrap badly: cards keep their gaps, text ellipsizes
  instead of spilling, buttons keep their size, icons stay aligned.
- Keep the look consistent with what's already there (spacing, colors, radius, fonts); reuse
  the existing classes before adding new ones.
- Errors and warnings say in plain words what happened and what to do; destructive actions
  ask first.
- If something can't be checked (needs a real event, a login, a restart), say so instead of
  calling it done.

How to look at a page without restarting the app: copy it with i18n.js to a folder, stub
`window.__TAURI__` with sample data, and screenshot it with headless Edge; or, in a debug build
with `MULTIVIEW_DEVTOOLS=1`, capture the app's own pages through CDP on port 9300.

## Other rules

- Every release adds an entry to `ui/changelog.js` in pt, en and es.
- Release = bump the version in `Cargo.toml` and `tauri.conf.json`, commit "Release X.Y.Z: …",
  push `main`, then push the tag `vX.Y.Z` (never the tag before the commit).
- Don't run plain `cargo fmt`: the code isn't rustfmt-formatted and it rewrites every file.
- English in code comments; the app's texts are written in English and translated in
  `ui/i18n.js`.
- Game, Twitch and Kick pages never get IPC; they report through brief title changes
  (`[multiview-…] …`).
- Never log or show secrets: session tokens, cookie values, the Discord webhook, the signing key.
