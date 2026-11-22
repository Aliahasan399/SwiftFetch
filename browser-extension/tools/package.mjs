import { mkdir, readFile, writeFile, copyFile } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const destination = path.resolve(process.argv[2] || path.join(root, '..', 'artifacts', 'browser-extension', randomUUID()));
await mkdir(path.dirname(destination), { recursive: true });
await mkdir(destination);

for (const browser of ['chromium', 'firefox']) {
  const output = path.join(destination, browser);
  await mkdir(path.join(output, 'shared'), { recursive: true });
  await mkdir(path.join(output, 'icons'));
  const source = browser === 'chromium' ? 'chrome' : 'firefox';
  const manifest = JSON.parse(await readFile(path.join(root, source, 'manifest.json'), 'utf8'));
  const canonical = JSON.stringify(manifest, null, 2)
    .replaceAll('../chrome/', '')
    .replaceAll('../shared/', './shared/');
  await writeFile(path.join(output, 'manifest.json'), `${canonical}\n`, { flag: 'wx' });
  const worker = await readFile(path.join(root, source, 'background.js'), 'utf8');
  await writeFile(path.join(output, 'background.js'), worker.replaceAll("'../shared/", "'./shared/"), { flag: 'wx' });
  for (const file of ['content.js', 'popup.js', 'popup.html']) await copyFile(path.join(root, 'chrome', file), path.join(output, file));
  for (const file of [
    'protocol.js', 'native-response.js', 'protocol.bundle.js', 'native-response.bundle.js',
    'media-detect.js', 'media-detect.bundle.js', 'media-indicator.js', 'media-indicator.bundle.js',
  ]) await copyFile(path.join(root, 'shared', file), path.join(output, 'shared', file));
  for (const size of [16, 32, 48, 128]) await copyFile(path.join(root, 'chrome', 'icons', `${size}.png`), path.join(output, 'icons', `${size}.png`));
}
console.log(destination);
