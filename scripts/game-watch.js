// Loaded at the start of every game page. PokeIdle's server keeps sending the game state over
// its WebSocket ({"t":"estado","estado":{…}}), including, for Kick and Twitch, the game's
// official partner channels ("oficiais": slug/login, aoVivo, ativo) and whether this account
// has that platform linked ("vinculado"). This listens to those messages (it never sends
// anything) and tells the app when one of them changes, through a brief title change:
// "[multiview-game] {"platform":"kick","channels":[…],"linked":true}".
(() => {
  const Native = window.WebSocket;
  if (!Native || Native.__multiview) return;
  const last = {};

  // Like report.js (inlined: this runs before the page's own scripts, on every game load).
  const queue = [];
  let original = null;
  let busy = false;
  function next() {
    if (!queue.length) {
      busy = false;
      if (original !== null && document.title.startsWith('[multiview')) document.title = original;
      original = null;
      return;
    }
    busy = true;
    if (original === null) original = document.title;
    document.title = queue.shift();
    setTimeout(next, 150);
  }
  function report(result) {
    queue.push(`[multiview-game] ${JSON.stringify(result)}`);
    if (!busy) next();
  }

  function inspect(data) {
    if (typeof data !== 'string' || !data.includes('"oficiais"')) return;
    let message;
    try { message = JSON.parse(data); } catch { return; }
    const state = message && message.estado;
    if (!state) return;
    for (const platform of ['kick', 'twitch']) {
      const info = state[platform];
      if (!info || !Array.isArray(info.oficiais)) continue;
      const channels = info.oficiais
        .filter((c) => c && c.ativo !== false)
        .map((c) => String(c.slug || c.login || '').toLowerCase())
        .filter(Boolean);
      const linked = info.vinculado === true;
      const key = JSON.stringify([channels, linked]);
      if (last[platform] === key) continue;
      last[platform] = key;
      report({ platform, channels, linked });
    }
  }

  class WatchedWebSocket extends Native {
    constructor(...args) {
      super(...args);
      this.addEventListener('message', (event) => {
        try { inspect(event.data); } catch {}
      });
    }
  }
  WatchedWebSocket.__multiview = true;
  window.WebSocket = WatchedWebSocket;
})();
