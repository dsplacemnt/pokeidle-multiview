// What changed in each version, newest first, shown by whatsnew.html after an update and from
// ⚙ → About. Add an entry with every release (pt, en and es; a missing language falls back to
// English).
window.CHANGELOG = [
  {
    version: '0.19.0',
    date: '2026-11-23',
    notes: {
      pt: [
        'Alertas no Discord (⚙ → Discord): cole o webhook de um canal e receba shinies, conta que caiu do jogo, live que o app não conseguiu resolver, voto do IdleHub liberado e um relatório diário na hora que quiser.',
        'Nova opção para PCs com pouca memória (⚙ → Geral → Memória): um navegador para todas as contas, cerca de 1 GB a menos de RAM. Vem desligada; ao ligar, as contas precisam entrar de novo (os logins atuais ficam guardados e voltam se desligar).',
        'Golden Potion e outros itens do jogo agora aparecem com nome e ícone na análise da hunt.',
        'O contorno do slot em tela cheia encosta no painel de estatísticas, e o painel não tem mais o botão GRID.',
      ],
      en: [
        'Discord alerts (⚙ → Discord): paste a channel\'s webhook and get shinies, an account dropped from the game, a stream the app couldn\'t fix, IdleHub votes ready and a daily report at the hour you pick.',
        'New option for PCs short on memory (⚙ → General → Memory): one browser for all accounts, about 1 GB less RAM. Off by default; turning it on means logging in again (the current logins are kept and come back if you turn it off).',
        'Golden Potion and other game items now show with their name and icon in the hunt analysis.',
        'The fullscreen slot\'s outline now meets the stats panel, and the panel no longer has the GRID button.',
      ],
      es: [
        'Alertas en Discord (⚙ → Discord): pega el webhook de un canal y recibe shinies, cuentas que se cayeron del juego, streams que la app no pudo arreglar, votos de IdleHub disponibles y un informe diario a la hora que elijas.',
        'Nueva opción para PCs con poca memoria (⚙ → General → Memoria): un navegador para todas las cuentas, cerca de 1 GB menos de RAM. Viene desactivada; al activarla hay que iniciar sesión de nuevo (las sesiones actuales se guardan y vuelven si la desactivas).',
        'Golden Potion y otros ítems del juego ahora aparecen con nombre e ícono en el análisis de la hunt.',
        'El contorno del slot en pantalla completa ahora llega al panel de estadísticas, y el panel ya no tiene el botón GRID.',
      ],
    },
  },
  {
    version: '0.18.1',
    date: '2026-11-23',
    notes: {
      pt: [
        'Ícones do próprio jogo na análise da hunt: bolas, poções e revives nos suprimentos, e moeda, XP, poção e bola nos números.',
        'O botão de atualizar aparece sozinho: o app procura versão nova a cada 30 minutos (antes, a cada 6 horas) e, com a barra de cima escondida, o ⬆ aparece no cabeçalho do canto.',
      ],
      en: [
        'The game\'s own icons in the hunt analysis: balls, potions and revives in the supplies, and coin, XP, potion and ball on the numbers.',
        'The update button shows up by itself: the app looks for a new version every 30 minutes (it used to be every 6 hours) and, with the top bar hidden, the ⬆ shows on the corner header.',
      ],
      es: [
        'Íconos del propio juego en el análisis de la hunt: balls, pociones y revivir en los suministros, y moneda, XP, poción y ball en los números.',
        'El botón de actualizar aparece solo: la app busca una versión nueva cada 30 minutos (antes, cada 6 horas) y, con la barra superior oculta, el ⬆ aparece en la cabecera de la esquina.',
      ],
    },
  },
  {
    version: '0.18.0',
    date: '2026-11-23',
    notes: {
      pt: [
        'Aviso quando o jogo de uma conta cai: logada em outro lugar, sem conexão por mais de 1 minuto ou sem login no jogo. Aparece como ⚠ Jogo no cabeçalho do slot, no modo economia e no painel, com botão para recarregar.',
        'Análise da hunt com as sprites do próprio jogo: seu Pokémon contra o selvagem.',
        'Barras de experiência do Treinador e do Pokémon, com o % do nível e o tempo até o próximo.',
        'Shinies da sessão: cada shiny visto, capturado ou que fugiu, com sprite e horário.',
      ],
      en: [
        'A warning when an account\'s game drops out: logged in somewhere else, disconnected for over a minute, or not logged in to the game. It shows as ⚠ Game on the slot header, in economy mode and in the panel, with a button to reload.',
        'Hunt analysis with the game\'s own sprites: your Pokémon vs the wild one.',
        'Experience bars for the Trainer and the Pokémon, with the % of the level and the time to the next one.',
        'Shinies this session: each shiny seen, caught or fled, with its sprite and time.',
      ],
      es: [
        'Aviso cuando el juego de una cuenta se cae: sesión iniciada en otro lugar, sin conexión por más de 1 minuto o sin sesión en el juego. Aparece como ⚠ Juego en la cabecera del slot, en el modo ahorro y en el panel, con un botón para recargar.',
        'Análisis de la hunt con los sprites del propio juego: tu Pokémon contra el salvaje.',
        'Barras de experiencia del Entrenador y del Pokémon, con el % del nivel y el tiempo hasta el siguiente.',
        'Shinies de la sesión: cada shiny visto, capturado o que huyó, con su sprite y hora.',
      ],
    },
  },
  {
    version: '0.17.1',
    date: '2026-11-23',
    notes: {
      pt: [
        'Menos memória: as lives do mesmo site em cada conta agora dividem um processo do navegador. No teste com 4 contas e 40 lives, o app caiu de ~12,2 GB para ~7,2 GB de RAM, com a CPU igual.',
      ],
      en: [
        'Less memory: each account\'s streams from the same site now share one browser process. In a test with 4 accounts and 40 streams, the app went from ~12.2 GB to ~7.2 GB of RAM, with the same CPU.',
      ],
      es: [
        'Menos memoria: los streams del mismo sitio de cada cuenta ahora comparten un proceso del navegador. En una prueba con 4 cuentas y 40 streams, la app bajó de ~12,2 GB a ~7,2 GB de RAM, con la misma CPU.',
      ],
    },
  },
  {
    version: '0.17.0',
    date: '2026-11-23',
    notes: {
      pt: [
        'Versão experimental para Linux (.deb e AppImage), com jogos, lives, modo economia, logins e atalhos. No Linux a extensão /hunt não está disponível.',
      ],
      en: [
        'Experimental Linux version (.deb and AppImage), with games, streams, economy mode, logins and shortcuts. The /hunt extension isn\'t available on Linux.',
      ],
      es: [
        'Versión experimental para Linux (.deb y AppImage), con juegos, streams, modo ahorro, inicios de sesión y atajos. En Linux la extensión /hunt no está disponible.',
      ],
    },
  },
  {
    version: '0.16.0',
    date: '2026-11-22',
    notes: {
      pt: [
        'Modo economia (botão ECO ou Ctrl+E): esconde todas as contas, que continuam rodando, e mostra uma tela leve com o que precisa de você e os bônus de cada conta. No teste com 4 contas, a CPU do app caiu de ~50% para ~22%.',
        'Medidor de CPU e RAM do app na barra de cima, com jogos e lives incluídos.',
        'Lado direito da barra organizado em seções: extensões, recursos, idiomas e opções; o apoio virou só o ♥.',
        'Contas sem login na Twitch ou na Kick não abrem mais lives (que não rendiam nada): aparece um botão Entrar na barra do slot, no painel, nas Settings e no modo economia, e as lives começam assim que você entra.',
        'Lives em segundo plano pedem memória baixa ao WebView2: cerca de 1 GB a menos com 32 lives.',
      ],
      en: [
        'Economy mode (ECO button or Ctrl+E): hides every account, which keeps running, and shows a light screen with what needs you and each account\'s bonuses. In a test with 4 accounts, the app\'s CPU went from ~50% to ~22%.',
        'CPU and RAM meter for the whole app on the top bar, games and streams included.',
        'Right side of the top bar arranged in sections: extensions, resources, languages and options; support is now just the ♥.',
        'Accounts not logged in on Twitch or Kick no longer open streams (they earned nothing): a Log in button shows on the slot bar, the side panel, Settings and economy mode, and the streams start as soon as you log in.',
        'Background streams ask WebView2 for low memory: about 1 GB less with 32 streams.',
      ],
      es: [
        'Modo ahorro (botón ECO o Ctrl+E): oculta todas las cuentas, que siguen funcionando, y muestra una pantalla ligera con lo que te necesita y los bonos de cada cuenta. En una prueba con 4 cuentas, la CPU de la app bajó de ~50% a ~22%.',
        'Medidor de CPU y RAM de toda la app en la barra superior, con juegos y streams incluidos.',
        'Lado derecho de la barra organizado en secciones: extensiones, recursos, idiomas y opciones; el apoyo ahora es solo el ♥.',
        'Las cuentas sin sesión en Twitch o Kick ya no abren streams (no daban nada): aparece un botón Entrar en la barra del slot, el panel, Ajustes y el modo ahorro, y los streams empiezan en cuanto entras.',
        'Los streams en segundo plano piden memoria baja a WebView2: cerca de 1 GB menos con 32 streams.',
      ],
    },
  },
  {
    version: '0.15.1',
    date: '2026-11-22',
    notes: {
      pt: ['"Entendi" nesta janela e Esc nas Settings agora fecham a janela (antes ela ficava branca e aberta).'],
      en: ['"Got it" in this window and Esc in Settings now close the window (it used to stay open and blank).'],
      es: ['"Entendido" en esta ventana y Esc en Ajustes ahora cierran la ventana (antes se quedaba abierta y en blanco).'],
    },
  },
  {
    version: '0.15.0',
    date: '2026-11-22',
    notes: {
      pt: [
        'Atalhos de teclado configuráveis em ⚙ → Atalhos, com os que já existiam (Ctrl+1…4).',
        'Novos atalhos: Ctrl+G alterna entre a grade e o último slot em tela cheia, Ctrl+Shift+R recarrega todas as páginas do jogo e Ctrl+T abre um novo slot.',
        'Os atalhos funcionam mesmo com o foco dentro de um jogo ou de uma live.',
        'Esta janela de novidades: aparece depois de cada atualização e também em ⚙ → Sobre.',
      ],
      en: [
        'Configurable keyboard shortcuts in ⚙ → Shortcuts, including the existing ones (Ctrl+1…4).',
        'New shortcuts: Ctrl+G switches between the grid and the last fullscreen slot, Ctrl+Shift+R reloads every game page and Ctrl+T opens a new slot.',
        'Shortcuts work even with the focus inside a game or stream.',
        'This "What\'s new" window: it shows after each update, and from ⚙ → About.',
      ],
      es: [
        'Atajos de teclado configurables en ⚙ → Atajos, incluidos los que ya existían (Ctrl+1…4).',
        'Nuevos atajos: Ctrl+G alterna entre la cuadrícula y el último slot en pantalla completa, Ctrl+Shift+R recarga todas las páginas del juego y Ctrl+T abre un nuevo slot.',
        'Los atajos funcionan incluso con el foco dentro de un juego o stream.',
        'Esta ventana de novedades: aparece tras cada actualización y también en ⚙ → Acerca de.',
      ],
    },
  },
  {
    version: '0.14.0',
    date: '2026-11-22',
    notes: {
      pt: [
        'O ♡ de "não segue" some em segundos quando você segue o canal, na Twitch e na Kick.',
        'O painel lateral também mostra o ♡ ao lado da live.',
        'Nas listas de prioridade (⚙ → Canais): ♡ quando alguma conta não segue o canal e ? quando ainda não abriu, com as contas ao passar o mouse.',
      ],
      en: [
        'The "not following" ♡ goes away within seconds when you follow the channel, on Twitch and Kick.',
        'The side panel shows the ♡ next to the stream too.',
        'In the priority lists (⚙ → Channels): ♡ when some account doesn\'t follow the channel and ? when it hasn\'t opened it yet, with the accounts on hover.',
      ],
      es: [
        'El ♡ de "no lo sigue" desaparece en segundos cuando sigues el canal, en Twitch y Kick.',
        'El panel lateral también muestra el ♡ junto al stream.',
        'En las listas de prioridad (⚙ → Canales): ♡ cuando alguna cuenta no sigue el canal y ? cuando aún no lo abrió, con las cuentas al pasar el ratón.',
      ],
    },
  },
  {
    version: '0.13.0',
    date: '2026-11-22',
    notes: {
      pt: [
        'Resgates na Twitch mais confiáveis: passa pela tela de boas-vindas dos pontos e espera o botão de confirmar ser liberado.',
        'Dica ♡ na pílula da live quando a conta não segue o canal (o app nunca segue sozinho).',
        'Bônus no cabeçalho do slot como XP, CAP e LURE.',
      ],
      en: [
        'More reliable Twitch redeems: steps past the points welcome screen and waits for the confirm button to turn on.',
        '♡ tip on a stream\'s pill when the account doesn\'t follow the channel (the app never follows by itself).',
        'Bonuses in the slot header read XP, CAP and LURE.',
      ],
      es: [
        'Canjes en Twitch más fiables: pasa la pantalla de bienvenida de los puntos y espera a que el botón de confirmar se active.',
        'Consejo ♡ en la píldora del stream cuando la cuenta no sigue el canal (la app nunca sigue por sí sola).',
        'Bonos en la cabecera del slot como XP, CAP y LURE.',
      ],
    },
  },
  {
    version: '0.12.1',
    date: '2026-11-21',
    notes: {
      pt: ['As lives continuam tocando com o app minimizado ou na bandeja (a Twitch não trava mais pausada).'],
      en: ['Streams keep playing while the app is minimized or in the tray (Twitch no longer gets stuck paused).'],
      es: ['Los streams siguen reproduciéndose con la app minimizada o en la bandeja (Twitch ya no se queda en pausa).'],
    },
  },
  {
    version: '0.12.0',
    date: '2026-11-21',
    notes: {
      pt: [
        'Metas de resgate: mantém cada bônus acima das horas escolhidas, na ordem da lista.',
        'Settings organizadas em abas.',
        'Troca mais rápida de lives que ficaram offline.',
      ],
      en: [
        'Redeem goals: keeps each bonus above the hours you choose, top of the list first.',
        'Settings organized in tabs.',
        'Faster swap of streams that went offline.',
      ],
      es: [
        'Metas de canje: mantiene cada bono por encima de las horas elegidas, en el orden de la lista.',
        'Ajustes organizados en pestañas.',
        'Cambio más rápido de streams que se quedaron offline.',
      ],
    },
  },
];
