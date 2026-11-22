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
 * SwiftFetch media detection — bounded, gesture-free discovery of accessible
 * HTML5 video/audio sources. This module never starts a download and never
 * inspects network traffic. It is generated into a classic bundle for Firefox.
 */
'use strict';

const MAX_MEDIA_ELEMENTS = 8;
const MAX_VARIANTS = 32;
const BLOCKED_SCHEMES = new Set(['blob:', 'data:', 'filesystem:', 'javascript:']);
const DIRECT_MEDIA_RE = /\.(?:mp4|m4v|webm|mkv|avi|mov|mp3|m4a|aac|flac|wav|ogg|opus)(?:[?#]|$)/i;

function absoluteUrl(raw, base) {
  if (typeof raw !== 'string' || !raw) return null;
  try {
    const parsed = new URL(raw, base);
    if (BLOCKED_SCHEMES.has(parsed.protocol.toLowerCase())) return null;
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return null;
    return parsed.href;
  } catch (error) {
    return null;
  }
}

function labelFor(mime, width, height) {
  const kind = mime && mime.startsWith('audio/') ? 'Audio' : 'Video';
  const size = width && height ? ` ${width}×${height}` : '';
  return `${kind}${size}`;
}

function descriptorForElement(element, index, doc) {
  if (!(element instanceof HTMLMediaElement)) return null;
  const base = doc.baseURI || doc.URL;
  const sources = Array.from(element.querySelectorAll('source'));
  const candidates = [];
  const direct = element.currentSrc || element.src;
  if (direct) candidates.push({ url: direct, mime: element.type || '' });
  for (const source of sources) {
    candidates.push({ url: source.src || source.getAttribute('src') || '', mime: source.type || '' });
  }

  const variants = [];
  const seen = new Set();
  for (const candidate of candidates) {
    const url = absoluteUrl(candidate.url, base);
    if (!url || seen.has(url)) continue;
    seen.add(url);
    variants.push({
      url,
      mime: candidate.mime || '',
      label: labelFor(candidate.mime, element.width || null, element.height || null),
    });
    if (variants.length >= MAX_VARIANTS) break;
  }
  if (variants.length === 0) return null;

  const first = variants[0];
  return {
    index,
    url: first.url,
    mime: first.mime,
    label: first.label,
    width: element.width || null,
    height: element.height || null,
    tag: element.tagName.toLowerCase(),
    direct: DIRECT_MEDIA_RE.test(first.url),
    variants,
    element,
  };
}

/** Scan the document for accessible media. Never mutates the page. */
function scanMedia(doc, limit) {
  if (!doc || typeof doc.querySelectorAll !== 'function') return [];
  const cap = typeof limit === 'number' && limit > 0 ? Math.min(limit, MAX_MEDIA_ELEMENTS) : MAX_MEDIA_ELEMENTS;
  const elements = Array.from(doc.querySelectorAll('video, audio')).filter(
    (element) => !element.srcObject
  );
  const found = [];
  for (let i = 0; i < elements.length && found.length < cap; i += 1) {
    const descriptor = descriptorForElement(elements[i], found.length, doc);
    if (descriptor) found.push(descriptor);
  }
  return found;
}

/**
 * Short, bounded DOM observation. It stops after `maxEvents` mutations or
 * `maxMs` milliseconds, so it cannot become permanent background monitoring.
 */
function createBoundedObserver(doc, onChange, options) {
  const opts = options || {};
  const debounceMs = typeof opts.debounceMs === 'number' ? opts.debounceMs : 300;
  const maxEvents = typeof opts.maxEvents === 'number' ? opts.maxEvents : 12;
  const maxMs = typeof opts.maxMs === 'number' ? opts.maxMs : 15000;
  if (typeof MutationObserver === 'undefined') return { disconnect() {} };

  let events = 0;
  let timer = null;
  let stopped = false;
  const observer = new MutationObserver((records) => {
    if (stopped) return;
    const relevant = records.some((record) =>
      Array.from(record.addedNodes || []).some(
        (node) => node && node.nodeType === 1 && /^(VIDEO|AUDIO|SOURCE)$/.test(node.tagName)
      )
    );
    if (!relevant) return;
    events += 1;
    if (timer !== null) clearTimeout(timer);
    timer = setTimeout(() => {
      if (stopped) return;
      onChange();
      if (events >= maxEvents) disconnect();
    }, debounceMs);
  });
  const startedAt = Date.now();
  const expiry = setTimeout(disconnect, maxMs);

  function disconnect() {
    if (stopped) return;
    stopped = true;
    observer.disconnect();
    if (timer !== null) clearTimeout(timer);
    clearTimeout(expiry);
  }

  observer.observe(doc.documentElement || doc, { childList: true, subtree: true });
  return { disconnect };
}

const api = { scanMedia, createBoundedObserver, MAX_MEDIA_ELEMENTS, MAX_VARIANTS, absoluteUrl };

if (typeof globalThis !== 'undefined') globalThis.SwiftFetchMediaDetect = api;
if (typeof module !== 'undefined' && module.exports) module.exports = api;

/* Classic-script export: available as a global in a non-module context. */
globalThis.SwiftFetchMediaDetect = api;

})();
