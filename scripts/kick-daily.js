// Claims Kick's "Daily Reward" for this account, the way a person would: the gift button
// ("Resgate sua recompensa" / "Claim your reward") opens Kick's rewards dialog, a carousel with
// other cards too (e.g. "Sunset Reward Case"); this presses the "Resgatar"/"Claim" button of the
// Daily Reward card only, then closes whatever opened. It doesn't redeem channel points.
//
// The app evaluates this file followed by `__multiviewDaily()` at most a few times a day per
// account (only while the option is on in ⚙ → Kick). It reports through report.js:
// "[multiview-daily] {"result":"claimed"|"not available"|…}".
(() => {
  if (window.__multiviewDaily) return;
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const text = (el) => (el.innerText || el.textContent || '').trim().replace(/\s+/g, ' ');
  const shown = (el) => el.getClientRects().length > 0;
  const CLAIM = /^(resgatar|claim|reclamar)$/i;
  const report = (result, detail = '') =>
    window.__multiviewReport(`[multiview-daily] ${JSON.stringify({ result, detail })}`);

  async function closeAll() {
    for (let i = 0; i < 3; i++) {
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', code: 'Escape', bubbles: true }));
      await sleep(250);
    }
  }

  // The dialog is a carousel: one card on screen at a time ("Próxima recompensa" moves on), and
  // a claim button per card in a shared footer, where only the current card's is on screen.
  // So: bring the Daily Reward card on screen, then take the claim button that is on screen.
  async function dailyClaimButton(dialog) {
    const title = [...dialog.querySelectorAll('*')].find((e) => e.children.length === 0 && /^daily reward$/i.test(text(e)));
    if (!title) return null;
    const onScreen = (e) => {
      const d = dialog.getBoundingClientRect();
      const r = e.getBoundingClientRect();
      return shown(e) && r.width > 0 && r.left >= d.left - 1 && r.right <= d.right + 1;
    };
    const next = [...dialog.querySelectorAll('button')].find((b) =>
      /próxima recompensa|next reward|siguiente recompensa/i.test(b.getAttribute('aria-label') || ''),
    );
    for (let i = 0; i < 6 && !onScreen(title); i++) {
      if (!next) return null;
      next.click();
      await sleep(700);
    }
    if (!onScreen(title)) return null;
    return [...dialog.querySelectorAll('button')].find((b) => CLAIM.test(text(b)) && onScreen(b)) || null;
  }

  window.__multiviewDaily = async () => {
    try {
      const entry = [...document.querySelectorAll('button')].find((b) =>
        /resgate sua recompensa|claim your reward|reclama tu recompensa/i.test(b.getAttribute('aria-label') || ''),
      );
      if (!entry) return report('no rewards button', 'Kick shows no rewards button here (logged out?)');
      entry.click();
      await sleep(1500);
      const dialog = [...document.querySelectorAll('[role="dialog"]')].find((d) => shown(d) && /daily reward/i.test(text(d)));
      if (!dialog) {
        await closeAll();
        return report('no daily reward', 'The rewards dialog has no Daily Reward card');
      }
      const button = await dailyClaimButton(dialog);
      if (!button || button.disabled) {
        await closeAll();
        return report('not available', 'No claim button on the Daily Reward card (already claimed today?)');
      }
      button.click();
      await sleep(3000);
      const after = [...document.querySelectorAll('[role="dialog"]')].filter(shown).map(text).join(' | ').slice(0, 200);
      await closeAll();
      return report('claimed', after);
    } catch (e) {
      await closeAll();
      return report('error', String(e));
    }
  };
})();
