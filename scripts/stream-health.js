// Run on every check (every 2 minutes) in each background stream: tells the app whether the
// stream is really playing. Needs report.js.
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
  let video;
  if (/(^|\.)kick\.com$/.test(host)) {
    video = document.getElementById('video-player');
    if (!video) return report('offline', 'No live player on the channel page');
    if (!document.querySelector('[data-testid="channel-points-button"]')) {
      return report('logged-out', 'Kick shows no channel points here: is this account logged in?');
    }
    report('logged-in', '');
  } else if (host === 'player.twitch.tv') {
    video = document.querySelector('video');
    if (!video || (!video.currentTime && /offline/i.test(document.body.innerText || ''))) {
      return report('offline', 'The Twitch player says the channel is offline');
    }
  } else {
    return;
  }
  const time = video.currentTime;
  const last = window.__multiviewLastTime;
  window.__multiviewLastTime = time;
  // A freshly (re)loaded page has nothing to compare with yet.
  if (last === undefined) return report('first', '');
  if (video.paused || time <= last + 1) {
    return report('stalled', video.paused ? 'The video is paused' : `The video stopped at ${Math.round(time)} s`);
  }
  return report('ok', '');
})();
