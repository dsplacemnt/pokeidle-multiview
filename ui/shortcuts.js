// The app's keyboard shortcuts in its own pages (toolbar, slot headers, stats panel): pressing
// a configured combination runs its action. Game, Twitch and Kick pages get the same through
// WebView2 (native.rs). The list comes from the app and follows changes made in ⚙ → Shortcuts.
//
// `Shortcuts.keys(action)` gives an action's keys ("" = off) for tooltips; `Shortcuts.onChange`
// runs a callback when they change.
(() => {
  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;
  let map = {};
  const callbacks = [];

  // "Ctrl+Shift+R" from a key event, written like the app writes it (shortcuts.rs); null when
  // the key can't be part of a shortcut.
  function combo(e) {
    let key = null;
    if (/^Key[A-Z]$/.test(e.code)) key = e.code.slice(3);
    else if (/^Digit[0-9]$/.test(e.code)) key = e.code.slice(5);
    else if (/^F([1-9]|1[0-2])$/.test(e.code)) key = e.code;
    if (!key) return null;
    const isFunction = key.startsWith('F') && key.length > 1;
    if (!isFunction && !e.ctrlKey && !e.altKey) return null;
    return (e.ctrlKey ? 'Ctrl+' : '') + (e.altKey ? 'Alt+' : '') + (e.shiftKey ? 'Shift+' : '') + key;
  }

  function set(next) {
    map = next || {};
    for (const cb of callbacks) cb();
  }

  document.addEventListener('keydown', (e) => {
    const keys = combo(e);
    if (!keys) return;
    const action = Object.keys(map).find((a) => map[a] === keys);
    if (!action) return;
    e.preventDefault();
    e.stopPropagation();
    invoke('run_shortcut', { action }).catch(() => {});
  }, true);

  invoke('get_shortcuts').then(set).catch(() => {});
  listen('shortcuts', (e) => set(e.payload));

  window.Shortcuts = {
    combo,
    keys: (action) => map[action] || '',
    onChange: (cb) => callbacks.push(cb),
  };
})();
