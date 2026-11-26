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

  // The rewards' names (every size of each kind; the cheapest wins) and how points read
  // (scripts/redeem-common.js, loaded first).
  const { KINDS, amount } = window.__multiviewRewards;
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const text = (el) => (el.innerText || el.textContent || '').trim().replace(/\s+/g, ' ');
  const shown = (el) => el.getClientRects().length > 0;

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
      if (!points) {
        // Shown as ⚠ on the stream's pill by the app's health check.
        window.__multiviewReport(`[multiview-health] ${JSON.stringify({
          state: 'logged-out', detail: 'Twitch shows no channel points here: is this account logged in?' })}`);
        return 'no channel points here (offline or logged out?)';
      }
      window.__multiviewReport(`[multiview-health] ${JSON.stringify({ state: 'logged-in', detail: '' })}`);

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
      // A viewer new to the channel first gets a welcome screen ("Comece agora mesmo!" /
      // "Get started") instead of the list; its button, inside the panel, opens the list.
      const tiles = await waitFor(() => {
        const t = rewardTiles(pattern);
        if (t.length) return t;
        const start = [...(panel()?.querySelectorAll('button') || [])].find((b) =>
          shown(b) && /comece agora|get started|empieza ahora|comienza ahora/i.test(text(b)));
        start?.click();
        return null;
      }, 12000);
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
      // The reward's detail view: its confirm button shows the cost (or says "Resgatar"), and
      // the panel shows that reward's title (only confirm in its own view). It may take a moment.
      const isConfirm = (b) => {
        const t = text(b);
        return shown(b) && (/^(resgatar|redeem|canjear)/i.test(t) || amount(t) === cheapest.cost);
      };
      // Twitch may show the confirm button disabled for a moment: wait for it to turn on.
      let found = null;
      await waitFor(() => {
        const root = panel();
        // Still the list (its tile on screen): not the detail view yet.
        if (!root || !text(root).includes(cheapest.title) || (cheapest.button.isConnected && shown(cheapest.button))) return null;
        found = [...root.querySelectorAll('button')].find(isConfirm) || null;
        return found && !found.disabled ? found : null;
      }, 8000);
      if (!found || found.disabled) {
        // What the view shows instead, so the log tells why (a limit per stream, a cooldown…):
        // Twitch explains it below the reward's description, at the end of the panel.
        const root = panel();
        const all = root ? text(root) : '';
        const seen = root
          ? `…${all.slice(-220)} | buttons: ${[...root.querySelectorAll('button')].filter(shown)
            .map((b) => text(b) + (b.disabled ? ' (disabled)' : '')).filter(Boolean).join(' / ')}`
          : 'panel closed';
        await closePanel();
        done.push(`${found ? 'redeem button disabled for' : 'no redeem button for'} ${cheapest.title} [${seen}]`);
        return done.join(', ');
      }
      const confirm = found;
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
