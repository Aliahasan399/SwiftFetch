/**
 * SwiftFetch — Firefox (Manifest V2) background script.
 *
 * This is the MV2 counterpart of chrome/background.js. Manifest V2 background
 * pages are classic scripts, so the shared protocol module is preloaded by
 * `background.scripts` from shared/protocol.bundle.js and read from the global.
 * The ES module copies in shared/protocol.js and shared/native-response.js are
 * byte-equivalent and stay for Node tests and the Chrome MV3 service worker.
 *
 * Every chrome.* call goes through `callApi`, which accepts both the callback
 * form (Firefox MV2) and the promise form (Chrome MV3 / newer Firefox).
 *
 * Behaviour is identical: explicit gestures only, full validation before
 * anything is forwarded to the native host.
 */
(function () {
  'use strict';

  const NATIVE_HOST = 'com.swiftfetch.app';
  const STORAGE_KEY = 'swiftfetch_settings';
  const MENU_ITEMS = [
    { id: 'swiftfetch-link', title: 'Download with SwiftFetch', contexts: ['link'] },
    {
      id: 'swiftfetch-media',
      title: 'Download media with SwiftFetch',
      contexts: ['image', 'video', 'audio'],
    },
  ];

  const state = {
    protocol: null,
    replayGuard: null,
  };

  /** Call a chrome.* API that may be callback-style or promise-style. */
  function callApi(fn, ...args) {
    return new Promise((resolve, reject) => {
      let settled = false;
      const settle = (value, error) => {
        if (settled) return;
        settled = true;
        if (error) reject(error);
        else resolve(value);
      };
      try {
        const returned = fn(...args, (result) => {
          const lastError = chrome.runtime.lastError;
          settle(result, lastError ? new Error(lastError.message) : null);
        });
        if (returned && typeof returned.then === 'function') {
          returned.then(
            (result) => settle(result, null),
            (error) => settle(null, error)
          );
        }
      } catch (error) {
        settle(null, error);
      }
    });
  }

  async function loadProtocol() {
    if (state.protocol !== null) return state.protocol;
    state.protocol = globalThis.SwiftFetchProtocol || null;
    return state.protocol;
  }

  async function readSettings() {
    const fallback = { enabled: true, siteRules: {} };
    try {
      const stored = await callApi(chrome.storage.local.get, STORAGE_KEY);
      const value = stored && stored[STORAGE_KEY];
      if (!value || typeof value !== 'object') return fallback;
      return {
        enabled: value.enabled !== false,
        siteRules:
          value.siteRules && typeof value.siteRules === 'object' ? value.siteRules : {},
      };
    } catch (error) {
      return fallback;
    }
  }

  async function writeSettings(settings) {
    await callApi(chrome.storage.local.set, { [STORAGE_KEY]: settings });
  }

  function siteRuleFor(settings, origin) {
    const rule = settings.siteRules[origin];
    return rule === 'allow' || rule === 'deny' ? rule : 'ask';
  }

  function safeOriginFromTab(tabUrl) {
    try {
      return new URL(tabUrl).origin;
    } catch (error) {
      return '';
    }
  }

  async function forwardToNativeHost(intent) {
    let response;
    try {
      response = await callApi(chrome.runtime.sendNativeMessage, NATIVE_HOST, intent);
    } catch (error) {
      return {
        ok: false,
        code: 'native_host_unavailable',
        message: 'SwiftFetch native host is not available. Check the per-user browser registration.',
        retryable: true,
      };
    }
    // The bundle is preloaded by background.scripts; a missing global means the
    // add-on is damaged, which is reported honestly instead of being blamed on
    // the host. A reply is never simplified by hand — normalizeNativeResponse
    // maps every known code and hides unknown host text.
    const shared = globalThis.SwiftFetchNativeResponse;
    if (!shared || typeof shared.normalizeNativeResponse !== 'function') {
      return {
        ok: false,
        code: 'protocol_unavailable',
        message: 'The SwiftFetch bridge could not read the native reply. Reload the add-on.',
        retryable: true,
      };
    }
    return shared.normalizeNativeResponse(response);
  }

  async function handleDownloadIntent(payload, source) {
    const settings = await readSettings();
    if (settings.enabled === false) {
      return {
        ok: false,
        code: 'extension_disabled',
        message: 'SwiftFetch capture is disabled. Enable it in the toolbar popup.',
        retryable: false,
      };
    }

    const rule = siteRuleFor(settings, payload.pageOrigin || '');
    if (rule === 'deny') {
      return {
        ok: false,
        code: 'site_blocked',
        message: 'This site is blocked in the SwiftFetch popup.',
        retryable: false,
      };
    }

    const protocol = await loadProtocol();
    if (!protocol) {
      return {
        ok: false,
        code: 'protocol_unavailable',
        message: 'SwiftFetch protocol module could not be loaded.',
        retryable: true,
      };
    }

    const built = protocol.buildIntent(
      {
        url: payload.url,
        suggestedName: payload.suggestedName,
        pageOrigin: payload.pageOrigin,
        contentType: payload.contentType,
        userInitiated: true,
        cookies: [],
        ttlMs: payload.ttlMs,
      },
      { now: Date.now(), allowCookies: false, replayGuard: state.replayGuard }
    );
    if (!built.ok) {
      return { ok: false, code: built.code, message: built.message, retryable: false };
    }

    const intent = built.value;
    const result = await forwardToNativeHost(intent);
    if (!result.ok) {
      // Log only a redacted summary — never the full URL.
      console.warn('[swiftfetch] intent rejected', {
        source,
        code: result.code,
        summary: protocol.redactForLog(intent),
      });
    }
    return result;
  }

  async function installMenus() {
    try {
      await callApi(chrome.contextMenus.removeAll);
    } catch (error) {
      // Ignore: menus are retried on the next startup.
    }
    for (const item of MENU_ITEMS) {
      try {
        chrome.contextMenus.create({ id: item.id, title: item.title, contexts: item.contexts });
      } catch (error) {
        // Ignore duplicate-create races.
      }
    }
  }

  function handleMessage(message, _sender, sendResponse) {
    if (!message || typeof message !== 'object') return undefined;

    if (message.type === 'swiftfetch_intent') {
      handleDownloadIntent(message.payload || {}, message.source || 'content_script')
        .then(sendResponse)
        .catch(() =>
          sendResponse({
            ok: false,
            code: 'internal_error',
            message: 'SwiftFetch could not process this download.',
            retryable: true,
          })
        );
      return true;
    }

    if (message.type === 'swiftfetch_get_state') {
      Promise.all([
        readSettings(),
        callApi(chrome.tabs.query, { active: true, currentWindow: true }).catch(() => []),
      ])
        .then(([settings, tabs]) => {
          const tab = tabs && tabs[0] ? tabs[0] : null;
          let origin = '';
          try {
            origin = tab && tab.url ? new URL(tab.url).origin : '';
          } catch (error) {
            origin = '';
          }
          sendResponse({
            ok: true,
            enabled: settings.enabled !== false,
            origin,
            pageUrl: tab && tab.url ? tab.url : '',
            rule: origin ? siteRuleFor(settings, origin) : 'ask',
          });
        })
        .catch(() => sendResponse({ ok: false, code: 'state_error' }));
      return true;
    }

    if (message.type === 'swiftfetch_set_enabled') {
      const enabled = message.enabled === true;
      readSettings()
        .then((settings) => {
          settings.enabled = enabled;
          return writeSettings(settings);
        })
        .then(() => sendResponse({ ok: true, enabled }))
        .catch(() => sendResponse({ ok: false, code: 'storage_error' }));
      return true;
    }

    if (message.type === 'swiftfetch_set_rule') {
      const origin = typeof message.origin === 'string' ? message.origin : '';
      const rule = message.rule === 'allow' || message.rule === 'deny' ? message.rule : 'ask';
      readSettings()
        .then((settings) => {
          settings.siteRules[origin] = rule;
          return writeSettings(settings);
        })
        .then(() => sendResponse({ ok: true, origin, rule }))
        .catch(() => sendResponse({ ok: false, code: 'storage_error' }));
      return true;
    }

    return undefined;
  }

  async function bootstrap() {
    const protocol = await loadProtocol();
    if (protocol) state.replayGuard = protocol.createReplayGuard();
    await installMenus();
    chrome.contextMenus.onClicked.addListener((info, tab) => {
      const url = info.linkUrl || info.srcUrl;
      if (!url) return;
      handleDownloadIntent(
        {
          url,
          suggestedName: '',
          pageOrigin: tab && tab.url ? safeOriginFromTab(tab.url) : '',
          contentType: 'application/octet-stream',
        },
        'context_menu'
      ).catch(() => {});
    });
    chrome.runtime.onMessage.addListener(handleMessage);
  }

  bootstrap();
})();
