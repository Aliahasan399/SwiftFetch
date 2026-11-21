/**
 * Firefox Manifest V2 background behaviour — VM tests against the real file.
 *
 * firefox/background.js is a classic script for a non-persistent event page, so
 * it cannot be imported. This suite evaluates the actual file in a vm context
 * that reproduces the Firefox MV2 surface:
 *
 *   - callback-style chrome.* APIs (no promises anywhere), the way Firefox 91+
 *     implements the chrome namespace;
 *   - `chrome.runtime.lastError` set before the callback, as Firefox does for a
 *     failed native-messaging spawn;
 *   - the two shared bundles preloaded by `background.scripts`, evaluated in
 *     the same context exactly as the event page loads them.
 *
 * `chrome.runtime.getURL` is a spy that hands out a bogus URL. Any fallback
 * that needed a dynamic import() would fail loudly, so a green run proves the
 * classic preload is what makes the add-on work.
 */
import { describe, it, before } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import vm from 'node:vm';
import { fileURLToPath } from 'node:url';
import * as NativeResponse from '../shared/native-response.js';
import * as Protocol from '../shared/protocol.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const EXTENSION = path.resolve(HERE, '..');
const STORAGE_KEY = 'swiftfetch_settings';
const PAGE_ORIGIN = 'http://127.0.0.1:8080';
const FIXTURE_URL = 'http://127.0.0.1:8080/fixtures/file.zip';

/** Let the event page's async bootstrap settle (its own microtasks + timers). */
async function settle(rounds = 4) {
  for (let i = 0; i < rounds; i += 1) {
    await new Promise((resolve) => setTimeout(resolve, 5));
  }
}

/**
 * The event page lives in its own vm realm, so objects it hands back carry that
 * realm's prototypes. Structured-compare everything that is plain data.
 */
const plain = (value) => (value === undefined ? value : JSON.parse(JSON.stringify(value)));

function createFirefoxChrome() {
  const calls = {
    getURL: [],
    native: [],
    tabsQuery: [],
    storageSet: [],
    menus: [],
  };
  const messageListeners = [];
  const menuListeners = [];
  const storage = new Map();
  const state = { nativeReply: null, nativeError: null, tabs: [] };

  const chrome = {
    runtime: {
      lastError: null,
      getURL(target) {
        calls.getURL.push(target);
        return `moz-extension://integration.test/${target}`;
      },
      sendNativeMessage(host, message, callback) {
        calls.native.push({ host, message });
        if (state.nativeError) {
          chrome.runtime.lastError = { message: state.nativeError };
          callback();
          chrome.runtime.lastError = null;
          return;
        }
        callback(state.nativeReply);
      },
      onMessage: { addListener: (fn) => messageListeners.push(fn) },
      onStartup: { addListener: () => {} },
      onInstalled: { addListener: () => {} },
    },
    storage: {
      local: {
        get(key, callback) {
          callback(storage.has(key) ? { [key]: storage.get(key) } : {});
        },
        set(items, callback) {
          calls.storageSet.push(items);
          for (const [key, value] of Object.entries(items)) storage.set(key, value);
          if (callback) callback();
        },
      },
    },
    contextMenus: {
      removeAll(callback) {
        calls.menus.length = 0;
        if (callback) callback();
      },
      create(properties) {
        calls.menus.push(properties);
      },
      onClicked: { addListener: (fn) => menuListeners.push(fn) },
    },
    tabs: {
      query(queryInfo, callback) {
        calls.tabsQuery.push(queryInfo);
        callback(state.tabs);
      },
    },
    action: {
      setBadgeText: () => {},
      setBadgeBackgroundColor: () => {},
    },
  };

  return {
    chrome,
    calls,
    state,
    storage,
    messageListeners,
    menuListeners,
  };
}

