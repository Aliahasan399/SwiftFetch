/**
 * Generate classic-script bundles from the shared ES modules.
 *
 * Firefox Manifest V2 background pages are classic scripts. A classic script
 * cannot contain `export`, and a dynamic `import()` inside a Manifest V2 event
 * page is not guaranteed, so the shipped Firefox package preloads these bundles
 * through `background.scripts` and reads them from `globalThis`.
 *
 * The source modules keep their `export` statements: `test/test_protocol.js`
 * and every other Node consumer import them as ES modules. The bundles are
 * pure transformations — no logic is duplicated by hand.
 *
 * Usage:
 *   node browser-extension/tools/make-protocol-bundle.mjs           # write bundles
 *   node browser-extension/tools/make-protocol-bundle.mjs --check   # fail on drift
 */
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const SHARED = join(ROOT, 'shared');
const CHECK = process.argv.includes('--check');

const BANNER = [
  '# ------------------------------------------------------------------ #',
  '# GENERATED FILE — do not edit.',
  '#',
  '# Produced by tools/make-protocol-bundle.mjs from the matching ES module',
  '# in this folder. Run `node browser-extension/tools/make-protocol-bundle.mjs`',
  '# after changing the module. The classic form exists so Firefox Manifest V2',
  '# can preload the protocol with `background.scripts`, without relying on a',
  '# dynamic import() inside an event page. The module keeps its exports for',
  '# Node tests and for the Chrome service worker.',
  '# ------------------------------------------------------------------ #',
  '',
].map((line) => line.replace(/^#/, '//')).join('\n');

/**
 * Cut the trailing `export { ... }` block off an ES module, keeping the
 * `api` object it exports, and append the classic-script global assignment.
 * Everything before the export block is byte-identical to the module body, so
 * the two implementations cannot drift apart.
 */
function classicFromModule(source, { global }) {
  const marker = '\nexport {';
  const index = source.indexOf(marker);
  if (index < 0) {
    throw new Error('module has no trailing `export { ... }` block to replace with a global');
  }
  if (!/\bconst api = \{/.test(source)) {
    throw new Error('module does not define the `api` object the global assigns');
  }
  const head = source.slice(0, index).replace(/\s+$/, '');
  const body = `${head}\n\n/* Classic-script export: available as a global in a non-module context. */\nglobalThis.${global} = api;\n`;
  return `${BANNER}${body}`;
}

/** Small modules that export a single function: rewrite the statement. */
function classicFromSingleExport(source, { functionName, global }) {
  const declaration = `export function ${functionName}`;
  if (!source.includes(declaration)) {
    throw new Error(`module does not export ${functionName}`);
  }
  const body = source
    .replace(declaration, `function ${functionName}`)
    .replace(/\s+$/, '');
  return `${BANNER}${body}\n\n/* Classic-script export: available as a global in a non-module context. */\nglobalThis.${global} = { ${functionName} };\n`;
}

const TARGETS = [
  {
    module: 'protocol.js',
    bundle: 'protocol.bundle.js',
    global: 'SwiftFetchProtocol',
    transform: classicFromModule,
  },
  {
    module: 'native-response.js',
    bundle: 'native-response.bundle.js',
    global: 'SwiftFetchNativeResponse',
    functionName: 'normalizeNativeResponse',
    transform: classicFromSingleExport,
  },
];

let drift = 0;
for (const target of TARGETS) {
  const source = await readFile(join(SHARED, target.module), 'utf8');
  const bundle = target.transform(source, target);
  const bundlePath = join(SHARED, target.bundle);
  if (CHECK) {
    let current = null;
    try {
      current = await readFile(bundlePath, 'utf8');
    } catch (error) {
      current = null;
    }
    if (current !== bundle) {
      drift += 1;
      process.stderr.write(
        `bundle drift: shared/${target.bundle} is out of date with shared/${target.module}\n`
      );
    } else {
      process.stdout.write(`up to date: shared/${target.bundle}\n`);
    }
    continue;
  }
  await writeFile(bundlePath, bundle, 'utf8');
  process.stdout.write(`wrote shared/${target.bundle} (${bundle.length} bytes) from shared/${target.module}\n`);
}

if (CHECK && drift > 0) {
  process.exit(1);
}
