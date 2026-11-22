/**
 * SwiftFetch — Chrome / Edge (Manifest V3) service worker.
 *
 * Responsibilities:
 *   1. Expose explicit, user-driven capture paths (context menu, popup).
 *   2. Validate every download_intent with the shared protocol module.
 *   3. Forward validated intents to the SwiftFetch native messaging host.
 *
 * This worker never inspects page content and never builds an intent without a
 * user gesture or a per-site rule the user configured in the popup.
 */
import {
  MAX_COOKIES,
  buildIntent,
  redactForLog,
  validateIntent,
  createReplayGuard,
} from '../shared/protocol.js';
import { normalizeNativeResponse } from '../shared/native-response.js';

const NATIVE_HOST = 'com.swiftfetch.app';
const STORAGE_KEY = 'swiftfetch_settings';

/** In-memory replay protection. Bounded by REPLAY_WINDOW_MS. */
const replayGuard = createReplayGuard();

/* ------------------------------------------------------------------ *
 * Settings
 * ------------------------------------------------------------------ */

async function readSettings() {
  const fallback = { enabled: true, siteRules: {} };
  try {
    const stored = await chrome.storage.local.get(STORAGE_KEY);
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
  await chrome.storage.local.set({ [STORAGE_KEY]: settings });
}

/**
 * Per-site decision. Returns 'allow' | 'deny' | 'ask'.
 * Default is 'ask': the extension never acts without a gesture or a rule.
 */
function siteRuleFor(settings, origin) {
  const rule = settings.siteRules[origin];
  if (rule === 'allow' || rule === 'deny') return rule;
  return 'ask';
}

/* ------------------------------------------------------------------ *
 * Native messaging
 * ------------------------------------------------------------------ */

/**
 * Forward a validated intent to the native host. Never throws: every failure
 * is returned as a structured error so the UI can show a safe message.
 */
async function forwardToNativeHost(intent) {
  let response;
  try {
    response = await chrome.runtime.sendNativeMessage(NATIVE_HOST, intent);
  } catch (error) {
    return {
      ok: false,
      code: 'native_host_unavailable',
      message: 'SwiftFetch native host is unavailable. Check the per-user browser registration.',
      retryable: true,
    };
  }

  return normalizeNativeResponse(response);
}

/* ------------------------------------------------------------------ *
 * Intent pipeline
 * ------------------------------------------------------------------ */

/**
 * Validate and forward a download intent.
 *
 * `source` is 'context_menu' | 'popup' | 'content_script'. All three originate
 * from an explicit user gesture; the background never synthesises one.
 */
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

  const built = buildIntent(
    {
      url: payload.url,
      suggestedName: payload.suggestedName,
      pageOrigin: payload.pageOrigin,
      contentType: payload.contentType,
      userInitiated: true,
      cookies: [],
      ttlMs: payload.ttlMs,
      metadata: payload.metadata,
    },
    { now: Date.now(), allowCookies: false, replayGuard }
  );

  if (!built.ok) {
    return { ok: false, code: built.code, message: built.message, retryable: false };
  }

  // Defence in depth: the host and the app validate again.
  const intent = built.value;
  const result = await forwardToNativeHost(intent);
  if (!result.ok) {
    // Log only a redacted summary — never the full URL.
    console.warn('[swiftfetch] intent rejected', {
      source,
      code: result.code,
      summary: redactForLog(intent),
    });
  }
  return result;
}

/* ------------------------------------------------------------------ *
 * Context menu (explicit gesture: right-click a link/media)
 * ------------------------------------------------------------------ */

const MENU_ITEMS = [
  { id: 'swiftfetch-link', title: 'Download with SwiftFetch', contexts: ['link'] },
  { id: 'swiftfetch-media', title: 'Download media with SwiftFetch', contexts: ['image', 'video', 'audio'] },
];

async function installMenus() {
  try {
    await chrome.contextMenus.removeAll();
  } catch (error) {
    // Not fatal: menus are re-created on the next worker start.
  }
  for (const item of MENU_ITEMS) {
    try {
      await chrome.contextMenus.create({
        id: item.id,
        title: item.title,
        contexts: item.contexts,
      });
    } catch (error) {
      // Ignore duplicate-create races.
    }
  }
}