/** Evaluate the real bundles and the real background script in one context. */
async function loadFirefoxBackground() {
  const harness = createFirefoxChrome();
  const sandbox = {
    chrome: harness.chrome,
    console: { warn: () => {}, log: () => {}, error: () => {} },
    URL,
    setTimeout,
    clearTimeout,
  };
  const context = vm.createContext(sandbox);
  for (const bundle of ['protocol.bundle.js', 'native-response.bundle.js']) {
    vm.runInContext(
      await readFile(path.join(EXTENSION, 'shared', bundle), 'utf8'),
      context,
      { filename: bundle }
    );
  }
  assert.equal(typeof sandbox.SwiftFetchProtocol, 'object', 'bundle must define the protocol global');
  assert.equal(
    typeof sandbox.SwiftFetchNativeResponse?.normalizeNativeResponse,
    'function',
    'bundle must define the native-response global'
  );
  vm.runInContext(
    await readFile(path.join(EXTENSION, 'firefox', 'background.js'), 'utf8'),
    context,
    { filename: 'firefox/background.js' }
  );
  await settle();
  harness.sendMessage = (message) =>
    new Promise((resolve) => {
      let response = null;
      let async = false;
      for (const listener of harness.messageListeners) {
        const returned = listener(message, { id: 'swiftfetch@localhost' }, (value) => {
          response = value;
          async = true;
        });
        if (returned === true) async = true;
      }
      const done = () => resolve(async ? response : null);
      if (async) setTimeout(done, 20);
      else done();
    });
  return harness;
}

