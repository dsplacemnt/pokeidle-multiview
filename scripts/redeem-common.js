// Shared by the app's redeem scripts in Kick and Twitch pages (kick-redeem.js, twitch-visit.js),
// loaded before them: the PokeIdle rewards' names and how a points amount reads.
(() => {
  if (window.__multiviewRewards) return;
  window.__multiviewRewards = {
    // Every size of each kind ("…", "… (3h)", "… (5h)", "… (10h)") at the same price per hour;
    // the scripts pick the cheapest.
    KINDS: {
      xp: /\+15% XP PokeIdle/i,
      capture: /\+15% Capture Boost/i,
      lure: /\+15% Secret Lure/i,
    },
    // "210", "1.000", "2,500", "1,8 mil", "12,3 mil", "1.2K" → a number of points.
    amount(raw) {
      const m = String(raw).trim().toLowerCase().match(/^([\d.,]+)\s*(mil|k)?/);
      if (!m) return NaN;
      if (m[2]) return Math.round(parseFloat(m[1].replace(',', '.')) * 1000);
      return Number(m[1].replace(/[.,]/g, ''));
    },
  };
})();
