// Run every 30 seconds in each background stream (and 10 s after an "offline", to confirm it):
// tells the app whether the stream is really playing. Needs report.js.
//
// - "ok": the video moved forward since the last check;
// - "stalled": it didn't (or it is paused) — the app reloads the page, and warns if that
//   doesn't help;
// - "logged-out": Kick shows no channel points, so the account is not logged in there;
// - "offline": there is no live player (the channel ended its stream).
//
// Only the light background pages are checked (Kick channel pages and Twitch's embed player);
// full Twitch pages (visits, or the stream being shown) are left alone.
(() => {
  const host = location.hostname;
  const report = (state, detail) =>
    window.__multiviewReport(`[multiview-health] ${JSON.stringify({ state, detail })}`);
  // A page that is still loading has no player yet: that's not "offline" (it used to raise a
  // false ⚠ on every stream the manager had just opened).
  const young = performance.now() < 45000;
  // The player's own "offline" screen (it stays on the last frame when a live ends). Hidden
  // parts (Kick's chat in light mode) don't count: innerText skips them.
  const saysOffline = () => /\boffline\b/i.test(document.body.innerText || '');
  let video;
  let offlineDetail;
  if (/(^|\.)kick\.com$/.test(host)) {
    offlineDetail = 'No live player on the channel page';
    video = document.getElementById('video-player');
    if (!video) return report(young ? 'first' : 'offline', offlineDetail);
    if (!document.querySelector('[data-testid="channel-points-button"]')) {
      return report('logged-out', 'Kick shows no channel points here: is this account logged in?');
    }
    report('logged-in', '');
    // Whether this account follows the channel (only read): the app shows a tip on the pill
    // when it doesn't. Kick's follow button says "Follow"/"Seguir" only while not following.
    const follow = [...document.querySelectorAll('[data-testid="follow-button"]')];
    if (follow.length) {
      const notFollowing = follow.some((b) => /^(follow|seguir)$/i.test((b.textContent || '').trim()));
      window.__multiviewReport(`[multiview-follow] ${JSON.stringify({ following: !notFollowing })}`);
    }
  } else if (host === 'player.twitch.tv') {
    offlineDetail = 'The Twitch player says the channel is offline';
    video = document.querySelector('video');
    if (!video) return report(young ? 'first' : 'offline', offlineDetail);
    if (!video.currentTime && saysOffline()) return report('offline', offlineDetail);
  } else {
    return;
  }
  const time = video.currentTime;
  const last = window.__multiviewLastTime;
  window.__multiviewLastTime = time;
  const stuck = video.paused || video.ended || (last !== undefined && time <= last + 1);
  // A live that ended leaves the player on its last frame with an "offline" screen: that's
  // offline now, not a stall to reload first.
  if (stuck && saysOffline()) return report('offline', offlineDetail);
  // Merely paused (the player paused itself): pressed play here, which keeps the watch time;
  // only after a few checks still paused is it a stall the app reloads.
  if (video.paused && !video.ended) {
    const tries = (window.__multiviewPlayTries || 0) + 1;
    window.__multiviewPlayTries = tries;
    video.play().catch(() => {});
    if (tries < 3) return report('first', 'pressed play');
  } else {
    window.__multiviewPlayTries = 0;
  }
  // A freshly (re)loaded page has nothing to compare with yet; neither has a player that
  // started over while playing (Twitch swaps in a new video for ads or after reconnecting).
  if (last === undefined || (!video.paused && time + 1 < last)) return report('first', '');
  if (stuck) {
    return report('stalled', video.paused ? 'The video is paused' : `The video stopped at ${Math.round(time)} s`);
  }
  return report('ok', '');
})();
