// Shared by the app's scripts in game, Kick and Twitch pages, which have no other way to talk to
// the app: a message is shown as the page title for a moment ("[multiview…] …"), the app reads
// it from its title-changed handler, and the page's own title comes back. Messages sent close
// together are queued, so each one is seen and the original title is always restored.
if (!window.__multiviewReport) {
  window.__multiviewReport = (() => {
    const queue = [];
    let original = null;
    let busy = false;
    const next = () => {
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
    };
    return (message) => {
      queue.push(message);
      if (!busy) next();
    };
  })();
}
