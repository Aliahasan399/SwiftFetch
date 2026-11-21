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

})();
