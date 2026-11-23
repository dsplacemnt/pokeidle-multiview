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

  const game = { balls: {}, items: null, itemsAt: 0, xp: null, xpStart: null, xpNext: null, level: null, pokemon: null, hunt: null, wild: null, wildInfo: null, gold: null };

  // Sprites for the panel, cut from the game's own sprite sheets: the game's sprites.mjs module
  // (already loaded by the page, so this costs no download) knows where each Pokémon's frames
  // are. Each looktype is reported once: "[multiview-sprite] {"look":60006,"src":…,"x":…}".
  const spriteSent = new Set();
  let spriteModule = null;
  function sprite(look) {
    look = Number(look);
    if (!Number.isFinite(look) || look <= 1 || spriteSent.has(look)) return;
    spriteSent.add(look);
    (spriteModule ??= import('/sprites.mjs'))
      .then((m) => m.carregarOutfit(look))
      .then((s) => {
        if (!s) return;
        const frames = (s.byDir && (s.byDir[3] || Object.values(s.byDir)[0])) || [];
        const f = frames[0];
        const img = f && s.imagens && s.imagens[f.pagina];
        if (!img || !img.src) return;
        queue.push(`[multiview-sprite] ${JSON.stringify({ look, src: img.src, x: f.x, y: f.y, w: f.w, h: f.h })}`);
        if (!busy) next();
      })
      .catch(() => {});
  }

  // Shinies met while this page is open (newest first): seen, caught or fled. Kept across hunt
  // changes; reported with the hunt analysis, right away when something happens.
  const shinies = [];
  const SHINY_KEEP = 10;
  function shinyEvent(battle, info, result) {
    let s = shinies.find((x) => x.battle === battle && battle !== undefined);
    if (!s) {
      if (result !== 'seen' && !info) return;
      s = { battle, name: info?.name || '?', level: info?.level ?? null, look: info?.look ?? null, result: 'seen', at: Date.now() };
      shinies.unshift(s);
      shinies.length = Math.min(shinies.length, SHINY_KEEP);
    }
    if (result !== 'seen') s.result = result;
    if (s.look) sprite(s.look);
    statsAt = 0; // send now
    setTimeout(sendStats, 500);
  }
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
    if (Number.isFinite(Number(state.xpNivel))) game.xpStart = Number(state.xpNivel);
    if (Number.isFinite(Number(state.level))) game.level = Number(state.level);
    if (Number.isFinite(Number(state.gold))) game.gold = Number(state.gold);
    const pk = Array.isArray(state.pkMud) ? state.pkMud.find((p) => p && p.slot === 0) || state.pkMud[0] : null;
    if (pk && pk.nome) {
      // A shiny shows with its own look when it has one.
      const look = Number(pk.shiny && pk.lookShiny ? pk.lookShiny : pk.looktype) || null;
      game.pokemon = { name: pk.nome, level: pk.level, xp: Number(pk.xp), start: Number(pk.xpNivel), next: Number(pk.xpProximo), look, shiny: !!pk.shiny };
      sprite(look);
    }
    const w = state.selvagem;
    if (w && w.nome) {
      game.wild = `${w.nome} Lv ${w.level ?? '?'}`;
      game.wildInfo = { name: w.nome, level: w.level ?? null, look: Number(w.looktype) || null, shiny: !!w.shiny, battle: w.slot };
      sprite(game.wildInfo.look);
      if (w.shiny) shinyEvent(w.slot, game.wildInfo, 'seen');
    }
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
        case 'spawn':
          if (e.shiny) shinyEvent(e.slot, { name: e.nome, level: e.level, look: Number(e.looktype) || null }, 'seen');
          break;
        case 'bola':
          run.throws++;
          if (e.shiny) {
            run.shinies++;
            const w = game.wildInfo && game.wildInfo.battle === e.slot ? game.wildInfo : null;
            shinyEvent(e.slot, w, 'seen');
          }
          add(run.used.ball, e.ballId);
          break;
        case 'capturado': {
          run.catches++;
          // Which battle: the event's own, or the wild Pokémon being fought.
          const battle = e.slot ?? game.wildInfo?.battle;
          if (e.shiny || (battle !== undefined && shinies.some((s) => s.battle === battle))) {
            shinyEvent(battle, e.nome ? { name: e.nome, level: e.level, look: Number(e.looktype) || null } : game.wildInfo, 'caught');
          }
          break;
        }
        case 'fugiu':
          if (e.shiny) shinyEvent(e.slot ?? game.wildInfo?.battle, { name: e.nome, level: e.level, look: game.wildInfo?.look ?? null }, 'fled');
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
    // How far into the current level (0–100), from the level's start and end XP.
    const pct = (xp, start, next) => (Number.isFinite(xp) && Number.isFinite(start) && Number.isFinite(next) && next > start
      ? Math.max(0, Math.min(100, Math.floor(((xp - start) / (next - start)) * 1000) / 10)) : null);
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
      trainer: game.level !== null ? { level: game.level, pct: pct(game.xp, game.xpStart, game.xpNext), hoursLeft: eta(game.xp, game.xpNext, xpPerHour) } : null,
      pokemon: game.pokemon ? {
        name: game.pokemon.name, level: game.pokemon.level, look: game.pokemon.look, shiny: game.pokemon.shiny,
        pct: pct(game.pokemon.xp, game.pokemon.start, game.pokemon.next), hoursLeft: eta(game.pokemon.xp, game.pokemon.next, pkRate),
      } : null,
      wild: game.wild,
      foe: game.wildInfo ? { name: game.wildInfo.name, level: game.wildInfo.level, look: game.wildInfo.look, shiny: game.wildInfo.shiny } : null,
      shinyList: shinies.map(({ battle, ...s }) => s),
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

  // The game's connection and session, for the "this account was dropped" warning:
  // "[multiview-conn] {"open":false,"code":4001,"session":true}". open = the game's socket is
  // connected (null until it first tries), code = how it last closed (4001: the account logged
  // in somewhere else), session = the page holds a game session at all (only whether the
  // "sessao" entry exists; its value is never read out). Sent when one of them changes.
  const conn = { open: null, code: null, session: null };
  let connSent = '';
  let current = null; // the newest game socket; older ones closing don't count
  function reportConn() {
    const text = JSON.stringify(conn);
    if (text === connSent) return;
    connSent = text;
    queue.push(`[multiview-conn] ${text}`);
    if (!busy) next();
  }
  function checkSession() {
    let has = null;
    try { has = !!localStorage.getItem('sessao'); } catch {}
    if (has !== conn.session) {
      conn.session = has;
      reportConn();
    }
  }
  setTimeout(checkSession, 8000);
  setInterval(checkSession, 15000);
  // A page that never got connected after a minute counts as disconnected.
  setTimeout(() => {
    if (conn.open === null) {
      conn.open = false;
      reportConn();
    }
  }, 60000);

  class WatchedWebSocket extends Native {
    constructor(...args) {
      super(...args);
      current = this;
      this.addEventListener('message', (event) => {
        try { inspect(event.data); } catch {}
      });
      this.addEventListener('open', () => {
        if (current !== this) return;
        conn.open = true;
        conn.code = null;
        reportConn();
      });
      this.addEventListener('close', (event) => {
        if (current !== this) return;
        conn.open = false;
        conn.code = Number(event.code) || null;
        reportConn();
      });
    }
  }
  WatchedWebSocket.__multiview = true;
  window.WebSocket = WatchedWebSocket;
})();
