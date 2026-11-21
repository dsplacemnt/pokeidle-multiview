// Loaded at the start of every Twitch/Kick stream page: the page always looks visible, so a
// player never pauses itself for not being watched (behind other windows, screen off). The
// window itself never really minimizes or hides (see src/away.rs).
(() => {
  const visible = { configurable: true, get: () => 'visible' };
  const notHidden = { configurable: true, get: () => false };
  try {
    Object.defineProperty(Document.prototype, 'visibilityState', visible);
    Object.defineProperty(Document.prototype, 'hidden', notHidden);
    Object.defineProperty(Document.prototype, 'webkitVisibilityState', visible);
    Object.defineProperty(Document.prototype, 'webkitHidden', notHidden);
  } catch {}
  // Stopped before the page's own listeners see it.
  for (const type of ['visibilitychange', 'webkitvisibilitychange']) {
    window.addEventListener(type, (e) => e.stopImmediatePropagation(), true);
    document.addEventListener(type, (e) => e.stopImmediatePropagation(), true);
  }
})();
