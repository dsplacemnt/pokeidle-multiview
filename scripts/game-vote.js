// Run in the game page of the account chosen in ⚙ → "Remind me to vote": asks the game for this
// account's IdleHub vote status, the same request the game makes when its "Vote no IdleHub.io"
// window opens (POST /idlehub/me with the page's own session). The session stays in the page:
// only the status goes back to the app, through report.js:
// "[multiview-vote] {"votes":…,"diamonds":…,"waitMs":…,"url":"https://idlehub.io/votar/…"}".
// Votes are never cast here: the player votes on IdleHub (Google/Discord sign-in, human check).
(async () => {
  const report = (status) => window.__multiviewReport(`[multiview-vote] ${JSON.stringify(status)}`);
  try {
    const session = JSON.parse(localStorage.getItem('sessao') || 'null');
    if (!session || !session.token) return report({ error: 'This game page is not logged in' });
    const response = await fetch('/idlehub/me', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ token: session.token }),
    });
    const s = await response.json();
    report({
      votes: Number(s.votos) || 0,
      diamonds: Number(s.diamantes) || 0,
      perDiamond: Number(s.votosPorDiamante) || 0,
      inPair: Number(s.noPar) || 0,
      waitMs: Math.max(0, Number(s.esperaMs) || 0),
      nick: String(s.identificador || ''),
      url: String(s.url || ''),
    });
  } catch (e) {
    report({ error: String(e) });
  }
})();
