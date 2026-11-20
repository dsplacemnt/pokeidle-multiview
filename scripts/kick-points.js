// Run on every check in each live Kick page: tells the app this account's channel points on
// this channel, so it can prefer the channels where the account is closest to its next reward.
// Kick pages show the balance even in the background. Needs report.js.
(() => {
  if (!document.getElementById('video-player')) return;
  const button = document.querySelector('[data-testid="channel-points-button"]');
  if (!button) return;
  const raw = (button.innerText || button.textContent || '').trim().toLowerCase();
  const m = raw.match(/^([\d.,]+)\s*(mil|k)?/);
  if (!m) return;
  const points = m[2] ? Math.round(parseFloat(m[1].replace(',', '.')) * 1000) : Number(m[1].replace(/[.,]/g, ''));
  const channel = (location.pathname.split('/').filter(Boolean)[0] || '').toLowerCase();
  if (!channel || !Number.isFinite(points)) return;
  window.__multiviewReport(`[multiview-points] ${JSON.stringify({ platform: 'kick', channel, points })}`);
})();
