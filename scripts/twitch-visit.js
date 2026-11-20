// Runs in a background Twitch stream while the app briefly shows its full channel page (the
// light embed player has no channel points). It does what a person would do there:
//
// 1. claims the bonus chest ("Resgatar Bônus" / "Claim Bonus"), worth +50 points;
// 2. if a bonus was chosen for the account, redeems the cheapest PokeIdle reward of that kind
//    ("Bônus +15% XP PokeIdle", "Bônus de +15% Capture Boost", "Bônus de +15% Secret Lure")
//    when the points cover it. Every size costs the same per hour and the game adds the hours
//    up, so the cheapest one as soon as possible wastes nothing.
//
// It only presses buttons inside the channel points panel, and the final one only in that
// reward's own detail view. The app evaluates this file followed by
// `__multiviewTwitchVisit("xp" | "capture" | "lure" | null)`; it resolves to a short text.
(() => {
  if (window.__multiviewTwitchVisit) return;

  // Every size of each kind ("…", "… (3h)", "… (5h)", "… (10h)"); the cheapest wins.
  const KINDS = {
    xp: /\+15% XP PokeIdle/i,
    capture: /\+15% Capture Boost/i,
    lure: /\+15% Secret Lure/i,
  };
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const text = (el) => (el.innerText || el.textContent || '').trim().replace(/\s+/g, ' ');
  const shown = (el) => el.getClientRects().length > 0;

  // "350", "1.500", "2,5 mil", "12,3 mil", "1.2K" → points.
  function amount(raw) {
    const m = String(raw).trim().toLowerCase().match(/^([\d.,]+)\s*(mil|k)?/);
    if (!m) return NaN;
    if (m[2]) return Math.round(parseFloat(m[1].replace(',', '.')) * 1000);
    return Number(m[1].replace(/[.,]/g, ''));
  }

  async function waitFor(find, ms) {
    for (let waited = 0; waited < ms; waited += 500) {
      const found = find();
      if (found) return found;
      await sleep(500);
    }
    return null;
  }

  const summary = () => document.querySelector('[data-test-selector="community-points-summary"]');
  const panel = () =>
    document.querySelector('[data-test-selector="community-points-popover"]') ||
    document.querySelector('.reward-center__content') ||
    document.querySelector('[class*="reward-center"]');

  // The reward tile whose title matches `pattern`: Twitch puts the cost on the tile's button and
  // the title next to it, so the search goes from the title up to the nearest box with a button.
  function rewardTiles(pattern) {
    const root = panel();
    if (!root) return [];
    const titles = [...root.querySelectorAll('p, span, div, h3, h4')].filter(
      (el) => el.children.length === 0 && pattern.test(text(el)),
    );
    const tiles = [];
    for (const title of titles) {
      let box = title.parentElement;
      for (let i = 0; box && box !== root && i < 6; i++, box = box.parentElement) {
        const buttons = [...box.querySelectorAll('button')];
        if (buttons.length === 1) {
          const cost = amount(text(buttons[0]));
          if (Number.isFinite(cost)) tiles.push({ button: buttons[0], cost, title: text(title) });
          break;
        }
        if (buttons.length > 1) break;
      }
    }
    return tiles.sort((a, b) => a.cost - b.cost);
  }

  async function closePanel() {
    if (panel()) summary()?.querySelector('button')?.click();
    await sleep(300);
    if (panel()) document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', code: 'Escape', bubbles: true }));
  }

  let busy = false;
  window.__multiviewTwitchVisit = async (kind) => {
    if (busy) return 'busy';
    busy = true;
    const result = await visit(kind);
    busy = false;
    // Kept on the page, and told to the app (which logs it) through a brief title change.
    window.__multiviewLastVisit = result;
    window.__multiviewReport(`[multiview] Twitch visit: ${result}`);
    return result;
  };

  // The account's points on this channel, so the app can prefer the channels where it is
  // closest to its next reward.
  function reportPoints(points) {
    const channel = (location.pathname.split('/').filter(Boolean)[0] || '').toLowerCase();
    if (!channel || !Number.isFinite(points)) return;
    window.__multiviewReport(`[multiview-points] ${JSON.stringify({ platform: 'twitch', channel, points })}`);
  }

  async function visit(kind) {
    const done = [];
    try {
      const points = await waitFor(summary, 20000);
      if (!points) return 'no channel points here (offline or logged out?)';

      // 1. The bonus chest.
      const chest = [...points.querySelectorAll('button')].find((b) =>
        /resgatar b[ôo]nus|claim bonus|reclamar bono/i.test(b.getAttribute('aria-label') || ''),
      );
      if (chest) {
        chest.click();
        done.push('chest claimed');
        await sleep(1500);
      }

      // The balance (after the chest): the button shows Bits and channel points side by side;
      // the points have their own element.
      const balanceButton = points.querySelector('button');
      const pointsBalance = points.querySelector('[data-test-selector="copo-balance-string"]');
      const balance = amount(text(pointsBalance || balanceButton));
      reportPoints(balance);

      // 2. The bonus, if one was chosen.
      const pattern = KINDS[kind];
      if (!pattern) return done.join(', ') || 'nothing to do';
      balanceButton.click();
      const tiles = await waitFor(() => { const t = rewardTiles(pattern); return t.length ? t : null; }, 5000);
      if (!tiles) {
        await closePanel();
        done.push('no such reward on this channel');
        return done.join(', ');
      }
      const cheapest = tiles[0];
      if (!(balance >= cheapest.cost)) {
        await closePanel();
        done.push(`not enough points (${balance}/${cheapest.cost})`);
        return done.join(', ');
      }
      cheapest.button.click();
      await sleep(1200);
      // The reward's detail view: its confirm button shows the cost (or says "Resgatar").
      const root = panel();
      const confirm = root && [...root.querySelectorAll('button')].find((b) => {
        const t = text(b);
        return shown(b) && !b.disabled && (/^(resgatar|redeem|canjear)\b/i.test(t) || amount(t) === cheapest.cost);
      });
      // Only confirm in that reward's own view: the panel must now be showing its title.
      const titleShown = !!root && text(root).includes(cheapest.title);
      if (!confirm || !titleShown) {
        await closePanel();
        done.push(`no redeem button for ${cheapest.title}`);
        return done.join(', ');
      }
      confirm.click();
      await sleep(2000);
      await closePanel();
      done.push(`redeemed ${cheapest.title} (${cheapest.cost})`);
      reportPoints(balance - cheapest.cost);
      return done.join(', ');
    } catch (e) {
      await closePanel();
      return `error: ${e}`;
    }
  }
})();
