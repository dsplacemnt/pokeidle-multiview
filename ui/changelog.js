// What changed in each version, newest first, shown by whatsnew.html after an update and from
// ⚙ → About. Add an entry with every release (pt, en and es; a missing language falls back to
// English).
window.CHANGELOG = [
  {
    version: '0.22.1',
    date: '2026-11-24',
    notes: {
      pt: [
        'Corrigido: em redes de empresa, faculdade ou com antivírus que inspeciona o HTTPS, o app não conseguia ver quais canais da Twitch estavam ao vivo, e as lives nunca eram abertas nem trocadas. Agora ele confia nos mesmos certificados que o Windows e o navegador.',
        'Se o app não conseguir checar as lives da Twitch ou da Kick, agora ele avisa: um ⚠ vermelho na barra de cima (clique para checar de novo), um aviso no modo economia e uma linha no registro de atividades de cada conta. Antes, nada aparecia e as lives simplesmente paravam.',
      ],
      en: [
        'Fixed: on company or school networks, or with an antivirus that inspects HTTPS, the app couldn\x27t tell which Twitch channels were live, so their streams were never opened or swapped. It now trusts the same certificates as Windows and the browser.',
        'If the app can\x27t check the Twitch or Kick lives, it now says so: a red ⚠ in the top bar (click to check again), a notice in economy mode and a line in each account\x27s activity log. Before, nothing showed up and the streams just stopped.',
      ],
      es: [
        'Corregido: en redes de empresa o de universidad, o con un antivirus que inspecciona el HTTPS, la app no podía ver qué canales de Twitch estaban en vivo, y los streams nunca se abrían ni se cambiaban. Ahora confía en los mismos certificados que Windows y el navegador.',
        'Si la app no puede comprobar los streams de Twitch o de Kick, ahora avisa: un ⚠ rojo en la barra de arriba (haz clic para comprobar de nuevo), un aviso en el modo ahorro y una línea en el registro de actividad de cada cuenta. Antes no aparecía nada y los streams simplemente se detenían.',
      ],
    },
  },
  {
    version: '0.22.0',
    date: '2026-11-24',
    notes: {
      pt: [
        'O app agora guarda um registro do que faz (lives abertas e fechadas, recargas, erros e travamentos) em um arquivo no seu PC, que nunca é enviado a lugar nenhum. Se algo der errado, vá em ⚙ → Sobre → Copiar diagnóstico e cole no Discord ou numa issue do GitHub: ele traz as versões do app, do Windows e do WebView2 e as últimas linhas do registro, sem senhas, cookies, tokens ou o seu nome de usuário. O botão Abrir pasta de logs mostra o arquivo completo.',
      ],
      en: [
        'The app now keeps a log of what it does (streams opened and closed, reloads, errors and crashes) in a file on your PC that is never sent anywhere. If something goes wrong, go to ⚙ → About → Copy diagnostic and paste it on Discord or in a GitHub issue: it has the app, Windows and WebView2 versions and the last lines of the log, with no passwords, cookies, tokens or your user name. Open the logs folder shows the whole file.',
      ],
      es: [
        'La app ahora guarda un registro de lo que hace (streams abiertos y cerrados, recargas, errores y cierres inesperados) en un archivo en tu PC que nunca se envía a ningún lado. Si algo sale mal, ve a ⚙ → Acerca de → Copiar diagnóstico y pégalo en Discord o en un issue de GitHub: trae las versiones de la app, de Windows y de WebView2 y las últimas líneas del registro, sin contraseñas, cookies, tokens ni tu nombre de usuario. Abrir carpeta de registros muestra el archivo completo.',
      ],
    },
  },
  {
    version: '0.21.1',
    date: '2026-11-24',
    notes: {
      pt: [
        'Quando o Windows atualiza o WebView2 (o navegador que roda as contas, normalmente de madrugada), o app se reinicia sozinho para usar a versão nova. Antes, depois dessa atualização, as lives que caíam não eram reabertas e as Settings davam erro ao abrir. As lives abertas são salvas, e se o app estava na bandeja ou minimizado, ele volta do mesmo jeito, sem aparecer na tela.',
      ],
      en: [
        'When Windows updates WebView2 (the browser that runs the accounts, usually at night), the app now restarts by itself to use the new version. Until now, after such an update, streams that ended weren\'t replaced and Settings failed to open. The open streams are saved, and if the app was in the tray or minimized, it comes back the same way, without showing up on screen.',
      ],
      es: [
        'Cuando Windows actualiza WebView2 (el navegador que ejecuta las cuentas, normalmente de madrugada), la app ahora se reinicia sola para usar la versión nueva. Antes, tras esa actualización, los streams que terminaban no se reemplazaban y Ajustes daba error al abrir. Los streams abiertos se guardan, y si la app estaba en la bandeja o minimizada, vuelve igual, sin aparecer en pantalla.',
      ],
    },
  },
  {
    version: '0.21.0',
    date: '2026-11-23',
    notes: {
      pt: [
        'Modo economia mais leve: ao esconder as contas, o app também liga o Battery Saver do próprio jogo em cada uma (e desliga quando você sai). Se você já usava o Battery Saver em alguma conta, ela continua como estava.',
        'Páginas que incham em sessões longas recarregam sozinhas: quando uma live ou o jogo passa do dobro da memória normal, a página recarrega (nunca a live na tela nem o jogo em tela cheia).',
        'Prey do dia: botão "Hoje não" para esconder o aviso até as 18:00 quando nenhum dos 9 vale a pena.',
        'Cada release agora traz o arquivo SHA256SUMS.txt para conferir os downloads, e o repositório ganhou o SECURITY.md, que explica o que o app faz com as suas contas.',
      ],
      en: [
        'Lighter economy mode: while it hides the accounts, the app also turns on the game\'s own Battery Saver in each one (and turns it off when you leave). An account where you already used the Battery Saver stays as it was.',
        'Pages that swell over long sessions reload by themselves: when a stream or the game passes twice its normal memory, the page reloads (never the stream on screen or the fullscreen game).',
        'Daily Prey: a "Not today" button hides the reminder until 18:00 when none of the 9 is worth it.',
        'Every release now has a SHA256SUMS.txt to check the downloads, and the repository has a SECURITY.md that explains what the app does with your accounts.',
      ],
      es: [
        'Modo ahorro más ligero: al ocultar las cuentas, la app también activa el Battery Saver del propio juego en cada una (y lo desactiva al salir). Si ya usabas el Battery Saver en alguna cuenta, sigue como estaba.',
        'Las páginas que se inflan en sesiones largas se recargan solas: cuando un stream o el juego pasa del doble de su memoria normal, la página se recarga (nunca el stream en pantalla ni el juego en pantalla completa).',
        'Prey del día: botón "Hoy no" para ocultar el aviso hasta las 18:00 cuando ninguno de los 9 vale la pena.',
        'Cada versión ahora trae el archivo SHA256SUMS.txt para comprobar las descargas, y el repositorio tiene un SECURITY.md que explica qué hace la app con tus cuentas.',
      ],
    },
  },
  {
    version: '0.20.0',
    date: '2026-11-23',
    notes: {
      pt: [
        'Prey do dia: o app avisa quando a roletada grátis do Prey está liberada (renova às 18:00 de Brasília) ou quando a conta roletou mas não escolheu o Pokémon. O aviso aparece no cabeçalho do slot, no modo economia, nas Tarefas diárias e, se quiser, no Discord.',
        'Card "Prey do dia" no painel de estatísticas: o Pokémon escolhido de cada cartela, o bônus com as estrelas e o efeito, e se a conta está caçando ele agora (o bônus só vale na hunt onde ele aparece).',
      ],
      en: [
        'Daily Prey: the app tells you when the free Prey roll is ready (it renews at 18:00 Brasília) or when an account rolled but didn\'t pick the Pokémon. It shows on the slot header, in economy mode, in Daily chores and, if you want, on Discord.',
        '"Daily Prey" card in the stats panel: each card\'s picked Pokémon, its bonus with the stars and the effect, and whether the account is hunting it right now (the bonus only works in a hunt where it spawns).',
      ],
      es: [
        'Prey del día: la app avisa cuando la tirada gratis del Prey está disponible (se renueva a las 18:00 de Brasilia) o cuando una cuenta giró pero no eligió el Pokémon. El aviso aparece en la cabecera del slot, en el modo ahorro, en Tareas diarias y, si quieres, en Discord.',
        'Tarjeta "Prey del día" en el panel de estadísticas: el Pokémon elegido de cada cartilla, su bono con las estrellas y el efecto, y si la cuenta lo está cazando ahora (el bono solo sirve en una hunt donde aparece).',
      ],
    },
  },
  {
    version: '0.19.1',
    date: '2026-11-23',
    notes: {
      pt: [
        'O app não volta mais sozinho para a tela quando está minimizado ou na bandeja: as lives que ele abre ou troca em segundo plano não tiram mais a janela de lá.',
        'Removida a opção "Um navegador para todas as contas": com as lives abertas, ela economizava só cerca de 250 MB (não 1 GB, como dizia a 0.19.0). Quem ligou volta para um navegador por conta, com os logins de antes de ligar, e a pasta que ela usava é apagada.',
        'Settings: os cards de Atalhos, Discord e Sobre não encostam mais um no outro, e os da aba Geral ficam alinhados no topo.',
      ],
      en: [
        'The app no longer comes back on screen by itself while minimized or in the tray: streams it opens or swaps in the background don\'t pull the window back anymore.',
        'Removed the "One browser for all accounts" option: with the streams open it only saved about 250 MB (not 1 GB, as 0.19.0 said). If you turned it on, each account goes back to its own browser with the logins it had before, and the folder the option used is deleted.',
        'Settings: the Shortcuts, Discord and About cards no longer touch each other, and the General cards keep their content at the top.',
      ],
      es: [
        'La app ya no vuelve sola a la pantalla cuando está minimizada o en la bandeja: los streams que abre o cambia en segundo plano ya no traen la ventana de vuelta.',
        'Se quitó la opción "Un navegador para todas las cuentas": con los streams abiertos solo ahorraba cerca de 250 MB (no 1 GB, como decía la 0.19.0). Si la activaste, cada cuenta vuelve a su propio navegador con las sesiones que tenía antes, y se borra la carpeta que usaba.',
        'Ajustes: las tarjetas de Atajos, Discord y Acerca de ya no se tocan, y las de General mantienen su contenido arriba.',
      ],
    },
  },
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
