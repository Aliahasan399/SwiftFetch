import assert from 'node:assert/strict';
import test from 'node:test';
import { execFileSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';
import { normalizeNativeResponse } from '../shared/native-response.js';

const project = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

test('native replies use the executable receipt and hide unknown host text', () => {
  const id = '2ac6f999-a959-4afb-a998-e055c801a3d0';
  assert.deepEqual(normalizeNativeResponse({ ok: true, value: { task_id: id } }), { ok: true, taskId: id });
  assert.equal(normalizeNativeResponse({ ok: true }).ok, false);
  const error = normalizeNativeResponse({ ok: false, code: 'arbitrary', message: 'https://secret.example/?token=private' });
  assert.ok(!JSON.stringify(error).includes('private'));
  assert.equal(normalizeNativeResponse({ ok: false, code: 'bridge_disabled' }).code, 'bridge_disabled');
});

test('packaged manifests and imports resolve entirely inside their own directory', async () => {
  const artifactRoot = path.join(project, 'artifacts', 'extension-tests');
  await mkdir(artifactRoot, { recursive: true });
  const workspace = await mkdtemp(path.join(artifactRoot, 'run-'));
  const packaged = path.join(workspace, 'packages');
  execFileSync(process.execPath, [path.join(project, 'browser-extension/tools/package.mjs'), packaged], { cwd: project });
  for (const browser of ['chromium', 'firefox']) {
    const root = path.join(packaged, browser);
    const manifest = JSON.parse(await readFile(path.join(root, 'manifest.json'), 'utf8'));
    const permission = [...manifest.permissions].sort();
    assert.deepEqual(permission, ['contextMenus', 'nativeMessaging', 'storage', 'tabs']);
    const action = manifest.action || manifest.browser_action;
    const references = [
      ...(manifest.background.scripts || [manifest.background.service_worker]),
      ...manifest.content_scripts.flatMap((script) => script.js),
      action.default_popup, ...Object.values(manifest.icons), ...Object.values(action.default_icon),
    ];
    for (const reference of references) {
      const target = path.resolve(root, reference);
      assert.ok(target.startsWith(`${root}${path.sep}`));
      assert.ok((await stat(target)).isFile());
    }
    const worker = await readFile(path.join(root, 'background.js'), 'utf8');
    assert.ok(!worker.includes('../shared/'));
    for (const shared of ['protocol.js', 'native-response.js']) assert.ok((await stat(path.join(root, 'shared', shared))).isFile());
  }
});

test('synthetic page clicks cannot send a browser download intent', async () => {
  const handlers = {};
  let messages = 0;
  const context = {
    window: {}, document: { addEventListener: (name, handler) => { handlers[name] = handler; } },
    chrome: { runtime: { sendMessage: () => { messages++; } } },
  };
  vm.runInNewContext(await readFile(path.join(project, 'browser-extension/chrome/content.js'), 'utf8'), context);
  await handlers.click({ isTrusted: false, button: 0, altKey: true });
  assert.equal(messages, 0);
  assert.equal(context.window.__swiftfetchSend, undefined);
});

test('Firefox classic bundles initialize the protocol and native response without dynamic imports', async () => {
  const globals = {URL, crypto:globalThis.crypto, console};
  vm.createContext(globals);
  vm.runInContext(await readFile(path.join(project,'browser-extension/shared/protocol.bundle.js'),'utf8'),globals);
  vm.runInContext(await readFile(path.join(project,'browser-extension/shared/native-response.bundle.js'),'utf8'),globals);
  assert.equal(typeof globals.SwiftFetchProtocol.buildIntent,'function');
  assert.equal(typeof globals.SwiftFetchNativeResponse.normalizeNativeResponse,'function');
  assert.equal(globals.SwiftFetchProtocol.validateUrl('file:///C:/secret').ok,false);
  execFileSync(process.execPath,[path.join(project,'browser-extension/tools/make-protocol-bundle.mjs'),'--check'],{cwd:project});
});