chrome.contextMenus.onClicked.addListener((info, tab) => {
  const url = info.linkUrl || info.srcUrl;
  if (!url) return;
  handleDownloadIntent(
    {
      url,
      suggestedName: typeof info.linkUrl === 'string' ? undefined : undefined,
      pageOrigin: tab && tab.url ? safeOriginFromTab(tab.url) : '',
      contentType: 'application/octet-stream',
    },
    'context_menu'
  ).catch(() => {});
});

/** Derive a bare origin from a tab URL without ever storing the full URL. */
function safeOriginFromTab(tabUrl) {
  try {
    return new URL(tabUrl).origin;
  } catch (error) {
    return '';
  }
}

/* ------------------------------------------------------------------ *
 * Ordinary browser download interception (Manifest V3)
 * ------------------------------------------------------------------ */

function safeOriginFromUrl(value) {
  try { return new URL(value).origin; } catch (error) { return ''; }
}

async function cancelBrowserDownload(id) {
  try { await chrome.downloads.cancel(id); } catch (error) { /* already completed */ }
}

/**
 * Hand an ordinary browser download to SwiftFetch. We only cancel the browser
 * download after the native host accepted the intent, so a bridge failure never
 * loses the user's download.
 */
async function handleBrowserDownload(item) {
  if (!item || typeof item.url !== 'string' || !item.url) return;
  const settings = await readSettings();
  if (settings.enabled === false) return;

  const page = typeof item.referrer === 'string' && item.referrer ? item.referrer : item.url;
  const origin = safeOriginFromUrl(page) || safeOriginFromUrl(item.url);
  const rule = siteRuleFor(settings, origin);
  if (rule === 'deny') { await cancelBrowserDownload(item.id); return; }

  const sizeHint =
    typeof item.totalBytes === 'number' && Number.isFinite(item.totalBytes) && item.totalBytes >= 0
      ? item.totalBytes
      : undefined;
  const built = buildIntent(
    {
      url: item.url,
      suggestedName: item.filename || '',
      pageOrigin: origin,
      contentType: item.mime || 'application/octet-stream',
      userInitiated: true,
      cookies: [],
      metadata: {
        kind: 'ordinary',
        site_rule: rule,
        auto_accept: rule === 'allow',
        ...(sizeHint === undefined ? {} : { size_hint: sizeHint }),
        ...(item.mime ? { mime: item.mime } : {}),
      },
    },
    { now: Date.now(), allowCookies: false, replayGuard }
  );
  if (!built.ok) return;

  const result = await forwardToNativeHost(built.value);
  if (result.ok) await cancelBrowserDownload(item.id);
}

function installDownloadInterceptor() {
  if (!chrome.downloads || !chrome.downloads.onCreated || typeof chrome.downloads.cancel !== 'function') return;
  chrome.downloads.onCreated.addListener((item) => { void handleBrowserDownload(item); });
}

/* ------------------------------------------------------------------ *
 * Message bus (content script + popup)
 * ------------------------------------------------------------------ */

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (!message || typeof message !== 'object') return undefined;

  if (message.type === 'swiftfetch_media_selection') {
    handleDownloadIntent(message.payload || {}, 'media_panel')
      .then(sendResponse)
      .catch(() => sendResponse({ ok: false, code: 'internal_error', message: 'SwiftFetch could not process this media selection.', retryable: true }));
    return true;
  }

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
    return true; // async response
  }

  if (message.type === 'swiftfetch_get_state') {
    Promise.all([
      readSettings(),
      chrome.tabs.query({ active: true, currentWindow: true }).catch(() => []),
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
});

/* ------------------------------------------------------------------ *
 * Badge + startup
 * ------------------------------------------------------------------ */

async function refreshBadge() {
  const settings = await readSettings();
  const text = settings.enabled === false ? 'OFF' : '';
  try {
    await chrome.action.setBadgeText({ text });
    await chrome.action.setBadgeBackgroundColor({ color: settings.enabled === false ? '#888888' : '#1a7f37' });
  } catch (error) {
    // Badge is cosmetic.
  }
}

chrome.runtime.onStartup.addListener(() => {
  installMenus();
  refreshBadge();
  installDownloadInterceptor();
});

chrome.runtime.onInstalled.addListener((details) => {
  installMenus();
  refreshBadge();
  installDownloadInterceptor();
  if (details.reason === 'install') {
    chrome.storage.local.set({ [STORAGE_KEY]: { enabled: true, siteRules: {} } }).catch(() => {});
  }
});

// Keep a reference so tree-shaking/tooling does not drop the constant.
void MAX_COOKIES;

installMenus();
refreshBadge();
installDownloadInterceptor();
