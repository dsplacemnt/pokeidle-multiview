// Runs in every Twitch and Kick channel page (loaded once per page; needs report.js): tells the
// app whether this account follows the channel, at the start and again whenever it changes, so
// the ♡ tip on the stream's pill comes and goes right away. It only reads the page: the app
// never follows anyone.
//
// - Twitch (full channel page): a "follow-button" means not following, an "unfollow-button"
//   means following. Its light embed player has neither, so it says nothing there.
// - Kick: the "follow-button" says "Follow"/"Seguir" only while not following (following, it
//   is just an icon).
(() => {
  if (window.__multiviewFollowWatch) return;
  window.__multiviewFollowWatch = true;
  const host = location.hostname;
  const twitch = host === 'www.twitch.tv' || host === 'twitch.tv';
  const kick = /(^|\.)kick\.com$/.test(host);
  if (!twitch && !kick) return;

  function following() {
    if (twitch) {
      const follow = document.querySelector('[data-a-target="follow-button"]');
      const unfollow = document.querySelector('[data-a-target="unfollow-button"]');
      if (!follow && !unfollow) return null;
      return !follow;
    }
    const buttons = [...document.querySelectorAll('[data-testid="follow-button"]')];
    if (!buttons.length) return null;
    return !buttons.some((b) => /^(follow|seguir)$/i.test((b.textContent || '').trim()));
  }

  let last = null;
  let path = location.pathname;
  setInterval(() => {
    // Moving to another channel inside the page starts over.
    if (location.pathname !== path) {
      path = location.pathname;
      last = null;
    }
    const now = following();
    if (now === null || now === last) return;
    last = now;
    window.__multiviewReport(`[multiview-follow] ${JSON.stringify({ following: now })}`);
  }, 3000);
})();
