// Runs before any page script. Turns WebAuthn off so Twitch/Kick/Google logins fall back to
// password + code instead of the Windows passkey prompt, which tends to hang inside embedded
// webviews. Set MULTIVIEW_ALLOW_WEBAUTHN=1 to keep WebAuthn on.
(() => {
  try {
    const reject = () => Promise.reject(new DOMException('NotAllowedError', 'NotAllowedError'));
    Object.defineProperty(Navigator.prototype, 'credentials', {
      value: { get: reject, create: reject, store: reject },
      configurable: false,
    });
    window.PublicKeyCredential = undefined;
  } catch {}
})();
