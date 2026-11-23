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
  const POTIONS = new Set([200, 201, 202, 203, 204, 207, 70070]);
  const REVIVES = new Set([205, 206]);
  const names = new Map(); // item/ball id → { name, price }
  let ballCatalog = new Map(); // ball id → { name, price }
  fetch('/assets/items.json')
    .then((r) => r.json())
    .then((j) => {
      for (const i of j.items || j || []) {
        names.set(i.id, { name: i.name, price: Number(i.npcPrice) || 0, icon: FIXED_ITEMS[i.id]?.icon || itemIcon(i.icon) });
      }
    })
    .catch(() => {});
  // Icons, as the game itself draws them: items.json paths live under /assets/site/assets/
  // ("/assets/markitems/x.png" → /assets/site/assets/markitems/x.png, a bare name is in
  // items/), and balls use the game's own small table (other balls show as a Poké Ball).
  function itemIcon(icon) {
    if (!icon || typeof icon !== 'string') return null;
    const path = icon.startsWith('/assets/site/') || icon.startsWith('/img/') ? icon
      : icon.startsWith('/assets/') ? `/assets/site${icon}`
      : icon.startsWith('/') ? icon
      : `/assets/site/assets/items/${icon}`;
    // Just the path: the report goes through the page title, which has a length limit; the
    // panel puts the game's address in front.
    return path;
  }
  const BALL_ICONS = {
    1: '/assets/site/assets/ui/ball-poke.png',
    2: '/assets/site/assets/ui/ball-great.png',
    3: '/img/ball-super.png',
    4: '/assets/site/assets/ui/ball-ultra.png',
    5: '/img/ball-beast.png',
  };
  const ballIcon = (id) => BALL_ICONS[id] || BALL_ICONS[1];
  // Items the game gives a fixed icon (some aren't in items.json at all, like the Golden
  // Potion); the game's own table.
  const FIXED_ITEMS = {
    30: { icon: '/assets/site/assets/items/dark_gem.png' },
    63: { icon: '/assets/site/assets/items/goggles.png' },
    47: { icon: '/assets/site/assets/items/farfetchd_stick.png' },
    5041: { icon: '/assets/site/assets/items/farfetchd_stick.png' },
    70070: { icon: '/img/itens/golden-potion.png', name: 'Golden Potion' },
  };
  for (const [id, f] of Object.entries(FIXED_ITEMS)) names.set(Number(id), { name: f.name || `#${id}`, price: 0, icon: f.icon });
  // The game's own items, sent when it connects ("itensNossos"): names and icons the public
  // list doesn't have. The fixed icons above still win, as in the game.
  function ownItems(list) {
    for (const i of list) {
      const id = Number(i && i.id);
      if (!Number.isFinite(id)) continue;
      const before = names.get(id) || {};
      const name = i.nome || i.name;
      const icon = FIXED_ITEMS[id]?.icon || (i.icon ? (i.icon.startsWith('/') ? i.icon : itemIcon(i.icon)) : before.icon) || null;
      names.set(id, {
        name: name || before.name || `#${id}`,
        price: Number(i.npcPrice) || before.price || 0,
        icon,
      });
    }
  }

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
      // Whether the Prey Pokémon is in this hunt changes with it.
      if (huntSpecies.size) reportPrey();
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
        icon: kind === 'ball' ? ballIcon(Number(id)) : info?.icon || null,
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
      // The page's JavaScript memory in MB (Chromium only), for src/heavy.rs.
      heap: performance.memory ? Math.round(performance.memory.usedJSHeapSize / 1048576) : null,
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

  // The game's own Battery Saver ("Turn the scene on" to leave it): the scene stops drawing and
  // the page tells the server. The app's economy mode turns it on in every hidden account and
  // back off when it ends, through the game's own buttons, and only undoes what it did: a
  // Battery Saver the player turned on stays (the game remembers it in "cfg-economia"; the
  // app's own mark is "multiview-eco").
  window.__multiviewEco = async (on) => {
    const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
    let prefs;
    try { prefs = await import('/preferencias.mjs'); } catch { return 'no prefs'; }
    const mine = () => { try { return localStorage.getItem('multiview-eco') === '1'; } catch { return false; } };
    const mark = (v) => { try { if (v) localStorage.setItem('multiview-eco', '1'); else localStorage.removeItem('multiview-eco'); } catch {} };
    // The page may still be starting: try for up to a minute.
    for (let i = 0; i < 30; i++) {
      const isOn = prefs.economiaLigado();
      if (on) {
        if (isOn) return mine() ? 'on (ours)' : 'on (the player\'s)';
        document.querySelector('#hunt-eco')?.click();
        await sleep(300);
        if (prefs.economiaLigado()) { mark(true); return 'turned on'; }
      } else {
        if (!mine()) return 'left as is';
        if (!isOn) { mark(false); return 'already off'; }
        document.querySelector('#eco-sair')?.click();
        await sleep(300);
        if (!prefs.economiaLigado()) { mark(false); return 'turned off'; }
      }
      await sleep(2000);
    }
    return 'not ready';
  };

  // The daily Prey (renews at 18:00 Brasília): the account's cards, as the game keeps them in
  // its automation ({dia, cartelas:[{lista, pk, bonus, est}]}, sent when the page connects) and
  // as "preyRoletou"/"preyEscolheu" change them. Reported with the chosen Pokémon's name, sprite
  // and whether the account is hunting it: "[multiview-prey] {…}".
  const prey = { dia: null, cards: [null, null], unlocked2: false };
  let huntSpecies = new Map(); // hunt slug → Set of pokeIds
  let speciesHunt = new Map(); // pokeId → { name, hunt, level } (its lowest-level hunt)
  let looks = null; // pokeId → looktype (from the game's species lists, loaded once)
  let preySent = '';
  let serverOffset = 0; // server clock − PC clock
  const preyDay = () => new Date(Date.now() + serverOffset + 3 * 3600000).toISOString().slice(0, 10);
  function loadLooks() {
    if (looks) return Promise.resolve(looks);
    const files = ['/assets/creatures.json', '/assets/creatures-novos.json', '/assets/creatures-outland-novos.json'];
    return Promise.all(files.map((f) => fetch(f).then((r) => (r.ok ? r.json() : null)).catch(() => null))).then((all) => {
      looks = new Map();
      for (const j of all) {
        const list = Array.isArray(j) ? j : j && (j.creatures || Object.values(j)[0]);
        for (const c of Array.isArray(list) ? list : []) if (c && c.pokeId != null && c.looktype) looks.set(Number(c.pokeId), Number(c.looktype));
      }
      return looks;
    });
  }
  function onHunts(hunts) {
    huntSpecies = new Map();
    speciesHunt = new Map();
    for (const h of hunts) {
      if (!h || !h.slug) continue;
      const ids = new Set();
      for (const e of h.especies || []) {
        const id = Number(e.pokeId);
        ids.add(id);
        const best = speciesHunt.get(id);
        if (!best || h.nivel < best.level) speciesHunt.set(id, { name: e.nome, hunt: h.nome, level: Number(h.nivel) || 0 });
      }
      huntSpecies.set(h.slug, ids);
    }
    reportPrey();
  }
  function onPrey(p, cartela2) {
    if (cartela2 !== undefined) prey.unlocked2 = Number(cartela2) > 0;
    if (!p || typeof p !== 'object') return reportPrey();
    prey.dia = typeof p.dia === 'string' ? p.dia : null;
    const cards = Array.isArray(p.cartelas) ? p.cartelas : [];
    prey.cards = [0, 1].map((i) => cards[i] || null);
    reportPrey();
  }
  function onPreyCard(i, c) {
    i = Number(i);
    if (!(i >= 0 && i < 2) || !c) return;
    // A roll or a pick is always about today.
    if (prey.dia !== preyDay()) prey.cards = [null, null];
    prey.dia = preyDay();
    prey.cards[i] = c;
    reportPrey();
  }
  async function reportPrey() {
    await loadLooks();
    const cards = prey.cards.map((c, i) => {
      if (i === 1 && !prey.unlocked2) return null;
      if (!c) return { rolled: false };
      const pk = c.pk == null ? null : Number(c.pk);
      const info = pk == null ? null : speciesHunt.get(pk);
      const look = pk == null ? null : looks.get(pk) || null;
      if (look) sprite(look);
      return {
        rolled: Array.isArray(c.lista) && c.lista.length > 0,
        pk,
        name: info?.name || (pk == null ? null : `#${pk}`),
        look,
        bonus: c.bonus || null,
        stars: Number(c.est) || null,
        // Hunting it right now (it spawns in the account's current hunt).
        hunting: pk == null || !game.hunt || !huntSpecies.size ? null : !!huntSpecies.get(game.hunt)?.has(pk),
        hunt: info?.hunt || null,
        huntLevel: info?.level ?? null,
      };
    });
    const out = { day: prey.dia, today: preyDay(), cards };
    const text = JSON.stringify(out);
    if (text === preySent) return;
    preySent = text;
    queue.push(`[multiview-prey] ${text}`);
    if (!busy) next();
  }

  function inspect(data) {
    if (typeof data !== 'string') return;
    const battle = data.includes('"batalha"');
    const catalog = data.includes('"catalogoBolas"');
    const preyMsg = data.includes('"prey');
    if (!battle && !catalog && !preyMsg && !data.includes('"estado"')) return;
    let message;
    try { message = JSON.parse(data); } catch { return; }
    if (!message) return;
    if (Array.isArray(message.hunts)) onHunts(message.hunts);
    if (message.t === 'preyRoletou' || message.t === 'preyEscolheu') onPreyCard(message.cartela, message.c);
    if (message.t === 'preyDesbloqueou') { prey.unlocked2 = true; reportPrey(); }
    if (message.estado && Number(message.estado.servidorAgora)) serverOffset = Number(message.estado.servidorAgora) - Date.now();
    // The welcome always counts (no "prey" there = nothing rolled yet); the full snapshots sent
    // later leave it out, so only a message that has it changes it.
    if (message.estado && message.estado.automation && (message.t === 'welcome' || 'prey' in message.estado.automation || 'preyCartela2' in message.estado.automation)) {
      onPrey(message.estado.automation.prey, message.estado.automation.preyCartela2);
    }
    if (Array.isArray(message.itensNossos)) ownItems(message.itensNossos);
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
