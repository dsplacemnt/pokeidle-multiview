// Lighter Kick channel page for a stream running hidden in the background. Kick's embed player
// shows live channels as offline, so the full page stays loaded and this trims it instead:
//
// - hides what surrounds the video (chat, sidebars, navigation, footer) and stops CSS
//   animations, so the page does far less layout and painting;
// - pauses the page's decorative looping videos (reward animations);
// - picks the player's lowest quality ("160p"), like the Twitch light player does.
//
// The video itself and every element around it are never hidden: the browser may pause a
// hidden video, and the stream must keep playing to keep counting.
//
// The app evaluates this file followed by `__multiviewLight(true)` when the page goes to the
// background and `__multiviewLight(false)` when it is shown again (which also puts the quality
// back on "Auto"). Evaluating it again is harmless. If Kick changes its page or player menu,
// the parts that can't find their target simply do nothing.
(() => {
  if (window.__multiviewLight) return;

  const LIGHT = 'mv-light';
  const HIDE = 'mv-light-hide';
  const style = document.createElement('style');
  style.textContent = `
    html.${LIGHT} *, html.${LIGHT} *::before, html.${LIGHT} *::after {
      animation: none !important;
      transition: none !important;
    }
    html.${LIGHT} .${HIDE} { display: none !important; }
  `;

  // The parts of the page around the player.
  const AROUND = [
    '#channel-chatroom',
    '#chatroom',
    '[id*="chat" i]',
    '[class*="chatroom" i]',
    '#sidebar-wrapper',
    '[id*="sidebar" i]',
    '[data-testid*="sidebar" i]',
    'aside',
    'nav',
    'footer',
  ].join(',');

  // The stream's own video. The page has others too: looping reward animations
  // (static.kick.com/rewards/…) and ad placeholders.
  function streamVideo() {
    const byId = document.getElementById('video-player');
    if (byId) return byId;
    return [...document.querySelectorAll('video')]
      .filter((v) => !v.loop && (v.currentSrc || v.src || '').startsWith('blob:'))
      .sort((a, b) => b.videoWidth - a.videoWidth)[0] || null;
  }

  // Decorative videos paused while the page is light, played again when it is shown.
  const pausedExtras = new Set();
  function pauseExtras() {
    const player = streamVideo();
    for (const v of document.querySelectorAll('video')) {
      if (v === player || v.paused || !v.loop) continue;
      v.pause();
      pausedExtras.add(v);
    }
  }

  // Returns how many parts it hid or showed again (0 = the page was already in shape).
  function hideAround() {
    const player = streamVideo();
    // Reward animations play on offline channel pages too.
    pauseExtras();
    // Until the player exists, nothing is hidden: a wrapper hidden now could receive it later.
    if (!player) return 1;
    let changed = 0;
    // A part hidden earlier that has since received the player comes back.
    for (const el of document.querySelectorAll(`.${HIDE}`)) {
      if (el.contains(player)) { el.classList.remove(HIDE); changed++; }
    }
    for (const el of document.querySelectorAll(AROUND)) {
      if (el.classList.contains(HIDE) || el.contains(player)) continue;
      el.classList.add(HIDE);
      changed++;
    }
    return changed;
  }

  // Kick is a single-page app that keeps adding content (chat messages, recommendations), so
  // new parts are hidden too: at most once a second, or every 5 s once several sweeps in a row
  // found nothing to do (the hidden chat keeps changing the page all the time).
  let observer = null;
  let sweep = 0;
  let quiet = 0;
  const scheduleSweep = () => {
    if (sweep) return;
    sweep = setTimeout(() => {
      sweep = 0;
      quiet = hideAround() ? 0 : quiet + 1;
    }, quiet >= 5 ? 5000 : 1000);
  };

  // --- player quality -------------------------------------------------------------------
  // Kick's player only draws its controls while the mouse is over it, so the script "hovers"
  // the video, presses its Quality button (aria-label "Quality"/"Qualidade"/"Calidad") and
  // picks an option. The options are radio buttons whose value is the quality ("Auto",
  // "1080p60", …, "160p"), shown in a menu or, on a small page, in a sheet from the bottom.
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const height = (value) => Number((String(value).match(/(\d{3,4})p/) || [])[1] || NaN);
  const shown = (el) => el.getClientRects().length > 0;

  function hover(video) {
    const r = video.getBoundingClientRect();
    const at = { bubbles: true, cancelable: true, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2, view: window };
    for (const type of ['pointerover', 'pointerenter', 'mouseover', 'mouseenter', 'pointermove', 'mousemove']) {
      video.dispatchEvent(new (type.startsWith('pointer') ? PointerEvent : MouseEvent)(type, at));
    }
  }

  function qualityButton(video) {
    // Climb from the video to the box that holds the player's controls.
    let box = video.parentElement;
    for (let i = 0; box && i < 8; i++, box = box.parentElement) {
      const button = [...box.querySelectorAll('button')].find((b) =>
        /quality|qualidade|calidad/i.test(b.getAttribute('aria-label') || ''),
      );
      if (button) return button;
    }
    return null;
  }

  const qualityRadios = () =>
    [...document.querySelectorAll('[role="radio"][value]')].filter(
      (r) => shown(r) && (Number.isFinite(height(r.getAttribute('value'))) || /^auto/i.test(r.getAttribute('value'))),
    );

  let qualityRun = 0;
  // Whether this script lowered the quality, so showing the page only touches the menu then.
  let lowered = false;
  async function setQuality(lowest) {
    const run = ++qualityRun;
    if (!lowest && !lowered) return null;
    for (let attempt = 0; attempt < 15 && run === qualityRun; attempt++) {
      const video = streamVideo();
      if (!video) {
        await sleep(2000);
        continue;
      }
      hover(video);
      await sleep(400);
      const button = qualityButton(video);
      if (!button) {
        await sleep(2000);
        continue;
      }
      button.click();
      await sleep(700);
      const radios = qualityRadios();
      if (!radios.length) {
        document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', code: 'Escape', bubbles: true }));
        await sleep(2000);
        continue;
      }
      const pick = lowest
        ? radios
            .filter((r) => Number.isFinite(height(r.getAttribute('value'))))
            .sort((a, b) => height(a.getAttribute('value')) - height(b.getAttribute('value')))[0]
        : radios.find((r) => /^auto/i.test(r.getAttribute('value')));
      if (pick && pick.getAttribute('aria-checked') !== 'true') pick.click();
      if (pick) lowered = lowest;
      await sleep(500);
      // Picking usually closes it; if not, close it like a person would.
      if (qualityRadios().length) {
        document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', code: 'Escape', bubbles: true }));
      }
      return pick ? pick.getAttribute('value') : null;
    }
    return null;
  }

  window.__multiviewLight = (on) => {
    const root = document.documentElement;
    if (on) {
      if (!style.isConnected) (document.head || root).appendChild(style);
      root.classList.add(LIGHT);
      quiet = 0;
      hideAround();
      if (!observer) {
        observer = new MutationObserver(scheduleSweep);
        observer.observe(document.body || root, { childList: true, subtree: true });
      }
    } else {
      root.classList.remove(LIGHT);
      observer?.disconnect();
      observer = null;
      clearTimeout(sweep);
      sweep = 0;
      for (const el of document.querySelectorAll(`.${HIDE}`)) el.classList.remove(HIDE);
      for (const v of pausedExtras) v.play().catch(() => {});
      pausedExtras.clear();
    }
    return setQuality(on);
  };
})();
