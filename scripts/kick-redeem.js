// Redeems the account's chosen PokeIdle bonus with this Kick channel's points, the way a
// person would: channel points button → the cheapest reward of that kind → "Resgatar"/"Redeem".
//
// PokeIdle's channels offer "Bônus +15% XP PokeIdle", "Bônus de +15% Capture Boost" and
// "Bônus de +15% Secret Lure" in several sizes at the same price per hour, and the game adds
// the hours up, so redeeming the cheapest one as soon as the points cover it never wastes
// anything and starts the bonus sooner. (The account must have its Kick linked in the game,
// "Bônus na Kick"; otherwise Kick gives the points back.)
//
// The app evaluates this file followed by `__multiviewRedeem("xp" | "capture" | "lure")` every
// few minutes in the account's live Kick pages. It does nothing while the points are short,
// and it only presses the redeem button inside that reward's own dialog (never Kick's own
// daily reward or anything else). It resolves to a short text saying what happened.
(() => {
  if (window.__multiviewRedeem) return;

  const KINDS = {
    xp: /\+15% XP PokeIdle/i,
    capture: /\+15% Capture Boost/i,
    lure: /\+15% Secret Lure/i,
  };
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const text = (el) => (el.innerText || el.textContent || '').trim().replace(/\s+/g, ' ');

  // "210", "1.000", "2,500", "1,8 mil", "12 mil", "1.2K" → a number of points.
  function amount(raw) {
    const m = String(raw).trim().toLowerCase().match(/^([\d.,]+)\s*(mil|k)?/);
    if (!m) return NaN;
    if (m[2]) return Math.round(parseFloat(m[1].replace(',', '.')) * 1000);
    return Number(m[1].replace(/[.,]/g, ''));
  }

  const dialogs = () => [...document.querySelectorAll('[role="dialog"]')].filter((d) => d.getClientRects().length);
  async function closeAll() {
    for (let i = 0; i < 3; i++) {
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', code: 'Escape', bubbles: true }));
      await sleep(250);
    }
  }

  let busy = false;
  window.__multiviewRedeem = async (kind) => {
    const pattern = KINDS[kind];
    if (!pattern) return `unknown bonus ${kind}`;
    if (busy) return 'busy';
    busy = true;
    try {
      if (!document.getElementById('video-player')) return 'not live';
      const pointsButton = document.querySelector('[data-testid="channel-points-button"]');
      if (!pointsButton) return 'no points button (logged out?)';
      const balance = amount(text(pointsButton));
      // The cheapest price seen on this channel, so the panel isn't opened while points are short.
      const known = Number(sessionStorage.getItem(`multiview-cost-${kind}`));
      if (known && balance < known) return `not enough points (${balance}/${known})`;

      pointsButton.click();
      await sleep(1500);
      const rewards = dialogs()
        .flatMap((d) => [...d.querySelectorAll('button')])
        .map((button) => ({ button, label: text(button) }))
        .filter((r) => pattern.test(r.label))
        .map((r) => ({ ...r, cost: amount(r.label) }))
        .filter((r) => Number.isFinite(r.cost))
        .sort((a, b) => a.cost - b.cost);
      if (!rewards.length) {
        await closeAll();
        return 'this channel has no such reward';
      }
      const cheapest = rewards[0];
      sessionStorage.setItem(`multiview-cost-${kind}`, String(cheapest.cost));
      if (balance < cheapest.cost) {
        await closeAll();
        return `not enough points (${balance}/${cheapest.cost})`;
      }

      cheapest.button.click();
      await sleep(1200);
      // The reward's own dialog: it names the reward and holds the redeem button.
      const detail = dialogs().find((d) => pattern.test(text(d)) && [...d.querySelectorAll('button')].some((b) => /^(resgatar|redeem|canjear)$/i.test(text(b))));
      const redeem = detail && [...detail.querySelectorAll('button')].find((b) => /^(resgatar|redeem|canjear)$/i.test(text(b)));
      if (!redeem || redeem.disabled) {
        const why = detail ? 'redeem button disabled' : 'no redeem button';
        await closeAll();
        return `${why} for ${cheapest.label}`;
      }
      redeem.click();
      await sleep(2000);
      await closeAll();
      return `redeemed ${cheapest.label}`;
    } catch (e) {
      await closeAll();
      return `error: ${e}`;
    } finally {
      busy = false;
    }
  };

  // Tells the app (which logs it) what happened, through a brief title change. The routine
  // "not enough points" / "not live" answers of every check are left out.
  const redeemOnly = window.__multiviewRedeem;
  window.__multiviewRedeem = async (kind) => {
    const result = await redeemOnly(kind);
    if (!/^(not enough points|not live|busy)/.test(result)) {
      window.__multiviewReport(`[multiview] Kick redeem: ${result}`);
    }
    return result;
  };
})();
