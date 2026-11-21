import { spawn } from 'node:child_process';
import { mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { createServer } from 'node:net';
const binary = path.resolve('src-tauri/target/release/swiftfetch.exe');
const root = await mkdtemp(path.resolve('artifacts/startup-probe/diag-'));
const reservation = createServer();
await new Promise(r => reservation.listen(0, '127.0.0.1', r));
const port = reservation.address().port;
await new Promise(r => reservation.close(r));
const app = spawn(binary, [], { cwd: root, windowsHide: true, stdio: 'ignore', env: { ...process.env,
  SWIFTFETCH_DB_PATH: path.join(root, 'state.sqlite'), SWIFTFETCH_DATA_DIR: root,
  SWIFTFETCH_DOWNLOADS_DIR: path.join(root, 'downloads'), WEBVIEW2_USER_DATA_FOLDER: path.join(root, 'webview'),
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-address=127.0.0.1 --remote-debugging-port=${port}`,
}});
const snapshots = [];
try {
  for (let i = 0; i < 60; i++) {
    await delay(500);
    try {
      const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`, { signal: AbortSignal.timeout(500) })).json();
      snapshots.push({ at: i * 500, targets: targets.map(t => ({ type: t.type, url: t.url, title: t.title })) });
    } catch (e) { snapshots.push({ at: i * 500, error: String(e) }); }
  }
  await writeFile(path.join(root, 'targets.json'), JSON.stringify(snapshots, null, 2));
  console.log(root);
} finally {
  if (app.exitCode === null) app.kill();
}
