// Runs before the game's scripts. pokeidle.io switches to its phone layout when the page is
// 720px wide or less, and decides only once, at load. A 2×2 grid on a laptop screen is often
// that narrow, so default the game to its desktop layout (it then scales itself down to fit).
// "cfg-movel" is the game's own setting: we only fill it in when the player never picked one.
(() => {
  try {
    if (!/(^|\.)pokeidle\.io$/.test(location.hostname)) return;
    if (localStorage.getItem('cfg-movel') === null) localStorage.setItem('cfg-movel', '0');
  } catch {}
})();