describe('firefox/background.js (real file, callback-style chrome)', () => {
  let harness;

  before(async () => {
    harness = await loadFirefoxBackground();
  });

  it('bootstraps menus and listeners without any dynamic import', async () => {
    assert.equal(harness.calls.menus.length, 2);
    assert.deepEqual(
      harness.calls.menus.map((menu) => menu.id),
      ['swiftfetch-link', 'swiftfetch-media']
    );
    assert.ok(harness.messageListeners.length >= 1, 'onMessage listener registered');
    assert.ok(harness.menuListeners.length >= 1, 'contextMenus.onClicked listener registered');
    assert.deepEqual(
      harness.calls.getURL,
      [],
      'no chrome.runtime.getURL call: the classic preload is the only module source'
    );
  });

  it('normalizes an accepted executable reply into a task id', async () => {
    const taskId = '2ac6f999-a959-4afb-a998-e055c801a3d0';
    harness.state.nativeReply = { ok: true, value: { task_id: taskId } };
    const result = await harness.sendMessage({
      type: 'swiftfetch_intent',
      source: 'popup',
      payload: {
        url: FIXTURE_URL,
        suggestedName: 'file.zip',
        pageOrigin: PAGE_ORIGIN,
        contentType: 'application/zip',
      },
    });
    // `result` is built inside the vm realm, so compare its shape and values.
    assert.equal(result.ok, true);
    assert.equal(result.taskId, taskId);
    assert.deepEqual(Object.keys(result).sort(), ['ok', 'taskId']);
    assert.equal(harness.calls.native.length, 1);
    assert.equal(harness.calls.native[0].host, 'com.swiftfetch.app');
  });

  it('maps the opt-in rejection to its fixed, safe message', async () => {
    harness.state.nativeReply = {
      ok: false,
      code: 'bridge_disabled',
      message: 'raw host text that must not reach the UI',
    };
    const result = await harness.sendMessage({
      type: 'swiftfetch_intent',
      source: 'popup',
      payload: { url: FIXTURE_URL, suggestedName: 'file.zip', pageOrigin: PAGE_ORIGIN },
    });
    assert.equal(result.ok, false);
    assert.equal(result.code, 'bridge_disabled');
    assert.equal(result.message, 'Enable browser integration in the SwiftFetch desktop Settings.');
  });

  it('hides unknown host text instead of surfacing it', async () => {
    harness.state.nativeReply = {
      ok: false,
      code: 'unheard_of_code',
      message: 'https://user.example/private?token=sup3rsecret',
    };
    const result = await harness.sendMessage({
      type: 'swiftfetch_intent',
      source: 'popup',
      payload: { url: FIXTURE_URL, suggestedName: 'file.zip', pageOrigin: PAGE_ORIGIN },
    });
    assert.equal(result.code, 'native_host_bad_response');
    assert.ok(!JSON.stringify(result).includes('sup3rsecret'));
    assert.equal(result.message, NativeResponse.normalizeNativeResponse({ ok: false }).message);
  });

  it('reports a missing host as unavailable when the spawn fails', async () => {
    harness.state.nativeError = 'Native host has not been registered';
    harness.state.nativeReply = null;
    const result = await harness.sendMessage({
      type: 'swiftfetch_intent',
      source: 'popup',
      payload: { url: FIXTURE_URL, suggestedName: 'file.zip', pageOrigin: PAGE_ORIGIN },
    });
    assert.equal(result.code, 'native_host_unavailable');
    assert.equal(result.retryable, true);
    assert.ok(!JSON.stringify(result).includes('registered by'), 'no raw host error text');
    harness.state.nativeError = null;
    harness.state.nativeReply = null;
  });

  it('fails closed on a malformed reply', async () => {
    harness.state.nativeReply = { unexpected: true };
    const result = await harness.sendMessage({
      type: 'swiftfetch_intent',
      source: 'popup',
      payload: { url: FIXTURE_URL, suggestedName: 'file.zip', pageOrigin: PAGE_ORIGIN },
    });
    assert.equal(result.ok, false);
    assert.equal(result.code, 'native_host_bad_response');
    harness.state.nativeReply = null;
  });

  it('refuses everything when the master switch is off, without spawning a host', async () => {
    harness.storage.set(STORAGE_KEY, { enabled: false, siteRules: {} });
    const before = harness.calls.native.length;
    const result = await harness.sendMessage({
      type: 'swiftfetch_intent',
      source: 'popup',
      payload: { url: FIXTURE_URL, suggestedName: 'file.zip', pageOrigin: PAGE_ORIGIN },
    });
    assert.equal(result.code, 'extension_disabled');
    assert.equal(result.retryable, false);
    assert.equal(harness.calls.native.length, before, 'no native host spawn while disabled');
    harness.storage.delete(STORAGE_KEY);
  });

  it('honours a per-site deny rule', async () => {
    harness.storage.set(STORAGE_KEY, {
      enabled: true,
      siteRules: { [PAGE_ORIGIN]: 'deny' },
    });
    const before = harness.calls.native.length;
    const result = await harness.sendMessage({
      type: 'swiftfetch_intent',
      source: 'popup',
      payload: { url: FIXTURE_URL, suggestedName: 'file.zip', pageOrigin: PAGE_ORIGIN },
    });
    assert.equal(result.code, 'site_blocked');
    assert.equal(harness.calls.native.length, before);
    harness.storage.delete(STORAGE_KEY);
  });

  it('forwards exactly the ten protocol fields and never cookies, even from a hostile payload', async () => {
    harness.state.nativeReply = { ok: true, value: { task_id: 'f81d4fae-7dec-41d0-a765-00a0c91e6bf6' } };
    const result = await harness.sendMessage({
      type: 'swiftfetch_intent',
      source: 'content_script',
      payload: {
        url: FIXTURE_URL,
        suggestedName: 'C:\\Users\\me\\Downloads\\archive.zip',
        pageOrigin: PAGE_ORIGIN,
        contentType: 'application/zip',
        cookies: [{ name: 'session', value: 'abc' }],
        authorization: 'Bearer session-token',
        headers: { cookie: 'a=b' },
        junk: 'ignored',
      },
    });
    assert.equal(result.ok, true);
    const forwarded = harness.calls.native.at(-1).message;
    assert.deepEqual(
      Object.keys(forwarded).sort(),
      [...Protocol.INTENT_FIELDS].sort(),
      'no extra field may reach the host'
    );
    assert.deepEqual(plain(forwarded.cookies), []);
    assert.equal(forwarded.user_initiated, true);
    assert.equal(forwarded.suggested_name, 'archive.zip');
    assert.equal(forwarded.page_origin, PAGE_ORIGIN);
    assert.equal(forwarded.url, FIXTURE_URL);
    assert.match(forwarded.request_id, Protocol.UUID_V4_RE);
    assert.ok(Protocol.isExpiredIntent(forwarded, Date.parse(forwarded.expires_at) + 1));
    harness.state.nativeReply = null;
  });

  it('gives every gesture a fresh request id', async () => {
    harness.state.nativeReply = { ok: true, value: { task_id: '0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0' } };
    const payload = {
      url: FIXTURE_URL,
      suggestedName: 'file.zip',
      pageOrigin: PAGE_ORIGIN,
      contentType: 'application/zip',
    };
    await harness.sendMessage({ type: 'swiftfetch_intent', source: 'popup', payload });
    await harness.sendMessage({ type: 'swiftfetch_intent', source: 'popup', payload });
    const [first, second] = harness.calls.native.slice(-2);
    assert.notEqual(first.message.request_id, second.message.request_id);
    harness.state.nativeReply = null;
  });

  it('serves the popup state query and settings writes', async () => {
    harness.state.tabs = [{ url: 'http://127.0.0.1:8080/dir/page.zip' }];
    const state = await harness.sendMessage({ type: 'swiftfetch_get_state' });
    assert.equal(state.ok, true);
    assert.equal(state.enabled, true);
    assert.equal(state.origin, PAGE_ORIGIN);
    assert.equal(state.rule, 'ask');
    // Cross-realm object from the event page: project it before comparing.
    assert.deepEqual({ ...harness.calls.tabsQuery[0] }, { active: true, currentWindow: true });

    const written = await harness.sendMessage({ type: 'swiftfetch_set_rule', origin: PAGE_ORIGIN, rule: 'deny' });
    assert.deepEqual({ ...written }, { ok: true, origin: PAGE_ORIGIN, rule: 'deny' });
    assert.deepEqual(plain(harness.storage.get(STORAGE_KEY)), {
      enabled: true,
      siteRules: { [PAGE_ORIGIN]: 'deny' },
    });

    const disabled = await harness.sendMessage({ type: 'swiftfetch_set_enabled', enabled: false });
    assert.deepEqual({ ...disabled }, { ok: true, enabled: false });
    assert.equal(harness.storage.get(STORAGE_KEY).enabled, false);
    harness.storage.delete(STORAGE_KEY);
  });

  it('forwards a context-menu gesture with the tab origin', async () => {
    harness.state.nativeReply = { ok: true, value: { task_id: '3b8a6f2e-1c4d-4e5f-9a7b-2c6d8e0f1a2b' } };
    const callsBefore = harness.calls.native.length;
    harness.menuListeners[0]({ linkUrl: 'https://example.com/report.pdf' }, { url: 'https://example.com/dir/' });
    await settle(2);
    assert.equal(harness.calls.native.length, callsBefore + 1, 'the context menu spawns the host exactly once');
    const menuCall = harness.calls.native.at(-1);
    const forwarded = menuCall.message;
    assert.equal(forwarded.url, 'https://example.com/report.pdf', `forwarded url was ${forwarded.url}`);
    assert.equal(forwarded.page_origin, 'https://example.com');
    assert.equal(menuCall.host, 'com.swiftfetch.app');
    harness.state.nativeReply = null;
  });

  it('ignores unrelated messages', async () => {
    const result = await harness.sendMessage({ type: 'something_else' });
    assert.equal(result, null);
    assert.deepEqual(harness.calls.getURL, [], 'still no dynamic import at any point');
  });
});