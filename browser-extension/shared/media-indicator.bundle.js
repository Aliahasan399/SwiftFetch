// ------------------------------------------------------------------ #
// GENERATED FILE — do not edit.
//
// Produced by tools/make-protocol-bundle.mjs from the matching ES module
// in this folder. Run `node browser-extension/tools/make-protocol-bundle.mjs`
// after changing the module. The classic form exists so Firefox Manifest V2
// can preload the protocol with `background.scripts`, without relying on a
// dynamic import() inside an event page. The module keeps its exports for
// Node tests and for the Chrome service worker.
// ------------------------------------------------------------------ #
(function () {
/**
 * SwiftFetch floating media indicator and selection panel.
 * Original lightweight design: small, theme-aware, keyboard accessible,
 * positioned near the player without covering its controls.
 */
'use strict';

const MARK = '<svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path d="M13 2 5 13h5l-1 9 9-12h-5z" fill="currentColor"/></svg>';

const INDICATOR_CSS = `
:host { all: initial; }
button {
  display: flex; align-items: center; justify-content: center;
  width: 30px; height: 30px; padding: 0; border-radius: 9px;
  border: 1px solid rgba(255,255,255,.22);
  background: #163d3c; color: #78ebd5; cursor: pointer;
  box-shadow: 0 3px 12px rgba(0,0,0,.35); pointer-events: auto;
}
button:hover, button:focus-visible { background: #1c514e; outline: 2px solid #78ebd5; outline-offset: 2px; }
@media (prefers-color-scheme: light) {
  button { background: #e0f4ee; color: #096b59; border-color: rgba(0,0,0,.18); }
  button:hover, button:focus-visible { background: #c7e9df; outline-color: #096b59; }
}
`;

const PANEL_CSS = `
:host { all: initial; }
.panel {
  font: 13px/1.5 "Segoe UI", system-ui, sans-serif;
  width: 288px; max-width: calc(100vw - 24px); padding: 14px;
  border-radius: 14px; border: 1px solid #2c394a; background: #161e2a; color: #ecf0f7;
  box-shadow: 0 18px 60px rgba(0,0,0,.45); pointer-events: auto;
}
h3 { margin: 0 0 4px; font-size: 14px; }
p { margin: 0 0 10px; color: #a9b5c7; font-size: 12px; }
ul { list-style: none; margin: 0 0 12px; padding: 0; display: grid; gap: 6px; max-height: 180px; overflow: auto; }
li button { width: 100%; text-align: left; padding: 8px 10px; border-radius: 8px; border: 1px solid #2c394a; background: #1b2534; color: #ecf0f7; cursor: pointer; }
li button[aria-pressed="true"] { border-color: #73dec7; box-shadow: inset 0 0 0 1px #73dec7; }
.actions { display: flex; gap: 8px; }
.actions button { flex: 1; padding: 8px 10px; border-radius: 8px; border: 1px solid #2c394a; background: #1b2534; color: #ecf0f7; cursor: pointer; }
.actions .primary { background: #73dec7; color: #082f29; border-color: #73dec7; font-weight: 600; }
@media (prefers-color-scheme: light) {
  .panel { background: #fff; color: #1d293b; border-color: #d3dbe5; }
  p { color: #526179; }
  li button, .actions button { background: #ecf0f5; color: #1d293b; border-color: #d3dbe5; }
  .actions .primary { background: #096b59; color: #fff; border-color: #096b59; }
}
`;

function makeHost(doc, tag) {
  const host = doc.createElement(tag);
  host.style.pointerEvents = 'none';
  host.style.position = 'fixed';
  host.style.zIndex = '2147483000';
  return host;
}

/** Attach one compact indicator to a media element. */
function attachIndicator(descriptor, onClick, doc) {
  const documentRef = doc || descriptor.element.ownerDocument;
  const host = makeHost(documentRef, 'div');
  const shadow = host.attachShadow ? host.attachShadow({ mode: 'closed' }) : host;
  const style = documentRef.createElement('style');
  style.textContent = INDICATOR_CSS;
  const button = documentRef.createElement('button');
  button.type = 'button';
  button.setAttribute('aria-label', 'SwiftFetch media options');
  button.title = 'SwiftFetch media options';
  button.innerHTML = MARK;
  button.addEventListener('click', (event) => {
    event.preventDefault();
    event.stopPropagation();
    onClick(descriptor);
  });
  shadow.appendChild(style);
  shadow.appendChild(button);
  (documentRef.body || documentRef.documentElement).appendChild(host);

  let removed = false;
  function update() {
    if (removed) return;
    if (!descriptor.element.isConnected) {
      host.style.display = 'none';
      return;
    }
    const rect = descriptor.element.getBoundingClientRect();
    const visible = rect.width >= 180 && rect.height >= 100 && rect.bottom > 0 && rect.top < window.innerHeight;
    host.style.display = visible ? 'block' : 'none';
    if (!visible) return;
    host.style.left = `${Math.max(0, rect.left + 8)}px`;
    host.style.top = `${Math.max(0, rect.top + 8)}px`;
  }
  update();
  window.addEventListener('scroll', update, true);
  window.addEventListener('resize', update);

  return {
    update,
    remove() {
      removed = true;
      window.removeEventListener('scroll', update, true);
      window.removeEventListener('resize', update);
      host.remove();
    },
  };
}

/** Open an original SwiftFetch panel listing detectable variants. */
function openMediaPanel(descriptor, onChoose, doc) {
  const documentRef = doc || descriptor.element.ownerDocument;
  const host = makeHost(documentRef, 'div');
  const shadow = host.attachShadow ? host.attachShadow({ mode: 'closed' }) : host;
  const style = documentRef.createElement('style');
  style.textContent = PANEL_CSS;
  const panel = documentRef.createElement('div');
  panel.className = 'panel';
  panel.setAttribute('role', 'dialog');
  panel.setAttribute('aria-label', 'SwiftFetch media options');

  const title = documentRef.createElement('h3');
  title.textContent = 'Media found';
  const subtitle = documentRef.createElement('p');
  subtitle.textContent = descriptor.domain || 'Choose the version you want SwiftFetch to download.';
  const list = documentRef.createElement('ul');
  const actions = documentRef.createElement('div');
  actions.className = 'actions';

  let selected = descriptor.variants[0];
  function renderVariants() {
    list.textContent = '';
    for (const variant of descriptor.variants) {
      const li = documentRef.createElement('li');
      const button = documentRef.createElement('button');
      button.type = 'button';
      button.textContent = `${variant.label || 'Original'}${variant.mime ? ` · ${variant.mime}` : ''}`;
      button.setAttribute('aria-pressed', String(variant.url === selected.url));
      button.addEventListener('click', () => { selected = variant; renderVariants(); });
      li.appendChild(button);
      list.appendChild(li);
    }
  }
  renderVariants();

  function makeButton(label, primary, action) {
    const button = documentRef.createElement('button');
    button.type = 'button';
    button.className = primary ? 'primary' : '';
    button.textContent = label;
    button.addEventListener('click', () => { onChoose(selected, action); close(); });
    return button;
  }
  actions.appendChild(makeButton('Add to queue', false, 'queue'));
  actions.appendChild(makeButton('Start download', true, 'start'));

  panel.append(title, subtitle, list, actions);
  shadow.append(style, panel);
  (documentRef.body || documentRef.documentElement).appendChild(host);

  const rect = descriptor.element.getBoundingClientRect();
  host.style.left = `${Math.max(8, Math.min(window.innerWidth - 296, rect.left))}px`;
  host.style.top = `${Math.max(8, Math.min(window.innerHeight - 240, rect.top + 36))}px`;

  function onKey(event) { if (event.key === 'Escape') close(); }
  documentRef.addEventListener('keydown', onKey);

  function close() {
    documentRef.removeEventListener('keydown', onKey);
    host.remove();
  }
  return { close };
}

const api = { attachIndicator, openMediaPanel, INDICATOR_CSS, PANEL_CSS };
if (typeof globalThis !== 'undefined') globalThis.SwiftFetchMediaIndicator = api;
if (typeof module !== 'undefined' && module.exports) module.exports = api;

/* Classic-script export: available as a global in a non-module context. */
globalThis.SwiftFetchMediaIndicator = api;

})();
