// The app's two languages: Portuguese (default) and English. Every local page loads this
// first. The pages are written in English; `I18N.t(english, vars)` gives the text in the
// current language, elements marked `data-i18n` / `data-i18n-title` / `data-i18n-placeholder`
// are translated in place, and `I18N.text()` translates messages that come from the app or
// from the stream pages (warnings, activity log, errors). The choice is saved by the app (the
// flags on the top bar or ⚙) and sent to every open page, which switches at once.
(() => {
  // English → Portuguese. Keys must match the English text exactly ({name} = filled in).
  const PT = {
    // Top bar
    '♥ Support': '♥ Apoiar',
    'Support the dev: Pix and Discord': 'Apoiar o dev: Pix e Discord',
    'Hide this bar to give the slots more room (▾ in the same corner brings it back)': 'Esconder esta barra para dar mais espaço aos slots (▾ no mesmo canto traz de volta)',
    "Grid: show all slots (click a slot's name to show it fullscreen instead)": 'Grade: mostra todos os slots (clique no nome de um slot para vê-lo em tela cheia)',
    'Settings': 'Configurações',
    'Back to the grid': 'Voltar para a grade',
    'Downloading {pct}%': 'Baixando {pct}%',
    'Version {v} is out: click to update (the app closes, installs it and opens again)': 'Saiu a versão {v}: clique para atualizar (o app fecha, instala e abre de novo)',
    'Version {v} is out: click to open its download page': 'Saiu a versão {v}: clique para abrir a página de download',
    'Update now?': 'Atualizar agora?',
    'Check for updates': 'Procurar atualizações',
    'Version {v} is out: use ⬆ on the top bar': 'Saiu a versão {v}: use o ⬆ na barra de cima',
    "Couldn't check for updates": 'Não deu para procurar atualizações',
    'Up to date': 'Atualizado',
    'Show {name} fullscreen': 'Mostrar {name} em tela cheia',
    'Add a slot (another account, up to 4)': 'Adicionar um slot (outra conta, até 4)',
    '"/hunt for PokeIdle" extension loaded': 'Extensão "/hunt for PokeIdle" carregada',
    '"/hunt for PokeIdle" not found in any installed browser': '"/hunt for PokeIdle" não encontrada em nenhum navegador instalado',

    // Slot header
    "Left: this slot's controls · Right: its Twitch/Kick streams": 'Esquerda: controles deste slot · Direita: suas streams da Twitch/Kick',
    'Scroll the streams left': 'Rolar as streams para a esquerda',
    'Scroll the streams right': 'Rolar as streams para a direita',
    '{platform} (not on a channel)': '{platform} (fora de um canal)',
    'Show this slot fullscreen': 'Mostrar este slot em tela cheia',
    'double-click to rename': 'clique duplo para renomear',
    'Back to the game ({name} keeps running muted)': 'Voltar para o jogo ({name} continua rodando sem som)',
    '{count} {name} streams running in the background — click to show one (the pills pick which)': '{count} streams da {name} rodando em segundo plano — clique para mostrar uma (as pílulas escolhem qual)',
    '1 {name} stream running in the background — click to show it': '1 stream da {name} rodando em segundo plano — clique para mostrá-la',
    'Open {name} in this slot (log in here)': 'Abrir a {name} neste slot (faça login aqui)',
    'Add a Twitch or Kick channel to this slot': 'Adicionar um canal da Twitch ou da Kick a este slot',
    'Channel on {name}': 'Canal na {name}',
    'channel name or link': 'nome do canal ou link',
    'Stop {name} ({platform})': 'Parar {name} ({platform})',
    'Show {name} ({platform}) in this slot': 'Mostrar {name} ({platform}) neste slot',
    'Stop {name}': 'Parar {name}',
    '{bonus} from {platform}: {left} left': '{bonus} da {platform}: faltam {left}',
    'no bonus': 'sem bônus',
    'No PokeIdle bonus from {platform} running': 'Nenhum bônus do PokeIdle da {platform} ativo',
    "The game hasn't reported this account's {platform} bonuses yet": 'O jogo ainda não informou os bônus da {platform} desta conta',
    'Bonuses from {platform}': 'Bônus da {platform}',
    'Vote': 'Votar',
    '{n}/{of} toward the next diamond': '{n}/{of} para o próximo diamante',
    "IdleHub vote for {nick} is ready: opens the vote page in a window with this slot's session (sign in and confirm there)": 'O voto no IdleHub de {nick} está liberado: abre a página de votação numa janela com a sessão deste slot (entre e confirme lá)',
    'Next IdleHub vote for {nick} in {left} (click to open the vote page anyway)': 'Próximo voto no IdleHub de {nick} em {left} (clique para abrir a página de votação mesmo assim)',
    '{n} vote': '{n} voto',
    '{n} votes': '{n} votos',
    '{n} diamond': '{n} diamante',
    '{n} diamonds': '{n} diamantes',
    'Unmute this game': 'Ativar o som deste jogo',
    'Mute this game': 'Silenciar este jogo',
    'Show the top bar (+ SLOT, GRID, settings…)': 'Mostrar a barra de cima (+ SLOT, GRID, configurações…)',
    'Hide the stats panel': 'Esconder o painel de estatísticas',
    'Show the stats panel': 'Mostrar o painel de estatísticas',
    'Reload the game': 'Recarregar o jogo',
    'Close?': 'Fechar?',
    'Close slot (keeps its login)': 'Fechar o slot (mantém o login)',
    '{platform} streams': 'Streams da {platform}',
    'No streams · + adds a channel': 'Sem streams · + adiciona um canal',

    // Stats panel
    'Hide this panel (▤ in the slot header brings it back)': 'Esconder este painel (▤ no topo do slot traz de volta)',
    'Bonuses': 'Bônus',
    'linked in game': 'vinculada no jogo',
    'not linked in game': 'não vinculada no jogo',
    'waiting for the game': 'aguardando o jogo',
    '{bonus} from {platform}: ends at {time}': '{bonus} da {platform}: termina às {time}',
    '{bonus}: not running': '{bonus}: inativo',
    'Kick redeem target:': 'Resgate na Kick:',
    'none (⚙ to pick)': 'nenhum (escolha no ⚙)',
    'Streams': 'Streams',
    '{n} open': '{n} abertas',
    'No streams open in this slot': 'Nenhuma stream aberta neste slot',
    '{platform} (no channel)': '{platform} (sem canal)',
    'on screen': 'na tela',
    'Channel points': 'Pontos de canal',
    'None seen yet (they show once a stream reports them)': 'Nenhum visto ainda (aparecem quando uma stream informar)',
    'Daily chores': 'Tarefas diárias',
    'IdleHub vote': 'Voto no IdleHub',
    'Reminder off for this account (⚙)': 'Lembrete desligado para esta conta (⚙)',
    'off': 'desligado',
    'Checking…': 'Verificando…',
    '{n}/{of} to next': '{n}/{of} para o próximo',
    'Vote now': 'Votar agora',
    'in {left}': 'em {left}',
    "Opens the vote page in a window with this slot's session": 'Abre a página de votação numa janela com a sessão deste slot',
    'Kick Daily Reward': 'Daily Reward da Kick',
    'Auto-claim off (⚙)': 'Resgate automático desligado (⚙)',
    'Claimed today (UTC)': 'Resgatado hoje (UTC)',
    'done': 'feito',
    'Claimed on the next manager pass': 'Resgatado na próxima verificação',
    'pending': 'pendente',
    'Game session': 'Sessão do jogo',
    'since the game loaded': 'desde que o jogo abriu',
    'Gold': 'Gold',
    'Gold this session': 'Gold nesta sessão',
    'Gold per hour': 'Gold por hora',
    'XP': 'XP',
    'XP this session': 'XP nesta sessão',
    'XP per hour': 'XP por hora',
    'Balls left': 'Pokébolas',
    'Fighting': 'Batalhando',
    'Hunt analysis': 'Análise da hunt',
    'Supplies': 'Suprimentos',
    'Waiting for the game to show the bag (a few minutes)': 'Aguardando o jogo mostrar a mochila (alguns minutos)',
    '{n}/h': '{n}/h',
    'unused': 'sem uso',
    '{n} used': '{n} usadas',
    '{name}: {left} left, {rate} per hour — lasts about {time}': '{name}: restam {left}, {rate} por hora — dura uns {time}',
    '{name}: {left} left, not used in this hunt yet': '{name}: restam {left}, ainda sem uso nesta hunt',
    'left': 'restam',
    'per hour': 'por hora',
    'lasts': 'dura',
    'Balls': 'Pokébolas',
    'Potions': 'Poções',
    'Revives': 'Revives',
    'Kills per hour': 'Abates por hora',
    'Supplies per hour': 'Gasto por hora',
    'Net gold per hour': 'Lucro por hora',
    'Catches ({rate}%)': 'Capturas ({rate}%)',
    'Catches': 'Capturas',
    'Next level': 'Próximo nível',
    'in {time}': 'em {time}',
    'Trainer': 'Treinador',
    'Rates show after 5 minutes of hunting.': 'As taxas aparecem após 5 minutos de hunt.',
    '{n} shiny seen': '{n} shiny visto',
    'Fighting {wild}': 'Batalhando contra {wild}',
    'Activity': 'Atividade',
    'last {n}': 'últimas {n}',
    'Nothing yet — the app logs swaps, redeems, reloads and claims here': 'Nada ainda — o app registra aqui trocas, resgates, recarregamentos e coletas',

    // Support window
    'Support the dev': 'Apoiar o dev',
    'If PokeIdle Multiview helps you and you can chip in, any amount helps keep it going. Thank you!': 'Se o PokeIdle Multiview te ajuda e você puder contribuir, qualquer valor ajuda a manter o desenvolvimento. Obrigado!',
    "Scan it with your bank's app (Pix, Brazil)": 'Escaneie com o app do seu banco (Pix)',
    'Pix copy and paste': 'Pix copia e cola',
    'Copy': 'Copiar',
    'Copied ✓': 'Copiado ✓',
    'Invite friends to PokeIdle (free)': 'Convide amigos para o PokeIdle (grátis)',
    "Costs nothing: people who don't play yet can sign up with this invite link.": 'Sem gastar nada: quem ainda não joga pode criar a conta por este link de convite.',
    'Open': 'Abrir',
    'Open in the browser': 'Abrir no navegador',
    'Star on GitHub (free)': 'Estrela no GitHub (grátis)',
    'A ⭐ on the repository helps more players find the app.': 'Uma ⭐ no repositório ajuda mais gente a encontrar o app.',
    '⭐ Give a star': '⭐ Dar estrela',
    'Discord (ideas, bugs, questions)': 'Discord (sugestões, bugs, dúvidas)',
    'Pix': 'Pix',
    "Scan the QR code with your bank's app, or copy the Pix code and paste it there.": 'Escaneie o QR Code com o app do seu banco, ou copie o código Pix e cole lá.',
    'Copy Pix code': 'Copiar código Pix',
    'Open profile': 'Abrir perfil',
    'Copy username': 'Copiar usuário',
    "Ideas, bugs, questions: the developer's Discord": 'Sugestões, bugs, dúvidas: o Discord do dev',
    'channel name': 'nome do canal',

    // Settings
    'Language': 'Idioma',
    'App language': 'Idioma do app',
    'Also on the top bar: the flags next to ⚙.': 'Também na barra de cima: as bandeiras ao lado do ⚙.',
    'Window': 'Janela',
    'Close to the tray': 'Fechar para a bandeja',
    '× keeps the app and its streams running near the clock. Quit from the tray icon.': 'O × mantém o app e as streams rodando perto do relógio. Para sair, use o ícone da bandeja.',
    'Minimize to the tray': 'Minimizar para a bandeja',
    'Minimizing hides the app from the taskbar too.': 'Minimizar também esconde o app da barra de tarefas.',
    'Layout': 'Layout',
    'Smooth transitions': 'Transições suaves',
    'Slots slide into place when you go fullscreen, reset the grid or hide the top bar. Off by default: changes are instant.': 'Os slots deslizam até o lugar ao entrar em tela cheia, resetar a grade ou esconder a barra de cima. Desligado por padrão: as mudanças são instantâneas.',
    'Reset the grid': 'Resetar a grade',
    'Leaves fullscreen and makes every slot the same size again.': 'Sai da tela cheia e deixa todos os slots do mesmo tamanho de novo.',
    'Reset': 'Resetar',
    'Keep the PC awake while streams run': 'Manter o PC acordado com streams rodando',
    "Windows won't go to sleep and cut the streams. The screen can still turn off.": 'O Windows não entra em suspensão e não corta as streams. A tela ainda pode desligar.',
    'Lighter Kick streams in the background': 'Streams da Kick mais leves em segundo plano',
    'Hidden Kick pages drop their chat and sidebars and play at the lowest quality, to save memory and CPU. Turn off if a Kick channel stops earning points.': 'Páginas da Kick escondidas ficam sem chat e barras laterais e tocam na menor qualidade, para poupar memória e CPU. Desligue se um canal da Kick parar de render pontos.',
    'Game': 'Jogo',
    'Reload all game pages': 'Recarregar todas as páginas do jogo',
    "Useful to pick up the game's daily updates.": 'Útil para pegar as atualizações diárias do jogo.',
    'Never': 'Nunca',
    'Every 15 min': 'A cada 15 min',
    'Every 30 min': 'A cada 30 min',
    'Every hour': 'A cada hora',
    'Every 3 hours': 'A cada 3 horas',
    'Every 6 hours': 'A cada 6 horas',
    'Every 12 hours': 'A cada 12 horas',
    'Every 24 hours': 'A cada 24 horas',
    'About': 'Sobre',
    'Ideas, bugs, questions: Discord': 'Sugestões, bugs, dúvidas: Discord',
    '♥ Support the dev': '♥ Apoiar o dev',
    '⭐ Star on GitHub': '⭐ Estrela no GitHub',
    'Opens the project page: a star there helps more players find the app': 'Abre a página do projeto: uma estrela lá ajuda mais gente a encontrar o app',
    'Invite friends to PokeIdle': 'Convide amigos para o PokeIdle',
    'A free way to support: new players can sign up with this referral link.': 'Um jeito grátis de apoiar: novos jogadores podem criar a conta por este link de convite.',
    'Copy invite link': 'Copiar link de convite',
    'Copy invite': 'Copiar convite',
    'Open invite': 'Abrir convite',
    'Free ways to help': 'Ajude de graça',
    "Invite friends who don't play yet with your invite link, or give the app a ⭐ on GitHub so more players find it.": 'Convide quem ainda não joga com o link de convite, ou dê uma ⭐ no GitHub para mais gente encontrar o app.',
    "Every account keeps live channels of this list open, as many as you choose: <b>the ones where it has the most points first</b> (closest to its next bonus), this list's order deciding between equal ones. Every ~15 minutes each stream briefly loads its full page to grab the <b>bonus chest</b> (+50) and redeem the account's bonus. Empty list = you manage Twitch streams by hand.":
      'Cada conta mantém abertos canais ao vivo desta lista, quantos você escolher: <b>primeiro os que ela tem mais pontos</b> (mais perto do próximo bônus), e a ordem da lista desempata. A cada ~15 minutos cada stream abre a página completa rapidinho para pegar o <b>baú de bônus</b> (+50) e resgatar o bônus da conta. Lista vazia = você gerencia as streams da Twitch na mão.',
    'Twitch lives per account': 'Lives da Twitch por conta',
    'No limit on Twitch: choose 1 to 8.': 'A Twitch não tem limite: escolha de 1 a 8.',
    'Keep updated from the game': 'Manter atualizada pelo jogo',
    'channel name or twitch.tv link': 'nome do canal ou link da twitch.tv',
    'Add': 'Adicionar',
    'Kick pays channel points for 2 lives at a time per account, so every account keeps 2 live channels of this list open and closes its other Kick streams: <b>the ones where it has the most points first</b>, this list\'s order deciding between equal ones. When one goes offline, the next takes its place (checked every 2 minutes). Empty list = you manage Kick streams by hand.':
      'A Kick paga pontos de canal em 2 lives por vez por conta, então cada conta mantém 2 canais ao vivo desta lista abertos e fecha suas outras streams da Kick: <b>primeiro os que ela tem mais pontos</b>, e a ordem da lista desempata. Quando um fica offline, o próximo assume (verificado a cada 2 minutos). Lista vazia = você gerencia as streams da Kick na mão.',
    'Kick only pays channel points for 2 lives at a time per account': 'A Kick só paga pontos de canal em 2 lives por vez por conta',
    'Kick lives per account': 'Lives da Kick por conta',
    'Fixed: Kick pays points for 2 at a time.': 'Fixo: a Kick paga pontos em 2 por vez.',
    "Claim Kick's Daily Reward": 'Resgatar o Daily Reward da Kick',
    "Once a day per account, from Kick's gift button (not channel points). Off by default.": 'Uma vez por dia por conta, pelo botão de presente da Kick (não usa pontos de canal). Desligado por padrão.',
    'channel name or kick.com link': 'nome do canal ou link da kick.com',
    'Voting on IdleHub gives the account a diamond every 2 votes (one vote a day counts per nick). Tick the accounts to be reminded: their header shows when the vote is ready, and clicking it opens the vote page in a window with that slot\'s session, where you sign in and confirm. The app never votes by itself. IdleHub asks for "one person, one account" and may void votes it finds suspicious.':
      'Votar no IdleHub dá à conta um diamante a cada 2 votos (conta um voto por dia por nick). Marque as contas para receber o lembrete: o topo do slot mostra quando o voto está liberado, e clicar abre a página de votação numa janela com a sessão daquele slot, onde você entra e confirma. O app nunca vota sozinho. O IdleHub pede "uma pessoa, uma conta" e pode anular votos que achar suspeitos.',
    'Bonus per account': 'Bônus por conta',
    "Redeems the chosen PokeIdle bonus with the account's channel points as soon as they cover it, on its live Kick and Twitch channels (hours add up in the game). The account must have Kick and Twitch linked in the game (<i>Bônus na Kick</i>, <i>Bônus Twitch</i>), or the points are given back.":
      'Resgata o bônus do PokeIdle escolhido com os pontos de canal da conta assim que eles cobrirem o custo, nos canais ao vivo da Kick e da Twitch (as horas somam no jogo). A conta precisa ter Kick e Twitch vinculadas no jogo (<i>Bônus na Kick</i>, <i>Bônus Twitch</i>), senão os pontos são devolvidos.',
    'Saved': 'Salvo',
    'Grid reset': 'Grade resetada',
    'Not checked yet': 'Ainda não verificado',
    'Live': 'Ao vivo',
    'Offline': 'Offline',
    'Open in 1 account right now': 'Aberto em 1 conta agora',
    'Open in {n} accounts right now': 'Aberto em {n} contas agora',
    'Higher priority': 'Subir prioridade',
    'Lower priority': 'Descer prioridade',
    'Remove': 'Remover',
    'No channels: {platform} streams are managed by hand.': 'Sem canais: as streams da {platform} são gerenciadas na mão.',
    'Asking the game for the vote status…': 'Perguntando ao jogo o status do voto…',
    "Couldn't read the vote status: {error}": 'Não deu para ler o status do voto: {error}',
    'vote ready now': 'voto liberado agora',
    'next vote in {left}': 'próximo voto em {left}',
    '{n}/{of} toward the next': '{n}/{of} para o próximo',
    'No slots open.': 'Nenhum slot aberto.',
    '1 stream needs attention': '1 stream precisa de atenção',
    '{n} streams need attention': '{n} streams precisam de atenção',
    'New partner channels of the game are added at the end of the list (the game has {n} on {platform}).': 'Novos canais parceiros do jogo entram no fim da lista (o jogo tem {n} na {platform}).',
    'New partner channels of the game are added at the end of the list (waiting for the game to announce them).': 'Novos canais parceiros do jogo entram no fim da lista (aguardando o jogo informar).',
    'Nothing': 'Nada',
    '{platform} not linked': '{platform} não vinculada',
    'Link {platform} in the game, or redeemed points are given back': 'Vincule a {platform} no jogo, senão os pontos resgatados são devolvidos',
    '{platform} is linked in the game': 'A {platform} está vinculada no jogo',
    'Not reported by the game yet': 'O jogo ainda não informou',
    'Points: not seen yet': 'Pontos: ainda não vistos',
    'Not a new {platform} channel name': 'Não é um canal novo da {platform}',
    '1 channel added from the game': '1 canal adicionado pelo jogo',
    '{n} channels added from the game': '{n} canais adicionados pelo jogo',
  };

  // Messages from the app and the stream pages (warnings, activity log, errors), translated
  // piece by piece, in order.
  const PHRASES = [
    // Errors
    [/^All 4 slots are open$/, 'Os 4 slots já estão abertos'],
    [/^No such slot$/, 'Slot não encontrado'],
    [/^"(.*)" is not a Twitch or Kick channel$/, '"$1" não é um canal da Twitch ou da Kick'],
    [/^Slot (\d+) already runs (\d+) (\w+) streams; close one first$/, 'O slot $1 já tem $2 streams da $3; feche uma antes'],
    [/^No vote status yet$/, 'Ainda sem status do voto'],
    [/This game page is not logged in/, 'Esta página do jogo não está logada'],
    // Stream warnings
    [/^Logged out: /, 'Deslogado: '],
    [/^Stalled: (.*)\. Reloaded, still not playing\.$/, 'Travada: $1. Recarregada, ainda sem tocar.'],
    [/No live player on the channel page/, 'Sem player ao vivo na página do canal'],
    [/Kick shows no channel points here: is this account logged in\?/, 'A Kick não mostra pontos de canal aqui: esta conta está logada?'],
    [/The Twitch player says the channel is offline/, 'O player da Twitch diz que o canal está offline'],
    [/The video is paused/, 'O vídeo está pausado'],
    [/The video stopped at (\d+) s/, 'O vídeo parou em $1 s'],
    // Activity log
    [/^(\w+) closed: /, '$1 fechou: '],
    [/^(\w+) opened: /, '$1 abriu: '],
    [/: stalled, reloaded$/, ': travou, recarregada'],
    [/: playing normally again$/, ': tocando normalmente de novo'],
    [/^Kick Daily Reward claimed$/, 'Daily Reward da Kick resgatado'],
    [/^Kick Daily Reward already claimed today$/, 'Daily Reward da Kick já resgatado hoje'],
    [/^Kick Daily Reward: /, 'Daily Reward da Kick: '],
    [/^IdleHub vote counted \(\+1 diamond\)$/, 'Voto no IdleHub contado (+1 diamante)'],
    [/^IdleHub vote counted$/, 'Voto no IdleHub contado'],
    [/Kick redeem: /, 'Resgate na Kick: '],
    [/chest claimed/g, 'baú resgatado'],
    [/not enough points/g, 'pontos insuficientes'],
    [/redeem button disabled for /g, 'botão de resgate desativado para '],
    [/no redeem button for /g, 'sem botão de resgate para '],
    [/\bredeemed /g, 'resgatou '],
    [/no such reward on this channel|this channel has no such reward/g, 'este canal não tem essa recompensa'],
    [/no channel points here \(offline or logged out\?\)/g, 'sem pontos de canal aqui (offline ou deslogado?)'],
    [/no points button \(logged out\?\)/g, 'sem botão de pontos (deslogado?)'],
    [/\bnot live\b/g, 'não está ao vivo'],
    [/nothing to do/g, 'nada a fazer'],
    [/\berror: /g, 'erro: '],
  ];

  const norm = (s) => String(s).replace(/\s+/g, ' ').trim();
  const fill = (s, vars) => (vars ? s.replace(/\{(\w+)\}/g, (m, k) => (k in vars ? String(vars[k]) : m)) : s);
  const listeners = [];
  let lang = 'pt';
  try { lang = localStorage.getItem('multiview-lang') === 'en' ? 'en' : 'pt'; } catch {}

  function t(en, vars) {
    return fill(lang === 'pt' && PT[en] !== undefined ? PT[en] : en, vars);
  }

  function text(message) {
    if (lang !== 'pt' || !message) return message;
    let out = String(message);
    for (const [re, to] of PHRASES) out = out.replace(re, to);
    return out;
  }

  function apply(root = document) {
    document.documentElement.lang = lang === 'pt' ? 'pt-BR' : 'en';
    for (const el of root.querySelectorAll('[data-i18n]')) {
      if (el.dataset.en === undefined) el.dataset.en = norm(el.innerHTML);
      el.innerHTML = t(el.dataset.en);
    }
    for (const [attr, key] of [['title', 'enTitle'], ['placeholder', 'enPlaceholder']]) {
      for (const el of root.querySelectorAll(`[data-i18n-${attr}]`)) {
        if (el.dataset[key] === undefined) el.dataset[key] = norm(el.getAttribute(attr) || '');
        el.setAttribute(attr, t(el.dataset[key]));
      }
    }
  }

  function set(next) {
    if (next !== 'pt' && next !== 'en') return;
    const changed = next !== lang;
    lang = next;
    try { localStorage.setItem('multiview-lang', lang); } catch {}
    apply();
    if (changed) for (const f of listeners) f(lang);
  }

  window.I18N = {
    get lang() { return lang; },
    PT,
    t,
    text,
    apply,
    set,
    onChange: (f) => listeners.push(f),
  };

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', () => apply());
  else apply();
  // The saved choice comes from the app; a change made anywhere reaches every page.
  const tauri = window.__TAURI__;
  if (tauri) {
    tauri.core.invoke('get_language').then(set).catch(() => {});
    tauri.event.listen('language', (e) => set(e.payload));
  }
})();
