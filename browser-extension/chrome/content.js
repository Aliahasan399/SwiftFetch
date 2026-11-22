/**
 * SwiftFetch — content script (explicit gestures only).
 *
 * PRIVACY CONTRACT
 *   - This script never enumerates links, never reads page content, and never
 *     observes network traffic.
 *   - It registers click listeners and does nothing else until the user
 *     performs a gesture (Alt+Click or Ctrl+Shift+Click on a link).
 *   - Nothing is sent to SwiftFetch without that gesture, or without a
 *     per-site "allow" rule the user configured in the popup.
 *
 * The background service worker is the authoritative validator; the optional
 * protocol import here is only a fast pre-check so obvious junk is not sent.
 */
(function () {
  'use strict';

  if (window.__swiftfetchContentLoaded === true) return;
  window.__swiftfetchContentLoaded = true;

  function isGestureEvent(event) {
    return event.altKey === true || (event.ctrlKey === true && event.shiftKey === true);
  }

  function anchorFromEvent(event) {
    let node = event.target;
    while (node && node !== document) {
      if (node.tagName === 'A' && typeof node.href === 'string' && node.href.length > 0) {
        return node;
      }
      node = node.parentNode;
    }
    return null;
  }

  async function getSiteRule(origin) {
    try {
      const stored = await chrome.storage.local.get('swiftfetch_settings');
      const settings = stored && stored.swiftfetch_settings;
      if (settings?.enabled === false) return 'deny';
      if (!settings || !settings.siteRules) return 'ask';
      const rule = settings.siteRules[origin];
      return rule === 'allow' || rule === 'deny' ? rule : 'ask';
    } catch (error) {
      return 'ask';
    }
  }

  function currentOrigin() {
    return window.location.origin || '';
  }

  async function handleGesture(event) {
    if (event.isTrusted !== true || event.button !== 0 || !isGestureEvent(event)) return;

    const anchor = anchorFromEvent(event);
    if (!anchor) return;

    let url;
    try {
      url = new URL(anchor.href).href;
    } catch (error) {
      return;
    }

    // Only http/https ever leave the browser.
    if (url.indexOf('http://') !== 0 && url.indexOf('https://') !== 0) return;

    // Cancel navigation during the trusted event, before the first await.
    event.preventDefault();
    event.stopPropagation();

    const origin = currentOrigin();
    const rule = await getSiteRule(origin);
    if (rule === 'deny') return;

    if (rule === 'ask') {
      const confirmed = window.confirm(
        `Send this download to SwiftFetch?\n\n${url}\n\nOnly the link address is sent. No cookies or credentials are included.`
      );
      if (!confirmed) return;
    }

    try {
      const response = await chrome.runtime.sendMessage({
        type: 'swiftfetch_intent',
        source: 'content_script',
        payload: {
          url,
          suggestedName: anchor.getAttribute('download') || '',
          pageOrigin: origin,
          contentType: anchor.type || 'application/octet-stream',
        },
      });
      if (!response?.ok) window.alert(response?.message || 'SwiftFetch did not accept the download. Open the app or paste the link manually.');
    } catch (error) {
      window.alert('SwiftFetch is unavailable. Open the app or paste the link manually.');
    }
  }

  document.addEventListener('click', handleGesture, true);
  document.addEventListener('auxclick', handleGesture, true);

  /* ---------------------------------------------------------------- *
   * Bounded accessible-media detection (Phase 2–4)
   * Detection alone never sends a download. Only a real click on the
   * SwiftFetch indicator opens the panel, and only a panel button creates
   * a confirmation request.
   * ---------------------------------------------------------------- */
  async function startMediaDetection() {
    const detect = globalThis.SwiftFetchMediaDetect;
    const indicator = globalThis.SwiftFetchMediaIndicator;
    if (!detect || !indicator) return;
    let settings = null;
    try {
      const stored = await chrome.storage.local.get('swiftfetch_settings');
      settings = stored && stored.swiftfetch_settings;
    } catch (error) {
      return;
    }
    if (settings && settings.enabled === false) return;
    const origin = currentOrigin();
    const rule = settings && settings.siteRules && settings.siteRules[origin];
    if (rule === 'deny') return;

    const attached = new Set();
    const controllers = [];

    function attachAll() {
      const descriptors = detect.scanMedia(document, 8);
      for (const descriptor of descriptors) {
        const key = descriptor.url;
        if (attached.has(key)) continue;
        attached.add(key);
        descriptor.domain = origin;
        controllers.push(indicator.attachIndicator(descriptor, openPanel, document));
      }
    }

    function openPanel(descriptor) {
      indicator.openMediaPanel(descriptor, (variant, action) => {
        let suggestedName = '';
        try { suggestedName = decodeURIComponent(new URL(variant.url).pathname.split('/').pop() || ''); }
        catch (error) { suggestedName = ''; }
        chrome.runtime.sendMessage({
          type: 'swiftfetch_media_selection',
          payload: {
            url: variant.url,
            suggestedName,
            pageOrigin: origin,
            contentType: variant.mime || descriptor.mime || 'application/octet-stream',
            action,
            metadata: {
              kind: 'media',
              site_rule: rule === 'allow' ? 'allow' : 'ask',
              mime: variant.mime || descriptor.mime || undefined,
              media: {
                url: variant.url,
                mime: variant.mime || descriptor.mime || undefined,
                label: variant.label,
                width: descriptor.width || undefined,
                height: descriptor.height || undefined,
                variants: descriptor.variants.map((item) => ({
                  url: item.url, mime: item.mime || undefined, label: item.label,
                })),
              },
            },
          },
        }).catch(() => {});
      }, document);
    }

    attachAll();
    const observer = detect.createBoundedObserver(document, attachAll);
    window.addEventListener('pagehide', () => {
      observer.disconnect();
      controllers.forEach((controller) => controller.remove());
    }, { once: true });
  }

  void startMediaDetection();

})();
