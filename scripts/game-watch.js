// Loaded at the start of every game page. PokeIdle's server keeps sending the game state over
// its WebSocket ({"t":"estado","estado":{…}}), including, for Kick and Twitch, the game's
// official partner channels ("oficiais": slug/login, aoVivo, ativo), whether this account has
// that platform linked ("vinculado") and when its bonuses from that platform end ("bonus":
// xp/captura/shiny, server time in ms). This listens to those messages (it never sends
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

  // Session numbers for the app's stats panel: gold and XP now and gained since this page
  // loaded (with hourly rates), balls left and the wild Pokémon being fought. The state comes
  // in pieces (only what changed), so each number keeps its last value. Sent at most every 30 s:
  // "[multiview-stats] [["Gold","77.8M"],…]".
  const session = { start: Date.now() };
  let statsSent = '';
  let statsAt = 0;
  const short = (n) => {
    const a = Math.abs(n);
    if (a >= 1e9) return `${(n / 1e9).toFixed(2)}B`;
    if (a >= 1e6) return `${(n / 1e6).toFixed(1)}M`;
    if (a >= 1e4) return `${(n / 1e3).toFixed(1)}k`;
    return String(Math.round(n));
  };
  function sessionStats(state) {
    for (const key of ['gold', 'xp']) {
      const v = Number(state[key]);
      if (!Number.isFinite(v)) continue;
      if (session[key + '0'] === undefined) session[key + '0'] = v;
      session[key] = v;
    }
    if (state.balls && typeof state.balls === 'object') {
      session.balls = Object.values(state.balls).reduce((s, n) => s + (Number(n) || 0), 0);
    }
    const wild = state.selvagem;
    if (wild && wild.nome) session.wild = `${wild.nome} Lv ${wild.level ?? '?'}`;
    const hours = (Date.now() - session.start) / 3600000;
    const rows = [];
    for (const [key, label] of [['gold', 'Gold'], ['xp', 'XP']]) {
      if (session[key] === undefined) continue;
      const gained = session[key] - session[key + '0'];
      rows.push([label, short(session[key])]);
      rows.push([`${label} this session`, `+${short(gained)}`]);
      // A rate only once there's enough time behind it to mean something.
      if (hours >= 5 / 60) rows.push([`${label} per hour`, `+${short(gained / hours)}`]);
    }
    if (session.balls !== undefined) rows.push(['Balls left', short(session.balls)]);
    if (session.wild) rows.push(['Fighting', session.wild]);
    const text = JSON.stringify(rows);
    if (!rows.length || text === statsSent || Date.now() - statsAt < 30000) return;
    statsSent = text;
    statsAt = Date.now();
    queue.push(`[multiview-stats] ${text}`);
    if (!busy) next();
  }

  function inspect(data) {
    if (typeof data !== 'string' || !data.includes('"estado"')) return;
    let message;
    try { message = JSON.parse(data); } catch { return; }
    const state = message && message.estado;
    if (!state) return;
    sessionStats(state);
    if (!data.includes('"oficiais"')) return;
    for (const platform of ['kick', 'twitch']) {
      const info = state[platform];
      if (!info || !Array.isArray(info.oficiais)) continue;
      const channels = info.oficiais
        .filter((c) => c && c.ativo !== false)
        .map((c) => String(c.slug || c.login || '').toLowerCase())
        .filter(Boolean);
      const linked = info.vinculado === true;
      // When each bonus of this account on this platform ends (server time, ms; 0 = none).
      const b = info.bonus || {};
      const bonus = { xp: Number(b.xp) || 0, capture: Number(b.captura) || 0, lure: Number(b.shiny) || 0 };
      const key = JSON.stringify([channels, linked, bonus]);
      if (last[platform] === key) continue;
      last[platform] = key;
      // The server's clock, so the app can correct a PC clock that is off.
      const now = Number(state.servidorAgora) || 0;
      report({ platform, channels, linked, bonus, now });
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
