// What changed in each version, newest first, shown by whatsnew.html after an update and from
// ⚙ → About. Add an entry with every release (pt, en and es; a missing language falls back to
// English).
window.CHANGELOG = [
  {
    version: '0.27.0',
    date: '2026-11-26',
    notes: {
      pt: [
        'Novo: Análise do time (a Pokébola ao lado do ⚙). Mostra a força de cada Pokémon das suas contas e o que ele vira ao evoluir, com as fórmulas do próprio jogo, para escolher quem vale a pena upar com o Exp. Share e os postos da Casa.',
        'Uma aba por conta, com Time, Coleção ★ e Depot separados, e um Ranking com todos de todas as contas, do mais forte ao mais fraco.',
        'Para cada Pokémon: a Nota agora e a Nota potencial (a melhor forma final ou Mega), o dano físico (ATK) e o especial (SP.ATK) com o IV que conta em cada um, Potência, Qualidade e IV com as cores do jogo, e todas as evoluções e Megas com o dano em cada uma.',
        'Filtros: o nível em que todos são comparados (1000 de início), a Nota final mínima (3,5 de início) e se as Megas contam.',
        'Os dados das espécies vêm do site do jogo e ficam guardados por um dia. A lista de cada conta atualiza sozinha enquanto o jogo do slot está aberto (slots "Só lives" não têm lista).',
      ],
      en: [
        'New: Team analysis (the Pokéball next to ⚙). It shows how strong each Pokémon in your accounts is and what it becomes when it evolves, with the game\'s own formulas, to pick who is worth leveling with Exp. Share and House posts.',
        'One tab per account, with Team, Collection ★ and Depot apart, and a Ranking with everyone from every account, strongest first.',
        'For each Pokémon: its Score now and its Score Potential (its best final form or Mega), its physical (ATK) and special (SP.ATK) damage with the IV that counts for each, Potency, Quality and IV in the game\'s colours, and every evolution and Mega with its damage there.',
        'Filters: the level everyone is compared at (1000 to start), the minimum final Score (3.5 to start) and whether Megas count.',
        'Species data comes from the game\'s site and is kept for a day. Each account\'s list updates by itself while the slot\'s game is open ("Streams only" slots have no list).',
      ],
      es: [
        'Nuevo: Análisis del equipo (la Pokébola al lado de ⚙). Muestra la fuerza de cada Pokémon de tus cuentas y en qué se convierte al evolucionar, con las fórmulas del propio juego, para elegir a quién vale la pena subir con el Exp. Share y los puestos de la Casa.',
        'Una pestaña por cuenta, con Equipo, Colección ★ y Depósito por separado, y un Ranking con todos los de todas las cuentas, del más fuerte al más débil.',
        'Para cada Pokémon: su Nota ahora y su Potencial de Nota (su mejor forma final o Mega), su daño físico (ATK) y especial (SP.ATK) con el IV que cuenta en cada uno, Potencia, Calidad e IV con los colores del juego, y todas sus evoluciones y Megas con el daño en cada una.',
        'Filtros: el nivel en que se comparan todos (1000 al inicio), la Nota final mínima (3,5 al inicio) y si cuentan las Megas.',
        'Los datos de las especies vienen del sitio del juego y se guardan por un día. La lista de cada cuenta se actualiza sola mientras el juego del slot está abierto (los slots "Solo streams" no tienen lista).',
      ],
    },
  },
  {
    version: '0.26.2',
    date: '2026-11-25',
    notes: {
      pt: [
        'Painel de estatísticas: depois de uma captura, o Pokémon capturado aparecia como se fosse o seu em campo (ex.: "Snover Lv 40 vs Snover"). Agora o painel segue o Pokémon que o jogo diz estar em campo, e as capturas não trocam mais ele.',
      ],
      en: [
        'Stats panel: after a catch, the Pokémon just caught showed as if it were yours on the field (e.g. "Snover Lv 40 vs Snover"). The panel now follows the Pokémon the game says is on the field, and catches no longer swap it.',
      ],
      es: [
        'Panel de estadísticas: después de una captura, el Pokémon recién capturado aparecía como si fuera el tuyo en campo (ej.: "Snover Lv 40 vs Snover"). Ahora el panel sigue al Pokémon que el juego dice que está en campo, y las capturas ya no lo cambian.',
      ],
    },
  },
  {
    version: '0.26.1',
    date: '2026-11-25',
    notes: {
      pt: [
        'Slots "Só lives" consertados: o slot ficava com a tela preta e sem cabeçalho (nem dava para renomear), e com um slot desses o app podia nem abrir na próxima vez. Agora, no lugar do jogo, aparece a lista das lives abertas dessa conta, com os pontos de canal dela em cada uma, da que tem mais para a que tem menos. Clique numa live para assistir ali mesmo; clicar de novo volta para a lista.',
        'Os slots Só lives não mostram mais tempos de bônus estimados: sem o jogo aqui, a conta saía errada sempre que um resgate era feito em outro lugar. Os tempos certos aparecem no jogo, no computador que roda ele.',
        'Ao fechar um slot, o que o jogo dele tinha informado (caçada, itens, bônus, voto, Prey) é apagado. Um slot novo no mesmo lugar começa limpo, e o modo economia não mostra mais os dados da conta antiga.',
        'Se um slot não conseguir abrir, o app abre mesmo assim com os outros, em vez de não abrir.',
      ],
      en: [
        'Streams-only slots fixed: the slot showed a black screen and no header (so it couldn\'t even be renamed), and with such a slot the app might not open the next time. Now, where the game would be, it lists that account\'s open streams with its channel points on each, most first. Click a stream to watch it right there; clicking again comes back to the list.',
        'Streams-only slots no longer show estimated bonus times: with no game here, they went wrong whenever a redeem was made elsewhere. The real times show in the game, on the computer that runs it.',
        'Closing a slot clears what its game had reported (hunt, items, bonuses, vote, Prey). A new slot in the same place starts clean, and economy mode no longer shows the old account\'s data.',
        'If a slot can\'t open, the app still opens with the others instead of not opening at all.',
      ],
      es: [
        'Slots "Solo streams" arreglados: el slot quedaba con la pantalla negra y sin encabezado (ni se podía renombrar), y con un slot así la app podía no abrir la próxima vez. Ahora, en lugar del juego, aparece la lista de los streams abiertos de esa cuenta, con sus puntos de canal en cada uno, de mayor a menor. Haz clic en un stream para verlo ahí mismo; hacer clic de nuevo vuelve a la lista.',
        'Los slots Solo streams ya no muestran tiempos de bono estimados: sin el juego aquí, la cuenta salía mal cada vez que se canjeaba en otro lado. Los tiempos reales se ven en el juego, en la computadora que lo corre.',
        'Al cerrar un slot, se borra lo que su juego había informado (caza, objetos, bonos, voto, Prey). Un slot nuevo en el mismo lugar empieza limpio, y el modo ahorro ya no muestra los datos de la cuenta anterior.',
        'Si un slot no puede abrir, la app abre igual con los demás en vez de no abrir.',
      ],
    },
  },
  {
    version: '0.26.0',
    date: '2026-11-25',
    notes: {
      pt: [
        'Slots "Só jogo": para uma conta que você quer só jogando, sem lives. Ao clicar em + SLOT agora são três opções: Jogo + lives (o normal), Só jogo e Só lives. Um slot Só jogo não abre lives da Twitch nem da Kick (nem baús ou resgates), então pesa menos no PC; os bônus que o jogo informa continuam aparecendo no cabeçalho e no modo economia.',
        'Em ⚙ → Contas, cada conta tem os três modos lado a lado para trocar quando quiser. Passar uma conta para Só jogo fecha as lives dela na hora, sem recarregar o jogo; voltar para Jogo + lives abre as lives de novo.',
        'O modo economia ficou mais compacto: em telas não muito altas (como Full HD com a janela maximizada), a Pokébola, o título e o botão ficam numa linha só, e os avisos de "Precisa de você" ocupam menos espaço. Assim quatro contas cabem sem rolar a tela.',
        'Trocar o modo de um slot em ⚙ → Contas não mostra mais uma mensagem de erro por engano.',
      ],
      en: [
        'Game-only slots: for an account you only want playing, with no streams. Clicking + SLOT now offers three choices: Game + streams (the usual), Game only and Streams only. A game-only slot opens no Twitch or Kick streams (and no chests or redeems), so it\'s lighter on the PC; the bonuses the game reports still show in its header and in economy mode.',
        'In ⚙ → Accounts, each account has the three modes side by side to switch at any time. Switching an account to Game only closes its streams right away, without reloading the game; going back to Game + streams opens them again.',
        'Economy mode is more compact: on screens that aren\'t very tall (like Full HD with the window maximized), the Poké Ball, the title and the button sit on one line, and the "Needs you" notices take less room. So four accounts fit without scrolling.',
        'Changing a slot\'s mode in ⚙ → Accounts no longer shows an error message by mistake.',
      ],
      es: [
        'Slots "Solo juego": para una cuenta que solo quieres jugando, sin streams. Al hacer clic en + SLOT ahora hay tres opciones: Juego + streams (lo normal), Solo juego y Solo streams. Un slot Solo juego no abre streams de Twitch ni de Kick (ni cofres ni canjes), así que pesa menos en la PC; los bonos que informa el juego se siguen mostrando en su encabezado y en el modo ahorro.',
        'En ⚙ → Cuentas, cada cuenta tiene los tres modos lado a lado para cambiar cuando quieras. Pasar una cuenta a Solo juego cierra sus streams al instante, sin recargar el juego; volver a Juego + streams los abre de nuevo.',
        'El modo ahorro quedó más compacto: en pantallas no muy altas (como Full HD con la ventana maximizada), la Poké Ball, el título y el botón quedan en una sola línea, y los avisos de "Te necesita" ocupan menos espacio. Así caben cuatro cuentas sin desplazarse.',
        'Cambiar el modo de un slot en ⚙ → Cuentas ya no muestra un mensaje de error por equivocación.',
      ],
    },
  },
  {
    version: '0.25.0',
    date: '2026-11-24',
    notes: {
      pt: [
        'Slots "Só lives": para uma conta que está com o jogo aberto em outro computador. Ao clicar em + SLOT, escolha entre Jogo + lives (o normal) e Só lives. Um slot Só lives não abre o jogo (abrir aqui também derrubaria o do outro PC): ele só roda as lives da Twitch e da Kick dessa conta, abre os baús e resgata os bônus, que chegam à conta do jogo pelo login da Twitch/Kick vinculado.',
        'Sem o jogo aqui, o app estima o tempo de cada bônus pelos resgates que ele mesmo faz (aparece com ≈). Por isso essas contas usam bônus fixo, sem metas, e ficam sem o lembrete de voto. Em ⚙ → Contas dá para trocar um slot entre os dois modos quando quiser, e no modo economia o card dele mostra uma TV transmitindo.',
      ],
      en: [
        'Streams-only slots: for an account whose game is open on another computer. Clicking + SLOT asks Game + streams (the usual) or Streams only. A streams-only slot doesn\'t open the game (opening it here too would log the other PC out): it only runs that account\'s Twitch and Kick streams, opens the chests and redeems the bonuses, which reach the game account through its linked Twitch/Kick login.',
        'With no game here, the app estimates each bonus\'s time from the redeems it makes itself (shown with ≈). So these accounts use a fixed bonus, no goals, and have no vote reminder. ⚙ → Accounts switches a slot between the two modes at any time, and in economy mode its card shows a broadcasting TV.',
      ],
      es: [
        'Slots "Solo streams": para una cuenta que tiene el juego abierto en otra computadora. Al hacer clic en + SLOT, elige entre Juego + streams (lo normal) y Solo streams. Un slot Solo streams no abre el juego (abrirlo aquí también cerraría el de la otra PC): solo corre los streams de Twitch y Kick de esa cuenta, abre los cofres y canjea los bonos, que llegan a la cuenta del juego por el login de Twitch/Kick vinculado.',
        'Sin el juego aquí, la app estima el tiempo de cada bono por los canjes que hace ella misma (aparece con ≈). Por eso esas cuentas usan bono fijo, sin metas, y no tienen el recordatorio de voto. En ⚙ → Cuentas puedes cambiar un slot entre los dos modos cuando quieras, y en el modo ahorro su tarjeta muestra una TV transmitiendo.',
      ],
    },
  },
  {
    version: '0.24.0',
    date: '2026-11-24',
    notes: {
      pt: [
        'Modo economia com todos os bônus de cada conta, e de onde vem cada um: VIP, boosts da loja, Twitch, Kick, guild, sacrifício, raid, eventos e a Prey do dia. Uma linha por efeito (XP, Captura, Lure, Loot e Prey), e cada fonte com o ícone, quanto dá e quanto tempo falta. O XP mostra o total como o jogo calcula (as fontes se multiplicam). Bônus acabando em menos de 15 minutos ficam em amarelo, e os pausados no Safari ficam apagados.',
        'Cada card mostra o seu Pokémon contra o selvagem, em miniatura (nome e nível ao passar o mouse), e o nível do treinador ao lado do nome da conta.',
        'Os cards ficam alinhados: os bônus, a caçada e os itens começam na mesma altura que no card ao lado, tenha a conta muitos bônus ou nenhum.',
      ],
      en: [
        'Economy mode shows every bonus of each account, and where each one comes from: VIP, shop boosts, Twitch, Kick, guild, sacrifice, raid, events and the day\'s Prey. One line per effect (XP, Capture, Lure, Loot and Prey), each source with its icon, how much it gives and the time left. XP shows the total as the game works it out (the sources multiply). Bonuses ending in under 15 minutes turn amber, and ones paused in the Safari are dimmed.',
        'Each card shows your Pokémon against the wild one, as small pictures (name and level when you hover), and the trainer level beside the account\'s name.',
        'The cards line up: bonuses, hunt and items start at the same height as in the card beside them, whether the account has many bonuses or none.',
      ],
      es: [
        'El modo ahorro muestra todos los bonos de cada cuenta, y de dónde viene cada uno: VIP, boosts de la tienda, Twitch, Kick, guild, sacrificio, raid, eventos y la Prey del día. Una línea por efecto (XP, Captura, Lure, Loot y Prey), y cada fuente con su icono, cuánto da y cuánto tiempo le queda. El XP muestra el total como lo calcula el juego (las fuentes se multiplican). Los bonos que terminan en menos de 15 minutos se ponen en ámbar, y los pausados en el Safari quedan atenuados.',
        'Cada tarjeta muestra tu Pokémon contra el salvaje, en miniatura (nombre y nivel al pasar el ratón), y el nivel del entrenador junto al nombre de la cuenta.',
        'Las tarjetas quedan alineadas: los bonos, la caza y los objetos empiezan a la misma altura que en la tarjeta de al lado, tenga la cuenta muchos bonos o ninguno.',
      ],
    },
  },
  {
    version: '0.23.0',
    date: '2026-11-24',
    notes: {
      pt: [
        'Modo economia mais completo: cada conta ganha um card com o que ela está fazendo. Mostra se está caçando (e onde), parada ou desconectada, o nível do treinador, o gold com o lucro por hora e todas as Pokébolas, poções e revives que ela tem, separados por tipo. Os itens marcados para uso automático aparecem primeiro, na ordem em que o jogo usa; os outros ficam apagados. As contas ficam lado a lado, sem precisar rolar.',
        'Os itens aparecem só com o ícone do jogo, no modo economia e na análise da caçada. O nome aparece ao passar o mouse.',
      ],
      en: [
        'Fuller economy mode: each account gets a card with what it\'s doing. It shows whether it\'s hunting (and where), stopped or disconnected, the trainer level, the gold with the profit per hour and every Poké Ball, potion and revive it has, by kind. The items ticked for automatic use come first, in the order the game uses them; the rest are dimmed. The accounts sit side by side, with no scrolling.',
        'Items show just the game\'s icon, in economy mode and in the hunt analysis. The name shows when you hover over it.',
      ],
      es: [
        'Modo ahorro más completo: cada cuenta tiene una tarjeta con lo que está haciendo. Muestra si está cazando (y dónde), parada o desconectada, el nivel del entrenador, el oro con la ganancia por hora y todas las Poké Balls, pociones y revivir que tiene, por tipo. Los objetos marcados para uso automático salen primero, en el orden en que el juego los usa; los demás quedan atenuados. Las cuentas quedan lado a lado, sin tener que desplazarse.',
        'Los objetos se muestran solo con el icono del juego, en el modo ahorro y en el análisis de la caza. El nombre aparece al pasar el ratón.',
      ],
    },
  },
  {
    version: '0.22.2',
    date: '2026-11-24',
    notes: {
      pt: [
        'Corrigido: com o app minimizado, minimizar outro programa às vezes fazia o Multiview voltar para a tela sozinho. Agora ele só volta quando você pede: pelo botão na barra de tarefas, pelo Alt+Tab ou pelo ícone da bandeja.',
        'No modo economia, cada conta mostra quantas lives tem abertas na Twitch e na Kick, com o logo de cada uma.',
      ],
      en: [
        'Fixed: with the app minimized, minimizing another program sometimes brought Multiview back on screen by itself. It now only comes back when you ask: its taskbar button, Alt+Tab or the tray icon.',
        'In economy mode, each account shows how many Twitch and Kick streams it has open, with each one\'s logo.',
      ],
      es: [
        'Corregido: con la app minimizada, minimizar otro programa a veces hacía que Multiview volviera a la pantalla solo. Ahora solo vuelve cuando lo pides: con su botón en la barra de tareas, Alt+Tab o el icono de la bandeja.',
        'En el modo ahorro, cada cuenta muestra cuántos streams tiene abiertos en Twitch y en Kick, con el logo de cada uno.',
      ],
    },
  },
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
