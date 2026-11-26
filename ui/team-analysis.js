// Team analysis engine for ui/team.html: how strong each Pokémon is and what it becomes when
// it evolves, with the game's own formulas, reimplemented from its public code
// (pokeidle.io/shared: ginasios.mjs, nota-pokemon.mjs, potencial-nota.mjs, golpes-especiais.mjs,
// golpes-planilhas.mjs, megas.mjs, evolucoes-ramificadas.mjs). The game's files arrive as text
// (team.rs) and are only parsed here with regular expressions: nothing from the game is run.
// The grade it computes matches the one the server sends for every Pokémon (checked against
// real lists), which is how the formulas are known to be right.
(() => {
  const KEYS = ['hp', 'atk', 'def', 'spAtk', 'spDef', 'speed'];
  const CAMPO = { hp: 'baseHp', atk: 'baseAtk', def: 'baseDef', spAtk: 'baseSpAtk', spDef: 'baseSpDef', speed: 'baseSpeed' };
  const EXPO_QUALIDADE = { hp: 0.95, atk: 0.8, def: 0.8, spAtk: 0.8, spDef: 0.8, speed: 0.95 };
  const BONUS_POTENCIA = [0, 0.05, 0.1, 0.25, 1];
  const multPotencia = (n) => 1 + (BONUS_POTENCIA[(Math.round(n) || 1) - 1] ?? 0);
  const IV_MIN = 6, IV_MAX = 192, Q_MIN = 0.8, Q_MAX = 1.8, POT_MIN = 1, POT_MAX = 5;
  const GOLPE_BASICO = { name: 'Basic', power: 30, cooldownMs: 2000, category: 'PHYSICAL' };
  const ESCALA = 1e7;
  const TIPOS = ['NORMAL', 'FIRE', 'WATER', 'GRASS', 'ELECTRIC', 'ICE', 'FIGHTING', 'POISON', 'GROUND',
    'FLYING', 'PSYCHIC', 'BUG', 'ROCK', 'GHOST', 'DRAGON', 'DARK', 'STEEL', 'FAIRY'];
  const num = (s) => Number(String(s).replace(/_/g, ''));

  // shared/cooldown-golpes.mjs
  function cooldownDoPoder(power) {
    const p = Number.isFinite(Number(power)) && power > 15 ? Number(power) : 15;
    const seg = p <= 100 ? 10 + ((p - 15) * 20) / 85 : 30 + ((p - 100) * 30) / 500;
    return Math.round(seg * 10) * 100;
  }

  // ---- parsers for the game's .mjs data (text only) ----
  // `export const GOLPES_X = { 248: [ ['Rock Blast', 'ROCK', 'PHYSICAL', 25], … ], … };`
  function parseSheets(text) {
    const sheets = new Map();
    for (const m of text.matchAll(/export const (GOLPES_\w+) = \{([\s\S]*?)\n\};/g)) {
      const table = new Map();
      for (const e of m[2].matchAll(/\n\s*(\d+):\s*\[([\s\S]*?)\n\s*\],/g)) {
        const rows = [];
        for (const r of e[2].matchAll(/\[\s*(['"])(.*?)\1\s*,\s*'(\w+)'\s*,\s*'(\w+)'\s*,\s*([\d_]+)\s*(?:,\s*(MEGA))?\s*\]/g)) {
          rows.push({ name: r[2], type: r[3], category: r[4], power: num(r[5]), mega: !!r[6] });
        }
        if (rows.length) table.set(Number(e[1]), rows);
      }
      sheets.set(m[1], table);
    }
    const list = (name) => {
      const b = text.match(new RegExp(`const ${name} = (?:new Set\\()?\\[([\\s\\S]*?)\\]`));
      return b ? [...b[1].matchAll(/GOLPES_\w+/g)].map((x) => x[0]) : [];
    };
    const megaOrder = text.match(/function golpesDaPlanilhaDaMega[\s\S]*?;\n/);
    return {
      sheets,
      order: list('TABELAS'),
      closed: new Set(list('TABELAS_FECHADAS')),
      megaOrder: megaOrder ? [...megaOrder[0].matchAll(/GOLPES_MEGAS\w+/g)].map((x) => x[0]) : [],
    };
  }
  function golpeDaLinha(r) {
    if (r.mega) return { name: r.name, type: r.type, category: r.category, power: r.power, cooldownMs: 30000, learnLevel: 1, categoriaFixa: true, cdFixo: true };
    return { name: r.name, type: r.type, category: r.category, power: r.power, cooldownMs: cooldownDoPoder(r.power), learnLevel: 1, ...(r.power >= 600 ? { categoriaFixa: true } : {}) };
  }
  function parseMegas(text) {
    const out = [];
    for (const blockName of ['MEGAS', 'MEGAS_SEGUNDAS']) {
      const b = text.match(new RegExp(`export const ${blockName} = \\[([\\s\\S]*?)\\n\\];`));
      if (!b) continue;
      for (const o of b[1].split(/\n  \{/).slice(1)) {
        const g = (re) => o.match(re)?.[1];
        const bases = o.match(/bases:\s*\{([^}]*)\}/)?.[1];
        if (!bases) continue;
        out.push({
          dex: num(g(/dex:\s*([\d_]+)/)),
          base: g(/\bbase:\s*([\d_]+)/) ? num(g(/\bbase:\s*([\d_]+)/)) : null,
          segunda: /segunda:\s*true/.test(o),
          nome: g(/nome:\s*'([^']*)'/),
          pedra: g(/pedra:\s*'([^']*)'/),
          tipos: [...(o.match(/tipos:\s*\[([^\]]*)\]/)?.[1] ?? '').matchAll(/'(\w+)'/g)].map((x) => x[1]),
          bases: Object.fromEntries([...bases.matchAll(/(\w+):\s*([\d_]+)/g)].map((x) => [x[1], num(x[2])])),
        });
      }
    }
    return out;
  }
  function parseBranches(text) {
    const b = text.match(/export const EVOLUCOES_RAMIFICADAS = \{([\s\S]*?)\n\};/);
    const map = new Map();
    for (const e of b?.[1].matchAll(/(\d+):\s*\[([^\]]*)\]/g) ?? []) {
      map.set(Number(e[1]), [...e[2].matchAll(/para:\s*([\d_]+)\s*,\s*nivel:\s*([\d_]+)/g)].map((x) => ({ para: num(x[1]), nivel: num(x[2]) })));
    }
    return map;
  }
  const parseJson = (text) => { try { return JSON.parse(text); } catch { return null; } };

  // ---- the species, built the way the game's client builds them ----
  function buildSpecies(files) {
    const species = new Map();
    for (const f of ['assets/creatures.json', 'assets/creatures-novos.json', 'assets/creatures-formas.json']) {
      for (const c of parseJson(files[f] ?? '')?.creatures ?? []) species.set(c.pokeId, { ...c, attacks: (c.attacks ?? []).map((a) => ({ ...a })) });
    }
    for (const o of parseJson(files['assets/creatures-audit-overrides.json'] ?? '')?.overrides ?? []) {
      const c = species.get(o.pokeId);
      if (!c) continue;
      for (const k of Object.values(CAMPO)) if (typeof o[k] === 'number') c[k] = o[k];
      if (Array.isArray(o.attacks)) c.attacks = o.attacks.map((a) => ({ ...a }));
    }
    const planilhas = files['shared/golpes-planilhas.mjs'] ? parseSheets(files['shared/golpes-planilhas.mjs']) : null;
    for (const name of planilhas?.order ?? []) {
      for (const [id, rows] of planilhas.sheets.get(name) ?? []) {
        const c = species.get(id);
        if (!c) continue;
        c.attacks = rows.map(golpeDaLinha);
        if (planilhas.closed.has(name)) c.golpesFechados = true; else delete c.golpesFechados;
      }
    }
    // shared/golpes-especiais.mjs: every final form gets a 600-power move of each of its types.
    for (const c of species.values()) {
      for (const a of c.attacks) if ((a.power ?? 0) >= 600 && !a.cdFixo) a.cooldownMs = 60000;
      if (c.evolvesToId || c.golpesFechados) continue;
      const cat = (c.baseAtk ?? 0) >= (c.baseSpAtk ?? 0) ? 'PHYSICAL' : 'SPECIAL';
      c.attacks = c.attacks.filter((a) => TIPOS.includes(a.type) && (a.power ?? 0) > 10);
      for (const a of c.attacks) if (a.power >= 600) { if (!a.categoriaFixa) a.category = cat; a.cooldownMs = 60000; }
      for (const t of new Set([c.type1, c.type2].filter((x) => TIPOS.includes(x)))) {
        if (!c.attacks.some((a) => a.power >= 600 && a.type === t)) c.attacks.push({ name: `600 ${t}`, power: 600, type: t, category: cat, cooldownMs: 60000, learnLevel: 1 });
      }
      const six = c.attacks.filter((a) => a.power >= 600);
      if (six.length === 1) c.attacks.push({ ...six[0], category: cat, categoriaFixa: undefined });
    }
    // Megas (shared/megas.mjs criarEspeciesMega): own bases and types; the base's moves plus a
    // Mega move (600 power, 30 s) unless a Mega sheet gives its moves.
    const megasOf = new Map();
    for (const m of parseMegas(files['shared/megas.mjs'] ?? '')) {
      const baseId = m.base ?? m.dex;
      const base = species.get(baseId);
      if (!base) continue;
      const pokeId = (m.segunda ? 9000 : 3000) + m.dex;
      let attacks = null;
      for (const name of planilhas?.megaOrder ?? []) {
        const rows = planilhas.sheets.get(name)?.get(pokeId);
        if (rows) { attacks = rows.map(golpeDaLinha); break; }
      }
      if (!attacks) {
        attacks = base.attacks.map((a) => ({ ...a }));
        if (TIPOS.includes(m.tipos[0])) {
          attacks.push({ name: `Mega ${m.tipos[0]}`, power: 600, type: m.tipos[0], category: m.bases.atk >= m.bases.spAtk ? 'PHYSICAL' : 'SPECIAL', cooldownMs: 30000, learnLevel: 1, cdFixo: true });
        }
      }
      const esp = {
        ...base, pokeId, name: m.nome, type1: m.tipos[0], type2: m.tipos[1] ?? null,
        baseHp: m.bases.hp, baseAtk: m.bases.atk, baseDef: m.bases.def, baseSpAtk: m.bases.spAtk, baseSpDef: m.bases.spDef, baseSpeed: m.bases.speed,
        evolvesToId: 0, attacks, mega: m,
      };
      species.set(pokeId, esp);
      if (!megasOf.has(baseId)) megasOf.set(baseId, []);
      megasOf.get(baseId).push(esp);
    }
    return { species, megasOf, branches: parseBranches(files['shared/evolucoes-ramificadas.mjs'] ?? '') };
  }

  // ---- formulas ----
  const tiposDe = (esp) => [esp.type1, esp.type2].filter(Boolean);
  const lendario = (esp) => /LEGEND|MYTH/i.test(esp.rarity || '');
  const basesDe = (esp, refino) => Object.fromEntries(KEYS.map((k) => [k, (esp[CAMPO[k]] ?? 80) + (Number(refino?.[k]) || 0)]));

  function statsNoNivel(pk, esp, nivelBruto) {
    const nivel = Math.max(1, Math.floor(nivelBruto));
    const mult = multPotencia(pk.potencia) * (pk.shiny ? 3 : 1) * (lendario(esp) ? 4 : 1);
    const q = Number(pk.quality) || 1;
    const b = basesDe(esp, pk.refino);
    return Object.fromEntries(KEYS.map((k) => [k, Math.round((b[k] + 2 * (pk.ivs?.[k] ?? 1)) * (nivel / 100) * (q ** EXPO_QUALIDADE[k] * mult))]));
  }
  function golpesDe(esp, nivel, pk) {
    const g = esp.attacks.filter((a) => (a.learnLevel ?? 1) <= nivel);
    if (pk.tmElemental) g.push({ name: `Z-Move ${pk.tmElemental}`, power: 2800, type: pk.tmElemental, category: pk.tmFisico ? 'PHYSICAL' : 'SPECIAL', cooldownMs: 60000 });
    return g;
  }
  const statDaCat = (g, st) => (g.category === 'SPECIAL' ? st.spAtk : st.atk);

  // The game's PvE power (ginasios.mjs ratingNoNivel / poderNoNivel): uses the higher of
  // ATK / SP.ATK whatever the move's category.
  function poderJogo(pk, esp, nivelBruto) {
    const st = statsNoNivel(pk, esp, nivelBruto);
    const nivel = Math.max(1, Math.floor(nivelBruto));
    const tipos = tiposDe(esp);
    let g = GOLPE_BASICO, best = -1;
    for (const a of golpesDe(esp, nivel, pk)) {
      const def = a.category === 'SPECIAL' ? st.spDef : st.def;
      const n = (((2 * nivel) / 5 + 2) * (a.power ?? 0) * (statDaCat(a, st) / Math.max(1, def)) / 50 + 2) * (tipos.includes(a.type) ? 1.5 : 1);
      if (n > best) { best = n; g = a; }
    }
    const atk = Math.max(st.atk, st.spAtk);
    const def = (st.def + st.spDef) / 2;
    const dps = (((2 * nivel) / 5 + 2) * (g.power ?? 30) * atk) / 50 / (Math.max(400, g.cooldownMs ?? 2000) / 1000);
    return Math.max(1, Math.round((dps * Math.max(24, Math.round(st.hp * 12)) * def) / ESCALA));
  }

  // Damage per second against a neutral target: the move's category picks ATK or SP.ATK, STAB
  // counts (ginasios.mjs metricasDeCombateContra, without the opponent's defense). The best
  // move overall, and the best of each category (`cats`; null = no move of that category).
  function danoReal(pk, esp, nivelBruto) {
    const st = statsNoNivel(pk, esp, nivelBruto);
    const nivel = Math.max(1, Math.floor(nivelBruto));
    const tipos = tiposDe(esp);
    const cats = { PHYSICAL: null, SPECIAL: null };
    for (const g of golpesDe(esp, nivel, pk)) {
      const dano = (((2 * nivel) / 5 + 2) * (g.power ?? 0) * statDaCat(g, st)) / 50 * (tipos.includes(g.type) ? 1.5 : 1);
      const dps = Math.round(dano / (Math.max(400, g.cooldownMs ?? 2000) / 1000));
      const cat = g.category === 'SPECIAL' ? 'SPECIAL' : 'PHYSICAL';
      if (!cats[cat] || dps > cats[cat].dps) cats[cat] = { dps, golpe: g };
    }
    const best = [cats.PHYSICAL, cats.SPECIAL].filter(Boolean).sort((a, b) => b.dps - a.dps)[0] ?? { dps: 0, golpe: GOLPE_BASICO };
    return { dps: best.dps, golpe: best.golpe, cats };
  }

  // The game's grade, "Score" (nota-pokemon.mjs notaPokemon), with the evolution floor: an
  // evolved form keeps at least its ancestor's grade + 0.1 per step.
  function somaNascimento({ ivs, q, pot, shiny, bases }) {
    const mult = multPotencia(pot) * (shiny ? 3 : 1);
    let s = 0;
    for (const k of KEYS) s += Math.round((bases[k] + 2 * (ivs[k] || 1)) * (q ** EXPO_QUALIDADE[k]) * mult);
    return s;
  }
  const ivsDeSoma = (soma) => { const med = Math.floor(soma / 6), rest = soma - med * 6; const o = {}; KEYS.forEach((k, i) => { o[k] = med + (i < rest ? 1 : 0); }); return o; };
  const REF = Object.fromEntries(KEYS.map((k) => [k, 80]));
  const REF_MIN = somaNascimento({ ivs: ivsDeSoma(IV_MIN), q: Q_MIN, pot: POT_MIN, shiny: false, bases: REF });
  const REF_MAX = somaNascimento({ ivs: ivsDeSoma(IV_MAX), q: Q_MAX, pot: POT_MAX, shiny: true, bases: REF });
  const score = (p) => {
    const s = Math.min(REF_MAX, Math.max(REF_MIN, somaNascimento(p)));
    return Math.min(1, Math.max(0, Math.round(10 * (Math.log(s) - Math.log(REF_MIN)) / (Math.log(REF_MAX) - Math.log(REF_MIN)) * 1000) / 1000 / 10));
  };
  function notaPara(pk, esp, chain) {
    const one = (e) => {
      const limBases = basesDe(e);
      const lo = score({ ivs: ivsDeSoma(IV_MIN), q: Q_MIN, pot: POT_MIN, shiny: false, bases: limBases });
      const hi = score({ ivs: ivsDeSoma(IV_MAX), q: Q_MAX, pot: POT_MAX, shiny: true, bases: limBases });
      const s = Math.min(hi, Math.max(lo, score({ ivs: pk.ivs || {}, q: Number(pk.quality) || 1, pot: pk.potencia || 1, shiny: !!pk.shiny, bases: basesDe(e, pk.refino) })));
      return hi <= lo ? 0 : Math.round(10 * (s - lo) / (hi - lo) * 1000) / 1000;
    };
    let n = one(esp);
    chain.forEach((anc, i) => { n = Math.max(n, Math.min(10, one(anc) + (chain.length - i) * 0.1)); });
    return Math.round(n * 1000) / 1000;
  }

  // ---- the engine ----
  function create(files) {
    const { species, megasOf, branches } = buildSpecies(files);
    const destinos = (esp) => {
      const out = [];
      for (const r of branches.get(esp.pokeId) ?? []) if (species.has(r.para)) out.push(species.get(r.para));
      const next = esp.evolvesToId && species.get(esp.evolvesToId);
      if (next && !out.includes(next)) out.push(next);
      return out;
    };
    const parents = new Map();
    for (const e of species.values()) for (const d of destinos(e)) if (!parents.has(d.pokeId)) parents.set(d.pokeId, e);
    const ancestrais = (esp) => {
      const out = [];
      let p = parents.get(esp.pokeId);
      while (p && !out.includes(p) && out.length < 10) { out.unshift(p); p = parents.get(p.pokeId); }
      return out;
    };
    // Every final form and Mega reachable (shared/potencial-nota.mjs formasFinaisDaEspecie).
    const formasFinais = (esp) => {
      if (esp.mega) return [];
      const out = new Map();
      const queue = [{ esp, chain: [] }];
      const seen = new Set([esp.pokeId]);
      while (queue.length && seen.size < 64) {
        const cur = queue.shift();
        for (const m of megasOf.get(cur.esp.pokeId) ?? []) out.set(m.pokeId, { esp: m, chain: [...cur.chain, cur.esp] });
        const next = destinos(cur.esp).filter((d) => !seen.has(d.pokeId));
        if (!next.length) { if (cur.esp !== esp) out.set(cur.esp.pokeId, cur); continue; }
        for (const d of next) { seen.add(d.pokeId); queue.push({ esp: d, chain: [...cur.chain, cur.esp] }); }
      }
      return [...out.values()];
    };

    // One Pokémon at level `nivel`: null when its species isn't in the data. `megas: false`
    // leaves Megas out of its forms and of its Score Potential.
    function analyse(pk, nivel, { megas = true } = {}) {
      const esp = species.get(pk.speciesId);
      if (!esp || !pk.ivs) return null;
      const anc = ancestrais(esp);
      const forms = formasFinais(esp)
        .filter((f) => megas || !f.esp.mega)
        .map((f) => ({
          name: f.esp.name,
          mega: f.esp.mega ? { stone: f.esp.mega.pedra } : null,
          types: tiposDe(f.esp),
          score: notaPara(pk, f.esp, [...anc, ...f.chain].filter((e, i, a) => a.indexOf(e) === i && e !== f.esp)),
          ...danoReal(pk, f.esp, nivel),
          power: poderJogo(pk, f.esp, nivel),
        }))
        .sort((a, b) => b.dps - a.dps);
      const score = Number.isFinite(pk.nota) ? pk.nota : notaPara(pk, esp, anc);
      const potential = Math.max(score, ...forms.map((f) => f.score));
      const real = danoReal(pk, esp, nivel);
      const bestNoMega = [{ name: esp.name, dps: real.dps, golpe: real.golpe, cats: real.cats, self: true }, ...forms.filter((f) => !f.mega)].sort((a, b) => b.dps - a.dps)[0];
      const bestMega = forms.filter((f) => f.mega).sort((a, b) => b.dps - a.dps)[0] ?? null;
      return {
        types: tiposDe(esp),
        score,
        potential,
        dps: real.dps,
        golpe: real.golpe,
        cats: real.cats,
        powerNow: poderJogo(pk, esp, pk.level),
        powerAt: poderJogo(pk, esp, nivel),
        forms,
        best: bestNoMega,
        bestMega,
      };
    }

    return { analyse, speciesCount: species.size };
  }

  // The game's colour steps (1 = grey … 5 = gold) for its P · Q · IV badges (app.js
  // degrauPorCortes with CORTES_QUALIDADE_PCT / CORTES_IV_PCT, nota-pokemon.mjs
  // percentilQualidade), and for one stat's IV (app.js classeIvStat).
  const BANDAS_QUALIDADE = [
    { min: 0.8, max: 0.9, chance: 5 }, { min: 0.9, max: 1.0, chance: 5 }, { min: 1.0, max: 1.1, chance: 34.03846 },
    { min: 1.1, max: 1.2, chance: 20 }, { min: 1.2, max: 1.3, chance: 10 }, { min: 1.3, max: 1.4, chance: 10 },
    { min: 1.4, max: 1.5, chance: 10 }, { min: 1.5, max: 1.6, chance: 5 }, { min: 1.7, max: 1.8, chance: 0.67308 },
    { min: 1.8, max: 1.8, chance: 0.28846 },
  ];
  function percentilQualidade(q) {
    const v = Math.min(Q_MAX, Math.max(Q_MIN, Number(q) || Q_MIN));
    let acc = 0;
    for (const b of BANDAS_QUALIDADE) {
      if (v < b.min) break;
      if (v >= b.max) { acc += b.chance; continue; }
      acc += b.chance * ((v - b.min) / (b.max - b.min));
      break;
    }
    return Math.min(1, Math.max(0, acc / 100));
  }
  const degrau = (v, cortes) => cortes.reduce((n, c) => (v >= c ? n + 1 : n), 1);
  const tiers = {
    potency: (p) => Math.min(5, Math.max(1, Math.round(Number(p) || 1))),
    quality: (q) => degrau(percentilQualidade(q), [0.44, 0.74, 0.94, 0.99]),
    ivTotal: (iv) => degrau((iv - IV_MIN) / (IV_MAX - IV_MIN), [0.5, 0.7, 0.85, 0.95]),
    ivStat: (v) => { const n = Math.round(Number(v) || 0); return n <= 5 ? 1 : n <= 10 ? 2 : n <= 15 ? 3 : n <= 24 ? 4 : 5; },
  };

  window.TeamAnalysis = { create, tiers };
})();
