// What changed in each version, newest first, shown by whatsnew.html after an update and from
// ⚙ → About. Add an entry with every release (pt, en and es; a missing language falls back to
// English).
window.CHANGELOG = [
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
