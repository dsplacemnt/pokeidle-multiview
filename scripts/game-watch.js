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

  // Hunt analysis for the app's stats panel, from what the game already sends this page (it
  // never asks the server anything): kills, XP and gold per hour, what the supplies cost, how
  // many balls, potions and revives are used per hour and how long the stock lasts, the time to
  // the next level and the catch rate. Counted since this page loaded, and again from zero when
  // the player changes hunts. Sent at most every 30 s: "[multiview-stats] {…}".
  //
  // What the game sends: "estado" with only what changed (balls after each throw, xp, gold,
  // the active Pokémon) and, every few minutes, the whole state ("cheio": the bag in "items",
  // the auto-use choices in "automation"); "batalha" events (morte = a kill, bola = a throw,
  // capturado, cura = a potion used, revive); and, when it connects, the ball catalog.
  const POTIONS = new Set([200, 201, 202, 203, 204, 207]);
  const REVIVES = new Set([205, 206]);
  const names = new Map(); // item/ball id → { name, price }
  let ballCatalog = new Map(); // ball id → { name, price }
  fetch('/assets/items.json')
    .then((r) => r.json())
    .then((j) => { for (const i of j.items || j || []) names.set(i.id, { name: i.name, price: Number(i.npcPrice) || 0 }); })
    .catch(() => {});

  const game = { balls: {}, items: null, itemsAt: 0, xp: null, xpNext: null, level: null, pokemon: null, hunt: null, wild: null, gold: null };
  let run = null;
  const newRun = () => ({
    start: Date.now(), kills: 0, xp: 0, gold: 0, throws: 0, catches: 0, shinies: 0,
    used: { ball: {}, potion: {}, revive: {} },
    potionsSinceBag: {}, // potions used since the last full bag, to keep the stock live
  });
  const add = (map, id, n = 1) => { map[id] = (map[id] || 0) + n; };
  let statsSent = '';
  let statsAt = 0;

  function onState(state) {
    if (typeof state.huntSlug === 'string' && state.huntSlug !== game.hunt) {
      // A new hunt: its own numbers.
      if (game.hunt !== null) run = newRun();
      game.hunt = state.huntSlug;
    }
    if (!run) run = newRun();
    if (state.balls && typeof state.balls === 'object') {
      for (const [id, n] of Object.entries(state.balls)) game.balls[id] = Number(n) || 0;
    }
    if (state.items && typeof state.items === 'object') {
      game.items = {};
      for (const [id, n] of Object.entries(state.items)) game.items[id] = Number(n) || 0;
      run.potionsSinceBag = {};
    }
    if (Number.isFinite(Number(state.xp))) game.xp = Number(state.xp);
    if (Number.isFinite(Number(state.xpProximo))) game.xpNext = Number(state.xpProximo);
    if (Number.isFinite(Number(state.level))) game.level = Number(state.level);
    if (Number.isFinite(Number(state.gold))) game.gold = Number(state.gold);
    const pk = Array.isArray(state.pkMud) ? state.pkMud.find((p) => p && p.slot === 0) || state.pkMud[0] : null;
    if (pk && pk.nome) game.pokemon = { name: pk.nome, level: pk.level, xp: Number(pk.xp), next: Number(pk.xpProximo) };
    if (state.selvagem && state.selvagem.nome) game.wild = `${state.selvagem.nome} Lv ${state.selvagem.level ?? '?'}`;
  }

  function onBattle(events) {
    if (!run) run = newRun();
    for (const e of events) {
      if (!e) continue;
      switch (e.k) {
        case 'morte':
          if (e.quem !== 'selvagem') break;
          run.kills++;
          run.xp += Number(e.xpTreinador ?? e.xp) || 0;
          run.gold += Number(e.ouro) || 0;
          break;
        case 'bola':
          run.throws++;
          if (e.shiny) run.shinies++;
          add(run.used.ball, e.ballId);
          break;
        case 'capturado':
          run.catches++;
          break;
        case 'cura':
          add(run.used.potion, e.itemId);
          add(run.potionsSinceBag, e.itemId);
          break;
        case 'revive':
          add(run.used.revive, e.itemId);
          add(run.potionsSinceBag, e.itemId);
          break;
        default:
      }
    }
  }

  function supplies(hours) {
    const rows = [];
    const row = (kind, id, left) => {
      const used = run.used[kind][id] || 0;
      const perHour = hours > 0 ? used / hours : 0;
      const info = kind === 'ball' ? ballCatalog.get(Number(id)) : names.get(Number(id));
      rows.push({
        kind,
        id: Number(id),
        name: info?.name || `#${id}`,
        left,
        used,
        // null until the rate means something (see sendStats).
        perHour: hours > 0 ? Math.round(perHour) : null,
        // Hours left at this pace (null while nothing was used yet).
        hoursLeft: perHour > 0 && left !== null ? left / perHour : null,
        cost: (info?.price || 0) * perHour,
      });
    };
    // Balls: the ones thrown this run, and any the game holds.
    const ballIds = new Set([...Object.keys(run.used.ball), ...Object.entries(game.balls).filter(([, n]) => n > 0).map(([id]) => id)]);
    for (const id of ballIds) if (run.used.ball[id] || game.balls[id] > 0) row('ball', id, game.balls[id] ?? null);
    // Potions and revives: stock from the last full bag, minus what was used since.
    for (const [kind, set] of [['potion', POTIONS], ['revive', REVIVES]]) {
      const ids = new Set([...Object.keys(run.used[kind]), ...Object.keys(game.items || {}).filter((id) => set.has(Number(id)) && game.items[id] > 0)]);
      for (const id of ids) {
        const bag = game.items ? (game.items[id] ?? 0) : null;
        row(kind, id, bag === null ? null : Math.max(0, bag - (run.potionsSinceBag[id] || 0)));
      }
    }
    return rows;
  }

  function sendStats() {
    if (!run) return;
    const now = Date.now();
    if (now - statsAt < 30000) return;
    const ms = now - run.start;
    // Rates only once there's enough time behind them to mean something.
    const hours = ms >= 5 * 60000 ? ms / 3600000 : 0;
    const per = (n) => (hours ? Math.round(n / hours) : null);
    const list = supplies(hours);
    const costPerHour = hours ? Math.round(list.reduce((t, s) => t + s.cost, 0)) : null;
    const goldPerHour = per(run.gold);
    const xpPerHour = per(run.xp);
    const eta = (xp, next, rate) => (rate && Number.isFinite(xp) && Number.isFinite(next) && next > xp ? (next - xp) / rate : null);
    const pkRate = hours && game.pokemon ? per(run.xp) : null; // the active Pokémon gets the same XP per kill
    const stats = {
      v: 2,
      hunt: game.hunt,
      minutes: Math.floor(ms / 60000),
      kills: run.kills,
      killsPerHour: per(run.kills),
      xpPerHour,
      goldPerHour,
      costPerHour,
      netPerHour: goldPerHour !== null && costPerHour !== null ? goldPerHour - costPerHour : null,
      throws: run.throws,
      catches: run.catches,
      shinies: run.shinies,
      trainer: game.level !== null ? { level: game.level, hoursLeft: eta(game.xp, game.xpNext, xpPerHour) } : null,
      pokemon: game.pokemon ? { name: game.pokemon.name, level: game.pokemon.level, hoursLeft: eta(game.pokemon.xp, game.pokemon.next, pkRate) } : null,
      wild: game.wild,
      supplies: list.map(({ cost, ...s }) => s),
    };
    const text = JSON.stringify(stats);
    if (text === statsSent) return;
    statsSent = text;
    statsAt = now;
    queue.push(`[multiview-stats] ${text}`);
    if (!busy) next();
  }
  setInterval(sendStats, 30000);

  function inspect(data) {
    if (typeof data !== 'string') return;
    const battle = data.includes('"batalha"');
    const catalog = data.includes('"catalogoBolas"');
    if (!battle && !catalog && !data.includes('"estado"')) return;
    let message;
    try { message = JSON.parse(data); } catch { return; }
    if (!message) return;
    if (catalog && Array.isArray(message.catalogoBolas)) {
      ballCatalog = new Map(message.catalogoBolas.map((x) => [Number(x.id), { name: x.nome || x.name, price: Number(x.priceGold) || 0 }]));
    }
    if (message.t === 'batalha' && Array.isArray(message.ev)) {
      onBattle(message.ev);
      return;
    }
    const state = message.estado;
    if (!state) return;
    onState(state);
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
